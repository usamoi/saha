use crate::table2::Table2;
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
use core_simd::MaskElement;
use std::mem::MaybeUninit;
use std::ops::BitAnd;
use std::ops::BitOr;

pub struct BatchHashtable<K: Key, V> {
    raw: Table2<K, V>,
}

impl<K: Key, V> BatchHashtable<K, V> {
    pub fn new() -> Self {
        Self { raw: Table2::new() }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn len(&self) -> usize {
        self.raw.len()
    }
    pub fn capacity(&self) -> usize {
        self.raw.capacity()
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        self.raw.get(key)
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.raw.get_mut(key)
    }
    pub unsafe fn insert(&mut self, key: K) -> Result<&mut MaybeUninit<V>, &mut V> {
        if (self.raw.len() + 1) * 2 > self.raw.capacity() {
            self.raw.grow();
        }
        self.raw.insert(key)
    }
    pub unsafe fn batch_update<'a, const LANES: usize, D, F, G>(
        &'a mut self,
        f: F,
        g: G,
    ) -> BatchUpdater<'a, LANES, K, V, D, F, G>
    where
        K: Key,
    {
        BatchUpdater::<LANES, K, V, D, F, G>::new(&mut self.raw, f, g)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.raw.iter()
    }
}

pub struct BatchUpdater<'a, const LANES: usize, K, V, D, F, G>
where
    K: Key,
{
    // todo: specialization for AVX (gather & scatter with scale)
    pub(crate) table: &'a mut Table2<K, V>,
    pub(crate) keys: ArrayVec<K, 1024>,
    pub(crate) dels: ArrayVec<D, 1024>,
    pub(crate) insert: F,
    pub(crate) update: G,
}

impl<'a, const LANES: usize, K, V, D, F, G> BatchUpdater<'a, LANES, K, V, D, F, G>
where
    K: Key,
{
    pub(crate) unsafe fn new(table: &'a mut Table2<K, V>, f: F, g: G) -> Self {
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
    K: BatchKey,
    V: Copy + SimdElement + Default,
    D: Copy + SimdElement + Default,
    F: Fn(D) -> V,
    G: Fn(V, D) -> V,
    Simd<usize, LANES>: DynamicSwizzle<I = [usize; LANES]>,
    Simd<K, LANES>: DynamicSwizzle<I = [usize; LANES]>,
    Simd<K, LANES>: SimdPartialEq<Mask = Mask<K::Mask, LANES>>,
    Mask<K::Mask, LANES>: BitOr<Output = Mask<K::Mask, LANES>>,
    Mask<K::Mask, LANES>: BitAnd<Output = Mask<K::Mask, LANES>>,
    Simd<V, LANES>: DynamicSwizzle<I = [usize; LANES]>,
    Simd<D, LANES>: DynamicSwizzle<I = [usize; LANES]>,
    Simd<u64, LANES>: DynamicSwizzle<I = [usize; LANES]>,
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
        assert!(LANES >= 4);
        while (self.keys.len() + self.table.len()) * 2 > self.table.capacity() {
            self.table.grow();
        }
        let n = self.table.keys.len();
        let m = self.keys.len();
        let table_keys = unsafe { std::mem::transmute::<_, &mut [K]>(self.table.keys.as_mut()) };
        let table_vals = unsafe { std::mem::transmute::<_, &mut [V]>(self.table.vals.as_mut()) };
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
                // Or `dynamic swizzle` could be used, but it's not supported by portable_simd too.
                // https://github.com/rust-lang/portable-simd/issues/242
                idx = DynamicSwizzle::dynamic_swizzle(idx, reorder(mask));
                key = DynamicSwizzle::dynamic_swizzle(key, reorder(mask));
                del = DynamicSwizzle::dynamic_swizzle(del, reorder(mask));
                mask = init_zeros(mask);
                let t = LANES - count_ones(mask);
                let ptr_key = input_keys[i - t..].as_ptr() as *const Simd<K, LANES>;
                let ptr_del = input_dels[i - t..].as_ptr() as *const Simd<D, LANES>;
                key = mask.select(ptr_key.read_unaligned(), key);
                del = mask.cast().select(ptr_del.read_unaligned(), del);
                idx = mask
                    .cast()
                    .select(hash(key).cast() & Simd::splat(n - 1), idx);
                i += count_ones(mask);
                let result = Simd::gather_or_default(table_keys, idx);
                let zz = result.simd_eq(Simd::default());
                let mm = result.simd_eq(key);
                mask = conflict_detection::<LANES>(idx, (zz | mm).cast()).cast();
                let result_z = zz & mask;
                let result_m = mm & mask;
                let base = Simd::gather_select(table_vals, result_m.cast(), idx, Simd::default());
                key.scatter_select(table_keys, result_z.cast(), idx);
                result_z
                    .cast()
                    .select(map(&self.insert, del), map2(&self.update, base, del))
                    .scatter_select(table_vals, mask.cast(), idx);
                self.table.len += count_ones(result_z);
                idx = (zz | mm)
                    .cast()
                    .select(idx, (idx + Simd::splat(1)) & Simd::splat(n - 1));
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

fn hash<T, const LANES: usize>(simd: Simd<T, LANES>) -> Simd<u64, LANES>
where
    T: SimdElement + Key,
    LaneCount<LANES>: SupportedLaneCount,
{
    let mut hash = Simd::<u64, LANES>::splat(0);
    for i in 0..LANES {
        hash[i] = simd[i].hash();
    }
    hash
}

// `mask swizzle` is needed here, but it's not supported by portable_simd.
// https://github.com/rust-lang/portable-simd/issues/268
macro_rules! mask_swizzle {
    ($vector:expr, $index:expr $(,)?) => {
        unsafe { Mask::from_int_unchecked(simd_swizzle!($vector.to_int(), $index)) }
    };
}

fn init_zeros<T, const LANES: usize>(mask: Mask<T, LANES>) -> Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    let mut res: Mask<T, LANES> = Mask::splat(false);
    for i in LANES - count_ones(mask)..LANES {
        res.set(i, true);
    }
    res
}

fn count_ones<T, const LANES: usize>(mask: Mask<T, LANES>) -> usize
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    let mut ans = 0;
    for i in 0..LANES {
        if mask.test(i) {
            ans += 1;
        }
    }
    ans
}

// todo: cache it
fn reorder<T, const LANES: usize>(mask: Mask<T, LANES>) -> [usize; LANES]
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    let mut zeros = Vec::<usize>::new();
    let mut ones = Vec::<usize>::new();
    for i in 0..LANES {
        if mask.test(i) {
            ones.push(i);
        } else {
            zeros.push(i);
        }
    }
    let mut ans = [0usize; LANES];
    ans[..zeros.len()].copy_from_slice(&zeros);
    ans[zeros.len()..].copy_from_slice(&ones);
    ans
}

// todo: specialization for AVX-512
fn conflict_detection<const LANES: usize>(
    simd: Simd<usize, LANES>,
    mask: Mask<isize, LANES>,
) -> Mask<isize, LANES>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn _1(_: Simd<usize, 1>, mask: Mask<isize, 1>) -> Mask<isize, 1> {
        mask
    }

    fn _2(simd: Simd<usize, 2>, mut mask: Mask<isize, 2>) -> Mask<isize, 2> {
        const V0: [usize; 2] = [1, 0];
        let value = mask.test(1);
        mask &= simd.simd_ne(simd_swizzle!(simd, V0)) | !mask_swizzle!(mask, V0);
        mask.set(1, value);
        mask
    }

    fn _4(simd: Simd<usize, 4>, mut mask: Mask<isize, 4>) -> Mask<isize, 4> {
        const V0: [usize; 4] = [1, 2, 3, 0];
        const V1: [usize; 4] = [2, 3, 1, 0];
        let value = mask.test(3);
        mask &= simd.simd_ne(simd_swizzle!(simd, V0)) | !mask_swizzle!(mask, V0);
        mask.set(3, value);
        let value = mask.test(2);
        mask &= simd.simd_ne(simd_swizzle!(simd, V1)) | !mask_swizzle!(mask, V1);
        mask.set(2, value);
        mask
    }

    fn _8(simd: Simd<usize, 8>, mut mask: Mask<isize, 8>) -> Mask<isize, 8> {
        const V0: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 0];
        const V1: [usize; 8] = [2, 3, 5, 0, 6, 7, 1, 4];
        const V2: [usize; 8] = [6, 7, 4, 5, 0, 2, 1, 3];
        const V3: [usize; 8] = [5, 4, 7, 6, 3, 1, 2, 0];
        let value = mask.test(7);
        mask &= simd.simd_ne(simd_swizzle!(simd, V0)) | !mask_swizzle!(mask, V0);
        mask.set(7, value);
        let value = mask.test(6);
        mask &= simd.simd_ne(simd_swizzle!(simd, V1)) | !mask_swizzle!(mask, V1);
        mask.set(6, value);
        let value = mask.test(5);
        mask &= simd.simd_ne(simd_swizzle!(simd, V2)) | !mask_swizzle!(mask, V2);
        mask.set(5, value);
        let value = mask.test(4);
        mask &= simd.simd_ne(simd_swizzle!(simd, V3)) | !mask_swizzle!(mask, V3);
        mask.set(4, value);
        mask
    }

    match LANES {
        1 => unsafe {
            let simd = std::mem::transmute_copy::<_, Simd<usize, 1>>(&simd);
            let mask = std::mem::transmute_copy::<_, Mask<isize, 1>>(&mask);
            std::mem::transmute_copy(&_1(simd, mask))
        },
        2 => unsafe {
            let simd = std::mem::transmute_copy::<_, Simd<usize, 2>>(&simd);
            let mask = std::mem::transmute_copy::<_, Mask<isize, 2>>(&mask);
            std::mem::transmute_copy(&_2(simd, mask))
        },
        4 => unsafe {
            let simd = std::mem::transmute_copy::<_, Simd<usize, 4>>(&simd);
            let mask = std::mem::transmute_copy::<_, Mask<isize, 4>>(&mask);
            std::mem::transmute_copy(&_4(simd, mask))
        },
        8 => unsafe {
            let simd = std::mem::transmute_copy::<_, Simd<usize, 8>>(&simd);
            let mask = std::mem::transmute_copy::<_, Mask<isize, 8>>(&mask);
            std::mem::transmute_copy(&_8(simd, mask))
        },
        _ => unimplemented!(),
    }
}

pub trait DynamicSwizzle {
    type I;

    fn dynamic_swizzle(simd: Self, index: Self::I) -> Self;
}

macro_rules! dynamic_swizzle_proxy {
    ($surface: ty, $underlaying: ty) => {
        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 1>
        where
            Simd<$underlaying, 1>: DynamicSwizzle<I = [usize; 1]>,
        {
            type I = [usize; 1];

            fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
                unsafe {
                    let simd = std::mem::transmute(simd);
                    std::mem::transmute(Simd::<$underlaying, 1>::dynamic_swizzle(simd, index))
                }
            }
        }

        #[allow(trivial_bounds)]
        impl DynamicSwizzle for Simd<$surface, 2>
        where
            Simd<$underlaying, 2>: DynamicSwizzle<I = [usize; 2]>,
        {
            type I = [usize; 2];

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
            Simd<$underlaying, 4>: DynamicSwizzle<I = [usize; 4]>,
        {
            type I = [usize; 4];

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
            Simd<$underlaying, 8>: DynamicSwizzle<I = [usize; 8]>,
        {
            type I = [usize; 8];

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
            Simd<$underlaying, 16>: DynamicSwizzle<I = [usize; 16]>,
        {
            type I = [usize; 16];

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
            Simd<$underlaying, 32>: DynamicSwizzle<I = [usize; 32]>,
        {
            type I = [usize; 32];

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
            Simd<$underlaying, 64>: DynamicSwizzle<I = [usize; 64]>,
        {
            type I = [usize; 64];

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

#[cfg(target_pointer_width = "64")]
impl DynamicSwizzle for Simd<i32, 8> {
    type I = [usize; 8];

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
    type I = [usize; 4];

    fn dynamic_swizzle(simd: Self, index: Self::I) -> Self {
        unsafe {
            use std::arch::x86_64::__m256i;
            use std::arch::x86_64::_mm256_permutevar8x32_epi32;
            let a = __m256i::from(simd);
            let b = Simd::from(index).cast::<i32>() * Simd::splat(2);
            let b = simd_swizzle!(b, [0, 0, 1, 1, 2, 2, 3, 3]);
            let b = b + Simd::from_array([0, 1, 0, 1, 0, 1, 0, 1]);
            let b = __m256i::from(b);
            let c = _mm256_permutevar8x32_epi32(a, b);
            Simd::from(c)
        }
    }
}
