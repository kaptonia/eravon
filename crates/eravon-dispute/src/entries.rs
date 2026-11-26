use eravon_galeed as k1;
use eravon_scan as a3;
use eravon_standard as a1;
use eravon_watch::order::{self, Order};

use crate::{Why, Wire, a1w, case_of, small, status, why};

#[derive(Clone, Debug)]
pub struct EntryView {
    pub word: String,
    pub kind: String,
    pub signer: String,
    pub text: String,
    pub red: Option<String>,
    pub block: u64,
}

fn lineage(ledgers: &[(String, a3::ledger::Ledger)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, l) in ledgers {
        for a in &l.lineage {
            if !out.iter().any(|x| x.eq_ignore_ascii_case(a)) {
                out.push(a.clone());
            }
        }
    }
    out
}


fn ledger_of(w: &Wire<'_>, seated: &str, head: u64) -> Result<a3::ledger::Ledger, Why> {
    let ev = a3::evidence::OnChain { t: w.t, table: w.table };
    let class = eravon_literals::constants::active_standard().proxy_code_hash.to_ascii_lowercase();
    let book = a3::succession::logs(w.t, w.table).map_err(|e| why(&e.name(), e.detail()))?;
    let keys = eravon_galeed::book::record_keys(&book, seated);
    let root = keys.first().cloned().unwrap_or_else(|| seated.to_string()).to_ascii_lowercase();
    let root = root.as_str();
    let mut senders: Vec<String> = vec![root.to_string()];
    for k in keys {
        if !senders.iter().any(|s| s.eq_ignore_ascii_case(&k)) {
            senders.push(k.to_ascii_lowercase());
        }
    }
    loop {
        let ws = [a3::ledger::Window::to_head(w.chain_id, head, vec![a3::ledger::Emitter::Class(class.clone())], senders.clone())];
        let l = a3::ledger::read(w.t, w.table, &ev, w.chain_id, root, &a3::ledger::basis_of(&ws), head).map_err(|e| why(&e.name(), e.detail()))?;
        let more: Vec<String> = l.lineage.iter().filter(|k| !senders.iter().any(|s| s.eq_ignore_ascii_case(k))).cloned().collect();
        if more.is_empty() {
            return Ok(l);
        }
        senders.extend(more);
        senders.sort();
    }
}

fn in_lineage(signer: &str, set: &[String]) -> bool {
    set.iter().any(|j| j.eq_ignore_ascii_case(signer))
}

fn keep(v: EntryView) -> Option<EntryView> {
    Some(v)
}


fn judges(w: &Wire<'_>, o: &Order) -> Result<Vec<String>, Why> {
    let st = status(w, &o.instance)?;
    let seats = small(&st, eravon_standard::chain::status_col::SEATS)?;
    let case = case_of(o)?;
    let mut out: Vec<String> = Vec::new();
    a1::chain::ahead_seats(w.t, w.table, &(0..seats).map(|i| (o.instance.clone(), case, i)).collect::<Vec<_>>());
    for i in 0..seats {
        let s = a1::chain::seat_of(w.t, w.table, &o.instance, &case, i)
            .map_err(a1w)?
            .value;
        let j = a1::order::hex_of(
            &s.first()
                .ok_or_else(|| why("E_DISPUTE_READ", "seatOf 没有答 judge"))?
                .addr()
                .map_err(a1w)?,
        );
        if !out.iter().any(|x| x.eq_ignore_ascii_case(&j)) {
            out.push(j);
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn read(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    o: &Order,
    from: u64,
) -> Result<Vec<EntryView>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let js = judges(w, o)?;
    let (head, _) = eravon_chain::read::height(w.t, w.table).map_err(|e| why(&e.name(), e.detail()))?;
    let mut ledgers: Vec<(String, a3::ledger::Ledger)> = Vec::new();
    let held = std::sync::Arc::new(w.held());
    for (j, l) in js.iter().zip(eravon_chain::read::side_by_side(js.clone(), move |j| ledger_of(&held.wire(), &j, head))) {
        ledgers.push((j.clone(), l?));
    }
    let set = lineage(&ledgers);
    let mine = |l: &a3::ledger::Ledger, tx: &str| l.emitter_of(tx).is_some_and(|e| e.eq_ignore_ascii_case(&o.instance));
    let mut rows: Vec<(u64, String, Vec<u8>, Option<String>)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (_, l) in &ledgers {
        for r in l.records.iter().filter(|r| mine(l, &r.tx) && r.block_number >= from) {
            if seen.iter().any(|x| x.eq_ignore_ascii_case(&r.tx)) {
                continue;
            }
            seen.push(r.tx.clone());
            let red = match &r.entry {
                None => Some("携带形读不出条目字节(框、补零或 sha256 三样之一对不上)".to_string()),
                Some(b) if k1::entry::accept(b).is_err() => Some("条目过不了 galeed/1 §4.3 的信封测试".to_string()),
                Some(_) if l.discarded_entry(&r.hash).is_some() => Some(a3::ledger::DISCARDED_SAYS.to_string()),
                Some(_) => None,
            };
            rows.push((r.block_number, r.hash.clone(), r.entry.clone().unwrap_or_default(), red));
        }
        for n in l.not_anchors.iter().chain(&l.unchecked).filter(|n| n.emitter.eq_ignore_ascii_case(&o.instance) && n.block >= from) {
            if seen.iter().any(|x| x.eq_ignore_ascii_case(&n.tx)) {
                continue;
            }
            seen.push(n.tx.clone());
            rows.push((n.block, String::new(), Vec::new(), Some(format!("不是锚:{}", n.reason))));
        }
    }
    rows.sort_by_key(|r| r.0);
    let mut out = Vec::new();
    let mut parts = Vec::new();
    for (block, word, entry, red) in rows {
        let mut v = EntryView { word, kind: String::new(), signer: String::new(), text: String::from_utf8_lossy(&entry).to_string(), red, block };
        if v.red.is_none() {
            match k1::entry::accept(&entry) {
                Ok(en) => {
                    v.kind = en.value.get("entryType").and_then(k1::Value::as_str).unwrap_or_default().to_string();
                    v.signer = en.value.get("judge").and_then(k1::Value::as_str).unwrap_or_default().to_string();
                    if !in_lineage(&v.signer, &set) {
                        v.red = Some(format!("签名者 {} 不在判官世系(席上判官 {} 的世系:{})", v.signer, js.join("、"), set.join("、")));
                    }
                }
                Err(t) => v.red = Some(format!("记录格式无效:{}", t.name())),
            }
        }
        if let Some(v) = keep(v) {
            if !v.word.is_empty() {
                let short = v.word.trim_start_matches("0x").chars().take(16).collect::<String>();
                parts.push((format!("entry-{short}.bin"), entry.clone()));
            }
            out.push(v);
        }
    }
    let mut o2 = order::load(data, me, &o.id)?;
    o2.set_extra("entries", &out.len().to_string());
    o2.set_extra(
        "entriesRed",
        &out.iter().filter(|v| v.red.is_some()).count().to_string(),
    );
    order::save(data, writer, now, me, &o2, &parts)?;
    Ok(out)
}
