#[cfg(windows)]
pub mod win32 {
    use std::{
        default::Default,
        mem::transmute,
        os::windows::io::{AsHandle, AsRawHandle},
        ptr::{addr_of, null_mut},
        slice,
    };
    use windows::Win32::{
        Foundation::{HANDLE, PSID},
        Security::{
            Authorization::{
                BuildTrusteeWithSidW, GetEffectiveRightsFromAclW, GetSecurityInfo, SE_FILE_OBJECT,
                TRUSTEE_W,
            },
            ACL, DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SID,
        },
        Storage::FileSystem::{FILE_ACCESS_FLAGS, FILE_GENERIC_READ, READ_CONTROL, FILE_GENERIC_EXECUTE, FILE_GENERIC_WRITE},
        System::Memory::LocalFree,
    };
    pub fn access_mask_to_unix_perms(mask: FILE_ACCESS_FLAGS) -> u8 {
        // returns a u8, not a bool so we can add them.
        fn mask_incl(s: FILE_ACCESS_FLAGS, m: FILE_ACCESS_FLAGS) -> u8 {
            (s & m == m) as u8
        }  
        mask_incl(mask, FILE_GENERIC_EXECUTE) * 1 +
        mask_incl(mask, FILE_GENERIC_WRITE) * 2 +
        mask_incl(mask, FILE_GENERIC_READ) * 4
    }
    pub fn get_owner_permissions(f: impl AsHandle) -> Result<u8, windows::core::Error> {
        let f = f.as_handle();
        unsafe {
            let mut sec_desc = PSECURITY_DESCRIPTOR::default();
            let mut owner = PSID::default();
            let mut dacl: *mut ACL = null_mut();
            GetSecurityInfo(
                HANDLE(f.as_raw_handle() as _),
                SE_FILE_OBJECT,
                (OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION).0,
                Some(&mut owner),
                None,
                Some(&mut dacl),
                None,
                Some(&mut sec_desc),
            )
            .ok()?;
            let mut owner_trustee = TRUSTEE_W::default();
            BuildTrusteeWithSidW(&mut owner_trustee, owner);
            let mut access_rights = FILE_ACCESS_FLAGS::default();
            GetEffectiveRightsFromAclW(dacl, &owner_trustee, &mut access_rights.0).ok()?;
            println!("{:b}", access_rights.0);
            println!("{:o}", access_mask_to_unix_perms(access_rights));
            println!("{:b}", FILE_GENERIC_READ.0);
            // let owner_sid: &SID = &*owner.0.cast();
            // println!("{:?}", owner_sid);
            // println!(
            //     "{:?}",
            //     slice::from_raw_parts(
            //         addr_of!(owner_sid.SubAuthority),
            //         owner_sid.SubAuthorityCount as usize
            //     )
            // );
            // println!("{:?}", *dacl);
            LocalFree(transmute(sec_desc));
            Ok(access_mask_to_unix_perms(access_rights))
        }
    }
    #[test]
    fn test_w32_owner_perms() {
        use std::{fs::{File, OpenOptions}, path::PathBuf};
        let datapath: PathBuf = [env!("CARGO_MANIFEST_DIR"), "testdata"]
            .into_iter()
            .collect();
        let test_f = |p: &str, perm: u8| {
            assert_eq!(get_owner_permissions(OpenOptions::new().open(datapath.join(p)).unwrap()).unwrap(), perm);
        };
        test_f("testpermsro", 4);
        test_f("testpermsrx", 5);
        test_f("testpermsrwx", 7);
    }
}

