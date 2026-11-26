//! This desk's own places, outside the family component `cli-desktop`: the default data directory, this machine's
//! identifier file, this desk's own folder and the machine directory's names, **resolved here and nowhere else**,
//! for the app and the command line alike.
//!
//! The machine directory is found by the family's own lookup ([`crate::machine::here`]: the pointer file, the older
//! layout, then [`DIR`] in this desk's own folder); the family's machine module takes its two names from here
//! ([`DIR`], [`POINTER`]), so each build's names are written once.

use std::path::{Path, PathBuf};

/// The machine directory's name in this desk's own folder.
#[cfg(not(feature = "chain-sepolia"))]
pub const DIR: &str = ".galeed-desk.d";

/// The pointer file's name, beside the machine directory and outside it (one absolute path line naming the machine
/// directory elsewhere). Not this desk's data directory's name, which is a directory beside it.
#[cfg(not(feature = "chain-sepolia"))]
pub const POINTER: &str = ".galeed-desk-pointer";

/// The command line's name, **the program's and its link's alike** (the link the "enable command line" switch makes
/// is named as the program it points at, the family's one name): the trial build's carries `-sepolia`, so a trial and a
/// release installed side by side never take each other's program or link.
#[cfg(not(feature = "chain-sepolia"))]
pub const CLI_LINK: &str = "galeed";
#[cfg(feature = "chain-sepolia")]
pub const CLI_LINK: &str = "galeed-sepolia";

/// This desk's own folder under the place for applications' data where the system keeps applications in folders
/// of their own (Windows); plain names there, no dot files.
#[cfg(not(feature = "chain-sepolia"))]
pub const APP_FOLDER: &str = "galeed-desk";

/// This machine's identifier, outside every data directory (copying a data directory to another machine never
/// carries it).
#[cfg(not(feature = "chain-sepolia"))]
const MACHINE_FILE_DOT: &str = ".galeed-desk-machine";
const MACHINE_FILE_PLAIN: &str = "machine";

/// This desk's own folder under `root` (the place the system gives for applications' data), by `layout`.
pub fn own_folder(root: &Path, layout: crate::Layout) -> PathBuf {
    match layout {
        crate::Layout::Itself => root.to_path_buf(),
        crate::Layout::AppFolder => root.join(APP_FOLDER),
    }
}

/// This machine's identifier file under `root`, by `layout`.
pub fn machine_file_in(root: &Path, layout: crate::Layout) -> PathBuf {
    match layout {
        crate::Layout::Itself => root.join(MACHINE_FILE_DOT),
        crate::Layout::AppFolder => own_folder(root, layout).join(MACHINE_FILE_PLAIN),
    }
}

/// The default machine directory under `root`, by `layout`.
pub fn dir_in(root: &Path, layout: crate::Layout) -> PathBuf {
    own_folder(root, layout).join(DIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each place is a plain function of the root and the layout, the same on every system: the identifier file and the
    /// machine directory (the desk's data directory, one place) never coincide, and the machine directory sits in this
    /// desk's own folder.
    #[test]
    fn the_desks_own_places_are_two_and_distinct_under_either_layout() {
        let root = Path::new("/r");
        for layout in [crate::Layout::Itself, crate::Layout::AppFolder] {
            let (id, machine) = (machine_file_in(root, layout), dir_in(root, layout));
            assert!(id != machine, "{layout:?}");
            assert_eq!(machine.parent(), Some(own_folder(root, layout).as_path()), "{layout:?}");
            assert!(root.join(POINTER) != machine && root.join(POINTER) != id, "the pointer is no other place's name");
        }
        #[cfg(not(feature = "chain-sepolia"))]
        {
            assert_eq!(dir_in(root, crate::Layout::Itself), Path::new("/r/.galeed-desk.d"));
            assert_eq!(dir_in(root, crate::Layout::AppFolder), Path::new("/r/galeed-desk/.galeed-desk.d"));
        }
    }

    /// The family's lookup lands on this desk's default machine directory where this system keeps it, given a home
    /// with no pointer and no older layout.
    #[test]
    fn the_familys_default_is_this_desks_machine_directory() {
        let home = crate::scratch_dir("places");
        assert_eq!(crate::machine::of(&home).expect("default").0, crate::app_data_dir(&home).join(DIR));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A pointer that is there and does not read as one absolute path line (a directory under its name, an empty file)
    /// is refused by name; the lookup never falls through to the default, which would leave the machine directory
    /// behind. A pointer naming an absolute path is taken.
    #[test]
    fn a_pointer_that_does_not_read_is_said_and_never_skipped() {
        let home = crate::scratch_dir("pointer");
        let at = crate::app_data_dir(&home).join(POINTER);
        std::fs::create_dir_all(&at).expect("a directory under the pointer's name");
        assert!(matches!(crate::machine::of(&home), Err(crate::machine::Unread::Pointer(..))), "a directory is not a pointer");
        std::fs::remove_dir(&at).expect("gone");
        std::fs::write(&at, b"").expect("empty");
        assert!(matches!(crate::machine::of(&home), Err(crate::machine::Unread::PointerShape(..))), "an empty pointer");
        let there = home.join("elsewhere");
        std::fs::write(&at, format!("{}\n", there.display())).expect("pointer");
        assert_eq!(crate::machine::of(&home).expect("pointer").0, there);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The Sepolia trial's places are its own: identifier file, machine directory, pointer and folder all carry
    /// `-sepolia`, so the trial never reads or writes what the release (or a development build) left.
    #[cfg(feature = "chain-sepolia")]
    #[test]
    fn the_trials_places_are_its_own() {
        let root = Path::new("/r");
        assert_eq!(machine_file_in(root, crate::Layout::Itself), Path::new("/r/.galeed-desk-sepolia-machine"));
        assert_eq!(dir_in(root, crate::Layout::Itself), Path::new("/r/.galeed-desk-sepolia.d"));
        assert_eq!(dir_in(root, crate::Layout::AppFolder), Path::new("/r/galeed-desk-sepolia/.galeed-desk-sepolia.d"));
        assert_eq!(POINTER, ".galeed-desk-sepolia-pointer");
    }
}

// The Sepolia trial build's names (the cargo feature `chain-sepolia`, forwarded from galeed-cli: the chain row of
// galeed-cli `src/chain.rs`): the same places under names of its own, so the trial installs and keeps its data
// beside the release, never in it.

/// The machine directory's name in this desk's own folder (the Sepolia trial).
#[cfg(feature = "chain-sepolia")]
pub const DIR: &str = ".galeed-desk-sepolia.d";

/// The pointer file's name (the Sepolia trial).
#[cfg(feature = "chain-sepolia")]
pub const POINTER: &str = ".galeed-desk-sepolia-pointer";

/// This desk's own folder where the system keeps applications in folders of their own (the Sepolia trial).
#[cfg(feature = "chain-sepolia")]
pub const APP_FOLDER: &str = "galeed-desk-sepolia";

/// This machine's identifier file (the Sepolia trial).
#[cfg(feature = "chain-sepolia")]
const MACHINE_FILE_DOT: &str = ".galeed-desk-sepolia-machine";
