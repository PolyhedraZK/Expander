use std::{arch::aarch64::*, mem::transmute};

#[derive(Clone, Copy)]
pub struct NeonGF2_128 {
    data: uint32x4_t,
}

#[inline(always)]
fn add_internal(a: &NeonGF2_128, b: &NeonGF2_128) -> NeonGF2_128 {
    unsafe {
        NeonGF2_128 {
            data: vaddq_u32(a.data, b.data),
        }
    }
}

#[inline(always)]
fn mul_internal(a: &NeonGF2_128, b: &NeonGF2_128) -> NeonGF2_128 {
    unsafe {
        NeonGF2_128 {
            data: gfmul(a.data, b.data),
        }
    }
}

// TODO: Is there an instruction for this?
unsafe fn shuffle_epi32_neon_147(input: uint32x4_t) -> uint32x4_t {
    let a = vgetq_lane_u32(input, 0);
    let b = vgetq_lane_u32(input, 1);
    let c = vgetq_lane_u32(input, 2);
    let d = vgetq_lane_u32(input, 3);

    let res = transmute(0u128);
    let res = vsetq_lane_u32(a, res, 1);
    let res = vsetq_lane_u32(b, res, 2);
    let res = vsetq_lane_u32(c, res, 3);
    let res = vsetq_lane_u32(d, res, 0);
    res
}

// TODO: Is there an instruction for this?
unsafe fn shuffle_epi32_neon_78(input: uint32x4_t) -> uint32x4_t {
    let a = vgetq_lane_u32(input, 0);
    let b = vgetq_lane_u32(input, 1);
    let c = vgetq_lane_u32(input, 2);
    let d = vgetq_lane_u32(input, 3);

    let res = transmute(0u128);
    let res = vsetq_lane_u32(a, res, 2);
    let res = vsetq_lane_u32(b, res, 3);
    let res = vsetq_lane_u32(c, res, 0);
    let res = vsetq_lane_u32(d, res, 1);
    res
}

unsafe fn gfmul(a: uint32x4_t, b: uint32x4_t) -> uint32x4_t {
    let xmm_mask = transmute([0xffffffffu32, 0, 0, 0]);

    // a = a0|a1, b = b0|b1
    println!("{:?}", a);
    println!("{:?}", b);
    let a64 = vreinterpretq_u64_u32(a);
    let b64 = vreinterpretq_u64_u32(b);

    println!("{:?}", a64);
    println!("{:?}", b64);

    let tmp3 = transmute::<_, uint64x2_t>(vmull_p64(
        vget_lane_u64(vget_low_u64(a64), 0),
        vget_lane_u64(vget_low_u64(b64), 0),
    )); // tmp3 = a0 * b0

    println!("tmp3: {:?} {:?}", tmp3, transmute::<_, u128>(tmp3));

    let tmp6 = transmute::<_, uint64x2_t>(vmull_p64(
        vget_lane_u64::<0>(vget_high_u64(a64)),
        vget_lane_u64::<0>(vget_high_u64(b64)),
    )); // tmp6 = a1 * b1

    println!("tmp6: {:?} {:?}", tmp6, transmute::<_, u128>(tmp6));

    // let tmp4 = transmute::<_, uint32x4_t>(
    //     ((vgetq_lane_u64(a64, 0) | vgetq_lane_u64(a64, 1)) as u128) << 64,
    // );
    // let tmp5 =
    //     transmute::<_, uint32x4_t>((vgetq_lane_u64(b64, 0) | vgetq_lane_u64(b64, 1)) as u128);

    let tmp4 = shuffle_epi32_neon_78(a);
    let tmp5 = shuffle_epi32_neon_78(b);

    // let tmp4 = vrev64q_u32(a); // tmp4 = a1|a0
    // let tmp5 = vrev64q_u32(b); // tmp5 = b1|b0

    // and => mul
    // xor => add

    println!("tmp4 shuffled: {:?} {:?}", tmp4, transmute::<_, u128>(tmp4));
    println!("tmp5 shuffled: {:?} {:?}", tmp5, transmute::<_, u128>(tmp5));
    println!();
    let tmp4 = veorq_u32(tmp4, a); // tmp4 = (a0 + a1) | (a0 + a1)
    let tmp5 = veorq_u32(tmp5, b); // tmp5 = (b0 + b1) | (b0 + b1)

    println!("tmp4: {:?} {:?}", tmp4, transmute::<_, u128>(tmp4));
    println!("tmp5: {:?} {:?}", tmp5, transmute::<_, u128>(tmp5));

    let tmp4_64 = transmute::<_, uint64x2_t>(vmull_p64(
        vget_lane_u64::<0>(vreinterpret_u64_u32(vget_low_u32(tmp4))),
        vget_lane_u64::<0>(vreinterpret_u64_u32(vget_low_u32(tmp5))),
    )); // tmp4 = (a0 + a1) * (b0 + b1)

    println!("tmp4_64 {:?}", tmp4_64);

    let tmp4_64 = veorq_u64(tmp4_64, tmp3); // tmp4 = (a0 + a1) * (b0 + b1) - a0 * b0

    println!("tmp4 {:?}", tmp4_64);

    let tmp4_64 = veorq_u64(tmp4_64, tmp6); // tmp4 = (a0 + a1) * (b0 + b1) - a0 * b0 - a1 * b1 = a0 * b1 + a1 * b0

    println!("tmp4 before shift {:?}", tmp4_64);

    // let tmp5_shifted_left = vshlq_n_u64(tmp4_64, 8);
    // probably not correct
    // let tmp5_shifted_left = vcombine_u64(vget_high_u64(tmp4_64),vget_low_u64(tmp4_64));
    let tmp5_shifted_left = transmute(transmute::<_, u128>(tmp4_64) << 64);

    let tmp4_64 = transmute(transmute::<_, u128>(tmp4_64) >> 64);
    let tmp3 = veorq_u64(tmp3, tmp5_shifted_left);
    let tmp6 = veorq_u64(tmp6, tmp4_64);

    println!(
        "tmp5_shifted_left: {:?} {:?}",
        tmp5_shifted_left,
        transmute::<_, u128>(tmp5_shifted_left)
    );
    println!("tmp4_64: {:?}", tmp4_64);
    println!();
    println!("tmp3: {:?}", tmp3);
    println!("tmp6: {:?}", tmp6);

    let tmp7 = vshrq_n_u32(vreinterpretq_u32_u64(tmp6), 31);
    let tmp8 = vshrq_n_u32(vreinterpretq_u32_u64(tmp6), 30);
    let tmp9 = vshrq_n_u32(vreinterpretq_u32_u64(tmp6), 25);

    println!("tmp7: {:?}", tmp7);
    println!("tmp9: {:?}", tmp9);
    println!("tmp7: {:?}", tmp7);

    let tmp7 = veorq_u32(tmp7, tmp8);
    let tmp7 = veorq_u32(tmp7, tmp9);

    println!("tmp7: {:?}", tmp7);
    println!("tmp7: {:?}", tmp7);

    // let tmp8 = vrev64q_u32(tmp7);
    let tmp8 = shuffle_epi32_neon_147(tmp7);
    println!("shuffled tmp8: {:?}", transmute::<_, uint64x2_t>(tmp8));
    println!();

    let tmp7 = vandq_u32(xmm_mask, tmp8);
    let tmp8 = vbicq_u32(tmp8, xmm_mask);

    println!("tmp7: {:?}", tmp7);
    println!("tmp8: {:?}", tmp8);

    let tmp3 = veorq_u64(tmp3, vreinterpretq_u64_u32(tmp8));
    let tmp6 = veorq_u64(tmp6, vreinterpretq_u64_u32(tmp7));

    println!("tmp3: {:?}", tmp3);
    println!("tmp6: {:?}", tmp6);

    let tmp10 = vshlq_n_u32(transmute(tmp6), 1);

    let tmp3 = veorq_u64(tmp3, vreinterpretq_u64_u32(tmp10));

    println!("tmp10: {:?}", tmp10);
    println!("tmp3: {:?}", tmp3);

    let tmp11 = vshlq_n_u32(vreinterpretq_u32_u64(tmp6), 2);
    let tmp3 = veorq_u64(tmp3, vreinterpretq_u64_u32(tmp11));

    println!("tmp11: {:?}", tmp11);
    println!("tmp3: {:?}", tmp3);

    let tmp12 = vshlq_n_u32(vreinterpretq_u32_u64(tmp6), 7);
    let tmp3 = veorq_u64(tmp3, vreinterpretq_u64_u32(tmp12));

    println!("tmp12: {:?}", tmp12);
    println!("tmp3: {:?}", tmp3);

    let res = transmute::<_, uint32x4_t>(veorq_u64(tmp3, tmp6));

    println!("res: {:?}", res);

    res
}

// todo: move those tests to a separate file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gfmul_zero() {
        unsafe {
            let a = vdupq_n_u32(0);
            let b = vcombine_u32(vcreate_u32(1234567890), vcreate_u32(0));
            let result = gfmul(a, b);
            assert_eq!(vgetq_lane_u32(result, 0), 0);
            assert_eq!(vgetq_lane_u32(result, 1), 0);
        }
    }

    #[test]
    fn test_gfmul_one() {
        unsafe {
            // {
            //     let zero = transmute(0u128);
            // let a = transmute((3u128 << 64) + 5);
            //     let result = gfmul(a, zero);
            //     assert_eq!(vgetq_lane_u32(result, 0), 0);
            //     assert_eq!(vgetq_lane_u32(result, 1), 0);
            //     assert_eq!(vgetq_lane_u32(result, 2), 0);
            //     assert_eq!(vgetq_lane_u32(result, 3), 0);
            // }

            // {

            //     let one = transmute(1u128);
            // let a = transmute((3u128 << 64) + 5);
            //     let result = gfmul(one, a);
            //     println!("{:?}", result);
            //     println!("{:?}", transmute::<_, u128>(result));
            //     assert_eq!(vgetq_lane_u32(result, 0), 5);
            //     assert_eq!(vgetq_lane_u32(result, 1), 0);
            //     assert_eq!(vgetq_lane_u32(result, 2), 3);
            //     assert_eq!(vgetq_lane_u32(result, 3), 0);
            // }

            // {

            // let a = transmute((3u128 << 64) + 5);

            // let b =  transmute((1u128 << 64) + 7);
            //     let result = gfmul(a, b);
            //     println!("{:?}", result);
            //     println!("{:?}", transmute::<_, u128>(result));
            //     assert_eq!(vgetq_lane_u32(result, 0), 402);
            //     assert_eq!(vgetq_lane_u32(result, 1), 0);
            //     assert_eq!(vgetq_lane_u32(result, 2), 12);
            //     assert_eq!(vgetq_lane_u32(result, 3), 0);
            // }

            // {

            //     let b =  transmute((1u128 << 64) + 7);

            // let c = transmute((1u128<<96) + (1<<64) + (1<<32) + 1 );

            //     let result = gfmul(b, c);
            //     println!("{:?}", result);
            //     println!("{:?}", transmute::<_, u128>(result));
            //     assert_eq!(vgetq_lane_u32(result, 0), 128);
            //     assert_eq!(vgetq_lane_u32(result, 1), 128);
            //     assert_eq!(vgetq_lane_u32(result, 2), 6);
            //     assert_eq!(vgetq_lane_u32(result, 3), 6);
            // }

            // {
            //     let a = transmute::<_,uint32x4_t>([7u8; 16]);
            //     let b = transmute::<_,uint32x4_t>([5u8; 16]);

            //     let result = gfmul(a, b, );

            //     println!("{:?}", result);
            //     println!("{:?}", transmute::<_, u128>(result));
            //     assert_eq!(vgetq_lane_u32(result, 0), 232394202);
            //     assert_eq!(vgetq_lane_u32(result, 1), 232394202);
            //     assert_eq!(vgetq_lane_u32(result, 2), 232394202);
            //     assert_eq!(vgetq_lane_u32(result, 3), 232394202);
            // }

            {
                let mut a = [6u8; 16];
                a[8] = 0;
                let a = transmute::<_, uint32x4_t>(a);
                let mut b = [5u8; 16];
                b[4] = 1;
                let b = transmute::<_, uint32x4_t>(b);

                let result = gfmul(a, b);

                println!("{:?}", result);
                println!("{:?}", transmute::<_, u128>(result));
                assert_eq!(vgetq_lane_u32(result, 0), 508894806);
                assert_eq!(vgetq_lane_u32(result, 1), 1107902981);
                assert_eq!(vgetq_lane_u32(result, 2), 155322701);
                assert_eq!(vgetq_lane_u32(result, 3), 155322714);
            }
        }
    }
}
