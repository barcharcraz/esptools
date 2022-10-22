use std::{error, result};
use std::borrow::{Borrow, BorrowMut};
use std::cell::BorrowError;

use std::cmp::min;
use std::ffi::{CStr, CString, FromBytesWithNulError};
use std::fmt::Debug;
use std::io::{self, BorrowedBuf, BufReader, Read, Seek, SeekFrom};
use std::mem::{MaybeUninit, size_of, transmute};

use bitflags::bitflags;
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use bytes::Buf;
use thiserror::Error;

use crate::common::*;



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




pub struct ParseStream<R: Read>(R);

impl<R: Read> Read for ParseStream<R> {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		self.0.read(buf)
	}
}

pub trait ParseBsa: Sized {
	fn parse(input: &mut impl Read) -> Result<Self>;
}




trait BsaReadExt: Read + Sized {
	fn parse<T: ParseBsa>(&mut self) -> Result<T> {
		<T as ParseBsa>::parse(self)
	}
}


impl<R: Read> BsaReadExt for R{}

impl SeekPredicate for std::fs::File {}

impl SeekPredicate for std::io::BufReader<std::fs::File> {}



impl ParseBsa for ArchiveHeader {
	fn parse(input: &mut impl Read) -> Result<Self> {
		use Error::*;
		let mut buf: [MaybeUninit<u8>; Self::SIZE as usize]
			= MaybeUninit::uninit_array();
		let mut buf = BorrowedBuf::from(&mut buf[..]);
		input.read_buf_exact(buf.unfilled())?;
		let mut buf = buf.filled();
		Ok(Self {
			tag: buf.eat_tag(b"BSA\0")?,
			version: ArchiveVersion::from_primitive(buf.get_u32_le()).ok_or(ParseError)?,
			offset: buf.get_u32_le(),
			flags: ArchiveFlags::from_bits(buf.get_u32_le()).ok_or(ParseError)?,
			folder_count: buf.get_u32_le(),
			file_count: buf.get_u32_le(),
			total_folder_name_length: buf.get_u32_le(),
			total_file_name_length: buf.get_u32_le(),
			file_flags: FileFlags::from_bits(buf.get_u16_le()).ok_or(ParseError)?,
		})
	}
}

#[test]
fn test_archive_reader() {
	use std::{fs::File, path::Path};
	let test1_p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/test1.bsa");
	let mut test1_f = File::open(test1_p).unwrap();
	let mut reader = ArchiveHeader::parse(&mut test1_f).unwrap();

	println!("{:#x?}", reader);
	let folder = FolderRecord::parse_given(&mut test1_f, &reader).unwrap();
	println!("{:#x?}", folder);
	let folder = FolderRecord::parse_given(&mut test1_f, &reader).unwrap();
	println!("{:#x?}", folder);

	test1_f.seek(SeekFrom::Start(folder.offset as u64 - reader.total_file_name_length as u64)).unwrap();

	println!("{:?}", test1_f.parse_bzstring().unwrap());

	let file = FileRecord::parse_given(&mut test1_f, &reader).unwrap();
	println!("{:x?}", file);
	test1_f.seek(SeekFrom::Start(0));
	test1_f.skip(file.offset as u64).unwrap();

	let file_data = (&test1_f).parse_bytes(file.size as usize);
	println!("{:?}", file_data);
}


pub trait SizedRecord {
	fn size(ver: ArchiveVersion) -> usize;
}

impl<T: ConstantSizedRecord> SizedRecord for T {
	fn size(ver: ArchiveVersion) -> usize {
		Self::SIZE
	}
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum NameError {
	FromBytesUntilNulError(#[from] FromBytesWithNulError),
	Io(#[from] io::Error),
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

impl ConstantSizedRecord for ArchiveHeader { const SIZE: usize = 36; }

#[repr(C)]
pub struct RawFolderRecord105 {
	pub hash: u64,
	pub count: u32,
	pad1: u32,
	offset: u32,
	pad2: u32
}

#[repr(C)]
pub struct RawFolderRecord104 {
	pub hash: u64,
	pub count: u32,
	pub offset: u32
}

#[derive(Debug, Copy, Clone)]
pub struct FolderRecord {
	pub hash: u64,
	pub count: u32,
	pub offset: u32,
}

#[derive(Debug, Copy, Clone)]
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

#[test]
fn test_archive_header() -> io::Result<()> {
	use std::{fs::File, path::Path};
	let test1_p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/test1.bsa");
	let mut test1_f = File::open(test1_p)?;
	let hdr = ArchiveHeader::parse(&mut test1_f).unwrap();
	println!("{:?}", hdr);
	Ok(())
}

impl FolderRecord {
	pub fn parse_given(input: &mut impl Read, header: &ArchiveHeader) -> Result<Self> {
		match header.flags {
			ArchiveFlags::XBOX360_ARCHIVE => Self::parse_endian::<BigEndian>(input, header.version),
			_ => Self::parse_endian::<LittleEndian>(input, header.version)
		}
	}
	pub fn parse_endian<E: ByteOrder>(value: &mut impl Read, ver: ArchiveVersion) -> Result<Self> {
		let result = Self {
			hash: value.read_u64::<E>()?,
			count: value.read_u32::<E>()?,
			offset: {
				if ver.is_64_bit() {
					value.read_u32::<E>()?;
				};
				value.read_u32::<E>()?
			},
		};
		if ver.is_64_bit() {
			value.read_u32::<E>()?;
		};
		Ok(result)
	}
}

impl SizedRecord for FolderRecord {
	fn size(ver: ArchiveVersion) -> usize {
		// there's padding from pointers inside this field,
		// so archives for 64-bit games are bigger
		if ver.is_64_bit() {
			24
		} else {
			16
		}
	}
}

impl FileRecord {
	fn parse_given(input: &mut impl Read, header: &ArchiveHeader) -> Result<Self> {
		match header.flags {
			ArchiveFlags::XBOX360_ARCHIVE => Self::parse_endian::<BigEndian>(input),
			_ => Self::parse_endian::<LittleEndian>(input)
		}
	}
	fn parse_endian<E: ByteOrder>(value: &mut impl Read) -> Result<Self> {
		Ok(Self {
			hash: value.read_u64::<E>()?,
			size: value.read_u32::<E>()?,
			offset: value.read_u32::<E>()?,
		})
	}

	fn is_compressed(&self, header: &ArchiveHeader) -> bool {
		header.flags.contains(ArchiveFlags::COMPRESSED_ARCHIVE)
			^ (self.size & 0x40000000 != 0)
	}
}
pub struct IndexedFolder {
	pub name: CString,
	pub hash: u64,
	pub first_file_idx: u32
}
pub struct IndexedFile {
	pub path: Option<CString>,
	pub data_size: u64,

}
pub struct IndexedArchive {
	header: ArchiveHeader,
	file_hashes: Vec<u64>,
	file_names: Vec<CString>,
	folders: Vec<IndexedFolder>,



}