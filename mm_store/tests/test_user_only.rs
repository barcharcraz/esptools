// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use byteorder::BE;
use camino::{Utf8PathBuf};
use cap_std::{ambient_authority, fs::Dir};
use mm_store::*;
use std::{io};
use zvariant::{EncodingContext};
fn datapath() -> Utf8PathBuf {
    let mut datapath = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    datapath.push("testdata");
    datapath
}


// #[test]
// fn test_matches_hash_file() -> io::Result<()> {

// }
