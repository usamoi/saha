use crate::table0::Table0;
use crate::traits::Key;
use std::mem::MaybeUninit;

pub struct Hashtable<K: Key, V> {
    raw: Table0<K, V>,
}

impl<K: Key, V> Hashtable<K, V> {
    pub fn new() -> Self {
        Self { raw: Table0::new() }
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
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.raw.iter()
    }
}
