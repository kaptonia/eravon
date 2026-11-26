use eravon_galeed as k1;

use crate::adapter::{self, Guild, Logs};
use crate::book::{Engaged, PanelBook, PanelRecord};
use crate::mark;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateForm {
    Named(String),
    GuildAsSet(String),
    GuildAsPool(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PoolRefusal {
    Pointer,
    Roster,
    RosterEmpty,
    NoGuildTerms,
    TermsDigest,
    NoQuote,
    QuoteForm,
    NoPanel,
    BookUnread,
    PanelHash,
    PanelExpired,
    PanelWord,
    PanelEmpty,
    Pace,
}

impl PoolRefusal {
    pub fn name(self) -> &'static str {
        match self {
            PoolRefusal::Pointer => "E_POOL_POINTER",
            PoolRefusal::Roster => "E_POOL_ROSTER",
            PoolRefusal::RosterEmpty => "E_POOL_ROSTER_EMPTY",
            PoolRefusal::NoGuildTerms => "E_POOL_NO_GUILDTERMS",
            PoolRefusal::TermsDigest => "E_POOL_TERMS_DIGEST",
            PoolRefusal::NoQuote => "E_POOL_NO_QUOTE",
            PoolRefusal::QuoteForm => "E_POOL_QUOTE_FORM",
            PoolRefusal::NoPanel => "E_POOL_NO_PANEL",
            PoolRefusal::BookUnread => "E_POOL_BOOK_UNREAD",
            PoolRefusal::PanelHash => "E_POOL_PANEL_HASH",
            PoolRefusal::PanelExpired => "E_POOL_PANEL_EXPIRED",
            PoolRefusal::PanelWord => "E_POOL_PANEL_WORD",
            PoolRefusal::PanelEmpty => "E_POOL_PANEL_EMPTY",
            PoolRefusal::Pace => "E_POOL_PACE",
        }
    }
}

pub fn paced_interval(chain: u64) -> Option<u64> {
    eravon_literals::constants::NETWORKS.iter().find(|n| n.standard.chain_id == chain && n.consensus_hashes).map(|n| n.block_secs)
}

pub const SEED_BLOCKS: u64 = 256;

pub fn pace(chain: u64, offer_window: u64) -> Result<(), PoolRefusal> {
    match paced_interval(chain) {
        Some(interval) if offer_window >= SEED_BLOCKS.saturating_mul(interval) => Ok(()),
        _ => Err(PoolRefusal::Pace),
    }
}

#[derive(Clone, Debug)]
pub struct PoolCandidate {
    pub guild: String,
    pub root: String,
    pub router: String,
    pub amanah_hash: String,
    pub quote: String,
    pub seq: u64,
    pub panel_hash: String,
    pub members: usize,
    pub panel_len: usize,
    pub expires: u64,
}

pub fn panel_encoding(members: &[String], amanah_hashes: &[String]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let head = 64usize;
    let first = 32 + 32 * members.len();
    out.extend_from_slice(&word(head as u64));
    out.extend_from_slice(&word((head + first) as u64));
    out.extend_from_slice(&word(members.len() as u64));
    for m in members {
        if !k1::hexform::is_hex20(m) {
            return None;
        }
        let mut w = [0u8; 32];
        w[12..].copy_from_slice(&k1::hexform::decode_form(m));
        out.extend_from_slice(&w);
    }
    out.extend_from_slice(&word(amanah_hashes.len() as u64));
    for h in amanah_hashes {
        if !k1::hexform::is_hex32(h) {
            return None;
        }
        out.extend_from_slice(&k1::hexform::decode_form(h));
    }
    Some(out)
}


fn word(x: u64) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[24..].copy_from_slice(&x.to_be_bytes());
    w
}

pub fn panel_hash(members: &[String], amanah_hashes: &[String]) -> Option<String> {
    Some(k1::hash::hex_encode(&k1::hash::keccak256(&panel_encoding(members, amanah_hashes)?)))
}

pub fn standard_code_hash() -> &'static str {
    eravon_literals::constants::active_standard().proxy_code_hash
}

pub fn standard_chain_id() -> u64 {
    eravon_literals::constants::active_standard().chain_id
}

pub fn candidate_set(
    logs: &Logs,
    chain: u64,
    pointers: &[String],
    allow: &[String],
    parties: &[String],
) -> adapter::Candidates {
    mark::mark(mark::FEATURE);
    adapter::candidates(logs, chain, pointers, allow, parties_of(parties))
}

fn parties_of(parties: &[String]) -> &[String] {
    parties
}


fn terms_digest_ok(declared: &str, text: &str) -> bool {
    k1::hash::hex_encode(&k1::hash::sha256(text.as_bytes())) == declared
}


fn quote_of<'a>(quotes: &'a k1::Value, chain: u64, code_hash: &str, token: &str) -> Option<&'a k1::Value> {
    let keys = adapter::quote_keys_standard(chain, code_hash, token);
    quotes.get(&keys[0])
}


pub fn panel_matches(e: &Engaged, panel_hash: &str) -> bool {
    panel_hash_of(e).is_some_and(|h| h.eq_ignore_ascii_case(panel_hash))
}

fn panel_hash_of(e: &Engaged) -> Option<String> {
    panel_hash(&e.members, &e.amanah_hashes)
}

pub fn panel_ok(rec: &PanelRecord, e: &Engaged, now: u64, word: &str, parties: &[String]) -> Result<(), PoolRefusal> {
    if !panel_matches(e, &rec.panel_hash) {
        return Err(PoolRefusal::PanelHash);
    }
    if !window_open(rec.expires, now) {
        return Err(PoolRefusal::PanelExpired);
    }
    if e.amanah_hashes.iter().any(|w| !w.eq_ignore_ascii_case(word)) {
        return Err(PoolRefusal::PanelWord);
    }
    if !routed(&rec.router) && pool_size(&e.members, parties) == 0 {
        return Err(PoolRefusal::PanelEmpty);
    }
    Ok(())
}

pub fn routed(router: &str) -> bool {
    !router.trim_start_matches("0x").bytes().all(|b| b == b'0')
}

pub fn frozen(book: &dyn PanelBook, root: &str, terms_hash: &str, now: u64, block: u64) -> Result<Option<PanelRecord>, String> {
    let mut n = book.count(root, terms_hash)?;
    while n != 0 {
        n -= 1;
        let r = book.panel_at(root, terms_hash, n)?.ok_or_else(|| format!("簿上 count 之内的第 {n} 条读不到"))?;
        if r.recorded < block && window_open(r.expires, now) {
            return Ok(Some(r));
        }
    }
    Ok(None)
}

fn none_frozen(book: &dyn PanelBook, root: &str, terms_hash: &str, block: u64) -> PoolRefusal {
    match book.count(root, terms_hash) {
        Ok(n) if (0..n).any(|i| book.panel_at(root, terms_hash, i).ok().flatten().is_some_and(|r| r.recorded < block)) => PoolRefusal::PanelExpired,
        Ok(_) => PoolRefusal::NoPanel,
        Err(_) => PoolRefusal::BookUnread,
    }
}

pub fn pool_size(members: &[String], parties: &[String]) -> usize {
    let zero = |m: &str| m.trim_start_matches("0x").bytes().all(|b| b == b'0');
    members
        .iter()
        .enumerate()
        .filter(|(i, m)| !zero(m) && !parties_of(parties).iter().any(|p| p.eq_ignore_ascii_case(m)) && !members[..*i].iter().any(|x| x.eq_ignore_ascii_case(m)))
        .count()
}

fn window_open(expires: u64, now: u64) -> bool {
    expires >= now
}



#[allow(clippy::too_many_arguments)]
pub fn pool_candidate(
    logs: &Logs,
    book: &dyn PanelBook,
    chain: u64,
    pointer: &str,
    guild_terms_text: &str,
    token: &str,
    parties: &[String],
    now: u64,
    block: u64,
) -> Result<PoolCandidate, PoolRefusal> {
    mark::mark(mark::FEATURE);
    let code_hash = standard_code_hash();
    let Some((c, addr)) = adapter::pointer(pointer) else {
        return Err(PoolRefusal::Pointer);
    };
    if c != chain {
        return Err(PoolRefusal::Pointer);
    }
    let g: Guild = adapter::guild_of(logs, chain, addr).map_err(|_| PoolRefusal::Roster)?;
    if g.roster.judges.is_empty() {
        return Err(PoolRefusal::RosterEmpty);
    }

    let cov = match g.current_covenant() {
        Some(c) => &c.covenant,
        None => return Err(PoolRefusal::NoGuildTerms),
    };
    let declared = cov.sha256.as_str();
    if !terms_digest_ok(declared, guild_terms_text) {
        return Err(PoolRefusal::TermsDigest);
    }

    let quotes = cov.quotes.as_ref().ok_or(PoolRefusal::NoQuote)?;
    let quote = match quote_of(quotes, chain, code_hash, token).ok_or(PoolRefusal::NoQuote)?.as_str() {
        Some(q) if adapter::quote_spelling(q) => q.to_string(),
        _ => return Err(PoolRefusal::QuoteForm),
    };

    let venue = addr.to_ascii_lowercase();
    let root = book.root_of(&venue).map_err(|_| PoolRefusal::BookUnread)?.to_ascii_lowercase();
    let rec = match frozen(book, &root, code_hash, now, block).map_err(|_| PoolRefusal::BookUnread)? {
        Some(r) => r,
        None => return Err(none_frozen(book, &root, code_hash, block)),
    };
    let e = book.engaged(&root, code_hash, rec.index).map_err(|_| PoolRefusal::BookUnread)?.ok_or(PoolRefusal::NoPanel)?;
    panel_ok(&rec, &e, now, declared, parties)?;

    Ok(PoolCandidate {
        guild: venue,
        root,
        router: rec.router.clone(),
        amanah_hash: declared.to_string(),
        quote,
        seq: rec.index,
        panel_hash: rec.panel_hash.clone(),
        members: pool_size(&e.members, parties),
        panel_len: e.members.len(),
        expires: rec.expires,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(n: u8) -> String {
        format!("0x{}", format!("{n:02x}").repeat(20))
    }
    fn w(n: u8) -> String {
        format!("0x{}", format!("{n:02x}").repeat(32))
    }

    #[test]
    fn the_pace_needs_a_consensus_chain_and_a_window_of_256_blocks() {
        assert_eq!(pace(1, 256 * 12), Ok(()));
        assert_eq!(pace(11_155_111, 256 * 12 + 1), Ok(()));
        assert_eq!(pace(1, 256 * 12 - 1), Err(PoolRefusal::Pace));
        assert_eq!(pace(31_337, u64::MAX), Err(PoolRefusal::Pace), "块哈希不出于共识的链不给行会道");
        assert_eq!(pace(1, 0), Err(PoolRefusal::Pace));
    }

    #[test]
    fn the_pool_is_the_panel_less_the_parties_and_zero() {
        let zero = format!("0x{}", "0".repeat(40));
        assert_eq!(pool_size(&[a(1), a(2)], &[a(1).to_ascii_uppercase().replace("0X", "0x")]), 1, "比对不分大小写");
        assert_eq!(pool_size(&[a(1), zero.clone()], &[a(1)]), 0);
        assert_eq!(pool_size(&[], &[]), 0);
        assert_eq!(pool_size(&[a(1), a(2), a(3)], &[a(9)]), 3);
        assert_eq!(pool_size(&[a(1), a(2), a(1)], &[]), 2, "同一成员列两次只算一次");
    }

    fn engaged(members: Vec<String>, words: Vec<String>) -> (String, Engaged) {
        let h = panel_hash(&members, &words).expect("合式名单");
        (h, Engaged { index: 0, members, amanah_hashes: words, roster_hash: w(9), expires: 100, router: a(0) })
    }

    fn rec(h: &str, router: String) -> PanelRecord {
        PanelRecord { index: 0, panel_hash: h.to_string(), roster_hash: w(9), expires: 100, recorded: 1, router, router_hash: w(0) }
    }

    #[test]
    fn a_panel_passes_only_the_tests_init_runs() {
        let (h, e) = engaged(vec![a(1), a(2)], vec![w(7), w(7)]);
        assert_eq!(panel_ok(&rec(&h, a(0)), &e, 100, &w(7), &[a(1)]), Ok(()), "到期那一秒仍可冻结");
        assert_eq!(panel_ok(&rec(&w(5), a(0)), &e, 50, &w(7), &[]), Err(PoolRefusal::PanelHash));
        assert_eq!(panel_ok(&rec(&h, a(0)), &e, 101, &w(7), &[]), Err(PoolRefusal::PanelExpired));
        assert_eq!(panel_ok(&rec(&h, a(0)), &e, 50, &w(8), &[]), Err(PoolRefusal::PanelWord));
        assert_eq!(panel_ok(&rec(&h, a(0)), &e, 50, &w(7), &[a(1), a(2)]), Err(PoolRefusal::PanelEmpty));
        assert_eq!(panel_ok(&rec(&h, a(3)), &e, 50, &w(7), &[a(1), a(2)]), Ok(()), "记录点名路由器时池可空");
        let (h2, e2) = engaged(vec![a(1), a(2)], vec![w(7), w(8)]);
        assert_eq!(panel_ok(&rec(&h2, a(0)), &e2, 50, &w(7), &[]), Err(PoolRefusal::PanelWord), "一词不同即拒");
    }

    #[test]
    fn a_redraw_panel_is_the_one_that_hashes_to_the_frozen_hash() {
        let (h, e) = engaged(vec![a(1), a(2)], vec![w(7), w(7)]);
        assert!(panel_matches(&e, &h));
        assert!(panel_matches(&e, &h.to_ascii_uppercase().replacen("0X", "0x", 1)), "比对不分大小写");
        assert!(!panel_matches(&e, &w(5)), "别的摘要不对");
        let bad = Engaged { members: vec!["0x12".into()], ..e.clone() };
        assert!(!panel_matches(&bad, &h), "成员形不合的名单对不上任何摘要");
    }

    #[test]
    fn the_frozen_record_walks_back_past_late_and_expired() {
        let doc = |rows: &str| crate::book::FixtureBook::from_bytes(format!("{{\"records\":[{rows}]}}").as_bytes()).expect("夹具");
        let row = |i: u64, exp: u64, rec: u64| format!(
            "{{\"root\":\"{}\",\"termsHash\":\"{}\",\"index\":{i},\"members\":[\"{}\"],\"amanahHashes\":[\"{}\"],\"rosterHash\":\"{}\",\"expires\":{exp},\"recorded\":{rec}}}",
            a(1), w(2), a(3), w(4), w(5));
        let b = doc(&[row(0, 1000, 5), row(1, 50, 6), row(2, 1000, 10)].join(","));
        let at = |now, block| frozen(&b, &a(1), &w(2), now, block).expect("读得到").map(|r| r.index);
        assert_eq!(at(100, 11), Some(2), "最新一条已登记、未过期");
        assert_eq!(at(100, 10), Some(0), "第 2 条与本块同块未登记、第 1 条已过期:回到第 0 条");
        assert_eq!(at(40, 10), Some(1), "第 1 条在到期之前");
        assert_eq!(at(2000, 11), None);
        assert_eq!(none_frozen(&b, &a(1), &w(2), 11), PoolRefusal::PanelExpired);
        assert_eq!(none_frozen(&b, &a(9), &w(2), 11), PoolRefusal::NoPanel);
    }
}
