pub use eravon_store::lock::{Lock, Mark, Mode, LOCK_FILE, MARK_FILE};

use crate::fault::Fault;

fn at(home: &crate::home::Home) -> eravon_store::writer::Home {
    eravon_store::writer::Home::of(&home.dir(crate::home::Slot::Settings).display().to_string())
}

pub fn take(home: &crate::home::Home) -> Result<Lock, Fault> {
    eravon_store::lock::take(&at(home)).map_err(Fault::from)
}

pub fn holds_writer_at(held: Option<&std::path::Path>, home: &crate::home::Home) -> bool {
    eravon_store::lock::holds_writer_at(held, &at(home))
}

pub fn writer_path(held: Option<&Lock>) -> Option<&std::path::Path> {
    eravon_store::lock::writer_path(held)
}

pub fn take_over(lock: &mut Lock, home: &crate::home::Home) -> Result<(), Fault> {
    eravon_store::writer::take_over(&crate::house::data()?).map_err(Fault::from)?;
    lock.take_over(&at(home)).map_err(Fault::from)
}

pub fn mark_trouble(lock: &Lock) -> Option<Fault> {
    lock.mark_trouble().cloned().map(Fault::from)
}

pub fn this_machine() -> Result<String, Fault> {
    eravon_store::lock::this_machine().map_err(Fault::from)
}

pub fn mark_path(home: &crate::home::Home) -> std::path::PathBuf {
    eravon_store::lock::mark_path(&at(home))
}

pub fn path_of(home: &crate::home::Home) -> std::path::PathBuf {
    eravon_store::lock::path_of(&at(home))
}
