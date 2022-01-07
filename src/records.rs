
use std::mem::{size_of_val, size_of, transmute};

#[repr(packed)]
pub struct RecordHeader {
    pub typ: [u8; 4],
    pub data_size: u32,
    pub flags: u32,
    pub form_id: u32,
    pub timestamp: u16,
    pub vcs_info: u16,
    pub internal_version: u16,
    pub unknown: u16
}

#[repr(packed)]
pub struct GroupHeader {
    pub typ: [u8; 4],
    pub group_size: u32,
    pub label: [u8; 4],
    pub group_type: i32,
    pub timestamp: u16,
    pub vcs_info: u16,
    pub unknown: u32
}

#[repr(packed)]
pub struct FieldHeader {
    pub typ: [u8; 4],
    // sometimes field_size is a lie
    pub field_size: u16,
}
#[repr(packed)]
pub struct Record {
    pub header: RecordHeader,
    pub data: [u8]
}

#[repr(packed)]
pub struct Group {
    pub header: GroupHeader,
    pub data: [u8]
}

#[repr(packed)]
pub struct Field {
    pub header: FieldHeader,
    pub data: [u8]
}

impl Record {
    pub fn first_field(&self) -> Option<&Field> {
        fn first_field_header(rec: &Record) -> Option<&FieldHeader> {
            if size_of_val(&rec.data) < size_of::<FieldHeader>() {
                None
            } else {
                unsafe {
                    Some(transmute::<&u8, &FieldHeader>(&rec.data[0]))
                }
            }
        }
        match first_field_header(self) {
            None => None,
            Some(header) => {
                if size_of_val(&self.data) < size_of::<FieldHeader>() + header.field_size as usize {
                    None
                } else {
                    unsafe {
                        Some(transmute::<(&FieldHeader, usize), &Field>((header, header.field_size as usize)))
                    }
                }
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
    use super::*;
    use memmap2::Mmap;
    #[test]
    fn first_field() -> Result<(), std::io::Error>  {
        
        let empty = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/empty.esm");
        let file = File::open(empty)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let recHdr = unsafe { &*(mmap.as_ptr() as *const RecordHeader) };
        assert_eq!(recHdr.typ, [b'T', b'E', b'S', b'4']);
        assert_eq!(recHdr.data_size, 52);
        let record = unsafe { transmute::<(&RecordHeader, usize), &Record>((recHdr, recHdr.data_size as usize + size_of::<RecordHeader>())) };
        assert_eq!(record.header.typ, [b'T', b'E', b'S', b'4']);
        assert_eq!(record.header.data_size, 52);
        
        Ok(())
    }
}