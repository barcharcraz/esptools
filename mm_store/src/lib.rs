// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

#![feature(ptr_metadata)]
#![feature(variant_count)]
#![feature(map_try_insert)]
#![feature(read_buf)]
#![feature(error_generic_member_access)]
#![feature(rustc_attrs)]
//#![feature(provide_any)]
#![feature(macro_metavar_expr_concat)]
#![feature(min_specialization)]
#![feature(core_io_borrowed_buf)]

#[cfg(windows)]
mod xattr_util;
mod keyfile;
pub mod repo;


pub mod mutable_tree;
pub mod perms;
pub mod archive;
pub use crate::repo::*;
