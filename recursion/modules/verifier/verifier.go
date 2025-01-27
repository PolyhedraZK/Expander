package verifier

import (
	"log"
	"math/bits"

	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"
	"ExpanderVerifierCircuit/modules/polycommit"
	"ExpanderVerifierCircuit/modules/transcript"

	"github.com/consensys/gnark/frontend"
)

func SumcheckStepVerify(
	api fields.ArithmeticEngine,
	proof *circuit.Proof,
	degree uint,
	fsTranscript *transcript.FieldHasherTranscript,
	claimedEval []frontend.Variable,
	randomPoints [][]frontend.Variable,
	sp ScratchPad,
) ([]frontend.Variable, [][]frontend.Variable) {

	ps := make([][]frontend.Variable, degree+1)
	for i := uint(0); i <= degree; i++ {
		ps[i] = proof.NextChallengeF(api)
		fsTranscript.AppendFs(ps[i]...)
	}

	api.AssertEq(api.ExtensionAdd(ps[0], ps[1]), claimedEval)

	randomPoint := fsTranscript.ChallengeF()
	randomPoints = append(randomPoints, randomPoint)

	switch degree {
	case 2:
		claimedEval = Degree2Eval(api, ps, randomPoint, sp)
	case 3:
		claimedEval = Degree3Eval(api, ps, randomPoint, sp)
	default:
		panic("Degree unsupported")
	}

	return claimedEval, randomPoints
}

func SumcheckLayerVerify(
	api fields.ArithmeticEngine,
	layer *circuit.Layer,
	// NOTE(HS) this is SIMD circuit field
	publicInput [][]frontend.Variable,

	rz0, rz1, rSIMD, rMPI [][]frontend.Variable,

	claimedV0, claimedV1, alpha []frontend.Variable,

	proof *circuit.Proof,
	fsTranscript *transcript.FieldHasherTranscript,
	sp *ScratchPad,
	isOutputLayer bool,
) (
	rx, ry, r_simd_xy, r_mpi_xy [][]frontend.Variable,
	vx_claim, vy_claim []frontend.Variable,
) {

	PrepareLayer(
		api,
		layer,
		alpha,
		rz0,
		rz1,
		rSIMD,
		rMPI,
		sp,
		isOutputLayer,
	)

	var var_num = layer.InputLenLog
	var simd_var_num = len(rSIMD)
	var mpi_var_num = len(rMPI)
	var sum = claimedV0

	if alpha != nil && claimedV1 != nil {
		sum = api.ExtensionAdd(
			sum,
			api.ExtensionMul(
				alpha,
				claimedV1,
			),
		)
	}

	sum = api.ExtensionSub(
		sum,
		EvalCst(api, layer.Cst, publicInput, sp),
	)

	for i := uint(0); i < var_num; i++ {
		sum, rx = SumcheckStepVerify(
			api,
			proof,
			2,
			fsTranscript,
			sum,
			rx,
			*sp,
		)
	}
	SetRx(api, rx, sp)

	for i := 0; i < simd_var_num; i++ {
		sum, r_simd_xy = SumcheckStepVerify(
			api,
			proof,
			3,
			fsTranscript,
			sum,
			r_simd_xy,
			*sp,
		)
	}
	SetRSimdXY(api, r_simd_xy, sp)

	for i := 0; i < mpi_var_num; i++ {
		sum, r_mpi_xy = SumcheckStepVerify(
			api,
			proof,
			3,
			fsTranscript,
			sum,
			r_mpi_xy,
			*sp,
		)
	}
	SetRMPIXY(api, r_mpi_xy, sp)

	vx_claim = proof.NextChallengeF(api)
	sum = api.ExtensionSub(
		sum,
		api.ExtensionMul(
			vx_claim,
			EvalAdd(api, layer.Add, sp),
		),
	)
	fsTranscript.AppendFs(vx_claim...)

	if layer.StructureInfo.MaxDegreeOne {
		api.AssertEq(sum, api.Zero())
	} else {
		for i := uint(0); i < var_num; i++ {
			sum, ry = SumcheckStepVerify(
				api,
				proof,
				2,
				fsTranscript,
				sum,
				ry,
				*sp,
			)
		}
		SetRY(api, ry, sp)

		vy_claim = proof.NextChallengeF(api)
		fsTranscript.AppendFs(vy_claim...)
		api.AssertEq(
			sum,
			api.ExtensionMul(
				vx_claim,
				vy_claim,
				EvalMul(api, layer.Mul, sp),
			),
		)
	}

	return
}

func GKRVerify(
	api fields.ArithmeticEngine,
	circuit *circuit.Circuit,
	public_input [][]frontend.Variable,
	claimed_v []frontend.Variable,
	mpiSize uint,
	fsTranscript *transcript.FieldHasherTranscript,
	proof *circuit.Proof,
) (
	rz0, rz1, rSIMD, rMPI [][]frontend.Variable,
	claimedV0, claimedV1 []frontend.Variable,
) {
	simdSize := api.SIMDPackSize()
	sp := NewScratchPad(api, circuit, mpiSize)

	layerNum := len(circuit.Layers)

	for i := 0; i < int(circuit.Layers[len(circuit.Layers)-1].OutputLenLog); i++ {
		rz0 = append(rz0, fsTranscript.ChallengeF())
	}

	for i := 0; i < bits.TrailingZeros(simdSize); i++ {
		rSIMD = append(rSIMD, fsTranscript.ChallengeF())
	}

	for i := 0; i < bits.TrailingZeros(mpiSize); i++ {
		rMPI = append(rMPI, fsTranscript.ChallengeF())
	}

	var alpha []frontend.Variable = nil
	claimedV0 = claimed_v
	claimedV1 = nil

	for i := layerNum - 1; i >= 0; i-- {
		rz0, rz1, rSIMD, rMPI, claimedV0, claimedV1 = SumcheckLayerVerify(
			api,
			&circuit.Layers[i],
			public_input,
			rz0,
			rz1,
			rSIMD,
			rMPI,
			claimedV0,
			claimedV1,
			alpha,
			proof,
			fsTranscript,
			&sp,
			i == layerNum-1,
		)

		if rz1 != nil && claimedV1 != nil {
			alpha = fsTranscript.ChallengeF()
		} else {
			alpha = nil
		}
	}

	for size, count := range sp.EqEvalsCount {
		log.Println("Eq Evals Size", size, " Count: ", count)
	}

	return
}

func Verify(
	api fields.ArithmeticEngine,
	fieldEnum fields.ECCFieldEnum,
	originalCircuit *circuit.Circuit,
	public_input [][]frontend.Variable,
	claimed_v []frontend.Variable,
	mpiSize uint,
	proof *circuit.Proof,
) {
	fsTranscript := transcript.NewTranscript(api)

	// Only supports RawCommitment now
	circuitInputSize := uint(1) << originalCircuit.Layers[0].InputLenLog

	// NOTE(HS) for now just raw commitment scheme
	polyCom := polycommit.NewCommitment(
		polycommit.RawCommitmentScheme,
		fieldEnum,
		circuitInputSize, mpiSize,
		proof,
		fsTranscript,
	)

	// NOTE: MPI Fiat-Shamir sync randomness
	if mpiSize > 1 {
		newState := fsTranscript.HashAndReturnState()
		fsTranscript.SetState(newState)
	}

	if mpiSize > 1 {
		log.Println("#Hashes for input: ", fsTranscript.GetCount())
	}
	fsTranscript.ResetCount()

	originalCircuit.FillRndCoef(fsTranscript)

	log.Println("#Hashes for random gate: ", fsTranscript.GetCount())
	fsTranscript.ResetCount()

	var rx, ry, r_simd, r_mpi, claimed_v0, claimed_v1 = GKRVerify(
		api,
		originalCircuit,
		public_input,
		claimed_v,
		mpiSize,
		fsTranscript,
		proof,
	)

	log.Println("#Hashes for gkr challenge: ", fsTranscript.GetCount())
	fsTranscript.ResetCount()

	polyCom.Verify(api, rx, r_simd, r_mpi, claimed_v0)

	if ry != nil {
		polyCom.Verify(api, ry, r_simd, r_mpi, claimed_v1)
	}
}
