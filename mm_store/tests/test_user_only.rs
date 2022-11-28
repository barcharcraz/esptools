// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use byteorder::BE;
use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use mm_store::*;
use std::{io, path::PathBuf};
use zvariant::{from_slice, to_bytes, EncodingContext};
fn datapath() -> Utf8PathBuf {
    let mut datapath = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    datapath.push("testdata");
    datapath
}

#[test]
fn test_matches_hash_blank() -> io::Result<()> {
    let mut d = Dir::open_ambient_dir(datapath(), ambient_authority())?;
    let t = MutableTree::new_recursive_blank(d.open_dir("tree1")?)?;
    let ctx = EncodingContext::<BE>::new_gvariant(0);
    assert_eq!(
        hex::encode(t.metadata_checksum),
        "446a0ef11b7cc167f3b603e585c7eeeeb675faa412d5ec73f62988eb0b6c5488"
    );
    assert_eq!(
        hex::encode(hash_file(&mut d.open("tree1/test1").unwrap()).unwrap()),
        "81381808c56d4f3d643ec12f6a18fcf2993af8daba7c2a1b2cbfc315b39424c6"
    );
    assert_eq!(
        hex::encode(t.contents_checksum),
        "2978d502d1a9ba2745d6627857e28975247059faf350e30119bffde04412bfb2"
    );

    Ok(())
}

// #[test]
// fn test_matches_hash_file() -> io::Result<()> {

// }
