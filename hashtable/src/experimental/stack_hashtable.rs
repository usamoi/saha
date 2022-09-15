use crate::container::StackContainer;
use crate::table0::{Slot, Table0};
use crate::traits::Key;
use std::alloc::Allocator;
use std::intrinsics::unlikely;
use std::mem::MaybeUninit;

pub struct StackHashtable<K, V, const N: usize = 16, A = crate::allocator::Default>
where
    K: Key,
    A: Allocator + Clone,
{
    zero: Option<Slot<K, V>>,
    table: Table0<K, V, StackContainer<Slot<K, V>, N, A>, A>,
}

impl<K, V, A, const N: usize> StackHashtable<K, V, N, A>
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

impl<K, V, A, const N: usize> StackHashtable<K, V, N, A>
where
    K: Key,
    A: Allocator + Clone,
{
    pub fn new_in(allocator: A) -> Self {
        Self::with_capacity_in(N, allocator)
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
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.table.iter()
    }
}
