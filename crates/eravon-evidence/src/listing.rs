use std::path::Path;

use eravon_galeed as k1;

use crate::mark;
use crate::sheet::{self, FileItem, Manifest};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ListError {
    NotDirectory,
    Unreadable,
    NameNotUtf8,
    TooBig,
    NameNotPortable(String),
    Unlisted(&'static str),
}

fn junk(rel: &str) -> bool {
    rel.split('/').any(|seg| sheet::JUNK_SEGMENTS.contains(&seg))
}

fn item(name: String, bytes: &[u8], media_type: &str) -> Result<FileItem, ListError> {
    sheet::listed(&name, bytes, media_type).map_err(|u| ListError::Unlisted(u.name()))
}

const CHARSET_HEAD: &str = "名字「";
const CHARSET_TAIL: &str = "」带清单不收的字符(清单里的名只许 ASCII 可见字符),装不进清单:改成只用英文字母、数字与常见符号的名之后再拖入";

pub fn name_packable(name: &str) -> Result<(), String> {
    eravon_literals::seam::name_portable(name)?;
    if k1::canon::is_skeleton(name) { Ok(()) } else { Err(format!("{CHARSET_HEAD}{name}{CHARSET_TAIL}")) }
}

pub fn charset_refused(said: &str) -> Option<&str> {
    let rest = &said[said.find(CHARSET_HEAD)? + CHARSET_HEAD.len()..];
    rest.find(CHARSET_TAIL).map(|j| &rest[..j])
}

pub const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn in_order(out: &mut [(String, Vec<u8>)]) {
    out.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
}

fn regular_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, ListError> {
    let mut out = collect(dir)?;
    in_order(&mut out);
    Ok(out)
}



fn collect(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, ListError> {
    let meta = std::fs::metadata(dir).map_err(|_| ListError::NotDirectory)?;
    if !meta.is_dir() {
        return Err(ListError::NotDirectory);
    }
    let entries = std::fs::read_dir(dir).map_err(|_| ListError::NotDirectory)?;
    let mut out = Vec::new();
    for e in entries {
        let e = e.map_err(|_| ListError::NotDirectory)?;
        let p = e.path();
        let m = match std::fs::metadata(&p) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(ListError::Unreadable),
        };
        if !m.is_file() {
            continue;
        }
        if m.len() > FILE_BOUND {
            return Err(ListError::TooBig);
        }
        let name = e.file_name().into_string().map_err(|_| ListError::NameNotUtf8)?;
        if junk(&name) {
            continue;
        }
        name_packable(&name).map_err(ListError::NameNotPortable)?;
        let bytes = std::fs::read(&p).map_err(|_| ListError::Unreadable)?;
        out.push((name, bytes));
    }
    Ok(out)
}

pub fn from_directory(
    dir: &Path,
    subject_ref: k1::Value,
    party: &str,
    media_type: &str,
    note_md: Option<String>,
) -> Result<Manifest, ListError> {
    mark::mark(mark::FEATURE);
    let files = regular_files(dir)?
        .into_iter()
        .map(|(name, bytes)| item(name, &bytes, media_type))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Manifest { subject_ref, party: party.to_string(), files, note_md })
}

pub fn items_from_tree(
    root: &Path,
    prefix: &str,
    media_type: &str,
) -> Result<Vec<FileItem>, ListError> {
    mark::mark(mark::FEATURE);
    let mut out = Vec::new();
    walk(root, "", &mut out)?;
    in_order(&mut out);
    out.into_iter().map(|(rel, bytes)| item(format!("{prefix}{rel}"), &bytes, media_type)).collect()
}

fn walk(dir: &Path, at: &str, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), ListError> {
    let meta = std::fs::metadata(dir).map_err(|_| ListError::NotDirectory)?;
    if !meta.is_dir() {
        return Err(ListError::NotDirectory);
    }
    let entries = std::fs::read_dir(dir).map_err(|_| ListError::NotDirectory)?;
    for e in entries {
        let e = e.map_err(|_| ListError::NotDirectory)?;
        let p = e.path();
        let m = match std::fs::metadata(&p) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(ListError::Unreadable),
        };
        let name = e.file_name().into_string().map_err(|_| ListError::NameNotUtf8)?;
        let rel = if at.is_empty() { name } else { format!("{at}/{name}") };
        if junk(&rel) {
            continue;
        }
        name_packable(&rel).map_err(ListError::NameNotPortable)?;
        if m.is_dir() {
            walk(&p, &rel, out)?;
            continue;
        }
        if !m.is_file() {
            continue;
        }
        if m.len() > FILE_BOUND {
            return Err(ListError::TooBig);
        }
        out.push((rel, std::fs::read(&p).map_err(|_| ListError::Unreadable)?));
    }
    Ok(())
}

pub fn from_items(
    subject_ref: k1::Value,
    party: &str,
    files: Vec<FileItem>,
    note_md: Option<String>,
) -> Manifest {
    mark::mark(mark::FEATURE);
    Manifest { subject_ref, party: party.to_string(), files, note_md }
}

pub fn claim_parameter(m: &Manifest) -> Result<[u8; 32], eravon_galeed::Token> {
    mark::mark(mark::FEATURE);
    let bytes = sheet::build(m)?;
    Ok(sheet::file_view(&bytes).1)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_name_saving_would_refuse_is_refused_when_listing() {
        let d = std::env::temp_dir().join(format!("eravon-listing-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("report 10:04.pdf"), b"x").unwrap();
        assert!(matches!(from_directory(&d, k1::Value::Null, "", "x", None), Err(ListError::NameNotPortable(_))));
        assert!(matches!(items_from_tree(&d, "", "x"), Err(ListError::NameNotPortable(_))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_name_outside_the_charset_is_refused_by_its_name() {
        let d = std::env::temp_dir().join(format!("eravon-listing-charset-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("chat")).unwrap();
        std::fs::write(d.join("chat/聊天记录.txt"), b"x").unwrap();
        let Err(ListError::NameNotPortable(w)) = items_from_tree(&d, "", "x") else { panic!("中文名该拒") };
        assert_eq!(charset_refused(&w), Some("chat/聊天记录.txt"), "{w}");
        assert_eq!(charset_refused(&format!("无法读取文件:{w}")), Some("chat/聊天记录.txt"), "包在别的句子里也读得回");
        assert_eq!(name_packable("聊天记录.txt").err().as_deref().and_then(charset_refused), Some("聊天记录.txt"));
        assert!(name_packable("chat/day1.txt").is_ok());
        assert_eq!(charset_refused("名字 \"a:b\" 带冒号"), None);
        std::fs::remove_file(d.join("chat/聊天记录.txt")).unwrap();
        std::fs::write(d.join("chat/day1.txt"), b"x").unwrap();
        assert_eq!(items_from_tree(&d, "", "x").map(|v| v.len()), Ok(1));
        let _ = std::fs::remove_dir_all(&d);
    }
}
