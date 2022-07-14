#![feature(allocator_api)]
#![feature(new_uninit)]
#![feature(portable_simd)]
#![feature(trivial_bounds)]

pub mod batch_hashtable;
pub mod hashtable;
pub mod traits;
pub mod unsized_hashtable;

mod table0;
mod table1;
mod table2;
