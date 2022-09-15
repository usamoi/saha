use core_simd::simd::*;

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Pext {}
}

pub struct Pext;

pub trait SupportedPext<const LANES: usize>: sealed::Sealed
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn pext(mask: Mask<i8, LANES>) -> Simd<u8, LANES>;
}

impl SupportedPext<1> for Pext {
    #[inline(always)]
    fn pext(_: Mask<i8, 1>) -> Simd<u8, 1> {
        Simd::splat(0)
    }
}

impl SupportedPext<2> for Pext {
    #[inline(always)]
    #[cfg(target_feature = "bmi2")]
    fn pext(i: Mask<i8, 2>) -> Simd<u8, 2> {
        unsafe {
            use std::arch::x86_64::_pext_u32;
            let a = std::mem::transmute::<_, u16>((!i).cast::<i8>());
            let b = _pext_u32(0x0100, a as u32) as u16;
            let c = std::mem::transmute::<_, Simd<u8, 2>>(b);
            c
        }
    }
}

impl SupportedPext<4> for Pext {
    #[inline(always)]
    #[cfg(target_feature = "bmi2")]
    fn pext(i: Mask<i8, 4>) -> Simd<u8, 4> {
        unsafe {
            use std::arch::x86_64::_pext_u32;
            let a = std::mem::transmute::<_, u32>((!i).cast::<i8>());
            let b = _pext_u32(0x03020100, a);
            let c = std::mem::transmute::<_, Simd<u8, 4>>(b);
            c
        }
    }
}

impl SupportedPext<8> for Pext {
    #[inline(always)]
    #[cfg(target_feature = "bmi2")]
    fn pext(i: Mask<i8, 8>) -> Simd<u8, 8> {
        unsafe {
            use std::arch::x86_64::_pext_u64;
            let a = std::mem::transmute::<_, u64>((!i).cast::<i8>());
            let b = _pext_u64(0x0706050403020100, a);
            let c = std::mem::transmute::<_, Simd<u8, 8>>(b);
            c
        }
    }
}
