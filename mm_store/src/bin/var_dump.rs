// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use std::path::{PathBuf, Path};


use byteorder::BE;
use clap::{value_parser, Arg, Command};
use mm_store::{DirTree, Commit, DirMeta};
use serde::Deserialize;
use std::fmt::Debug;
use zvariant::{from_slice, EncodingContext, Type};

// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
// enum ObjType {
//     DirTree(DirTree),
//     DirMeta(DirMeta),
//     Commit(Commit)
// }
fn main() {
    let args = Command::new("var_dump")
        .arg(Arg::new("path").required(true).value_parser(value_parser!(PathBuf)))
        .get_matches();
    let p: &Path = args.get_one::<PathBuf>("path").unwrap();
    let content = std::fs::read(p).unwrap();
    let content  = content.as_slice();
    fn print_type<'a: 'de, 'de, T: Debug + Type + Deserialize<'de>>(c: &'a[u8])
    {
        let ctx = EncodingContext::<BE>::new_gvariant(0);
        let v: T = from_slice(c, ctx).unwrap();
        println!("{:?}", v);
    }
    match p.extension().unwrap().to_str().unwrap() {
        "commit" => print_type::<Commit>(content),
        "dirtree" => print_type::<DirTree>(content),
        "dirmeta" => print_type::<DirMeta>(content),
        _ => panic!("bad parameter")
    };
}
