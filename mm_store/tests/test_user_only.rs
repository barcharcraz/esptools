// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only


use camino::{Utf8PathBuf};




fn datapath() -> Utf8PathBuf {
    let mut datapath = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    datapath.push("testdata");
    datapath
}


// #[test]
// fn test_matches_hash_file() -> io::Result<()> {

// }
