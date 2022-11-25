use std::{
    fmt::{self, Write as FmtWrite},
    io::{self, Write},
    path::{Path, PathBuf},
    collections::BTreeMap
};

use cap_std::{ambient_authority, fs::*};

use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use zvariant::{Type, Value};

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
    fn is_meta(self) -> bool {
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

fn loose_path(checksum: &str, typ: ObjectType, mode: RepoMode) -> PathBuf {
    [&checksum[0..2], &checksum[2..], typ.to_string().as_str(),
        match mode {
            RepoMode::ArchiveZ2 if typ.is_meta() => &"z",
            _ => &""
        }
    ].into_iter().collect()

}
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct RelatedObject {
    pub name: String,
    pub checksum: Vec<u8>
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
    pub root_dirmeta_checksum: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTreeFile {
    // note: filenames are NFC UTF-8
    // TODO: maybe store filenames using PEP-383 style surrogate escapes
    pub name: String,
    pub checksum: Vec<u8>
}
#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTreeDir {
    pub name: String,
    pub checksum: Vec<u8>,
    pub meta_checksum: Vec<u8>
}
#[derive(Serialize, Deserialize, Debug, Type)]
pub struct DirTree {
    pub files: Vec<DirTreeFile>,
    pub dirs: Vec<DirTreeDir>
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

    pub fn create(path: &Path) -> Result<Repo, RepoError> {
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
