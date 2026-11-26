use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;

pub const COMMAND_WORD: &str = "key.commandOther";

pub const LINK_DELIVERY: &str = "argv";

#[repr(C)]
struct Guid {
    d1: u32,
    d2: u16,
    d3: u16,
    d4: [u8; 8],
}

const DOWNLOADS: Guid = Guid { d1: 0x374D_E290, d2: 0x123F, d3: 0x4565, d4: [0x91, 0x64, 0x39, 0xC4, 0x92, 0x5E, 0x46, 0x7B] };

const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
const ERROR_ACCESS_DENIED: u32 = 5;
const SW_SHOWNORMAL: i32 = 1;
const REG_SZ: u32 = 1;
const MB_ICONERROR: u32 = 0x0000_0010;

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(id: *const Guid, flags: u32, token: *mut c_void, out: *mut *mut u16) -> i32;
    fn ShellExecuteW(hwnd: *mut c_void, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> *mut c_void;
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(p: *mut c_void);
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AttachConsole(pid: u32) -> i32;
    fn GetStdHandle(which: u32) -> *mut c_void;
    fn GetLastError() -> u32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegSetKeyValueW(key: *mut c_void, sub: *const u16, name: *const u16, kind: u32, data: *const c_void, len: u32) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn known(id: &Guid) -> Result<String, String> {
    let mut out: *mut u16 = std::ptr::null_mut();
    // SAFETY: a valid folder id; `out` receives a buffer freed with CoTaskMemFree below (also on failure, per the docs).
    let hr = unsafe { SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut out) };
    let got = if hr >= 0 && !out.is_null() {
        // SAFETY: a NUL-terminated string the call allocated.
        let len = (0..).take_while(|i| unsafe { *out.add(*i) } != 0).count();
        // SAFETY: `len` units before the NUL.
        Ok(String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(out, len) }))
    } else {
        Err(format!("SHGetKnownFolderPath 答 0x{:08x}", hr as u32))
    };
    // SAFETY: the buffer the call allocated (null is allowed).
    unsafe { CoTaskMemFree(out.cast()) };
    got
}

pub fn system_home() -> String {
    eravon_os::user_home().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn downloads_dir(home: &str) -> String {
    if !home.is_empty() && home == system_home()
        && let Ok(d) = known(&DOWNLOADS)
    {
        return d;
    }
    Path::new(home).join("Downloads").to_string_lossy().into_owned()
}

static CONSOLE: OnceLock<bool> = OnceLock::new();

pub fn console() -> bool {
    *CONSOLE.get_or_init(|| {
        // SAFETY: plain calls; the error is read right after the call that set it.
        unsafe {
            let attached = AttachConsole(ATTACH_PARENT_PROCESS) != 0 || GetLastError() == ERROR_ACCESS_DENIED;
            let out = GetStdHandle(STD_OUTPUT_HANDLE);
            attached || (!out.is_null() && out as isize != -1)
        }
    })
}

pub fn alert(text: &str) {
    if console() {
        return;
    }
    let (t, c) = (wide(text), wide("ERAVON"));
    // SAFETY: two NUL-terminated strings; no owner window.
    unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), MB_ICONERROR) };
}

pub fn open_url(url: &str) -> Result<(), String> {
    let (op, u) = (wide("open"), wide(url));
    // SAFETY: two NUL-terminated strings; no window, parameters or directory.
    let r = unsafe { ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), u.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) } as isize;
    if r > 32 { Ok(()) } else { Err(format!("ShellExecuteW 答 {r}")) }
}

fn hkcu() -> *mut c_void {
    0x8000_0001u32 as i32 as isize as *mut c_void
}

fn reg(sub: &str, name: Option<&str>, value: &str) -> i32 {
    let (s, n, v) = (wide(sub), name.map(wide), wide(value));
    // SAFETY: NUL-terminated strings; the data length counts the terminating NUL, in bytes.
    unsafe { RegSetKeyValueW(hkcu(), s.as_ptr(), n.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()), REG_SZ, v.as_ptr().cast(), (v.len() * 2) as u32) }
}

pub fn arm_links() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("登记不了 eravon:// 链接:找不到本程序的位置:{e}"))?;
    let scheme = eravon_literals::literals::URL_SCHEME;
    let root = format!("Software\\Classes\\{scheme}");
    let command = format!("\"{}\" {} --url \"%1\"", exe.display(), crate::house::seam::V_WINDOW);
    for (sub, name, value) in [(root.clone(), None, format!("URL:{scheme}")), (root.clone(), Some("URL Protocol"), String::new()), (format!("{root}\\shell\\open\\command"), None, command)] {
        let st = reg(&sub, name, &value);
        if st != 0 {
            return Err(format!("登记不了 eravon:// 链接:写不进 HKCU\\{sub}(答 {st})"));
        }
    }
    Ok(())
}
