use crate::container::HeapContainer;
use crate::simd::dynamic_swizzle::DynamicSwizzle;
use crate::simd::gather::{Gather, SupportedGather};
use crate::simd::pext::{Pext, SupportedPext};
use crate::simd::scatter::{Scatter, SupportedScatter};
use crate::table0::{Slot, Table0};
use crate::traits::Key;
use core_simd::simd::*;
use num::traits::AsPrimitive;
use num::Bounded;
use std::alloc::Allocator;

type I = u32;

pub(crate) unsafe fn batch_build<const LANES: usize, K, V, D, F, G, A>(
    table: &mut Table0<K, V, HeapContainer<Slot<K, V>, A>, A>,
    f: F,
    g: G,
    idxs: &[I],
    keys: &[K],
    dels: &[D],
) where
    K: SimdElement + Key + Default + AsPrimitive<usize> + Bounded,
    usize: AsPrimitive<K>,
    V: SimdElement + Default,
    D: SimdElement + Default,
    LaneCount<LANES>: SupportedLaneCount,
    Pext: SupportedPext<LANES>,
    F: Fn(D) -> V,
    G: Fn(V, D) -> V,
    Simd<I, LANES>: DynamicSwizzle<I = Simd<u8, LANES>>,
    Simd<K, LANES>: DynamicSwizzle<I = Simd<u8, LANES>>,
    Simd<K, LANES>: SimdPartialEq<Mask = Mask<<K as SimdElement>::Mask, LANES>>,
    Mask<<K as SimdElement>::Mask, LANES>: ToBitMask<BitMask = u8>,
    Simd<D, LANES>: DynamicSwizzle<I = Simd<u8, LANES>>,
    Gather: SupportedGather<K, LANES>,
    Gather: SupportedGather<V, LANES>,
    Scatter: SupportedScatter<K, LANES>,
    Scatter: SupportedScatter<V, LANES>,
    A: Allocator + Clone,
{
    assert_eq!(idxs.len(), keys.len());
    assert_eq!(idxs.len(), dels.len());
    while (keys.len() + table.len()) * 2 > table.capacity() {
        if (table.capacity() >> 22) == 0 {
            table.grow(2);
        } else {
            table.grow(1);
        }
    }
    let n = table.capacity() as u32;
    let m = keys.len();
    let offset_keys = memoffset::offset_of!(Slot<K, V>, key);
    let offset_vals = memoffset::offset_of!(Slot<K, V>, val);
    let scale_keys = (std::mem::size_of::<Slot<K, V>>() / std::mem::size_of::<K>()) as u32;
    let scale_vals = (std::mem::size_of::<Slot<K, V>>() / std::mem::size_of::<V>()) as u32;
    let raw_keys = (table.slots.as_mut_ptr() as *mut u8).add(offset_keys) as *mut K;
    let raw_vals = (table.slots.as_mut_ptr() as *mut u8).add(offset_vals) as *mut V;
    let mut idx: Simd<I, LANES> = Simd::default();
    let mut key: Simd<K, LANES> = Simd::default();
    let mut del: Simd<D, LANES> = Simd::default();
    let mut mask: Mask<<K as SimdElement>::Mask, LANES> = Mask::splat(true);
    let mut i = 0usize;
    while i + LANES <= m {
        let reorder = Pext::pext(mask.cast());
        let count = LANES - mask.to_bitmask().count_ones() as usize;
        mask = Mask::from_bitmask((!((1usize << count) - 1)) as u8);
        idx = idx.dynamic_swizzle(reorder);
        key = key.dynamic_swizzle(reorder);
        del = del.dynamic_swizzle(reorder);
        let idxu = idxs[i - count..].as_ptr() as *const Simd<I, LANES>;
        let idxl = idxu.read_unaligned() & Simd::splat(n - 1);
        let keyu = keys[i - count..].as_ptr() as *const Simd<K, LANES>;
        let keyl = keyu.read_unaligned();
        let delu = dels[i - count..].as_ptr() as *const Simd<D, LANES>;
        let dell = delu.read_unaligned();
        idx = mask.cast().select(idxl, idx);
        key = mask.cast().select(keyl, key);
        del = mask.cast().select(dell, del);
        i += mask.to_bitmask().count_ones() as usize;
        let fetch_keys = Gather::gather(raw_keys, idx * Simd::splat(scale_keys));
        let fetch_vals = Gather::gather(raw_vals, idx * Simd::splat(scale_vals));
        let insert_vals = map(&f, del);
        let update_vals = map2(&g, fetch_vals, del);
        let test_z = fetch_keys.simd_eq(Simd::default());
        let test_m = fetch_keys.simd_eq(key);
        let test = test_z | test_m;
        let flag = {
            let mut flag = Simd::<K, LANES>::default();
            for i in 0..LANES {
                flag[i] = i.as_();
            }
            flag
        };
        Scatter::mask_scatter(raw_keys, test.cast(), idx * Simd::splat(scale_keys), flag);
        mask = Gather::mask_gather(
            raw_keys,
            test.cast(),
            idx * Simd::splat(scale_keys),
            Simd::splat(Bounded::max_value()),
        )
        .simd_eq(flag);
        let output = test_z.cast().select(insert_vals, update_vals);
        Scatter::mask_scatter(raw_keys, mask.cast(), idx * Simd::splat(scale_keys), key);
        Scatter::mask_scatter(raw_vals, mask.cast(), idx * Simd::splat(scale_vals), output);
        table.len += (test_z & mask).to_bitmask().count_ones() as usize;
        idx = test
            .cast()
            .select(idx, (idx + Simd::splat(1)) & Simd::splat(n - 1));
    }
    for j in 0..LANES {
        if !mask.test(j) {
            let result = table.insert(key[j]);
            match result {
                Ok(x) => {
                    x.write(f(del[j]));
                }
                Err(x) => {
                    *x = g(*x, del[j]);
                }
            }
        }
    }
    while i < m {
        let result = table.insert(keys[i]);
        match result {
            Ok(x) => {
                x.write(f(dels[i]));
            }
            Err(x) => {
                *x = g(*x, dels[i]);
            }
        }
        i += 1;
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
