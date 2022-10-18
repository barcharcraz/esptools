use crate::raw_types::{
    ArchiveFlags, ArchiveHeader, ArchiveVersion, DirectoryRecord, FileFlags, FileRecord,
};
use crate::{common, raw_types};
use nom::bytes::complete::*;
use nom::error::{make_error, ErrorKind};
use nom::number::complete::*;
use nom::sequence::*;
use nom::IResult;
use nom::{combinator::*, Parser};
use nom::{multi::*, Finish};
use num_traits::FromPrimitive;
use std::mem::size_of;
use std::ops::Index;
use std::{ffi::OsString, os::raw};
pub struct FileRecordHeader<'a> {
    pub name: OsString,
    pub file_records: Vec<&'a FileRecord>,
}

pub struct DirectoryIterator<'a> {
    arch: &'a ArchiveHeader,
}

pub struct Archive {
    inner: Box<[u8]>,
    pub header: ArchiveHeader,
}

pub struct Directories<'a> {
    arch: &'a Archive
}

pub struct Directory<'a> {
    arch: &'a Archive,
    dir: DirectoryRecord
}

impl Directories<'_> {
    pub fn len(&self) -> u32 {
        self.arch.header.folder_count
    }
    pub fn get(&self, n: u32) -> Option<DirectoryRecord> {
        Some(DirectoryRecord::parse_from_bytes(
            self.arch.inner
                .get(self.arch.header.offset as usize + n as usize * DirectoryRecord::RECORD_SIZE..)
                .unwrap(),
            self.arch.header.version,
        )
        .finish().ok()?
        .1)
    }
}

impl Directory<'_> {
    pub fn len(&self) -> u32 {
        self.dir.count
    }
    pub fn get(&self, n: u32) -> Option<FileRecord> {
        
    }
}

impl TryFrom<Box<[u8]>> for Archive {
    type Error = nom::error::ErrorKind;

    fn try_from(value: Box<[u8]>) -> Result<Self, Self::Error> {
        let header = ArchiveHeader::parse_from_bytes(&value).finish();
        match header {
            Ok((_, header)) => Ok(Archive { inner: value, header}),
            Err(ref e) => Err(e.code)
        }
    }
}

impl Archive {
    pub fn directories(&self) -> Directories {
        Directories {
            arch: self
        }
    }
}

impl ArchiveHeader {
    pub fn parse_from_bytes(input: &[u8]) -> IResult<&[u8], Self> {
        let (
            i,
            (
                tag,
                version,
                offset,
                flags,
                folder_count,
                file_count,
                total_folder_name_length,
                total_file_name_length,
                file_flags,
            ),
        ) = tuple((
            map_res(tag(b"BSA\0"), TryInto::try_into),
            map_opt(le_u32, FromPrimitive::from_u32),
            le_u32,
            map_opt(le_u32, ArchiveFlags::from_bits),
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            map_opt(le_u16, FileFlags::from_bits),
        ))(input)?;
        Ok((
            i,
            ArchiveHeader {
                tag,
                version,
                offset,
                flags,
                folder_count,
                file_count,
                total_folder_name_length,
                total_file_name_length,
                file_flags,
            },
        ))
    }

    // pub fn nth_directory(n: u32) -> &DirectoryRecord {

    // }
}

impl DirectoryRecord {
    pub fn parse_from_bytes(i: &[u8], ver: ArchiveVersion) -> IResult<&[u8], Self> {
        let mut i = i;
        let (hash, count, offset);
        (i, hash) = le_u64(i)?;
        (i, count) = le_u32(i)?;
        if ver.is_64_bit() {
            (i, _) = take(4u32)(i)?;
        }
        (i, offset) = le_u32(i)?;
        Ok((
            i,
            DirectoryRecord {
                hash,
                count,
                offset,
            },
        ))
    }
}

impl FileRecord {
    pub fn parse_from_bytes(input: &[u8]) -> IResult<&[u8], Self> {
        let (i, (hash, size, offset)) = tuple((le_u64, le_u32, le_u32))(input)?;
        Ok((i, FileRecord { hash, size, offset }))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read, path::Path};

    use nom::Finish;

    use crate::{raw_types::ArchiveHeader, parser::Archive};

    #[test]
    fn test_archive_parse() {
        let pp = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/Particle Patch for ENB.bsa");
        let mut file = File::open(pp).unwrap();
        let mut input = Vec::new();
        file.read_to_end(&mut input).unwrap();
        let arch = Archive::try_from(input.into_boxed_slice()).unwrap();
        println!("{:?}", arch.header);
        let dir = arch.directories().get(0).unwrap();
        println!("{:?}, off: {:x}", dir, dir.offset);
    }
}
