use eravon_store::atomic;

use crate::fault::{Absent, R};
use crate::index::{self, Stand};
use crate::seam;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Manual,
    Pointer,
    Legacy,
    Default,
}

impl Layer {
    pub fn name(self) -> &'static str {
        match self {
            Layer::Manual => seam::L_MANUAL,
            Layer::Pointer => seam::L_POINTER,
            Layer::Legacy => seam::L_LEGACY,
            Layer::Default => seam::L_DEFAULT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub at: String,
    pub layer: Layer,
}

fn manual_layer(manual: Option<&str>) -> Option<&str> {
    manual.map(str::trim).filter(|s| !s.is_empty())
}


const POINTER_FALLS_BACK: bool = false;


fn stands(machine: &str) -> R<Stand> {
    index::stands(machine)
}

pub fn read_pointer(bytes: &[u8], folder: &std::path::Path) -> Option<String> {
    let s = std::str::from_utf8(bytes).ok()?;
    let line = s.strip_suffix('\n')?;
    if line.is_empty() || line.contains('\n') || line.contains('\r') {
        return None;
    }
    let p = std::path::Path::new(line);
    if p.is_absolute() {
        return Some(line.to_string());
    }
    let plain = p.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    plain.then(|| folder.join(p).to_string_lossy().into_owned())
}

fn pointer_target(home: &str) -> R<Option<(String, Result<String, String>)>> {
    let p = seam::pointer_path(home);
    let bytes = match atomic::read_file(&p) {
        Ok(b) => b,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(None),
        Err(e) => return Err(index::unreadable(&p, &e)),
    };
    let folder = std::path::Path::new(&p).parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    Ok(Some((p, read_pointer(&bytes, &folder).ok_or_else(|| String::from_utf8_lossy(&bytes).trim().chars().take(200).collect()))))
}

pub fn discover(home: &str, manual: Option<&str>) -> R<Link> {
    let mut looked: Vec<(String, String)> = Vec::new();

    if let Some(m) = manual_layer(manual) {
        match stands(m)? {
            Stand::Yes => return Ok(Link { at: m.to_string(), layer: Layer::Manual }),
            Stand::No(why) => looked.push((m.to_string(), why)),
        }
    }

    if let Some((pointer, read)) = pointer_target(home)? {
        let (at, stand) = match read {
            Ok(at) => {
                let s = stands(&at)?;
                (at, s)
            }
            Err(held) => (held, Stand::No("指针档该恰是一行路径(绝对路径,或只含普通名的相对路径)加恰一个换行".to_string())),
        };
        match stand {
            Stand::Yes => return Ok(Link { at, layer: Layer::Pointer }),
            Stand::No(why) => {
                if !POINTER_FALLS_BACK {
                    return Err(Absent::PointerStale { pointer, at, why }.into());
                }
                looked.push((at, why));
            }
        }
    }

    let legacy = seam::legacy_dir(home);
    if std::path::Path::new(&legacy).is_dir() {
        match stands(&legacy)? {
            Stand::Yes => return Ok(Link { at: legacy, layer: Layer::Legacy }),
            Stand::No(why) => looked.push((legacy, why)),
        }
    }

    let d = seam::default_dir(home);
    match stands(&d)? {
        Stand::Yes => Ok(Link { at: d, layer: Layer::Default }),
        Stand::No(why) => {
            looked.push((d, why));
            Err(Absent::NotConnected { looked }.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> String {
        let d = std::env::temp_dir().join(format!("eravon-zikaron-discover-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch");
        d.to_string_lossy().into_owned()
    }

    fn standing(at: &str) {
        let p = seam::index_path(at);
        std::fs::create_dir_all(std::path::Path::new(&p).parent().expect("kits")).expect("kits dir");
        std::fs::write(&p, format!("{{\"{}\":\"{}\",\"{}\":[]}}", seam::F_FORM, seam::form(), seam::F_KITS)).expect("index");
    }

    #[test]
    fn the_older_directory_sits_between_the_pointer_and_the_default() {
        let home = scratch("legacy");
        standing(&seam::default_dir(&home));
        assert_eq!(discover(&home, None).expect("default").layer, Layer::Default);
        let legacy = seam::legacy_dir(&home);
        assert!(legacy.ends_with(&["Library", "Application Support", "ZIKARON"].join(std::path::MAIN_SEPARATOR_STR)));
        std::fs::create_dir_all(&legacy).expect("legacy, not standing");
        assert_eq!(discover(&home, None).expect("default still").layer, Layer::Default);
        standing(&legacy);
        assert_eq!(discover(&home, None).expect("legacy"), Link { at: legacy.clone(), layer: Layer::Legacy });
        let there = format!("{home}/elsewhere");
        standing(&there);
        std::fs::write(seam::pointer_path(&home), format!("{there}\n")).expect("pointer");
        assert_eq!(discover(&home, None).expect("pointer"), Link { at: there, layer: Layer::Pointer });
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_pointer_of_any_other_shape_is_stale_never_a_fallback() {
        let home = scratch("shape");
        standing(&seam::default_dir(&home));
        standing(&seam::legacy_dir(&home));
        let there = format!("{home}/elsewhere");
        standing(&there);
        for bad in [String::new(), "relative\n".into(), there.clone(), format!("{there}\n\n"), format!("{there}\r\n"), format!(" {there}\n"), format!("{there}\n{there}\n")] {
            std::fs::write(seam::pointer_path(&home), &bad).expect("pointer");
            let e = discover(&home, None).expect_err(&format!("{bad:?}"));
            let named = if cfg!(windows) && bad.starts_with(' ') { "E_ZK_IO" } else { "E_ZK_POINTER_STALE" };
            assert_eq!(e.name(), named, "{bad:?}");
        }
        std::fs::write(seam::pointer_path(&home), b"\xff\n").expect("pointer");
        assert_eq!(discover(&home, None).expect_err("not utf-8").name(), "E_ZK_POINTER_STALE");
        let _ = std::fs::remove_dir_all(&home);
    }
}
