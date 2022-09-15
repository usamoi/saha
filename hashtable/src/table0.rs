use crate::traits::{Container, Key};
use std::alloc::Allocator;
use std::borrow::Borrow;
use std::intrinsics::assume;
use std::mem::MaybeUninit;

pub(crate) struct Slot<K, V> {
    pub(crate) _alignment: [u64; 0],
    pub(crate) key: MaybeUninit<K>,
    pub(crate) val: MaybeUninit<V>,
}

impl<K: Key, V> Slot<K, V> {
    #[inline(always)]
    pub(crate) fn is_zero(&self) -> bool {
        K::is_zero(&self.key)
    }
}

pub(crate) struct Table0<K, V, C, A>
where
    K: Key,
    C: Container<T = Slot<K, V>, A = A>,
    A: Allocator + Clone,
{
    pub(crate) len: usize,
    pub(crate) allocator: A,
    pub(crate) slots: C,
    pub(crate) dropped: bool,
}

impl<K, V, C, A> Table0<K, V, C, A>
where
    K: Key,
    C: Container<T = Slot<K, V>, A = A>,
    A: Allocator + Clone,
{
    pub fn with_capacity_in(capacity: usize, allocator: A) -> Self {
        Self {
            slots: unsafe {
                C::new_zeroed(
                    std::cmp::max(8, capacity.next_power_of_two()),
                    allocator.clone(),
                )
            },
            len: 0,
            allocator,
            dropped: false,
        }
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
    /// # Safety
    ///
    /// `key` doesn't equal to zero.
    #[inline(always)]
    pub unsafe fn get(&self, key: &K) -> Option<&V> {
        self.get_with_hash(key, key.hash())
    }
    /// # Safety
    ///
    /// `key` doesn't equal to zero.
    /// Provided hash is correct.
    #[inline(always)]
    pub unsafe fn get_with_hash(&self, key: &K, hash: u64) -> Option<&V> {
        assume(!K::equals_zero(key));
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            assume(i < self.slots.len());
            if self.slots[i].is_zero() {
                return None;
            }
            if self.slots[i].key.assume_init_ref().borrow() == key {
                return Some(self.slots[i].val.assume_init_ref());
            }
        }
        None
    }
    /// # Safety
    ///
    /// `key` doesn't equal to zero.
    #[inline(always)]
    pub unsafe fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.get_with_hash_mut(key, key.hash())
    }
    /// # Safety
    ///
    /// `key` doesn't equal to zero.
    /// Provided hash is correct.
    #[inline(always)]
    pub unsafe fn get_with_hash_mut(&mut self, key: &K, hash: u64) -> Option<&mut V> {
        assume(!K::equals_zero(key));
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            assume(i < self.slots.len());
            if self.slots[i].is_zero() {
                return None;
            }
            if self.slots[i].key.assume_init_ref().borrow() == key {
                return Some(self.slots[i].val.assume_init_mut());
            }
        }
        None
    }
    /// # Safety
    ///
    /// `key` doesn't equal to zero.
    /// The resulted `MaybeUninit` should be initialized immedidately.
    ///
    /// # Panics
    ///
    /// Panics if the hash table overflows.
    #[inline(always)]
    pub unsafe fn insert(&mut self, key: K) -> Result<&mut MaybeUninit<V>, &mut V> {
        self.insert_with_hash(key, key.hash())
    }
    /// # Safety
    ///
    /// `key` doesn't equal to zero.
    /// The resulted `MaybeUninit` should be initialized immedidately.
    /// Provided hash is correct.
    ///
    /// # Panics
    /// The hashtable is full.
    #[inline(always)]
    pub unsafe fn insert_with_hash(
        &mut self,
        key: K,
        hash: u64,
    ) -> Result<&mut MaybeUninit<V>, &mut V> {
        assume(!K::equals_zero(&key));
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            assume(i < self.slots.len());
            if self.slots[i].is_zero() {
                self.len += 1;
                self.slots[i].key.write(key);
                return Ok(&mut self.slots[i].val);
            }
            if self.slots[i].key.assume_init_ref() == &key {
                return Err(self.slots[i].val.assume_init_mut());
            }
        }
        panic!("the hash table overflows")
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.slots
            .iter()
            .filter(|slot| !slot.is_zero())
            .map(|slot| unsafe { (slot.key.assume_init_ref(), slot.val.assume_init_ref()) })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> + '_ {
        self.slots
            .iter_mut()
            .filter(|slot| !slot.is_zero())
            .map(|slot| unsafe { (slot.key.assume_init_ref(), slot.val.assume_init_mut()) })
    }
    pub(crate) unsafe fn iter_raw_mut(&mut self) -> impl Iterator<Item = &mut Slot<K, V>> + '_ {
        self.slots.iter_mut().filter(|slot| !slot.is_zero())
    }
    pub unsafe fn merge<F>(&mut self, mut other: Self, mut f: F)
    where
        F: FnMut(K, Result<&mut MaybeUninit<V>, &mut V>, V),
    {
        assert!(self.capacity() >= self.len() + other.len());
        other.dropped = true;
        for slot in other.iter_raw_mut() {
            let key = *slot.key.assume_init_ref();
            let result = self.insert(key);
            f(key, result, slot.val.assume_init_read());
        }
    }
    pub fn grow(&mut self, shift: u8) {
        let old_capacity = self.slots.len();
        let new_capacity = self.slots.len() << shift;
        unsafe {
            self.slots.grow_zeroed(new_capacity);
        }
        for i in 0..old_capacity {
            unsafe {
                assume(i < self.slots.len());
            }
            if K::is_zero(&self.slots[i].key) {
                continue;
            }
            let key = unsafe { self.slots[i].key.assume_init_ref() };
            let hash = K::hash(key);
            let index = (hash as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                unsafe {
                    assume(j < self.slots.len());
                }
                if j == i {
                    break;
                }
                if self.slots[j].is_zero() {
                    unsafe {
                        self.slots[j] = std::ptr::read(&self.slots[i]);
                        self.slots[i].key = MaybeUninit::zeroed();
                    }
                    break;
                }
            }
        }
        for i in old_capacity..new_capacity {
            unsafe {
                assume(i < self.slots.len());
            }
            if K::is_zero(&self.slots[i].key) {
                break;
            }
            let key = unsafe { self.slots[i].key.assume_init_ref() };
            let hash = K::hash(key);
            let index = (hash as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                unsafe {
                    assume(j < self.slots.len());
                }
                if j == i {
                    break;
                }
                if self.slots[j].is_zero() {
                    unsafe {
                        self.slots[j] = std::ptr::read(&self.slots[i]);
                        self.slots[i].key = MaybeUninit::zeroed();
                    }
                    break;
                }
            }
        }
    }
    pub fn split(&mut self, mut f: impl FnMut(u64) -> bool) -> Self {
        let mut other = Self::with_capacity_in(self.slots.len(), self.allocator.clone());
        for i in 0..self.slots.len() {
            unsafe {
                assume(i < self.slots.len());
            }
            if K::is_zero(&self.slots[i].key) {
                continue;
            }
            let key = unsafe { self.slots[i].key.assume_init_ref() };
            let hash = K::hash(key);
            let index = (hash as usize) & (self.slots.len() - 1);
            let select = f(hash);
            unsafe {
                let val = std::ptr::read(self.slots[i].val.assume_init_ref());
                if select {
                    other.insert(*key).ok().unwrap().write(val);
                    self.slots[i].key = MaybeUninit::zeroed();
                    self.len -= 1;
                } else {
                    for j in (index..self.slots.len()).chain(0..index) {
                        assume(j < self.slots.len());
                        if j == i {
                            break;
                        }
                        if self.slots[j].is_zero() {
                            self.slots[j] = std::ptr::read(&self.slots[i]);
                            self.slots[i].key = MaybeUninit::zeroed();
                            break;
                        }
                    }
                }
            }
        }
        for i in 0..self.slots.len() {
            unsafe {
                assume(i < self.slots.len());
            }
            if K::is_zero(&self.slots[i].key) {
                break;
            }
            let key = unsafe { self.slots[i].key.assume_init_ref() };
            let hash = K::hash(key);
            let index = (hash as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                unsafe {
                    assume(j < self.slots.len());
                }
                if j == i {
                    break;
                }
                if self.slots[j].is_zero() {
                    unsafe {
                        self.slots[j] = std::ptr::read(&self.slots[i]);
                        self.slots[i].key = MaybeUninit::zeroed();
                    }
                    break;
                }
            }
        }
        other
    }
}

impl<K, V, C, A> Drop for Table0<K, V, C, A>
where
    K: Key,
    C: Container<T = Slot<K, V>, A = A>,
    A: Allocator + Clone,
{
    fn drop(&mut self) {
        if std::mem::needs_drop::<V>() && !self.dropped {
            self.iter_mut().for_each(|(_, v)| unsafe {
                std::ptr::drop_in_place(v);
            });
        }
    }
}
