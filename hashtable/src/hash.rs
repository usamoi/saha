use crate::traits::FastHash;
use std::num::NonZeroU64;

const CRC_A: u32 = u32::MAX;
const CRC_B: u32 = 0;

macro_rules! impl_fast_hash_for_primitive_types {
    ($t: ty) => {
        impl FastHash for $t {
            #[inline(always)]
            fn fast_hash(&self) -> u64 {
                cfg_if::cfg_if! {
                    if #[cfg(target_feature = "sse4.2")] {
                        use std::arch::x86_64::_mm_crc32_u64;
                        let mut high = CRC_A;
                        let mut low = CRC_B;
                        high = unsafe { _mm_crc32_u64(high as u64, *self as u64) as u32 };
                        low = unsafe { _mm_crc32_u64(low as u64, *self as u64) as u32 };
                        (high as u64) << 32 | low as u64
                    } else {
                        let mut hasher = *self as u64;
                        hasher ^= hasher >> 33;
                        hasher = hasher.wrapping_mul(0xff51afd7ed558ccd_u64);
                        hasher ^= hasher >> 33;
                        hasher = hasher.wrapping_mul(0xc4ceb9fe1a85ec53_u64);
                        hasher ^= hasher >> 33;
                        hasher
                    }
                }
            }
        }
    };
}

impl_fast_hash_for_primitive_types!(u8);
impl_fast_hash_for_primitive_types!(i8);
impl_fast_hash_for_primitive_types!(u16);
impl_fast_hash_for_primitive_types!(i16);
impl_fast_hash_for_primitive_types!(u32);
impl_fast_hash_for_primitive_types!(i32);
impl_fast_hash_for_primitive_types!(u64);
impl_fast_hash_for_primitive_types!(i64);

impl<const N: usize> FastHash for ([u64; N], NonZeroU64) {
    #[inline(always)]
    fn fast_hash(&self) -> u64 {
        cfg_if::cfg_if! {
            if #[cfg(target_feature = "sse4.2")] {
                use std::arch::x86_64::_mm_crc32_u64;
                let mut high = CRC_A;
                let mut low = CRC_B;
                for x in self.0 {
                    high = unsafe { _mm_crc32_u64(high as u64, x) as u32 };
                    low = unsafe { _mm_crc32_u64(low as u64, x) as u32 };
                }
                high = unsafe { _mm_crc32_u64(high as u64, self.1.get()) as u32 };
                low = unsafe { _mm_crc32_u64(low as u64, self.1.get()) as u32 };
                (high as u64) << 32 | low as u64
            } else {
                use std::hash::Hasher;
                let mut hasher = ahash::AHasher::default();
                for x in self.0 {
                    hasher.write_u64(x);
                }
                hasher.write_u64(self.1.get());
                hasher.finish()
            }
        }
    }
}

impl FastHash for [u8] {
    #[inline(always)]
    fn fast_hash(&self) -> u64 {
        cfg_if::cfg_if! {
            if #[cfg(target_feature = "sse4.2")] {
                use crate::utils::read_le;
                use std::arch::x86_64::_mm_crc32_u64;
                let mut high = CRC_A;
                let mut low = CRC_B;
                for i in (0..self.len()).step_by(8) {
                    if i + 8 < self.len() {
                        unsafe {
                            let x = (&self[i] as *const u8 as *const u64).read_unaligned();
                            high = _mm_crc32_u64(high as u64, x) as u32;
                            low = _mm_crc32_u64(low as u64, x) as u32;
                        }
                    } else {
                        unsafe {
                            let x = read_le(&self[i] as *const u8, self.len() - i);
                            high = _mm_crc32_u64(high as u64, x) as u32;
                            low = _mm_crc32_u64(low as u64, x) as u32;
                        }
                    }
                }
                (high as u64) << 32 | low as u64
            } else {
                use std::hash::Hasher;
                let mut hasher = ahash::AHasher::default();
                hasher.write(self);
                hasher.finish()
            }
        }
    }
}
