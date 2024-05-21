#[cfg(target_arch = "x86_64")]
#[test]
fn test_mm256_const_init() {
    use std::arch::x86_64::*;
    use std::mem::transmute;

    let x = unsafe { _mm256_set1_epi32(1) };
    println!("{:?}", x);
    pub const y: __m256i = unsafe { transmute([1, 1, 1, 1, 1, 1, 1, 1]) };
    println!("{:?}", y);
}
