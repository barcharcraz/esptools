

use std::num::TryFromIntError;
use std::mem::transmute;
use bitflags::bitflags;


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

impl ArchiveVersion {
    pub fn is_64_bit(&self) -> bool {
        match self {
            ArchiveVersion::Oblivion => false,
            ArchiveVersion::Skyrim => false,
            ArchiveVersion::SkyrimSE => true,
        }
    }
}
pub struct EnumTryFromIntError(pub(crate) ());
impl TryFrom<u32> for ArchiveVersion {
    type Error = EnumTryFromIntError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            v @ 103..=105 => Ok(unsafe { transmute::<u32, ArchiveVersion>(v) }),
            _ => Err(EnumTryFromIntError(()))
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ArchiveHeader {
    pub tag: [u8; 4],
    pub version: ArchiveVersion,
    pub offset: u32,
    pub flags: ArchiveFlags,
    pub folder_count: u32,
    pub file_count: u32,
    pub total_folder_name_length: u32,
    pub total_file_name_length: u32,
    pub file_flags: FileFlags,
}

const _: () = assert!(std::mem::size_of::<ArchiveHeader>() == 36usize, "bad size");

#[repr(C)]
#[derive(Debug)]
pub struct DirectoryRecord {

    pub hash: u64,
    pub count: u32,
    pub offset: u32,
}

impl DirectoryRecord {
    pub const RECORD_SIZE: usize = 24;
}

#[repr(C)]
pub struct FileRecord {
    pub hash: u64,
    pub size: u32,
    pub offset: u32
}