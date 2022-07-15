use crate::table0::Slot;
use crate::table0::Table0;
use crate::traits::BatchKey;
use crate::traits::Key;
use arrayvec::ArrayVec;
use core_simd::simd::simd_swizzle;
use core_simd::simd::LaneCount;
use core_simd::simd::Mask;
use core_simd::simd::Simd;
use core_simd::simd::SimdElement;
use core_simd::simd::SimdPartialEq;
use core_simd::simd::SupportedLaneCount;
use core_simd::ToBitMask;
use memoffset::offset_of;

pub struct BatchUpdater<'a, const LANES: usize, K, V, D, F, G>
where
    K: Key,
{
    pub(crate) table: &'a mut Table0<K, V>,
    pub(crate) keys: ArrayVec<K, 2048>,
    pub(crate) dels: ArrayVec<D, 2048>,
    pub(crate) insert: F,
    pub(crate) update: G,
}

impl<'a, const LANES: usize, K, V, D, F, G> BatchUpdater<'a, LANES, K, V, D, F, G>
where
    K: Key,
{
    pub(crate) unsafe fn new(table: &'a mut Table0<K, V>, f: F, g: G) -> Self {
        Self {
            table,
            keys: ArrayVec::new(),
            dels: ArrayVec::new(),
            insert: f,
            update: g,
        }
    }
}

impl<'a, const LANES: usize, K, V, D, F, G> BatchUpdater<'a, LANES, K, V, D, F, G>
where
    LaneCount<LANES>: SupportedLaneCount,
    Operations: SupportedOperations<LANES>,
    K: BatchKey,
    V: Copy + SimdElement + Default,
    D: Copy + SimdElement + Default,
    F: Fn(D) -> V,
    G: Fn(V, D) -> V,
    Simd<usize, LANES>: DynamicSwizzle<I = [u32; LANES]>,
    Simd<K, LANES>: DynamicSwizzle<I = [u32; LANES]>,
    Simd<K, LANES>: SimdPartialEq<Mask = Mask<K::Mask, LANES>>,
    Mask<K::Mask, LANES>: ToBitMask<BitMask = u8>,
    Simd<D, LANES>: DynamicSwizzle<I = [u32; LANES]>,
{
    pub fn push(&mut self, k: K, d: D) {
        if Key::equals_zero(&k) {
            if (1 + self.table.len()) * 2 > self.table.capacity() {
                self.table.grow();
            }
            unsafe {
                let result = self.table.insert(k);
                match result {
                    Ok(x) => {
                        x.write((self.insert)(d));
                    }
                    Err(x) => {
                        *x = (self.update)(*x, d);
                    }
                }
            }
        } else {
            if self.keys.is_full() {
                self.flush();
            }
            self.keys.push(k);
            self.dels.push(d);
        }
    }
    pub fn flush(&mut self) {
        while (self.keys.len() + self.table.len()) * 2 > self.table.capacity() {
            self.table.grow();
        }
        let n = self.table.slots.len();
        let m = self.keys.len();
        let offset_keys = offset_of!(Slot<K, V>, key);
        let offset_vals = offset_of!(Slot<K, V>, val);
        let data_keys =
            unsafe { (self.table.slots.as_mut().as_mut_ptr() as *mut u8).add(offset_keys) };
        let data_vals =
            unsafe { (self.table.slots.as_mut().as_mut_ptr() as *mut u8).add(offset_vals) };
        // Gathering with a scale helps a lot.
        // It seems there is no issue tracking for it.
        let table_keys = unsafe { std::slice::from_raw_parts_mut(data_keys as *mut K, 0) };
        let table_vals = unsafe { std::slice::from_raw_parts_mut(data_vals as *mut V, 0) };
        let scale = Simd::splat(std::mem::size_of::<Slot<K, V>>() / 8);
        let input_keys = self.keys.as_ref();
        let input_dels = self.dels.as_ref();
        let mut idx: Simd<usize, LANES> = Simd::default();
        let mut key: Simd<K, LANES> = Simd::default();
        let mut del: Simd<D, LANES> = Simd::default();
        let mut mask: Mask<<K as SimdElement>::Mask, LANES> = Mask::splat(true);
        let mut i = 0usize;
        unsafe {
            while i + LANES <= m {
                // `expand load` is needed here, but it's not supported by portable_simd.
                // https://github.com/rust-lang/portable-simd/issues/240
                idx = DynamicSwizzle::dynamic_swizzle(idx, Operations::reorder(mask.to_bitmask()));
                key = DynamicSwizzle::dynamic_swizzle(key, Operations::reorder(mask.to_bitmask()));
                del = DynamicSwizzle::dynamic_swizzle(del, Operations::reorder(mask.to_bitmask()));
                let t = LANES - mask.to_bitmask().count_ones() as usize;
                mask = Mask::from_bitmask(!((1 << t) - 1));
                let ptr_key = input_keys[i - t..].as_ptr() as *const Simd<K, LANES>;
                let ptr_del = input_dels[i - t..].as_ptr() as *const Simd<D, LANES>;
                key = mask.select(ptr_key.read_unaligned(), key);
                del = mask.cast().select(ptr_del.read_unaligned(), del);
                idx = mask.cast().select(
                    (map(|x| Key::hash(&x), key).cast() & Simd::splat(n - 1)) * scale,
                    idx,
                );
                i += mask.to_bitmask().count_ones() as usize;
                let result = Simd::gather_select_unchecked(
                    table_keys,
                    Mask::splat(true),
                    idx,
                    Simd::default(),
                );
                let test_z = result.simd_eq(Simd::default());
                let test_m = result.simd_eq(key);
                mask = Operations::conflict_detection(idx, (test_z | test_m).cast()).cast();
                let result_z = test_z & mask;
                let result_m = test_m & mask;
                let base = Simd::gather_select_unchecked(
                    table_vals,
                    result_m.cast(),
                    idx,
                    Simd::default(),
                );
                key.scatter_select_unchecked(table_keys, result_z.cast(), idx);
                result_z
                    .cast()
                    .select(map(&self.insert, del), map2(&self.update, base, del))
                    .scatter_select_unchecked(table_vals, mask.cast(), idx);
                self.table.len += result_z.to_bitmask().count_ones() as usize;
                idx = (test_z | test_m)
                    .cast()
                    .select(idx, (idx + scale) & (scale * Simd::splat(n - 1)));
            }
            for j in 0..LANES {
                if !mask.test(j) {
                    let result = self.table.insert(key[j]);
                    match result {
                        Ok(x) => {
                            x.write((self.insert)(del[j]));
                        }
                        Err(x) => {
                            *x = (self.update)(*x, del[j]);
                        }
                    }
                }
            }
            while i < m {
                let result = self.table.insert(input_keys[i]);
                match result {
                    Ok(x) => {
                        x.write((self.insert)(input_dels[i]));
                    }
                    Err(x) => {
                        *x = (self.update)(*x, input_dels[i]);
                    }
                }
                i += 1;
            }
            self.keys.set_len(0);
            self.dels.set_len(0);
        }
    }
}

// It should be vectorized by compiler.
#[inline(always)]
fn map<A, B, F, const LANES: usize>(f: F, simd: Simd<A, LANES>) -> Simd<B, LANES>
where
    F: Fn(A) -> B,
    A: SimdElement,
    B: SimdElement + Default,
    LaneCount<LANES>: SupportedLaneCount,
{
    let mut res = Simd::<B, LANES>::default();
    for i in 0..LANES {
        res[i] = f(simd[i]);
    }
    res
}

// It should be vectorized by compiler.
#[inline(always)]
fn map2<A, B, C, F, const LANES: usize>(
    f: &F,
    a: Simd<A, LANES>,
    b: Simd<B, LANES>,
) -> Simd<C, LANES>
where
    F: Fn(A, B) -> C,
    A: SimdElement,
    B: SimdElement,
    C: SimdElement + Default,
    LaneCount<LANES>: SupportedLaneCount,
{
    let mut res = Simd::<C, LANES>::default();
    for i in 0..LANES {
        res[i] = f(a[i], b[i]);
    }
    res
}

// `mask swizzle` is needed here, but it's not supported by portable_simd.
// https://github.com/rust-lang/portable-simd/issues/268
macro_rules! mask_swizzle {
    ($vector:expr, $index:expr $(,)?) => {
        unsafe { Mask::from_int_unchecked(simd_swizzle!($vector.to_int(), $index)) }
    };
}

pub struct Operations;

pub trait SupportedOperations<const LANES: usize>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn reorder(mask: u8) -> [u32; LANES];
    fn conflict_detection(simd: Simd<usize, LANES>, mask: Mask<isize, LANES>)
        -> Mask<isize, LANES>;
}

impl SupportedOperations<1> for Operations {
    #[inline(always)]
    fn reorder(_: u8) -> [u32; 1] {
        [0]
    }

    #[inline(always)]
    fn conflict_detection(_: Simd<usize, 1>, mask: Mask<isize, 1>) -> Mask<isize, 1> {
        mask
    }
}

impl SupportedOperations<2> for Operations {
    #[inline(always)]
    fn reorder(i: u8) -> [u32; 2] {
        const B: [[u32; 2]; 4] = {
            let mut ans = [[0u32; 2]; 4];
            let mut i = 0usize;
            while i < (1 << 2) {
                ans[i] = reorder(i as u8);
                i += 1;
            }
            ans
        };
        B[i as usize]
    }

    #[inline(always)]
    fn conflict_detection(simd: Simd<usize, 2>, mask: Mask<isize, 2>) -> Mask<isize, 2> {
        #[inline(always)]
        fn internal(simd: Simd<usize, 2>, mut mask: Mask<isize, 2>) -> Mask<isize, 2> {
            const V0: [usize; 2] = [1, 0];
            mask &= simd.simd_ne(simd_swizzle!(simd, V0))
                | !mask_swizzle!(mask, V0)
                | mask & Mask::from_array([false, true]);
            mask
        }

        unsafe {
            let simd = std::mem::transmute_copy::<_, Simd<usize, 2>>(&simd);
            let mask = std::mem::transmute_copy::<_, Mask<isize, 2>>(&mask);
            std::mem::transmute_copy(&internal(simd, mask))
        }
    }
}

impl SupportedOperations<4> for Operations {
    #[inline(always)]
    fn reorder(i: u8) -> [u32; 4] {
        const B: [[u32; 4]; 16] = {
            let mut ans = [[0u32; 4]; 16];
            let mut i = 0usize;
            while i < (1 << 4) {
                ans[i] = reorder(i as u8);
                i += 1;
            }
            ans
        };
        B[i as usize]
    }

    #[inline(always)]
    fn conflict_detection(simd: Simd<usize, 4>, mask: Mask<isize, 4>) -> Mask<isize, 4> {
        #[inline(always)]
        fn internal(simd: Simd<usize, 4>, mut mask: Mask<isize, 4>) -> Mask<isize, 4> {
            const V0: [usize; 4] = [1, 2, 3, 0];
            const V1: [usize; 4] = [2, 3, 1, 0];
            mask &= simd.simd_ne(simd_swizzle!(simd, V0))
                | !mask_swizzle!(mask, V0)
                | mask & Mask::from_array([false, false, false, true]);
            mask &= simd.simd_ne(simd_swizzle!(simd, V1))
                | !mask_swizzle!(mask, V1)
                | mask & Mask::from_array([false, false, true, false]);
            mask
        }

        unsafe {
            let simd = std::mem::transmute_copy::<_, Simd<usize, 4>>(&simd);
            let mask = std::mem::transmute_copy::<_, Mask<isize, 4>>(&mask);
            std::mem::transmute_copy(&internal(simd, mask))
        }
    }
}

impl SupportedOperations<8> for Operations {
    #[inline(always)]
    fn reorder(i: u8) -> [u32; 8] {
        const B: [[u32; 8]; 256] = {
            let mut ans = [[0u32; 8]; 256];
            let mut i = 0usize;
            while i < (1 << 8) {
                ans[i] = reorder(i as u8);
                i += 1;
            }
            ans
        };
        B[i as usize]
    }

    #[inline(always)]
    fn conflict_detection(simd: Simd<usize, 8>, mask: Mask<isize, 8>) -> Mask<isize, 8> {
        #[inline(always)]
        fn internal(simd: Simd<usize, 8>, mut mask: Mask<isize, 8>) -> Mask<isize, 8> {
            const V0: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 0];
            const V1: [usize; 8] = [2, 3, 5, 0, 6, 7, 1, 4];
            const V2: [usize; 8] = [6, 7, 4, 5, 0, 2, 1, 3];
            const V3: [usize; 8] = [5, 4, 7, 6, 3, 1, 2, 0];
            mask &= simd.simd_ne(simd_swizzle!(simd, V0))
                | !mask_swizzle!(mask, V0)
                | mask & Mask::from_array([false, false, false, false, false, false, false, true]);
            mask &= simd.simd_ne(simd_swizzle!(simd, V1))
                | !mask_swizzle!(mask, V1)
                | mask & Mask::from_array([false, false, false, false, false, false, true, false]);
            mask &= simd.simd_ne(simd_swizzle!(simd, V2))
                | !mask_swizzle!(mask, V2)
                | mask & Mask::from_array([false, false, false, false, false, true, false, false]);
            mask &= simd.simd_ne(simd_swizzle!(simd, V3))
                | !mask_swizzle!(mask, V3)
                | mask & Mask::from_array([false, false, false, false, true, false, false, false]);
            mask
        }

        unsafe {
            let simd = std::mem::transmute_copy::<_, Simd<usize, 8>>(&simd);
            let mask = std::mem::transmute_copy::<_, Mask<isize, 8>>(&mask);
            std::mem::transmute_copy(&internal(simd, mask))
        }
    }
}

const fn reorder<const LANES: usize>(mask: u8) -> [u32; LANES]
where
    LaneCount<LANES>: SupportedLaneCount,
{
    let mut ans = [0u32; LANES];
    let mut ptr = 0;
    let mut i;
    i = 0u32;
    while i < LANES as u32 {
        if 0 == (mask & (1u8 << i)) {
            ans[ptr] = i;
            ptr += 1;
        }
        i += 1;
    }
    i = 0u32;
    while i < LANES as u32 {
        if 0 != (mask & (1u8 << i)) {
            ans[ptr] = i;
            ptr += 1;
        }
        i += 1;
    }
    ans
}

// It's not supported by portable_simd.
// https://github.com/rust-lang/portable-simd/issues/242
pub trait DynamicSwizzle {
    type I;

    fn dynamic_swizzle(simd: Self, index: Self::I) -> Self;
}

macro_rules! dynamic_swizzle_proxy {
    ($surface: ty, $underlaying: ty) => {
        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 2>
        where
            Simd<$underlaying, 2>: DynamicSwizzle<I = [u32; 2]>,
        {
            type I = [u32; 2];

            #[inline(always)]
            fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
                unsafe {
                    let simd = std::mem::transmute(simd);
                    std::mem::transmute(Simd::<$underlaying, 2>::dynamic_swizzle(simd, index))
                }
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 4>
        where
            Simd<$underlaying, 4>: DynamicSwizzle<I = [u32; 4]>,
        {
            type I = [u32; 4];

            #[inline(always)]
            fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
                unsafe {
                    let simd = std::mem::transmute(simd);
                    std::mem::transmute(Simd::<$underlaying, 4>::dynamic_swizzle(simd, index))
                }
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 8>
        where
            Simd<$underlaying, 8>: DynamicSwizzle<I = [u32; 8]>,
        {
            type I = [u32; 8];

            #[inline(always)]
            fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
                unsafe {
                    let simd = std::mem::transmute(simd);
                    std::mem::transmute(Simd::<$underlaying, 8>::dynamic_swizzle(simd, index))
                }
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 16>
        where
            Simd<$underlaying, 16>: DynamicSwizzle<I = [u32; 16]>,
        {
            type I = [u32; 16];

            #[inline(always)]
            fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
                unsafe {
                    let simd = std::mem::transmute(simd);
                    std::mem::transmute(Simd::<$underlaying, 16>::dynamic_swizzle(simd, index))
                }
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 32>
        where
            Simd<$underlaying, 32>: DynamicSwizzle<I = [u32; 32]>,
        {
            type I = [u32; 32];

            #[inline(always)]
            fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
                unsafe {
                    let simd = std::mem::transmute(simd);
                    std::mem::transmute(Simd::<$underlaying, 32>::dynamic_swizzle(simd, index))
                }
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 64>
        where
            Simd<$underlaying, 64>: DynamicSwizzle<I = [u32; 64]>,
        {
            type I = [u32; 64];

            #[inline(always)]
            fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
                unsafe {
                    let simd = std::mem::transmute(simd);
                    std::mem::transmute(Simd::<$underlaying, 64>::dynamic_swizzle(simd, index))
                }
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

impl<T> DynamicSwizzle for Simd<T, 1>
where
    T: SimdElement,
{
    type I = [u32; 1];

    #[inline(always)]
    fn dynamic_swizzle(simd: Self, _index: Self::I) -> Self {
        simd
    }
}

#[cfg(target_pointer_width = "64")]
impl DynamicSwizzle for Simd<i32, 8> {
    type I = [u32; 8];

    #[inline(always)]
    fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
        unsafe {
            use std::arch::x86_64::__m256i;
            use std::arch::x86_64::_mm256_permutevar8x32_epi32;
            let a = __m256i::from(simd);
            let b = __m256i::from(Simd::from_array(index).cast::<u32>());
            let c = _mm256_permutevar8x32_epi32(a, b);
            Simd::from(c)
        }
    }
}

#[cfg(target_pointer_width = "64")]
impl DynamicSwizzle for Simd<i64, 4> {
    type I = [u32; 4];

    #[inline(always)]
    fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
        unsafe {
            use std::arch::x86_64::__m256i;
            use std::arch::x86_64::_mm256_permutevar8x32_epi32;
            let a = __m256i::from(simd);
            let b: Simd<i32, 4> = Simd::from(index).cast::<i32>() * Simd::splat(2);
            let b: Simd<i32, 8> = simd_swizzle!(b, [0, 0, 1, 1, 2, 2, 3, 3]);
            let b: Simd<i32, 8> = b + Simd::from_array([0, 1, 0, 1, 0, 1, 0, 1]);
            let b = __m256i::from(b);
            let c = _mm256_permutevar8x32_epi32(a, b);
            Simd::from(c)
        }
    }
}
