// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use crate::perms::{PermissionsExtExt};
use byteorder::{BE};
use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::*, io_lifetimes::AsFilelike};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt::{self, Debug},
    io::{self, copy, Write},
    path::{PathBuf},
    ptr::null_mut
};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use zvariant::{to_bytes, EncodingContext, Type, Value};

#[repr(transparent)]
#[derive(Serialize, Deserialize, Type, Default)]
pub struct Checksum(pub(self) Box<[u8]>);

impl<T> From<T> for Checksum 
    where Box<[u8]>: From<T>
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

#[derive(Debug, Display, SerializeDisplay, EnumString, DeserializeFromStr, Copy, Clone)]
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

pub fn loose_path(checksum: &str, typ: ObjectType, mode: RepoMode) -> PathBuf {
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
pub struct DirTreeFile {
    // note: filenames are NFC UTF-8
    // TODO: maybe store filenames using PEP-383 style surrogate escapes
    pub name: String,
    pub checksum: Checksum,
}

#[derive(Serialize, Deserialize, Debug, Type)]
struct DirTreeFileRef<'a> {
    name: &'a str,
    checksum: &'a [u8],
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTreeDir {
    pub name: String,
    pub checksum: Box<[u8]>,
    pub meta_checksum: Box<[u8]>,
}
#[derive(Serialize, Deserialize, Debug, Type)]
struct DirTreeDirRef<'a> {
    name: &'a str,
    checksum: &'a [u8],
    meta_checksum: &'a [u8],
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTree {
    pub files: Vec<DirTreeFile>,
    pub dirs: Vec<DirTreeDir>,
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTreeRef<'a> {
    #[serde(borrow)]
    files: Vec<DirTreeFileRef<'a>>,
    #[serde(borrow)]
    dirs: Vec<DirTreeDirRef<'a>>,
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
    #[zvariant(signature = "a{ayay}")]
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
}

#[test]
fn test_sigs_match_borrowed() {
    assert_eq!(DirTreeFile::signature(), DirTreeFileRef::signature());
    assert_eq!(DirTreeDir::signature(), DirTreeDirRef::signature());
    assert_eq!(DirTree::signature(), DirTreeRef::signature());


}

pub enum MutableTreeType {
    Lazy,
    Whole,
}

#[derive(Debug)]
pub struct MutableTree {
    parent: *mut MutableTree,
    pub contents_checksum: Checksum,
    pub metadata_checksum: Checksum,
    pub files: BTreeMap<String, Checksum>,
    pub dirs: BTreeMap<String, MutableTree>,
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

impl MutableTree {
    fn new() -> Self {
        let ctx = EncodingContext::<BE>::new_gvariant(0);
        Self {
            parent: null_mut(),
            contents_checksum: Default::default(),
            metadata_checksum: Sha256::digest(to_bytes(ctx, &DirMeta::default()).unwrap())
                .to_vec()
                .into(),
            files: Default::default(),
            dirs: Default::default(),
        }
    }
    pub fn insert_child(&mut self, name: String, mut child: MutableTree) {
        assert!(child.parent == null_mut());
        child.parent = self;
        self.dirs.insert(name, child);
    }
    pub fn get_dirtree(&self) -> DirTree {
        DirTree {
            files: self
                .files
                .iter()
                .map(|(name, checksum)| DirTreeFile {
                    name: name.clone(),
                    checksum: checksum.0.clone().into(),
                })
                .collect(),
            dirs: self
                .dirs
                .iter()
                .map(|(name, mtree)| DirTreeDir {
                    name: name.clone(),
                    checksum: mtree.contents_checksum.0.clone(),
                    meta_checksum: mtree.metadata_checksum.0.clone(),
                })
                .collect(),
        }
    }

    pub fn as_dirtree<'a>(&'a self) -> DirTreeRef<'a> {
        DirTreeRef {
            files: self
                .files
                .iter()
                .map(|(name, checksum)| DirTreeFileRef {
                    name: name.as_ref(),
                    checksum: &checksum.0,
                })
                .collect(),
            dirs: self
                .dirs
                .iter()
                .map(|(name, mtree)| DirTreeDirRef {
                    name: name.as_ref(),
                    checksum: &mtree.contents_checksum.0,
                    meta_checksum: &mtree.metadata_checksum.0,
                })
                .collect(),
        }
    }

    pub fn new_recursive_blank(dir: Dir) -> io::Result<MutableTree> {
        let mut result = Self::new();
        for ent in dir.entries()?.flatten() {
            if ent.file_type()?.is_dir() {
                // TODO: NFC
                result.insert_child(
                    ent.file_name()?,
                    Self::new_recursive_blank(ent.open_dir()?)?,
                );
            } else {
                result
                    .files
                    .insert(ent.file_name()?, hash_file(&mut ent.open()?)?);
            }
        }
        let ctx = EncodingContext::<BE>::new_gvariant(0);
        let mut hasher = Sha256::new();
        let dt = result.as_dirtree();
        hasher.update(to_bytes(ctx, &dt).unwrap());
        result.contents_checksum = hasher.finalize().to_vec().into();
        Ok(result)
    }
}

impl Default for MutableTree {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Repo {
    repo_dir: Dir,
}

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("Repo already exists.")]
    AlreadyExists,
    #[error("Repo is malformed.")]
    MalformedRepo,
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
        let result = Repo { repo_dir };
        let config_data = toml::to_string(&RepoConfig::default()).unwrap();
        result
            .repo_dir
            .open_with("config", OpenOptions::new().write(true).create_new(true))?
            .write_all(config_data.as_ref())?;
        for dir_path in Self::STATE_DIRS {
            result.repo_dir.create_dir(dir_path)?;
        }
        Ok(result)
    }

}
