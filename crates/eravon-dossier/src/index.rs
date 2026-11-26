use eravon_galeed as k1;
use eravon_literals::literals as lit;
use eravon_terms::form;

use crate::token::{Fault, Token};

#[derive(Clone, Debug)]
pub struct Index {
    pub chain_id: u64,
    pub instance: String,
    pub case_id: String,
    pub logic: String,
    pub proxy_code_hash: String,
    pub posted: Posted,
    pub files: Vec<(&'static str, FileRef)>,
    pub attachments: Vec<FileRef>,
    pub seat: Seat,
    pub terminal: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Posted {
    pub tx: String,
    pub hash: String,
}

#[derive(Clone, Debug)]
pub struct FileRef {
    pub name: String,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub struct Seat {
    pub judge: Option<String>,
    pub amanah_hash: String,
    pub quote_hash: String,
    pub seat: u64,
    pub entry_id: Option<String>,
}

const KEYS: [&str; 12] = [
    "v",
    "chainId",
    "instance",
    "caseId",
    "logic",
    "proxyCodeHash",
    "posted",
    "files",
    "seat",
    "transactions",
    "terminal",
    "compliance",
];

pub const FILE_KEYS: [&str; 3] = ["listing", "receipt", "entry"];

fn is_file_name(s: &str) -> bool {
    !s.is_empty() && k1::canon::is_skeleton(s) && !s.contains('/') && s != "." && s != ".."
}

fn file_ref(v: &k1::Value) -> Option<FileRef> {
    let m = form::as_obj(v)?;
    if m.len() != 2 {
        return None;
    }
    let name = form::as_str(v.get("name")?)?;
    let sha = form::as_str(v.get("sha256")?)?;
    if !is_file_name(name) || !k1::hexform::is_hex32(sha) {
        return None;
    }
    Some(FileRef { name: name.to_string(), sha256: sha.to_string() })
}

pub fn index(b: &[u8]) -> Result<Index, Fault> {
    let v = k1::canon::accept(b)
        .map_err(|t| Fault::new(Token::Up(eravon_link::token::up(eravon_terms::token::canon(t)))))?;
    let Some(m) = form::as_obj(&v) else {
        return Err(Fault::new(Token::Dossier));
    };
    for k in &KEYS[..10] {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::new(Token::Missing));
        }
    }
    if m.iter().any(|(key, _)| !KEYS.contains(&key.as_str())) {
        return Err(Fault::new(Token::Closed));
    }
    let g = |k: &str| v.get(k).expect("必有成员已查过");

    if form::as_str(g("v")) != Some(lit::DOC_DOSSIER) {
        return Err(Fault::new(Token::V));
    }
    let Some(chain_id) = form::as_int(g("chainId")) else {
        return Err(Fault::new(Token::Chain));
    };
    if !form::as_str(g("instance")).is_some_and(k1::hexform::is_hex20) {
        return Err(Fault::new(Token::Instance));
    }
    if !form::is_hex32(g("caseId")) {
        return Err(Fault::new(Token::Case));
    }
    if !form::as_str(g("logic")).is_some_and(k1::hexform::is_hex20) {
        return Err(Fault::new(Token::Logic));
    }
    if !form::is_hex32(g("proxyCodeHash")) {
        return Err(Fault::new(Token::Codehash));
    }

    let posted = read_posted(g("posted"))?;
    let (files, attachments) = read_files(g("files"))?;
    let seat = read_seat(g("seat"))?;

    let Some(rows) = form::as_arr(g("transactions")) else {
        return Err(Fault::new(Token::Transactions));
    };
    for (i, row) in rows.iter().enumerate() {
        check_transaction(row, i)?;
    }

    let terminal = match v.get("terminal") {
        None => None,
        Some(t) => Some(read_terminal(t)?),
    };
    if terminal.as_deref() == Some("Ruled") && (seat.entry_id.is_none() || seat.judge.is_none()) {
        return Err(Fault::new(Token::Ruled));
    }
    if let Some(c) = v.get("compliance") {
        check_compliance(c)?;
    }

    Ok(Index {
        chain_id,
        instance: form::as_str(g("instance")).expect("已过形").to_string(),
        case_id: form::as_str(g("caseId")).expect("已过形").to_string(),
        logic: form::as_str(g("logic")).expect("已过形").to_string(),
        proxy_code_hash: form::as_str(g("proxyCodeHash")).expect("已过形").to_string(),
        posted,
        files,
        attachments,
        seat,
        terminal,
    })
}


fn read_posted(v: &k1::Value) -> Result<Posted, Fault> {
    let shaped = form::as_obj(v).is_some_and(|m| {
        m.len() == 2 && m.iter().any(|(k, _)| k == "tx") && m.iter().any(|(k, _)| k == "hash")
    });
    let parts = shaped
        .then(|| (v.get("tx").and_then(form::as_str), v.get("hash").and_then(form::as_str)))
        .and_then(|(t, h)| Some((t?, h?)))
        .filter(|(t, h)| k1::hexform::is_hex32(t) && k1::hexform::is_hex32(h));
    let Some((tx, hash)) = parts else {
        return Err(Fault::new(Token::Posted));
    };
    Ok(Posted { tx: tx.to_string(), hash: hash.to_string() })
}

fn read_files(v: &k1::Value) -> Result<(Vec<(&'static str, FileRef)>, Vec<FileRef>), Fault> {
    let Some(m) = form::as_obj(v) else {
        return Err(Fault::new(Token::Files));
    };
    for k in &FILE_KEYS[..2] {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::new(Token::Files));
        }
    }
    if m.iter().any(|(key, _)| !FILE_KEYS.contains(&key.as_str()) && key != "attachments") {
        return Err(Fault::new(Token::Files));
    }
    let mut out = Vec::with_capacity(3);
    for k in FILE_KEYS {
        if let Some(x) = v.get(k) {
            let r = file_ref(x).ok_or_else(|| Fault::new(Token::Files))?;
            out.push((k, r));
        }
    }
    let mut attachments = Vec::new();
    if let Some(a) = v.get("attachments") {
        let a = form::as_arr(a).ok_or_else(|| Fault::new(Token::Files))?;
        for el in a {
            attachments.push(file_ref(el).ok_or_else(|| Fault::new(Token::Files))?);
        }
    }
    Ok((out, attachments))
}

fn read_seat(v: &k1::Value) -> Result<Seat, Fault> {
    const SK: [&str; 5] = ["amanahHash", "quoteHash", "seat", "judge", "entryId"];
    let Some(m) = form::as_obj(v) else {
        return Err(Fault::new(Token::Seat));
    };
    for k in &SK[..3] {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::new(Token::Seat));
        }
    }
    if m.iter().any(|(key, _)| !SK.contains(&key.as_str())) {
        return Err(Fault::new(Token::Seat));
    }
    let judge = match v.get("judge") {
        None => None,
        Some(x) if form::is_nonzero_hex20(x) => Some(form::as_str(x).expect("已过形").to_string()),
        Some(_) => return Err(Fault::new(Token::Seat)),
    };
    if !form::is_nonzero_hex32(v.get("amanahHash").expect("已查过"))
        || !form::is_hex32(v.get("quoteHash").expect("已查过"))
        || !form::is_int_at_most(v.get("seat").expect("已查过"), 2)
    {
        return Err(Fault::new(Token::Seat));
    }
    let entry_id = match v.get("entryId") {
        None => None,
        Some(x) if form::is_hex32(x) => Some(form::as_str(x).expect("已过形").to_string()),
        Some(_) => return Err(Fault::new(Token::Seat)),
    };
    let seat = form::as_int(v.get("seat").expect("已查过")).expect("已过形");
    if judge.is_none() && (seat != 0 || entry_id.is_some()) {
        return Err(Fault::new(Token::Seat));
    }
    Ok(Seat {
        judge,
        amanah_hash: form::as_str(v.get("amanahHash").expect("已查过")).expect("已过形").to_string(),
        quote_hash: form::as_str(v.get("quoteHash").expect("已查过")).expect("已过形").to_string(),
        seat,
        entry_id,
    })
}

fn check_transaction(v: &k1::Value, i: usize) -> Result<(), Fault> {
    const TK: [&str; 5] = ["kind", "tx", "block", "timestamp", "note_md"];
    let Some(m) = form::as_obj(v) else {
        return Err(Fault::at(Token::Transaction, i));
    };
    for k in &TK[..4] {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::at(Token::Transaction, i));
        }
    }
    if m.iter().any(|(key, _)| !TK.contains(&key.as_str())) {
        return Err(Fault::at(Token::Transaction, i));
    }
    let kind_ok = form::as_str(v.get("kind").expect("已查过"))
        .is_some_and(|s| lit::TRANSACTION_KINDS.contains(&s));
    if !kind_ok
        || !form::is_hex32(v.get("tx").expect("已查过"))
        || form::as_int(v.get("block").expect("已查过")).is_none()
        || form::as_int(v.get("timestamp").expect("已查过")).is_none()
    {
        return Err(Fault::at(Token::Transaction, i));
    }
    if let Some(n) = v.get("note_md")
        && form::as_str(n).is_none()
    {
        return Err(Fault::at(Token::Transaction, i));
    }
    Ok(())
}

fn read_terminal(v: &k1::Value) -> Result<String, Fault> {
    const NK: [&str; 5] = ["name", "buyerPaid", "sellerPaid", "judgePaid", "bondToBuyer"];
    let Some(m) = form::as_obj(v) else {
        return Err(Fault::new(Token::Terminal));
    };
    if m.len() != 5 || !NK.iter().all(|k| m.iter().any(|(key, _)| key == k)) {
        return Err(Fault::new(Token::Terminal));
    }
    let name = form::as_str(v.get("name").expect("已查过"))
        .filter(|s| lit::TERMINAL_NAMES.contains(s))
        .ok_or_else(|| Fault::new(Token::Terminal))?;
    for k in &NK[1..] {
        let ok = form::as_str(v.get(k).expect("已查过"))
            .and_then(eravon_terms::u256::U256::from_dec)
            .is_some();
        if !ok {
            return Err(Fault::new(Token::Terminal));
        }
    }
    Ok(name.to_string())
}

fn check_compliance(v: &k1::Value) -> Result<(), Fault> {
    const CK: [&str; 4] = ["buyer_md", "seller_md", "serial", "conversions"];
    let Some(m) = form::as_obj(v) else {
        return Err(Fault::new(Token::Compliance));
    };
    if m.iter().any(|(key, _)| !CK.contains(&key.as_str())) {
        return Err(Fault::new(Token::Compliance));
    }
    for k in ["buyer_md", "seller_md"] {
        if let Some(x) = v.get(k)
            && form::as_str(x).is_none()
        {
            return Err(Fault::new(Token::Compliance));
        }
    }
    if let Some(x) = v.get("serial") {
        let ok = form::as_str(x).is_some_and(|s| !s.is_empty() && k1::canon::is_skeleton(s));
        if !ok {
            return Err(Fault::new(Token::Compliance));
        }
    }
    if let Some(x) = v.get("conversions") {
        let Some(a) = form::as_arr(x) else {
            return Err(Fault::new(Token::Compliance));
        };
        for (i, el) in a.iter().enumerate() {
            check_conversion(el).map_err(|()| Fault::at(Token::Compliance, i))?;
        }
    }
    Ok(())
}

fn check_conversion(v: &k1::Value) -> Result<(), ()> {
    const XK: [&str; 5] = ["tx", "at", "unit", "rate", "source"];
    let m = form::as_obj(v).ok_or(())?;
    if m.len() != 5 || !XK.iter().all(|k| m.iter().any(|(key, _)| key == k)) {
        return Err(());
    }
    if !form::is_hex32(v.get("tx").ok_or(())?) || form::as_int(v.get("at").ok_or(())?).is_none() {
        return Err(());
    }
    for k in ["unit", "rate", "source"] {
        let ok = form::as_str(v.get(k).ok_or(())?)
            .is_some_and(|s| !s.is_empty() && k1::canon::is_skeleton(s));
        if !ok {
            return Err(());
        }
    }
    Ok(())
}
