use eravon_literals::json::{In, Out};

use crate::atomic;
use crate::doc;
use crate::fault::{Fault, R};
use crate::seam;

pub(crate) fn hex(b: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(D[(x >> 4) as usize] as char);
        s.push(D[(x & 15) as usize] as char);
    }
    s
}

pub(crate) fn unhex(s: &str) -> R<Vec<u8>> {
    let b = s.as_bytes();
    if b.len() % 2 != 0 {
        return Err(Fault::Archive("字节那一栏的十六进制是奇数位".into()));
    }
    let v = |c: u8| -> R<u8> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err(Fault::Archive(format!("字节那一栏有不是小写十六进制的字符:{c:?}"))),
        }
    };
    let mut out = Vec::with_capacity(b.len() / 2);
    for p in b.chunks(2) {
        out.push((v(p[0])? << 4) | v(p[1])?);
    }
    Ok(out)
}

fn collect(root: &str, rel: &str, out: &mut Walked) -> R<()> {
    walk(root, rel, out, false)
}


#[derive(Default)]
struct Walked {
    dirs: Vec<String>,
    files: Vec<(String, Vec<u8>)>,
}

fn put_tmp_shaped(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('.').and_then(|r| r.strip_suffix(".tmp")) else { return false };
    let mut parts = rest.rsplitn(3, '.');
    let digits = |x: Option<&str>| x.is_some_and(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit()));
    let (nanos, pid, base) = (parts.next(), parts.next(), parts.next());
    digits(nanos) && digits(pid) && base.is_some_and(|b| !b.is_empty())
}

fn walk(root: &str, rel: &str, out: &mut Walked, skip_parts: bool) -> R<()> {
    let dir = if rel.is_empty() { root.to_string() } else { format!("{root}/{rel}") };
    for name in atomic::entries(&dir)? {
        if (rel.is_empty() && name == seam::F_MIRROR) || name == seam::F_LOCK || name == seam::F_WRITER_MARK || name.ends_with(".lock") {
            continue;
        }
        if put_tmp_shaped(&name) {
            continue;
        }
        if !reading_carried() && name == seam::F_READING && rel.rsplit('/').next() == Some(seam::D_STORE) {
            continue;
        }
        if skip_parts && (name == seam::D_PARTS || (!name.starts_with("record.") && rel.rsplit('/').next().is_some_and(|d| d.len() == 64 && d.bytes().all(|c| c.is_ascii_hexdigit())))) {
            continue;
        }
        let child_rel = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        let path = format!("{dir}/{name}");
        let m = std::fs::metadata(&path)
            .map_err(|e| Fault::Io(format!("看 {path}:{e}")))?;
        if m.is_dir() {
            out.dirs.push(child_rel.clone());
            walk(root, &child_rel, out, skip_parts)?;
        } else if m.is_file() {
            out.files.push((child_rel, atomic::read_file(&path)?));
        }
    }
    Ok(())
}

pub fn export(data: &str, writer: &str, now: u64) -> R<Vec<u8>> {
    doc::need_time(now)?;
    let mut w = Walked::default();
    collect(data, "", &mut w)?;
    let Walked { mut dirs, mut files } = w;
    dirs.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Ok(doc::seal(
        seam::V_ARCHIVE,
        vec![
            ("createdAt", Out::N(now)),
            ("dirs", Out::A(dirs.iter().map(|d| Out::S(d.clone())).collect())),
            (
                "files",
                Out::A(
                    files
                        .iter()
                        .map(|(p, b)| {
                            Out::O(vec![
                                ("bytes", Out::S(hex(b))),
                                ("path", Out::S(p.clone())),
                                ("size", Out::N(b.len() as u64)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("writer", Out::S(writer.to_string())),
        ],
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub fn read_bundle(bytes: &[u8]) -> R<(Vec<String>, Vec<Entry>, u64, String)> {
    let v: In = doc::open(seam::V_ARCHIVE, bytes)?;
    let mut dirs = Vec::new();
    for d in doc::need_arr(&v, "dirs")? {
        let p = match d {
            In::Str(s) => s.clone(),
            other => return Err(Fault::Archive(format!("dirs 的每一项要是字符串:{other:?}"))),
        };
        check_rel(&p)?;
        dirs.push(p);
    }
    let mut out = Vec::new();
    for f in doc::need_arr(&v, "files")? {
        let path = doc::need_str(f, "path")?;
        check_rel(&path)?;
        let b = unhex(&doc::need_str(f, "bytes")?)?;
        let size = doc::need_u64(f, "size")?;
        if size != b.len() as u64 {
            return Err(Fault::Archive(format!(
                "{path} 的 size 是 {size},字节却有 {} 枚",
                b.len()
            )));
        }
        out.push(Entry { path, bytes: b });
    }
    Ok((dirs, out, doc::need_u64(&v, "createdAt")?, doc::need_str(&v, "writer")?))
}

fn check_rel(p: &str) -> R<()> {
    let rooted = std::path::Path::new(p).components().any(|c| matches!(c, std::path::Component::RootDir | std::path::Component::Prefix(_)));
    if p.is_empty() || rooted {
        return Err(Fault::Archive(format!("束里的路径要是相对的:{p:?}")));
    }
    for seg in p.split('/') {
        if !seam::name_ok(seg) {
            return Err(Fault::Archive(format!("束里的路径有不合形的一段:{seg:?}")));
        }
    }
    Ok(())
}

pub fn import(bundle: &[u8], target: &str, from: &str) -> R<usize> {
    let (dirs, files, cut_at, _writer) = read_bundle(bundle)?;
    match std::fs::read_dir(target) {
        Ok(mut rd) => {
            if rd.next().is_some() {
                return Err(Fault::Exists(format!("目标目录 {target} 不空")));
            }
        }
        Err(e) if crate::atomic::absent(&e) => {}
        Err(e) => return Err(Fault::Io(format!("看 {target}:{e}"))),
    }
    atomic::mkdirs(target)?;
    for d in &dirs {
        atomic::mkdirs(&format!("{target}/{d}"))?;
    }
    for f in &files {
        atomic::write_atomic(&format!("{target}/{}", f.path), &f.bytes)?;
    }
    crate::mirror::mark(target, cut_at, &crate::mirror::from_of(from))?;
    Ok(files.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_goes_there_and_back_byte_for_byte() {
        let b: Vec<u8> = (0u8..=255).collect();
        assert_eq!(unhex(&hex(&b)).unwrap(), b);
    }

    #[test]
    fn a_path_that_walks_out_of_the_data_root_is_refused() {
        assert!(check_rel("a/b").is_ok());
        assert!(check_rel("/a").is_err());
        assert!(check_rel("../a").is_err());
        assert!(check_rel("a/../b").is_err());
        assert!(check_rel("").is_err());
    }
}

pub fn reading_carried() -> bool {
    false
}


#[cfg(test)]
mod put_tmp {
    #[test]
    fn a_family_temporary_name_is_told_by_its_shape() {
        assert!(super::put_tmp_shaped(".identity.json.4242.1700000000000000000.tmp"));
        assert!(!super::put_tmp_shaped("identity.json"));
        assert!(!super::put_tmp_shaped(".mirror"));
        assert!(!super::put_tmp_shaped(".a.b.tmp"));
    }
}
