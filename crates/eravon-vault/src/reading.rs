use eravon_store::fault::R;
use eravon_store::seam;

use crate::local::Doc;
use crate::records;
use crate::store::{get_file, put_file};

pub use eravon_store::archive::reading_carried as carried;

fn place(data: &str, addr: &str) -> R<std::path::PathBuf> {
    seam::need_address(addr)?;
    records::bind(data).map_err(records::store_fault)?;
    records::single_path(addr, Doc::Reading).map_err(records::store_fault)
}

pub fn put(data: &str, addr: &str, plain: &[u8]) -> R<()> {
    let at = place(data, addr)?;
    if at.exists() && get_file(&at, Doc::Reading).is_err() {
        eravon_store::atomic::remove_file(&at.display().to_string())?;
    }
    put_file(&at, Doc::Reading, plain)
}

pub fn get(data: &str, addr: &str) -> Option<Vec<u8>> {
    get_file(&place(data, addr).ok()?, Doc::Reading).ok()
}

pub fn exists(data: &str, addr: &str) -> bool {
    place(data, addr).is_ok_and(|p| p.exists())
}

pub fn remove(data: &str, addr: &str) -> R<bool> {
    eravon_store::atomic::remove_file(&place(data, addr)?.display().to_string())
}
