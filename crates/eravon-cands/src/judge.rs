use eravon_galeed::Value;
use eravon_roster::adapter::{self, JudgeReading, Logs};

use crate::fault::Absent;
use crate::quote;
use crate::registry;
use crate::wire::Wire;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerState {
    pub label: String,
    pub narrow: Vec<String>,
    pub entries: usize,
    pub in_lineage: bool,
    pub amanah_found: bool,
    pub amanah_discarded: bool,
    pub discarded: Vec<String>,
    pub unchecked: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub at: Option<u64>,
    pub label: String,
    pub instance: String,
    pub tx: String,
    pub red: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    pub block: u64,
    pub to: String,
}

pub fn attempts_of(id: &adapter::Identity) -> Vec<Attempt> {
    id.attempts.iter().map(|a| Attempt { block: a.block_number, to: a.to.clone() }).collect()
}

pub fn beyond(raw: &Value, keys: &[String]) -> Vec<(String, String)> {
    keys.iter().filter_map(|k| raw.get(k).map(|v| (k.clone(), eravon_galeed::canon::canonical(v).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_else(|t| t.name().to_string())))).collect()
}

#[derive(Clone, Debug)]
pub struct JudgeView {
    pub listed: String,
    pub address: String,
    pub trail: Vec<String>,
    pub attempts: Vec<Attempt>,
    pub word: String,
    pub bound: String,
    pub block: u64,
    pub published: Option<u64>,
    pub name: Option<String>,
    pub home: Option<String>,
    pub note_md: Option<String>,
    pub beyond: Vec<(String, String)>,
    pub amanah: String,
    pub root: String,
    pub version: Option<u64>,
    pub text: Option<String>,
    pub accepting: bool,
    pub ledger: Result<LedgerState, String>,
    pub quotes: Vec<quote::Row>,
    pub decimals: Vec<(String, Option<u64>)>,
    pub records: Option<Result<Vec<Record>, String>>,
    pub needs_resign: bool,
    pub resign_items: Vec<String>,
}

impl JudgeView {
    pub fn offers(&self) -> bool {
        self.ledger.as_ref().is_ok_and(|l| l.in_lineage && l.amanah_found)
    }

    pub fn selectable(&self) -> bool {
        self.accepting && self.offers() && self.quotes.iter().any(|q| q.usable)
    }

    pub fn law(&self) -> Result<u64, Unread> {
        let l = self.ledger.as_ref().map_err(|e| Unread::Ledger(e.clone()))?;
        if !l.in_lineage {
            return Err(Unread::Stranger);
        }
        if !l.amanah_found {
            return Err(if l.amanah_discarded { Unread::Discarded } else { Unread::NotInLedger });
        }
        self.version.ok_or(Unread::NoVersion)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unread {
    Ledger(String),
    Stranger,
    NotInLedger,
    Discarded,
    NoVersion,
}

impl Unread {
    pub fn name(&self) -> &'static str {
        match self {
            Unread::Ledger(_) => "ledger",
            Unread::Stranger => "stranger",
            Unread::NotInLedger => "notInLedger",
            Unread::Discarded => "discarded",
            Unread::NoVersion => "noVersion",
        }
    }
}

pub fn unverified(why: String) -> Absent {
    Absent::Unverified(why)
}


pub fn resign_due(signed: &str, fresh: &str) -> bool {
    !signed.is_empty() && !signed.eq_ignore_ascii_case(fresh)
}


pub fn open(w: &Wire, logs: &Logs, addr: &str, seen: &str) -> Result<JudgeView, Absent> {
    open_read(w, logs, addr, seen).map(|(v, _)| v)
}

pub fn name_of(logs: &Logs, addr: &str) -> Option<String> {
    latest(adapter::resolve_judge(logs, &addr.to_ascii_lowercase()).judge).ok().and_then(|(_, _, doc)| doc.name.clone())
}

fn latest(j: JudgeReading) -> Result<(u64, String, Box<eravon_roster::adapter::JudgeDoc>), Absent> {
    match j {
        JudgeReading::None => Err(Absent::NoDocument),
        JudgeReading::Unread(t) => Err(unverified(t.to_string())),
        JudgeReading::Mismatch => Err(unverified("E_JUDGE_MISMATCH 判官资料写的是别的地址".into())),
        JudgeReading::Judge { block_number, word, document } => Ok((block_number, word, document)),
    }
}

fn handed_or_latest(logs: &Logs, r: &adapter::ResolvedJudge) -> Result<(u64, String, Box<eravon_roster::adapter::JudgeDoc>), Absent> {
    match latest(r.judge.clone()) {
        Err(Absent::NoDocument) if !r.trail.is_empty() => {
            let block = logs.accessions.iter().filter(|x| x.emitter.eq_ignore_ascii_case(&r.final_addr)).map(|x| x.block_number).max().unwrap_or(0);
            Err(Absent::Succeeded { to: r.final_addr.clone(), block })
        }
        other => other,
    }
}

pub struct InCase {
    pub view: JudgeView,
    pub frozen: String,
    pub frozen_version: Option<u64>,
    pub frozen_text: Option<String>,
    pub frozen_discarded: bool,
    pub ledger: Result<eravon_scan::ledger::Ledger, String>,
}

pub fn frozen_in(l: &eravon_scan::ledger::Ledger, amanah_hash: &str) -> Option<(Option<u64>, Option<String>)> {
    let is_amanah = |v: &Value| v.get("entryType").and_then(Value::as_str) == Some(eravon_galeed::body::AMANAH);
    let of = |v: &Value| {
        let body = v.get("body");
        let version = body.and_then(|b| b.get("version")).and_then(|x| match x {
            Value::Int(n) => Some(*n),
            _ => None,
        });
        let text = body.and_then(|b| b.get("text_md")).and_then(Value::as_str).map(str::to_string);
        (version, text)
    };
    if let Some(e) = l.entry(amanah_hash).filter(|e| is_amanah(&e.value)) {
        return Some(of(&e.value));
    }
    let want = amanah_hash.trim_start_matches("0x").to_ascii_lowercase();
    l.entries.iter().rev().filter(|e| is_amanah(&e.value)).map(|e| of(&e.value)).find(|(_, text)| {
        text.as_deref().is_some_and(|t| eravon_galeed::hash::sha256(t.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>() == want)
    })
}

pub fn in_case(w: &Wire, logs: &Logs, addr: &str, amanah_hash: &str) -> Result<InCase, Absent> {
    let (view, ledger) = open_read(w, logs, addr, "")?;
    let (frozen_version, frozen_text) = ledger.as_ref().ok().and_then(|l| frozen_in(l, amanah_hash)).unwrap_or((None, None));
    let frozen_discarded = frozen_text.is_none() && ledger.as_ref().is_ok_and(|l| l.discarded_entry(amanah_hash).is_some());
    Ok(InCase { view, frozen: amanah_hash.to_ascii_lowercase(), frozen_version, frozen_text, frozen_discarded, ledger })
}

fn open_read(w: &Wire, logs: &Logs, addr: &str, seen: &str) -> Result<(JudgeView, Result<eravon_scan::ledger::Ledger, String>), Absent> {
    crate::mark::mark(crate::mark::FEATURE);
    let chain = eravon_roster::pool::standard_chain_id();
    let r = adapter::resolve_judge(logs, &addr.to_ascii_lowercase());
    let identity = adapter::resolve_identity(logs, &addr.to_ascii_lowercase());
    let (block, word, doc) = handed_or_latest(logs, &r)?;
    let read = read_ledger(w, chain, &doc.root, &doc.basis);
    let amanah = read.as_ref().ok().and_then(|l| l.entry(bound_of(&doc.entry_id, seen))).filter(|e| e.value.get("entryType").and_then(Value::as_str) == Some(eravon_galeed::body::AMANAH));
    let body = amanah.and_then(|e| e.value.get("body"));
    let version = body.and_then(|b| b.get("version")).and_then(|x| match x {
        Value::Int(n) => Some(*n),
        _ => None,
    });
    let text = body.and_then(|b| b.get("text_md")).and_then(Value::as_str).map(str::to_string);
    let quotes = if amanah.is_some() { quote::rows(body.and_then(|b| b.get("quotes"))) } else { Vec::new() };
    let decimals = decimals_list(w, &quotes);
    let ledger = read.as_ref().map(|l| LedgerState {
        label: l.label(),
        narrow: l.narrow.clone(),
        entries: l.entries.len(),
        in_lineage: in_lineage(l, &doc.judge),
        amanah_found: amanah.is_some(),
        amanah_discarded: amanah.is_none() && l.discarded_entry(bound_of(&doc.entry_id, seen)).is_some(),
        discarded: l.discarded.iter().map(|d| format!("{} · {}", d.entry_id, d.tx)).collect(),
        unchecked: l.unchecked.iter().map(|n| format!("{} #{}:{}", n.tx, n.log_index, n.reason)).collect(),
    }).map_err(|e| e.clone());
    let dated = |l: &eravon_scan::ledger::Ledger| {
        l.entries
            .iter()
            .filter(|e| matches!(e.value.get("entryType").and_then(Value::as_str), Some(t) if t == eravon_galeed::body::JUDGMENT || t == eravon_galeed::body::RESIGNATION))
            .filter(|e| !l.records.iter().any(|r| r.block_number == e.block && r.tx.eq_ignore_ascii_case(&e.tx)))
            .map(|e| e.block)
            .collect::<Vec<u64>>()
    };
    eravon_scan::scan::ahead_stamps(&w.t, &w.table, &std::iter::once(block).chain(read.as_ref().map(dated).unwrap_or_default()).collect::<Vec<u64>>());
    let records = Some(read.as_ref().map(|l| records_of(w, l)).map_err(|e| e.clone()));
    let view = JudgeView {
        listed: addr.to_ascii_lowercase(),
        address: r.final_addr.clone(),
        trail: r.trail.clone(),
        attempts: attempts_of(&identity),
        word: word.clone(),
        block,
        published: registry::stamp(w, block),
        name: doc.name.clone(),
        home: doc.home.clone(),
        note_md: doc.note_md.clone(),
        beyond: beyond(&doc.raw, &doc.unknown_fields),
        amanah: doc.entry_id.clone(),
        bound: doc.entry_id.clone(),
        root: doc.root.clone(),
        version,
        text,
        accepting: r.active,
        ledger,
        quotes,
        decimals,
        records,
        needs_resign: false,
        resign_items: Vec::new(),
    };
    Ok((view, read))
}

fn read_ledger(w: &Wire, chain: u64, root: &str, basis: &Value) -> Result<eravon_scan::ledger::Ledger, String> {
    let (head, _) = eravon_chain::read::height(&w.t, &w.table).map_err(|e| format!("{}:{}", e.name(), e.detail()))?;
    let ev = eravon_scan::evidence::OnChain { t: &w.t, table: &w.table };
    eravon_scan::ledger::read(&w.t, &w.table, &ev, chain, root, basis, head).map_err(|e| format!("{}:{}", e.name(), e.detail()))
}


fn bound_of<'a>(offered: &'a str, _seen: &'a str) -> &'a str {
    offered
}


pub fn in_lineage(l: &eravon_scan::ledger::Ledger, judge: &str) -> bool {
    l.lineage.iter().any(|k| k.eq_ignore_ascii_case(judge))
}


fn records_of(w: &Wire, l: &eravon_scan::ledger::Ledger) -> Vec<Record> {
    let mut out: Vec<Record> = l
        .entries
        .iter()
        .filter_map(|e| {
            let t = e.value.get("entryType").and_then(Value::as_str)?;
            let label = match t {
                t if t == eravon_galeed::body::JUDGMENT => "裁决".to_string(),
                t if t == eravon_galeed::body::RESIGNATION => "辞任".to_string(),
                _ => return None,
            };
            let held = l.records.iter().find(|r| r.block_number == e.block && r.tx.eq_ignore_ascii_case(&e.tx)).map(|r| r.block_timestamp);
            Some(Record { at: held.or_else(|| registry::stamp(w, e.block)), label, instance: l.emitter_of(&e.tx).unwrap_or_default().to_string(), tx: e.tx.clone(), red: None })
        })
        .collect();
    out.extend(l.not_anchors.iter().map(|n| Record { at: None, label: "(不是锚)".into(), instance: String::new(), tx: n.tx.clone(), red: Some(format!("核不认这条为锚:{}", n.reason)) }));
    out.extend(l.discarded.iter().map(|d| Record { at: registry::stamp(w, d.block), label: "(已丢弃)".into(), instance: l.emitter_of(&d.tx).unwrap_or_default().to_string(), tx: d.tx.clone(), red: Some(format!("{}:{}", d.entry_id, eravon_scan::ledger::DISCARDED_SAYS)) }));
    out
}

pub fn decimals_of(w: &Wire, token: &str) -> Option<u64> {
    let rows = eravon_vault::tokens::current();
    eravon_vault::tokens::decimals_of(&rows, eravon_vault::tokens::chain(), token, None)
        .map(u64::from)
        .or_else(|| eravon_standard::chain::erc20_decimals(&w.t, &w.table, token))
}

pub fn decimals_list(w: &Wire, quotes: &[crate::quote::Row]) -> Vec<(String, Option<u64>)> {
    let mut out: Vec<(String, Option<u64>)> = Vec::new();
    let units = quotes.iter().filter_map(|q| q.quote.as_ref().ok().filter(|x| !x.fee_unit_is_zero()).map(|x| x.fee_unit.clone()));
    let rows = eravon_vault::tokens::current();
    let unknown: Vec<String> = quotes
        .iter()
        .map(|q| q.token.clone())
        .chain(units.clone())
        .filter(|k| eravon_vault::tokens::decimals_of(&rows, eravon_vault::tokens::chain(), k, None).is_none())
        .collect();
    eravon_standard::chain::ahead_decimals(&w.t, &w.table, &unknown);
    for t in quotes.iter().map(|q| q.token.clone()).chain(units) {
        if !out.iter().any(|(k, _)| k.eq_ignore_ascii_case(&t)) {
            let d = decimals_of(w, &t);
            out.push((t, d));
        }
    }
    out
}

#[cfg(test)]
mod frozen_tests {
    use super::*;

    fn held(id: &str, json: &str) -> eravon_scan::ledger::Held {
        eravon_scan::ledger::Held { entry_id: id.into(), bytes: json.as_bytes().to_vec(), value: eravon_galeed::canon::read(json.as_bytes()).expect("json"), block: 1, tx: String::new() }
    }

    fn ledger(entries: Vec<eravon_scan::ledger::Held>) -> eravon_scan::ledger::Ledger {
        eravon_scan::ledger::Ledger {
            root: String::new(),
            basis: Value::Null,
            narrow: vec![],
            records: vec![],
            not_anchors: vec![],
            unchecked: vec![],
            emitters: vec![],
            report: None,
            entries,
            lineage: Default::default(),
            discarded: vec![],
        }
    }

    #[test]
    fn a_judge_handed_on_to_a_key_with_no_document_reads_succeeded() {
        use eravon_galeed::book::{Accession, Declaration};
        let (old, new) = (format!("0x{}", "a1".repeat(20)), format!("0x{}", "c2".repeat(20)));
        let doc = format!(r#"{{"amanah":{{"entry_id":"0x{}"}},"issued":1790000000,"judge":"{old}","kind":"galeed.judge/1","ledger":{{"basis":{{"chains":[]}},"root":"{old}"}},"name":"乙","seal":[1,2]}}"#, "11".repeat(32));
        let document = doc.into_bytes();
        let word = format!("0x{}", eravon_galeed::hash::keccak256(&document).iter().map(|x| format!("{x:02x}")).collect::<String>());
        let logs = Logs {
            publications: vec![eravon_roster::adapter::Publication { emitter: old.clone(), block_number: 10, log_index: 0, word, document }],
            statuses: vec![],
            declarations: vec![Declaration { emitter: old.clone(), block_number: 20, log_index: 0, to: new.clone() }],
            accessions: vec![Accession { emitter: new.clone(), block_number: 21, log_index: 0, holder: old.clone(), root: old.clone() }],
        };
        let r = adapter::resolve_judge(&logs, &old);
        assert_eq!(handed_or_latest(&logs, &r).err(), Some(Absent::Succeeded { to: new.clone(), block: 21 }));
        let declared_only = Logs { accessions: vec![], ..logs.clone() };
        assert!(handed_or_latest(&declared_only, &adapter::resolve_judge(&declared_only, &old)).is_ok(), "declared only: its own document");
        let none = Logs::default();
        assert_eq!(handed_or_latest(&none, &adapter::resolve_judge(&none, &old)).err(), Some(Absent::NoDocument));
    }

    #[test]
    fn a_case_reads_the_covenant_it_froze_on_either_track() {
        let a = r#"{"body":{"text_md":"第四版","version":4},"entryType":"amanah"}"#;
        let b = r#"{"body":{"text_md":"第五版","version":5},"entryType":"amanah"}"#;
        let g = r#"{"body":{"text_md":"行会约法","version":2},"entryType":"amanah"}"#;
        let l = ledger(vec![held("0xaa", a), held("0xbb", b), held("0xcc", g)]);
        assert_eq!(frozen_in(&l, "0xAA"), Some((Some(4), Some("第四版".to_string()))), "named track: the entry the case froze, not the latest");
        let sha: String = eravon_galeed::hash::sha256("行会约法".as_bytes()).iter().map(|x| format!("{x:02x}")).collect();
        assert_eq!(frozen_in(&l, &format!("0x{sha}")), Some((Some(2), Some("行会约法".to_string()))), "guild track: the member's copy of that text");
        assert_eq!(frozen_in(&l, &format!("0x{}", "11".repeat(32))), None);
        let not_amanah = ledger(vec![held("0xaa", r#"{"body":{"text_md":"x"},"entryType":"judgment"}"#)]);
        assert_eq!(frozen_in(&not_amanah, "0xaa"), None, "only an amanah entry is a covenant");
    }
}

#[cfg(test)]
mod law_tests {
    use super::*;

    fn view(ledger: Result<LedgerState, String>, version: Option<u64>) -> JudgeView {
        JudgeView {
            listed: String::new(),
            address: String::new(),
            trail: vec![],
            attempts: vec![],
            word: String::new(),
            bound: String::new(),
            block: 0,
            published: None,
            name: None,
            home: None,
            note_md: None,
            beyond: vec![],
            amanah: String::new(),
            root: String::new(),
            version,
            text: None,
            accepting: true,
            ledger,
            quotes: vec![],
            decimals: vec![],
            records: None,
            needs_resign: false,
            resign_items: vec![],
        }
    }

    fn state(in_lineage: bool, amanah_found: bool) -> LedgerState {
        LedgerState { label: "COMPLETE".into(), narrow: vec![], entries: 1, in_lineage, amanah_found, amanah_discarded: false, discarded: vec![], unchecked: vec![] }
    }

    #[test]
    fn the_covenant_version_is_read_or_said_why_not() {
        assert_eq!(view(Ok(state(true, true)), Some(4)).law(), Ok(4));
        assert_eq!(view(Err("E_CHAIN_TRANSPORT:x".into()), None).law(), Err(Unread::Ledger("E_CHAIN_TRANSPORT:x".into())));
        assert_eq!(view(Ok(state(false, true)), Some(4)).law(), Err(Unread::Stranger), "a stranger's card offers nothing, its version included");
        assert_eq!(view(Ok(state(true, false)), None).law(), Err(Unread::NotInLedger));
        assert_eq!(view(Ok(state(true, true)), None).law(), Err(Unread::NoVersion));
    }
}
