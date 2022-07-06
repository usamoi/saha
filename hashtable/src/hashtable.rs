use crate::encoding::EncodingHashtable;
use crate::fallback::FallbackHashtable;
use crate::inline::InlineHashtable;
use crate::inline_ref::InlineRef;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

pub struct Hashtable {
    encoding: EncodingHashtable,
    inline1: InlineHashtable<1>,
    inline2: InlineHashtable<2>,
    inline3: InlineHashtable<3>,
    fallback: FallbackHashtable,
}

impl Hashtable {
    pub fn new() -> Self {
        Self {
            encoding: EncodingHashtable::new(),
            inline1: InlineHashtable::new(),
            inline2: InlineHashtable::new(),
            inline3: InlineHashtable::new(),
            fallback: FallbackHashtable::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn len(&self) -> usize {
        self.encoding.len()
            + self.inline1.len()
            + self.inline2.len()
            + self.inline3.len()
            + self.fallback.len()
    }
    pub fn capacity(&self) -> usize {
        65536
            + self.inline1.capacity()
            + self.inline2.capacity()
            + self.inline3.capacity()
            + self.fallback.capacity()
    }
    pub fn get(&self, key: &[u8]) -> Option<u64> {
        if key.len() <= 2 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u8::MAX; 2];
            t[..key.len()].copy_from_slice(key);
            self.encoding.get(t)
        } else if key.len() <= 8 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u64::MAX; 1];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 8]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline1.get(t, hash64(&t))
        } else if key.len() <= 16 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u64::MAX; 2];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 16]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline2.get(t, hash64(&t))
        } else if key.len() <= 24 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u64::MAX; 3];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 24]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline3.get(t, hash64(&t))
        } else {
            self.fallback.get(key, hash(&key))
        }
    }
    pub fn get_mut(&mut self, key: &[u8]) -> Option<&mut u64> {
        if key.len() <= 2 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u8::MAX; 2];
            t[..key.len()].copy_from_slice(key);
            self.encoding.get_mut(t)
        } else if key.len() <= 8 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u64::MAX; 1];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 8]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline1.get_mut(t, hash64(&t))
        } else if key.len() <= 16 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u64::MAX; 2];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 16]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline2.get_mut(t, hash64(&t))
        } else if key.len() <= 24 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u64::MAX; 3];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 24]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline3.get_mut(t, hash64(&t))
        } else {
            self.fallback.get_mut(key, hash(&key))
        }
    }
    pub fn insert(&mut self, key: &[u8]) -> Result<&mut u64, &mut u64> {
        if key.len() <= 2 && key.last().copied() != Some(u8::MAX) {
            let mut t = [u8::MAX; 2];
            t[..key.len()].copy_from_slice(key);
            self.encoding.insert(t)
        } else if key.len() <= 8 && key.last().copied() != Some(u8::MAX) {
            if (self.inline1.len() + 1) * 2 > self.inline1.capacity() {
                self.inline1.grow(hash64);
            }
            let mut t = [u64::MAX; 1];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 8]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline1.insert(t, hash64(&t)).map_err(|x| x.unwrap())
        } else if key.len() <= 16 && key.last().copied() != Some(u8::MAX) {
            if (self.inline2.len() + 1) * 2 > self.inline2.capacity() {
                self.inline2.grow(hash64);
            }
            let mut t = [u64::MAX; 2];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 16]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline2.insert(t, hash64(&t)).map_err(|x| x.unwrap())
        } else if key.len() <= 24 && key.last().copied() != Some(u8::MAX) {
            if (self.inline3.len() + 1) * 2 > self.inline3.capacity() {
                self.inline3.grow(hash64);
            }
            let mut t = [u64::MAX; 3];
            unsafe {
                std::mem::transmute::<_, &mut [u8; 24]>(&mut t)[0..key.len()].copy_from_slice(key);
            }
            self.inline3.insert(t, hash64(&t)).map_err(|x| x.unwrap())
        } else {
            if (self.fallback.len() + 1) * 2 > self.fallback.capacity() {
                self.fallback.grow();
            }
            self.fallback
                .insert(key, hash(&key))
                .map_err(|x| x.unwrap())
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (InlineRef<'_>, u64)> {
        self.fallback
            .iter()
            .map(|(key, value)| (InlineRef::new(key), value))
            .chain(self.inline1.iter().map(|(key, value)| {
                let mut bytes = [0u8; 24];
                bytes[0..8].copy_from_slice(&unsafe { std::mem::transmute::<_, [u8; 8]>(key) });
                for i in (2..8).rev() {
                    if bytes[i as usize] != u8::MAX {
                        return (InlineRef::new_owned(&bytes[0..=i]).unwrap(), value);
                    }
                }
                unreachable!()
            }))
            .chain(self.inline2.iter().map(|(key, value)| {
                let mut bytes = [0u8; 24];
                bytes[0..16].copy_from_slice(&unsafe { std::mem::transmute::<_, [u8; 16]>(key) });
                for i in (8..16).rev() {
                    if bytes[i as usize] != u8::MAX {
                        return (InlineRef::new_owned(&bytes[0..=i]).unwrap(), value);
                    }
                }
                unreachable!()
            }))
            .chain(self.inline3.iter().map(|(key, value)| {
                let mut bytes = [0u8; 24];
                bytes[0..24].copy_from_slice(&unsafe { std::mem::transmute::<_, [u8; 24]>(key) });
                for i in (16..24).rev() {
                    if bytes[i as usize] != u8::MAX {
                        return (InlineRef::new_owned(&bytes[0..=i]).unwrap(), value);
                    }
                }
                unreachable!()
            }))
            .chain(self.encoding.iter().map(|(key, value)| match key {
                [u8::MAX, u8::MAX] => (InlineRef::new(&[]), value),
                [x0, u8::MAX] => (InlineRef::new_owned(&[x0]).unwrap(), value),
                [x0, x1] => (InlineRef::new_owned(&[x0, x1]).unwrap(), value),
            }))
    }
}

fn hash(key: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::default();
    hasher.write(&key);
    hasher.finish()
}

fn hash64(key: &[u64]) -> u64 {
    let mut hasher = DefaultHasher::default();
    for &i in key {
        hasher.write_u64(i);
    }
    hasher.finish()
}
