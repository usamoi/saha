use crate::traits::Key;
use std::alloc::{Allocator, Global, Layout};
use std::borrow::Borrow;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

pub(crate) struct Slot<K, V> {
    _alignment: [u64; 0],
    pub(crate) key: MaybeUninit<K>,
    pub(crate) val: MaybeUninit<V>,
}

impl<K: Key, V> Slot<K, V> {
    #[inline(always)]
    fn is_zero(&self) -> bool {
        K::is_zero(&self.key)
    }
}

pub struct Table0<K: Key, V> {
    pub(crate) slots: Box<[Slot<K, V>]>,
    pub(crate) slot: Option<Box<Slot<K, V>>>,
    pub(crate) len: usize,
}

impl<K: Key, V> Table0<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(1 << 8)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: unsafe { Box::new_zeroed_slice(std::cmp::max(32, capacity)).assume_init() },
            slot: None,
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn capacity(&self) -> usize {
        self.slots.len() + if self.slot.is_some() { 1 } else { 0 }
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        if K::equals_zero(&key) {
            if let Some(slot) = self.slot.as_ref() {
                return Some(unsafe { slot.val.assume_init_ref() });
            } else {
                return None;
            }
        }
        let hash = key.hash();
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if self.slots[i].is_zero() {
                return None;
            }
            if unsafe { self.slots[i].key.assume_init_ref() }.borrow() == key {
                return Some(unsafe { self.slots[i].val.assume_init_ref() });
            }
        }
        None
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if K::equals_zero(&key) {
            if let Some(slot) = self.slot.as_mut() {
                return Some(unsafe { slot.val.assume_init_mut() });
            } else {
                return None;
            }
        }
        let hash = key.hash();
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if self.slots[i].is_zero() {
                return None;
            }
            if unsafe { self.slots[i].key.assume_init_ref() }.borrow() == key {
                return Some(unsafe { self.slots[i].val.assume_init_mut() });
            }
        }
        None
    }
    /// # Safety
    ///
    /// The resulted `MaybeUninit` should be initialized immedidately.
    ///
    /// # Panics
    ///
    /// Panics if the hash table overflows.
    pub unsafe fn insert(&mut self, key: K) -> Result<&mut MaybeUninit<V>, &mut V> {
        if K::equals_zero(&key) {
            let escape = &mut self.slot;
            if let Some(slot) = escape {
                return Err(slot.val.assume_init_mut());
            } else {
                *escape = Some(Box::new_zeroed().assume_init());
                self.len += 1;
                return Ok(&mut escape.as_mut().unwrap().val);
            }
        }
        let hash = key.hash();
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
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
    pub fn grow(&mut self) {
        let old_capacity = self.slots.len();
        let new_capacity = self.slots.len() * (1 << if self.slots.len() < 16 { 2 } else { 1 });
        let old_layout = Layout::for_value(self.slots.as_ref());
        let new_layout = Layout::array::<Slot<K, V>>(new_capacity).unwrap();
        unsafe {
            let old_ptr = NonNull::new(self.slots.as_mut() as *mut _ as *mut u8).unwrap();
            let new_ptr = Global.grow_zeroed(old_ptr, old_layout, new_layout).unwrap();
            let new_slots = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                new_ptr.as_ptr() as *mut _,
                new_capacity,
            ));
            std::ptr::write(&mut self.slots, new_slots);
        }
        for i in 0..old_capacity {
            if K::is_zero(&self.slots[i].key) {
                continue;
            }
            let key = unsafe { self.slots[i].key.assume_init_ref() };
            let index = (K::hash(key) as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if self.slots[j].is_zero() {
                    unsafe {
                        self.slots[j] = std::ptr::read(&mut self.slots[i]);
                        self.slots[i].key.as_mut_ptr().write_bytes(0, 1);
                    }
                    break;
                }
            }
        }
        for i in old_capacity..new_capacity {
            if K::is_zero(&self.slots[i].key) {
                break;
            }
            let key = unsafe { self.slots[i].key.assume_init_ref() };
            let index = (K::hash(key) as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if self.slots[j].is_zero() {
                    unsafe {
                        self.slots[j] = std::ptr::read(&mut self.slots[i]);
                        self.slots[i].key.as_mut_ptr().write_bytes(0, 1);
                    }
                    break;
                }
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.slots
            .iter()
            .filter(|slot| !slot.is_zero())
            .chain(self.slot.iter().map(Box::as_ref))
            .map(|slot| unsafe { (slot.key.assume_init_ref(), slot.val.assume_init_ref()) })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> + '_ {
        self.slots
            .iter_mut()
            .filter(|slot| !slot.is_zero())
            .chain(self.slot.iter_mut().map(Box::as_mut))
            .map(|slot| unsafe { (slot.key.assume_init_ref(), slot.val.assume_init_mut()) })
    }
}

impl<K: Key, V> Drop for Table0<K, V> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<V>() {
            self.iter_mut().for_each(|(_, v)| unsafe {
                std::ptr::drop_in_place(v);
            });
        }
    }
}
