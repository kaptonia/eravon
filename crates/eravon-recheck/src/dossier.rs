use std::path::Path;

use eravon_dossier as k4;
use eravon_galeed as k1;
use eravon_link as k3;
use eravon_terms as k2;

use crate::fault::{Fault, R};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub kind: String,
    pub tx: String,
    pub block: u64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminal {
    pub name: String,
    pub amounts: [String; 4],
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub bytes: Vec<u8>,
    pub value: k1::Value,
}

#[derive(Clone, Debug)]
pub struct Case {
    pub chain_id: u64,
    pub posted: k4::index::Posted,
    pub listing_bytes: Vec<u8>,
    pub instance: String,
    pub case_id: String,
    pub logic: String,
    pub proxy_code_hash: String,
    pub seat: k4::index::Seat,
    pub rows: Vec<Row>,
    pub terminal: Option<Terminal>,
    pub listing: k3::listing::Listing,
    pub receipt: k3::receipt::Receipt,
    pub bound: k3::receipt::Bound,
    pub buyer_consent: k2::sig::Form,
    pub seller_consent: k2::sig::Form,
    pub entry: Option<Entry>,
}

impl Case {
    pub fn pool(&self) -> bool {
        self.receipt.candidate as usize >= self.listing.judges.len()
    }
}

#[derive(Debug)]
pub enum Opened {
    Whole(Box<Case>),
    Refused(Vec<u8>),
}

pub fn open(dir: &Path) -> R<Opened> {
    crate::mark::mark(crate::mark::FEATURE);
    let idx_bytes = match k4::files::read_index_bytes(dir) {
        Err(k4::files::Stop::Fault(f)) => return Ok(Opened::Refused(k1::schema::answer_bytes(&f.to_value(true)))),
        Ok(b) => b,
    };
    let idx = match k4::index::index(&idx_bytes) {
        Err(f) => return Ok(Opened::Refused(k1::schema::answer_bytes(&f.to_value(true)))),
        Ok(i) => i,
    };
    let whole = match k4::files::files(dir, &idx) {
        Err(k4::files::Stop::Fault(f)) => return Ok(Opened::Refused(k1::schema::answer_bytes(&f.to_value(true)))),
        Ok(w) => w,
    };
    let value = k1::canon::accept(&idx_bytes).map_err(|t| Fault::Dossier(format!("K4 判过的索引再读不成:{t:?}")))?;
    let bytes_of = |key: &str| -> R<Vec<u8>> {
        let (_, r) = idx
            .files
            .iter()
            .find(|(k, _)| *k == key)
            .ok_or_else(|| Fault::Dossier(format!("索引里没有 {key} 那一份档")))?;
        let b = by_entry(dir, &r.name)?;
        if k1::hash::hex_encode(&k1::hash::sha256(&b)) != r.sha256 {
            return Err(Fault::Dossier(format!("按目录项读到的 {} 与索引的 sha256 不合", r.name)));
        }
        Ok(b)
    };
    let listing_bytes = bytes_of("listing")?;
    let listing = k3::listing::listing(&listing_bytes)
        .map_err(|f| Fault::Dossier(format!("K4 判过的清单再判不过:{:?}", f.to_value(true))))?;
    let receipt = k3::receipt::receipt(&bytes_of("receipt")?)
        .map_err(|f| Fault::Dossier(format!("K4 判过的回执再判不过:{:?}", f.to_value(true))))?;
    let bound = k3::receipt::bind_receipt(&listing, &receipt)
        .map_err(|v| Fault::Dossier(format!("K4 绑过的回执再绑不上:{:?}", v.to_answer())))?;
    let entry = match idx.seat.entry_id {
        None => None,
        Some(_) => {
            let b = bytes_of("entry")?;
            let e = k1::entry::accept(&b).map_err(|t| Fault::Dossier(format!("K4 判过的条目再判不过:{t:?}")))?;
            Some(Entry { bytes: b, value: e.value })
        }
    };
    Ok(Opened::Whole(Box::new(Case {
        chain_id: idx.chain_id,
        posted: idx.posted.clone(),
        listing_bytes,
        instance: idx.instance.clone(),
        case_id: idx.case_id.clone(),
        logic: idx.logic.clone(),
        proxy_code_hash: idx.proxy_code_hash.clone(),
        seat: idx.seat.clone(),
        rows: rows(&value)?,
        terminal: terminal(&value)?,
        listing,
        receipt,
        bound,
        buyer_consent: whole.buyer_consent,
        seller_consent: whole.seller_consent,
        entry,
    })))
}

fn by_entry(dir: &Path, name: &str) -> R<Vec<u8>> {
    let entries = std::fs::read_dir(dir).map_err(|e| Fault::Dossier(format!("目录 {} 读不动:{e}", dir.display())))?;
    for e in entries.flatten() {
        if e.file_name().as_encoded_bytes() == name.as_bytes() {
            return std::fs::read(e.path()).map_err(|x| Fault::Dossier(format!("{name} 读不动:{x}")));
        }
    }
    Err(Fault::Dossier(format!("目录里没有名为 {name} 的那一项")))
}

fn text(v: &k1::Value, key: &str) -> R<String> {
    v.get(key)
        .and_then(k1::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Fault::Dossier(format!("K4 判过的索引上 {key} 不是串")))
}

fn int(v: &k1::Value, key: &str) -> R<u64> {
    v.get(key).and_then(k2::form::as_int).ok_or_else(|| Fault::Dossier(format!("K4 判过的索引上 {key} 不是数")))
}

fn rows(v: &k1::Value) -> R<Vec<Row>> {
    let list = v
        .get("transactions")
        .and_then(k2::form::as_arr)
        .ok_or_else(|| Fault::Dossier("K4 判过的索引上 transactions 不是列".into()))?;
    list.iter()
        .map(|r| Ok(Row { kind: text(r, "kind")?, tx: text(r, "tx")?, block: int(r, "block")?, timestamp: int(r, "timestamp")? }))
        .collect()
}

fn terminal(v: &k1::Value) -> R<Option<Terminal>> {
    let Some(t) = v.get("terminal") else { return Ok(None) };
    Ok(Some(Terminal {
        name: text(t, "name")?,
        amounts: [text(t, "buyerPaid")?, text(t, "sellerPaid")?, text(t, "judgePaid")?, text(t, "bondToBuyer")?],
    }))
}
