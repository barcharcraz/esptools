use std::ffi::OsString;
use libc::mount;
use std::path::PathBuf;
use std::io::Write;

pub struct ModEnvironment {
    pub modDirectories: Vec<PathBuf>,
    pub overrideDirectory: PathBuf,
    pub targetDirectory: PathBuf
}


impl ModEnvironment {
    pub fn setup(&mut self) {
        let mut data_buf = Vec::new();
        data_buf.write_all(b"-olowerdir=");

        
    }
}