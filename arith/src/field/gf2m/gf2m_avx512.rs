use std::arch::x86_64::*;

#[derive(Clone, Copy)]
pub struct AVXGF2M {
    pub v: [__m512i],
}