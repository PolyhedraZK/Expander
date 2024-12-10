package circuit

import (
	"fmt"
	"math/big"

	ecc_bn254 "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/field/bn254"
	ecc_gf2 "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/field/gf2"
	ecc_m31 "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/field/m31"
)

type ECCFieldEnum uint

const (
	ECCBN254 ECCFieldEnum = iota
	ECCM31
	ECCGF2
)

func (f ECCFieldEnum) FieldModulus() (modulus *big.Int, err error) {
	switch f {
	case ECCBN254:
		modulus = ecc_bn254.ScalarField
	case ECCM31:
		modulus = ecc_m31.Pbig
	case ECCGF2:
		modulus = ecc_gf2.Pbig
	default:
		err = fmt.Errorf(`Unknown ECC Field Enum "%d"`, f)
	}
	return
}

func (f ECCFieldEnum) FieldBytes() (field_bytes uint, err error) {
	var fieldModulus *big.Int

	fieldModulus, err = f.FieldModulus()
	if err != nil {
		return
	}

	bitLen := fieldModulus.BitLen()
	// NOTE: round up against bit-byte rate
	field_bytes = (uint(bitLen) + 8 - 1) / 8
	return
}
