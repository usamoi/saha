use crate::container::HeapContainer;
use crate::table0::{Slot, Table0};
use crate::traits::Key;
use std::alloc::Allocator;
use std::intrinsics::unlikely;
use std::mem::MaybeUninit;

const BUCKETS: usize = 256;
const BUCKETS_LG2: u32 = 8;

pub struct TwolevelHashtable<K, V, A = crate::allocator::Default>
where
    K: Key,
    A: Allocator + Clone,
{
    zero: Option<Slot<K, V>>,
    tables: [Table0<K, V, HeapContainer<Slot<K, V>, A>, A>; BUCKETS],
}

impl<K, V, A> TwolevelHashtable<K, V, A>
where
    K: Key,
    A: Allocator + Clone + Default,
{
    pub fn new() -> Self {
        Self::new_in(Default::default())
    }
}

impl<K, V, A> TwolevelHashtable<K, V, A>
where
    K: Key,
    A: Allocator + Clone,
{
    pub fn new_in(allocator: A) -> Self {
        Self {
            zero: None,
            tables: std::array::from_fn(|_| Table0::with_capacity_in(256, allocator.clone())),
        }
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.zero.is_some() as usize + self.tables.iter().map(|x| x.len()).sum::<usize>()
    }
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.zero.is_some() as usize + self.tables.iter().map(|x| x.capacity()).sum::<usize>()
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
        let hash = K::hash(key);
        let index = hash as usize >> (64u32 - BUCKETS_LG2);
        unsafe { self.tables[index].get_with_hash(key, hash) }
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
        let hash = K::hash(key);
        let index = hash as usize >> (64u32 - BUCKETS_LG2);
        unsafe { self.tables[index].get_with_hash_mut(key, hash) }
    }
    #[inline(always)]
    pub unsafe fn insert(&mut self, key: K) -> Result<&mut MaybeUninit<V>, &mut V> {
        if unlikely(K::equals_zero(&key)) {
            let zero = &mut self.zero;
            if let Some(slot) = zero {
                return Err(slot.val.assume_init_mut());
            } else {
                *zero = Some(MaybeUninit::zeroed().assume_init());
                return Ok(&mut zero.as_mut().unwrap().val);
            }
        }
        let hash = K::hash(&key);
        let index = hash as usize >> (64u32 - BUCKETS_LG2);
        if unlikely((self.tables[index].len() + 1) * 2 > self.tables[index].capacity()) {
            if (self.tables[index].slots.len() >> 14) == 0 {
                self.tables[index].grow(2);
            } else {
                self.tables[index].grow(1);
            }
        }
        self.tables[index].insert_with_hash(key, hash)
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
        for (i, table) in other.tables.into_iter().enumerate() {
            while (self.tables[i].len() + table.len()) * 2 > self.tables[i].capacity() {
                if (self.tables[i].slots.len() >> 22) == 0 {
                    self.tables[i].grow(2);
                } else {
                    self.tables[i].grow(1);
                }
            }
            self.tables[i].merge(table, &mut f);
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.tables.iter().flat_map(|x| x.iter())
    }
}
