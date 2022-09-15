use crate::container::HeapContainer;
use crate::experimental::batch::batch_build;
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
use std::intrinsics::unlikely;
use std::mem::MaybeUninit;

type I = u32;

pub struct Hashtable<K, V, A = crate::allocator::Default>
where
    K: Key,
    A: Allocator + Clone,
{
    zero: Option<Slot<K, V>>,
    table: Table0<K, V, HeapContainer<Slot<K, V>, A>, A>,
}

impl<K, V, A> Hashtable<K, V, A>
where
    K: Key,
    A: Allocator + Clone + Default,
{
    pub fn new() -> Self {
        Self::new_in(Default::default())
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity, Default::default())
    }
}

impl<K, V, A> Hashtable<K, V, A>
where
    K: Key,
    A: Allocator + Clone,
{
    pub fn new_in(allocator: A) -> Self {
        Self::with_capacity_in(256, allocator)
    }
    pub fn with_capacity_in(capacity: usize, allocator: A) -> Self {
        Self {
            table: Table0::with_capacity_in(capacity, allocator),
            zero: None,
        }
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.zero.is_some() as usize + self.table.len()
    }
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.zero.is_some() as usize + self.table.capacity()
    }
    #[inline(always)]
    pub fn get(&self, key: &K) -> Option<&V> {
        if unlikely(K::equals_zero(key)) {
            if let Some(slot) = self.zero.as_ref() {
                return Some(unsafe { slot.val.assume_init_ref() });
            } else {
                return None;
            }
        }
        unsafe { self.table.get(key) }
    }
    #[inline(always)]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if unlikely(K::equals_zero(key)) {
            if let Some(slot) = self.zero.as_mut() {
                return Some(unsafe { slot.val.assume_init_mut() });
            } else {
                return None;
            }
        }
        unsafe { self.table.get_mut(key) }
    }
    #[inline(always)]
    pub unsafe fn insert(&mut self, key: K) -> Result<&mut MaybeUninit<V>, &mut V> {
        if unlikely(K::equals_zero(&key)) {
            let zero = &mut self.zero;
            if let Some(zero) = zero {
                return Err(zero.val.assume_init_mut());
            } else {
                *zero = Some(MaybeUninit::zeroed().assume_init());
                return Ok(&mut zero.as_mut().unwrap().val);
            }
        }
        if unlikely((self.table.len() + 1) * 2 > self.table.capacity()) {
            if (self.table.slots.len() >> 22) == 0 {
                self.table.grow(2);
            } else {
                self.table.grow(1);
            }
        }
        self.table.insert(key)
    }
    #[inline(always)]
    pub unsafe fn merge<F>(&mut self, other: Self, mut f: F)
    where
        F: FnMut(K, Result<&mut MaybeUninit<V>, &mut V>, V),
    {
        if let Some(Slot { key, val, .. }) = other.zero {
            let key = key.assume_init();
            let val = val.assume_init();
            f(key, self.insert(key), val);
        }
        while (self.table.len() + other.table.len()) * 2 > self.table.capacity() {
            if (self.table.slots.len() >> 22) == 0 {
                self.table.grow(2);
            } else {
                self.table.grow(1);
            }
        }
        self.table.merge(other.table, f);
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.table.iter()
    }
    pub unsafe fn batch_insert<const LANES: usize, D, F, G>(
        &mut self,
        f: F,
        g: G,
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
    {
        let m = keys.len();
        assert_eq!(m, dels.len());
        unsafe {
            let mut idxs = Vec::<I>::with_capacity(m);
            for i in 0..m {
                *idxs.get_unchecked_mut(i) = Key::hash(&keys[i]) as I;
            }
            idxs.set_len(m);
            batch_build(
                &mut self.table,
                f,
                g,
                idxs.as_ref(),
                keys.as_ref(),
                dels.as_ref(),
            );
        }
    }
}
