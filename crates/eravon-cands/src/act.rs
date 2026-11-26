use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed::Value;
use eravon_roster::adapter::{self, JudgeReading, Logs};
use eravon_roster::book::PanelBook;
use eravon_standard as a1;
use eravon_standard::order::Order;
use eravon_standard::precheck::{self, Priced};

pub mod gate {
    pub const STANDARD: &str = "standard";
    pub const JUDGE: &str = "judge";
    pub const LEDGER: &str = "ledger";
    pub const GUILD: &str = "guild";
    pub const PACE: &str = "pace";
    pub const QUOTE: &str = "quote";
    pub const GAS: &str = "gas";
}

pub struct Act<'a, T: Transport> {
    pub t: &'a T,
    pub table: &'a Table,
    pub logs: &'a Logs,
    pub book: &'a dyn PanelBook,
    pub ev: &'a dyn eravon_scan::ledger::Evidence,
    pub chain_id: u64,
    pub head: u64,
    pub now: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub gate: &'static str,
    pub name: String,
    pub why: String,
}

fn refuse(gate: &'static str, name: &str, why: impl Into<String>) -> Refused {
    Refused { gate, name: name.to_string(), why: why.into() }
}

impl From<a1::fault::Fault> for Refused {
    fn from(f: a1::fault::Fault) -> Self {
        let gate = match &f {
            a1::fault::Fault::Precheck { gate, .. } => *gate,
            _ => "chain",
        };
        Refused { gate, name: f.name().to_string(), why: f.detail() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seat {
    Named { judge: String, label: String, narrow: Vec<String> },
    Guild { guild: String, seq: u64, panel_hash: String, pool: usize, routed: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Passed {
    pub quote: Vec<u8>,
    pub priced: Priced,
    pub seat: Seat,
}

fn zero(a: &str) -> bool {
    a.trim_start_matches("0x").bytes().all(|b| b == b'0')
}

pub fn judge_party(judge: &str, parties: &[&str]) -> bool {
    parties.iter().any(|p| !zero(p) && p.eq_ignore_ascii_case(judge))
}

pub const E_JUDGE_PARTY: &str = "E_ACT_JUDGE_PARTY";

pub fn check<T: Transport>(a: &Act<'_, T>, o: &Order, logic: &str) -> Result<Passed, Refused> {
    crate::mark::mark(crate::mark::FEATURE);
    if !zero(&o.offer.judge) && judge_party(&o.offer.judge, &[&o.offer.seller_key, &o.buyer_key]) {
        return Err(refuse(gate::JUDGE, E_JUDGE_PARTY, format!("判官 {} 是这一单的当事人", o.offer.judge)));
    }
    standard(a, logic)?;
    let (quote, seat) = seat(a, &Candidate::of(o))?;
    let priced = precheck::priced(a.t, a.table, o, &quote)?;
    precheck::deposit(a.t, a.table, o, logic, &priced)?;
    precheck::not_revoked(a.t, a.table, o, logic)?;
    if matches!(seat, Seat::Guild { .. }) {
        let q = eravon_terms::quote::decode(&quote).map_err(|f| refuse(gate::QUOTE, "E_ACT_BAD_QUOTE", format!("{:?}", f.token)))?;
        crate::quote::paced(a.chain_id, &q).map_err(|why| refuse(gate::PACE, eravon_roster::pool::PoolRefusal::Pace.name(), why))?;
    }
    Ok(Passed { quote, priced, seat })
}

pub fn priced<T: Transport>(a: &Act<'_, T>, o: &Order, logic: &str) -> Result<Priced, Refused> {
    crate::mark::mark(crate::mark::FEATURE);
    standard(a, logic)?;
    let (quote, _) = seat(a, &Candidate::of(o))?;
    Ok(precheck::priced(a.t, a.table, o, &quote)?)
}

fn standard<T: Transport>(a: &Act<'_, T>, logic: &str) -> Result<(), Refused> {
    let row = eravon_literals::constants::active_standard();
    if !logic.eq_ignore_ascii_case(row.logic) {
        return Err(refuse(gate::STANDARD, "E_ACT_STANDARD_LOGIC", format!("逻辑地址 {logic} 不是标准表那一枚 {}", row.logic)));
    }
    let (code, _) = eravon_chain::read::code(a.t, a.table, logic, eravon_chain::seam::TAG_LATEST).map_err(|e| refuse(gate::STANDARD, e.name(), e.detail()))?;
    if code.is_empty() {
        return Err(refuse(gate::STANDARD, "E_ACT_STANDARD_NO_CODE", format!("逻辑地址 {logic} 上没有码")));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub judge: String,
    pub guild: String,
    pub amanah_hash: [u8; 32],
    pub token: String,
    pub parties: Vec<String>,
}

impl Candidate {
    pub fn of(o: &Order) -> Candidate {
        Candidate {
            judge: o.offer.judge.to_ascii_lowercase(),
            guild: o.offer.guild.to_ascii_lowercase(),
            amanah_hash: o.offer.amanah_hash,
            token: o.offer.token.to_ascii_lowercase(),
            parties: vec![o.offer.seller_key.to_ascii_lowercase(), o.buyer_key.to_ascii_lowercase()],
        }
    }
}

pub fn seat<T: Transport>(a: &Act<'_, T>, c: &Candidate) -> Result<(Vec<u8>, Seat), Refused> {
    if !zero(&c.judge) { named(a, c) } else { guild(a, c) }
}

pub fn guild_covenant(logs: &Logs, chain: u64, addr: &str) -> Result<(String, eravon_roster::adapter::CovenantFace), Refused> {
    let g = adapter::guild_of(logs, chain, &addr.to_ascii_lowercase()).map_err(|e| refuse(gate::GUILD, e.name(), "行会解析不出(§5.4)"))?;
    match g.covenant {
        adapter::CovenantReading::Covenant(c) => Ok((g.guild, *c)),
        adapter::CovenantReading::Stop(s) => Err(refuse(gate::GUILD, s, "行会公约读不出")),
        adapter::CovenantReading::None => Err(refuse(gate::GUILD, "E_POOL_NO_GUILDTERMS", "行会没有公约")),
    }
}

fn quote_under(quotes: Option<&Value>, chain: u64, token: &str) -> Result<Vec<u8>, Refused> {
    let key = adapter::quote_keys_standard(chain, eravon_roster::pool::standard_code_hash(), &token.to_ascii_lowercase())[0].clone();
    let v = quotes.and_then(|q| q.get(&key)).ok_or_else(|| refuse(gate::QUOTE, "E_ACT_NO_QUOTE", format!("键 {key} 下没有报价")))?;
    let s = v.as_str().filter(|s| adapter::quote_spelling(s)).ok_or_else(|| refuse(gate::QUOTE, "E_QUOTE_FORM", format!("键 {key} 下的报价不是小写 0x 十六进制")))?;
    eravon_galeed::hexform::decode_digits(&s.as_bytes()[2..]).ok_or_else(|| refuse(gate::QUOTE, "E_QUOTE_FORM", "报价不是偶数位十六进制"))
}

pub const E_ACT_AMANAH_DISCARDED: &str = "E_ACT_AMANAH_DISCARDED";

fn named<T: Transport>(a: &Act<'_, T>, c: &Candidate) -> Result<(Vec<u8>, Seat), Refused> {
    let judge = c.judge.clone();
    let r = adapter::resolve_judge(a.logs, &judge);
    let doc = match r.judge {
        JudgeReading::Judge { document, .. } => document,
        JudgeReading::None => return Err(refuse(gate::JUDGE, "E_ACT_JUDGE_NONE", format!("{} 没有判官资料", r.final_addr))),
        JudgeReading::Unread(t) => return Err(refuse(gate::JUDGE, t, "判官资料读不出")),
        JudgeReading::Mismatch => return Err(refuse(gate::JUDGE, "E_ACT_JUDGE_MISMATCH", "判官资料写的是别的地址")),
    };
    let want = offered(&doc.entry_id, r.active, &a1::order::hex_of(&c.amanah_hash))?;
    let l = eravon_scan::ledger::read(a.t, a.table, a.ev, a.chain_id, &doc.root, &doc.basis, a.head).map_err(|e| refuse(gate::LEDGER, &e.name(), e.detail()))?;
    if !crate::judge::in_lineage(&l, &doc.judge) {
        return Err(refuse(gate::LEDGER, "E_ACT_JUDGE_LINEAGE", format!("账本世系里没有 {}:这张资料不是此账本主人的", doc.judge)));
    }
    let entry = l
        .entry(&want)
        .filter(|e| e.value.get("entryType").and_then(Value::as_str) == Some(eravon_galeed::body::AMANAH))
        .ok_or_else(|| {
            if let Some(d) = l.discarded_entry(&want) {
                return refuse(gate::LEDGER, E_ACT_AMANAH_DISCARDED, format!("判法条目 {want} {}(交易 {},块 {})", eravon_scan::ledger::DISCARDED_SAYS, d.tx, d.block));
            }
            let unchecked = if l.unchecked.is_empty() { String::new() } else { format!(";另有 {} 条锚{}", l.unchecked.len(), eravon_scan::ledger::UNCHECKED) };
            refuse(gate::LEDGER, "E_ACT_AMANAH_UNANCHORED", format!("判法条目 {want} 不在审计出的账本里(标签 {}){unchecked}", l.label()))
        })?;
    let quote = quote_under(entry.value.get("body").and_then(|b| b.get("quotes")), a.chain_id, &c.token)?;
    Ok((quote, Seat::Named { judge: r.final_addr.clone(), label: l.label(), narrow: l.narrow.clone() }))
}

fn offered(offers: &str, active: bool, want: &str) -> Result<String, Refused> {
    if !active {
        return Err(refuse(gate::JUDGE, "E_ACT_JUDGE_PAUSED", "判官暂停接新案"));
    }
    if !offers.eq_ignore_ascii_case(want) {
        return Err(refuse(gate::JUDGE, "E_ACT_JUDGE_AMANAH", format!("判官资料现在供的是 {offers},不是本单的 {want}")));
    }
    Ok(want.to_string())
}


fn guild<T: Transport>(a: &Act<'_, T>, c: &Candidate) -> Result<(Vec<u8>, Seat), Refused> {
    let (_, cov) = guild_covenant(a.logs, a.chain_id, &c.guild)?;
    let cov = cov.covenant;
    let want = a1::order::hex_of(&c.amanah_hash);
    if !cov.sha256.eq_ignore_ascii_case(&want) {
        return Err(refuse(gate::GUILD, "E_ACT_GUILD_AMANAH", format!("行会公约正文的 sha256 是 {},不是本单的 {want}", cov.sha256)));
    }
    let pointer = adapter::pointer_string(a.chain_id, &c.guild.to_ascii_lowercase());
    let pc = eravon_roster::pool::pool_candidate(a.logs, a.book, a.chain_id, &pointer, &cov.text_md, &c.token, &c.parties, a.now, a.head.saturating_add(1))
        .map_err(|r| refuse(gate::GUILD, r.name(), "行会候选的池读法没过"))?;
    let quote = eravon_galeed::hexform::decode_digits(&pc.quote.as_bytes()[2..]).ok_or_else(|| refuse(gate::QUOTE, "E_QUOTE_FORM", "报价不是偶数位十六进制"))?;
    let routed = !zero(&pc.router);
    crate::gas::fits(pc.panel_len as u64, routed).map_err(|w| refuse(gate::GAS, "E_ACT_GAS", w))?;
    Ok((quote, Seat::Guild { guild: pc.guild, seq: pc.seq, panel_hash: pc.panel_hash, pool: pc.members, routed }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_judge_who_is_a_party_is_named() {
        let (s, b, j) = (format!("0x{}", "5a".repeat(20)), format!("0x{}", "d0".repeat(20)), format!("0x{}", "1c".repeat(20)));
        assert!(judge_party(&s.to_ascii_uppercase().replace("0X", "0x"), &[&s, &b]));
        assert!(judge_party(&b, &[&s, &b]));
        assert!(!judge_party(&j, &[&s, &b]));
        assert!(!judge_party(&format!("0x{}", "00".repeat(20)), &[&format!("0x{}", "00".repeat(20))]), "zero is no party");
    }
    use eravon_roster::adapter::Publication;

    fn hex(b: &[u8]) -> String {
        format!("0x{}", b.iter().map(|x| format!("{x:02x}")).collect::<String>())
    }

    fn published(emitter: &str, block: u64, doc: String) -> Publication {
        let document = doc.into_bytes();
        Publication { emitter: emitter.to_string(), block_number: block, log_index: 0, word: hex(&eravon_galeed::hash::keccak256(&document)), document }
    }

    fn roster(guild: &str) -> String {
        format!(r#"{{"chainId":1,"guild":"{guild}","issued":1790000000,"judges":[],"kind":"galeed.roster/1"}}"#)
    }

    fn covenant(guild: &str, text: &str) -> String {
        format!(r#"{{"chainId":1,"guild":"{guild}","issued":1790000000,"kind":"galeed.guildamanah/1","text_md":"{text}","version":1}}"#)
    }

    #[test]
    fn a_handed_over_guild_reads_the_covenant_at_its_final_address() {
        use eravon_galeed::book::{Accession, Declaration};
        let old = format!("0x{}", "a1".repeat(20));
        let new = format!("0x{}", "c2".repeat(20));
        let moved = || {
            (
                vec![Declaration { emitter: old.clone(), block_number: 11, log_index: 0, to: new.clone() }],
                vec![Accession { emitter: new.clone(), block_number: 11, log_index: 1, holder: old.clone(), root: old.clone() }],
            )
        };
        let (declarations, accessions) = moved();
        let logs = Logs {
            publications: vec![
                published(&old, 9, roster(&old)),
                published(&old, 10, covenant(&old, "old covenant")),
                published(&new, 12, roster(&new)),
                published(&new, 13, covenant(&new, "new covenant")),
            ],
            statuses: vec![],
            declarations,
            accessions,
        };
        let (at, c) = guild_covenant(&logs, 1, &old.to_ascii_uppercase().replacen("0X", "0x", 1)).expect("读得出");
        assert_eq!(at, new);
        assert_eq!(c.covenant.text_md, "new covenant");
        assert_eq!(c.block_number, 13);
        let (declarations, accessions) = moved();
        let bare = Logs { publications: logs.publications[..3].to_vec(), statuses: vec![], declarations, accessions };
        assert_eq!(guild_covenant(&bare, 1, &old).expect_err("该拒").name, "E_POOL_NO_GUILDTERMS");
        let (declarations, accessions) = moved();
        let no_roster = Logs { publications: logs.publications[..2].to_vec(), statuses: vec![], declarations, accessions };
        assert_eq!(guild_covenant(&no_roster, 1, &old).expect_err("该拒").name, "E_NO_DOCUMENT");
    }
}
