use std::path::{Path, PathBuf};

use mm_archive::zip_rs::Archive;
use std::ops::ControlFlow;
#[test]
fn test1() {
    let archivepath: PathBuf = [env!("CARGO_MANIFEST_DIR"), "testdata", "testdata1.zip"]
        .into_iter()
        .collect();
    let mut archive = Archive::from_path(&archivepath).unwrap();
    archive.for_each_entry(|e| {
        let e = match e {
            Err(e) => return ControlFlow::Break(e),
            Ok(e) => e,
        };
        println!("name: {}", e.0.name());
        println!("version made by: {:?}", e.0.version_made_by());
        ControlFlow::Continue(())
    });
}
