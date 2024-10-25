package circuit

type Allocation struct {
	IOffset uint
	OOffset uint
}

type ChildSegInfo struct {
	Id         uint
	Allocation []Allocation
}

type Segment struct {
	IVarNum    uint
	OVarNum    uint
	ChildSegs  []ChildSegInfo
	GateMuls   []Gate
	GateAdds   []Gate
	GateConsts []Gate
	// TODO: Support custom gate
}

type ECCCircuit struct {
	NumPublicInputs        uint
	NumOutputs             uint
	ExpectedNumOutputZeros uint

	Segments []Segment
	LayerIds []uint
}

func (segment *Segment) insert_gates(muls *[]Gate, adds *[]Gate, csts *[]Gate, i_offset uint, o_offset uint) {
	for i := 0; i < len(segment.GateMuls); i++ {
		mul_gate := segment.GateMuls[i]

		i_0 := mul_gate.IIds[0] + i_offset
		i_1 := mul_gate.IIds[1] + i_offset
		o := mul_gate.OId + o_offset

		*muls = append(*muls,
			Gate{
				IIds: []uint{i_0, i_1},
				OId:  o,
				Coef: mul_gate.Coef,
			},
		)
	}

	for i := 0; i < len(segment.GateAdds); i++ {
		add_gate := segment.GateAdds[i]
		i_0 := add_gate.IIds[0] + i_offset
		o := add_gate.OId + o_offset

		*adds = append(*adds,
			Gate{
				IIds: []uint{i_0},
				OId:  o,
				Coef: add_gate.Coef,
			},
		)
	}

	for i := 0; i < len(segment.GateConsts); i++ {
		cst_gate := segment.GateConsts[i]
		*csts = append(*csts,
			Gate{
				IIds: make([]uint, 0),
				OId:  cst_gate.OId + o_offset,
				Coef: cst_gate.Coef,
			},
		)
	}
}

// Return mul, add, cst gates
func (segment *Segment) FlattenInto(
	all_segments []Segment,
	i_offset uint,
	o_offset uint,
	muls *[]Gate,
	adds *[]Gate,
	csts *[]Gate,
) {
	segment.insert_gates(muls, adds, csts, i_offset, o_offset)
	for i := 0; i < len(segment.ChildSegs); i++ {
		child_seg_info := segment.ChildSegs[i]
		child_seg := &all_segments[child_seg_info.Id]
		for j := 0; j < len(child_seg_info.Allocation); j++ {
			alloc := child_seg_info.Allocation[j]
			child_seg.FlattenInto(
				all_segments,
				alloc.IOffset+i_offset,
				alloc.OOffset+o_offset,
				muls,
				adds,
				csts,
			)
		}
	}
}

func (ecc_circuit *ECCCircuit) Flatten() *Circuit {
	var ret_circuit Circuit
	ret_circuit.ExpectedNumOutputZeros = ecc_circuit.ExpectedNumOutputZeros

	all_segments := ecc_circuit.Segments
	for i := 0; i < len(ecc_circuit.LayerIds); i++ {
		layer_id := ecc_circuit.LayerIds[i]
		cur_segment := &all_segments[layer_id]

		var muls []Gate
		var adds []Gate
		var csts []Gate
		cur_segment.FlattenInto(
			all_segments,
			0,
			0,
			&muls,
			&adds,
			&csts,
		)

		ret_circuit.Layers = append(ret_circuit.Layers,
			Layer{
				InputLenLog:  max(cur_segment.IVarNum, 1),
				OutputLenLog: max(cur_segment.OVarNum, 1),

				Cst: csts,
				Add: adds,
				Mul: muls,

				StructureInfo: StructureInfo{len(muls) == 0},
			},
		)
	}

	return &ret_circuit
}
