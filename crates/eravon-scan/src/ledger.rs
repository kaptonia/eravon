use std::collections::BTreeSet;

use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed as k1;
use eravon_galeed::Value;

use crate::fault::{Fault, R};
use crate::log::Log;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Emitter {
    Address(String),
    Class(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub chain_id: u64,
    pub from: u64,
    pub to: u64,
    pub registries: Vec<Emitter>,
    pub senders: Vec<String>,
}

impl Window {
    pub fn to_head(chain_id: u64, head: u64, registries: Vec<Emitter>, senders: Vec<String>) -> Window {
        Window { chain_id, from: a2::read::records_from(head), to: head, registries, senders }
    }
}

pub fn windows(basis: &Value) -> Option<Vec<Window>> {
    let int = |v: Option<&Value>| match v {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    };
    let mut out = Vec::new();
    for c in basis.get(k1::audit::BASIS_CHAINS)?.as_arr()? {
        let registries = c
            .get("registries")?
            .as_arr()?
            .iter()
            .map(|e| match e {
                Value::Str(a) => Some(Emitter::Address(a.to_ascii_lowercase())),
                Value::Obj(_) => e.get("codeHash").and_then(Value::as_str).map(|h| Emitter::Class(h.to_ascii_lowercase())),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        let senders = c.get("senders")?.as_arr()?.iter().map(|s| s.as_str().map(str::to_ascii_lowercase)).collect::<Option<Vec<_>>>()?;
        out.push(Window { chain_id: int(c.get("chainId"))?, from: int(c.get("fromBlock"))?, to: int(c.get("toBlock"))?, registries, senders });
    }
    Some(out)
}

pub fn basis_of(ws: &[Window]) -> Value {
    let chains = ws
        .iter()
        .map(|w| {
            let regs = w
                .registries
                .iter()
                .map(|e| match e {
                    Emitter::Address(a) => Value::Str(a.clone()),
                    Emitter::Class(h) => Value::Obj(vec![("codeHash".into(), Value::Str(h.clone()))]),
                })
                .collect();
            Value::Obj(vec![
                ("chainId".into(), Value::Int(w.chain_id)),
                ("fromBlock".into(), Value::Int(w.from)),
                ("toBlock".into(), Value::Int(w.to)),
                ("registries".into(), Value::Arr(regs)),
                ("senders".into(), Value::Arr(w.senders.iter().map(|s| Value::Str(s.clone())).collect())),
            ])
        })
        .collect();
    Value::Obj(vec![(k1::audit::BASIS_CHAINS.into(), Value::Arr(chains))])
}

pub trait Evidence {
    fn tx(&self, chain_id: u64, tx: &str) -> R<Option<Value>>;
    fn status(&self, tx: &str) -> R<Option<bool>>;
    fn time(&self, chain_id: u64, block_hash: &str, number: u64) -> R<u64>;
    fn code_at(&self, chain_id: u64, block_hash: &str, who: &str, number: u64) -> Option<bool>;
    fn code_hash_at(&self, addr: &str, block: u64) -> R<String>;
    fn ahead(&self, chain_id: u64, wants: &[(&Log, &str)]) {
        let _ = (chain_id, wants);
    }
    fn ahead_codes(&self, wants: &[(String, u64)]) {
        let _ = wants;
    }
}

pub fn query_from(chain_id: u64, emitter: Option<&str>, from: u64) -> u64 {
    let declared = eravon_literals::constants::network_of(chain_id).and_then(|n| n.registries.get().copied()).unwrap_or(&[]);
    let start = declared.iter().map(|r| r.from_block).min().unwrap_or(0);
    let floor = emitter.and_then(|a| declared.iter().find(|r| r.address.eq_ignore_ascii_case(a))).map_or(start, |r| r.from_block);
    from.max(floor)
}

pub const UNCHECKED: &str = "未经证据核";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotAnchor {
    pub tx: String,
    pub log_index: u64,
    pub emitter: String,
    pub block: u64,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct Held {
    pub entry_id: String,
    pub bytes: Vec<u8>,
    pub value: Value,
    pub block: u64,
    pub tx: String,
}

#[derive(Clone, Debug)]
pub struct Ledger {
    pub root: String,
    pub basis: Value,
    pub narrow: Vec<String>,
    pub records: Vec<k1::audit::Record>,
    pub not_anchors: Vec<NotAnchor>,
    pub unchecked: Vec<NotAnchor>,
    pub emitters: Vec<(String, String)>,
    pub report: Option<Value>,
    pub entries: Vec<Held>,
    pub lineage: BTreeSet<String>,
    pub discarded: Vec<Discarded>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discarded {
    pub entry_id: String,
    pub tx: String,
    pub block: u64,
    pub sender: String,
}

pub const DISCARDED_SAYS: &str = "由已退任的钥在它退任之后所签,账本审计丢弃了它";

impl Ledger {
    pub fn label(&self) -> String {
        self.report.as_ref().and_then(|r| r.get("label")).and_then(Value::as_str).unwrap_or("NO_LABEL").to_string()
    }

    pub fn emitter_of(&self, tx: &str) -> Option<&str> {
        self.emitters.iter().find(|(t, _)| t.eq_ignore_ascii_case(tx)).map(|(_, e)| e.as_str())
    }

    pub fn entry(&self, entry_id: &str) -> Option<&Held> {
        self.entries.iter().find(|e| e.entry_id.eq_ignore_ascii_case(entry_id))
    }

    pub fn discarded_entry(&self, entry_id: &str) -> Option<&Discarded> {
        self.discarded.iter().find(|d| d.entry_id.eq_ignore_ascii_case(entry_id))
    }
}

pub fn reach(ws: &[Window], chain_id: u64, head: u64) -> (Vec<Window>, Vec<String>) {
    let mut narrow = Vec::new();
    let mut kept: Vec<Window> = Vec::new();
    for w in ws {
        if w.chain_id != chain_id {
            narrow.push(format!("链 {} 的窗 [{}, {}] 未读:本机端点只答链 {chain_id}", w.chain_id, w.from, w.to));
            continue;
        }
        kept.push(from_own(w.clone()));
    }
    if let Some(last) = kept.last_mut() {
        if head > last.to {
            last.to = head;
        } else if head < last.to {
            narrow.push(format!("链 {chain_id} 读到块 {head},基底写到块 {}", last.to));
            last.to = head;
        }
    }
    kept.retain(|w| w.from <= w.to);
    (kept, narrow)
}

fn from_own(w: Window) -> Window {
    w
}


pub fn read<T: Transport>(
    t: &T,
    table: &Table,
    ev: &dyn Evidence,
    chain_id: u64,
    root: &str,
    basis: &Value,
    head: u64,
) -> R<Ledger> {
    crate::mark::mark(crate::mark::FEATURE);
    eravon_galeed_scan::basis::read(basis).ok_or_else(|| Fault::Form("基底不合 galeed/1 §9.4(核不收)".into()))?;
    let declared = windows(basis).ok_or_else(|| Fault::Form("基底的成员读不出(chains/registries/senders)".into()))?;
    let (ws, narrow) = reach(&declared, chain_id, head);
    let topic0 = crate::scan::anchored_topic0();
    let mut seen: BTreeSet<(String, u64)> = BTreeSet::new();
    let mut records = Vec::new();
    let mut not_anchors = Vec::new();
    let mut emitters = Vec::new();
    let mut unchecked = Vec::new();
    let known = crate::judged::known();
    let mut new = crate::judged::Record::default();
    let mut candidates: Vec<(Log, String, bool, Window)> = Vec::new();
    for w in &ws {
        let senders: Vec<String> = w.senders.iter().map(|s| crate::log::address_topic(s)).collect::<R<_>>()?;
        if senders.is_empty() {
            continue;
        }
        let class = w.registries.iter().any(|e| matches!(e, Emitter::Class(_)));
        let mut asks: Vec<Option<String>> = if class {
            Vec::new()
        } else {
            w.registries.iter().filter_map(|e| match e {
                Emitter::Address(a) => Some(Some(a.clone())),
                Emitter::Class(_) => None,
            }).collect()
        };
        if class {
            asks.push(None);
        }
        let mut found: Vec<(Log, String)> = Vec::new();
        let topics: Vec<Vec<String>> = vec![vec![topic0.clone()], senders.to_vec()];
        let reads: Vec<(Option<&str>, &[Vec<String>], u64, a2::seam::Upto)> = asks
            .iter()
            .map(|addr| (addr.as_deref(), topics.as_slice(), query_from(w.chain_id, addr.as_deref(), w.from), a2::seam::Upto::Block(w.to)))
            .filter(|r| r.2 <= w.to)
            .collect();
        a2::read::logs_any_ahead(t, table, &reads, true);
        for addr in asks {
            let from = query_from(w.chain_id, addr.as_deref(), w.from);
            if from > w.to {
                continue;
            }
            {
                let (rows, _) = a2::read::logs_any_whole(t, table, addr.as_deref(), &topics, from, a2::seam::Upto::Block(w.to)).map_err(Fault::from)?;
                for row in &rows {
                    let l = Log::read(row)?;
                    if !seen.insert((l.tx.to_ascii_lowercase(), l.log_index)) {
                        continue;
                    }
                    if l.topics.len() != 3 {
                        continue;
                    }
                    let block_hash = row.get("blockHash").and_then(eravon_literals::json::In::as_str).ok_or_else(|| Fault::Form(format!("日志 {} 缺 blockHash", l.tx)))?.to_string();
                    found.push((l, block_hash));
                }
            }
        }
        let registry = |a: &str| w.registries.iter().any(|e| matches!(e, Emitter::Address(x) if x == a));
        let place = |a: &str| w.registries.iter().position(|e| matches!(e, Emitter::Address(x) if x == a)).unwrap_or(w.registries.len());
        found.sort_by_key(|(l, _)| place(&l.address.to_ascii_lowercase()));
        for (l, block_hash) in found {
            let listed = registry(&l.address.to_ascii_lowercase());
            if !listed && !class {
                continue;
            }
            candidates.push((l, block_hash, listed, w.clone()));
        }
    }
    let key_of = |l: &Log, h: &str| crate::judged::Key::of(chain_id, h, &l.tx, l.log_index);
    let new_ones: Vec<(&Log, &str)> = candidates.iter().filter(|(l, h, _, _)| !known.anchors.contains_key(&key_of(l, h))).map(|(l, h, _, _)| (l, h.as_str())).collect();
    ev.ahead(chain_id, &new_ones);
    for (l, block_hash, listed, w) in &candidates {
        let key = key_of(l, block_hash);
        let judged = match known.anchors.get(&key) {
            Some(j) => Ok(j.clone()),
            None => tested(ev, chain_id, l, block_hash).inspect(|j| {
                new.anchors.insert(key, j.clone());
            }),
        }
        .and_then(|j| in_basis(ev, chain_id, w, l, *listed, j, &known.codes, &mut new.codes));
        let judged = match judged {
            Ok(j) => j,
            Err(e) => {
                unchecked.push(NotAnchor { tx: l.tx.clone(), log_index: l.log_index, emitter: l.address.to_ascii_lowercase(), block: l.block, reason: format!("{UNCHECKED}:{}", e.detail()) });
                continue;
            }
        };
        match judged {
            crate::judged::Judged::Nothing(why) => not_anchors.push(NotAnchor { tx: l.tx.clone(), log_index: l.log_index, emitter: l.address.to_ascii_lowercase(), block: l.block, reason: why }),
            crate::judged::Judged::Shows(r) => {
                emitters.push((l.tx.to_ascii_lowercase(), l.address.to_ascii_lowercase()));
                records.push(r);
            }
        }
    }
    crate::judged::add(&new);
    let mut narrow = narrow;
    let cut_at: BTreeSet<u64> = unchecked.iter().map(|n| n.block).collect();
    for b in &cut_at {
        narrow.push(format!("链 {chain_id} 第 {b} 块有锚取不到证据,基底剔除这一块"));
    }
    records.retain(|r| !(r.chain_id == chain_id && cut_at.contains(&r.block_number)));
    let ws = cut(&ws, chain_id, &cut_at);
    let basis = basis_of(&ws);
    let mut l = audited(root, basis, narrow, records, not_anchors);
    l.emitters = emitters;
    l.unchecked = unchecked;
    Ok(l)
}

fn cut(ws: &[Window], chain_id: u64, blocks: &BTreeSet<u64>) -> Vec<Window> {
    let mut out = Vec::new();
    for w in ws {
        if w.chain_id != chain_id {
            out.push(w.clone());
            continue;
        }
        let mut from = w.from;
        for &b in blocks.iter().filter(|b| **b >= w.from && **b <= w.to) {
            if b > from {
                out.push(Window { from, to: b - 1, ..w.clone() });
            }
            from = b + 1;
        }
        if from <= w.to {
            out.push(Window { from, ..w.clone() });
        }
    }
    out
}

fn tested(ev: &dyn Evidence, chain_id: u64, l: &Log, block_hash: &str) -> R<crate::judged::Judged> {
    use eravon_galeed_scan::{txform, verdict};
    let no = |why: &str| Ok(crate::judged::Judged::Nothing(why.to_string()));
    let Some(tx_value) = ev.tx(chain_id, &l.tx)? else {
        return Err(Fault::Form(format!("交易 {} 没有一家给出", l.tx)));
    };
    if matches!(tx_value.get("to"), None | Some(Value::Null)) {
        return no("creation");
    }
    let tx = match txform::read_envelope(&tx_value) {
        Ok(tx) => tx,
        Err(txform::TxFault::UndecodableEnvelope) => return Err(Fault::Form(format!("交易 {} 的信封解不了,签名人认不出(§9.1)", l.tx))),
        Err(txform::TxFault::Unreadable) => return Err(Fault::Form(format!("交易 {} 读不成", l.tx))),
    };
    let Some(sender) = tx.sender.clone() else { return no("no signer") };
    if tx.chain_id != Some(chain_id) {
        return no("another chain");
    }
    if l.topic_address(1).map(|a| !a.eq_ignore_ascii_case(&sender)).unwrap_or(true) {
        return no("topic 1 not the signer");
    }
    match ev.status(&l.tx)? {
        Some(true) => {}
        Some(false) => return no("status not 1"),
        None => return Err(Fault::Form(format!("交易 {} 的回执没有一家给出", l.tx))),
    }
    let Some(h) = l.topics.get(2).copied() else { return no("no topic 2") };
    let hash_hex = k1::hash::hex_encode(&h);
    if !eravon_galeed_scan::contained(&tx.input, &h) {
        return no("hash not in the calldata");
    }
    let time = ev.time(chain_id, block_hash, l.block)?;
    let v = verdict::decide(
        &mut |n| match ev.code_at(chain_id, block_hash, &sender, n) {
            Some(true) => verdict::Boundary::HasCode,
            Some(false) => verdict::Boundary::Codeless,
            None => verdict::Boundary::Unconsulted,
        },
        l.block,
    );
    let entry = eravon_galeed_scan::carried(&l.data, &h);
    Ok(crate::judged::Judged::Shows(k1::audit::Record {
        chain_id,
        block_number: l.block,
        block_timestamp: time,
        tx: l.tx.to_ascii_lowercase(),
        sender: sender.to_ascii_lowercase(),
        hash: hash_hex.to_ascii_lowercase(),
        verdict: v,
        entry,
    }))
}

#[allow(clippy::too_many_arguments)]
fn in_basis(ev: &dyn Evidence, chain_id: u64, w: &Window, l: &Log, listed: bool, judged: crate::judged::Judged, known: &std::collections::BTreeMap<(u64, String, u64), String>, new: &mut std::collections::BTreeMap<(u64, String, u64), String>) -> R<crate::judged::Judged> {
    let crate::judged::Judged::Shows(r) = &judged else { return Ok(judged) };
    if !sender_listed(w, &r.sender) {
        return Ok(crate::judged::Judged::Nothing("sender outside the basis".into()));
    }
    if !listed && !admitted(ev, w, l, chain_id, known, new)? {
        return Ok(crate::judged::Judged::Nothing("emitter outside the basis".into()));
    }
    Ok(judged)
}

fn admitted(ev: &dyn Evidence, w: &Window, l: &Log, chain_id: u64, known: &std::collections::BTreeMap<(u64, String, u64), String>, new: &mut std::collections::BTreeMap<(u64, String, u64), String>) -> R<bool> {
    let addr = l.address.to_ascii_lowercase();
    if w.registries.iter().any(|e| matches!(e, Emitter::Address(a) if *a == addr)) {
        return Ok(true);
    }
    let classes: Vec<&String> = w.registries.iter().filter_map(|e| match e {
        Emitter::Class(h) => Some(h),
        Emitter::Address(_) => None,
    }).collect();
    if classes.is_empty() {
        return Ok(false);
    }
    let at = (chain_id, addr.clone(), l.block);
    let h = match known.get(&at).or_else(|| new.get(&at)) {
        Some(h) => h.clone(),
        None => {
            let h = ev.code_hash_at(&addr, l.block)?.to_ascii_lowercase();
            new.insert(at, h.clone());
            h
        }
    };
    Ok(classes.iter().any(|c| c.eq_ignore_ascii_case(&h)))
}

pub(crate) fn record_value(r: &k1::audit::Record) -> Value {
    let mut m = vec![
        ("chainId".into(), Value::Int(r.chain_id)),
        ("blockNumber".into(), Value::Int(r.block_number)),
        ("blockTimestamp".into(), Value::Int(r.block_timestamp)),
        ("tx".into(), Value::Str(r.tx.clone())),
        ("sender".into(), Value::Str(r.sender.clone())),
        ("hash".into(), Value::Str(r.hash.clone())),
        ("verdict".into(), Value::Str(r.verdict.as_str().to_string())),
    ];
    if let Some(e) = &r.entry {
        m.push(("entry".into(), Value::Str(k1::hash::hex_encode(e))));
    }
    Value::Obj(m)
}

pub(crate) fn record_read(v: &Value) -> Option<k1::audit::Record> {
    let int = |k: &str| match v.get(k) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    };
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let entry = match v.get("entry") {
        None => None,
        Some(e) => Some(k1::hexform::decode_digits(e.as_str()?.strip_prefix("0x")?.as_bytes())?),
    };
    Some(k1::audit::Record {
        chain_id: int("chainId")?,
        block_number: int("blockNumber")?,
        block_timestamp: int("blockTimestamp")?,
        tx: text("tx")?,
        sender: text("sender")?,
        hash: text("hash")?,
        verdict: k1::audit::Verdict::parse(v.get("verdict")?.as_str()?)?,
        entry,
    })
}

fn input_of(root: &str, basis: &Value, records: &[k1::audit::Record]) -> Value {
    Value::Obj(vec![
        ("root".into(), Value::Str(root.to_ascii_lowercase())),
        ("anchors".into(), Value::Arr(records.iter().map(record_value).collect())),
        ("basis".into(), basis.clone()),
    ])
}

impl Ledger {
    pub fn audit_input(&self) -> Option<Vec<u8>> {
        k1::canon::canonical(&input_of(&self.root, &self.basis, &self.records)).ok()
    }
}

fn sender_listed(w: &Window, sender: &str) -> bool {
    w.senders.iter().any(|s| s.eq_ignore_ascii_case(sender))
}


fn kept(records: Vec<k1::audit::Record>) -> Vec<k1::audit::Record> {
    records
}


pub fn audited(root: &str, basis: Value, narrow: Vec<String>, records: Vec<k1::audit::Record>, not_anchors: Vec<NotAnchor>) -> Ledger {
    let records = kept(records);
    let input = input_of(root, &basis, &records);
    let read = k1::audit::read_input(&input);
    let report = read.as_ref().map(k1::audit::audit);
    let mut carried: Vec<(String, Vec<u8>, Value, u64, String)> = Vec::new();
    let mut by_order: Vec<&k1::audit::Record> = records.iter().collect();
    by_order.sort_by(|a, b| {
        let key = |r: &k1::audit::Record| (r.verdict != k1::audit::Verdict::Counted, r.block_number);
        (key(a), a.tx.as_bytes()).cmp(&(key(b), b.tx.as_bytes()))
    });
    for r in by_order {
        let Some(b) = &r.entry else { continue };
        let Ok(e) = k1::entry::accept(b) else { continue };
        let id = k1::hash::hex_encode(&e.entry_id);
        if !carried.iter().any(|(i, ..)| *i == id) {
            carried.push((id, b.clone(), e.value, r.block_number, r.tx.clone()));
        }
    }
    let lineage = match &read {
        Some(i) => k1::lineage::of(i),
        None => BTreeSet::from([root.to_ascii_lowercase()]),
    };
    let counted: BTreeSet<String> = report
        .as_ref()
        .and_then(|r| r.get("anchored"))
        .and_then(Value::as_arr)
        .map(|a| a.iter().filter_map(|x| x.get("entry_id").and_then(Value::as_str).map(str::to_ascii_lowercase)).collect())
        .unwrap_or_default();
    let mut entries: Vec<Held> = carried
        .into_iter()
        .filter(|(id, ..)| counted.contains(&id.to_ascii_lowercase()))
        .map(|(entry_id, bytes, value, block, tx)| Held { entry_id, bytes, value, block, tx })
        .collect();
    let seq = |h: &Held| match h.value.get("seq") {
        Some(Value::Int(n)) => *n,
        _ => 0,
    };
    entries.sort_by(|a, b| (seq(a), a.entry_id.as_bytes()).cmp(&(seq(b), b.entry_id.as_bytes())));
    let discarded = discarded_of(report.as_ref());
    Ledger { root: root.to_ascii_lowercase(), basis, narrow, records, not_anchors, emitters: Vec::new(), unchecked: Vec::new(), report, entries, lineage, discarded }
}

fn discarded_of(report: Option<&Value>) -> Vec<Discarded> {
    let str_of = |o: &Value, k: &str| o.get(k).and_then(Value::as_str).map(str::to_ascii_lowercase).unwrap_or_default();
    report
        .and_then(|r| r.get("discarded"))
        .and_then(Value::as_arr)
        .map(|a| {
            a.iter()
                .map(|o| Discarded {
                    entry_id: str_of(o, "hash"),
                    tx: str_of(o, "tx"),
                    block: match o.get("blockNumber") {
                        Some(Value::Int(n)) => *n,
                        _ => 0,
                    },
                    sender: str_of(o, "sender"),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(chain: u64, from: u64, to: u64) -> Window {
        Window { chain_id: chain, from, to, registries: vec![Emitter::Address(format!("0x{}", "aa".repeat(20)))], senders: vec![format!("0x{}", "bb".repeat(20))] }
    }

    fn corpus_input(name: &str) -> k1::audit::Input {
        let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base/galeed-conformance/corpus/audit").join(name);
        let raw = std::fs::read(&at).expect("the corpus case");
        let (mut out, mut in_str, mut esc) = (Vec::new(), false, false);
        for b in raw {
            if in_str {
                out.push(b);
                match (esc, b) {
                    (true, _) => esc = false,
                    (false, b'\\') => esc = true,
                    (false, b'"') => in_str = false,
                    _ => {}
                }
            } else if !b.is_ascii_whitespace() {
                in_str = b == b'"';
                out.push(b);
            }
        }
        let v = k1::canon::accept(&out).expect("canonical once its white space is dropped");
        k1::audit::read_input(&v).expect("a well-formed audit input")
    }

    #[test]
    fn the_records_the_audit_discards_are_carried_and_named() {
        let mut some = false;
        for name in ["a516_retired_fork_same_second.json", "a515_retired_fork_same_second_reversed.json", "a508_retire_stray_same_second.json", "a523_retired_key_late_succession_same_second_reversed.json"] {
            let input = corpus_input(name);
            let l = audited(&input.root, input.basis.clone(), Vec::new(), input.anchors.clone(), Vec::new());
            let reported: Vec<String> = l
                .report
                .as_ref()
                .and_then(|r| r.get("discarded"))
                .and_then(Value::as_arr)
                .map(|a| a.iter().filter_map(|o| o.get("hash").and_then(Value::as_str).map(str::to_ascii_lowercase)).collect())
                .unwrap_or_default();
            assert_eq!(l.discarded.iter().map(|d| d.entry_id.clone()).collect::<Vec<_>>(), reported, "{name}");
            for d in &l.discarded {
                assert!(l.entry(&d.entry_id).is_none(), "{name}: a discarded entry is no ledger entry");
                assert_eq!(l.discarded_entry(&d.entry_id), Some(d), "{name}");
                assert!(!d.tx.is_empty() && !d.sender.is_empty(), "{name}: said with where it was anchored");
            }
            some |= !l.discarded.is_empty();
        }
        assert!(some, "the same-second cases discard at least one record");
    }

    #[test]
    fn the_reach_widens_to_the_head_and_names_every_narrowing() {
        let (ws, narrow) = reach(&[w(1, 0, 10), w(1, 20, 30), w(5, 0, 9)], 1, 100);
        assert_eq!(ws.iter().map(|x| (x.from, x.to)).collect::<Vec<_>>(), vec![(0, 10), (20, 100)]);
        assert_eq!(narrow.len(), 1, "他链具名");
        let (ws, _) = reach(&[w(1, 0, 10)], 1, 100);
        assert_eq!(ws[0].from, 0, "自块 0 读,不抬到标准表的首块");
        let (ws, narrow) = reach(&[w(1, 0, 200)], 1, 100);
        assert_eq!((ws[0].to, narrow.len()), (100, 1), "头低于基底所写也说");
    }

    #[test]
    fn a_kept_anchor_is_judged_against_each_reads_own_basis() {
        struct Codes(std::cell::Cell<u32>);
        impl Evidence for Codes {
            fn tx(&self, _: u64, _: &str) -> R<Option<Value>> { unreachable!() }
            fn status(&self, _: &str) -> R<Option<bool>> { unreachable!() }
            fn time(&self, _: u64, _: &str, _: u64) -> R<u64> { unreachable!() }
            fn code_at(&self, _: u64, _: &str, _: &str, _: u64) -> Option<bool> { unreachable!() }
            fn code_hash_at(&self, _: &str, _: u64) -> R<String> {
                self.0.set(self.0.get() + 1);
                Ok(format!("0x{}", "cc".repeat(32)))
            }
        }
        let (signer, emitter, other) = (format!("0x{}", "bb".repeat(20)), format!("0x{}", "dd".repeat(20)), format!("0x{}", "ee".repeat(20)));
        let record = k1::audit::Record { chain_id: 1, block_number: 7, block_timestamp: 1, tx: "0x01".into(), sender: signer.clone(), hash: format!("0x{}", "11".repeat(32)), verdict: k1::audit::Verdict::Counted, entry: None };
        let kept = crate::judged::Judged::Shows(record);
        let log = Log { address: emitter.clone(), topics: Vec::new(), data: Vec::new(), block: 7, log_index: 0, tx: "0x01".into() };
        let ev = Codes(std::cell::Cell::new(0));
        let mut codes = std::collections::BTreeMap::new();
        let mut new = std::collections::BTreeMap::new();
        let basis = |senders: &[&String], reg: Vec<Emitter>| Window { chain_id: 1, from: 0, to: 9, registries: reg, senders: senders.iter().map(|s| s.to_string()).collect() };
        let shows = |j: &crate::judged::Judged| matches!(j, crate::judged::Judged::Shows(_));
        let why = |j: crate::judged::Judged| match j { crate::judged::Judged::Nothing(w) => w, crate::judged::Judged::Shows(_) => "shows".into() };
        let named = basis(&[&signer], vec![Emitter::Address(emitter.clone())]);
        assert!(shows(&in_basis(&ev, 1, &named, &log, true, kept.clone(), &codes, &mut new).expect("judged")));
        let others = basis(&[&other], vec![Emitter::Address(emitter.clone())]);
        assert_eq!(why(in_basis(&ev, 1, &others, &log, true, kept.clone(), &codes, &mut new).expect("judged")), "sender outside the basis");
        let elsewhere = basis(&[&signer], vec![Emitter::Address(other.clone())]);
        assert_eq!(why(in_basis(&ev, 1, &elsewhere, &log, false, kept.clone(), &codes, &mut new).expect("judged")), "emitter outside the basis");
        codes.insert((1, emitter.clone(), 7), format!("0x{}", "cc".repeat(32)));
        let class = basis(&[&signer], vec![Emitter::Class(format!("0x{}", "cc".repeat(32)))]);
        assert!(shows(&in_basis(&ev, 1, &class, &log, false, kept.clone(), &codes, &mut new).expect("judged")));
        assert_eq!(ev.0.get(), 0, "the kept code value is read, nothing asked");
    }

    #[test]
    fn a_basis_reads_back_to_itself() {
        let mut x = w(1, 0, 10);
        x.registries.push(Emitter::Class(format!("0x{}", "cc".repeat(32))));
        let v = basis_of(&[x.clone()]);
        assert_eq!(windows(&v), Some(vec![x]));
        assert!(k1::audit::read_input(&Value::Obj(vec![
            ("root".into(), Value::Str(format!("0x{}", "bb".repeat(20)))),
            ("anchors".into(), Value::Arr(Vec::new())),
            ("basis".into(), v),
        ])).is_some(), "核认这份基底合式");
    }

    #[derive(Clone)]
    struct TwoLogs;

    impl Transport for TwoLogs {
        fn post(&self, _url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<eravon_chain::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let result = if b.contains("eth_chainId") {
                "\"0x7a69\"".to_string()
            } else if b.contains("eth_getLogs") {
                let sender = crate::log::address_topic(&format!("0x{}", "bb".repeat(20))).unwrap();
                let row = |emitter: &str, block: u64, tx: &str| {
                    format!(r#"{{"address":"{emitter}","topics":["{}","{sender}","0x{}"],"data":"0x","blockNumber":"0x{block:x}","logIndex":"0x0","transactionHash":"{tx}","blockHash":"0x{}"}}"#, crate::scan::anchored_topic0(), "77".repeat(32), "99".repeat(32))
                };
                format!("[{},{}]", row(UNREAD, 3, &format!("0x{}", "01".repeat(32))), row(READ, 5, &format!("0x{}", "02".repeat(32))))
            } else {
                "null".to_string()
            };
            Ok(eravon_chain::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    const UNREAD: &str = "0x00000000000000000000000000000000000000c1";
    const READ: &str = "0x00000000000000000000000000000000000000c2";

    struct Kits {
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl Evidence for Kits {
        fn tx(&self, _chain_id: u64, _tx: &str) -> R<Option<Value>> {
            let mut a = self.asked.lock().unwrap();
            let which = if a.is_empty() { UNREAD } else { READ };
            a.push(which.to_string());
            if which == UNREAD { Err(Fault::Form("端点不供这一笔交易".into())) } else { Ok(None) }
        }
        fn status(&self, _tx: &str) -> R<Option<bool>> {
            Ok(None)
        }
        fn time(&self, _chain_id: u64, _block_hash: &str, _number: u64) -> R<u64> {
            Ok(0)
        }
        fn code_at(&self, _chain_id: u64, _block_hash: &str, _who: &str, _number: u64) -> Option<bool> {
            None
        }
        fn code_hash_at(&self, addr: &str, _block: u64) -> R<String> {
            if addr == UNREAD { Err(Fault::Form("端点不供该块的码".into())) } else { Ok(format!("0x{}", "cc".repeat(32))) }
        }
    }

    #[test]
    fn an_unread_fact_leaves_only_that_candidate_unjudged() {
        let table = Table::new(vec!["http://127.0.0.1:1".into()]).unwrap();
        let mut x = w(31_337, 0, 10);
        x.registries = vec![Emitter::Class(format!("0x{}", "cc".repeat(32)))];
        let ev = Kits { asked: std::sync::Mutex::new(Vec::new()) };
        let l = read(&TwoLogs, &table, &ev, 31_337, &format!("0x{}", "bb".repeat(20)), &basis_of(&[x]), 10).expect("一条码读不出,账照读");
        assert_eq!(ev.asked.into_inner().unwrap().len(), 2, "后一条照去读");
        assert_eq!(l.unchecked.len(), 2, "两条各具名不判");
        assert!(l.unchecked.iter().all(|n| n.reason.starts_with(UNCHECKED)), "{:?}", l.unchecked);
        assert!(l.unchecked.iter().any(|n| n.reason.contains("端点不供")), "{:?}", l.unchecked);
    }

    #[test]
    fn an_empty_ledger_is_audited_by_the_core() {
        let root = format!("0x{}", "bb".repeat(20));
        let l = audited(&root, basis_of(&[w(1, 0, 10)]), Vec::new(), Vec::new(), Vec::new());
        assert_eq!(l.label(), "COMPLETE");
        assert!(l.entries.is_empty());
        assert_eq!(l.lineage.iter().cloned().collect::<Vec<_>>(), vec![root]);
    }

    #[test]
    fn a_ledger_question_starts_at_the_networks_start_not_block_0() {
        let reg = "0x5da2894fc5260832330305f50f948401b3c904d8";
        let venue = "0xd30f401174af8b7149513cf3b20b695f33fd04fe";
        assert_eq!(query_from(11_155_111, Some(reg), 0), 11_936_067, "the registry's deployment");
        assert_eq!(query_from(11_155_111, None, 0), 11_936_067, "the class items: the network's start");
        assert_eq!(query_from(11_155_111, Some(venue), 0), 11_936_067, "a venue by address: the network's start");
        assert_eq!(query_from(11_155_111, Some(reg), 12_000_000), 12_000_000, "a later window start stands");
        assert_eq!(query_from(31_337, Some(venue), 0), 0, "a chain of no known network");
    }
}
