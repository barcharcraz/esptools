
use std::mem::transmute;

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
pub struct GroupHeader {
    pub typ: [u8; 4],
    pub group_size: u32,
    pub label: [u8; 4],
    pub group_type: i32,
    pub timestamp: u16,
    pub vcs_info: u16,
    pub unknown: u32
}
pub struct FieldHeader {
    pub typ: [u8; 4],
    // sometimes field_size is a lie
    pub field_size: u16,
}
pub struct Record {
    pub header: RecordHeader,
    pub data: [u8]
}
pub struct Group {
    pub header: GroupHeader,
    pub data: [u8]
}

pub struct Field {
    pub header: FieldHeader,
    pub data: [u8]
}

impl Record {
    pub unsafe fn first_field(&self) -> Option<&Field> {
        if self.header.data_size == 0 {
            None
        } else {
            let fheader = &transmute::<&u8, &FieldHeader>(&self.data[0]);
            Some(transmute::<(&u8, usize), &Field>((&self.data[0], fheader.field_size as usize)))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::fs::File;
    use std::env;
    use memmap2::Mmap;
    #[test]
    fn first_field() -> Result<(), std::io::Error>  {
        let empty = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/empty.esm");
        let file = File::open(empty)?;
        unsafe {
            let mmap = Mmap
        }
        Ok(())
    }
}