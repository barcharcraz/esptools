use std::{io::{Read, Seek}, ops::Index, marker::PhantomData};

use crate::traits;
use zip::result::ZipResult;

pub struct Archive<R: Read + Seek>(pub(self) zip::read::ZipArchive<R>);
pub struct Entry<'a>(pub(self) zip::read::ZipFile<'a>);

pub struct Iter<R: Read + Seek> {
    archive: &'a zip::read::ZipArchive<R>,
    i: usize
}

impl<'a, R: Read + Seek> Iterator for Iter<'a, R>
{
    type Item = zip::result::ZipResult<Entry<'a>>;

    #[inline]
    fn next(&mut self) -> Option<zip::result::ZipResult<Entry<'a>>> {
        if self.i >= self.archive.len() {
            None
        } else {
            self.i += 1;
            Some(
                self.archive.by_index(self.i)
                    .map(Entry)
            )
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.archive.len(), Some(self.archive.len()))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n >= self.archive.len() {
            None
        } else {
            self.i = n + 1;
            Some(self.archive.by_index(n).map(Entry))
        }
    }

}

impl<'a, R: Read + Seek> ExactSizeIterator for Iter<R> {}

impl<'a, R: Read + Seek> IntoIterator for &'a mut Archive<R> {
    type Item = zip::result::ZipResult<Entry<'a>>;

    type IntoIter = Iter<R>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            archive: &mut self.0,
            i: 0
        }
    }
}
