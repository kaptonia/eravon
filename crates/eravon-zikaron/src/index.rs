use eravon_store::atomic;

use eravon_literals::json::In;

use crate::fault::{Absent, Fault, R};
use crate::seam;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kit {
    pub id: String,
    pub created: u64,
    pub root: String,
    pub contents: Vec<String>,
    pub note_md: String,
    pub path: String,
    pub link: String,
    pub sealed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub form: &'static str,
    pub kits: Vec<Kit>,
}

fn need_form(v: &In) -> R<&'static str> {
    match v.get(seam::F_FORM) {
        Some(In::Str(s)) if s == seam::form() => Ok(seam::form()),
        Some(other) => Err(Fault::IndexForm { want: seam::form(), got: format!("{other:?}") }),
        None => Err(Fault::IndexForm {
            want: seam::form(),
            got: "(档上没有这一栏)".to_string(),
        }),
    }
}


fn need_str(row: &In, key: &str) -> R<String> {
    match row.get(key) {
        Some(In::Str(s)) => Ok(s.clone()),
        other => Err(Fault::IndexShape(format!("索引一行上的 {key} 该是字符串,是 {other:?}"))),
    }
}

fn need_strs(row: &In, key: &str) -> R<Vec<String>> {
    match row.get(key) {
        Some(In::Arr(a)) => a
            .iter()
            .map(|x| match x {
                In::Str(s) => Ok(s.clone()),
                other => Err(Fault::IndexShape(format!("索引一行上的 {key} 该是一张字符串表,里面有 {other:?}"))),
            })
            .collect(),
        other => Err(Fault::IndexShape(format!("索引一行上的 {key} 该是一张字符串表,是 {other:?}"))),
    }
}

fn need_abs(row: &In, key: &str) -> R<String> {
    let p = need_str(row, key)?;
    if std::path::Path::new(&p).is_absolute() {
        Ok(p)
    } else {
        Err(Fault::IndexShape(format!("索引一行上的 {key} 该是包目录的绝对路径,是 {p:?}")))
    }
}

fn need_u64(row: &In, key: &str) -> R<u64> {
    match row.get(key) {
        Some(In::Int(n)) if *n >= 0 => Ok(*n as u64),
        other => Err(Fault::IndexShape(format!("索引一行上的 {key} 该是非负整数,是 {other:?}"))),
    }
}

fn opt_str(row: &In, key: &str) -> R<String> {
    match row.get(key) {
        None | Some(In::Null) => Ok(String::new()),
        Some(In::Str(s)) => Ok(s.clone()),
        other => Err(Fault::IndexShape(format!("索引一行上的 {key} 该是字符串,是 {other:?}"))),
    }
}

pub fn stands(machine: &str) -> R<Stand> {
    let p = seam::index_path(machine);
    let bytes = match atomic::read_file(&p) {
        Ok(b) => b,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(Stand::No(format!("{p} 读不到"))),
        Err(e) => return Err(unreadable(&p, &e)),
    };
    let v = eravon_literals::json::read(&bytes)
        .map_err(|e| Fault::IndexShape(format!("{p} 不是一份读得动的 JSON:{e}")))?;
    need_form(&v).map(|_| Stand::Yes)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stand {
    Yes,
    No(String),
}

pub fn load(machine: &str) -> R<Index> {
    let p = seam::index_path(machine);
    let bytes = match atomic::read_file(&p) {
        Ok(b) => b,
        Err(eravon_store::fault::Fault::Missing(_)) => {
            return Err(Absent::NoKits { at: machine.to_string(), why: format!("{p} 读不到") }.into());
        }
        Err(e) => return Err(unreadable(&p, &e)),
    };
    let v = eravon_literals::json::read(&bytes)
        .map_err(|e| Fault::IndexShape(format!("{p} 不是一份读得动的 JSON:{e}")))?;
    let form = need_form(&v)?;
    let rows = match v.get(seam::F_KITS) {
        Some(In::Arr(a)) => a,
        other => {
            return Err(Fault::IndexShape(format!("{p} 的 {} 该是一张表,是 {other:?}", seam::F_KITS)));
        }
    };
    if rows.is_empty() {
        return Err(Absent::NoKits {
            at: machine.to_string(),
            why: format!("{p} 的 {} 是空表", seam::F_KITS),
        }
        .into());
    }
    let mut kits = Vec::with_capacity(rows.len());
    for row in rows {
        kits.push(Kit {
            id: need_str(row, seam::F_ID)?,
            created: need_u64(row, seam::F_CREATED)?,
            root: need_str(row, seam::F_ROOT)?,
            contents: need_strs(row, seam::F_CONTENTS)?,
            note_md: opt_str(row, seam::F_NOTE)?,
            path: need_abs(row, seam::F_PATH)?,
            link: opt_str(row, seam::F_LINK)?,
            sealed: row.get(seam::F_ONE_TIME).is_some(),
        });
    }
    Ok(Index { form, kits })
}

pub fn pick<'a>(kits: &'a [Kit], id: &str) -> R<&'a Kit> {
    kits.iter().find(|k| k.id == id).ok_or_else(|| Fault::NoSuchKit(id.to_string()))
}

pub(crate) fn unreadable(path: &str, e: &eravon_store::fault::Fault) -> Fault {
    Fault::Io(format!("{path} 在而读不动:{}", e.detail()))
}
