use std::collections::BTreeMap;

use crate::{
    Checksum, DirMeta, DirTree, DirTreeChecksums, OsTreeRepo, RepoError, RepoErrorKind, RepoReadExt, RepoWrite, RepoWriteObject
};

#[derive(Debug, Clone)]
pub enum MutableTree<'repo> {
    Lazy(MutableTreeLazy<'repo>),
    Whole(MutableTreeWhole<'repo>),
}

#[derive(Debug, Clone)]
pub struct MutableTreeLazy<'repo> {
    checksums: DirTreeChecksums,
    repo: &'repo OsTreeRepo,
}

#[derive(Debug, Clone)]
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

impl<'repo> MutableTreeWhole<'repo> {
    fn _dir_chk_list<'a>(
        dirs: impl IntoIterator<Item = (String, MutableTree<'repo>)>,
        repo: &'a mut OsTreeRepo,
    ) -> Result<BTreeMap<String, DirTreeChecksums>, RepoError> {
        dirs.into_iter()
            .map(
                |(dir, tree)| -> Result<(String, DirTreeChecksums), RepoError> {
                    match tree {
                        MutableTree::Lazy(t) => Ok((dir, t.checksums)),
                        MutableTree::Whole(w) => {
                            Ok((dir, w.into_lazy(repo)?.checksums))
                        },
                    }
                },
            )
            .collect()
    }
    pub fn into_lazy<'a>(self, repo: &'a mut OsTreeRepo) -> Result<MutableTreeLazy<'a>, RepoError>
    {
        let dirtree = DirTree {
            files: self.files,
            dirs: Self::_dir_chk_list(self.subdirs, repo)?
        };
        let dirmeta = DirMeta::default();
        let checksums = DirTreeChecksums {
            checksum: repo.write(&dirtree)?,
            meta_checksum: repo.write(&dirmeta)?
        };

        Ok(MutableTreeLazy {
            checksums,
            repo,
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
    pub fn into_lazy<'a>(self, repo: &'a mut OsTreeRepo) -> Result<MutableTreeLazy<'a>, RepoError>
    where
        'repo: 'a
    {
        use MutableTree::*;
        match self {
            Lazy(l) => Ok(l),
            Whole(w) => w.into_lazy(repo)
        }
    }

    pub fn make_lazy<'a>(&'_ mut self, repo: &'a mut OsTreeRepo) -> Result<&'a mut MutableTreeLazy<'_>, RepoError>
    where
        'a: 'repo
    {
        use MutableTree::*;
        match self {
            Lazy(l) => return Ok(l),
            Whole(w) => {
                let mut drained = MutableTreeWhole::<'repo> {
                    files: BTreeMap::new(),
                    subdirs: BTreeMap::new()
                };

                drained.files.append(&mut w.files);
                drained.subdirs.append(&mut w.subdirs);
                *self = Lazy(drained.into_lazy(repo)?);
                match self {
                    Lazy(l) => Ok(l),
                    _ => unreachable!()
                }
            }
        }

    }
    pub fn ensure_dir(&mut self, dir_name: &str) -> Result<&mut MutableTree<'repo>, RepoError> {
        let tree = self.make_whole()?;
        if tree.files.contains_key(dir_name) {
            return Err(RepoErrorKind::InvalidMtree(format!(
                "Can't replace file with directory: {dir_name}"
            ))
            .into());
        }
        Ok(tree.subdirs.entry(dir_name.into()).or_insert(Self::new()))
    }
    pub fn replace_file(&mut self, file_name: &str, chk: Checksum) -> Result<(), RepoError> {
        let tree = self.make_whole()?;
        if tree.subdirs.contains_key(file_name) {
            return Err(RepoErrorKind::InvalidMtree(format!(
                "Can't replace directory with file: {}",
                file_name
            ))
            .into());
        }
        tree.files.insert(file_name.into(), chk);
        Ok(())
    }
}

impl Default for MutableTree<'_> {
    fn default() -> Self {
        Self::new()
    }
}
