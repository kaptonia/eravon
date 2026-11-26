use std::path::{Path, PathBuf};

pub const DIR: &str = "checked";
pub const LOGS: &str = "logs.json";

pub fn logs_file(part: &str) -> String {
    format!("logs-{part}.json")
}

pub fn logs_part(file: &str) -> Option<&str> {
    file.strip_prefix("logs-")?.strip_suffix(".json")
}

pub fn audit_file(key: &crate::behind::Key) -> String {
    format!("audit-{}.json", key.hex())
}

pub fn is_logs(file: &str) -> bool {
    file == LOGS || logs_part(file).is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}
pub trait FailSaid {
    fn said(&self) -> String;
    fn locked(&self) -> bool {
        false
    }
}

impl FailSaid for eravon_seal::Trouble {
    fn said(&self) -> String {
        format!("{:?} {}", self.code, self.tail)
    }
    fn locked(&self) -> bool {
        self.code == eravon_seal::Code::Locked
    }
}

pub fn unkept_of<F: FailSaid>(f: &F) -> eravon_chain::settled::Unkept {
    if f.locked() { eravon_chain::settled::Unkept::Locked } else { eravon_chain::settled::Unkept::Said(f.said()) }
}

impl FailSaid for () {
    fn said(&self) -> String {
        String::new()
    }
}

pub trait Room: Send + Sync {
    type Fail: FailSaid;
    fn read(&self, file: &str) -> Option<Vec<u8>>;
    fn put(&self, file: &str, plain: &[u8]) -> Result<(), Self::Fail>;
    fn hold(&self, _file: &str) -> Option<eravon_os::LockFile> {
        None
    }
    fn public(&self) -> bool {
        false
    }
    fn files(&self) -> Vec<String> {
        Vec::new()
    }
    fn remove(&self, file: &str) {
        let _ = file;
    }
}

pub fn files_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir).into_iter().flatten().flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_file())).map(|e| e.file_name().to_string_lossy().into_owned()).collect()
}

pub fn forget_logs_in(dir: &Path) -> std::io::Result<bool> {
    let mut had = false;
    for file in files_in(dir).into_iter().filter(|f| is_logs(f)) {
        match std::fs::remove_file(dir.join(&file)) {
            Ok(()) => had = true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(had)
}

pub fn hold_at(dir: &Path, file: &str) -> Option<eravon_os::LockFile> {
    std::fs::create_dir_all(dir).ok()?;
    let l = eravon_os::LockFile::open(eravon_os::owner_only(eravon_os::Options::new().read(true).write(true).create(true)), &dir.join(format!("{file}.lock"))).ok()?;
    eravon_os::lock_wait(l.file()).then_some(l)
}

pub fn rel(file: &str) -> String {
    format!("{DIR}/{file}")
}

pub struct Sealed(pub PathBuf);

impl Room for Sealed {
    type Fail = eravon_seal::Trouble;

    fn read(&self, file: &str) -> Option<Vec<u8>> {
        let raw = std::fs::read(self.0.join(DIR).join(file)).ok()?;
        let key = eravon_seal::keybox::local_key().ok()?;
        let names = eravon_seal::keybox::name_key().ok();
        let ex = eravon_seal::envelope::Expect { owner: Some(eravon_seal::envelope::Owner::Machine), doc: eravon_seal::envelope::Doc::Checked, rel: Some(rel(file)) };
        eravon_seal::envelope::open_with(&key, &ex, &raw, &rel(file), names.as_ref()).ok()
    }

    fn put(&self, file: &str, plain: &[u8]) -> Result<(), eravon_seal::Trouble> {
        let key = eravon_seal::keybox::local_key()?;
        let id = eravon_seal::envelope::Ident { owner: eravon_seal::envelope::Owner::Machine, doc: eravon_seal::envelope::Doc::Checked, logical: rel(file) };
        let sealed = eravon_seal::envelope::seal_checked(&key, &id, plain)?;
        eravon_seal::land::put_at(&self.0.join(DIR), file, &sealed)
    }

    fn hold(&self, file: &str) -> Option<eravon_os::LockFile> {
        hold_at(&self.0.join(DIR), file)
    }

    fn files(&self) -> Vec<String> {
        files_in(&self.0.join(DIR))
    }

    fn remove(&self, file: &str) {
        let _ = std::fs::remove_file(self.0.join(DIR).join(file));
    }
}

pub const PLAIN_DIR: &str = "cli-read";

pub struct Plain(pub PathBuf);

impl Plain {
    pub fn dir(&self) -> PathBuf {
        self.0.join(PLAIN_DIR)
    }
}

impl Room for Plain {
    type Fail = eravon_seal::Trouble;

    fn read(&self, file: &str) -> Option<Vec<u8>> {
        std::fs::read(self.dir().join(file)).ok()
    }

    fn put(&self, file: &str, plain: &[u8]) -> Result<(), eravon_seal::Trouble> {
        eravon_seal::land::put_at(&self.dir(), file, plain)
    }

    fn hold(&self, file: &str) -> Option<eravon_os::LockFile> {
        hold_at(&self.dir(), file)
    }

    fn public(&self) -> bool {
        true
    }

    fn files(&self) -> Vec<String> {
        files_in(&self.dir())
    }

    fn remove(&self, file: &str) {
        let _ = std::fs::remove_file(self.dir().join(file));
    }
}

impl Sealed {
    pub fn machine(&self) -> &Path {
        &self.0
    }
}

pub struct Logs<R: Room>(pub R);

impl<R: Room> eravon_chain::settled::Keep for Logs<R> {
    fn load(&self, part: &str) -> Option<Vec<u8>> {
        self.0.read(&logs_file(part))
    }

    fn save(&self, part: &str, plain: &[u8]) -> Result<(), eravon_chain::settled::Unkept> {
        self.0.put(&logs_file(part), plain).map_err(|f| unkept_of(&f))?;
        self.0.remove(LOGS);
        Ok(())
    }

    fn parts(&self) -> Vec<String> {
        self.0.files().iter().filter_map(|f| logs_part(f).map(str::to_string)).collect()
    }

    fn hold(&self) -> Option<Box<dyn std::any::Any>> {
        self.0.hold(LOGS).map(|l| Box::new(l) as Box<dyn std::any::Any>)
    }

    fn public(&self) -> bool {
        self.0.public()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_the_room_cannot_write_is_said_with_why() {
        struct ReadOnly;
        impl Room for ReadOnly {
            type Fail = eravon_seal::Trouble;
            fn read(&self, _file: &str) -> Option<Vec<u8>> {
                None
            }
            fn put(&self, file: &str, _plain: &[u8]) -> Result<(), eravon_seal::Trouble> {
                Err(eravon_seal::Trouble::io(&std::io::Error::from(std::io::ErrorKind::PermissionDenied), file))
            }
        }
        let said = eravon_chain::settled::Keep::save(&Logs(ReadOnly), "notes", b"x").expect_err("not written");
        assert!(matches!(&said, eravon_chain::settled::Unkept::Said(w) if !w.is_empty()), "why it was not written: {said:?}");
    }

    #[test]
    fn a_part_not_written_for_the_store_locked_is_told_apart() {
        struct Locked;
        impl Room for Locked {
            type Fail = eravon_seal::Trouble;
            fn read(&self, _file: &str) -> Option<Vec<u8>> {
                None
            }
            fn put(&self, _file: &str, _plain: &[u8]) -> Result<(), eravon_seal::Trouble> {
                Err(eravon_seal::Trouble::new(eravon_seal::Code::Locked, String::new()))
            }
        }
        let got = eravon_chain::settled::Keep::save(&Logs(Locked), "notes", b"x");
        assert_eq!(got, Err(eravon_chain::settled::Unkept::Locked));
    }
}
