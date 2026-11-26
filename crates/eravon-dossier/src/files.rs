use std::path::Path;

use eravon_galeed as k1;
use eravon_link as k3;
use eravon_literals::literals as lit;
use eravon_terms as k2;
use k2::form;

use crate::index::Index;
use crate::token::{Fault, Token, pass_link};

#[derive(Clone, Debug)]
pub struct Whole {
    pub terms_hash: String,
    pub instance: String,
    pub case_id: String,
    pub candidate: u64,
    pub entry_id: Option<String>,
    pub buyer_consent: k2::sig::Form,
    pub seller_consent: k2::sig::Form,
    pub attachments: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReadStop {
    Unreadable,
}

fn read_named(dir: &Path, name: &str) -> Result<Vec<u8>, ReadStop> {
    let entries = std::fs::read_dir(dir).map_err(|_| ReadStop::Unreadable)?;
    for e in entries.flatten() {
        if e.file_name().as_encoded_bytes() == name.as_bytes() {
            let p = e.path();
            let meta = std::fs::metadata(&p).map_err(|_| ReadStop::Unreadable)?;
            if !meta.is_file() {
                return Err(ReadStop::Unreadable);
            }
            return std::fs::read(&p).map_err(|_| ReadStop::Unreadable);
        }
    }
    Err(ReadStop::Unreadable)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Stop {
    Fault(Fault),
}

impl From<Fault> for Stop {
    fn from(f: Fault) -> Stop {
        Stop::Fault(f)
    }
}

pub fn index_name() -> &'static str {
    lit::DOSSIER_INDEX_FILE
}

pub fn read_index_bytes(dir: &Path) -> Result<Vec<u8>, Stop> {
    read_named(dir, index_name()).map_err(|e| match e {
        ReadStop::Unreadable => Stop::Fault(Fault::keyed(Token::File, "index")),
    })
}

fn refs(idx: &Index) -> Vec<(String, &crate::index::FileRef)> {
    let mut out: Vec<(String, &crate::index::FileRef)> = idx.files.iter().map(|(k, r)| (k.to_string(), r)).collect();
    out.extend(idx.attachments.iter().enumerate().map(|(i, r)| (format!("attachment:{i}"), r)));
    out
}

pub fn files(dir: &Path, idx: &Index) -> Result<Whole, Stop> {
    let mut bytes: Vec<(String, Vec<u8>)> = Vec::new();
    for (key, r) in refs(idx) {
        let b = read_named(dir, &r.name).map_err(|e| match e {
            ReadStop::Unreadable => Stop::Fault(Fault::keyed(Token::File, &key)),
            })?;
        let h = k1::hash::hex_encode(&k1::hash::sha256(&b));
        if h != r.sha256 {
            return Err(Stop::Fault(Fault::keyed(Token::Digest, &key)));
        }
        if !key.starts_with("attachment:") {
            bytes.push((key, b));
        }
    }
    Ok(check_documents(idx, &bytes)?)
}


fn of<'a>(bytes: &'a [(String, Vec<u8>)], key: &str) -> Option<&'a [u8]> {
    bytes.iter().find(|(k, _)| k == key).map(|(_, b)| b.as_slice())
}

fn check_documents(idx: &Index, bytes: &[(String, Vec<u8>)]) -> Result<Whole, Fault> {
    let listing_b = of(bytes, "listing").expect("必有键已读");
    let receipt_b = of(bytes, "receipt").expect("必有键已读");

    let s = k3::listing::listing(listing_b).map_err(|f| Fault::new(Token::Listing).link(f))?;
    if k1::hash::hex_encode(&k1::hash::sha256(listing_b)) != idx.posted.hash {
        return Err(Fault::new(Token::Hash));
    }
    let r = k3::receipt::receipt(receipt_b).map_err(|f| Fault::new(Token::Receipt).link(f))?;
    let bound = k3::receipt::bind_receipt(&s, &r).map_err(|v| Fault::new(Token::Order).verdict(v))?;

    if idx.chain_id != s.chain_id {
        return Err(Fault::new(Token::ChainMismatch));
    }
    if idx.logic != s.logic {
        return Err(Fault::new(Token::LogicMismatch));
    }
    let pch = k1::hash::hex_encode(&k2::derive::proxy_code_hash(&s.logic));
    if idx.proxy_code_hash != pch {
        return Err(Fault::new(Token::CodehashMismatch));
    }
    if idx.instance != bound.instance {
        return Err(Fault::new(Token::InstanceMismatch));
    }
    if idx.case_id != k1::hash::hex_encode(&bound.salt) {
        return Err(Fault::new(Token::CaseMismatch));
    }
    check_seat(idx, &s, &r)?;
    check_entry(idx, bytes)?;
    check_attachments(idx, &s)?;

    Ok(Whole {
        terms_hash: s.terms_hash.clone(),
        instance: idx.instance.clone(),
        case_id: idx.case_id.clone(),
        candidate: r.candidate,
        entry_id: idx.seat.entry_id.clone(),
        buyer_consent: bound.form,
        seller_consent: seller_consent(&s, r.candidate),
        attachments: idx.attachments.len(),
    })
}

fn seller_consent(s: &k3::listing::Listing, candidate: u64) -> k2::sig::Form {
    if s.by_call.iter().any(|(c, _)| *c as u64 == candidate) { k2::sig::Form::Call } else { k2::sig::Form::Secp256k1 }
}

fn check_attachments(idx: &Index, s: &k3::listing::Listing) -> Result<(), Fault> {
    let named = s.named_hashes();
    for (i, r) in idx.attachments.iter().enumerate() {
        if !named.iter().any(|n| *n == r.sha256) {
            return Err(Fault::at(Token::Attachment, i));
        }
    }
    Ok(())
}



fn check_seat(idx: &Index, s: &k3::listing::Listing, r: &k3::receipt::Receipt) -> Result<(), Fault> {
    let c = r.candidate as usize;
    let (amanah, quote_hash, judge) = if c < s.judges.len() {
        let j = &s.judges[c];
        (&j.amanah_hash, &j.quote_hash, Some(&j.judge))
    } else {
        let g = &s.guilds[c - s.judges.len()];
        (&g.amanah_hash, &g.quote_hash, None)
    };
    if idx.seat.amanah_hash != *amanah || idx.seat.quote_hash != *quote_hash {
        return Err(Fault::new(Token::SeatMismatch));
    }
    if let Some(j) = judge
        && (idx.seat.judge.as_ref() != Some(j) || idx.seat.seat != 0)
    {
        return Err(Fault::new(Token::SeatMismatch));
    }
    Ok(())
}

fn check_entry(idx: &Index, bytes: &[(String, Vec<u8>)]) -> Result<(), Fault> {
    let has_file = bytes.iter().any(|(k, _)| k == "entry");
    if has_file != idx.seat.entry_id.is_some() {
        return Err(Fault::new(Token::Entry));
    }
    let Some(b) = of(bytes, "entry") else {
        return Ok(());
    };
    let want = idx.seat.entry_id.as_deref().expect("上面已问过在与不在");
    if k1::hash::hex_encode(&k1::hash::sha256(b)) != want {
        return Err(Fault::new(Token::EntryDigest));
    }
    let e = k1::entry::accept(b).map_err(|t| {
        Fault::new(Token::EntryForm)
            .link(eravon_link::token::Fault::new(eravon_link::token::up(k2::token::canon(t))))
    })?;
    let et = form::as_str(e.value.get(k1::entry::KEYS[1]).expect("信封已过十三步"))
        .expect("信封已过十三步");
    if et != "judgment" && et != "resignation" {
        return Err(Fault::new(Token::EntryType));
    }
    let body = e.value.get(k1::entry::KEYS[5]).expect("信封已过十三步");
    check_subject(idx, body, et == "judgment")
}


fn check_subject(idx: &Index, body: &k1::Value, is_judgment: bool) -> Result<(), Fault> {
    let bad = || Fault::new(Token::Subject);
    let sr = body.get("subjectRef").ok_or_else(bad)?;
    if form::as_str(sr.get("kind").ok_or_else(bad)?) != Some("evm") {
        return Err(bad());
    }
    if form::as_int(sr.get("chainId").ok_or_else(bad)?) != Some(idx.chain_id) {
        return Err(bad());
    }
    if form::as_str(sr.get("contract").ok_or_else(bad)?) != Some(idx.instance.as_str()) {
        return Err(bad());
    }
    if form::as_str(sr.get("codeHash").ok_or_else(bad)?) != Some(idx.proxy_code_hash.as_str()) {
        return Err(bad());
    }
    if form::as_str(sr.get("id").ok_or_else(bad)?) != Some(idx.case_id.as_str()) {
        return Err(bad());
    }
    match sr.get("seat") {
        Some(x) => {
            if form::as_int(x) != Some(idx.seat.seat) {
                return Err(bad());
            }
        }
        None => {
            if is_judgment {
                return Err(bad());
            }
        }
    }
    Ok(())
}

pub fn pass(f: eravon_link::token::Fault) -> Fault {
    pass_link(f)
}

#[cfg(test)]
mod bound_tests {
    use super::*;

    #[test]
    fn a_file_past_sixteen_mib_is_read() {
        let dir = std::env::temp_dir().join(format!("erd-bound-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let big = vec![0x7bu8; 17 * 1024 * 1024];
        std::fs::write(dir.join("big.bin"), &big).expect("写得出");
        let got = read_named(&dir, "big.bin").expect("照读");
        assert_eq!(got.len(), big.len());
        assert_eq!(read_named(&dir, "absent.bin"), Err(ReadStop::Unreadable));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
