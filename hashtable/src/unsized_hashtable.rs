use crate::container::HeapContainer;
use crate::table0::{Slot, Table0};
use crate::table1::Table1;
use crate::traits::{FastHash, Key, UnsizedKey};
use crate::utils::read_le;
use bumpalo::Bump;
use std::alloc::Allocator;
use std::intrinsics::unlikely;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::num::NonZeroU64;
use std::ptr::NonNull;

pub struct UnsizedHashtable<K, V, A = crate::allocator::Default>
where
    K: UnsizedKey + ?Sized,
    A: Allocator + Clone,
{
    pub(crate) arena: Bump,
    pub(crate) table0: Table1<V, A>,
    pub(crate) table1: Table0<InlineKey<0>, V, HeapContainer<Slot<InlineKey<0>, V>, A>, A>,
    pub(crate) table2: Table0<InlineKey<1>, V, HeapContainer<Slot<InlineKey<1>, V>, A>, A>,
    pub(crate) table3: Table0<InlineKey<2>, V, HeapContainer<Slot<InlineKey<2>, V>, A>, A>,
    pub(crate) table4: Table0<FallbackKey, V, HeapContainer<Slot<FallbackKey, V>, A>, A>,
    pub(crate) _phantom: PhantomData<K>,
}

impl<K, V, A> UnsizedHashtable<K, V, A>
where
    K: UnsizedKey + ?Sized,
    A: Allocator + Clone + Default,
{
    pub fn new() -> Self {
        Self::new_in(Default::default())
    }
}

impl<K, V, A> UnsizedHashtable<K, V, A>
where
    K: UnsizedKey + ?Sized,
    A: Allocator + Clone + Default,
{
    /// The bump for strings doesn't allocate memory by `A`.
    pub fn new_in(allocator: A) -> Self {
        Self {
            arena: Bump::new(),
            table0: Table1::new_in(allocator.clone()),
            table1: Table0::with_capacity_in(128, allocator.clone()),
            table2: Table0::with_capacity_in(128, allocator.clone()),
            table3: Table0::with_capacity_in(128, allocator.clone()),
            table4: Table0::with_capacity_in(128, allocator),
            _phantom: PhantomData,
        }
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.table0.len()
            + self.table1.len()
            + self.table2.len()
            + self.table3.len()
            + self.table4.len()
    }
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.table0.capacity()
            + self.table1.capacity()
            + self.table2.capacity()
            + self.table3.capacity()
            + self.table4.capacity()
    }
    #[inline(always)]
    pub fn get(&self, key: &K) -> Option<&V> {
        let key = key.as_bytes();
        match key.len() {
            _ if key.last().copied() == Some(0) => unsafe {
                self.table4.get(&FallbackKey::new(key))
            },
            0 => self.table0.get([0, 0]),
            1 => self.table0.get([key[0], 0]),
            2 => self.table0.get([key[0], key[1]]),
            3..=8 => unsafe {
                let mut t = [0u64; 1];
                t[0] = read_le(key.as_ptr(), key.len());
                let t = std::mem::transmute::<_, InlineKey<0>>(t);
                self.table1.get(&t)
            },
            9..=16 => unsafe {
                let mut t = [0u64; 2];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = read_le(key.as_ptr().offset(8), key.len() - 8);
                let t = std::mem::transmute::<_, InlineKey<1>>(t);
                self.table2.get(&t)
            },
            17..=24 => unsafe {
                let mut t = [0u64; 3];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = (key.as_ptr() as *const u64).offset(1).read_unaligned();
                t[2] = read_le(key.as_ptr().offset(16), key.len() - 16);
                let t = std::mem::transmute::<_, InlineKey<2>>(t);
                self.table3.get(&t)
            },
            _ => unsafe { self.table4.get(&FallbackKey::new(key)) },
        }
    }
    #[inline(always)]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let key = key.as_bytes();
        match key.len() {
            _ if key.last().copied() == Some(0) => unsafe {
                self.table4.get_mut(&FallbackKey::new(key))
            },
            0 => self.table0.get_mut([0, 0]),
            1 => self.table0.get_mut([key[0], 0]),
            2 => self.table0.get_mut([key[0], key[1]]),
            3..=8 => unsafe {
                let mut t = [0u64; 1];
                t[0] = read_le(key.as_ptr(), key.len());
                let t = std::mem::transmute::<_, InlineKey<0>>(t);
                self.table1.get_mut(&t)
            },
            9..=16 => unsafe {
                let mut t = [0u64; 2];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = read_le(key.as_ptr().offset(8), key.len() - 8);
                let t = std::mem::transmute::<_, InlineKey<1>>(t);
                self.table2.get_mut(&t)
            },
            17..=24 => unsafe {
                let mut t = [0u64; 3];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = (key.as_ptr() as *const u64).offset(1).read_unaligned();
                t[2] = read_le(key.as_ptr().offset(16), key.len() - 16);
                let t = std::mem::transmute::<_, InlineKey<2>>(t);
                self.table3.get_mut(&t)
            },
            _ => unsafe { self.table4.get_mut(&FallbackKey::new(key)) },
        }
    }
    #[inline(always)]
    pub unsafe fn insert(&mut self, key: &K) -> Result<&mut MaybeUninit<V>, &mut V> {
        let key = key.as_bytes();
        match key.len() {
            _ if key.last().copied() == Some(0) => {
                if unlikely((self.table4.len() + 1) * 2 > self.table4.capacity()) {
                    if (self.table4.slots.len() >> 22) == 0 {
                        self.table4.grow(2);
                    } else {
                        self.table4.grow(1);
                    }
                }
                let s = self.arena.alloc_slice_copy(key);
                self.table4.insert(FallbackKey::new(s))
            }
            0 => self.table0.insert([0, 0]),
            1 => self.table0.insert([key[0], 0]),
            2 => self.table0.insert([key[0], key[1]]),
            3..=8 => {
                if unlikely((self.table1.len() + 1) * 2 > self.table1.capacity()) {
                    if (self.table1.slots.len() >> 22) == 0 {
                        self.table1.grow(2);
                    } else {
                        self.table1.grow(1);
                    }
                }
                let mut t = [0u64; 1];
                t[0] = read_le(key.as_ptr(), key.len());
                let t = std::mem::transmute::<_, InlineKey<0>>(t);
                self.table1.insert(t)
            }
            9..=16 => {
                if unlikely((self.table2.len() + 1) * 2 > self.table2.capacity()) {
                    if (self.table2.slots.len() >> 22) == 0 {
                        self.table2.grow(2);
                    } else {
                        self.table2.grow(1);
                    }
                }
                let mut t = [0u64; 2];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = read_le(key.as_ptr().offset(8), key.len() - 8);
                let t = std::mem::transmute::<_, InlineKey<1>>(t);
                self.table2.insert(t)
            }
            17..=24 => {
                if unlikely((self.table3.len() + 1) * 2 > self.table3.capacity()) {
                    if (self.table3.slots.len() >> 22) == 0 {
                        self.table3.grow(2);
                    } else {
                        self.table3.grow(1);
                    }
                }
                let mut t = [0u64; 3];
                t[0] = (key.as_ptr() as *const u64).read_unaligned();
                t[1] = (key.as_ptr() as *const u64).offset(1).read_unaligned();
                t[2] = read_le(key.as_ptr().offset(16), key.len() - 16);
                let t = std::mem::transmute::<_, InlineKey<2>>(t);
                self.table3.insert(t)
            }
            _ => {
                if unlikely((self.table4.len() + 1) * 2 > self.table4.capacity()) {
                    if (self.table4.slots.len() >> 22) == 0 {
                        self.table4.grow(2);
                    } else {
                        self.table4.grow(1);
                    }
                }
                let s = self.arena.alloc_slice_copy(key);
                self.table4.insert(FallbackKey::new(s))
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.table4
            .iter()
            .map(|(key, value)| {
                (
                    unsafe { UnsizedKey::from_bytes(key.key.unwrap().as_ref()) },
                    value,
                )
            })
            .chain(self.table1.iter().map(|(key, value)| {
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
            .chain(self.table2.iter().map(|(key, value)| {
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
            .chain(self.table3.iter().map(|(key, value)| {
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
            .chain(self.table0.iter().map(|(key, value)| unsafe {
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
pub(crate) struct InlineKey<const N: usize>(pub [u64; N], pub NonZeroU64);

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
        (self.0, self.1).fast_hash()
    }
}

#[derive(Copy, Clone)]
pub(crate) struct FallbackKey {
    key: Option<NonNull<[u8]>>,
    hash: u64,
}

impl FallbackKey {
    unsafe fn new(key: &[u8]) -> Self {
        Self {
            key: Some(NonNull::from(key)),
            hash: key.fast_hash(),
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
