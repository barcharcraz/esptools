// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

#![feature(ptr_metadata)]
#![feature(variant_count)]
#![feature(map_try_insert)]
#![feature(new_uninit)]
#![feature(read_buf)]
#![feature(error_generic_member_access)]
#![feature(provide_any)]
#![feature(min_specialization)]
#![feature(rustc_attrs)]
#![feature(maybe_uninit_uninit_array)]
#![feature(concat_idents)]

#[cfg(windows)]
mod xattr_util;
mod keyfile;
pub mod repo;


pub mod mutable_tree;
pub mod perms;
pub mod archive;
pub use crate::repo::*;
