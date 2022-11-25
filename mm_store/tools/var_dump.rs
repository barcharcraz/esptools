use std::path::{PathBuf};


use clap::{value_parser, Arg, Command};
use mm_store::DirTree;

use zvariant::{from_slice, EncodingContext, Type};

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
