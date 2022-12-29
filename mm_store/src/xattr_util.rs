// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use std::{
    ffi::{c_void},
};


use windows::{
    Win32::{
        Foundation::{BOOLEAN, HANDLE, NTSTATUS},
        System::WindowsProgramming::IO_STATUS_BLOCK,
    },
};

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
struct FILE_FULL_EA_INFORMATION {
    NextEntryOffset: u32,
    Flags: u8,
    EaNameLength: u8,
    EaValueLength: u16,
    EaName: [u8],
}

#[link(name = "ntdll")]
#[allow(non_snake_case)]
extern "system" {
    fn NtQueryEaFile(
        FileHandle: HANDLE,
        IoStatusBlock: *mut IO_STATUS_BLOCK,
        Buffer: *mut c_void,
        length: u32,
        ReturnSingleEntry: BOOLEAN,
        EaList: *mut c_void,
        EaListLength: u32,
        EaIndex: *const u32,
        RestartScan: BOOLEAN,
    ) -> NTSTATUS;
}
#[cfg(test)]
mod tests {
    use std::{ptr::{null_mut, null, from_raw_parts}, ffi::CStr};

    use widestring::u16cstr;
    use windows::{core::PCWSTR, Win32::{Storage::FileSystem::{CreateFile2, FILE_GENERIC_READ, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING}, System::WindowsProgramming::IO_STATUS_BLOCK}};
    use super::{NtQueryEaFile, FILE_FULL_EA_INFORMATION};
    #[test]
    #[ignore]
    fn test_query_attrs() {
        unsafe {
            let f = CreateFile2(
        PCWSTR::from_raw(u16cstr!("C:\\Users\\bartoc\\source\\ostree-test\\repo\\objects\\89\\5a1646b95228a5385fa5500f94507e09046e33d1db921292836db437206f39.file").as_ptr()),
        FILE_GENERIC_READ, FILE_SHARE_DELETE | FILE_SHARE_READ | FILE_SHARE_WRITE, OPEN_EXISTING, None).unwrap();
            let mut full_ea_info = [0u8; 100];
            let mut status_block = IO_STATUS_BLOCK::default();
            let res = NtQueryEaFile(
                f,
                &mut status_block,
                full_ea_info.as_mut_ptr().cast(),
                full_ea_info.len() as _,
                false.into(),
                null_mut(),
                0,
                null(),
                false.into(),
            );
            println!("{:X?}", res);
            let first_ea =
                from_raw_parts::<FILE_FULL_EA_INFORMATION>(full_ea_info.as_ptr().cast(), 0);
            let first_ea = from_raw_parts::<FILE_FULL_EA_INFORMATION>(
                first_ea as _,
                (*first_ea).EaNameLength as usize + (*first_ea).EaValueLength as usize + 2,
            );
            println!("{:?}", first_ea.as_ref());
            println!("{:?}", CStr::from_bytes_until_nul(&(*first_ea).EaName));
            println!(
                "{:?} {:?}",
                status_block.Anonymous.Status, status_block.Information
            );
        }
    }
}
