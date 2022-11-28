// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

#![feature(pointer_byte_offsets)]
#![allow(non_snake_case)]
#![feature(core_intrinsics)]
#![feature(inline_const)]
#![feature(const_maybe_uninit_zeroed)]
#![feature(atomic_from_mut)]

use log::*;
use num_traits::PrimInt;
use bitflags::bitflags;
use std::{
    backtrace::Backtrace,
    collections::BTreeMap,
    ffi::*,
    fmt::{self, Debug, Write as FmtWrite},
    fs::File,
    io::{self, BufWriter, Write},
    mem::{transmute, zeroed, MaybeUninit},
    os::windows::prelude::{FromRawHandle, OsStringExt},
    path::{Path, PathBuf},
    ptr::null,
    result::Result,
    str,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread::{self, *},
    time::*,
};

use windows::{
    core::{HRESULT, PCSTR, PCWSTR},
    Win32::{
        Foundation::*,
        Globalization::{MultiByteToWideChar, CP_ACP, CP_MACCP},
        Storage::FileSystem::{
            CreateFileA, FindFileHandle, GetFileSize, GetFileSizeEx, FILE_ACCESS_FLAGS,
            FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAGS_AND_ATTRIBUTES,
            FILE_READ_ATTRIBUTES, FILE_SHARE_MODE, OPEN_EXISTING, WIN32_FIND_DATAA,
        },
        System::{
            Memory::{
                VirtualProtect, VirtualProtectFromApp, PAGE_PROTECTION_FLAGS, PAGE_READWRITE,
            },
            SystemServices::IO_REPARSE_TAG_SYMLINK,
        },
    },
};

use windows::Win32::{
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

const fn make_u8array<const N: usize>(init: &[u8]) -> [u8; N] {
    let mut result: [u8; N] = [0; N];
    let mut i: usize = 0;
    while i < init.len() {
        result[i] = init[i];
        i += 1;
    }
    result
}
const fn make_u32array<const N: usize>(init: &[u32]) -> [u32; N] {
    let mut result: [u32; N] = [0; N];
    let mut i: usize = 0;
    while i < init.len() {
        result[i] = init[i];
        i += 1;
    }
    result
}

fn log_dir() -> Option<PathBuf> {
    let mut pbuf: PathBuf;
    unsafe {
        let mut buf = SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None).ok()?;
        pbuf = OsString::from_wide(buf.as_wide()).into();
        CoTaskMemFree(Some(buf.as_ptr() as _));
    }
    pbuf.push("My Games/Skyrim Special Edition/SKSE");
    Some(pbuf)
}



bitflags! {
    struct PluginCompatibility: u32 {
        const AddressLibraryPostAE = 1 << 0;
        const Signatures = 1 << 1;
        const StructsPost629 = 1 << 2;
    }
    struct PluginCompatibilityEx: u32 {
        const NoStructUse = 1 << 0;
    }
}
struct SKSEPluginVersionData {
    dataVersion: u32,
    pluginVersion: u32,
    name: [u8; 256],
    author: [u8; 256],
    supportEmail: [u8; 252],
    versionIndependenceEx: PluginCompatibilityEx,
    versionIndependence: PluginCompatibility,
    compatibleVersions: [u32; 16],
    seVersionRequired: u32
}

#[no_mangle]
static SKSEPlugin_Version: SKSEPluginVersionData = SKSEPluginVersionData {
    dataVersion: 1,
    pluginVersion: 1,
    name: make_u8array(b"symlink_hack"),
    author: [0; 256],
    supportEmail: [0; 252],
    versionIndependenceEx: PluginCompatibilityEx::NoStructUse,
    versionIndependence: PluginCompatibility::Signatures,
    compatibleVersions: make_u32array(&[make_exe_version(1, 6, 640)]),
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
    unsafe { GetModuleHandleW(None).unwrap().0 as _ }
}
struct SavedPtrs {
    FindFirstFileA: unsafe extern "system" fn(*const u8, *mut WIN32_FIND_DATAA) -> FindFileHandle,
    FindNextFileA: unsafe extern "system" fn(FindFileHandle, *mut WIN32_FIND_DATAA) -> BOOL,
    FindClose: unsafe extern "system" fn(FindFileHandle) -> BOOL,
}

#[repr(transparent)]
#[derive(Eq, PartialEq)]
struct FindFileHandleOrd(FindFileHandle);
impl PartialOrd for FindFileHandleOrd {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0 .0.partial_cmp(&other.0 .0)
    }
}

impl Ord for FindFileHandleOrd {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0 .0.cmp(&other.0 .0)
    }
}

impl From<FindFileHandle> for FindFileHandleOrd {
    fn from(value: FindFileHandle) -> Self {
        Self(value)
    }
}

static mut SAVED_PTRS: MaybeUninit<SavedPtrs> = MaybeUninit::uninit();
static mut FIND_FILE_INFO: MaybeUninit<Mutex<BTreeMap<FindFileHandleOrd, Box<[u8]>>>> =
    MaybeUninit::uninit();

fn maybe_correct_findFileData(dir: &[u8], findFileData: &mut WIN32_FIND_DATAA) {
    let file = unsafe { CStr::from_ptr((*findFileData).cFileName.as_ptr() as _) };
    let flags = FILE_FLAGS_AND_ATTRIBUTES((*findFileData).dwFileAttributes);
    if flags & FILE_ATTRIBUTE_REPARSE_POINT == FILE_ATTRIBUTE_REPARSE_POINT
        && (*findFileData).dwReserved0 == IO_REPARSE_TAG_SYMLINK
    {
        info!("Found symlink");
        let file = file.to_bytes();
        let mut symlink_path = Vec::<u8>::with_capacity(dir.len() + 2 + file.len());
        symlink_path.append(&mut dir.to_vec());
        symlink_path.push(b'\\');
        symlink_path.append(&mut file.to_owned());
        unsafe {
            let lpfilename = CString::from_vec_unchecked(symlink_path);
            info!("Getting size of: {:?}", lpfilename);

            let handle = CreateFileA(
                PCSTR::from_raw(lpfilename.as_ptr() as _),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            );

            if let Err(err) = handle {
                error!("Failed to open file: {:?}, error {:?}", lpfilename, err);
                return;
            }

            let handle = handle.unwrap();
            let mut fileSize: i64 = 0;
            let result = unsafe { GetFileSizeEx(handle, &mut fileSize) };
            if !result.as_bool() {
                error!("Could not get file size for {:?}", lpfilename);
            }
            (*findFileData).nFileSizeLow = fileSize as u32;
            (*findFileData).nFileSizeHigh = (fileSize >> 32) as u32;
            CloseHandle(handle);
        }
    }
}

unsafe extern "system" fn myFindFirstFileA(
    filename: *const u8,
    findFileData: *mut WIN32_FIND_DATAA,
) -> FindFileHandle {
    let ptrs = SAVED_PTRS.assume_init_ref();

    let ffh = (ptrs.FindFirstFileA)(filename, findFileData);
    {
        let mut inf = FIND_FILE_INFO.assume_init_ref().lock().unwrap();
        let fname_cstr = CStr::from_ptr(filename as _);
        info!("Adding directory {:?}", fname_cstr);
        if let Some(fname_directory_part) = fname_cstr
            .to_bytes()
            .rsplitn(2, |c| *c == b'\\' || *c == b'/')
            .nth(1)
        {
            // we know there isn't a nul byte, but we still need to copy over anyway
            maybe_correct_findFileData(fname_directory_part, &mut *findFileData);

            inf.insert(ffh.into(), Box::from(fname_directory_part));
        }
    }
    ffh
}

unsafe extern "system" fn myFindNextFileA(
    handle: FindFileHandle,
    findFileData: *mut WIN32_FIND_DATAA,
) -> BOOL {
    let ptrs = SAVED_PTRS.assume_init_ref();
    let res = (ptrs.FindNextFileA)(handle, findFileData);
    {
        let inf = FIND_FILE_INFO.assume_init_ref().lock().unwrap();
        if let Some(dir) = inf.get(&handle.into()) {
            maybe_correct_findFileData(dir, &mut *findFileData);
        }
    }
    res
}

unsafe extern "system" fn myFindClose(handle: FindFileHandle) -> BOOL {
    let ptrs = SAVED_PTRS.assume_init_ref();
    let mut inf = FIND_FILE_INFO.assume_init_ref().lock().unwrap();
    inf.remove(&handle.into());
    (ptrs.FindClose)(handle)
}

#[derive(Debug)]
enum HookError {
    VirtualProtectFailed(windows::core::Error),
    FunctionNotFound,
    ModuleNotFound,
}
unsafe fn hook_iat_function(
    module: &[u8],
    function_name: &[u8],
    new_function: usize,
) -> Result<usize, HookError> {
    let handle = image_base();
    let dosHeader: *const IMAGE_DOS_HEADER = handle as _;
    let peHeader: *const IMAGE_NT_HEADERS64 =
        dosHeader.byte_offset_cast((*dosHeader).e_lfanew as isize);
    let impDirectory: IMAGE_DATA_DIRECTORY =
        (*peHeader).OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT.0 as usize];
    let mut impDescriptor: *const IMAGE_IMPORT_DESCRIPTOR =
        dosHeader.byte_offset_cast(impDirectory.VirtualAddress as isize);
    info!("{:?}", (*impDescriptor).Name);
    while (*impDescriptor).Name != 0 {
        let libname = CStr::from_ptr(dosHeader.byte_offset_cast((*impDescriptor).Name as isize));
        if libname.to_bytes() == module {
            let mut oft: *const IMAGE_THUNK_DATA64 =
                handle.offset((*impDescriptor).Anonymous.OriginalFirstThunk as _) as _;
            let mut ft: *const IMAGE_THUNK_DATA64 =
                handle.offset((*impDescriptor).FirstThunk as _) as _;
            while (*oft).u1.AddressOfData != 0 {
                if (*oft).u1.AddressOfData & (1 << usize::BITS - 1) != 0 {
                    info!("Found Import by ordinal: {}", (*oft).u1.AddressOfData);
                    oft = oft.offset(1);
                    ft = ft.offset(1);
                    continue;
                }

                let fn_name: *const IMAGE_IMPORT_BY_NAME =
                    handle.offset((*oft).u1.AddressOfData as _) as _;
                let fn_name = CStr::from_ptr((*fn_name).Name.as_ptr() as _);
                if fn_name.to_bytes() == function_name {
                    let mut oldProtect = PAGE_PROTECTION_FLAGS::default();
                    let ft = ft.cast_mut();
                    let res = VirtualProtect(ft as _, 8, PAGE_READWRITE, &mut oldProtect);
                    res.ok().map_err(|e| HookError::VirtualProtectFailed(e))?;
                    let oldFunction = AtomicU64::from_mut(&mut (*ft).u1.Function)
                        .swap(new_function as _, Ordering::SeqCst);
                    // possible race
                    let res = VirtualProtect(ft as _, 8, oldProtect, &mut oldProtect);
                    res.ok().map_err(|e| HookError::VirtualProtectFailed(e))?;
                    return Ok(oldFunction as _);
                }
                oft = oft.offset(1);
                ft = ft.offset(1);
            }
            return Err(HookError::FunctionNotFound);
        }
        impDescriptor = impDescriptor.offset(1)
    }
    Err(HookError::ModuleNotFound)
}

fn init_hooks() {
    unsafe {
        SAVED_PTRS.write(SavedPtrs {
            FindFirstFileA: transmute(
                hook_iat_function(b"KERNEL32.dll", b"FindFirstFileA", myFindFirstFileA as _)
                    .unwrap(),
            ),
            FindNextFileA: transmute(
                hook_iat_function(b"KERNEL32.dll", b"FindNextFileA", myFindNextFileA as _).unwrap(),
            ),
            FindClose: transmute(
                hook_iat_function(b"KERNEL32.dll", b"FindClose", myFindClose as _).unwrap(),
            ),
        });
    }
}

fn init_globals() {
    unsafe {
        FIND_FILE_INFO.write(Mutex::new(BTreeMap::<FindFileHandleOrd, Box<[u8]>>::new()));
    }
}

#[repr(C)]
struct SKSEInterface(pub(self)[u8; 0]);

#[no_mangle]
extern "C" fn SKSEPlugin_Load(_skse: *const SKSEInterface) -> c_char {
    std::panic::set_hook(Box::new(|info| {
        let bt = Backtrace::force_capture();
        let msg = info
            .payload()
            .downcast_ref::<&'static str>()
            .unwrap_or(&"No Panic Message");
        let mut message = String::new();
        write!(
            message,
            "Panic, thread '{}' panicked at '{}'",
            thread::current().name().unwrap_or("<unnamed>"),
            msg
        ).unwrap();
        if let Some(loc) = info.location() {
            write!(message, ": {}:{}", loc.file(), loc.line()).unwrap();
        }
        writeln!(message, "{:?}", bt).unwrap();
        error!("{}", message);
    }));
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
    init_hooks();
    info!("testing");

    1
}
