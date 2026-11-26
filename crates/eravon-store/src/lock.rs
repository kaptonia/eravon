use crate::fault::Fault;
use crate::writer::{Home, Slot};
use std::path::PathBuf;
use eravon_seal::writer::{self, Took};

pub const LOCK_FILE: &str = eravon_seal::envelope::WRITER_LOCK;

pub const MARK_FILE: &str = writer::MARK_FILE;

pub const MACHINE_FILE: &str = writer::MACHINE_FILE;

pub fn files_under(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    crate::walk::walk(dir, "", &mut found);
    found.into_iter().filter(|(at, _)| at.file_name().is_some_and(|n| n == LOCK_FILE)).map(|(at, _)| at).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Writer,
    Reader,
    OtherMachine,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Writer => "writer",
            Mode::Reader => "reader",
            Mode::OtherMachine => "other-machine",
        }
    }

    pub fn writable(self) -> bool {
        matches!(self, Mode::Writer)
    }
}

pub struct Lock {
    _file: Option<eravon_os::LockFile>,
    mode: Mode,
    path: PathBuf,
    holder: String,
    trouble: Option<Fault>,
}

impl Lock {
    fn made(file: Option<eravon_os::LockFile>, mode: Mode, path: PathBuf, holder: String, trouble: Option<Fault>) -> Lock {
        Lock { _file: file, mode, path, holder, trouble }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn holder(&self) -> &str {
        &self.holder
    }

    pub fn mark_trouble(&self) -> Option<&Fault> {
        self.trouble.as_ref()
    }

    pub fn take_over(&mut self, home: &Home) -> Result<(), Fault> {
        if self.mode != Mode::OtherMachine {
            return Ok(());
        }
        write_mark(home)?;
        self.mode = Mode::Writer;
        self.holder = format!("pid {}", std::process::id());
        self.trouble = None;
        Ok(())
    }
}

pub fn this_machine() -> Result<String, Fault> {
    Ok(writer::this_machine(&crate::writer::machine_dir()?)?)
}

pub fn mark_path(home: &Home) -> PathBuf {
    writer::mark_path(home.root())
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    None,
    Machine(String),
    Unread(Fault),
}

pub fn mark_read(home: &Home) -> Mark {
    match writer::mark_read(home.root()) {
        writer::Mark::None => Mark::None,
        writer::Mark::Machine(m) => Mark::Machine(m),
        writer::Mark::Unread(t) => Mark::Unread(t.into()),
    }
}

pub fn mark_of(home: &Home) -> Option<String> {
    match mark_read(home) {
        Mark::Machine(m) => Some(m),
        _ => None,
    }
}

pub fn write_mark(home: &Home) -> Result<(), Fault> {
    let me = this_machine()?;
    crate::writer::put_at(&home.dir(Slot::Settings), MARK_FILE, format!("{me}\n").as_bytes())
}

pub fn grab(file: &std::fs::File) -> bool {
    crate::writer::lock_now(file)
}

pub fn grab_waiting(file: &std::fs::File) -> bool {
    crate::writer::lock_wait(file)
}

pub fn path_of(home: &Home) -> PathBuf {
    writer::lock_path(home.root())
}

pub fn holds_writer_at(held: Option<&std::path::Path>, home: &Home) -> bool {
    held.map(|p| crate::writer::same_place(p, &path_of(home))).unwrap_or(false)
}

pub fn writer_path(held: Option<&Lock>) -> Option<&std::path::Path> {
    held.filter(|l| l.mode().writable()).map(|l| l.path())
}

pub fn take(home: &Home) -> Result<Lock, Fault> {
    let path = path_of(home);
    let t = writer::take(home.root(), &crate::writer::machine_dir()?)?;
    Ok(match t.took {
        Took::Writer => Lock::made(Some(t.held), Mode::Writer, path, format!("pid {}", std::process::id()), None),
        Took::OtherMachine => {
            let holder = crate::writer::other_machine_said(&mark_path(home).display().to_string());
            Lock::made(Some(t.held), Mode::OtherMachine, path, holder, None)
        }
        Took::Unread(tr) => {
            let f = Fault::from(tr);
            Lock::made(Some(t.held), Mode::OtherMachine, path, f.evidence(), Some(f))
        }
        Took::Busy { holder } => {
            let holder = if holder.trim().is_empty() { crate::writer::NO_HOLDER.to_string() } else { holder };
            Lock::made(Some(t.held), Mode::Reader, path, holder, None)
        }
    })
}
