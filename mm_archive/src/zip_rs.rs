use std::{
    fs::File,
    io::{self, Read, Seek},
    marker::PhantomData,
    ops::{ControlFlow, Index},
    path::Path,
};

use crate::traits::{self, ArchiveConsumer, EntryMetadata, ThinArchive, ThinArchiveEntry};
use zip::{
    read::ZipFile,
    result::{ZipError, ZipResult},
    ZipArchive,
};

pub struct Archive<R: Read + Seek>(pub(self) zip::read::ZipArchive<R>);
pub struct Entry<'a>(pub zip::read::ZipFile<'a>);

impl Archive<File> {
    fn _from_path(path: &Path) -> ZipResult<Self> { ZipArchive::new(File::open(path)?).map(Self) }
    pub fn from_path(path: impl AsRef<Path>) -> ZipResult<Self> { Self::_from_path(path.as_ref()) }
}

impl<R: Read + Seek> Archive<R> {
    pub fn for_each_entry<B>(
        &mut self,
        mut f: impl FnMut(ZipResult<Entry>) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for i in 0..self.0.len() {
            let entry = self.0.by_index(i).map(Entry);
            f(entry)?;
        }
        ControlFlow::Continue(())
    }
}

impl<'a> EntryMetadata for ZipFile<'a> {
    type Error = time::error::ComponentRange;

    fn is_dir(&self) -> bool { self.is_dir() }

    fn is_file(&self) -> bool { self.is_file() }

    fn is_symlink(&self) -> bool { false }

    fn len(&self) -> u64 { self.size() }

    fn modified(&self) -> Result<std::time::SystemTime, Self::Error> {
        Ok(self.last_modified().to_time()?.into())
    }
}

pub fn generate_thin_archive<H, C>(r: impl Read + Seek + Clone, mut consume: C) -> Result<ThinArchive<H>, ZipError>
    where
        C: ArchiveConsumer<H>,
        ZipError: From<<C as ArchiveConsumer<H>>::Error>
{
    let mut zip = ZipArchive::new(r.clone())?;
    let mut result = Vec::<ThinArchiveEntry<H>>::with_capacity(zip.len());
    let mut cumulative_metadata_read = 0;
    let mut last_data_end = 0;
    
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        cumulative_metadata_read += entry.data_start() - last_data_end;
        last_data_end = entry.data_start() + entry.compressed_size();
        result.push(ThinArchiveEntry {
            offset: cumulative_metadata_read as usize,
            file: consume.consume(entry)?,
        });
    }
    Ok(result.into_boxed_slice())
    
}
