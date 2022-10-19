#![feature(min_specialization)]
use std::{array, io};
use std::fmt::Debug;
use std::io::{Cursor, Read};
use std::num::TryFromIntError;
use std::mem::{MaybeUninit, size_of, transmute};
use bitflags::bitflags;
use thiserror::Error;
use byteorder::{ByteOrder, ReadBytesExt};
use std::num;
use bytes::Buf;
use crate::raw::TryFromArchiveError::{InvalidTag, ParseError};


bitflags! {

    pub struct ArchiveFlags: u32 {
        const INCLUDE_DIR_NAMES = 1 << 0;
        const INCLUDE_FILE_NAMES = 1 << 1;
        const COMPRESSED_ARCHIVE = 1 << 2;
        const RETAIN_DIRECTORY_NAMES = 1 << 3;
        const RETAIN_FILE_NAMES = 1 << 4;
        const RETAIN_FILE_NAME_OFFSETS = 1 << 5;
        const XBOX360_ARCHIVE = 1 << 6;
        const RETAIN_STRINGS_DURING_STARTUP = 1 << 7;
        const EMBED_FILE_NAMES = 1 << 8;
        const XMEM_CODEC = 1 << 9;
    }
    pub struct FileFlags: u16 {
        const MESHES = 1 << 0;
        const TEXTURES = 1 << 1;
        const MENUS = 1 << 2;
        const SOUNDS = 1 << 3;
        const VOICES = 1 << 4;
        const SHADERS = 1 << 5;
        const TREES = 1 << 6;
        const FONTS = 1 << 7;
        const MISC = 1 << 8;
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ArchiveVersion {
    Oblivion = 103,
    Skyrim = 104,
    SkyrimSE = 105
}

#[repr(C)]
#[derive(Debug)]
pub struct ArchiveHeader {
    pub tag: [u8; 4],
    pub version: u32,
    pub offset: u32,
    pub flags: u32,
    pub folder_count: u32,
    pub file_count: u32,
    pub total_folder_name_length: u32,
    pub total_file_name_length: u32,
    pub file_flags: u16,
    pad: MaybeUninit<u16>
}

const _: () = assert!(std::mem::size_of::<ArchiveHeader>() == 36usize);

#[repr(C)]
#[derive(Debug)]
pub struct DirectoryRecord {
    pub hash: u64,
    pub count: u32,
    pad1: MaybeUninit<u32>,
    pub offset: u32,
    pad2: MaybeUninit<u32>
}

const _: () = assert!(std::mem::size_of::<DirectoryRecord>() == 24usize);


#[repr(C)]
pub struct FileRecord {
    pub hash: u64,
    pub size: u32,
    pub offset: u32
}


impl ArchiveVersion {
    pub fn is_64_bit(&self) -> bool {
        match self {
            ArchiveVersion::Oblivion => false,
            ArchiveVersion::Skyrim => false,
            ArchiveVersion::SkyrimSE => true,
        }
    }
    fn from_primitive(value: u32) -> Option<Self> {
        match value {
            v @ 103..=105 => Some(unsafe { transmute::<u32, ArchiveVersion>(v) }),
            _ => None
        }
    }
}

trait FromEndian<T> {

    fn from_endian<E: ByteOrder>(value: T) -> Self;
}

#[derive(Debug)]
pub enum TryFromArchiveError {
    InvalidTag,
    ParseError
}

impl ArchiveHeader {

    fn try_from(value: &mut impl Buf) -> Result<Self, TryFromArchiveError> {
        assert!(value.remaining() >= size_of::<Self>());
        use TryFromArchiveError::*;
        let mut tag = [0u8; 4];
        value.copy_to_slice(&mut tag);
        if &tag != b"BSA\0" {
            return Err(InvalidTag)
        }
        // note: the header is always little endian,
        // even for x360
        Ok(Self {
                    tag,
                    version: value.get_u32_le(),
                    offset: value.get_u32_le(),
                    flags: value.get_u32_le(),
                    folder_count: value.get_u32_le(),
                    file_count: value.get_u32_le(),
                    total_folder_name_length: value.get_u32_le(),
                    total_file_name_length: value.get_u32_le(),
                    file_flags: value.get_u16_le(),
                    pad: MaybeUninit::uninit()
                })
    }
}



impl<T: Read> FromEndian<&mut T> for DirectoryRecord {

    fn from_endian<E: ByteOrder>(value: &mut T) -> Self {
        Self {
            hash: value.read_u64::<E>().unwrap(),
            count: value.read_u32::<E>().unwrap(),
            pad1: {value.read_u32::<E>(); MaybeUninit::uninit()},
            offset: value.read_u32::<E>().unwrap(),
            pad2: MaybeUninit::uninit()
        }
    }
}

impl<T: Read> FromEndian<&mut T> for FileRecord {
    fn from_endian<E: ByteOrder>(value: &mut T) -> Self {
        Self {
            hash: value.read_u64::<E>().unwrap(),
            size: value.read_u32::<E>().unwrap(),
            offset: value.read_u32::<E>().unwrap()
        }
    }
}