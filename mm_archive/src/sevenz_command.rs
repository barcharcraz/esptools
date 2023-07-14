use num_enum::{FromPrimitive, TryFromPrimitive, TryFromPrimitiveError};
use std::{ffi::OsString, fmt::Display, io, path::Path, process::Command, sync::Arc, fs::{self, File, ReadDir}};
use strum_macros::Display;
use tempfile::{tempdir, tempdir_in, TempDir};
use thiserror::Error;

use crate::traits::{EntryMetadata, self};
pub struct Archive {
    dir: TempDir,
}
pub struct ArchiveOptions<'a, 'b> {
    dir: Option<&'a Path>,
    sevenz_path: &'b Path,
}

impl Default for ArchiveOptions<'static, 'static> {
    fn default() -> Self {
        Self {
            dir: None,
            sevenz_path: &Path::new("7z"),
        }
    }
}

#[derive(TryFromPrimitive, Debug, Display)]
#[repr(i32)]
pub enum SevenZExitCode {
    Success = 0,
    Warning = 1,
    Fatal = 2,
    CommandLine = 7,
    OOM = 8,
    UserStop = 255,
}

#[derive(Error, Debug, Display)]
pub enum Error {
    Io(#[from] io::Error),
    ExtractionFailed(SevenZExitCode),
    #[error(transparent)]
    UnknownExitCode(#[from] TryFromPrimitiveError<SevenZExitCode>),
    Terminated,
}

impl Archive {
    fn _from_path(path: &Path, opts: &ArchiveOptions) -> Result<Self, Error> {
        let tmpdir = match opts.dir {
            Some(d) => tempdir_in(d),
            None => tempdir(),
        }?;
        let res = Command::new(opts.sevenz_path)
            .arg("x").arg(path)
            .current_dir(tmpdir.path())
            .status()?;
        let sres = SevenZExitCode::try_from(res.code().ok_or(Error::Terminated)?)?;
        match sres {
            SevenZExitCode::Success => Ok(Self { dir: tmpdir }),
            _ => Err(Error::ExtractionFailed(sres))
        }
    }
    pub fn from_path(path: &impl AsRef<Path>, opts: &ArchiveOptions) -> Result<Self, Error> {
        Self::_from_path(path.as_ref(), opts)
    }
}

pub struct Entry(pub(self) std::fs::File);

impl traits::Entry for Entry {
    type Error = io::Error;

    type Metadata = fs::Metadata;

    type UncompressedRead<'a> = File;

    fn metadata(&self) -> Result<Self::Metadata, Self::Error> {
        self.0.metadata()
    }

    fn uncompressed_data(&mut self) -> Result<File, Self::Error> {
        Ok(self.0.try_clone()?)
    }
}


pub struct Iter {
    inner: ReadDir
}
