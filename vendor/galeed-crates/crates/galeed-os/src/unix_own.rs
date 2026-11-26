//! This desk's own unix extensions, outside the family component `os-platform` (declared from `unix.rs`): the
//! writer lock on a directory, a file's identity, the place for applications' data and the documents folder.

use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Unix keeps a user's documents under the home directory, by this name.
const DOCUMENTS_DIR_NAME: &str = "Documents";

/// Unix keeps an application's machine data under the home directory itself.
pub const LAYOUT: crate::Layout = crate::Layout::Itself;

/// Unix keeps applications' data under the home directory itself (dot-named files and directories).
pub fn app_data_root() -> Option<std::path::PathBuf> {
    crate::user_home()
}

pub fn documents_dir() -> Option<std::path::PathBuf> {
    crate::user_home().map(|h| h.join(DOCUMENTS_DIR_NAME))
}

pub type Held = File;

pub fn lock_dir(dir: &Path) -> std::io::Result<Option<Held>> {
    let f = File::open(dir)?;
    match f.try_lock() {
        Ok(()) => Ok(Some(f)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(e)) => Err(e),
    }
}

pub fn keep_owner_only(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(dir)?.mode();
    if mode & 0o077 == 0 {
        return Ok(());
    }
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode & 0o7700))
}

/// Device and inode.
pub type Id = (u64, u64);

pub fn file_id(f: &File) -> std::io::Result<Id> {
    f.metadata().map(|m| (m.dev(), m.ino()))
}

pub fn id_at(path: &Path) -> std::io::Result<Id> {
    std::fs::metadata(path).map(|m| (m.dev(), m.ino()))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    /// Creating in a directory this user may not write is refused with the system's kind, named, nothing made.
    #[test]
    fn creating_where_the_user_may_not_write_is_refused() {
        let d = crate::scratch_dir("no-write");
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o500)).expect("read-only");
        let at = d.join("secret");
        let mut o = crate::Options::new();
        o.write(true).create_new(true);
        let made = crate::owner_only(&mut o).open(&at);
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700)).expect("writable again");
        let e = made.err().expect("refused");
        assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(e.to_string().starts_with("open "), "{e}");
        assert!(!at.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A data directory others may list and enter is narrowed to its owner; one already its owner's alone is left as it is.
    #[test]
    fn a_data_directory_is_kept_its_owners_alone() {
        use std::os::unix::fs::MetadataExt;
        let d = crate::scratch_dir("owner-only");
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o755)).expect("open to others");
        crate::keep_owner_only(&d).expect("narrowed");
        assert_eq!(std::fs::metadata(&d).unwrap().mode() & 0o777, 0o700);
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o2775)).expect("group-shared, set-group-id");
        crate::keep_owner_only(&d).expect("narrowed");
        assert_eq!(std::fs::metadata(&d).unwrap().mode() & 0o7777, 0o2700, "only the others' and group's bits go");
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o500)).expect("owner read-only");
        crate::keep_owner_only(&d).expect("left as it is");
        assert_eq!(std::fs::metadata(&d).unwrap().mode() & 0o777, 0o500);
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700)).expect("writable again");
        let _ = std::fs::remove_dir_all(&d);
    }
}
