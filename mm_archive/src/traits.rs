use std::{
    fs::{FileType, Permissions},
    io::Read,
    ops::{ControlFlow, Range},
    time::SystemTime,
};

#[derive(Clone, Copy)]
pub enum CompressionMethod {
    Store,
    Deflate,
    Deflate64,
    Unknown,
}

pub trait EntryMetadata {
    type Error;
    fn is_dir(&self) -> bool;
    fn is_file(&self) -> bool;
    fn is_symlink(&self) -> bool;
    fn len(&self) -> u64;
    fn modified(&self) -> Result<SystemTime, Self::Error>;
    fn compression_method(&self) -> CompressionMethod;
    fn compression_level(&self) -> Option<u8>;
}

#[derive(Clone)]
pub struct EntryMetadataData<E> {
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub len: u64,
    pub modified: Result<SystemTime, E>,
    pub compression_method: CompressionMethod,
    pub compression_level: Option<u8>,
}

impl<E: Copy> EntryMetadata for EntryMetadataData<E> {
    type Error = E;

    fn is_dir(&self) -> bool { self.is_dir }
    fn is_file(&self) -> bool { self.is_file }
    fn is_symlink(&self) -> bool { self.is_symlink }
    fn len(&self) -> u64 { self.len }
    fn modified(&self) -> Result<SystemTime, Self::Error> { self.modified }
    fn compression_method(&self) -> CompressionMethod { self.compression_method }
    fn compression_level(&self) -> Option<u8> { self.compression_level }
}

impl<E> EntryMetadataData<E> {
    pub fn new(value: impl EntryMetadata<Error = E>) -> Self {
        Self {
            is_file: value.is_file(),
            is_dir: value.is_dir(),
            is_symlink: value.is_symlink(),
            len: value.len(),
            modified: value.modified(),
            compression_method: value.compression_method(),
            compression_level: value.compression_level()
        }
    }
}

pub trait Entry {
    type Error;
    type Metadata: EntryMetadata;
    type UncompressedRead<'a>: Read where Self: 'a;
    fn metadata(&self) -> Result<Self::Metadata, Self::Error>;
    fn uncompressed_data<'a>(&'a mut self) -> Result<Self::UncompressedRead<'a>, Self::Error>;
}

impl EntryMetadata for std::fs::Metadata {
    type Error = std::io::Error;
    fn is_dir(&self) -> bool { self.is_dir() }
    fn is_file(&self) -> bool { self.is_file() }
    fn is_symlink(&self) -> bool { self.is_symlink() }
    fn len(&self) -> u64 { self.len() }
    fn modified(&self) -> Result<SystemTime, Self::Error> { self.modified() }
    fn compression_method(&self) -> CompressionMethod {
        // Even with FS compression this is Store, since we never see the compressed bytes
        CompressionMethod::Store
    }
    fn compression_level(&self) -> Option<u8> { None }
}
