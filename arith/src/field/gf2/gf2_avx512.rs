use std::arch::x86_64::__m512i;

#[derive(Clone, Copy)]
pub struct AVX512GF2 {
    pub v: __m512i,
}

// We don't need this field
