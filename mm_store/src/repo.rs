// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use crate::perms::PermissionsExtExt;
use byteorder::BE;
use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs::*, io_lifetimes::AsFilelike};
use serde::{de, de::Visitor, Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt::{self, Debug},
    io::{self, copy, ErrorKind, Read, Write},
    ptr::NonNull,
};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use zvariant::{from_slice, gvariant, to_bytes, DynamicType, EncodingContext, Type, Value};

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

#[derive(Debug, Display, PartialEq, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum ObjectType {
    File = 1,
    DirTree,
    DirMeta,
    Commit,
    TombstoneCommit,
    Commitmeta,
    PayloadLink,
    FileXattrs,
    FileXattrsLink,
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
pub struct Commit<'a> {
    #[serde(borrow)]
    pub metadata: BTreeMap<String, Value<'a>>,
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

#[derive(Debug)]
enum MutableTree<'repo> {
    Lazy {
        checksums: DirTreeChecksums,
        repo: &'repo Repo,
    },
    Whole {
        files: BTreeMap<String, Checksum>,
        subdirs: BTreeMap<String, MutableTree<'repo>>,
    },
}

impl<'repo> MutableTree<'repo> {
    pub fn new() -> Self {
        Self::Whole {
            files: BTreeMap::new(),
            subdirs: BTreeMap::new(),
        }
    }

    pub fn from_dirtree_chk(repo: &'repo Repo, dtree: DirTree) -> Self {
        Self::Whole {
            files: dtree.files,
            subdirs: dtree
                .dirs
                .into_iter()
                .map(|(k, v)| (k, Self::new_lazy_from_repo(repo, v)))
                .collect(),
        }
    }

    pub fn new_lazy_from_repo(repo: &'repo Repo, chk: DirTreeChecksums) -> Self {
        Self::Lazy {
            checksums: chk,
            repo: repo,
        }
    }

    pub fn make_whole(&mut self) -> Result<(), RepoError> {
        use MutableTree::*;
        match self {
            Whole => Ok(()),
            Lazy {checksums, repo } => {
                let dirtree = repo.load_dirtree(&checksums.checksum)?;
                *self = Whole {
                    files: dirtree.files,
                    subdirs: dirtree
                        .dirs
                        .into_iter()
                        .map(|(k, v)| (k, Self::new_lazy_from_repo(repo, v)))
                        .collect(),
                };
                Ok(())
            }
        }
    }
    pub fn ensure_dir(&mut self, dir_name: &str) -> Result<&mut MutableTree, RepoError> {
        self.make_whole()?;

    }
}

impl Default for MutableTree<'_> {
    fn default() -> Self {
        Self::new()
    }
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

//     pub fn new_recursive_blank(dir: Dir) -> io::Result<MutableTree> {
//         let mut result = Self::new();
//         fn handle_inval(o: OsString) -> io::Error {
//             io::Error::new(ErrorKind::InvalidData, o.to_string_lossy())
//         }
//         for ent in dir.entries()?.flatten() {
//             if ent.file_type()?.is_dir() {
//                 // TODO: NFC
//                 result.insert_child(
//                     ent.file_name().into_string().map_err(handle_inval)?,
//                     Self::new_recursive_blank(ent.open_dir()?)?,
//                 );
//             } else {
//                 result.files.insert(
//                     ent.file_name().into_string().map_err(handle_inval)?,
//                     hash_file(&mut ent.open()?)?,
//                 );
//             }
//         }
//         let ctx = EncodingContext::<BE>::new_gvariant(0);
//         let mut hasher = Sha256::new();
//         let dt = result.as_dirtree();
//         hasher.update(to_bytes(ctx, &dt).unwrap());
//         result.contents_checksum = hasher.finalize().to_vec().into();
//         Ok(result)
//     }
// }

#[derive(Debug)]
pub struct Repo {
    repo_dir: Dir,
    objects_dir: Dir,
    mode: RepoMode,
}

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("Repo already exists.")]
    AlreadyExists,
    #[error("Invalid mutable tree")]
    InvalidMtree,
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

impl Repo {
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

    pub fn create(path: &Utf8Path) -> Result<Repo, RepoError> {
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
        let result = Repo {
            objects_dir: repo_dir.open_dir("objects")?,
            repo_dir,
            mode: RepoMode::BareUserOnly,
        };
        Ok(result)
    }
    fn has_object(&self, typ: ObjectType, chk: &Checksum) -> io::Result<bool> {
        self.objects_dir.try_exists(loose_path(chk, typ, self.mode))
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

    pub fn write_content(&mut self, mut file: impl Read) -> io::Result<Checksum> {
        let mut hasher = Sha256::new();
        copy(&mut file, &mut hasher)?;
        let chk = hasher.finalize().to_vec().into_boxed_slice().into();
        let mut fd = self.new_object_fd_mut(ObjectType::File, &chk)?;
        copy(&mut file, &mut fd)?;
        Ok(chk)
    }

    pub fn write_dirmeta(&mut self, meta: &DirMeta) -> io::Result<Checksum> {
        let (chk, val) = gv_hash_and_val(meta);
        let mut fd = self.new_object_fd_mut(ObjectType::DirMeta, &chk)?;
        fd.write_all(&val)?;
        Ok(chk)
    }

    pub fn write_dfd_to_mtree(&mut self, dfd: Dir, mtree: &mut MutableTree) {}
}