use std::{env, path::PathBuf, process::Output, io, fmt::{Display, LowerHex}};

use bindgen::{BindgenError};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Bind(bindgen::BindgenError)
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}
impl From<BindgenError> for Error {
    fn from(value: BindgenError) -> Self {
        Error::Bind(value)
    }
}


fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_arg("-ICommonLibSSE-NG/include")
        .allowlist_type("RE::BSArchiveHeader")
        .use_core()
        .rust_target(bindgen::RustTarget::Nightly)
        .generate()?;
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    Ok(bindings.write_to_file(out_path.join("bindings.rs"))?)
}