// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use camino::{Utf8Path, Utf8PathBuf};

use mm_store::*;

fn datapath() -> Utf8PathBuf {
    let mut datapath = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    datapath.push("testdata");
    datapath
}

#[test]
fn test1() -> Result<(), RepoError> {
    let tmpdir = env!("CARGO_TARGET_TMPDIR");
    println!("{}", tmpdir);
    let repo = OsTreeRepo::create(Utf8PathBuf::from_iter([tmpdir, "testrepo1"].iter()))?;
    
    Ok(())
}

// #[test]
// fn test_matches_hash_file() -> io::Result<()> {

// }
