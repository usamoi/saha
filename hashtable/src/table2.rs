use crate::traits::Key;
use std::alloc::{Allocator, Global, Layout};
use std::mem::MaybeUninit;
use std::ptr::NonNull;

pub(crate) struct Slot<K, V> {
    pub(crate) key: MaybeUninit<K>,
    pub(crate) val: MaybeUninit<V>,
}

pub struct Table2<K: Key, V> {
    pub(crate) keys: Box<[MaybeUninit<K>]>,
    pub(crate) vals: Box<[MaybeUninit<V>]>,
    pub(crate) slot: Option<Box<Slot<K, V>>>,
    pub(crate) len: usize,
}

impl<K: Key, V> Table2<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(1 << 8)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            keys: unsafe { Box::new_zeroed_slice(capacity).assume_init() },
            vals: unsafe { Box::new_zeroed_slice(capacity).assume_init() },
            slot: None,
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn capacity(&self) -> usize {
        self.keys.len()
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
        let index = (hash as usize) & (self.keys.len() - 1);
        for i in (index..self.keys.len()).chain(0..index) {
            if K::is_zero(&self.keys[i]) {
                return None;
            }
            if unsafe { self.keys[i].assume_init_ref() } == key {
                return Some(unsafe { self.vals[i].assume_init_ref() });
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
        let index = (hash as usize) & (self.keys.len() - 1);
        for i in (index..self.keys.len()).chain(0..index) {
            if K::is_zero(&self.keys[i]) {
                return None;
            }
            if unsafe { self.keys[i].assume_init_ref() } == key {
                return Some(unsafe { self.vals[i].assume_init_mut() });
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
                let escape = escape.as_mut().unwrap();
                self.len += 1;
                return Ok(&mut escape.val);
            }
        }
        let hash = key.hash();
        let index = (hash as usize) & (self.keys.len() - 1);
        for i in (index..self.keys.len()).chain(0..index) {
            if K::is_zero(&self.keys[i]) {
                self.len += 1;
                self.keys[i].write(key);
                return Ok(&mut self.vals[i]);
            }
            if self.keys[i].assume_init_ref() == &key {
                return Err(self.vals[i].assume_init_mut());
            }
        }
        panic!("the hash table overflows")
    }
    pub fn grow(&mut self) {
        let old_capacity = self.keys.len();
        let new_capacity = self.keys.len() * (1 << if self.keys.len() < 16 { 2 } else { 1 });
        let old_layout_keys = Layout::for_value(self.keys.as_ref());
        let new_layout_keys = Layout::array::<K>(new_capacity).unwrap();
        let old_layout_vals = Layout::for_value(self.vals.as_ref());
        let new_layout_vals = Layout::array::<V>(new_capacity).unwrap();
        unsafe {
            let old_ptr_keys = NonNull::new(self.keys.as_mut() as *mut _ as *mut u8).unwrap();
            let old_ptr_vals = NonNull::new(self.vals.as_mut() as *mut _ as *mut u8).unwrap();
            let new_ptr_keys = Global
                .grow_zeroed(old_ptr_keys, old_layout_keys, new_layout_keys)
                .unwrap();
            let new_ptr_vals = Global
                .grow(old_ptr_vals, old_layout_vals, new_layout_vals)
                .unwrap();
            let new_keys = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                new_ptr_keys.as_ptr() as *mut _,
                new_capacity,
            ));
            let new_vals = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                new_ptr_vals.as_ptr() as *mut _,
                new_capacity,
            ));
            std::ptr::write(&mut self.keys, new_keys);
            std::ptr::write(&mut self.vals, new_vals);
        }
        for i in 0..old_capacity {
            if K::is_zero(&self.keys[i]) {
                continue;
            }
            let key = unsafe { self.keys[i].assume_init_ref() };
            let index = (K::hash(key) as usize) & (self.keys.len() - 1);
            for j in (index..self.keys.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if K::is_zero(&self.keys[j]) {
                    unsafe {
                        self.keys[j] = std::ptr::read(&mut self.keys[i]);
                        self.vals[j] = std::ptr::read(&mut self.vals[i]);
                        self.keys[i].as_mut_ptr().write_bytes(0, 1);
                    }
                    break;
                }
            }
        }
        for i in old_capacity..new_capacity {
            if K::is_zero(&self.keys[i]) {
                break;
            }
            let key = unsafe { self.keys[i].assume_init_ref() };
            let index = (K::hash(key) as usize) & (self.keys.len() - 1);
            for j in (index..self.keys.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if K::is_zero(&self.keys[j]) {
                    unsafe {
                        self.keys[j] = std::ptr::read(&mut self.keys[i]);
                        self.vals[j] = std::ptr::read(&mut self.vals[i]);
                        self.keys[i].as_mut_ptr().write_bytes(0, 1);
                    }
                    break;
                }
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        (0..self.keys.len())
            .filter(|&i| !K::is_zero(&self.keys[i]))
            .map(|i| unsafe {
                (
                    self.keys[i].assume_init_ref(),
                    self.vals[i].assume_init_ref(),
                )
            })
            .chain(
                self.slot
                    .iter()
                    .map(|x| unsafe { (x.key.assume_init_ref(), x.val.assume_init_ref()) }),
            )
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> + '_ {
        (0..self.keys.len())
            .filter(|&i| !K::is_zero(&self.keys[i]))
            .map(|i| unsafe {
                (
                    self.keys[i].assume_init_ref(),
                    &mut *(self.vals[i].assume_init_mut() as *mut _),
                )
            })
            .chain(
                self.slot
                    .iter_mut()
                    .map(|x| unsafe { (x.key.assume_init_ref(), x.val.assume_init_mut()) }),
            )
    }
}

impl<K: Key, V> Drop for Table2<K, V> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<V>() {
            self.iter_mut().for_each(|(_, v)| unsafe {
                std::ptr::drop_in_place(v);
            });
        }
    }
}
