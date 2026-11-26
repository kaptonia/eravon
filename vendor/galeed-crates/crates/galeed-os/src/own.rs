//! This desk's own extensions of the platform crate, outside the family component `os-platform`: how a system lays
//! out an application's data under the place it gives for it, the one way a directory lock is let go, a written
//! file's bytes and name brought to disk, the directory lock, a file's identity, the home and the documents folder.
//! Declared from `lib.rs` and re-exported at the crate's root, so callers name these as before.

use crate::{imp, named, sync_dir, user_home, CHILDREN};
use std::path::Path;

/// How a system keeps an application's machine data under the place it gives for it ([`app_data_dir`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Under the place itself (unix: the home directory, with dot names).
    Itself,
    /// In a folder of the application's name there (Windows: the local application data folder).
    AppFolder,
}

/// This system's [`Layout`].
pub const LAYOUT: Layout = imp::LAYOUT;

/// ZIKARON's folder in the local application data folder, as ZIKARON names it (read here, never written).
pub const ZIKARON_FOLDER: &str = "ZIKARON";

/// Where ZIKARON keeps its machine data under `root` (the place the system gives for applications' data) when
/// the system keeps it as `layout` says. A plain function of its two inputs, the same on every system, so each
/// system's answer is tested on all.
pub fn app_data_dir_under(root: &Path, layout: Layout) -> std::path::PathBuf {
    match layout {
        Layout::Itself => root.to_path_buf(),
        Layout::AppFolder => root.join(ZIKARON_FOLDER),
    }
}

/// Held while a kernel lock's descriptor is closed (see [`CHILDREN`]).
fn letting_go() -> std::sync::RwLockWriteGuard<'static, ()> {
    CHILDREN.write().unwrap_or_else(|e| e.into_inner())
}

/// **The one way a kernel lock is let go** (see [`CHILDREN`]): under the turn, unlocked by name, then closed. A child
/// still holding its copy of the descriptor keeps nothing.
pub fn release(f: std::fs::File) {
    let _turn = letting_go();
    let _ = f.unlock();
    drop(f);
}

/// Bring a written file's bytes to disk through the handle it was written on (before it is closed and renamed);
/// a failure is named with what failed. [`sync_file`] does the same for a file closed elsewhere, by its path.
pub fn sync_handle(f: &std::fs::File) -> std::io::Result<()> {
    f.sync_all().map_err(|e| named("file sync", "(open handle)", e))
}

/// Bring to disk the name just placed at `placed`: [`sync_dir`] of its directory, with `placed` as the file
/// that landed there.
pub fn sync_placed(placed: &Path) -> std::io::Result<()> {
    let dir = placed.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    sync_dir(dir, placed)
}

/// The head of the scratch directories this crate's tests make in the temporary directory (a family name, so
/// they never meet another product's).
pub const TEST_DIR_HEAD: &str = "galeed-os-";

/// **The one way a test of this crate makes a directory to work in**: empty, of this call's own. The tag says what it
/// is for; the counter keeps two tests that run at once (whatever their tags) from ever sharing one and clearing each
/// other's.
#[cfg(test)]
pub(crate) fn scratch_dir(tag: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("{TEST_DIR_HEAD}{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

/// One writer at a time in a directory: held while the guard lives, let go through [`release`] when it goes.
pub struct DirLock {
    held: Option<imp::Held>,
}

impl Drop for DirLock {
    fn drop(&mut self) {
        if let Some(f) = self.held.take() {
            release(f);
        }
    }
}

/// **Keep a data directory this user's alone**: where the system gives directories modes, one that others may list or
/// enter loses the group's and others' bits (the owner's and the special bits stay; one already its owner's alone is
/// left untouched).
/// Windows keeps an application's data under the user's profile, whose access list is the user's own: nothing to do.
/// The data directories are made by the usual creation (the system's default modes); the desk narrows them here.
pub fn keep_owner_only(dir: &Path) -> std::io::Result<()> {
    imp::keep_owner_only(dir).map_err(|e| named("owner-only", dir.display(), e))
}

/// Take the directory's writer lock now, without waiting: `Ok(None)` when another holder has it.
pub fn lock_dir(dir: &Path) -> std::io::Result<Option<DirLock>> {
    imp::lock_dir(dir).map(|h| h.map(|h| DirLock { held: Some(h) })).map_err(|e| named("lock-dir", dir.display(), e))
}

/// The identity of one open file (which file on which device), to ask later whether a path still names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileId(imp::Id);

/// The identity of the file `f` is open on.
pub fn file_id(f: &std::fs::File) -> std::io::Result<FileId> {
    imp::file_id(f).map(FileId)
}

/// The identity of the file (or directory) `path` names right now.
pub fn id_of(path: &Path) -> std::io::Result<FileId> {
    imp::id_at(path).map(FileId)
}

/// Whether `path` names, right now, the very file `id` was taken from (`false` when it names another, or nothing).
pub fn same_file(id: &FileId, path: &Path) -> bool {
    imp::id_at(path).is_ok_and(|now| now == id.0)
}

/// This user's home directory, as the system gives it ([`user_home`]): absent, empty or not absolute is a named
/// failure ([`std::io::ErrorKind::NotFound`], `home: …`), never `.` or an empty path standing in.
pub fn home() -> std::io::Result<std::path::PathBuf> {
    user_home().filter(|h| h.is_absolute()).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "home: the system names no home directory for this user"))
}

/// The place the system gives this user for applications' data: this desk's default data directory and
/// ZIKARON's pointer live under it (unix: the home directory itself, where they are dot-named; Windows: the
/// local application data folder). A named failure when the system names none.
pub fn app_data_root() -> std::io::Result<std::path::PathBuf> {
    imp::app_data_root().filter(|h| h.is_absolute()).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "app-data: the system names no place for this user's application data"))
}

/// This user's documents directory as the system names it (where exports go by default); `None` when it names none.
pub fn documents_dir() -> Option<std::path::PathBuf> {
    imp::documents_dir().filter(|d| d.is_absolute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        crate::scratch_dir(tag)
    }

    /// Creating where the parent directory is not there is a named failure of the system's kind, and nothing
    /// is made on the way.
    #[test]
    fn creating_under_a_missing_directory_is_said() {
        let d = scratch("parent");
        let at = d.join("absent").join("secret");
        let mut o = Options::new();
        o.write(true).create_new(true);
        let e = owner_only(&mut o).open(&at).err().expect("no parent");
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        assert!(e.to_string().starts_with(&format!("open {}: ", at.display())), "{e}");
        assert!(!d.join("absent").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// ZIKARON's machine data under the place for applications' data, each system's layout answered on every
    /// system: unix under the place itself, Windows in ZIKARON's folder there.
    #[test]
    fn the_contact_machine_data_is_where_each_system_lays_it_out() {
        let root = Path::new("/stand-in/root");
        assert_eq!(app_data_dir_under(root, Layout::Itself), root);
        assert_eq!(app_data_dir_under(root, Layout::AppFolder), root.join(ZIKARON_FOLDER));
    }

    /// A child is started through the one door; a program that is not there, or not to be run, is said with the
    /// system's kind and nothing runs.
    #[test]
    fn a_child_starts_through_the_one_door() {
        let d = scratch("spawn");
        let missing = spawn(&mut std::process::Command::new(d.join("no-such-program"))).err().expect("no such program");
        assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);
        let not_runnable = d.join("not-runnable");
        std::fs::write(&not_runnable, b"").expect("a plain file");
        assert!(spawn(&mut std::process::Command::new(&not_runnable)).is_err(), "a file that may not be run is refused");
        drop(letting_go());
        let mut ok = std::process::Command::new(std::env::current_exe().expect("this test binary"));
        ok.arg("--list").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        assert!(spawn(&mut ok).expect("started").wait().expect("ended").success());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_directory_lock_has_one_holder_and_a_placed_name_syncs() {
        let d = scratch("lock");
        let held = lock_dir(&d).unwrap();
        assert!(held.is_some());
        assert!(lock_dir(&d).unwrap().is_none(), "同一刻只一个持有者");
        // Let go through the one door (`release`, under the turn): a child forked in this moment by a test beside this
        // one keeps nothing.
        drop(held);
        assert!(lock_dir(&d).unwrap().is_some(), "放开即可再取");
        std::fs::write(d.join("placed"), b"x").unwrap();
        sync_placed(&d.join("placed")).unwrap();
        assert!(sync_placed(&d.join("absent").join("f")).unwrap_err().to_string().starts_with("directory sync "), "同步失败具名");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_home_is_absolute_or_a_named_failure() {
        match home() {
            Ok(h) => assert!(h.is_absolute()),
            Err(e) => assert!(e.to_string().starts_with("home: ")),
        }
    }

    #[test]
    fn a_path_names_the_same_file_until_it_is_replaced() {
        let d = scratch("same");
        let p = d.join("f");
        let f = std::fs::File::create(&p).unwrap();
        let id = file_id(&f).unwrap();
        assert!(same_file(&id, &p));
        std::fs::write(d.join("g"), b"").unwrap();
        replace(&d.join("g"), &p).unwrap();
        assert!(!same_file(&id, &p), "换过即不是同一档");
        assert!(!same_file(&id, &d.join("absent")));
        let dir = id_of(&d).unwrap();
        assert!(same_file(&dir, &d) && !same_file(&dir, &p), "目录也有身份");
        assert!(id_of(&d.join("absent")).is_err());
    }

    #[test]
    fn the_documents_are_said_or_absent() {
        assert!(documents_dir().is_none_or(|d| d.is_absolute()));
        match app_data_root() {
            Ok(r) => assert!(r.is_absolute()),
            Err(e) => assert!(e.to_string().starts_with("app-data: ")),
        }
    }
}
