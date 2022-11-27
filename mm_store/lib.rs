#![feature(ptr_metadata)]
#![feature(cstr_from_bytes_until_nul)]
#![feature(variant_count)]
#![feature(map_try_insert)]
mod xattr_util;
pub mod repo;
pub mod perms;
pub use crate::repo::*;
