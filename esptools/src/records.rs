// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use std::mem::{size_of_val, size_of, transmute};


use crate::common::ConstantSizedRecord;



impl ConstantSizedRecord for RecordHeader {
    const SIZE: usize = size_of::<RawRecordHeader>();
}

pub struct RecordHeader {
    pub typ: [u8; 4],
    pub data_size: u32,
    pub flags: u32,
    pub form_id: u32,
    pub timestamp: u16,
    pub vcs_info: u16,
    pub internal_version: u16,
}

#[repr(C)]
pub struct RawRecordHeader {
    pub typ: [u8; 4],
    pub data_size: u32,
    pub flags: u32,
    pub form_id: u32,
    pub timestamp: u16,
    pub vcs_info: u16,
    pub internal_version: u16,
    pub unknown: u16
}

pub struct GroupHeader {
    pub typ: [u8; 4],
    pub group_size: u32,
    pub label: [u8; 4],
    pub group_type: u32,
    pub timestamp: u16,
    pub vcs_info: u16,
}

impl ConstantSizedRecord for GroupHeader {
    const SIZE: usize = size_of::<RawGroupHeader>();
}

#[repr(C)]
pub struct RawGroupHeader {
    pub typ: [u8; 4],
    pub group_size: u32,
    pub label: [u8; 4],
    pub group_type: u32,
    pub timestamp: u16,
    pub vcs_info: u16,
    pub unknown: u32
}

#[repr(C)]
pub struct RawFieldHeader {
    pub typ: [u8; 4],
    // sometimes field_size is a lie
    pub field_size: u16,
}
#[repr(C)]
pub struct Record {
    pub header: RawRecordHeader,
    pub data: [u8]
}

#[repr(C)]
pub struct Group {
    pub header: RawGroupHeader,
    pub data: [u8]
}

#[repr(C)]
pub struct Field {
    pub header: RawFieldHeader,
    pub data: [u8]
}

impl Record {
    pub fn first_field(&self) -> Option<&Field> {
        fn first_field_header(rec: &Record) -> Option<&RawFieldHeader> {
            if size_of_val(&rec.data) < size_of::<RawFieldHeader>() {
                None
            } else {
                unsafe {
                    Some(transmute::<&u8, &RawFieldHeader>(&rec.data[0]))
                }
            }
        }
        let header = first_field_header(self)?;
        if size_of_val(&self.data) < size_of::<RawFieldHeader>() + header.field_size as usize {
            None
        } else {
            unsafe {
                Some(transmute::<(&RawFieldHeader, usize), &Field>((header, header.field_size as usize)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::fs::File;
    use std::env;
    use std::mem::{size_of};
    use std::ptr::addr_of;
    use super::*;
    use memmap2::Mmap;
    #[test]
    fn first_field() -> Result<(), std::io::Error>  {
        
        let empty = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/empty.esm");
        let file = File::open(empty)?;
        let mmap = unsafe { Mmap::map(&file)? };
        unsafe {
            let recHdr = &*(mmap.as_ptr() as *const RawRecordHeader);
            assert_eq!(recHdr.typ, [b'T', b'E', b'S', b'4']);
            assert_eq!(addr_of!(recHdr.data_size).read_unaligned(), 52);
            let record = transmute::<(&RawRecordHeader, usize), &Record>((recHdr, recHdr.data_size as usize + size_of::<RawRecordHeader>()));
            assert_eq!(record.header.typ, [b'T', b'E', b'S', b'4']);
            assert_eq!(addr_of!(record.header.data_size).read_unaligned(), 52);
        }
        Ok(())
    }
}
