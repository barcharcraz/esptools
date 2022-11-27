// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

#![feature(ptr_metadata)]
#![feature(cstr_from_bytes_until_nul)]
#![feature(variant_count)]
#![feature(map_try_insert)]
#![feature(new_uninit)]
#![feature(read_buf)]

mod xattr_util;
pub mod repo;
pub mod perms;
pub use crate::repo::*;
