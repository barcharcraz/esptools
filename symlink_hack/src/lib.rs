#![feature(pointer_byte_offsets)]
#![allow(non_snake_case)]
#![feature(inline_const)]
#![feature(const_maybe_uninit_zeroed)]

use log::*;
use skse64_sys::*;
use std::{
    collections::BTreeMap,
    fmt::Debug,
    fs::File,
    io::{self, BufWriter, Write},
    mem::{transmute, zeroed, MaybeUninit},
    os::windows::prelude::FromRawHandle,
    path::{Path, PathBuf},
    ptr::null,
};

use std::str;

use std::{
    ffi::*,
    os::windows::prelude::OsStringExt,
    sync::Mutex,
    thread::{self, *},
    time::*,
};

use windows_sys::{
    core::PCWSTR,
    Win32::{
        Foundation::*,
        Storage::FileSystem::{FindFileHandle, FILE_ATTRIBUTE_REPARSE_POINT, WIN32_FIND_DATAA},
        System::{
            Memory::{VirtualProtect, PAGE_READWRITE},
            SystemServices::IO_REPARSE_TAG_SYMLINK,
        },
    },
};

use windows_sys::Win32::{
    Storage::FileSystem::{FindFirstFileA, FindNextFileA},
    System::{
        Com::CoTaskMemFree,
        Diagnostics::Debug::{
            IMAGE_DATA_DIRECTORY, IMAGE_DIRECTORY_ENTRY_IMPORT, IMAGE_NT_HEADERS64,
        },
        LibraryLoader::GetModuleHandleW,
        SystemServices::{IMAGE_DOS_HEADER, IMAGE_IMPORT_BY_NAME, IMAGE_IMPORT_DESCRIPTOR},
        WindowsProgramming::IMAGE_THUNK_DATA64,
    },
    UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
};

const fn make_exe_version(major: u8, minor: u8, build: u16) -> u32 {
    (((major & 0xFF) as u32) << 24)
        | (((minor & 0xFF) as u32) << 16)
        | (((build & 0xFFF) as u32) << 4)
}

const fn make_string<const N: usize>(name: &[u8]) -> [::core::ffi::c_char; N] {
    let mut result: [c_char; N] = [0; N];
    let mut i = 0;
    while i < name.len() {
        result[i] = name[i] as _;
        i += 1;
    }
    result
}

const fn make_u32<const N: usize>(init: &[u32]) -> [u32; N] {
    let mut result: [u32; N] = [0; N];
    let mut i: usize = 0;
    while i < init.len() {
        result[i] = init[i];
        i += 1;
    }
    result
}

unsafe fn wcslen(mut str: *const u16) -> usize {
    let mut len: usize = 0;
    while *str != 0 {
        len += 1;
        str = str.offset(1);
    }
    len
}

unsafe fn as_wide<'a>(str: *const u16) -> &'a [u16] {
    std::slice::from_raw_parts(str, wcslen(str))
}

fn log_dir() -> Option<PathBuf> {
    let mut pbuf: PathBuf;
    unsafe {
        let mut buf: *mut u16 = 0 as _;
        let res = SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, 0, &mut buf);
        if res != S_OK {
            return None;
        }
        pbuf = OsString::from_wide(as_wide(buf)).into();
        CoTaskMemFree(buf as _);
    }
    pbuf.push("My Games/Skyrim Special Edition/SKSE");
    Some(pbuf)
}

#[no_mangle]
pub static SKSEPlugin_Version: SKSEPluginVersionData = SKSEPluginVersionData {
    dataVersion: SKSEPluginVersionData_kVersion as u32,
    pluginVersion: 1,
    name: make_string(b"symlink_hack"),
    author: [0; 256],
    supportEmail: [0i8; 252],
    versionIndependenceEx: SKSEPluginVersionData_kVersionIndependentEx_NoStructUse as u32,
    versionIndependence: SKSEPluginVersionData_kVersionIndependent_Signatures as u32,
    compatibleVersions: make_u32(&[make_exe_version(1, 6, 640)]),
    seVersionRequired: 0,
};
struct MyLogger {
    inner: Mutex<(u64, BufWriter<File>)>,
}
impl MyLogger {
    fn new(p: &Path) -> io::Result<Self> {
        Ok(Self {
            inner: Mutex::new((0, BufWriter::new(File::create(p)?))),
        })
    }
}

impl Log for MyLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let l = &mut *self.inner.lock().unwrap();
            writeln!(l.1, "{:10}: {} - {}", l.0, record.level(), record.args()).unwrap();
            l.0 += 1;
            l.1.flush().unwrap();
        }
    }

    fn flush(&self) {
        self.inner.lock().unwrap().1.flush().unwrap();
    }
}

trait CastOffset {
    unsafe fn byte_offset_cast<U>(self, count: isize) -> *const U;
}
impl<T> CastOffset for *const T {
    unsafe fn byte_offset_cast<U>(self, count: isize) -> *const U {
        self.cast::<u8>().offset(count).cast()
    }
}

fn image_base() -> *const u8 {
    unsafe { GetModuleHandleW(null()) as _ }
}
struct SavedPtrs {
    FindFirstFileA: unsafe extern "system" fn(*const u8, *mut WIN32_FIND_DATAA) -> FindFileHandle,
    FindNextFileA: unsafe extern "system" fn(FindFileHandle, *mut WIN32_FIND_DATAA) -> BOOL,
}

impl Debug for SavedPtrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SavedPtrs")
            .field("FindFirstFileA", &(self.FindFirstFileA as *const u8))
            .field("FindNextFileA", &(self.FindNextFileA as *const u8))
            .finish()
    }
}

static mut savedPtrs: MaybeUninit<SavedPtrs> = MaybeUninit::uninit();
static mut findFileInfo: MaybeUninit<Mutex<BTreeMap<FindFileHandle, CString>>> =
    MaybeUninit::uninit();

fn init_globals() {
    unsafe {
        // note: the full qualification is quite important here since the "windows" crate has
        // functions at the same relative paths that import the "real" function internally, if we got the address
        // of one of those we would have a bad time, as it would go and call our hooked function!
        savedPtrs.write(SavedPtrs {
            FindFirstFileA: ::windows_sys::Win32::Storage::FileSystem::FindFirstFileA,
            FindNextFileA: ::windows_sys::Win32::Storage::FileSystem::FindNextFileA,
        });
        findFileInfo.write(Mutex::new(BTreeMap::<FindFileHandle, CString>::new()));
    }
}

unsafe extern "system" fn myFindFirstFileA(
    filename: *const u8,
    findFileData: *mut WIN32_FIND_DATAA,
) -> FindFileHandle {
    let ptrs = savedPtrs.assume_init_ref();

    let ffh = (ptrs.FindFirstFileA)(filename, findFileData);
    info!(
        "File: {:?}",
        CStr::from_ptr((*findFileData).cFileName.as_ptr() as _)
    );
    {
        let mut inf = findFileInfo.assume_init_ref().lock().unwrap();
        let fname_cstr = CStr::from_ptr(filename as _);
        info!("FindFirstFile: {:?}", fname_cstr);
        inf.insert(ffh, CStr::from_ptr(filename as _).to_owned());
    }
    ffh
}

unsafe extern "system" fn myFindNextFileA(
    handle: FindFileHandle,
    findFileData: *mut WIN32_FIND_DATAA,
) -> BOOL {
    let ptrs = savedPtrs.assume_init_ref();
    let res = (ptrs.FindNextFileA)(handle, findFileData);
    {
        let mut inf = findFileInfo.assume_init_ref().lock().unwrap();
        info!("NextFile in folder: {:?}", inf[&handle]);
        if (*findFileData).dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
            && (*findFileData).dwReserved0 == IO_REPARSE_TAG_SYMLINK
        {
            info!("Found symlink");
        }
    }
    res
}

fn do_hook(module: &str, fns: &[&mut *const u8], names: &[&str]) {
    unsafe {
        let handle = image_base();
        info!("Module Handle: {:?}", handle);
        let dosHeader: *const IMAGE_DOS_HEADER = handle as _;
        info!("DOS Header {:?}", dosHeader);
        let peHeader: *const IMAGE_NT_HEADERS64 =
            dosHeader.byte_offset_cast((*dosHeader).e_lfanew as isize);
        let impDirectory: IMAGE_DATA_DIRECTORY =
            (*peHeader).OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT as usize];
        let mut impDescriptor: *const IMAGE_IMPORT_DESCRIPTOR =
            dosHeader.byte_offset_cast(impDirectory.VirtualAddress as isize);
        info!("{:?}", (*impDescriptor).Name);
        while (*impDescriptor).Name != 0 {
            let libname =
                CStr::from_ptr(dosHeader.byte_offset_cast((*impDescriptor).Name as isize));

            info!("Libname: {:?}", libname);
            if libname.to_bytes() == module.as_bytes() {
                info!("Found Target descriptor: {}", module);
                let mut oft: *const IMAGE_THUNK_DATA64 =
                    handle.offset((*impDescriptor).Anonymous.OriginalFirstThunk as _) as _;
                let mut ft: *const IMAGE_THUNK_DATA64 =
                    handle.offset((*impDescriptor).FirstThunk as _) as _;
                while (*oft).u1.AddressOfData != 0 {
                    if (*oft).u1.AddressOfData & (1 << usize::BITS - 1) != 0 {
                        info!("Found Import by ordinal: {}", (*oft).u1.AddressOfData);
                    }

                    let fn_name: *const IMAGE_IMPORT_BY_NAME =
                        handle.offset((*oft).u1.AddressOfData as _) as _;
                    let fn_name = CStr::from_ptr((*fn_name).Name.as_ptr() as _);
                    info!("Found FN: {:?}", fn_name);
                    if fn_name.to_bytes() == b"FindFirstFileA"  {
                        info!("Hooking FindFirstFileA");
                        let mut oldProtect: u32 = 0;
                        let ft = ft.cast_mut();
                        let res = VirtualProtect(ft as _, 8, PAGE_READWRITE, &mut oldProtect);
                        if res == 0 {
                            info!("Hooking FindFirstFileA failed VirtualProtect");
                            return;
                        }
                        (*ft).u1.Function = myFindFirstFileA as _;
                        let res = VirtualProtect(ft as _, 8, oldProtect, &mut oldProtect);
                        if res == 0 {
                            info!("Resetting protection flags failed for FindFirstFileA");
                            return;
                        }
                        return;
                    }
                    oft = oft.offset(1);
                    ft = ft.offset(1);
                }
                return;
            }
            impDescriptor = impDescriptor.offset(1)
        }
        info!("failed to find targeted library")
    }
}

#[no_mangle]
pub extern "C" fn SKSEPlugin_Load(_skse: *const SKSEInterface) -> c_char {
    let mut p = log_dir().unwrap();

    p.push("symlink_hack.txt");
    log::set_boxed_logger(Box::new(MyLogger::new(&p).unwrap())).unwrap();
    log::set_max_level(LevelFilter::Trace);
    thread::Builder::new()
        .name("symlink_hack log flusher".into())
        .spawn(|| loop {
            sleep(Duration::from_secs(5));
            log::logger().flush();
        })
        .unwrap();
    info!(
        "loading on thread: {:?} id: {:?}",
        thread::current().name().unwrap_or("NONE"),
        thread::current().id()
    );
    init_globals();
    do_hook("KERNEL32.dll");
    info!("testing");

    1
}
