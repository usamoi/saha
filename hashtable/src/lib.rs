#![feature(allocator_api)]
#![feature(new_uninit)]
#![feature(portable_simd)]
#![feature(generic_associated_types)]

pub mod adaptive_hashtable;
pub mod traits;

mod hash;
mod table0;
mod table1;
