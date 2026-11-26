use std::collections::BTreeSet;

use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed::Value;
use eravon_roster::adapter::{self, JudgeReading, Logs};
use eravon_scan::ledger::{self, Emitter, Ledger, Window};

use crate::dossier::Case;
use crate::fault::{Fault, R};

pub enum Audit {
    Read { root: String, how: String, ledger: Box<Ledger> },
    Unread(String),
}

impl Audit {
    pub fn basis_text(&self) -> String {
        match self {
            Audit::Read { ledger, .. } => eravon_galeed::canon::canonical(&ledger.basis).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default(),
            Audit::Unread(_) => String::new(),
        }
    }
}

fn lower_sorted(v: impl IntoIterator<Item = String>) -> Vec<String> {
    let set: BTreeSet<String> = v.into_iter().map(|a| a.to_ascii_lowercase()).collect();
    set.into_iter().collect()
}

fn successors(l: &Ledger, senders: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for r in &l.records {
        let Some(b) = &r.entry else { continue };
        let Ok(e) = eravon_galeed::entry::accept(b) else { continue };
        if e.value.get("entryType").and_then(Value::as_str) != Some(eravon_galeed::body::SUCCESSION) {
            continue;
        }
        let judge = e.value.get("judge").and_then(Value::as_str).unwrap_or_default().to_ascii_lowercase();
        if !senders.contains(&judge) {
            continue;
        }
        if let Some(to) = e.value.get("body").and_then(|b| b.get("to")).and_then(Value::as_str) {
            out.push(to.to_ascii_lowercase());
        }
    }
    out
}

fn under<T: Transport>(t: &T, table: &Table, c: &Case, registries: &[String], root: &str, declared: &[Window], head: u64) -> R<Ledger> {
    let ev = eravon_scan::evidence::OnChain { t, table };
    let mut senders: Vec<String> = vec![root.to_string()];
    let logs = logs_for(t, table, registries, &[root.to_string()])?;
    senders.extend(eravon_galeed::book::record_keys(&logs, root));
    let id = adapter::resolve_identity(&logs, root);
    senders.push(id.final_addr);
    senders.extend(id.trail);
    senders.extend(c.seat.judge.clone());
    let mut regs: Vec<String> = registries.to_vec();
    for w in declared.iter().filter(|w| w.chain_id == c.chain_id) {
        senders.extend(w.senders.iter().cloned());
        regs.extend(w.registries.iter().filter_map(|e| match e {
            Emitter::Address(a) => Some(a.clone()),
            Emitter::Class(_) => None,
        }));
    }
    let mut classes: Vec<String> = vec![c.proxy_code_hash.to_ascii_lowercase()];
    for w in declared.iter().filter(|w| w.chain_id == c.chain_id) {
        classes.extend(w.registries.iter().filter_map(|e| match e {
            Emitter::Class(h) => Some(h.clone()),
            Emitter::Address(_) => None,
        }));
    }
    let regs = lower_sorted(regs);
    let classes = lower_sorted(classes);
    let mut senders = lower_sorted(senders);
    loop {
        let mut ws: Vec<Window> = declared.iter().filter(|w| w.chain_id != c.chain_id).cloned().collect();
        ws.push(Window::to_head(
            c.chain_id,
            head,
            regs.iter().cloned().map(Emitter::Address).chain(classes.iter().cloned().map(Emitter::Class)).collect(),
            senders.clone(),
        ));
        ws.sort_by_key(|w| (w.chain_id, w.from));
        let l = ledger::read(t, table, &ev, c.chain_id, root, &ledger::basis_of(&ws), head).map_err(Fault::from)?;
        let more: Vec<String> = successors(&l, &senders).into_iter().filter(|s| !senders.contains(s)).collect();
        if more.is_empty() {
            return Ok(l);
        }
        senders = lower_sorted(senders.into_iter().chain(more));
    }
}

pub fn audit<T: Transport>(t: &T, table: &Table, c: &Case, registries: &[String], head: u64) -> R<Audit> {
    let Some(judge) = c.seat.judge.as_deref().map(str::to_ascii_lowercase) else {
        return Ok(Audit::Unread("席位没有判官:无账本可审".into()));
    };
    let mut asked = vec![judge.clone()];
    let pool = c.pool();
    let guild = if pool { c.listing.guilds.get(c.receipt.candidate as usize - c.listing.judges.len()).map(|g| g.address.to_ascii_lowercase()) } else { None };
    asked.extend(guild.clone());
    let logs = logs_for(t, table, registries, &asked)?;
    let r = adapter::resolve_judge(&logs, &judge);
    if let JudgeReading::Judge { document, .. } = r.judge {
        let declared = ledger::windows(&document.basis).unwrap_or_default();
        let root = document.root.to_ascii_lowercase();
        let l = under(t, table, c, registries, &root, &declared, head)?;
        if l.lineage.contains(&judge) {
            return finish(root, format!("判官资料(最终址 {})的 ledger.root", r.final_addr), l, &judge);
        }
    }
    let root = if !pool {
        eravon_galeed::book::record_keys(&logs, &judge).into_iter().next().unwrap_or_else(|| judge.clone()).to_ascii_lowercase()
    } else {
        let Some(g) = guild else { return Ok(Audit::Unread("回执选的行会候选在清单里没有".into())) };
        let resolved = match adapter::guild_of(&logs, c.chain_id, &g) {
            Ok(x) => x,
            Err(e) => return Ok(Audit::Unread(format!("行会 {g} 解析不出({}):无从定根", e.name()))),
        };
        match pool_root(resolved.judges.iter().map(|row| eravon_galeed::book::record_keys(&logs, &row.listed.to_ascii_lowercase())), &judge) {
            Some(root) => root,
            None => return Ok(Audit::Unread(format!("行会 {g} 的名册里没有一行的记录含席位判官 {judge}:无从定根"))),
        }
    };
    let l = under(t, table, c, registries, &root, &[], head)?;
    let how = if pool { "行会名册里含席位判官那一行记录的首钥".to_string() } else { "席位判官记录的首钥".to_string() };
    finish(root, how, l, &judge)
}

fn pool_root(records: impl IntoIterator<Item = Vec<String>>, judge: &str) -> Option<String> {
    let keys = records.into_iter().find(|keys| keys.iter().any(|k| k.eq_ignore_ascii_case(judge)))?;
    keys.into_iter().next().map(|r| r.to_ascii_lowercase())
}

fn logs_for<T: Transport>(t: &T, table: &Table, registries: &[String], addrs: &[String]) -> R<Logs> {
    if registries.is_empty() || addrs.is_empty() {
        let book = eravon_scan::succession::logs(t, table)?;
        return Ok(Logs { declarations: book.declarations, accessions: book.accessions, ..Logs::default() });
    }
    eravon_cands::registry::logs_over(t, table, registries, addrs).map_err(|a| Fault::Form(format!("{}:{a:?}", a.name())))
}

fn finish(root: String, how: String, l: Ledger, judge: &str) -> R<Audit> {
    if l.report.is_none() {
        return Ok(Audit::Unread("审计输入不成 galeed/1 审计输入(NO_LABEL)".into()));
    }
    if !l.lineage.contains(judge) {
        return Ok(Audit::Unread(format!("以 {root} 为根算出的世系不含席位判官 {judge}")));
    }
    Ok(Audit::Read { root, how, ledger: Box::new(l) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    #[test]
    fn the_pool_root_is_the_first_key_of_the_record_holding_the_judge() {
        let records = || vec![rec(&["0xa0"]), rec(&["0xb0", "0xb1", "0xb2"])];
        assert_eq!(pool_root(records(), "0xB2"), Some("0xb0".into()));
        assert_eq!(pool_root(records(), "0xb0"), Some("0xb0".into()));
        assert_eq!(pool_root(records(), "0xa0"), Some("0xa0".into()));
        assert_eq!(pool_root(records(), "0xc0"), None);
    }
}
