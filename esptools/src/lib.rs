#![feature(read_buf)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(new_uninit)]
#![feature(min_specialization)]
#![feature(rustc_attrs)]
//#![feature(specialization)]
pub mod records;
pub mod fields;
pub mod bsa;
pub mod espparser;
mod common;

pub const GROUP_SIZE: usize = 24;