use std::path::{Path, PathBuf};

use byteorder::LE;
use clap::{value_parser, Arg, Command};
use mm_store::DirTree;
use serde::{Serialize, Deserialize};
use zvariant::{from_slice, from_slice_for_signature, EncodingContext, Signature, Value, Type};

fn main() {
    let args = Command::new("var_dump")
        .arg(Arg::new("path").value_parser(value_parser!(PathBuf)))
        .get_matches();
    let content = std::fs::read(args.get_one::<PathBuf>("path").unwrap()).unwrap();
    println!("{:?}", content);
    println!("{}", DirTree::signature());
    let v: DirTree = from_slice(
        content.as_slice(),
        EncodingContext::<byteorder::LE>::new_gvariant(0),
    ).unwrap();
    println!("{:?}", v);
}
