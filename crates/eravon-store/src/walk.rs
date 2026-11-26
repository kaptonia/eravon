use std::path::{Path, PathBuf};

pub fn walk(root: &Path, rel: &str, out: &mut Vec<(PathBuf, String)>) {
    let at = if rel.is_empty() { root.to_path_buf() } else { root.join(rel) };
    let Ok(listing) = std::fs::read_dir(&at) else { return };
    for e in listing.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        let r = if rel.is_empty() { name } else { format!("{rel}/{name}") };
        match std::fs::symlink_metadata(e.path()) {
            Ok(m) if m.is_dir() => walk(root, &r, out),
            Ok(m) if m.is_file() => out.push((e.path(), r)),
            _ => {}
        }
    }
}
