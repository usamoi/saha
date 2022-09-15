use crate::container::HeapContainer;
use crate::table0::{Slot, Table0};
use crate::traits::Key;
use std::alloc::Allocator;
use std::intrinsics::unlikely;
use std::mem::MaybeUninit;

const CAPACITY: usize = 1 << 16;

pub struct ExtendibleHashtable<K, V, A = crate::allocator::Default>
where
    K: Key,
    A: Allocator + Clone,
{
    pub(crate) count: u8,
    pub(crate) pointers: Vec<usize, A>,
    pub(crate) zero: Option<Slot<K, V>>,
    pub(crate) tables: Vec<(u8, Table0<K, V, HeapContainer<Slot<K, V>, A>, A>)>,
}

impl<K, V, A> ExtendibleHashtable<K, V, A>
where
    K: Key,
    A: Allocator + Clone + Default,
{
    pub fn new() -> Self {
        Self::new_in(Default::default())
    }
}

impl<K, V, A> ExtendibleHashtable<K, V, A>
where
    K: Key,
    A: Allocator + Clone,
{
    pub fn new_in(allocator: A) -> Self {
        Self {
            count: 3,
            pointers: {
                let mut pointers = Vec::new_in(allocator.clone());
                pointers.resize(1 << 3, 0);
                pointers
            },
            zero: None,
            tables: vec![(0, Table0::with_capacity_in(CAPACITY, allocator))],
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn len(&self) -> usize {
        self.zero.is_some() as usize + self.tables.iter().map(|(_, x)| x.len()).sum::<usize>()
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        if unlikely(K::equals_zero(key)) {
            if let Some(slot) = self.zero.as_ref() {
                return Some(unsafe { slot.val.assume_init_ref() });
            } else {
                return None;
            }
        }
        let hash = key.hash();
        let prefix = (hash >> (64 - self.count)) as usize;
        let index = self.pointers[prefix];
        unsafe { self.tables[index].1.get_with_hash(key, hash) }
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if unlikely(K::equals_zero(key)) {
            if let Some(slot) = self.zero.as_mut() {
                return Some(unsafe { slot.val.assume_init_mut() });
            } else {
                return None;
            }
        }
        let hash = key.hash();
        let prefix = (hash >> (64 - self.count)) as usize;
        let index = self.pointers[prefix];
        unsafe { self.tables[index].1.get_with_hash_mut(key, hash) }
    }
    /// # Safety
    ///
    /// The resulted `MaybeUninit` should be initialized immedidately.
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
        loop {
            let hash = key.hash();
            let prefix = (hash >> (64 - self.count)) as usize;
            let index = self.pointers[prefix];
            if unlikely(self.tables[index].1.len() >= CAPACITY / 2) {
                let level = self.tables[index].0;
                if unlikely(self.count == level) {
                    self.count += 1;
                    self.pointers.resize(1 << self.count, 0);
                    for i in (0..1 << self.count).rev() {
                        self.pointers[i] = self.pointers[i >> 1];
                    }
                    continue;
                } else {
                    let other = self.tables[index]
                        .1
                        .split(|hash| (hash >> (63 - level)) & 1 != 0);
                    self.tables[index].0 = level + 1;
                    self.tables.push((level + 1, other));
                    let start = prefix >> (self.count - level);
                    let shift = self.count - (level + 1);
                    self.pointers[(start * 2 + 1) << shift..(start * 2 + 2) << shift]
                        .fill(self.tables.len() - 1);
                    continue;
                }
            } else {
                break self.tables[index].1.insert_with_hash(key, hash);
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.tables.iter().flat_map(|x| x.1.iter()).chain(
            self.zero
                .iter()
                .map(|x| unsafe { (x.key.assume_init_ref(), x.val.assume_init_ref()) }),
        )
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> + '_ {
        self.tables.iter_mut().flat_map(|x| x.1.iter_mut()).chain(
            self.zero
                .iter_mut()
                .map(|x| unsafe { (x.key.assume_init_ref(), x.val.assume_init_mut()) }),
        )
    }
    pub fn count(&self) -> u8 {
        self.count
    }
    pub fn buckets(&self) -> usize {
        self.tables.len()
    }
}
