// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use crate::{mutable_tree::MutableTree, perms::PermissionsExtExt};
use byteorder::BE;
use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs::*, io_lifetimes::AsFilelike};
use cap_tempfile::TempFile;
use hex::FromHexError;
use io_tee::{ReadExt, WriteExt};
use serde::{de::{DeserializeOwned, IntoDeserializer}, Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use sha2::{Digest, Sha256};
use std::{
    backtrace::Backtrace,
    collections::BTreeMap,
    ffi::OsString,
    fmt::{self, Debug, Display},
    io::{self, copy, BorrowedBuf, Read, Seek, Write},
    mem::MaybeUninit,
    path::{Path, PathBuf},
    str::FromStr,
};
use strum_macros::{AsRefStr, Display, EnumString};
use thiserror::Error;
use zvariant::{gvariant, serialized::{Context, Data, Format}, to_bytes, Endian, OwnedValue, Type};

#[repr(transparent)]
#[derive(Serialize, Deserialize, Type, Default, Clone)]
pub struct Checksum(pub(self) Box<[u8]>);

impl<T> From<T> for Checksum
where
    Box<[u8]>: From<T>,
{
    fn from(value: T) -> Self { Self(Box::from(value)) }
}

impl AsRef<[u8]> for Checksum {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

impl Debug for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&hex::encode(&self.0)) }
}

impl Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { <Self as Debug>::fmt(self, f) }
}

impl FromStr for Checksum {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(Self(hex::decode(s)?.into_boxed_slice())) }
}

fn to_bytes_gv(value: &(impl Serialize + Type)) -> Vec<u8> {
    let ctx = Context::new(Format::GVariant, Endian::Big, 0);
    // any errors should be impossible, we use str to enforce utf-8, and it's a precondition violation
    // to get bogus types
    to_bytes(ctx, value).unwrap().to_vec()
}

fn from_slice_gv<'de, 'r: 'de, T: DeserializeOwned + Type>(slice: &'r [u8]) -> zvariant::Result<T> {
    let ctx = Context::new_gvariant(Endian::Big, 0);
    
    let data: Data<'de, 'static> = zvariant::serialized::Data::new(slice, ctx);
    //T::deserialize(data)
    data.deserialize().map(move |e|e.0)
}

fn gv_hash(value: &(impl Serialize + Type)) -> Checksum {
    Sha256::digest(to_bytes_gv(value))
        .to_vec()
        .into_boxed_slice()
        .into()
}
fn gv_hash_and_val(value: &(impl Serialize + Type)) -> (Checksum, Vec<u8>) {
    let data = to_bytes_gv(value);
    let chk = Sha256::digest(data.as_slice())
        .to_vec()
        .into_boxed_slice()
        .into();
    (chk, data)
}

pub trait Object {
    const OBJECT_TYPE: ObjectType;
}

macro_rules! stamp_out_object_enum {
    (
        $($name1:ident $(<$lt:lifetime>)? $(=$n:literal)?),*
        !,
        $($name2:ident $(=$n2:literal)?),*
    ) => {
        #[derive(Debug, Display, PartialEq, Clone, Copy, AsRefStr, EnumString)]
        #[strum(serialize_all = "lowercase")]
        pub enum ObjectType {
            $($name1 $(= $n)?,)*
            $($name2 $(= $n2)?,)*
        }
        $(impl$(<$lt>)? Object for $name1 $(<$lt>)? {
            const OBJECT_TYPE: ObjectType = ObjectType::$name1;
        })*
    };
}

stamp_out_object_enum! {
    DirTree = 2,
    DirMeta,
    Commit
    !, // this seperates enumerants we have a type for from others
    File = 1,
    TombstoneCommit = 5,
    Commitmeta,
    PayloadLink,
    FileXattrs,
    FileXattrsLink
}

impl ObjectType {
    pub fn is_meta(self) -> bool {
        use ObjectType::*;
        match self {
            DirMeta | Commit | TombstoneCommit | Commitmeta => true,
            _ => false,
        }
    }
}

#[derive(
    Debug, Display, SerializeDisplay, EnumString, DeserializeFromStr, Copy, Clone, PartialEq,
)]
#[repr(u32)]
#[strum(serialize_all = "kebab-case")]
pub enum RepoMode {
    Bare = 0,
    BareUser,
    BareUserOnly,
    ArchiveZ2,
    BareSplitXattrs,
}

#[test]
fn test_repo_mode() {
    assert_eq!(RepoMode::Bare.to_string(), "bare");
    assert_eq!(RepoMode::BareUser.to_string(), "bare-user");
    assert_eq!(RepoMode::ArchiveZ2.to_string(), "archive-z2");
    assert_eq!(RepoMode::BareSplitXattrs.to_string(), "bare-split-xattrs")
}
fn loose_path_extension(typ: ObjectType, mode: RepoMode) -> String {
    let mut result = typ.to_string();
    if mode == RepoMode::ArchiveZ2 && !typ.is_meta() {
        result.push('z');
    }
    result
}
pub fn loose_path(checksum: &Checksum, typ: ObjectType, mode: RepoMode) -> PathBuf {
    let checksum = checksum.to_string();
    // not exact but 15 should be enough for the extension and all seperators
    let mut result = PathBuf::with_capacity(checksum.len() + 15);
    #[cfg(debug_assertions)]
    let starting_cap = result.capacity();
    result.push(&checksum[0..2]);
    result.push(&checksum[2..]);
    result.set_extension(loose_path_extension(typ, mode));
    debug_assert_eq!(
        starting_cap,
        result.capacity(),
        "loose_path had to allocate, increase the capacity"
    );
    result
}
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct RelatedObject {
    pub name: String,
    pub checksum: Checksum,
}
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct Commit {
    pub metadata: BTreeMap<String, OwnedValue>,
    pub parent: Checksum,
    pub related_objects: Vec<RelatedObject>,
    pub subject: String,
    pub body: String,
    pub timestamp: u64,
    pub root_dirtree_checksum: Checksum,
    pub root_dirmeta_checksum: Checksum,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DirMeta {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub xattrs: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Default for DirMeta {
    fn default() -> Self {
        Self {
            uid: 0,
            gid: 0,
            // directory, rwxr-xr-x
            mode: 0o40755,
            xattrs: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Type, Clone)]
#[zvariant(signature = "ayay")]
pub struct DirTreeChecksums {
    pub checksum: Checksum,
    pub meta_checksum: Checksum,
}

#[derive(Serialize, Deserialize, Debug, Type, Default, Clone)]
#[zvariant(signature = "(a(say)a(sayay))")]
pub struct DirTree {
    pub files: BTreeMap<String, Checksum>,
    pub dirs: BTreeMap<String, DirTreeChecksums>,
}

/// This is the file header in archive-z2 mode, and also the "synthetic" file
/// header for other modes. in non-archive modes the gvariant serialization of
/// this is hashed but it's not actually written out because it's stored in the filesystem
#[derive(Serialize, Deserialize, Debug, Type)]
pub struct FileHeader {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    // must be zero
    pub rdev: u32,
    pub symlink_target: String,
    pub xattrs: Vec<(Vec<u8>, Vec<u8>)>,
}

fn canonical_mode(m: u32) -> u32 {
    m & (/* IFMT */0o170000 | 0o755)
}

impl Default for FileHeader {
    fn default() -> Self {
        Self {
            uid: 0,
            gid: 0,
            mode: 0o100644,
            rdev: 0,
            symlink_target: Default::default(),
            xattrs: Default::default(),
        }
    }
}

impl FileHeader {
    pub fn cannonical_from_file(file: impl AsFilelike) -> io::Result<Self> {
        let o = file.unixy_permissions()?;
        Ok(Self {
            uid: 0,
            gid: 0,
            mode: canonical_mode(o),
            rdev: 0,
            symlink_target: Default::default(),
            xattrs: Default::default(),
        })
    }
}

#[test]
fn test_sigs_match_upstream() {
    assert_eq!(DirMeta::signature(), "(uuua(ayay))");
    assert_eq!(FileHeader::signature(), "(uuuusa(ayay))");
    assert_eq!(DirTree::signature(), "(a(say)a(sayay))");
}

impl DirTreeChecksums {
    pub fn new(tree: &DirTree, meta: &DirMeta) -> Self {
        Self {
            checksum: gv_hash(tree),
            meta_checksum: gv_hash(meta),
        }
    }
}

#[derive(Debug)]
pub struct OsTreeRepo {
    repo_dir: Dir,
    objects_dir: Dir,
    tmp_dir_fd: Dir,
    config: RepoConfig,
}

#[derive(Error, Debug)]
pub enum RepoErrorKind {
    #[error("Repo already exists.")]
    AlreadyExists,
    #[error("Filename is invalid: {0:?}")]
    InvalidFilename(OsString),
    #[error("Invalid mutable tree: {0}")]
    InvalidMtree(String),
    #[error("Repo is malformed.")]
    MalformedRepo,
    #[error("variant error")]
    Variant(#[from] zvariant::Error),
    #[error("IO Error")]
    Io(#[from] io::Error),
    #[error("Format error")]
    Format(#[from] fmt::Error),
}
#[derive(Error, Debug)]
#[error("{source}")]
pub struct RepoError {
    source: RepoErrorKind,
    #[backtrace]
    backtrace: Backtrace,
}

impl<T> From<T> for RepoError
where
    RepoErrorKind: From<T>,
{
    fn from(value: T) -> Self {
        Self {
            source: RepoErrorKind::from(value),
            backtrace: Backtrace::capture(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct RepoCoreConfig {
    pub repo_version: u32,
    pub mode: RepoMode,
}

impl Default for RepoCoreConfig {
    fn default() -> Self {
        Self {
            repo_version: 1,
            mode: RepoMode::BareUserOnly,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RepoConfig {
    core: RepoCoreConfig,
}

pub mod traits {
    use std::io::{Read, Seek};

    use serde::de::DeserializeOwned;

    use crate::{Checksum, Object, ObjectType};

    use super::from_slice_gv;
    pub trait RepoRead {
        type Error;
        type ObjectHandle: Read;
        fn try_contains(&self, typ: ObjectType, chk: &Checksum) -> Result<bool, Self::Error>;
        // Note: Unlike git OsTree objects don't necessairly know their type,
        // additionally it's quite possible to have two objects with the same hash, but different object types
        // in the same repository (for instance you could store a file with the content of a dirmeta object,
        // as well as that dirmeta object itself).
        //
        // Git doesn't have this problem because _all_ objects are prefixed with their size and type on disk
        // so objects of different types by definition have different hashes, even if they have the same content
        // (not including this header).
        fn try_get(
            &self,
            typ: ObjectType,
            chk: &Checksum,
        ) -> Result<Option<Self::ObjectHandle>, Self::Error>;

        fn contains(&self, typ: ObjectType, chk: &Checksum) -> bool {
            self.try_contains(typ, chk).unwrap_or(false)
        }
        fn get(&self, typ: ObjectType, chk: &Checksum) -> Option<Self::ObjectHandle> {
            self.try_get(typ, chk).unwrap_or_default()
        }
    }

    pub trait RepoWriteObject<T> {
        type Error;
        fn write(&mut self, object: T) -> Result<Checksum, Self::Error>;
    }

    pub trait RepoWrite<T> {
        type Error;
        fn write_with_type(&mut self, object: T, typ: ObjectType) -> Result<Checksum, Self::Error>;
    }

    pub trait RepoReadExt<T>: RepoRead {
        fn try_load(&self, chk: &Checksum) -> Result<Option<T>, Self::Error>;
        fn load(&self, chk: &Checksum) -> Option<T> { self.try_load(chk).unwrap_or_default() }
    }
    impl<T, R> RepoReadExt<T> for R
    where
        T: zvariant::Type + DeserializeOwned + Object,
        R: RepoRead,
        R::Error: From<std::io::Error> + From<zvariant::Error>,
    {
        fn try_load(&self, chk: &Checksum) -> Result<Option<T>, Self::Error> {
            let mut bytes = Vec::new();
            let Some(mut handle) = self.try_get(T::OBJECT_TYPE, chk)? else {
                return Ok(None);
            };
            handle.read_to_end(&mut bytes)?;
            Ok(Some(from_slice_gv::<T>(&bytes)?))
        }
    }
}

fn write_header(mut w: impl Write, object: impl Read) -> io::Result<()> {
    let ctx = Context::new_gvariant(Endian::Big, 0);
    let header = FileHeader::default();
    let header_data = to_bytes(ctx, &header).unwrap();
    let header_data_size = header_data.len();
    assert!(header_data_size < u32::MAX as usize);
    let mut header_size_pfx = [0u8; 8];
    header_size_pfx[0..4].copy_from_slice(&(header_data_size as u32).to_be_bytes()[..]);
    copy(&mut &header_size_pfx[..], &mut w)?;
    copy(&mut &*header_data, &mut w)?;
    Ok(())
}

impl<R: Read> traits::RepoWrite<R> for OsTreeRepo {
    type Error = RepoError;

    fn write_with_type(&mut self, mut object: R, typ: ObjectType) -> Result<Checksum, Self::Error> {
        let mut temp_file = self.tmpfile_for_type(typ)?;
        let mut hasher: Sha256 = Sha256::new();
        if typ == ObjectType::File && self.config.core.mode != RepoMode::ArchiveZ2 {
            write_header(&mut hasher, &mut object)?;
        }
        let mut tee = (&mut hasher).tee(&mut temp_file);
        if typ == ObjectType::File && self.config.core.mode != RepoMode::ArchiveZ2 {
            write_header(&mut tee, &mut object)?;
        }
        // write to the hasher and the file
        copy(&mut object, &mut tee)?;
        let chk: Checksum = hasher.finalize().to_vec().into_boxed_slice().into();
        temp_file.commit(&chk)?;
        Ok(chk)
    }
}

impl traits::RepoWriteObject<&File> for OsTreeRepo {
    type Error = RepoError;

    fn write(&mut self, mut object: &File) -> Result<Checksum, Self::Error> {
        self.write_with_type(object, ObjectType::File)
        //self.write(object, ObjectType::File)
    }
}

impl<T: Object + Serialize + Type> traits::RepoWriteObject<&T> for OsTreeRepo {
    type Error = RepoError;

    fn write(&mut self, object: &T) -> Result<Checksum, Self::Error> {
        let mut hasher = Sha256::new();
        let ctx = Context::new_gvariant(Endian::Big, 0);
        let object_bytes = zvariant::to_bytes(ctx, &object).unwrap();
        hasher.update(&object_bytes);
        let chk: Checksum = hasher.finalize().to_vec().into_boxed_slice().into();
        let mut tmp = self.tmpfile_for_type(T::OBJECT_TYPE)?;
        tmp.write_all(&object_bytes)?;
        tmp.commit(&chk)?;
        Ok(chk)
    }
}
impl traits::RepoRead for OsTreeRepo {
    type Error = RepoError;

    type ObjectHandle = File;

    fn try_contains(&self, typ: ObjectType, chk: &Checksum) -> Result<bool, Self::Error> {
        Ok(self
            .objects_dir
            .try_exists(loose_path(chk, typ, self.config.core.mode))?)
    }

    fn try_get(
        &self,
        typ: ObjectType,
        chk: &Checksum,
    ) -> Result<Option<Self::ObjectHandle>, Self::Error> {
        if self.config.core.mode != RepoMode::BareUserOnly {
            unimplemented!()
        }
        let p = loose_path(chk, typ, self.config.core.mode);
        println!("{:?}", p);
        match self.objects_dir.open(p) {
            Ok(f) => Ok(Some(f)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

pub use traits::{RepoRead, RepoReadExt, RepoWrite, RepoWriteObject};

// Similar to cap-std::TempFile but just for ostree writing, knows the repo
// to write to and the type of the object being written so when the user is done
// writing and wants to finally add the object to the repo they can just call commit
#[derive(Debug)]
struct OsTreeTempFile<'repo> {
    file: TempFile<'repo>,
    repo: &'repo OsTreeRepo,
    typ: ObjectType,
}

impl<'repo> Write for OsTreeTempFile<'repo> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.file.write(buf) }

    fn flush(&mut self) -> io::Result<()> { self.file.flush() }
}

impl<'repo> OsTreeTempFile<'repo> {
    fn commit(self, chk: &Checksum) -> io::Result<()> {
        let mut temp_name = PathBuf::from(chk.to_string());
        temp_name.set_extension(loose_path_extension(self.typ, self.repo.config.core.mode));
        let final_name = loose_path(&chk, self.typ, self.repo.config.core.mode);
        // TODO: implement rename in cap-tempfile, and use that instaed of this two-stage deal
        self.file.replace(&temp_name)?;
        self.repo.objects_dir.create_dir_all(&final_name)?;
        self.repo
            .tmp_dir_fd
            .rename(temp_name, &self.repo.objects_dir, final_name)?;
        Ok(())
    }
}

impl OsTreeRepo {
    const STATE_DIRS: &[&'static str] = &[
        "tmp",
        "extensions",
        "state",
        "refs",
        "refs/heads",
        "refs/mirrors",
        "refs/remotes",
        "objects",
    ];

    fn _create(path: &Utf8Path) -> Result<OsTreeRepo, RepoError> {
        if let Err(e) = std::fs::create_dir(path) {
            if e.kind() == io::ErrorKind::AlreadyExists {
                return Err(RepoErrorKind::AlreadyExists.into());
            } else {
                return Err(e.into());
            }
        }
        let repo_dir = Dir::open_ambient_dir(path, ambient_authority())?;
        let config = RepoConfig::default();
        repo_dir
            .open_with("config", OpenOptions::new().write(true).create_new(true))?
            .write_all(serde_ini::to_string(&config).unwrap().as_ref())?;
        for dir_path in Self::STATE_DIRS {
            repo_dir.create_dir(dir_path)?;
        }
        let result = OsTreeRepo {
            objects_dir: repo_dir.open_dir("objects")?,
            tmp_dir_fd: repo_dir.open_dir("tmp")?,
            repo_dir,
            config,
        };
        Ok(result)
    }

    fn _open(path: &Utf8Path) -> Result<OsTreeRepo, RepoError> {
        let repo_dir: Dir = Dir::open_ambient_dir(path, ambient_authority())?;
        for dir in Self::STATE_DIRS {
            if !repo_dir.is_dir(dir) {
                return Err(RepoErrorKind::MalformedRepo.into());
            }
        }
        let objects_dir = repo_dir.open_dir("objects")?;
        let config: RepoConfig = serde_ini::from_str(&repo_dir.read_to_string("config")?)
            .or(Err(RepoError::from(RepoErrorKind::MalformedRepo)))?;
        Ok(OsTreeRepo {
            objects_dir: repo_dir.open_dir("objects")?,
            tmp_dir_fd: repo_dir.open_dir("tmp")?,
            repo_dir,
            config,
        })
    }

    pub fn create(path: &impl AsRef<Utf8Path>) -> Result<OsTreeRepo, RepoError> {
        Self::_create(path.as_ref())
    }

    pub fn open(path: &impl AsRef<Utf8Path>) -> Result<OsTreeRepo, RepoError> {
        Self::_open(path.as_ref())
    }

    pub fn object_fd(&self, typ: ObjectType, chk: &Checksum) -> io::Result<File> {
        if self.config.core.mode != RepoMode::BareUserOnly {
            unimplemented!()
        }
        let p = loose_path(chk, typ, self.config.core.mode);
        self.objects_dir.open(p)
    }
    fn tmpfile_for_type(&self, typ: ObjectType) -> io::Result<OsTreeTempFile<'_>> {
        Ok(OsTreeTempFile {
            file: TempFile::new(&self.tmp_dir_fd)?,
            repo: self,
            typ,
        })
    }
    /// get a fd for a new object, if the object already exists you get an error with ErrorKind::AlreadyExists
    pub fn new_object_fd_mut(&self, typ: ObjectType, chk: &Checksum) -> io::Result<File> {
        if self.config.core.mode != RepoMode::BareUserOnly {
            unimplemented!()
        }
        let p = loose_path(chk, typ, self.config.core.mode);
        self.objects_dir.create_dir(p.parent().unwrap())?;
        self.objects_dir
            .open_with(p, OpenOptions::new().create_new(true).write(true))
    }

    pub fn load_dirtree(&self, chk: &Checksum) -> Result<DirTree, RepoError> {
        if self.config.core.mode != RepoMode::BareUserOnly {
            unimplemented!()
        }

        let mut bytes = Vec::new();
        self.object_fd(ObjectType::DirTree, chk)?
            .read_to_end(&mut bytes)?;
        Ok(from_slice_gv::<DirTree>(&bytes)?)
    }

    pub fn write_dirmeta(&mut self, meta: &DirMeta) -> io::Result<Checksum> {
        let (chk, val) = gv_hash_and_val(meta);
        let mut fd = self.new_object_fd_mut(ObjectType::DirMeta, &chk)?;
        fd.write_all(&val)?;
        Ok(chk)
    }

    pub fn write_dfd_to_mtree(
        &mut self,
        dfd: Dir,
        mtree: &mut MutableTree,
    ) -> Result<(), RepoError> {
        use traits::RepoWriteObject;
        for item in dfd.entries()? {
            let item = item?;
            let file_type = item.file_type()?;
            let filename = item
                .file_name()
                .to_str()
                .ok_or(RepoErrorKind::InvalidFilename(item.file_name()))?
                .to_owned();
            if file_type.is_dir() {
                let mut child = mtree.ensure_dir(&filename)?;
                self.write_dfd_to_mtree(item.open_dir()?, &mut child)?;
            } else if file_type.is_file() {
                println!("{:?}", filename);
                let fd = item.open()?;
                let chk = self.write(&fd)?;
                mtree.replace_file(&filename, chk)?;
            }
        }

        Ok(())
    }
    pub fn write_dirpath_to_mtree(
        &mut self,
        dir: &impl AsRef<Path>,
        mtree: &mut MutableTree,
    ) -> Result<(), RepoError> {
        let dfd = Dir::open_ambient_dir(dir, ambient_authority())?;
        self.write_dfd_to_mtree(dfd, mtree)
    }
}
