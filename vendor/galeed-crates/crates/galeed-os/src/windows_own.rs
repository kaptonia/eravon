//! This desk's own Windows extensions, outside the family component `os-platform` (declared from `windows.rs`, whose
//! private declarations it uses): the writer lock on a directory, a file's identity, the local application data
//! and documents folders.
//!

use super::*;

// The writer lock takes one of two ways, each with its own name ([`LOCK_BY_DIR`], [`LOCK_BY_STREAM`]), and a
// failure says each way's words: which one holds is for a real machine to settle (the hand checklist names it).

use std::fs::File;
use std::os::windows::io::AsRawHandle;

const FILE_READ_ATTRIBUTES: u32 = 0x0080;
const LOCKFILE_FAIL_IMMEDIATELY: u32 = 0x1;
const LOCKFILE_EXCLUSIVE_LOCK: u32 = 0x2;
const ERROR_INVALID_FUNCTION: i32 = 1;
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_LOCK_VIOLATION: i32 = 33;
const ERROR_NOT_SUPPORTED: i32 = 50;
const ERROR_INVALID_PARAMETER: i32 = 87;
/// What the system answers when it will not do a thing to a directory (or a volume does not keep a thing at all).
const REFUSED_FOR_KIND: [i32; 4] = [ERROR_INVALID_FUNCTION, ERROR_ACCESS_DENIED, ERROR_NOT_SUPPORTED, ERROR_INVALID_PARAMETER];

/// The writer lock taken on a handle on the directory itself.
pub const LOCK_BY_DIR: &str = "dir-handle";
/// The writer lock taken on a named stream of the directory (never listed among its files), where the system will not
/// lock a directory's handle.
pub const LOCK_BY_STREAM: &str = "dir-stream";
/// The directory's named stream the second way locks on.
const LOCK_STREAM: &str = ":galeed-desk-lock";
/// The writer lock covers bytes beyond any data: a reader of a file never meets it.
const LOCK_AT: u64 = 0xFFFF_FFFF_0000_0000;
/// `FOLDERID_Documents`.
const FOLDER_DOCUMENTS: Guid = Guid { data1: 0xFDD3_9AD0, data2: 0x238F, data3: 0x46AF, data4: [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7] };

#[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: Handle,
}

#[repr(C)]
#[derive(Default)]
struct FileTime {
    low: u32,
    high: u32,
}

#[repr(C)]
#[derive(Default)]
struct ByHandleInformation {
    attributes: u32,
    created: FileTime,
    accessed: FileTime,
    written: FileTime,
    volume_serial: u32,
    size_high: u32,
    size_low: u32,
    links: u32,
    index_high: u32,
    index_low: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LockFileEx(file: Handle, flags: u32, reserved: u32, low: u32, high: u32, overlapped: *mut Overlapped) -> i32;
    fn GetFileInformationByHandle(file: Handle, info: *mut ByHandleInformation) -> i32;
}

/// Open `path` with the given access, disposition and flags, as a handle the standard library owns from here on.
fn open_raw(path: &Path, access: u32, disposition: u32, flags: u32) -> std::io::Result<File> {
    let name = wide(path);
    let share = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
    // SAFETY: `name` ends with a zero and lives across the call; no security attributes.
    let h = unsafe { CreateFileW(name.as_ptr(), access, share, std::ptr::null_mut(), disposition, flags, std::ptr::null_mut()) };
    if h == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a handle just opened and owned by nothing else.
    Ok(unsafe { File::from_raw_handle(h) })
}

fn refused_for_kind(e: &std::io::Error) -> bool {
    e.raw_os_error().is_some_and(|c| REFUSED_FOR_KIND.contains(&c))
}

/// Both ways' words in one failure.
fn both(first: (&str, std::io::Error), second: (&str, std::io::Error)) -> std::io::Error {
    std::io::Error::new(second.1.kind(), format!("{}: {}; {}: {}", first.0, first.1, second.0, second.1))
}

fn only(way: &str, e: std::io::Error) -> std::io::Error {
    std::io::Error::new(e.kind(), format!("{way}: {e}"))
}

/// The held writer lock: a handle whose lock goes with it.
pub type Held = File;

/// Lock the bytes beyond any data on `f`, without waiting: `Ok(false)` when another holder has them.
fn lock_beyond_data(f: &File) -> std::io::Result<bool> {
    let mut at = Overlapped { internal: 0, internal_high: 0, offset: LOCK_AT as u32, offset_high: (LOCK_AT >> 32) as u32, event: std::ptr::null_mut() };
    // SAFETY: a handle the file owns; the overlapped record lives across this synchronous call.
    if unsafe { LockFileEx(f.as_raw_handle() as Handle, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY, 0, 1, 0, &mut at) } != 0 {
        return Ok(true);
    }
    let e = std::io::Error::last_os_error();
    if e.raw_os_error() == Some(ERROR_LOCK_VIOLATION) { Ok(false) } else { Err(e) }
}

fn held(f: File) -> std::io::Result<Option<Held>> {
    lock_beyond_data(&f).map(|got| got.then_some(f))
}

pub fn lock_dir(dir: &Path) -> std::io::Result<Option<Held>> {
    match open_raw(dir, GENERIC_READ, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS).and_then(held) {
        Ok(got) => Ok(got),
        Err(e) if refused_for_kind(&e) => {
            let mut stream = dir.as_os_str().to_os_string();
            stream.push(LOCK_STREAM);
            open_raw(Path::new(&stream), GENERIC_READ | GENERIC_WRITE, OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL).and_then(held).map_err(|e2| both((LOCK_BY_DIR, e), (LOCK_BY_STREAM, e2)))
        }
        Err(e) => Err(only(LOCK_BY_DIR, e)),
    }
}

pub fn keep_owner_only(dir: &Path) -> std::io::Result<()> {
    std::fs::metadata(dir).map(|_| ())
}

/// The volume's serial number and the file's index on it.
pub type Id = (u64, u64);

pub fn file_id(f: &File) -> std::io::Result<Id> {
    let mut info = ByHandleInformation::default();
    // SAFETY: a handle the file owns; the record is the system's shape.
    if unsafe { GetFileInformationByHandle(f.as_raw_handle() as Handle, &mut info) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok((u64::from(info.volume_serial), (u64::from(info.index_high) << 32) | u64::from(info.index_low)))
}

pub fn id_at(path: &Path) -> std::io::Result<Id> {
    file_id(&open_raw(path, FILE_READ_ATTRIBUTES, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS)?)
}

/// The system's local application data folder (never the roaming one: a key store and a ledger of this machine do
/// not travel to the user's other machines).
pub fn app_data_root() -> Option<std::path::PathBuf> {
    known_folder(&FOLDER_LOCAL_APP_DATA)
}

pub fn documents_dir() -> Option<std::path::PathBuf> {
    known_folder(&FOLDER_DOCUMENTS)
}

/// What only a real machine can settle, printed for the hand checklist (`cargo test -p galeed-os -- --nocapture`):
/// which way the writer lock takes, and whether a directory can be renamed while its writer lock is held.
#[cfg(test)]
mod own_tests {
    use super::*;

    #[test]
    fn which_way_the_writer_lock_takes_and_whether_a_held_directory_renames() {
        let d = crate::scratch_dir("win-lock");
        let by_dir = open_raw(&d, GENERIC_READ, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS).and_then(held).map(|h| h.is_some());
        println!("checklist lock: {LOCK_BY_DIR} {by_dir:?}");
        let held = lock_dir(&d).unwrap();
        assert!(held.is_some() && lock_dir(&d).unwrap().is_none(), "同一刻只一个持有者");
        let moved = d.with_extension("moved");
        let renamed = std::fs::rename(&d, &moved);
        println!("checklist rename while held: {renamed:?}");
        drop(held);
        let _ = std::fs::rename(&moved, &d);
    }
}

/// Windows keeps an application's machine data in a folder of its name in the local application data folder.
pub const LAYOUT: crate::Layout = crate::Layout::AppFolder;
