// mod bn254;
mod field;
mod m31;
// mod m31_ext;

#[cfg(target_arch = "x86_64")]
#[test]
fn test_mm256_const_init() {
    use std::arch::x86_64::*;
    use std::mem::transmute;

    let all_equal = unsafe {
        let x = _mm256_set1_epi32(1);
        let y = transmute::<_, __m256i>([1, 1, 1, 1, 1, 1, 1, 1]);
        let cmp = _mm256_cmpeq_epi32(x, y);
        _mm256_testc_si256(cmp, _mm256_set1_epi32(-1))
    };

    assert!(all_equal != 0, "x and y are not equal");
}
