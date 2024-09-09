/// A M31x16 stores 512 bits of data.
/// With AVX it stores a single __m512i element.
/// With NEON it stores four uint32x4_t elements.
#[cfg(target_arch = "x86_64")]
cfg_if::cfg_if! {
    if #[cfg(feature = "avx256")] {
        pub type M31x16 = super::m31_avx256::AVXM31;
    } else {
        pub type M31x16 = super::m31_avx::AVXM31;
    }
}


#[cfg(target_arch = "aarch64")]
pub type M31x16 = super::m31_neon::NeonM31;

/*
use raw_cpuid::CpuId;

fn has_avx512() {
    let cpuid = CpuId::new();

    if let Some(feature_info) = cpuid.get_extended_feature_info() {
        if feature_info.has_avx512f() {
            return true;
        }
    } else {
        return false;
    }
}
*/

/*
#[cfg(target_arch = "x86_64")]
{
    #[cfg(feature = "avx256")]
    pub type M31x16 = super::m31_avx256::AVXM31;

    #[cfg(not(feature = "avx256"))]
    if has_avx512() {
        pub type M31x16 = super::m31_avx::AVXM31;
    } else {
        pub type M31x16 = super::m31_avx256::AVXM31;
    }
}
*/

