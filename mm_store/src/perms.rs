// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use std::io;

pub trait PermissionsExtExt {
    /// should never return anything except the first 9 bits
    fn unixy_permissions(self) -> io::Result<u32>;
}
#[cfg(unix)]
pub mod unix {
    use cap_std::io_lifetimes::AsFilelike;

    use super::PermissionsExtExt;
    use std::{io, os::unix::io::{AsFd, OwnedFd}, fs::File};
    use std::os::unix::fs::{MetadataExt, FileExt};
    impl<T: AsFd> PermissionsExtExt for T {
        fn unixy_permissions(self) -> io::Result<u32> {
            let file = self.as_filelike_view::<File>();
            let mode = file.metadata()?.mode();
            Ok(mode & 0o777)
        }
    }
}

#[cfg(windows)]
pub mod win32 {

    use std::{
        default::Default,
        io::{self},
        mem::transmute,
        os::windows::io::{AsHandle, AsRawHandle},
        ptr::null_mut,
    };
    use windows::Win32::{
        Foundation::{HANDLE, PSID},
        Security::{
            Authorization::{
                BuildTrusteeWithSidW, GetEffectiveRightsFromAclW, GetSecurityInfo, SE_FILE_OBJECT,
                TRUSTEE_W,
            },
            CreateWellKnownSid, WinWorldSid, ACL, DACL_SECURITY_INFORMATION,
            GROUP_SECURITY_INFORMATION, OBJECT_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR,
        },
        Storage::FileSystem::{
            FILE_ACCESS_RIGHTS, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        },
        System::Memory::LocalFree,
    };

    use super::PermissionsExtExt;
    pub struct SecurityInfo {
        desc: PSECURITY_DESCRIPTOR,
        owner: PSID,
        group: PSID,
        dacl: *mut ACL,
        _sacl: *mut ACL,
    }
    impl Drop for SecurityInfo {
        fn drop(&mut self) {
            unsafe {
                match LocalFree(transmute(self.desc)) {
                    0 => (),
                    _ => panic!("memory deallocation failed"),
                }
            };
        }
    }

    impl SecurityInfo {
        pub fn new_from_handle(
            h: impl AsHandle,
            info: OBJECT_SECURITY_INFORMATION,
        ) -> Result<Self, windows::core::Error> {
            let mut desc = PSECURITY_DESCRIPTOR::default();
            let mut owner = PSID::default();
            let mut group = PSID::default();
            let mut dacl = null_mut();
            let mut sacl = null_mut();
            unsafe {
                GetSecurityInfo(
                    HANDLE(h.as_handle().as_raw_handle() as _),
                    SE_FILE_OBJECT,
                    info.0,
                    Some(&mut owner),
                    Some(&mut group),
                    Some(&mut dacl),
                    Some(&mut sacl),
                    Some(&mut desc),
                )
                .ok()?;
            }
            Ok(Self {
                desc,
                owner,
                group,
                dacl,
                _sacl: sacl,
            })
        }
        pub fn owner(&self) -> Option<PSID> {
            if self.owner.is_invalid() {
                None
            } else {
                Some(self.owner)
            }
        }
        pub fn group(&self) -> Option<PSID> {
            if self.group.is_invalid() {
                None
            } else {
                Some(self.group)
            }
        }
        pub fn dacl(&self) -> Option<&ACL> {
            unsafe { self.dacl.as_ref() }
        }
    }

    fn access_mask_to_unix_perms(mask: FILE_ACCESS_RIGHTS) -> u32 {
        let mut result = 0;
        if mask & FILE_GENERIC_EXECUTE == FILE_GENERIC_EXECUTE {
            result += 1;
        }
        if mask & FILE_GENERIC_WRITE == FILE_GENERIC_WRITE {
            result += 2;
        }
        if mask & FILE_GENERIC_READ == FILE_GENERIC_READ {
            result += 4;
        }
        result
    }
    fn unix_perms_to_access_mask(perms: u8) -> FILE_ACCESS_RIGHTS {
        let mut result = FILE_ACCESS_RIGHTS::default();
        if perms & 1 == 1 {
            result |= FILE_GENERIC_EXECUTE;
        }
        if perms & 2 == 2 {
            result |= FILE_GENERIC_WRITE;
        }
        if perms & 4 == 4 {
            result |= FILE_GENERIC_READ;
        }
        result
    }
    fn get_unixy_mode(f: impl AsHandle) -> Result<u32, windows::core::Error> {
        let info = SecurityInfo::new_from_handle(
            f,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
        )?;
        unsafe fn unix_perms(sid: PSID, dacl: *const ACL) -> Result<u32, windows::core::Error> {
            let mut trustee = TRUSTEE_W::default();
            BuildTrusteeWithSidW(&mut trustee, sid);
            let mut access_rights = FILE_ACCESS_RIGHTS::default();
            GetEffectiveRightsFromAclW(dacl, &trustee, &mut access_rights.0).ok()?;
            Ok(access_mask_to_unix_perms(access_rights))
        }
        unsafe {
            let owner = unix_perms(info.owner, info.dacl)?;
            let group = unix_perms(info.group, info.dacl)?;
            let mut buf = Box::<[u8]>::new_uninit_slice(100);
            let mut sz = 100;
            CreateWellKnownSid(WinWorldSid, None, transmute(buf.as_mut_ptr()), &mut sz).ok()?;
            let everyone_sid = PSID(buf.as_mut_ptr().cast());
            let everyone = unix_perms(everyone_sid, info.dacl)?;
            Ok(everyone + (group << 3) + (owner << 6))
        }
    }

    impl<T: AsHandle> PermissionsExtExt for T {
        fn unixy_permissions(self) -> io::Result<u32> {
            Ok(get_unixy_mode(self)?)
        }
    }

    #[test]
    fn test_w32_owner_perms() {
        use std::{fs::File, path::PathBuf};
        let datapath: PathBuf = [env!("CARGO_MANIFEST_DIR"), "testdata"]
            .into_iter()
            .collect();
        let test_f = |p: &str, perm: u8| {
            let mode = get_unixy_mode(File::open(datapath.join(p)).unwrap()).unwrap();
            assert_eq!(((mode & 0o700) >> 6) as u8, perm);
        };
        test_f("testpermsro", 4);
        test_f("testpermsrx", 5);
        test_f("testpermsrwx", 7);
    }
}
