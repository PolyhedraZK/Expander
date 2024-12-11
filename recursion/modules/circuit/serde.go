package circuit

import (
	"encoding/binary"
	"fmt"
	"math/big"
	"math/bits"
	"os"
	"slices"

	"github.com/consensys/gnark/frontend"
)

type InputBuf struct {
	data      []byte
	fieldEnum ECCFieldEnum
}

func NewInputBuf(fileName string, fieldEnum ECCFieldEnum) (*InputBuf, error) {
	data, err := os.ReadFile(fileName)
	if err != nil {
		return nil, err
	}
	return &InputBuf{data: data, fieldEnum: fieldEnum}, nil
}

func (buf *InputBuf) Step(n_bytes uint) {
	buf.data = buf.data[n_bytes:]
}

func (buf *InputBuf) Len() uint {
	return uint(len(buf.data))
}

func (buf *InputBuf) ReadUint64() uint64 {
	x := binary.LittleEndian.Uint64(buf.data[:8])
	buf.Step(8)
	return x
}

func (buf *InputBuf) ReadUint() uint {
	return uint(buf.ReadUint64())
}

func (buf *InputBuf) ReadUint8() uint8 {
	x := buf.data[0]
	buf.Step(1)
	return x
}

func (buf *InputBuf) ReadField() (x *big.Int, err error) {
	fieldBytes, err := buf.fieldEnum.FieldBytes()
	if buf.Len() < fieldBytes {
		err = fmt.Errorf("Trailing bytes, proof parsing fails")
	}

	if err != nil {
		return
	}

	// little endian to big endian
	slices.Reverse(buf.data[:fieldBytes])
	x = big.NewInt(0).SetBytes(buf.data[:fieldBytes])
	buf.Step(fieldBytes)
	return
}

func (buf *InputBuf) ReadGate(inputNum uint) (gate Gate, err error) {
	iIds := make([]uint, inputNum)
	for i := uint(0); i < inputNum; i++ {
		iIds[i] = buf.ReadUint()
	}

	oId := buf.ReadUint()

	coef := Coef{RandomValue: 0}
	tempValue := big.NewInt(0)
	coef_type_u8 := buf.ReadUint8()

	switch coef_type_u8 {
	case 1:
		coef.CoefType = Constant
		tempValue, err = buf.ReadField()
		coef.Value = *tempValue
	case 2:
		coef.CoefType = Random
		// Give some default value for random,
		// the actual value should be generated by transcript
		coef.RandomValue = 1
	case 3:
		coef.CoefType = PublicInput
		coef.InputIdx = buf.ReadUint()
		if inputNum != 0 {
			err = fmt.Errorf(
				"Public input can only appear in the form of cst gate",
			)
		}
	default:
		err = fmt.Errorf("Unrecognized coef type")
	}

	if err != nil {
		return
	}

	gate = Gate{
		IIds: iIds,
		OId:  oId,
		Coef: coef,
	}
	return
}

func (buf *InputBuf) ReadAllocation() Allocation {
	return Allocation{
		IOffset: buf.ReadUint(),
		OOffset: buf.ReadUint(),
	}
}

func (buf *InputBuf) ReadChildSegInfo() ChildSegInfo {
	id := buf.ReadUint()

	allocationNum := buf.ReadUint()
	allocation := make([]Allocation, allocationNum)
	for i := uint(0); i < allocationNum; i++ {
		allocation[i] = buf.ReadAllocation()
	}

	return ChildSegInfo{
		Id:         id,
		Allocation: allocation,
	}
}

func (buf *InputBuf) ReadSegment() (segment Segment, err error) {
	i_len := buf.ReadUint()
	o_len := buf.ReadUint()

	if bits.OnesCount(i_len) != 1 || bits.OnesCount(o_len) != 1 {
		err = fmt.Errorf("Incorrect input or output length")
		return
	}

	n_child_seg := buf.ReadUint()
	var child_segs []ChildSegInfo
	for i := uint(0); i < n_child_seg; i++ {
		child_segs = append(child_segs, buf.ReadChildSegInfo())
	}

	n_muls := buf.ReadUint()
	var tempGate Gate
	var muls []Gate
	for i := uint(0); i < n_muls; i++ {
		tempGate, err = buf.ReadGate(2)
		muls = append(muls, tempGate)
	}

	n_adds := buf.ReadUint()
	var adds []Gate
	for i := uint(0); i < n_adds; i++ {
		tempGate, err = buf.ReadGate(1)
		adds = append(adds, tempGate)
	}

	n_csts := buf.ReadUint()
	var csts []Gate
	for i := uint(0); i < n_csts; i++ {
		tempGate, err = buf.ReadGate(0)
		csts = append(csts, tempGate)
	}

	if err != nil {
		return
	}

	n_customs := buf.ReadUint()
	if n_customs != 0 {
		err = fmt.Errorf("Custom gate not supported yet.")
		return
	}

	segment = Segment{
		IVarNum:    uint(bits.TrailingZeros(i_len)),
		OVarNum:    uint(bits.TrailingZeros(o_len)),
		ChildSegs:  child_segs,
		GateMuls:   muls,
		GateAdds:   adds,
		GateConsts: csts,
	}
	return
}

// detectFieldModulus reads 256 bits from the input buffer,
// take it as field modulus, and then check against its own field modulus.
// NOTE: this is used only once for circuit and witness, and it should be called
// in the beginning of the deserialization, as it would consume 256 bits in the
// buffer to retrieve expected modulus
func (buf *InputBuf) detectFieldModulus() (err error) {
	slices.Reverse(buf.data[:LEADING_FIELD_BYTES])
	fieldMod := big.NewInt(0).SetBytes(buf.data[:LEADING_FIELD_BYTES])
	buf.Step(LEADING_FIELD_BYTES)

	expectedFieldModulus, err := buf.fieldEnum.FieldModulus()
	if err != nil {
		return
	}

	if fieldMod.Cmp(expectedFieldModulus) != 0 {
		err = fmt.Errorf("Incorrect field mod detected")
	}

	return
}

func (buf *InputBuf) ReadECCCircuit() (circuit *ECCCircuit, err error) {
	version_num := buf.ReadUint()
	if version_num != VERSION_NUM {
		err = fmt.Errorf("Incorrect version of circuit serialization")
		return
	}

	if err = buf.detectFieldModulus(); err != nil {
		return
	}

	numPubInputs := buf.ReadUint()
	numOutputs := buf.ReadUint()
	expectedNumOutputZeros := buf.ReadUint()

	segmentNum := buf.ReadUint()
	segments := make([]Segment, segmentNum)
	for i := uint(0); i < segmentNum; i++ {
		if segments[i], err = buf.ReadSegment(); err != nil {
			return
		}
	}

	layerNum := buf.ReadUint()
	layerIds := make([]uint, layerNum)
	for i := uint(0); i < layerNum; i++ {
		layerIds[i] = buf.ReadUint()
	}

	circuit = &ECCCircuit{
		NumPublicInputs:        numPubInputs,
		NumOutputs:             numOutputs,
		ExpectedNumOutputZeros: expectedNumOutputZeros,

		Segments: segments,
		LayerIds: layerIds,
	}
	return
}

func (buf *InputBuf) ReadWitness() (witness *Witness, err error) {
	numWitnesses := buf.ReadUint()
	numPrivInputsPerWitness := buf.ReadUint()
	numPubInputsPerWitness := buf.ReadUint()

	totalVariables :=
		numWitnesses * (numPubInputsPerWitness + numPrivInputsPerWitness)

	if err = buf.detectFieldModulus(); err != nil {
		return
	}

	var value *big.Int
	var values []big.Int
	for i := 0; i < int(totalVariables); i++ {
		if value, err = buf.ReadField(); err != nil {
			return
		}
		values = append(values, *value)
	}

	witness = &Witness{
		NumWitnesses:               numWitnesses,
		NumPrivateInputsPerWitness: numPrivInputsPerWitness,
		NumPublicInputsPerWitness:  numPubInputsPerWitness,
		Values:                     values,
	}
	return
}

func (buf *InputBuf) ReadProof() (proof *Proof, err error) {
	var elem frontend.Variable
	elems := make([]frontend.Variable, 0)
	// TODO FIXME (HS) Raw proof deserialization part
	// TODO maybe start with a pcs deserialition?
	_ = buf.ReadUint64()
	for buf.Len() > 0 {
		if elem, err = buf.ReadField(); err != nil {
			return
		}
		elems = append(elems, elem)
	}
	proof = &Proof{
		Idx:   0,
		Elems: elems,
	}
	return
}

// CircuitRelation stands for a pair of satisfying circuit-witness together with
// the field that the circuit runs on and the MPI size
type CircuitRelation struct {
	CircuitPath string
	WitnessPath string
	FieldEnum   ECCFieldEnum
	MPISize     uint
}

// TODO:
// Verifier should not have access to the private part of witness, consider separating the witness
func ReadCircuit(circuitRel CircuitRelation) (expanderCircuit *Circuit, privInputs []PrivInput, err error) {
	circuit_input_buf, err := NewInputBuf(circuitRel.CircuitPath, circuitRel.FieldEnum)
	if err != nil {
		return
	}

	eccCircuit, err := circuit_input_buf.ReadECCCircuit()
	if err != nil {
		return
	}

	expanderCircuit = eccCircuit.Flatten()

	witnessBuf, err := NewInputBuf(circuitRel.WitnessPath, circuitRel.FieldEnum)
	if err != nil {
		return
	}
	witness, err := witnessBuf.ReadWitness()
	if err != nil {
		return
	}

	// Now the witness only takes into account the simd size
	// We're repeating the witness for each mpi
	// TODO: fix this later
	witness.NumWitnesses *= circuitRel.MPISize
	n_witness_per_mpi_node := len(witness.Values)
	for i := 1; i < int(circuitRel.MPISize); i++ {
		for j := 0; j < int(n_witness_per_mpi_node); j++ {
			witness.Values = append(witness.Values, witness.Values[j])
		}
	}

	pubInputs, privInputs := witness.ToPubPrivInputs()
	expanderCircuit.PublicInput = pubInputs

	return
}

func ReadProofFile(proofFile string, fieldEnum ECCFieldEnum) (*Proof, error) {
	proofBuf, err := NewInputBuf(proofFile, fieldEnum)
	if err != nil {
		return nil, err
	}
	return proofBuf.ReadProof()
}
