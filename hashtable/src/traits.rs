use core_simd::SimdElement;
use std::mem::MaybeUninit;

pub unsafe trait Key: Sized + Copy + Eq {
    fn is_zero(this: &MaybeUninit<Self>) -> bool;

    fn equals_zero(this: &Self) -> bool;

    fn hash(&self) -> u64;
}

pub unsafe trait UnsizedKey {
    fn as_bytes(&self) -> &[u8];

    unsafe fn from_bytes(bytes: &[u8]) -> &Self;
}

pub unsafe trait BatchKey: Key + Default + SimdElement {}

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
                unsafe { std::arch::x86_64::_mm_crc32_u64(u64::MAX, *self as u64) }
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

unsafe impl UnsizedKey for [u8] {
    fn as_bytes(&self) -> &[u8] {
        self
    }

    unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        bytes
    }
}

unsafe impl UnsizedKey for str {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        std::str::from_utf8_unchecked(bytes)
    }
}

unsafe impl BatchKey for u64 {}
