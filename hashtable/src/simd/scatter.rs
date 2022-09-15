use core_simd::simd::*;

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Scatter {}
}

pub struct Scatter;

pub trait SupportedScatter<T, const LANES: usize>: sealed::Sealed
where
    T: SimdElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    #[inline(always)]
    unsafe fn scatter(ptr: *mut T, idxs: Simd<u32, LANES>, simd: Simd<T, LANES>) {
        Simd::scatter_select_unchecked(
            simd,
            std::slice::from_raw_parts_mut(ptr, 0),
            Mask::splat(true),
            idxs.cast(),
        );
    }

    #[inline(always)]
    unsafe fn mask_scatter(
        ptr: *mut T,
        mask: Mask<T::Mask, LANES>,
        idxs: Simd<u32, LANES>,
        simd: Simd<T, LANES>,
    ) {
        Simd::scatter_select_unchecked(
            simd,
            std::slice::from_raw_parts_mut(ptr, 0),
            mask.cast(),
            idxs.cast(),
        );
    }
}

impl<T: SimdElement, const LANES: usize> SupportedScatter<T, LANES> for Scatter where
    LaneCount<LANES>: SupportedLaneCount
{
}
