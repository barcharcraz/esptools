use std::{io::{self, ErrorKind}, path::Path};

use cap_std::{fs::*, ambient_authority};

use thiserror::Error;

#[derive(Debug, PartialEq)]
pub enum RepoMode {
    Bare,
    Archive,
    BareUser,
    BareUserOnly,
    BareSplitXattrs
}

pub struct Repo {
    repo_dir: Dir,
    mode: RepoMode
}

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("Repo already exists.")]
    AlreadyExists,
    #[error("Repo mode {0:?} is not yet supported.")]
    UnsupportedMode(RepoMode),
    #[error("Repo is malformed.")]
    MalformedRepo,
    #[error("IO Error")]
    Io(#[from] io::Error)
}

impl Repo {
    const STATE_DIRS: &'static [&'static str] = &[
        "tmp", "extensions", "state", "refs",
        "refs/heads", "refs/mirrors", "refs/remotes", "objects"
    ];
    pub fn create(path: &Path, mode: RepoMode) -> Result<Repo, RepoError> {
        match std::fs::create_dir(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(e)
        }?;
        if !(mode == RepoMode::BareUserOnly) {
            return Err(RepoError::UnsupportedMode(mode));
        }
        let repo_dir = Dir::open_ambient_dir(path, ambient_authority())?;
        if repo_dir.is_dir("objects") {
            return Err(RepoError::AlreadyExists);
        }
        let result = Repo {
            repo_dir,
            mode
        };
        Ok(result)
    }
}
