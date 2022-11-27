#[cfg(windows)]
mod win32 {
    use std::os::windows::io::AsHandle;
    pub fn get_owner_permissions(f: impl AsHandle) -> u8 {
        let f = f.as_handle();
    }
}
