use eravon_cands::act::{self, Act, Passed, Refused};
use eravon_standard::order::Order;

use crate::Wire;

pub const GATE_READ: &str = "chain";

fn read_refused(name: &str, why: String) -> Refused {
    Refused { gate: GATE_READ, name: name.to_string(), why }
}

pub fn registries(given: &[String]) -> Result<Vec<String>, Refused> {
    eravon_cands::wire::registries(given).map_err(|a| read_refused(a.name(), a.detail()))
}

pub fn check(w: &Wire<'_>, o: &Order) -> Result<Passed, Refused> {
    crate::mark::mark(crate::mark::FEATURE);
    let regs = registries_of(w.registries)?;
    eravon_standard::chain::ahead_act(w.t, w.table, o, w.logic);
    let (head, _) = eravon_chain::read::height(w.t, w.table).map_err(|e| read_refused(e.name(), e.detail()))?;
    let now = eravon_scan::scan::block_stamp(w.t, w.table, head).map_err(|e| read_refused(&e.name(), e.detail()))?;
    let zero = |a: &str| a.trim_start_matches("0x").bytes().all(|b| b == b'0');
    let asked: Vec<String> = [o.offer.judge.clone(), o.offer.guild.clone()].into_iter().filter(|a| !zero(a)).collect();
    let logs = eravon_cands::registry::logs_over(w.t, w.table, &regs, &asked).map_err(|a| read_refused(a.name(), a.detail()))?;
    let ev = eravon_scan::evidence::OnChain { t: w.t, table: w.table };
    let book = eravon_cands::book::ChainBook { t: w.t, table: w.table, logic: w.logic };
    act::check(&Act { t: w.t, table: w.table, logs: &logs, book: &book, ev: &ev, chain_id: w.chain_id, head, now }, o, w.logic)
}

pub fn priced(w: &Wire<'_>, o: &Order) -> Result<eravon_standard::precheck::Priced, Refused> {
    crate::mark::mark(crate::mark::FEATURE);
    let regs = registries_of(w.registries)?;
    let (head, _) = eravon_chain::read::height(w.t, w.table).map_err(|e| read_refused(e.name(), e.detail()))?;
    let now = eravon_scan::scan::block_stamp(w.t, w.table, head).map_err(|e| read_refused(&e.name(), e.detail()))?;
    let zero = |a: &str| a.trim_start_matches("0x").bytes().all(|b| b == b'0');
    let asked: Vec<String> = [o.offer.judge.clone(), o.offer.guild.clone()].into_iter().filter(|a| !zero(a)).collect();
    let logs = eravon_cands::registry::logs_over(w.t, w.table, &regs, &asked).map_err(|a| read_refused(a.name(), a.detail()))?;
    let ev = eravon_scan::evidence::OnChain { t: w.t, table: w.table };
    let book = eravon_cands::book::ChainBook { t: w.t, table: w.table, logic: w.logic };
    act::priced(&Act { t: w.t, table: w.table, logs: &logs, book: &book, ev: &ev, chain_id: w.chain_id, head, now }, o, w.logic)
}

pub fn opening_slots(w: &Wire<'_>, o: &Order, sig_seller: &[u8], sig_buyer: &[u8]) -> eravon_standard::fault::R<(Vec<u8>, Vec<u8>)> {
    eravon_standard::precheck::slots(o, w.chain_id, w.logic, sig_seller, sig_buyer, &mut |k: &str| eravon_standard::chain::current(w.t, w.table, k).map(|r| r.value))
}

pub fn opening(w: &Wire<'_>, o: &Order, instance: &str, sig_seller: &[u8], sig_buyer: &[u8], now: u64, from: &str) -> eravon_standard::fault::R<()> {
    use eravon_standard as a1;
    let mut reg = |who: &str, d: &[u8; 32]| a1::consent::registered(w.t, w.table, w.logic, who, d);
    let mut current = |k: &str| a1::chain::current(w.t, w.table, k).map(|r| r.value);
    a1::precheck::consents(o, w.chain_id, w.logic, sig_seller, sig_buyer, a1::consent::Sender { from: Some(from), via_multicall: false }, &mut current, &mut reg)?;
    a1::precheck::funded(w.t, w.table, o, instance)?;
    a1::precheck::in_window(o.offer.expiry.min(o.open_by), now)?;
    Ok(())
}

fn registries_of(given: &[String]) -> Result<Vec<String>, Refused> {
    registries(given)
}

pub fn why_of(r: &Refused) -> crate::Why {
    (r.name.clone(), format!("{}:{}", r.gate, r.why))
}

pub fn quote(w: &Wire<'_>, c: &eravon_cands::act::Candidate, quote_hash: &str) -> Result<eravon_terms::quote::Quote, crate::Why> {
    let bytes = quote_bytes(w, c, quote_hash)?;
    eravon_terms::quote::decode(&bytes).map_err(|f| ("E_ACT_QUOTE".to_string(), format!("报价读不成:{:?}", f.token)))
}

pub fn quote_bytes(w: &Wire<'_>, c: &eravon_cands::act::Candidate, quote_hash: &str) -> Result<Vec<u8>, crate::Why> {
    let regs = registries_of(w.registries).map_err(|r| why_of(&r))?;
    let (head, _) = eravon_chain::read::height(w.t, w.table).map_err(|e| (e.name().to_string(), e.detail()))?;
    let now = eravon_scan::scan::block_stamp(w.t, w.table, head).map_err(|e| (e.name(), e.detail()))?;
    let zero = |a: &str| a.trim_start_matches("0x").bytes().all(|b| b == b'0');
    let asked: Vec<String> = [c.judge.clone(), c.guild.clone()].into_iter().filter(|a| !zero(a)).collect();
    let logs = eravon_cands::registry::logs_over(w.t, w.table, &regs, &asked).map_err(|a| (a.name().to_string(), a.detail()))?;
    let ev = eravon_scan::evidence::OnChain { t: w.t, table: w.table };
    let book = eravon_cands::book::ChainBook { t: w.t, table: w.table, logic: w.logic };
    let a = act::Act { t: w.t, table: w.table, logs: &logs, book: &book, ev: &ev, chain_id: w.chain_id, head, now };
    let (bytes, _) = act::seat(&a, c).map_err(|r| why_of(&r))?;
    if !eravon_standard::order::hex_of(&eravon_terms::derive::keccak(&bytes)).eq_ignore_ascii_case(quote_hash) {
        return Err(("E_ACT_QUOTE".to_string(), "候选现读出的报价与文书的 quoteHash 不合:报价已变更".to_string()));
    }
    Ok(bytes)
}
