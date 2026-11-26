use crate::fault::{Fault, R};

pub fn is_temp(name: &str) -> bool {
    crate::layout::tmp_shaped(name)
}

pub fn is_system_side(name: &str) -> bool {
    let (view, sidecar) = crate::layout::SYSTEM_SIDE;
    name == view || name.starts_with(sidecar)
}

fn io<T>(what: &str, at: &str, r: std::io::Result<T>) -> R<T> {
    r.map_err(|e| Fault::Io(format!("{what} {at}: {e}")))
}

fn named<T>(r: std::io::Result<T>) -> R<T> {
    r.map_err(|e| Fault::Io(e.to_string()))
}

static ERA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

std::thread_local! {
    static TICKET: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

pub fn era() -> u64 {
    ERA.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn bump_era() {
    ERA.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

pub fn hold_ticket(at: Option<u64>) {
    TICKET.with(|t| t.set(at));
}

pub fn ticket_void() -> bool {
    TICKET.with(|t| t.get()).is_some_and(|at| at != era())
}

fn gates(path: &str) -> R<()> {
    crate::mirror::gate(path)?;
    if ticket_void() {
        return Err(Fault::Stale(format!("这一趟开始时的场合已经换了,{path} 不写")));
    }
    Ok(())
}

pub fn mkdirs(dir: &str) -> R<()> {
    gates(dir)?;
    io("create directory", dir, std::fs::create_dir_all(dir))
}

pub fn write_atomic(path: &str, bytes: &[u8]) -> R<()> {
    gates(path)?;
    write_raw(path, bytes)
}

pub(crate) fn write_raw(path: &str, bytes: &[u8]) -> R<()> {
    named(eravon_literals::durable::put(std::path::Path::new(path), bytes))
}

pub fn land_new_only(path: &str, bytes: &[u8]) -> R<bool> {
    gates(path)?;
    match crate::landing::land_bytes_for(crate::landing::Readers::Owner, std::path::Path::new(path), bytes) {
        Ok(()) => Ok(true),
        Err(crate::landing::Trouble::Occupied(_)) => Ok(false),
        Err(crate::landing::Trouble::Io(_, said)) => Err(Fault::Io(said)),
    }
}

pub fn open_lock(path: &str) -> R<eravon_os::LockFile> {
    gates(path)?;
    let mut o = eravon_os::Options::new();
    o.create(true).read(true).write(true);
    named(eravon_os::LockFile::open(eravon_os::owner_only(&mut o), std::path::Path::new(path)))
}

pub fn open_lock_existing(path: &str) -> R<Option<eravon_os::LockFile>> {
    match eravon_os::LockFile::open(eravon_os::Options::new().read(true), std::path::Path::new(path)) {
        Ok(f) => Ok(Some(f)),
        Err(e) if absent(&e) => Ok(None),
        Err(e) => Err(Fault::Io(e.to_string())),
    }
}

pub fn say_on(f: &std::fs::File, bytes: &[u8]) -> R<()> {
    use std::io::{Seek, Write};
    let mut h = f;
    io("write", "the lock file", h.set_len(0).and_then(|()| h.rewind()).and_then(|()| h.write_all(bytes)).and_then(|()| h.flush()))
}

pub fn remove_file(path: &str) -> R<bool> {
    gates(path)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if absent(&e) => Ok(false),
        Err(e) => Err(Fault::Io(format!("remove {path}: {e}"))),
    }
}

pub fn retire_dir(dir: &str, hint: &str) -> R<()> {
    let tomb = named(eravon_literals::durable::temp_beside(std::path::Path::new(dir), hint))?.to_string_lossy().into_owned();
    gates(dir)?;
    gates(&tomb)?;
    named(eravon_literals::durable::rename_over(std::path::Path::new(dir), std::path::Path::new(&tomb)))?;
    io("remove directory", &tomb, std::fs::remove_dir_all(&tomb))
}

pub fn rename(from: &str, to: &str) -> R<()> {
    gates(from)?;
    gates(to)?;
    named(eravon_literals::durable::rename_over(std::path::Path::new(from), std::path::Path::new(to)))
}

pub fn remove_dir_all(dir: &str) -> R<()> {
    gates(dir)?;
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if absent(&e) => Ok(()),
        Err(e) => Err(Fault::Io(format!("remove directory {dir}: {e}"))),
    }
}


pub fn sweep(dir: &str) -> R<usize> {
    let rd = match std::fs::read_dir(dir) {
        Ok(x) => x,
        Err(e) if absent(&e) => return Ok(0),
        Err(e) => return Err(Fault::Io(format!("read directory {dir}: {e}"))),
    };
    let mut n = 0;
    for e in rd {
        let e = io("read directory", dir, e)?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !is_temp(&name) {
            continue;
        }
        let p = e.path().to_string_lossy().into_owned();
        gates(&p)?;
        let kind = io("look at", &p, e.file_type())?;
        if kind.is_dir() {
            io("remove directory", &p, std::fs::remove_dir_all(&p))?;
        } else if kind.is_file() {
            io("remove", &p, std::fs::remove_file(&p))?;
        } else {
            continue;
        }
        n += 1;
    }
    Ok(n)
}

pub fn read_file(path: &str) -> R<Vec<u8>> {
    match std::fs::read(path) {
        Ok(b) => Ok(b),
        Err(e) if absent(&e) => Err(Fault::Missing(format!("盘上没有 {path}"))),
        Err(e) => Err(Fault::Io(format!("read {path}: {e}"))),
    }
}

pub fn absent(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory)
}

pub fn exists(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}

pub fn owner_only_of(path: &str) -> R<bool> {
    named(eravon_os::is_owner_only(std::path::Path::new(path)))
}

pub fn entries(dir: &str) -> R<Vec<String>> {
    strict_entries(dir, |_| true)
}

fn loose() -> bool {
    false
}


pub fn strict_entries(dir: &str, known: impl Fn(&str) -> bool) -> R<Vec<String>> {
    let rd = match std::fs::read_dir(dir) {
        Ok(x) => x,
        Err(e) if absent(&e) => return Ok(Vec::new()),
        Err(e) => return Err(Fault::Io(format!("read directory {dir}: {e}"))),
    };
    let mut out = Vec::new();
    for e in rd {
        let e = io("read directory", dir, e)?;
        let name = e.file_name().to_string_lossy().to_string();
        if is_temp(&name) || is_system_side(&name) {
            continue;
        }
        if !known(&name) {
            if loose() {
                continue;
            }
            return Err(Fault::Form(format!("{dir} 里有一个认不出的名字:{name:?}")));
        }
        out.push(name);
    }
    out.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(tag: &str) -> String {
        let d = std::env::temp_dir().join(format!("eravon-atomic-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch");
        d.display().to_string()
    }

    #[test]
    fn a_disk_error_names_its_interface_and_path() {
        let d = home("io");
        let e = read_file(&d).err().expect("a directory is no file");
        assert_eq!(e.name(), "E_STORE_IO");
        assert!(e.detail().starts_with(&format!("read {d}: ")), "{}", e.detail());
        let t = format!("{d}/x.json");
        eravon_os::pretend_sync_fails_at(Some(std::path::PathBuf::from(&d)));
        let e = write_atomic(&t, b"{}").err().expect("the sync failing is no write");
        eravon_os::pretend_sync_fails_at(None);
        assert_eq!(e.name(), "E_STORE_IO");
        assert!(e.detail().starts_with(&format!("directory sync {d}: ")), "{}", e.detail());
        write_atomic(&t, b"{}").expect("written again");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_strict_listing_names_a_stranger_and_the_sweep_takes_only_temporaries() {
        let d = home("strict");
        for n in ["a", "b", ".DS_Store", "._a"] {
            std::fs::write(format!("{d}/{n}"), b"x").expect("write");
        }
        let tmp = eravon_literals::durable::temp_name("a").expect("entropy");
        std::fs::write(format!("{d}/{tmp}"), b"half").expect("write");
        std::fs::create_dir_all(format!("{d}/{}", eravon_literals::durable::temp_name("b").expect("entropy"))).expect("dir");
        assert_eq!(strict_entries(&d, |n| n == "a" || n == "b").expect("told"), ["a", "b"]);
        let e = strict_entries(&d, |n| n == "a").err().expect("b is not told");
        assert_eq!(e.name(), "E_STORE_FORM");
        assert!(e.detail().contains("\"b\""), "{}", e.detail());
        assert_eq!(sweep(&d).expect("swept"), 2);
        let mut left: Vec<String> = std::fs::read_dir(&d).expect("dir").map(|e| e.expect("e").file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, [".DS_Store", "._a", "a", "b"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn landing_new_only_takes_no_name_in_use() {
        let d = home("new-only");
        let at = |n: &str| format!("{d}/{n}");
        std::fs::write(at("empty"), b"").expect("an empty file");
        std::fs::write(at("full"), b"theirs").expect("a file");
        std::fs::create_dir_all(at("dir")).expect("a directory");
        #[cfg(unix)]
        std::os::unix::fs::symlink(at("nowhere"), at("link")).expect("a broken link");
        for n in ["empty", "full", "dir"] {
            assert!(!land_new_only(&at(n), b"mine").expect("answered"), "{n}");
        }
        #[cfg(unix)]
        assert!(!land_new_only(&at("link"), b"mine").expect("answered"), "a broken link holds it");
        assert_eq!(std::fs::read(at("empty")).expect("kept"), b"");
        assert_eq!(std::fs::read(at("full")).expect("kept"), b"theirs");
        assert!(land_new_only(&at("free"), b"mine").expect("landed where nothing is"));
        assert_eq!(std::fs::read(at("free")).expect("landed"), b"mine");
        assert!(owner_only_of(&at("free")).expect("mode"), "owner-only");
        assert!(!land_new_only(&at("free"), b"again").expect("answered"), "landed once, never replaced");
        assert_eq!(std::fs::read(at("free")).expect("kept"), b"mine");
        let left: Vec<String> = std::fs::read_dir(&d).expect("dir").map(|e| e.expect("e").file_name().to_string_lossy().into_owned()).filter(|n| is_temp(n)).collect();
        assert!(left.is_empty(), "no temporary sibling left: {left:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn landing_siblings_are_temporary_and_side_files_are_told_by_name() {
        let d = home("sibling");
        let sib = crate::layout::tmp_file_name("0123456789abcdef");
        assert!(is_temp(&sib) && crate::layout::tmp_shaped(&sib));
        std::fs::write(format!("{d}/{sib}"), b"half").expect("write");
        std::fs::write(format!("{d}/a.json"), b"x").expect("write");
        assert_eq!(strict_entries(&d, |n| n == "a.json").expect("told"), ["a.json"]);
        assert_eq!(sweep(&d).expect("swept"), 1);
        assert!(!exists(&format!("{d}/{sib}")));
        assert!(is_system_side(".DS_Store") && is_system_side("._record.json"));
        for entry in ["record.json", "settings.json", "a.DS_Store", "_x"] {
            assert!(!is_temp(entry) && !is_system_side(entry), "{entry}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_void_ticket_writes_nothing() {
        let d = home("ticket");
        let t = format!("{d}/x.json");
        std::thread::spawn(move || {
            hold_ticket(Some(era()));
            write_atomic(&t, b"1").expect("the era holds");
            bump_era();
            let e = write_atomic(&t, b"2").err().expect("void");
            assert_eq!(e.name(), "E_STORE_STALE");
            assert_eq!(std::fs::read(&t).expect("read"), b"1");
            hold_ticket(None);
            write_atomic(&t, b"3").expect("no ticket, no gate");
        })
        .join()
        .expect("thread");
        let _ = std::fs::remove_dir_all(&d);
    }
}
