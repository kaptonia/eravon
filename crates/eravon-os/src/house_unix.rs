#![allow(unsafe_code)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn make_executable(path: &Path) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

pub fn at_exit(cb: extern "C" fn()) {
    unsafe extern "C" {
        fn atexit(cb: extern "C" fn()) -> std::os::raw::c_int;
    }
    // SAFETY: `atexit` takes a no-argument C callback and keeps it until exit; the caller's callback does not unwind.
    unsafe {
        atexit(cb);
    }
}

pub fn zone_rules() -> Option<super::Zone> {
    let tzif = |b: Vec<u8>| if b.starts_with(b"TZif") { Some(super::Zone::File(b)) } else { None };
    match std::env::var("TZ") {
        Ok(tz) if !tz.trim().is_empty() => {
            let name = tz.trim().trim_start_matches(':');
            let path = if name.starts_with('/') { std::path::PathBuf::from(name) } else { std::path::Path::new("/usr/share/zoneinfo").join(name) };
            match std::fs::read(&path) {
                Ok(b) => tzif(b),
                Err(_) => Some(super::Zone::Rule(name.to_string())),
            }
        }
        _ => std::fs::read("/etc/localtime").ok().and_then(tzif),
    }
}
