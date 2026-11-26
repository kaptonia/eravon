use crate::onefile::{self, Caps, Shape};
use std::io::Read;
use std::path::Path;

pub use crate::onefile::Bad;

pub const MAGIC: &str = "zikaron-kit-file/1\n";

pub const SHAPE: Shape = Shape { magic: MAGIC, first: Some(crate::names::MANIFEST), name_ok: crate::kitpath::is_kit_path };

pub fn is_container(head: &[u8]) -> bool {
    head.starts_with(MAGIC.as_bytes())
}

pub fn is_container_at(path: &Path) -> bool {
    let mut head = [0u8; 32];
    let Ok(mut f) = std::fs::File::open(path) else { return false };
    let n = f.read(&mut head).unwrap_or(0);
    is_container(&head[..n])
}

pub fn unpack_to(file: &Path, dir: &Path) -> Result<usize, Bad> {
    let f = std::fs::File::open(file).map_err(|e| Bad::Io(format!("open {}: {e}", file.display())))?;
    if dir.exists() {
        return Err(Bad::Io(format!("create directory {}: already there", dir.display())));
    }
    let land = |t: crate::landing::Trouble| Bad::Io(t.said().unwrap_or(t.subject()).to_string());
    crate::landing::mkdir(dir).map_err(land)?;
    let read = onefile::each(&mut std::io::BufReader::new(f), SHAPE, Caps::default(), |name, _, body| {
        let at = dir.join(name);
        if let Some(p) = at.parent() {
            crate::landing::mkdir(p).map_err(land)?;
        }
        crate::landing::put_from(&at, body).map_err(land)
    });
    if read.is_err() {
        let _ = std::fs::remove_dir_all(dir);
    }
    read
}
