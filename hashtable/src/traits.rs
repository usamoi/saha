use std::alloc::Allocator;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

pub trait FastHash {
    fn fast_hash(&self) -> u64;
}

pub unsafe trait Key: Sized + Copy + Eq {
    fn is_zero(this: &MaybeUninit<Self>) -> bool;

    fn equals_zero(this: &Self) -> bool;

    fn hash(&self) -> u64;
}

pub trait UnsizedKey {
    fn as_bytes(&self) -> &[u8];

    unsafe fn from_bytes(bytes: &[u8]) -> &Self;
}

macro_rules! impl_key_for_primitive_types {
    ($t: ty) => {
        unsafe impl Key for $t {
            #[inline(always)]
            fn equals_zero(this: &Self) -> bool {
                *this == 0
            }

            #[inline(always)]
            fn is_zero(this: &MaybeUninit<Self>) -> bool {
                unsafe { this.assume_init() == 0 }
            }

            #[inline(always)]
            fn hash(&self) -> u64 {
                self.fast_hash()
            }
        }
    };
}

impl_key_for_primitive_types!(u8);
impl_key_for_primitive_types!(i8);
impl_key_for_primitive_types!(u16);
impl_key_for_primitive_types!(i16);
impl_key_for_primitive_types!(u32);
impl_key_for_primitive_types!(i32);
impl_key_for_primitive_types!(u64);
impl_key_for_primitive_types!(i64);

impl UnsizedKey for [u8] {
    fn as_bytes(&self) -> &[u8] {
        self
    }

    unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        bytes
    }
}

impl UnsizedKey for str {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        std::str::from_utf8_unchecked(bytes)
    }
}

pub unsafe trait Container
where
    Self: Deref<Target = [Self::T]> + DerefMut,
{
    type T;

    type A: Allocator;

    fn len(&self) -> usize;

    unsafe fn new_zeroed(len: usize, allocator: Self::A) -> Self;

    unsafe fn grow_zeroed(&mut self, new_len: usize);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WithHash<K: Key> {
    hash: u64,
    key: K,
}

impl<K: Key> WithHash<K> {
    #[inline(always)]
    pub fn new(key: K) -> Self {
        Self {
            hash: K::hash(&key),
            key,
        }
    }
    #[inline(always)]
    pub fn key(self) -> K {
        self.key
    }
}

unsafe impl<K: Key> Key for WithHash<K> {
    #[inline(always)]
    fn is_zero(this: &MaybeUninit<Self>) -> bool {
        unsafe {
            let addr = std::ptr::addr_of!((*this.as_ptr()).key);
            K::is_zero(&*(addr as *const MaybeUninit<K>))
        }
    }

    #[inline(always)]
    fn equals_zero(this: &Self) -> bool {
        K::equals_zero(&this.key)
    }

    #[inline(always)]
    fn hash(&self) -> u64 {
        self.hash
    }
}

pub unsafe trait CuckooKey: Key {
    fn left_hash(&self) -> u64 {
        self.hash()
    }

    fn right_hash(&self) -> u64;
}

unsafe impl CuckooKey for u64 {
    fn right_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(*self);
        hasher.finish()
    }
}
