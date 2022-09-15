use core_simd::simd::Simd;
use core_simd::simd::SimdElement;

// It's not supported by portable_simd.
// https://github.com/rust-lang/portable-simd/issues/242
pub trait DynamicSwizzle {
    type I;

    fn dynamic_swizzle(self, index: Self::I) -> Self;
}

macro_rules! dynamic_swizzle_proxy {
    ($surface: ty, $underlaying: ty) => {
        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 2>
        where
            Simd<$underlaying, 2>: DynamicSwizzle<I = Simd<u8, 2>>,
        {
            type I = Simd<u8, 2>;

            #[inline(always)]
            fn dynamic_swizzle(self, index: Self::I) -> Self {
                self.cast::<$underlaying>().dynamic_swizzle(index).cast()
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 4>
        where
            Simd<$underlaying, 4>: DynamicSwizzle<I = Simd<u8, 4>>,
        {
            type I = Simd<u8, 4>;

            #[inline(always)]
            fn dynamic_swizzle(self, index: Self::I) -> Self {
                self.cast::<$underlaying>().dynamic_swizzle(index).cast()
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 8>
        where
            Simd<$underlaying, 8>: DynamicSwizzle<I = Simd<u8, 8>>,
        {
            type I = Simd<u8, 8>;

            #[inline(always)]
            fn dynamic_swizzle(self, index: Self::I) -> Self {
                self.cast::<$underlaying>().dynamic_swizzle(index).cast()
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 16>
        where
            Simd<$underlaying, 16>: DynamicSwizzle<I = Simd<u8, 16>>,
        {
            type I = Simd<u8, 16>;

            #[inline(always)]
            fn dynamic_swizzle(self, index: Self::I) -> Self {
                self.cast::<$underlaying>().dynamic_swizzle(index).cast()
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 32>
        where
            Simd<$underlaying, 32>: DynamicSwizzle<I = Simd<u8, 32>>,
        {
            type I = Simd<u8, 32>;

            #[inline(always)]
            fn dynamic_swizzle(self, index: Self::I) -> Self {
                self.cast::<$underlaying>().dynamic_swizzle(index).cast()
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 64>
        where
            Simd<$underlaying, 64>: DynamicSwizzle<I = Simd<u8, 64>>,
        {
            type I = Simd<u8, 64>;

            #[inline(always)]
            fn dynamic_swizzle(self, index: Self::I) -> Self {
                self.cast::<$underlaying>().dynamic_swizzle(index).cast()
            }
        }
    };
}

dynamic_swizzle_proxy!(usize, isize);
dynamic_swizzle_proxy!(u8, i8);
dynamic_swizzle_proxy!(u16, i16);
dynamic_swizzle_proxy!(u32, i32);
dynamic_swizzle_proxy!(u64, i64);
#[cfg(target_pointer_width = "32")]
dynamic_swizzle_proxy!(isize, i32);
#[cfg(target_pointer_width = "64")]
dynamic_swizzle_proxy!(isize, i64);

impl<T: SimdElement> DynamicSwizzle for Simd<T, 1> {
    type I = Simd<u8, 1>;

    #[inline(always)]
    fn dynamic_swizzle(self, _index: Self::I) -> Self {
        self
    }
}

impl DynamicSwizzle for Simd<i32, 2> {
    type I = Simd<u8, 2>;

    #[inline(always)]
    fn dynamic_swizzle(self, index: Self::I) -> Self {
        let mut result = Simd::<i32, 2>::splat(0);
        for i in 0..2 {
            result[i] = self[index[i] as usize];
        }
        result
    }
}

impl DynamicSwizzle for Simd<i32, 4> {
    type I = Simd<u8, 4>;

    #[inline(always)]
    #[cfg(target_feature = "avx2")]
    fn dynamic_swizzle(self, index: Self::I) -> Self {
        unsafe {
            use std::arch::x86_64::_mm_permutevar_ps;
            use std::arch::x86_64::{__m128, __m128i};
            let a = std::mem::transmute::<_, __m128>(__m128i::from(self));
            let b = __m128i::from(index.cast::<u32>());
            let c = std::mem::transmute::<_, __m128i>(_mm_permutevar_ps(a, b));
            Simd::from(c)
        }
    }
}

impl DynamicSwizzle for Simd<i32, 8> {
    type I = Simd<u8, 8>;

    #[inline(always)]
    #[cfg(target_feature = "avx2")]
    fn dynamic_swizzle(self, index: Self::I) -> Self {
        unsafe {
            use std::arch::x86_64::__m256i;
            use std::arch::x86_64::_mm256_permutevar8x32_epi32;
            let a = __m256i::from(self);
            let b = __m256i::from(index.cast::<u32>());
            let c = _mm256_permutevar8x32_epi32(a, b);
            Simd::from(c)
        }
    }
}

impl DynamicSwizzle for Simd<i64, 2> {
    type I = Simd<u8, 2>;

    #[inline(always)]
    fn dynamic_swizzle(self, index: Self::I) -> Self {
        let mut result = Simd::<i64, 2>::splat(0);
        for i in 0..2 {
            result[i] = self[index[i] as usize];
        }
        result
    }
}

impl DynamicSwizzle for Simd<i64, 4> {
    type I = Simd<u8, 4>;

    #[inline(always)]
    #[cfg(target_feature = "avx2")]
    fn dynamic_swizzle(self, index: Self::I) -> Self {
        unsafe {
            use std::arch::x86_64::__m256i;
            use std::arch::x86_64::_mm256_permutevar8x32_epi32;
            let a = __m256i::from(self);
            let b = index.cast::<i64>();
            let c = b * Simd::splat(0x200000002) + Simd::splat(0x100000000);
            let d = __m256i::from(c);
            let e = _mm256_permutevar8x32_epi32(a, d);
            Simd::from(e)
        }
    }
}

impl DynamicSwizzle for Simd<i64, 8> {
    type I = Simd<u8, 8>;

    #[inline(always)]
    fn dynamic_swizzle(self, index: Self::I) -> Self {
        let mut result = Simd::<i64, 8>::splat(0);
        for i in 0..8 {
            result[i] = self[index[i] as usize];
        }
        result
    }
}
