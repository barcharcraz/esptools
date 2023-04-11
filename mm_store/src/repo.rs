// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use crate::perms::PermissionsExtExt;
use byteorder::BE;
use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs::*, io_lifetimes::AsFilelike};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt::{self, Debug},
    io::{self, copy, Read, Write}, borrow::Borrow,
};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use zvariant::{from_slice, to_bytes, EncodingContext, Type, OwnedValue};

#[repr(transparent)]
#[derive(Serialize, Deserialize, Type, Default)]
pub struct Checksum(pub(self) Box<[u8]>);

impl<T> From<T> for Checksum
where
    Box<[u8]>: From<T>,
{
    fn from(value: T) -> Self {
        Self(Box::from(value))
    }
}

impl AsRef<[u8]> for Checksum {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Debug for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(&self.0))
    }
}

fn to_bytes_gv(value: &(impl Serialize + Type)) -> Vec<u8> {
    let ctx = EncodingContext::<BE>::new_gvariant(0);
    // any errors should be impossible, we use str to enforce utf-8, and it's a precondition violation
    // to get bogus types
    to_bytes(ctx, value).unwrap()
}

fn from_slice_gv<'de, 'r: 'de, T: Deserialize<'de> + Type>(slice: &'r [u8]) -> zvariant::Result<T> {
    let ctx = EncodingContext::<BE>::new_gvariant(0);
    from_slice(slice, ctx)
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
        $($name2:ident),*
    ) => {
        #[derive(Debug, Display, PartialEq, Clone, Copy)]
        #[strum(serialize_all = "kebab-case")]
        pub enum ObjectType {
            $($name1 $(= $n)?,)*
            $($name2),*
        }
        $(impl$(<$lt>)? Object for $name1 $(<$lt>)? {
            const OBJECT_TYPE: ObjectType = ObjectType::$name1;
        })*
    };
}

stamp_out_object_enum! {
    File = 1,
    DirTree,
    DirMeta,
    Commit
    !, // this seperates enumerants we have a type for from others
    TombstoneCommit,
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

pub fn loose_path(checksum: &Checksum, typ: ObjectType, mode: RepoMode) -> Utf8PathBuf {
    let checksum = hex::encode(checksum);
    [
        &checksum[0..2],
        &checksum[2..],
        typ.to_string().as_str(),
        match mode {
            RepoMode::ArchiveZ2 if typ.is_meta() => &"z",
            _ => &"",
        },
    ]
    .into_iter()
    .collect()
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

#[derive(Serialize, Deserialize, Debug, Type)]
#[zvariant(signature = "ayay")]
pub struct DirTreeChecksums {
    pub checksum: Checksum,
    pub meta_checksum: Checksum,
}

#[derive(Serialize, Deserialize, Debug, Type)]
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


pub fn hash_file(file: &mut File) -> io::Result<Checksum> {
    let mut hasher = Sha256::new();
    let ctx = EncodingContext::<BE>::new_gvariant(0);
    let header = FileHeader::default();
    let header_data = to_bytes(ctx, &header).unwrap();
    let header_data_size = header_data.len();
    assert!(header_data_size < u32::MAX as usize);
    let header_size_pfx: [u8; 4] = (header_data_size as u32).to_be_bytes();
    hasher.update(header_size_pfx);
    // alignment
    hasher.update([0u8; 4]);
    hasher.update(header_data);
    copy(file, &mut hasher)?;
    Ok(hasher.finalize().to_vec().into())
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
    mode: RepoMode,
}

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("Repo already exists.")]
    AlreadyExists,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct RepoConfig {
    pub repo_version: u32,
    pub mode: RepoMode,
}
impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            repo_version: 1,
            mode: RepoMode::BareUserOnly,
        }
    }
}

pub mod traits {
    use std::{io::Read};

    use serde::{de::DeserializeOwned};

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

    pub trait RepoWrite {
        type Error;
        fn write(&mut self, typ: ObjectType, object: impl Read) -> Result<Checksum, Self::Error>;
    }

    pub trait RepoReadExt<T>: RepoRead {
        fn try_load(&self, chk: &Checksum) -> Result<Option<T>, Self::Error>;
        fn load(&self, chk: &Checksum) -> Option<T> {
            self.try_load(chk).unwrap_or_default()
        }
    }
    impl<T, R> RepoReadExt<T> for R
    where
        T: zvariant::Type + DeserializeOwned + Object,
        R: RepoRead,
        R::Error: From<std::io::Error> + From<zvariant::Error>
    {
        fn try_load(&self, chk: &Checksum) -> Result<Option<T>, Self::Error> {
            let mut bytes = Vec::new();
            let Some(mut handle) = 
                self.try_get(T::OBJECT_TYPE, chk)?
            else {
                return Ok(None);
            };
            handle.read_to_end(&mut bytes)?;
            Ok(Some(from_slice_gv::<T>(&bytes)?))
        }
    }
}
pub use traits::RepoReadExt;

impl traits::RepoWrite for OsTreeRepo {
    type Error = RepoError;

    fn write(&mut self, typ: ObjectType, mut object: impl Read) -> Result<Checksum, Self::Error> {
        let mut hasher = Sha256::new();
        copy(&mut object, &mut hasher)?;
        let chk = hasher.finalize().to_vec().into_boxed_slice().into();
        let mut fd = self.new_object_fd_mut(typ, &chk)?;
        copy(&mut object, &mut fd)?;
        Ok(chk)
    }
}

impl traits::RepoRead for OsTreeRepo {
    type Error = RepoError;

    type ObjectHandle = File;

    fn try_contains(&self, typ: ObjectType, chk: &Checksum) -> Result<bool, Self::Error> {
        Ok(self.objects_dir.try_exists(loose_path(chk, typ, self.mode))?)
    }

    fn try_get(
        &self,
        typ: ObjectType,
        chk: &Checksum,
    ) -> Result<Option<Self::ObjectHandle>, Self::Error> {
        if self.mode != RepoMode::BareUserOnly {
            unimplemented!()
        }
        let p = loose_path(chk, typ, self.mode);
        match self.objects_dir.open(p) {
            Ok(f) => Ok(Some(f)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
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
        std::fs::create_dir(path)?;
        let repo_dir = Dir::open_ambient_dir(path, ambient_authority())?;
        if repo_dir.is_dir("objects") {
            return Err(RepoError::AlreadyExists);
        }

        let config_data = toml::to_string(&RepoConfig::default()).unwrap();

        repo_dir
            .open_with("config", OpenOptions::new().write(true).create_new(true))?
            .write_all(config_data.as_ref())?;
        for dir_path in Self::STATE_DIRS {
            repo_dir.create_dir(dir_path)?;
        }
        let result = OsTreeRepo {
            objects_dir: repo_dir.open_dir("objects")?,
            repo_dir,
            mode: RepoMode::BareUserOnly,
        };
        Ok(result)
    }

    pub fn create(path: impl AsRef<Utf8Path>) -> Result<OsTreeRepo, RepoError> {
        Self::_create(path.as_ref())
    }

    pub fn object_fd(&self, typ: ObjectType, chk: &Checksum) -> io::Result<File> {
        if self.mode != RepoMode::BareUserOnly {
            unimplemented!()
        }
        let p = loose_path(chk, typ, self.mode);
        self.objects_dir.open(p)
    }

    /// get a fd for a new object, if the object already exists you get an error with ErrorKind::AlreadyExists
    pub fn new_object_fd_mut(&mut self, typ: ObjectType, chk: &Checksum) -> io::Result<File> {
        if self.mode != RepoMode::BareUserOnly {
            unimplemented!()
        }
        let p = loose_path(chk, typ, self.mode);
        self.objects_dir
            .open_with(p, OpenOptions::new().create_new(true).write(true))
    }

    pub fn load_dirtree(&self, chk: &Checksum) -> Result<DirTree, RepoError> {
        if self.mode != RepoMode::BareUserOnly {
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

    pub fn write_dfd_to_mtree(&mut self, _dfd: Dir, _mtree: &mut MutableTree) {}
}
