use crate::table0::Table0;
use crate::table1::Table1;
use crate::traits::{Key, UnsizedKey};
use bumpalo::Bump;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::num::NonZeroU64;
use std::ptr::NonNull;

pub struct UnsizedHashtable<K: UnsizedKey + ?Sized, V> {
    arena: Bump,
    raw0: Table1<V>,
    raw1: Table0<InlineKey<0>, V>,
    raw2: Table0<InlineKey<1>, V>,
    raw3: Table0<InlineKey<2>, V>,
    raw4: Table0<FallbackKey, V>,
    _phantom: PhantomData<K>,
}

impl<K: UnsizedKey + ?Sized, V> UnsizedHashtable<K, V> {
    pub fn new() -> Self {
        Self {
            arena: Bump::new(),
            raw0: Table1::new(),
            raw1: Table0::new(),
            raw2: Table0::new(),
            raw3: Table0::new(),
            raw4: Table0::new(),
            _phantom: PhantomData,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn len(&self) -> usize {
        self.raw0.len() + self.raw1.len() + self.raw2.len() + self.raw3.len() + self.raw4.len()
    }
    pub fn capacity(&self) -> usize {
        self.raw0.capacity()
            + self.raw1.capacity()
            + self.raw2.capacity()
            + self.raw3.capacity()
            + self.raw4.capacity()
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let key = key.as_bytes();
        match key.len() {
            _ if key.last().copied() == Some(0) => self.raw4.get(&FallbackKey::new(key)),
            0 => self.raw0.get([0, 0]),
            1 => self.raw0.get([key[0], 0]),
            2 => self.raw0.get([key[0], key[1]]),
            3..=8 => unsafe {
                let mut t = [0u64; 1];
                t[0] = read_little(key.as_ptr(), key.len());
                let t = std::mem::transmute::<_, InlineKey<0>>(t);
                self.raw1.get(&t)
            },
            9..=16 => unsafe {
                let mut t = [0u64; 2];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = read_little(key.as_ptr().offset(8), key.len() - 8);
                let t = std::mem::transmute::<_, InlineKey<1>>(t);
                self.raw2.get(&t)
            },
            17..=24 => unsafe {
                let mut t = [0u64; 3];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = (key.as_ptr() as *const u64).offset(1).read_unaligned();
                t[2] = read_little(key.as_ptr().offset(16), key.len() - 16);
                let t = std::mem::transmute::<_, InlineKey<2>>(t);
                self.raw3.get(&t)
            },
            _ => self.raw4.get(&FallbackKey::new(key)),
        }
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let key = key.as_bytes();
        match key.len() {
            _ if key.last().copied() == Some(0) => self.raw4.get_mut(&FallbackKey::new(key)),
            0 => self.raw0.get_mut([0, 0]),
            1 => self.raw0.get_mut([key[0], 0]),
            2 => self.raw0.get_mut([key[0], key[1]]),
            3..=8 => unsafe {
                let mut t = [0u64; 1];
                t[0] = read_little(key.as_ptr(), key.len());
                let t = std::mem::transmute::<_, InlineKey<0>>(t);
                self.raw1.get_mut(&t)
            },
            9..=16 => unsafe {
                let mut t = [0u64; 2];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = read_little(key.as_ptr().offset(8), key.len() - 8);
                let t = std::mem::transmute::<_, InlineKey<1>>(t);
                self.raw2.get_mut(&t)
            },
            17..=24 => unsafe {
                let mut t = [0u64; 3];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = (key.as_ptr() as *const u64).offset(1).read_unaligned();
                t[2] = read_little(key.as_ptr().offset(16), key.len() - 16);
                let t = std::mem::transmute::<_, InlineKey<2>>(t);
                self.raw3.get_mut(&t)
            },
            _ => self.raw4.get_mut(&FallbackKey::new(key)),
        }
    }
    pub unsafe fn insert(&mut self, key: &K) -> Result<&mut MaybeUninit<V>, &mut V> {
        let key = key.as_bytes();
        match key.len() {
            _ if key.last().copied() == Some(0) => {
                if (self.raw4.len() + 1) * 2 > self.raw4.capacity() {
                    self.raw4.grow();
                }
                let s = self.arena.alloc_slice_copy(key);
                self.raw4.insert(FallbackKey::new(s))
            }
            0 => self.raw0.insert([0, 0]),
            1 => self.raw0.insert([key[0], 0]),
            2 => self.raw0.insert([key[0], key[1]]),
            3..=8 => {
                if (self.raw1.len() + 1) * 2 > self.raw1.capacity() {
                    self.raw1.grow();
                }
                let mut t = [0u64; 1];
                t[0] = read_little(key.as_ptr(), key.len());
                let t = std::mem::transmute::<_, InlineKey<0>>(t);
                self.raw1.insert(t)
            }
            9..=16 => {
                if (self.raw2.len() + 1) * 2 > self.raw2.capacity() {
                    self.raw2.grow();
                }
                let mut t = [0u64; 2];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = read_little(key.as_ptr().offset(8), key.len() - 8);
                let t = std::mem::transmute::<_, InlineKey<1>>(t);
                self.raw2.insert(t)
            }
            17..=24 => {
                if (self.raw3.len() + 1) * 2 > self.raw3.capacity() {
                    self.raw3.grow();
                }
                let mut t = [0u64; 3];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = (key.as_ptr() as *const u64).offset(1).read_unaligned();
                t[2] = read_little(key.as_ptr().offset(16), key.len() - 16);
                let t = std::mem::transmute::<_, InlineKey<2>>(t);
                self.raw3.insert(t)
            }
            _ => {
                if (self.raw4.len() + 1) * 2 > self.raw4.capacity() {
                    self.raw4.grow();
                }
                let s = self.arena.alloc_slice_copy(key);
                self.raw4.insert(FallbackKey::new(s))
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.raw4
            .iter()
            .map(|(key, value)| {
                (
                    unsafe { UnsizedKey::from_bytes(key.key.unwrap().as_ref()) },
                    value,
                )
            })
            .chain(self.raw1.iter().map(|(key, value)| {
                let bytes = key.1.get().to_le_bytes();
                unsafe {
                    for i in (0..=7).rev() {
                        if bytes[i] != 0 {
                            return (
                                UnsizedKey::from_bytes(std::slice::from_raw_parts(
                                    key as *const _ as *const u8,
                                    i + 1,
                                )),
                                value,
                            );
                        }
                    }
                }
                unreachable!()
            }))
            .chain(self.raw2.iter().map(|(key, value)| {
                let bytes = key.1.get().to_le_bytes();
                unsafe {
                    for i in (0..=7).rev() {
                        if bytes[i] != 0 {
                            return (
                                UnsizedKey::from_bytes(std::slice::from_raw_parts(
                                    key as *const _ as *const u8,
                                    i + 9,
                                )),
                                value,
                            );
                        }
                    }
                }
                unreachable!()
            }))
            .chain(self.raw3.iter().map(|(key, value)| {
                let bytes = key.1.get().to_le_bytes();
                unsafe {
                    for i in (0..=7).rev() {
                        if bytes[i] != 0 {
                            return (
                                UnsizedKey::from_bytes(std::slice::from_raw_parts(
                                    key as *const _ as *const u8,
                                    i + 17,
                                )),
                                value,
                            );
                        }
                    }
                }
                unreachable!()
            }))
            .chain(self.raw0.iter().map(|(key, value)| unsafe {
                if key[1] != 0 {
                    (UnsizedKey::from_bytes(&key[..2]), value)
                } else if key[0] != 0 {
                    (UnsizedKey::from_bytes(&key[..1]), value)
                } else {
                    (UnsizedKey::from_bytes(&key[..0]), value)
                }
            }))
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct InlineKey<const N: usize>(pub [u64; N], pub NonZeroU64);

unsafe impl<const N: usize> Key for InlineKey<N> {
    #[inline(always)]
    fn equals_zero(_: &Self) -> bool {
        false
    }

    #[inline(always)]
    fn is_zero(this: &MaybeUninit<Self>) -> bool {
        unsafe { *(this as *const _ as *const u64).add(N) == 0 }
    }

    #[inline(always)]
    fn hash(&self) -> u64 {
        use std::arch::x86_64::_mm_crc32_u64;
        let mut hasher = u64::MAX;
        for x in self.0 {
            hasher = unsafe { _mm_crc32_u64(hasher, x) };
        }
        hasher = unsafe { _mm_crc32_u64(hasher, self.1.get()) };
        hasher
    }
}

#[derive(Copy, Clone)]
struct FallbackKey {
    key: Option<NonNull<[u8]>>,
    hash: u64,
}

impl FallbackKey {
    fn new(key: &[u8]) -> Self {
        Self {
            key: Some(NonNull::from(key)),
            hash: {
                use std::arch::x86_64::_mm_crc32_u64;
                let mut hasher = u64::MAX;
                for i in (0..key.len()).step_by(8) {
                    if i + 8 < key.len() {
                        unsafe {
                            let x = (&key[i] as *const u8 as *const u64).read_unaligned();
                            hasher = _mm_crc32_u64(hasher, x);
                        };
                    } else {
                        unsafe {
                            let x = read_little(&key[i] as *const u8, key.len() - i);
                            hasher = _mm_crc32_u64(hasher, x);
                        }
                    }
                }
                hasher
            },
        }
    }
}

impl PartialEq for FallbackKey {
    fn eq(&self, other: &Self) -> bool {
        if self.hash == other.hash {
            unsafe { self.key.map(|x| x.as_ref()) == other.key.map(|x| x.as_ref()) }
        } else {
            false
        }
    }
}

impl Eq for FallbackKey {}

unsafe impl Key for FallbackKey {
    #[inline(always)]
    fn equals_zero(_: &Self) -> bool {
        false
    }

    #[inline(always)]
    fn is_zero(this: &MaybeUninit<Self>) -> bool {
        unsafe { this.assume_init_ref().key.is_none() }
    }

    #[inline(always)]
    fn hash(&self) -> u64 {
        self.hash
    }
}

#[inline(always)]
fn read_little(data: *const u8, len: usize) -> u64 {
    debug_assert!(0 < len && len <= 8);
    let s = 64 - 8 * len as isize;
    unsafe {
        if data as usize & 2048 == 0 {
            (data as *const u64).read_unaligned() & (u64::MAX >> s)
        } else {
            (data.offset(len as isize - 8) as *const u64).read_unaligned() >> s
        }
    }
}
