use std::collections::BTreeMap;

use crate::{Checksum, DirTreeChecksums, OsTreeRepo, DirTree, RepoReadExt, RepoError};

#[derive(Debug)]
pub enum MutableTree<'repo> {
    Lazy {
        checksums: DirTreeChecksums,
        repo: &'repo OsTreeRepo,
    },
    Whole(MutableTreeWhole<'repo>),
}

#[derive(Debug)]
pub struct MutableTreeWhole<'repo> {
    files: BTreeMap<String, Checksum>,
    subdirs: BTreeMap<String, MutableTree<'repo>>,
}

impl<'repo> MutableTree<'repo> {
    pub fn new() -> Self {
        Self::Whole(MutableTreeWhole {
            files: BTreeMap::new(),
            subdirs: BTreeMap::new(),
        })
    }

    pub fn from_dirtree_chk(repo: &'repo OsTreeRepo, dtree: DirTree) -> Self {
        Self::Whole(MutableTreeWhole {
            files: dtree.files,
            subdirs: dtree
                .dirs
                .into_iter()
                .map(|(k, v)| (k, Self::new_lazy_from_repo(repo, v)))
                .collect(),
        })
    }

    pub fn new_lazy_from_repo(repo: &'repo OsTreeRepo, chk: DirTreeChecksums) -> Self {
        Self::Lazy {
            checksums: chk,
            repo: repo,
        }
    }

    pub fn make_whole<'s>(&mut self) -> Result<&mut MutableTreeWhole<'repo>, RepoError>
    where
        's: 'repo,
    {
        use MutableTree::*;
        match self {
            Whole(ref mut w) => Ok(w),
            Lazy { checksums, repo } => {
                let dirtree: DirTree = repo.try_load(&checksums.checksum)?.unwrap();
                *self = Whole(MutableTreeWhole {
                    files: dirtree.files,
                    subdirs: dirtree
                        .dirs
                        .into_iter()
                        .map(|(k, v)| (k, Self::new_lazy_from_repo(repo, v)))
                        .collect(),
                });
                // we just made ourselves whole so this will return a reference to the above just-constructed tree
                self.make_whole()
            }
        }
    }
    pub fn ensure_dir(&mut self, dir_name: &str) -> Result<&mut MutableTree<'repo>, RepoError> {
        let tree = self.make_whole()?;
        if tree.files.contains_key(dir_name) {
            return Err(RepoError::InvalidMtree(format!(
                "Can't replace file with directory: {dir_name}"
            )));
        }
        Ok(tree
            .subdirs
            .entry(dir_name.to_owned())
            .or_insert(Self::new()))
    }
}

impl Default for MutableTree<'_> {
    fn default() -> Self {
        Self::new()
    }
}
