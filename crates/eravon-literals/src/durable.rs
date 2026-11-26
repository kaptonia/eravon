use std::io::Write;
use std::path::{Path, PathBuf};

pub fn temp_name(_hint: &str) -> std::io::Result<String> {
    let mut nonce = String::with_capacity(eravon_glue::layout::TMP_NONCE_LEN);
    for b in eravon_os::random(eravon_glue::layout::TMP_NONCE_LEN / 2)? {
        nonce.push_str(&format!("{b:02x}"));
    }
    Ok(eravon_glue::layout::tmp_file_name(&nonce))
}


pub fn is_temp(name: &str) -> bool {
    eravon_glue::layout::tmp_shaped(name)
}

pub fn dir_of(target: &Path) -> &Path {
    target.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."))
}

pub fn temp_beside(target: &Path, hint: &str) -> std::io::Result<PathBuf> {
    Ok(dir_of(target).join(temp_name(hint)?))
}

fn named(what: &str, at: &Path, e: std::io::Error) -> std::io::Error {
    std::io::Error::new(e.kind(), format!("{what} {}: {e}", at.display()))
}

fn staged(target: &Path, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let dir = dir_of(target);
    std::fs::create_dir_all(dir).map_err(|e| named("create directory", dir, e))?;
    let tmp = temp_beside(target, &target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())?;
    let r = (|| -> std::io::Result<()> {
        let mut o = eravon_os::Options::new();
        o.write(true).create_new(true);
        let mut f = eravon_os::owner_only(&mut o).open(&tmp)?;
        f.write_all(bytes).map_err(|e| named("write", &tmp, e))?;
        f.sync_all().map_err(|e| named("file sync", &tmp, e))
    })();
    match r {
        Ok(()) => Ok(tmp),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

pub fn put(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = staged(target, bytes)?;
    if let Err(e) = eravon_os::replace(&tmp, target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    eravon_os::sync_dir(dir_of(target), target)
}

pub fn rename_over(from: &Path, to: &Path) -> std::io::Result<()> {
    eravon_os::replace(from, to)?;
    eravon_os::sync_dir(dir_of(to), to)?;
    if dir_of(from) != dir_of(to) {
        eravon_os::sync_dir(dir_of(from), dir_of(from))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eravon-durable-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch");
        d
    }

    #[test]
    fn a_write_is_done_only_once_its_directory_is_synced() {
        let d = scratch("put");
        let t = d.join("a.json");
        put(&t, b"one").expect("written");
        put(&t, b"two").expect("written over");
        assert_eq!(std::fs::read(&t).expect("read"), b"two");
        assert!(eravon_os::is_owner_only(&t).expect("mode"));
        let left: Vec<_> = std::fs::read_dir(&d).expect("dir").map(|e| e.expect("e").file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left, ["a.json"], "no temporary name left");
        eravon_os::pretend_sync_fails_at(Some(d.clone()));
        let e = put(&t, b"three").err().expect("the sync failing is an error");
        eravon_os::pretend_sync_fails_at(None);
        assert!(e.to_string().starts_with(&format!("directory sync {}", d.display())), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn temporary_names_are_told_by_name() {
        let t = temp_name("p1003").expect("entropy");
        assert!(is_temp(&t) && !t.contains("p1003"));
        assert_eq!(t.len(), eravon_glue::layout::TMP_PREFIX.len() + eravon_glue::layout::TMP_NONCE_LEN);
        for entry in ["record.json", "settings.json", "a.DS_Store", "_x"] {
            assert!(!is_temp(entry), "{entry}");
        }
    }
}
