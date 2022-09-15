#![feature(allocator_api)]
#![feature(core_intrinsics)]
#![feature(new_uninit)]
#![feature(portable_simd)]
#![feature(ptr_metadata)]
#![feature(trivial_bounds)]
#![feature(let_else)]
#![feature(maybe_uninit_slice)]
#![feature(once_cell)]
#![allow(clippy::new_without_default)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::type_complexity)]

pub mod allocator;
pub mod container;
pub mod hash;
pub mod traits;

pub mod hashtable;
pub mod twolevel_hashtable;
pub mod unsized_hashtable;

mod simd;
mod table0;
mod table1;
mod utils;

pub mod experimental;
