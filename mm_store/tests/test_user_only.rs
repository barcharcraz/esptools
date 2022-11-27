// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use byteorder::BE;
use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use mm_store::*;
use std::{io, path::PathBuf};
use zvariant::{to_bytes, EncodingContext, from_slice};
fn datapath() -> Utf8PathBuf {
    let mut datapath = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    datapath.push("testdata");
    datapath
}

#[test]
fn test_matches_hash_blank() -> io::Result<()> {
    let mut d = Dir::open_ambient_dir(datapath(), ambient_authority())?;
    let t = MutableTree::new_recursive_blank(d.open_dir("tree1")?)?;
    let correctDirtreeData = hex::decode("74657374310081381808c56d4f3d643ec12f6a18fcf2993af8daba7c2a1b2cbfc315b39424c6062728").unwrap();
    let ctx = EncodingContext::<BE>::new_gvariant(0);
    let correctDirtree: DirTree = from_slice(&correctDirtreeData, ctx).unwrap();
    println!("Good: {:#?}", correctDirtree);
    println!("Bad: {:#?}", t);
    assert_eq!(
        hex::encode(t.metadata_checksum),
        "446a0ef11b7cc167f3b603e585c7eeeeb675faa412d5ec73f62988eb0b6c5488"
    );
    assert_eq!(
        hex::encode(t.contents_checksum),
        "a8ebaef3054ae4447749286d387f332a116876b49fac667c393989bcf00ae17a"
    );
    
    Ok(())
}


// #[test]
// fn test_matches_hash_file() -> io::Result<()> {

// }
