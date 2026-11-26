use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::json_in::In;
use eravon_chain::transport::Transport;
use eravon_literals::json::Out;
use eravon_scan::log::{self, Log};
use eravon_standard as a1;

use crate::fault::{Fault, R};

#[derive(Clone, Debug)]
pub struct Tx {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub input: Vec<u8>,
    pub ok: bool,
    pub block: u64,
    pub logs: Vec<Log>,
}

fn low(v: &In, key: &str) -> String {
    v.get(key).and_then(In::as_str).unwrap_or_default().to_ascii_lowercase()
}

pub fn tx<T: Transport>(t: &T, table: &Table, hash: &str) -> R<Option<Tx>> {
    let (body, _) = a2::read::transaction(t, table, hash)?;
    if matches!(body, In::Null) {
        return Ok(None);
    }
    let r = a2::read::read_agreed(t, table, a2::seam::M_GET_RECEIPT, vec![Out::S(hash.to_string())])?;
    if matches!(r.value, In::Null) {
        return Ok(None);
    }
    let rec = r.value;
    let block = a2::seam::read_quantity(&low(&rec, "blockNumber"))?;
    let mut logs = Vec::new();
    for l in rec.get("logs").and_then(In::as_arr).unwrap_or(&[]) {
        logs.push(Log::read(l)?);
    }
    let input = a1::order::bytes_of("交易的 input", &low(&body, "input"))?;
    Ok(Some(Tx {
        hash: hash.to_ascii_lowercase(),
        from: low(&body, "from"),
        to: low(&body, "to"),
        input,
        ok: low(&rec, "status") == "0x1",
        block,
        logs,
    }))
}

#[derive(Clone, Debug)]
pub struct Event {
    pub log: Log,
    pub ev: a1::event::Log,
}

pub fn events(logs: &[Log], addr: &str) -> Vec<Event> {
    logs.iter()
        .filter(|l| l.address.eq_ignore_ascii_case(addr))
        .filter_map(|l| a1::event::decode(&l.topics, &l.data).ok().map(|ev| Event { log: l.clone(), ev }))
        .collect()
}

pub fn instance_events<T: Transport>(t: &T, table: &Table, instance: &str, from: u64) -> R<Vec<Event>> {
    let (rows, _) = a2::read::logs(t, table, instance, from, a2::seam::Upto::of_range(0, None, None)?)?;
    let mut logs = Vec::new();
    for r in &rows {
        logs.push(Log::read(r)?);
    }
    logs.sort_by(|a, b| a.block.cmp(&b.block).then(a.log_index.cmp(&b.log_index)));
    Ok(events(&logs, instance))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Caller {
    Own { from: String, input: Vec<u8> },
    Inner { from: String, input: Vec<u8> },
    Missing,
    Untraced,
}

pub fn call_in<T: Transport>(t: &T, table: &Table, tx: &Tx, to: &str, sel: &[u8]) -> R<Caller> {
    if !tx.ok {
        return Ok(Caller::Missing);
    }
    if tx.to.eq_ignore_ascii_case(to) && tx.input.starts_with(sel) && code_before(t, table, to, tx.block) == Before::Code {
        return Ok(Caller::Own { from: tx.from.clone(), input: tx.input.clone() });
    }
    inner(t, table, tx, to, sel)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Before {
    Code,
    Empty,
    Unread,
}

pub fn code_before<T: Transport>(t: &T, table: &Table, addr: &str, block: u64) -> Before {
    let Some(b) = block.checked_sub(1) else { return Before::Empty };
    match code_at(t, table, addr, Some(b)) {
        Ok(c) if !c.is_empty() => Before::Code,
        Ok(_) => Before::Empty,
        Err(_) => unread_before(),
    }
}

fn unread_before() -> Before {
    Before::Unread
}


pub fn traced<T: Transport>(t: &T, table: &Table, tx: &Tx, to: &str, sel: &[u8]) -> R<Caller> {
    if !tx.ok {
        return Ok(Caller::Missing);
    }
    inner(t, table, tx, to, sel)
}

fn inner<T: Transport>(t: &T, table: &Table, tx: &Tx, to: &str, sel: &[u8]) -> R<Caller> {
    Ok(match a2::trace::find(t, table, &tx.hash, to, &a1::order::hex_of(sel))?.0 {
        a2::trace::Traced::Found { call, .. } => {
            Caller::Inner { from: call.from, input: a1::order::bytes_of("内层 calldata", &call.input)? }
        }
        a2::trace::Traced::NotFound { .. } => Caller::Missing,
        a2::trace::Traced::Untraced => Caller::Untraced,
    })
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Consent {
    pub block: u64,
    pub log_index: u64,
    pub tx: String,
}

pub fn consented<T: Transport>(t: &T, table: &Table, logic: &str, keys: &[String], who: &str, digest: &[u8; 32]) -> R<Vec<Consent>> {
    let mut out = Vec::new();
    for l in logic_logs(t, table, logic, keys, &a1::sol::CONSENTED_EV.topic0(), &[Some(log::address_topic(who)?), Some(a1::order::hex_of(digest))])? {
        if a1::consent::from_logic(&l.address, logic) {
            out.push(Consent { block: l.block, log_index: l.log_index, tx: l.tx.clone() });
        }
    }
    Ok(out)
}

pub fn logic_logs<T: Transport>(t: &T, table: &Table, logic: &str, keys: &[String], event: &[u8; 32], rest: &[Option<String>]) -> R<Vec<Log>> {
    let records = eravon_scan::scan::logic_records(t, table, logic, keys, a2::seam::Upto::Latest)?;
    Ok(eravon_scan::scan::picked(&records, event, rest).into_iter().cloned().collect())
}

pub fn consents_in(tx: &Tx, logic: &str) -> R<Vec<(String, [u8; 32])>> {
    let t0 = a1::sol::CONSENTED_EV.topic0();
    let mut out = Vec::new();
    for l in &tx.logs {
        if !a1::consent::from_logic(&l.address, logic) || l.topics.first() != Some(&t0) {
            continue;
        }
        let digest = *l.topics.get(2).ok_or_else(|| Fault::Form("Consented 该有三枚 topic".into()))?;
        out.push((l.topic_address(1)?, digest));
    }
    Ok(out)
}

pub fn code_at<T: Transport>(t: &T, table: &Table, addr: &str, block: Option<u64>) -> R<Vec<u8>> {
    let at = match block {
        Some(n) => a2::seam::quantity(n),
        None => a2::seam::TAG_LATEST.to_string(),
    };
    Ok(a2::read::code(t, table, addr, &at)?.0)
}

pub fn logs_of<T: Transport>(t: &T, table: &Table, addr: &str, topics: &[Option<String>], from: u64) -> R<Vec<Log>> {
    let any: Vec<Vec<String>> = topics.iter().map(|x| x.iter().cloned().collect()).collect();
    logs_any_of(t, table, addr, &any, from)
}

pub fn logs_any_of<T: Transport>(t: &T, table: &Table, addr: &str, topics: &[Vec<String>], from: u64) -> R<Vec<Log>> {
    let (rows, _) = a2::read::logs_any(t, table, Some(addr), topics, from, a2::seam::Upto::of_range(0, None, None)?)?;
    let mut out = Vec::new();
    for r in &rows {
        let l = Log::read(r)?;
        if l.address.eq_ignore_ascii_case(addr) {
            out.push(l);
        }
    }
    out.sort_by(|a, b| a.block.cmp(&b.block).then(a.log_index.cmp(&b.log_index)));
    Ok(out)
}

