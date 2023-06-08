use std::{
    fs::{FileType, Permissions},
    io::Read,
    ops::{ControlFlow, Range},
    time::SystemTime,
};

pub trait EntryMetadata {
    type Error;
    fn is_dir(&self) -> bool;
    fn is_file(&self) -> bool;
    fn is_symlink(&self) -> bool;
    fn len(&self) -> u64;
    fn modified(&self) -> Result<SystemTime, Self::Error>;
}

impl EntryMetadata for std::fs::Metadata {
    type Error = std::io::Error;
    fn is_dir(&self) -> bool { self.is_dir() }
    fn is_file(&self) -> bool { self.is_file() }
    fn is_symlink(&self) -> bool { self.is_symlink() }
    fn len(&self) -> u64 { self.len() }
    fn modified(&self) -> Result<SystemTime, Self::Error> { self.modified() }
}

pub struct ThinArchiveEntry<H> {
    pub offset: usize,
    pub file: H,
}

pub type ThinArchive<H> = Box<[ThinArchiveEntry<H>]>;

pub trait ArchiveConsumer<H> {
    type Error;
    fn consume(&mut self, entry_data: impl Read + EntryMetadata) -> Result<H, Self::Error>;
}
