//! The sole unsafe module is the Windows create-with-protected-DACL boundary.
use std::fs::File;
use std::io;
use std::path::Path;

#[cfg(unix)]
pub(crate) fn create_new(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn create_new(_path: &Path) -> io::Result<File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "protected credential files unavailable",
    ))
}

#[cfg(windows)]
pub(crate) use windows::create_new;

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use std::ptr::null_mut;

    type Handle = *mut c_void;
    #[repr(C)]
    struct SecurityAttributes {
        length: u32,
        descriptor: *mut c_void,
        inherit: i32,
    }
    #[repr(C)]
    struct SidAndAttributes {
        sid: *mut c_void,
        attributes: u32,
    }
    #[repr(C)]
    struct TokenUser {
        user: SidAndAttributes,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        fn LocalFree(pointer: *mut c_void) -> *mut c_void;
        fn CreateFileW(
            name: *const u16,
            access: u32,
            sharing: u32,
            security: *const SecurityAttributes,
            disposition: u32,
            flags: u32,
            template: Handle,
        ) -> Handle;
    }
    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(process: Handle, access: u32, token: *mut Handle) -> i32;
        fn GetTokenInformation(
            token: Handle,
            class: u32,
            information: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
        fn ConvertSidToStringSidW(sid: *mut c_void, text: *mut *mut u16) -> i32;
        fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text: *const u16,
            revision: u32,
            descriptor: *mut *mut c_void,
            size: *mut u32,
        ) -> i32;
    }

    struct OwnedHandle(Handle);
    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    struct LocalAllocation(*mut c_void);
    impl Drop for LocalAllocation {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0);
            }
        }
    }

    pub(crate) fn create_new(path: &Path) -> io::Result<File> {
        let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if name[..name.len() - 1].contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid credential path",
            ));
        }
        // SAFETY: Win32 outputs have aligned writable storage, all owned handles
        // and allocations are wrapped immediately, and the descriptor outlives
        // CreateFileW. No secret has been generated into this file yet.
        unsafe {
            let mut token = null_mut();
            if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token) == 0 {
                return Err(io::Error::last_os_error());
            }
            let token = OwnedHandle(token);
            let mut needed = 0;
            GetTokenInformation(token.0, 1, null_mut(), 0, &mut needed);
            if needed == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut information =
                vec![0_usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
            if GetTokenInformation(
                token.0,
                1,
                information.as_mut_ptr().cast(),
                needed,
                &mut needed,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            let user = &*(information.as_ptr().cast::<TokenUser>());
            let mut sid_text = null_mut();
            if ConvertSidToStringSidW(user.user.sid, &mut sid_text) == 0 {
                return Err(io::Error::last_os_error());
            }
            let sid_allocation = LocalAllocation(sid_text.cast());
            let mut sid_len = 0;
            while *sid_text.add(sid_len) != 0 {
                sid_len += 1;
            }
            let sid = String::from_utf16(std::slice::from_raw_parts(sid_text, sid_len))
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "owner SID unavailable"))?;
            drop(sid_allocation);
            // Protected DACL with one ACE for the process owner's SID.
            let sddl: Vec<u16> = format!("D:P(A;;FA;;;{sid})")
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let mut descriptor = null_mut();
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                null_mut(),
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            let descriptor = LocalAllocation(descriptor);
            let attributes = SecurityAttributes {
                length: std::mem::size_of::<SecurityAttributes>() as u32,
                descriptor: descriptor.0,
                inherit: 0,
            };
            let handle = CreateFileW(
                name.as_ptr(),
                0x4000_0000,
                0,
                &attributes,
                1,
                0x0020_0080,
                null_mut(),
            );
            if handle as isize == -1 {
                return Err(io::Error::last_os_error());
            }
            // CREATE_NEW rejects existing names including reparse points;
            // OPEN_REPARSE_POINT additionally prevents final traversal.
            Ok(File::from_raw_handle(handle))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[link(name = "advapi32")]
        extern "system" {
            fn GetFileSecurityW(
                name: *const u16,
                requested: u32,
                descriptor: *mut c_void,
                length: u32,
                needed: *mut u32,
            ) -> i32;
            fn GetSecurityDescriptorControl(
                descriptor: *const c_void,
                control: *mut u16,
                revision: *mut u32,
            ) -> i32;
            fn GetSecurityDescriptorDacl(
                descriptor: *const c_void,
                present: *mut i32,
                acl: *mut *mut c_void,
                defaulted: *mut i32,
            ) -> i32;
            fn GetSecurityDescriptorOwner(
                descriptor: *const c_void,
                owner: *mut *mut c_void,
                defaulted: *mut i32,
            ) -> i32;
            fn GetAce(acl: *const c_void, index: u32, ace: *mut *mut c_void) -> i32;
            fn EqualSid(first: *const c_void, second: *const c_void) -> i32;
        }

        #[test]
        fn windows_file_is_protected_and_owner_only_before_any_secret_write() {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("credential.json");
            drop(create_new(&path).unwrap());
            assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
            let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                let mut needed = 0;
                GetFileSecurityW(name.as_ptr(), 5, null_mut(), 0, &mut needed);
                assert!(needed > 0);
                let mut bytes =
                    vec![0_usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
                let descriptor = bytes.as_mut_ptr().cast();
                assert_ne!(
                    GetFileSecurityW(name.as_ptr(), 5, descriptor, needed, &mut needed),
                    0
                );
                let mut control = 0;
                let mut revision = 0;
                assert_ne!(
                    GetSecurityDescriptorControl(descriptor, &mut control, &mut revision),
                    0
                );
                assert_ne!(control & 0x1000, 0, "SE_DACL_PROTECTED");
                let mut present = 0;
                let mut defaulted = 0;
                let mut acl: *mut c_void = null_mut();
                assert_ne!(
                    GetSecurityDescriptorDacl(descriptor, &mut present, &mut acl, &mut defaulted),
                    0
                );
                assert_eq!(present, 1);
                assert!(!acl.is_null());
                // ACL header's AceCount follows revision/reserved/size.
                assert_eq!(*(acl.cast::<u16>().add(2)), 1);
                let mut ace: *mut c_void = null_mut();
                assert_ne!(GetAce(acl, 0, &mut ace), 0);
                assert_eq!(*ace.cast::<u8>(), 0, "ACCESS_ALLOWED_ACE_TYPE");
                assert_eq!(*ace.cast::<u8>().add(1), 0, "explicit, non-inherited ACE");
                assert_eq!(*ace.cast::<u32>().add(1), 0x001f_01ff, "FILE_ALL_ACCESS");
                let mut owner = null_mut();
                assert_ne!(
                    GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted),
                    0
                );
                assert_ne!(EqualSid(owner, ace.cast::<u8>().add(8).cast()), 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_never_overwrites_an_existing_credential_name() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("credential.json");
        std::fs::write(&path, b"synthetic-existing").unwrap();
        assert!(create_new(&path).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"synthetic-existing");
    }

    #[cfg(unix)]
    #[test]
    fn unix_file_is_owner_only_before_any_secret_write() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("credential.json");
        drop(create_new(&path).unwrap());
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
