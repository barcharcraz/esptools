use std::{env, path::PathBuf};





fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_arg("-Iskse64")
        .use_core()
        .rust_target(bindgen::RustTarget::Nightly)
        .generate().unwrap();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs")).unwrap()
}