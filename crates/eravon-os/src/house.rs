use crate::named;
use std::path::Path;

pub const APP_FOLDER: &str = "ERAVON";

pub const DATA_FOLDER: &str = "ERAVON";

pub const BUNDLE_CONTENTS: &str = "Contents";
pub const BUNDLE_CODE: &str = "MacOS";
pub const BUNDLE_RESOURCES: &str = "Resources";

pub fn bundle_resources(exe_dir: &Path) -> Option<std::path::PathBuf> {
    if exe_dir.file_name()? != BUNDLE_CODE {
        return None;
    }
    Some(exe_dir.parent()?.join(BUNDLE_RESOURCES))
}

pub fn data_folder() -> &'static str {
    DATA_FOLDER
}

#[cfg(unix)]
#[path = "house_unix.rs"]
mod sys;
#[cfg(windows)]
#[path = "house_windows.rs"]
mod sys;

pub const THREAD_HEAD: &str = "eravon-";

std::thread_local! {
    static THREAD_REFUSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn refuse_threads(on: bool) {
    THREAD_REFUSED.with(|r| r.set(on));
}

pub fn start_named<T: Send + 'static>(name: &str, work: impl FnOnce() -> T + Send + 'static) -> std::io::Result<std::thread::JoinHandle<T>> {
    if THREAD_REFUSED.with(std::cell::Cell::get) {
        return Err(std::io::Error::new(std::io::ErrorKind::OutOfMemory, format!("thread {THREAD_HEAD}{name}: the system would not start a thread")));
    }
    std::thread::Builder::new().name(format!("{THREAD_HEAD}{name}")).spawn(work).map_err(|e| named("thread", format!("{THREAD_HEAD}{name}"), e))
}

pub fn make_executable(path: &Path) -> std::io::Result<()> {
    sys::make_executable(path).map_err(|e| named("make executable", path.display(), e))
}

pub fn at_exit(cb: extern "C" fn()) {
    sys::at_exit(cb);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Zone {
    File(Vec<u8>),
    Rule(String),
    ByYear(Vec<(i64, String)>),
}

pub fn zone_rules() -> Option<Zone> {
    sys::zone_rules()
}

#[cfg(test)]
mod tests {
    use crate::{lock_now, owner_only, pretend_entropy_fails, random, spawn, wipe, LockFile, Options, ENTROPY_SOURCE};

    fn scratch(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("eravon-os-house-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch");
        d
    }

    #[test]
    fn entropy_is_read_and_an_unreadable_source_is_said_by_name() {
        let (a, b) = (random(32).expect("entropy"), random(32).expect("entropy"));
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        pretend_entropy_fails(true);
        let e = random(8).err().expect("refused");
        pretend_entropy_fails(false);
        assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(e.to_string().starts_with(&format!("entropy {ENTROPY_SOURCE}: ")), "{e}");
        assert_eq!(random(8).expect("read again").len(), 8);
    }

    #[test]
    fn the_kernel_lock_is_one_holder_and_goes_with_its_descriptor() {
        let d = scratch("lock");
        let p = d.join("lock");
        let open = || {
            let mut o = Options::new();
            o.read(true).write(true).create(true);
            LockFile::open(owner_only(&mut o), &p).expect("lock file")
        };
        let first = open();
        assert!(lock_now(first.file()), "the first takes it");
        let second = open();
        assert!(!lock_now(second.file()), "held: the second does not take it");
        drop(first);
        assert!(lock_now(second.file()), "let go: taken");
        assert!(p.exists(), "the file stays; it is not the lock");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_bundles_resources_are_beside_its_code_folder() {
        use super::{bundle_resources, BUNDLE_CODE, BUNDLE_CONTENTS, BUNDLE_RESOURCES};
        let d = scratch("bundle-place");
        let contents = d.join("X.app").join(BUNDLE_CONTENTS);
        assert_eq!(bundle_resources(&contents.join(BUNDLE_CODE)), Some(contents.join(BUNDLE_RESOURCES)));
        assert_eq!(bundle_resources(&d), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn wiped_bytes_are_zero() {
        let mut b = *b"plain text";
        wipe(&mut b);
        assert_eq!(b, [0u8; 10]);
    }

    #[test]
    fn a_child_starts_through_the_one_door() {
        let mut c = std::process::Command::new(std::env::current_exe().expect("this test binary"));
        c.arg("--list").stdout(std::process::Stdio::null());
        let mut child = spawn(&mut c).expect("started");
        assert!(child.wait().expect("waited").success());
    }
}
