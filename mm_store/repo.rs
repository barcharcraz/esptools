use std::{
    collections::BTreeMap,
    fmt::{self, Write as FmtWrite, Debug},
    io::{self, copy, ErrorKind, Write},
    path::{Path, PathBuf},
    ptr::null_mut, ops::Deref, borrow::Borrow,
};
use crate::perms::{self, PermissionsExtExt};
use byteorder::{BE, LE};
use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::*, io_lifetimes::AsFilelike};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use sha2::{Digest, Sha256};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use zvariant::{to_bytes, EncodingContext, Type, Value};

#[repr(transparent)]
#[derive(Serialize, Deserialize, Type, Default)]
pub struct ChecksumVec(Vec<u8>);

impl Deref for ChecksumVec {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<T> for ChecksumVec
    where 
        Vec<u8>: AsRef<T>,
        T: ?Sized
{
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}



impl From<Vec<u8>> for ChecksumVec {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

// impl AsRef<[u8]> for ChecksumVec {
//     fn as_ref(&self) -> &[u8] {
//         &self.0
//     }
// }

impl Debug for ChecksumVec {
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
    pub checksum: Vec<u8>,
}
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct Commit<'a> {
    #[serde(borrow)]
    pub metadata: BTreeMap<String, Value<'a>>,
    pub parent: Vec<u8>,
    pub related_objects: Vec<RelatedObject>,
    pub subject: String,
    pub body: String,
    pub timestamp: u64,
    pub root_dirtree_checksum: Vec<u8>,
    pub root_dirmeta_checksum: Vec<u8>,
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
    pub checksum: ChecksumVec,
}

#[derive(Serialize, Deserialize, Debug, Type)]
struct DirTreeFileRef<'a> {
    name: &'a str,
    checksum: &'a [u8],
}
#[test]
fn test_dir_tree_file_ref_match() {
    assert_eq!(DirTreeFile::signature(), DirTreeFileRef::signature());
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTreeDir {
    pub name: String,
    pub checksum: Vec<u8>,
    pub meta_checksum: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug, Type)]
struct DirTreeDirRef<'a> {
    name: &'a str,
    checksum: &'a [u8],
    meta_checksum: &'a [u8],
}

#[test]
fn test_dir_tree_dir_ref_match() {
    assert_eq!(DirTreeDir::signature(), DirTreeDirRef::signature());
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTree {
    pub files: Vec<DirTreeFile>,
    pub dirs: Vec<DirTreeDir>,
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
    pub xattrs: Vec<(Vec<u8>, Vec<u8>)>
}

fn canonical_mode(m: u32) -> u32 {
    m & ( /* IFMT */ 0o170000 | 0o755)
}

impl FileHeader {
    pub fn cannonical_from_file(file: impl AsFilelike) -> io::Result<Self> {
        let o = file.unixy_mode()?;
        Ok(Self {
            uid: 0,
            gid: 0,
            mode: canonical_mode(o),
            rdev: 0,
            symlink_target: Default::default(),
            xattrs: Default::default()
        })
    }
}

#[test]
fn test_sigs_match_upstream() {
    assert_eq!(DirMeta::signature(), "(uuua(ayay))");
    assert_eq!(FileHeader::signature(), "(uuuusa(ayay))");
}

pub enum MutableTreeType {
    Lazy,
    Whole,
}

#[derive(Debug)]
pub struct MutableTree {
    parent: *mut MutableTree,
    pub contents_checksum: ChecksumVec,
    pub metadata_checksum: ChecksumVec,
    pub files: BTreeMap<String, ChecksumVec>,
    pub dirs: BTreeMap<String, MutableTree>,
}

impl MutableTree {
    fn new() -> Self {
        let ctx = EncodingContext::<BE>::new_gvariant(0);
        Self {
            parent: null_mut(),
            contents_checksum: Default::default(),
            metadata_checksum: Sha256::digest(to_bytes(ctx, &DirMeta::default()).unwrap()).to_vec().into(),            files: Default::default(),
            dirs: Default::default(),
        }
    }
    pub fn insert_child(&mut self, name: String, mut child: MutableTree) {
        assert!(child.parent == null_mut());
        child.parent = self;
        self.dirs.insert(name, child);
    }
    pub fn new_recursive_blank(dir: Dir) -> io::Result<MutableTree> {
        fn hash_file(file: &mut File) -> io::Result<ChecksumVec> {
            let mut hasher = Sha256::new();
            let ctx = EncodingContext::<BE>::new_gvariant(0);
            let header = FileHeader::cannonical_from_file(&file)?;
            println!("{:?}", header);
            hasher.update(to_bytes(ctx, &header).unwrap());
            copy(file, &mut hasher)?;
            Ok(hasher.finalize().to_vec().into())
        }
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
        for (k, v) in result.files.iter() {
            // it should be structurally impossible for this error to happen
            hasher.update(
                to_bytes(
                    ctx,
                    &DirTreeFileRef {
                        name: k,
                        checksum: v.as_ref(),
                    },
                )
                .unwrap(),
            );
        }
        for (k, v) in result.dirs.iter() {
            hasher.update(
                to_bytes(
                    ctx,
                    &DirTreeDirRef {
                        name: k,
                        checksum: v.contents_checksum.as_ref(),
                        meta_checksum: v.metadata_checksum.as_ref(),
                    },
                )
                .unwrap(),
            );
        }
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
