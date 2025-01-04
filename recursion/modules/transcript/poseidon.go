package transcript

import (
	"encoding/binary"
	"math/big"

	"ExpanderVerifierCircuit/modules/fields"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/utils/customgates"
	"github.com/consensys/gnark/frontend"
	"golang.org/x/crypto/sha3"
)

var (
	poseidonM31x16FullRounds    uint
	poseidonM31x16PartialRounds uint

	poseidonM31x16RoundConstant [][]uint
	poseidonM31x16MDS           [][]uint

	POW_5_GATE_ID     uint64 = 12345
	POW_5_COST_PSEUDO int    = 20
)

func sBox(api frontend.API, f frontend.Variable) frontend.Variable {
	return api.(ecgo.API).CustomGate(POW_5_GATE_ID, f)
}

func Power5(field *big.Int, inputs []*big.Int, outputs []*big.Int) error {
	a := big.NewInt(0)
	a.Mul(inputs[0], inputs[0])
	a.Mul(a, a)
	a.Mul(a, inputs[0])
	outputs[0] = a
	return nil
}

func poseidonM31x16Init() {
	poseidonM31x16FullRounds = 8
	poseidonM31x16PartialRounds = 14

	var m31Modulus uint = uint(fields.ECCM31.FieldModulus().Int64())

	// NOTE Poseidon full round parameter generation
	poseidonM31x16Seed := []byte("poseidon_seed_Mersenne 31_16")

	hasher := sha3.NewLegacyKeccak256()
	hasher.Write(poseidonM31x16Seed)
	poseidonM31x16Seed = hasher.Sum(nil)

	poseidonM31x16RoundConstant = make([][]uint, poseidonM31x16FullRounds+poseidonM31x16PartialRounds)
	for i := 0; i < int(poseidonM31x16FullRounds+poseidonM31x16PartialRounds); i++ {
		poseidonM31x16RoundConstant[i] = make([]uint, 16)

		for j := 0; j < 16; j++ {
			hasher.Reset()
			hasher.Write(poseidonM31x16Seed)
			poseidonM31x16Seed = hasher.Sum(nil)

			u32LE := binary.LittleEndian.Uint32(poseidonM31x16Seed[:4])
			poseidonM31x16RoundConstant[i][j] = uint(u32LE) % m31Modulus
		}
	}

	// NOTE MDS generation
	poseidonM31x16MDS = make([][]uint, 16)
	poseidonM31x16MDS[0] = []uint{1, 1, 51, 1, 11, 17, 2, 1, 101, 63, 15, 2, 67, 22, 13, 3}
	for i := 1; i < 16; i++ {
		poseidonM31x16MDS[i] = make([]uint, 16)
		for j := 0; j < 16; j++ {
			poseidonM31x16MDS[i][j] = poseidonM31x16MDS[0][(i+j)%16]
		}
	}

	// NOTE register pow-5 gate
	customgates.Register(POW_5_GATE_ID, Power5, POW_5_COST_PSEUDO)
}

func init() {
	poseidonM31x16Init()
}

func poseidonM31x16MDSApply(
	api frontend.API, state []frontend.Variable) []frontend.Variable {

	res := make([]frontend.Variable, 16)
	for i := 0; i < 16; i++ {
		res[i] = 0
	}

	for i := 0; i < 16; i++ {
		for j := 0; j < 16; j++ {
			res[i] = api.Add(api.Mul(poseidonM31x16MDS[i][j], state[j]), res[i])
		}
	}

	return res
}

func poseidonM31x16FullRoundSBox(
	api frontend.API, state []frontend.Variable) []frontend.Variable {

	for i := 0; i < 16; i++ {
		state[i] = sBox(api, state[i])
	}

	return state
}

func poseidonM31x16PartialRoundSbox(
	api frontend.API, state []frontend.Variable) []frontend.Variable {

	state[0] = sBox(api, state[0])

	return state
}

func poseidonM31x16RoundConstantApply(
	api frontend.API, state []frontend.Variable, round uint) []frontend.Variable {

	for i := 0; i < 16; i++ {
		state[i] = api.Add(state[i], poseidonM31x16RoundConstant[round][i])
	}

	return state
}

func poseidonM31x16Permutate(
	api frontend.API, state []frontend.Variable) []frontend.Variable {

	partialRoundEnds := poseidonM31x16FullRounds/2 + poseidonM31x16PartialRounds
	allRoundEnds := poseidonM31x16FullRounds + poseidonM31x16PartialRounds

	for i := uint(0); i < poseidonM31x16FullRounds/2; i++ {
		state = poseidonM31x16RoundConstantApply(api, state, i)
		state = poseidonM31x16MDSApply(api, state)
		state = poseidonM31x16FullRoundSBox(api, state)
	}

	for i := poseidonM31x16FullRounds / 2; i < partialRoundEnds; i++ {
		state = poseidonM31x16RoundConstantApply(api, state, i)
		state = poseidonM31x16MDSApply(api, state)
		state = poseidonM31x16PartialRoundSbox(api, state)
	}

	for i := partialRoundEnds; i < allRoundEnds; i++ {
		state = poseidonM31x16RoundConstantApply(api, state, i)
		state = poseidonM31x16MDSApply(api, state)
		state = poseidonM31x16FullRoundSBox(api, state)
	}

	return state
}

func poseidonM31x16HashToState(
	api frontend.API, fs []frontend.Variable) ([]frontend.Variable, uint) {

	poseidonM31x16Rate := 8
	poseidonM31x16Capacity := 16 - poseidonM31x16Rate
	numChunks := (len(fs) + poseidonM31x16Rate - 1) / poseidonM31x16Rate

	absorbBuffer := make([]frontend.Variable, numChunks*poseidonM31x16Rate)
	copy(absorbBuffer, fs)
	for i := len(fs); i < len(absorbBuffer); i++ {
		absorbBuffer[i] = 0
	}

	res := make([]frontend.Variable, 16)
	for i := 0; i < 16; i++ {
		res[i] = 0
	}

	for i := 0; i < numChunks; i++ {
		for j := poseidonM31x16Capacity; j < 16; j++ {
			res[j] = api.Add(res[j], absorbBuffer[i*poseidonM31x16Rate+j-poseidonM31x16Capacity])
		}
		res = poseidonM31x16Permutate(api, res)
	}

	return res, uint(numChunks)
}

type PoseidonM31x16Hasher struct {
	fields.ArithmeticEngine
}

func NewPoseidonM31x16Hasher(api fields.ArithmeticEngine) PoseidonM31x16Hasher {
	return PoseidonM31x16Hasher{ArithmeticEngine: api}
}

func (h *PoseidonM31x16Hasher) StateCapacity() uint {
	return 8
}

func (h *PoseidonM31x16Hasher) HashToState(fs ...frontend.Variable) ([]frontend.Variable, uint) {
	return poseidonM31x16HashToState(h.ArithmeticEngine.API, fs)
}
