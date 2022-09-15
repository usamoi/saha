use core_simd::simd::*;

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Gather {}
}

pub struct Gather;

pub trait SupportedGather<T, const LANES: usize>: sealed::Sealed
where
    T: SimdElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    unsafe fn gather(ptr: *const T, idxs: Simd<u32, LANES>) -> Simd<T, LANES>;

    unsafe fn mask_gather(
        ptr: *const T,
        mask: Mask<T::Mask, LANES>,
        idxs: Simd<u32, LANES>,
        or: Simd<T, LANES>,
    ) -> Simd<T, LANES>;
}

impl<T> SupportedGather<T, 1> for Gather
where
    T: SimdElement,
{
    #[inline(always)]
    unsafe fn gather(ptr: *const T, idxs: Simd<u32, 1>) -> Simd<T, 1> {
        Simd::splat(*ptr.add(idxs[0] as usize))
    }

    #[inline(always)]
    unsafe fn mask_gather(
        ptr: *const T,
        mask: Mask<T::Mask, 1>,
        idxs: Simd<u32, 1>,
        or: Simd<T, 1>,
    ) -> Simd<T, 1> {
        if mask.test(0) {
            Simd::splat(*ptr.add(idxs[0] as usize))
        } else {
            or
        }
    }
}

#[cfg(target_feature = "avx2")]
impl SupportedGather<i32, 8> for Gather {
    #[inline(always)]
    unsafe fn gather(ptr: *const i32, idxs: Simd<u32, 8>) -> Simd<i32, 8> {
        use std::arch::x86_64::*;
        _mm256_i32gather_epi32::<4>(ptr, idxs.into()).into()
    }

    #[inline(always)]
    unsafe fn mask_gather(
        ptr: *const i32,
        mask: Mask<i32, 8>,
        idxs: Simd<u32, 8>,
        or: Simd<i32, 8>,
    ) -> Simd<i32, 8> {
        use std::arch::x86_64::*;
        _mm256_mask_i32gather_epi32::<4>(or.into(), ptr, idxs.into(), mask.to_int().into()).into()
    }
}

#[cfg(target_feature = "avx2")]
impl SupportedGather<u32, 8> for Gather {
    #[inline(always)]
    unsafe fn gather(ptr: *const u32, idxs: Simd<u32, 8>) -> Simd<u32, 8> {
        <Gather as SupportedGather<i32, 8>>::gather(ptr as _, idxs).cast()
    }

    #[inline(always)]
    unsafe fn mask_gather(
        ptr: *const u32,
        mask: Mask<i32, 8>,
        idxs: Simd<u32, 8>,
        or: Simd<u32, 8>,
    ) -> Simd<u32, 8> {
        <Gather as SupportedGather<i32, 8>>::mask_gather(ptr as _, mask, idxs, or.cast()).cast()
    }
}

#[cfg(target_feature = "avx2")]
impl SupportedGather<i64, 4> for Gather {
    #[inline(always)]
    unsafe fn gather(ptr: *const i64, idxs: Simd<u32, 4>) -> Simd<i64, 4> {
        use std::arch::x86_64::*;
        _mm256_i32gather_epi64::<8>(ptr, idxs.into()).into()
    }

    #[inline(always)]
    unsafe fn mask_gather(
        ptr: *const i64,
        mask: Mask<i64, 4>,
        idxs: Simd<u32, 4>,
        or: Simd<i64, 4>,
    ) -> Simd<i64, 4> {
        use std::arch::x86_64::*;
        _mm256_mask_i32gather_epi64::<8>(or.into(), ptr, idxs.into(), mask.to_int().into()).into()
    }
}

#[cfg(target_feature = "avx2")]
impl SupportedGather<u64, 4> for Gather {
    #[inline(always)]
    unsafe fn gather(ptr: *const u64, idxs: Simd<u32, 4>) -> Simd<u64, 4> {
        <Gather as SupportedGather<i64, 4>>::gather(ptr as _, idxs).cast()
    }

    #[inline(always)]
    unsafe fn mask_gather(
        ptr: *const u64,
        mask: Mask<i64, 4>,
        idxs: Simd<u32, 4>,
        or: Simd<u64, 4>,
    ) -> Simd<u64, 4> {
        <Gather as SupportedGather<i64, 4>>::mask_gather(ptr as _, mask, idxs, or.cast()).cast()
    }
}

#[cfg(target_feature = "avx2")]
impl SupportedGather<f32, 8> for Gather {
    #[inline(always)]
    unsafe fn gather(ptr: *const f32, idxs: Simd<u32, 8>) -> Simd<f32, 8> {
        use std::arch::x86_64::*;
        _mm256_i32gather_ps::<4>(ptr, idxs.into()).into()
    }

    #[inline(always)]
    unsafe fn mask_gather(
        ptr: *const f32,
        mask: Mask<i32, 8>,
        idxs: Simd<u32, 8>,
        or: Simd<f32, 8>,
    ) -> Simd<f32, 8> {
        use std::arch::x86_64::*;
        _mm256_mask_i32gather_ps::<4>(
            or.into(),
            ptr,
            idxs.into(),
            std::mem::transmute(__m256i::from(mask.to_int())),
        )
        .into()
    }
}

#[cfg(target_feature = "avx2")]
impl SupportedGather<f64, 4> for Gather {
    #[inline(always)]
    unsafe fn gather(ptr: *const f64, idxs: Simd<u32, 4>) -> Simd<f64, 4> {
        use std::arch::x86_64::*;
        _mm256_i32gather_pd::<8>(ptr, idxs.into()).into()
    }

    #[inline(always)]
    unsafe fn mask_gather(
        ptr: *const f64,
        mask: Mask<i64, 4>,
        idxs: Simd<u32, 4>,
        or: Simd<f64, 4>,
    ) -> Simd<f64, 4> {
        use std::arch::x86_64::*;
        _mm256_mask_i32gather_pd::<8>(
            or.into(),
            ptr,
            idxs.into(),
            std::mem::transmute(__m256i::from(mask.to_int())),
        )
        .into()
    }
}
