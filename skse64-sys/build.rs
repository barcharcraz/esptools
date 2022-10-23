use std::{env, path::PathBuf};

const TYPES: &[&str] = &[
    "PluginInfo",
    "SKSEInterface",
    "SKSEScaleformInterface",
    "SKSESerializationInterface",
    "SKSETaskInterface",
    "SKSEPapyrusInterface",
    "SKSEMessagingInterface",
    "SKSEObjectInterface",
    "SKSETrampolineInterface",
    "SKSEPluginVersionData"
];

const VARS: &[&str] = &[
    "kPluginHandle_.*",
    "kInterface_.*"
];


fn main() {
    println!("cargo:rerun-if-changed=wrapper.hpp");
    let mut bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_args(["-Iskse64", "-Icommon"])
        .use_core()
        .rust_target(bindgen::RustTarget::Nightly);
    for ty in TYPES {
        bindings = bindings.allowlist_type(ty);
    }
    for var in VARS {
        bindings = bindings.allowlist_var(var);
    }
    let bindings = bindings.generate().unwrap();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs")).unwrap()
}
