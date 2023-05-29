use std::collections::BTreeMap;

use crate::{Checksum, DirTree, DirTreeChecksums, OsTreeRepo, RepoError, RepoReadExt};

#[derive(Debug)]
pub enum MutableTree<'repo> {
    Lazy(MutableTreeLazy<'repo>),
    Whole(MutableTreeWhole<'repo>),
}

#[derive(Debug)]
pub struct MutableTreeLazy<'repo> {
    checksums: DirTreeChecksums,
    repo: &'repo OsTreeRepo,
}

#[derive(Debug)]
pub struct MutableTreeWhole<'repo> {
    files: BTreeMap<String, Checksum>,
    subdirs: BTreeMap<String, MutableTree<'repo>>,
}

impl<'repo> MutableTreeLazy<'repo> {
    pub fn to_whole(&self) -> Result<MutableTreeWhole<'repo>, RepoError> {
        let dirtree: DirTree = self.repo.try_load(&self.checksums.checksum)?.unwrap();
        Ok(MutableTreeWhole {
            files: dirtree.files,
            subdirs: dirtree
                .dirs
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        MutableTree::Lazy(Self {
                            repo: self.repo,
                            checksums: v,
                        }),
                    )
                })
                .collect(),
        })
    }
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
        Self::Lazy(MutableTreeLazy {
            checksums: chk,
            repo: repo,
        })
    }
    pub fn make_whole(&mut self) -> Result<&mut MutableTreeWhole<'repo>, RepoError> {
        use MutableTree::*;
        match self {
            Whole(ref mut w) => Ok(w),
            Lazy(l) => {
                *self = Self::Whole(l.to_whole()?);
                self.make_whole()
            }
        }
    }
    pub fn ensure_dir(&mut self, dir_name: String) -> Result<&mut MutableTree<'repo>, RepoError> {
        let tree = self.make_whole()?;
        if tree.files.contains_key(&dir_name) {
            return Err(RepoError::InvalidMtree(format!(
                "Can't replace file with directory: {dir_name}"
            )));
        }
        Ok(tree.subdirs.entry(dir_name).or_insert(Self::new()))
    }
    pub fn replace_file(&mut self, file_name: String, chk: Checksum) -> Result<(), RepoError> {
        let tree = self.make_whole()?;
        if tree.subdirs.contains_key(&file_name) {
            return Err(RepoError::InvalidMtree(format!(
                "Can't replace directory with file: {}",
                file_name
            )));
        }
        tree.files.insert(file_name, chk);
        Ok(())
    }
}

impl Default for MutableTree<'_> {
    fn default() -> Self {
        Self::new()
    }
}
