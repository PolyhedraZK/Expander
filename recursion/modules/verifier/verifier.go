package verifier

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"
	"ExpanderVerifierCircuit/modules/polycommit"
	"ExpanderVerifierCircuit/modules/transcript"
	"log"
	"math/bits"

	"github.com/consensys/gnark/frontend"
)

func SumcheckStepVerify(
	api frontend.API,
	proof *circuit.Proof,
	degree uint,
	fsTranscript transcript.Transcript,
	claimed_sum frontend.Variable,
	randomness_vec []frontend.Variable,
	sp *ScratchPad,
) (frontend.Variable, []frontend.Variable) {
	var ps = make([]frontend.Variable, 0)
	for i := uint(0); i < (degree + 1); i++ {
		ps = append(ps, proof.Next())
		fsTranscript.AppendF(ps[i])
	}

	var r = fsTranscript.ChallengeF()
	randomness_vec = append(randomness_vec, r)
	api.AssertIsEqual(api.Add(ps[0], ps[1]), claimed_sum)

	if degree == 2 {
		return Degree2Eval(api, ps, r, sp), randomness_vec
	} else if degree == 3 {
		return Degree3Eval(api, ps, r, sp), randomness_vec
	} else {
		panic("Incorrect Degree")
	}
}

func SumcheckLayerVerify(
	api frontend.API,
	layer *circuit.Layer,
	public_input [][]frontend.Variable,
	rz0 []frontend.Variable,
	rz1 []frontend.Variable,
	r_simd []frontend.Variable,
	r_mpi []frontend.Variable,
	claimed_v0 frontend.Variable,
	claimed_v1 frontend.Variable,
	alpha frontend.Variable,
	proof *circuit.Proof,
	fsTranscript transcript.Transcript,
	sp *ScratchPad,
	is_output_layer bool,
) (
	[]frontend.Variable,
	[]frontend.Variable,
	[]frontend.Variable,
	[]frontend.Variable,
	frontend.Variable,
	frontend.Variable,
) {
	PrepareLayer(
		api,
		layer,
		alpha,
		rz0,
		rz1,
		r_simd,
		r_mpi,
		sp,
		is_output_layer,
	)

	var var_num = layer.InputLenLog
	var simd_var_num = len(r_simd)
	var mpi_var_num = len(r_mpi)
	var sum = claimed_v0
	if alpha != nil && claimed_v1 != nil {
		sum = api.Add(sum, api.Mul(alpha, claimed_v1))
	}
	sum = api.Sub(sum, EvalCst(api, layer.Cst, public_input, sp))

	var rx = make([]frontend.Variable, 0)
	var ry []frontend.Variable = nil
	var r_simd_xy = make([]frontend.Variable, 0)
	var r_mpi_xy = make([]frontend.Variable, 0)

	for i := uint(0); i < var_num; i++ {
		sum, rx = SumcheckStepVerify(
			api,
			proof,
			2,
			fsTranscript,
			sum,
			rx,
			sp,
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
			sp,
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
			sp,
		)
	}
	SetRMPIXY(api, r_mpi_xy, sp)

	var vx_claim = proof.Next()
	sum = api.Sub(sum, api.Mul(
		vx_claim,
		EvalAdd(api, layer.Add, sp),
	))
	fsTranscript.AppendF(vx_claim)

	var vy_claim frontend.Variable = nil
	if layer.StructureInfo.MaxDegreeOne {
		api.AssertIsEqual(sum, 0)
	} else {
		ry = make([]frontend.Variable, 0)
		for i := uint(0); i < var_num; i++ {
			sum, ry = SumcheckStepVerify(
				api,
				proof,
				2,
				fsTranscript,
				sum,
				ry,
				sp,
			)
		}
		SetRY(api, ry, sp)

		vy_claim = proof.Next()
		fsTranscript.AppendF(vy_claim)
		api.AssertIsEqual(sum, api.Mul(
			vx_claim,
			vy_claim,
			EvalMul(api, layer.Mul, sp),
		))
	}

	return rx, ry, r_simd_xy, r_mpi_xy, vx_claim, vy_claim
}

func GKRVerify(
	api frontend.API,
	circuit *circuit.Circuit,
	public_input [][]frontend.Variable,
	claimed_v frontend.Variable,
	simd_size uint,
	mpi_size uint,
	fsTranscript transcript.Transcript,
	proof *circuit.Proof,
) (
	[]frontend.Variable,
	[]frontend.Variable,
	[]frontend.Variable,
	[]frontend.Variable,
	frontend.Variable,
	frontend.Variable,
) {
	var sp, err = NewScratchPad(api, circuit, simd_size, mpi_size)
	if err != nil {
		panic("Error init scratch pad")
	}

	var n_layers = len(circuit.Layers)
	var rz0 = make([]frontend.Variable, 0)
	var rz1 []frontend.Variable = nil
	var r_simd = make([]frontend.Variable, 0)
	var r_mpi = make([]frontend.Variable, 0)

	for i := 0; i < int(circuit.Layers[len(circuit.Layers)-1].OutputLenLog); i++ {
		rz0 = append(rz0, fsTranscript.ChallengeF())
	}

	for i := 0; i < bits.TrailingZeros(simd_size); i++ {
		r_simd = append(r_simd, fsTranscript.ChallengeF())
	}

	for i := 0; i < bits.TrailingZeros(mpi_size); i++ {
		r_mpi = append(r_mpi, fsTranscript.ChallengeF())
	}

	var alpha frontend.Variable = nil
	var claimed_v0 = claimed_v
	var claimed_v1 frontend.Variable = nil

	for i := n_layers - 1; i >= 0; i-- {
		rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1 = SumcheckLayerVerify(
			api,
			&circuit.Layers[i],
			public_input,
			rz0,
			rz1,
			r_simd,
			r_mpi,
			claimed_v0,
			claimed_v1,
			alpha,
			proof,
			fsTranscript,
			sp,
			i == n_layers-1,
		)

		if rz1 != nil && claimed_v1 != nil {
			alpha = fsTranscript.ChallengeF()
		} else {
			alpha = nil
		}
	}

	for size, count := range sp.EqEvalsCount {
		log.Println("Eq Evals Size", size, " Count: ", count)
	}

	return rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1
}

func Verify(
	api frontend.API,
	originalCircuit *circuit.Circuit,
	public_input [][]frontend.Variable,
	claimed_v frontend.Variable,
	simdSize uint,
	mpiSize uint,
	fieldEnum fields.ECCFieldEnum,
	proof *circuit.Proof,
) {
	fsTranscript, err := transcript.NewMiMCTranscript(api)
	if err != nil {
		panic("Err in transcript init")
	}

	// Only supports RawCommitment now
	circuitInputSize := uint(1) << originalCircuit.Layers[0].InputLenLog

	// NOTE(HS) for now just raw commitment scheme
	polyCom, err := polycommit.NewCommitment(
		polycommit.RawCommitmentScheme,
		fieldEnum,
		circuitInputSize, mpiSize,
		proof,
		fsTranscript,
	)
	if err != nil {
		panic(err.Error())
	}

	// Trigger an additional hash
	if mpiSize > 1 {
		_ = fsTranscript.ChallengeF()
	}

	log.Println("#Hashes for input: ", fsTranscript.GetCount())
	fsTranscript.ResetCount()

	originalCircuit.FillRndCoef(fsTranscript)

	log.Println("#Hashes for random gate: ", fsTranscript.GetCount())
	fsTranscript.ResetCount()

	var rx, ry, r_simd, r_mpi, claimed_v0, claimed_v1 = GKRVerify(
		api,
		originalCircuit,
		public_input,
		claimed_v,
		simdSize,
		mpiSize,
		fsTranscript,
		proof,
	)

	log.Println("#Hashes for gkr challenge: ", fsTranscript.GetCount())
	fsTranscript.ResetCount()

	// TODO(HS) remove this after I work on M31?
	if len(r_simd) > 0 {
		panic("Simd not supported yet.")
	}

	rx = append(rx, r_mpi...)
	ry = append(ry, r_mpi...)

	polyCom.Verify(api, rx, claimed_v0)
	polyCom.Verify(api, ry, claimed_v1)
}
