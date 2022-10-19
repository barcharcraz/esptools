use std::error;
use std::ffi::{CStr, CString, FromBytesWithNulError};
use std::{fmt::Debug};
use std::io::{Read, self, Seek, SeekFrom, BorrowedBuf};

use bitflags::bitflags;
use std::mem::{size_of, transmute, MaybeUninit};
use byteorder::{ByteOrder, ReadBytesExt, LittleEndian, BigEndian};
use thiserror::Error;
use bytes::Buf;

#[derive(Debug, Error)]
pub enum TryFromArchiveError {
    #[error("Invalid Type Tag")]
    InvalidTag,
    #[error("Bogus size in archive data")]
    BogusSize,
    #[error("Parsing error")]
    ParseError,
    #[error("IO Error")]
    Io(#[from] io::Error)
}

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
    SkyrimSE = 105,
}

pub struct ArchiveReader<T: Read + Seek> {
    input: T,
    header: ArchiveHeader
}

impl<T: Read + Seek> ArchiveReader<T> {

    pub fn into_inner(self) -> T {
        self.input
    }
    pub fn try_new(mut input: T) -> Result<Self, TryFromArchiveError> {
        let header = ArchiveHeader::try_parse(&mut input)?;
        Ok(Self {
            input,
            header
        })
    }

    pub fn header(&self) -> &ArchiveHeader {
        &self.header
    }

    pub fn get_folder(&mut self, n: u32) -> io::Result<FolderRecord> {
        let offset = ArchiveHeader::SIZE + u32::checked_mul(FolderRecord::SIZE, n).unwrap();
        self.input.seek(SeekFrom::Start(offset as u64))?;
        Ok(match self.header.flags {
            ArchiveFlags::XBOX360_ARCHIVE => FolderRecord::from_endian::<BigEndian>(&mut self.input),
            _ => FolderRecord::from_endian::<LittleEndian>(&mut self.input)
        })
    }
}
#[test]
fn test_archive_reader() {
    use std::{fs::File, path::Path};
    let test1_p = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/test1.bsa");
    let test1_f = File::open(test1_p).unwrap();
    let mut reader = ArchiveReader::try_new(test1_f).unwrap();
    
    println!("{:#x?}", reader.header());

    let folder = reader.get_folder(1).unwrap();
    println!("{:#x?}", folder);


}

pub struct DirectoryReader<T: Read + Seek> {
    inner: ArchiveReader<T>,
    header: FolderRecord
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum NameError {
    FromBytesUntilNulError(#[from] FromBytesWithNulError),
    Io(#[from] io::Error)
}
impl<T: Read + Seek> DirectoryReader<T> {
    pub fn into_inner(self) -> ArchiveReader<T> {
        self.inner
    }
    pub fn real_offset(&self) -> u32 {
        u32::checked_sub(self.header.offset, self.inner.header.total_file_name_length).unwrap()
    }
    pub fn name(&mut self) -> Result<CString, NameError> {
        assert!(self.inner.header.flags.contains(ArchiveFlags::INCLUDE_DIR_NAMES));
        self.inner.input.seek(SeekFrom::Start(self.real_offset() as u64))?;
        let sz = self.inner.input.read_u8()?;
        let mut buf = [MaybeUninit::<u8>::uninit(); u8::MAX as usize];
        let mut buf = BorrowedBuf::from(&mut buf[..]);
        self.inner.input.read_buf_exact(buf.unfilled())?;
        Ok(CStr::from_bytes_with_nul(buf.filled())?.to_owned())
    }
}

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

#[derive(Debug)]
pub struct FolderRecord {
    pub hash: u64,
    pub count: u32,
    pub offset: u32
}

pub struct FileRecord {
    pub hash: u64,
    pub size: u32,
    pub offset: u32,
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
            _ => None,
        }
    }
}

impl ArchiveHeader {
    pub const SIZE: u32 = 36;
    fn try_parse_buf(value: & mut impl Buf) -> Result<Self, TryFromArchiveError> {
        assert!(value.remaining() >= Self::SIZE as usize);
        use TryFromArchiveError::*;
        let mut tag = [0u8; 4];
        value.copy_to_slice(&mut tag);
        if &tag != b"BSA\0" {
            return Err(InvalidTag);
        }
        // note: the header is always little endian,
        // even for x360
        let result = Self {
            tag,
            version: ArchiveVersion::from_primitive(value.get_u32_le()).ok_or(ParseError)?,
            offset: value.get_u32_le(),
            flags: ArchiveFlags::from_bits(value.get_u32_le()).ok_or(ParseError)?,
            folder_count: value.get_u32_le(),
            file_count: value.get_u32_le(),
            total_folder_name_length: value.get_u32_le(),
            total_file_name_length: value.get_u32_le(),
            file_flags: FileFlags::from_bits(value.get_u16_le()).ok_or(ParseError)?,
        };
        if result.offset != Self::SIZE {
            return Err(BogusSize)
        }
        value.advance(2); // padding
        Ok(result)
    }
    fn try_parse(input: &mut impl Read) -> Result<Self, TryFromArchiveError> {
        let mut buf = [0u8; Self::SIZE as usize];
        input.read_exact(&mut buf).map_err(|e|{TryFromArchiveError::Io(e)})?;
        Self::try_parse_buf(&mut buf.as_slice())
    }

}
#[test]
fn test_archive_header() -> io::Result<()> {
    use std::{fs::File, path::Path};
    let test1_p = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/test1.bsa");
    let mut test1_f = File::open(test1_p)?;
    let hdr = ArchiveHeader::try_parse(&mut test1_f).unwrap();
    println!("{:?}", hdr);
    Ok(())
}

impl FolderRecord {
    pub const SIZE: u32 = 24;
    pub fn from_endian<E: ByteOrder>(value: &mut impl Read) -> Self {
        let result = Self {
            hash: value.read_u64::<E>().unwrap(),
            count: value.read_u32::<E>().unwrap(),
            offset:  {
                value.read_u32::<E>().unwrap();
                value.read_u32::<E>().unwrap()
            }
        };
        value.read_u32::<E>().unwrap();
        result
    }
}

impl FileRecord {
    fn from_endian<E: ByteOrder>(value: &mut impl Read) -> Self {
        Self {
            hash: value.read_u64::<E>().unwrap(),
            size: value.read_u32::<E>().unwrap(),
            offset: value.read_u32::<E>().unwrap(),
        }
    }
}
