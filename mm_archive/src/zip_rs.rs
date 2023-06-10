use std::{
    fs::File,
    io::{self, Read, Seek},
    marker::PhantomData,
    ops::{ControlFlow, Index},
    path::Path, rc::Rc,
};

use crate::traits::{self, EntryMetadata, EntryMetadataData};
use time::error::ComponentRange;
use zip::{
    read::ZipFile,
    result::{ZipError, ZipResult},
    ZipArchive,
};

pub struct Archive<R: Read + Seek>(zip::read::ZipArchive<R>);

pub struct Entry<'a, R: Read + Seek> {
    archive: &'a mut Archive<R>,
    metadata: EntryMetadataData<ComponentRange>,
    idx: usize
}

impl<'ar, R: Read + Seek> traits::Entry for Entry<'ar, R> {
    type Error = ZipError;

    type Metadata = EntryMetadataData<ComponentRange>;

    type UncompressedRead<'a> = ZipFile<'a> where Self: 'a;

    type CompressedRead<'a> = ZipFile<'a> where Self: 'a;

    fn metadata(&self) -> Self::Metadata {
        self.metadata
    }

    fn uncompressed_data<'a>(&'a mut self) -> Result<Self::UncompressedRead<'a>, Self::Error> {
        self.archive.0.by_index_raw(self.idx)
    }

    fn compressed_data<'a>(&'a mut self) -> Result<Self::UncompressedRead<'a>, Self::Error> {
        self.archive.0.by_index(self.idx)
    }
}

impl Archive<File> {
    fn _from_path(path: &Path) -> ZipResult<Self> { ZipArchive::new(File::open(path)?).map(Self) }
    pub fn from_path(path: impl AsRef<Path>) -> ZipResult<Self> { Self::_from_path(path.as_ref()) }
}

impl<R: Read + Seek> Archive<R> {
    pub fn for_each_entry<B>(
        &mut self,
        mut f: impl FnMut(Result<Entry<R>, ZipError>) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for i in 0..self.0.len() {
            let entry = self.0.by_index_raw(i).map(|f| Entry {
                metadata: EntryMetadataData::new(f),
                archive: self,
                idx: i
            });
            f(entry)?;
        }
        ControlFlow::Continue(())
    }
}

struct Iter<'a, R: Read + Seek> {
    archive: &'a mut ZipArchive<R>,
    idx: usize,
}

impl<'a> EntryMetadata for ZipFile<'a> {
    type Error = ComponentRange;

    fn is_dir(&self) -> bool { self.is_dir() }

    fn is_file(&self) -> bool { self.is_file() }

    fn is_symlink(&self) -> bool { false }

    fn len(&self) -> u64 { self.size() }

    fn modified(&self) -> Result<std::time::SystemTime, Self::Error> {
        Ok(self.last_modified().to_time()?.into())
    }

    fn compression_method(&self) -> traits::CompressionMethod {
        match self.compression() {
            zip::CompressionMethod::Stored => traits::CompressionMethod::Store,
            zip::CompressionMethod::Deflated => traits::CompressionMethod::Deflate,
            _ => traits::CompressionMethod::Unknown,
        }
    }

    fn compression_level(&self) -> Option<u8> { None }
}
