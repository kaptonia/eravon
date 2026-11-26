use eravon_cands::fault::Absent;
use eravon_cands::guild;
use eravon_cands::judge::{self, JudgeView};
use eravon_cands::table::Side;
use eravon_cands::wire::Wire;
use eravon_roster::adapter::Logs;
use eravon_store::record::Candidate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeEl {
    pub judge: String,
    pub amanah_hash: String,
    pub quote: String,
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildEl {
    pub pointer: String,
    pub amanah_hash: String,
    pub quote: String,
    pub seq: u64,
    pub panel_hash: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Excluded {
    pub address: String,
    pub name: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resolved {
    pub judges: Vec<JudgeEl>,
    pub guilds: Vec<GuildEl>,
    pub excluded: Vec<Excluded>,
    pub fees: Vec<Fee>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fee {
    pub candidate: String,
    pub quote: String,
    pub token: String,
    pub decimals: Option<u64>,
    pub amount: eravon_terms::u256::U256,
    pub priced: Result<eravon_standard::precheck::Quoted, (String, String)>,
}

impl Fee {
    pub fn same_terms(&self, other: &Fee) -> bool {
        self.candidate.eq_ignore_ascii_case(&other.candidate) && self.quote == other.quote && self.token.eq_ignore_ascii_case(&other.token) && self.amount == other.amount
    }
}

impl Fee {
    fn line(&self) -> String {
        let (p, lf, d, rate) = match &self.priced {
            Ok(q) => (
                q.premium.to_dec(),
                q.lf.to_dec(),
                q.deposit.to_dec(),
                q.rate.as_ref().map(|r| format!("{}:{}:{}", r.price_token.to_dec(), r.price_unit.to_dec(), r.rate_at)).unwrap_or_default(),
            ),
            Err((n, _)) => (format!("!{n}"), String::new(), String::new(), String::new()),
        };
        let dec = self.decimals.map(|x| x.to_string()).unwrap_or_default();
        [self.candidate.as_str(), &self.quote, &self.token, &dec, &self.amount.to_dec(), &p, &lf, &d, &rate].join("|")
    }

    fn of_line(s: &str) -> Option<Fee> {
        let v: Vec<&str> = s.split('|').collect();
        let [candidate, quote, token, dec, amount, p, lf, d, rate] = v.as_slice() else { return None };
        let u = |x: &str| eravon_terms::u256::U256::from_dec(x);
        let priced = match p.strip_prefix('!') {
            Some(name) => Err((name.to_string(), String::new())),
            None => {
                let rate = if rate.is_empty() {
                    None
                } else {
                    let r: Vec<&str> = rate.split(':').collect();
                    let [pt, pu, at] = r.as_slice() else { return None };
                    Some(eravon_standard::precheck::Rate { price_token: u(pt)?, price_unit: u(pu)?, rate_at: at.parse().ok()? })
                };
                Ok(eravon_standard::precheck::Quoted { premium: u(p)?, lf: u(lf)?, deposit: u(d)?, rate })
            }
        };
        Some(Fee {
            candidate: candidate.to_string(),
            quote: quote.to_string(),
            token: token.to_string(),
            decimals: if dec.is_empty() { None } else { Some(dec.parse().ok()?) },
            amount: u(amount)?,
            priced,
        })
    }
}

pub fn fees_text(fees: &[Fee]) -> String {
    fees.iter().map(Fee::line).collect::<Vec<_>>().join(";")
}

pub fn fees_of_text(s: &str) -> Option<Vec<Fee>> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split(';').map(Fee::of_line).collect()
}

pub fn same_fees(a: &[Fee], b: &[Fee]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same_terms(y))
}

pub fn fees_of(w: &Wire, bytes: &[u8], r: &Resolved, decimals: Option<u64>) -> Vec<Fee> {
    let Ok(t) = eravon_terms::doc::terms(bytes) else { return Vec::new() };
    let rows: Vec<(usize, (String, String))> = r.judges.iter().map(|j| (j.judge.clone(), j.quote.clone())).chain(r.guilds.iter().map(|g| (g.pointer.clone(), g.quote.clone()))).enumerate().collect();
    let held = std::sync::Arc::new((w.t.clone(), w.table.clone(), t));
    eravon_chain::read::side_by_side(rows, move |(i, (candidate, quote))| {
        let offer = eravon_terms::derive::offer(&held.2, i as u64).ok()?;
        let priced = eravon_standard::order::bytes_of("报价", &quote)
            .and_then(|b| eravon_standard::precheck::quoted(&held.0, &held.1, &offer, &b))
            .map_err(|f| (f.name(), f.detail()));
        Some(Fee { candidate, quote, token: offer.token.clone(), decimals, amount: offer.amount, priced })
    })
    .into_iter()
    .flatten()
    .collect()
}

fn quote_for(v: &JudgeView, token: &str) -> Result<String, (&'static str, String)> {
    let rows = in_token(v, token);
    if rows.is_empty() {
        return Err(("E_GOODS_NO_QUOTE", format!("{} 不支持该结算币", v.address)));
    }
    match rows.iter().find(|q| q.usable) {
        Some(q) => Ok(q.spelling.clone()),
        None => Err(("E_GOODS_QUOTE_UNREAD", format!("{} 的这一币种报价读不成", v.address))),
    }
}

fn in_token<'a>(v: &'a JudgeView, token: &str) -> Vec<&'a eravon_cands::quote::Row> {
    v.quotes.iter().filter(|q| q.token == token).collect()
}


fn read_judge(w: &Wire, logs: &Logs, addr: &str) -> Result<JudgeView, Absent> {
    judge::open(w, logs, addr, "")
}

fn seat_for(v: &JudgeView, token: &str) -> Result<String, (&'static str, String)> {
    if !v.accepting {
        return Err(("E_GOODS_NOT_ACCEPTING", format!("{} 暂停接单", v.address)));
    }
    match &v.ledger {
        Err(why) => return Err(("E_GOODS_LEDGER_UNREAD", format!("{} 的判官账本读不出:{why}", v.address))),
        Ok(l) if !l.in_lineage => return Err(("E_ACT_JUDGE_LINEAGE", format!("账本世系里没有 {}:这张资料不是此账本主人的", v.address))),
        Ok(l) if !l.amanah_found && l.amanah_discarded => return Err((eravon_cands::act::E_ACT_AMANAH_DISCARDED, format!("{} 所供的判法条目{}", v.address, eravon_scan::ledger::DISCARDED_SAYS))),
        Ok(l) if !l.amanah_found => return Err(("E_ACT_AMANAH_UNANCHORED", format!("{} 所供的判法条目不在审计出的账本里", v.address))),
        Ok(_) => {}
    }
    quote_for(v, token)
}

pub fn resolve(w: &Wire, logs: &Logs, me: &str, table: &[Candidate], picks: &[String], token: &str) -> Resolved {
    crate::mark::mark(crate::mark::FEATURE);
    let mut r = Resolved::default();
    let mut named: Vec<String> = Vec::new();
    let mut sets: Vec<(String, Vec<String>)> = Vec::new();
    for p in picks {
        let a = p.to_ascii_lowercase();
        let Some(row) = table.iter().find(|x| x.address == a) else {
            r.excluded.push(Excluded { address: a, name: "E_GOODS_NOT_IN_TABLE".into(), detail: "该候选已被移除".into() });
            continue;
        };
        match Side::of(&row.kind) {
            Some(Side::Judge) => named.push(a),
            Some(Side::Guild) => match guild::open(w, logs, &a) {
                Err(e) => r.excluded.push(Excluded { address: a, name: e.name().into(), detail: e.detail() }),
                Ok(g) if g.pool => match pool_one(w, logs, me, &g, token) {
                    Ok(el) => r.guilds.push(el),
                    Err((n, d)) => r.excluded.push(Excluded { address: a, name: n.to_string(), detail: d }),
                },
                Ok(g) => {
                    let members = g.members.iter().map(|m| m.resolved.clone()).collect();
                    sets.push((g.pointer.clone(), members));
                }
            },
            None => r.excluded.push(Excluded { address: a, name: "E_CANDS_KIND".into(), detail: "候选类型无效".into() }),
        }
    }
    let held = std::sync::Arc::new((w.clone(), logs.clone()));
    let views = eravon_chain::read::side_by_side(named.clone(), move |a| read_judge(&held.0, &held.1, &a));
    for (a, view) in named.iter().zip(views) {
        match view.map_err(|e| (e.name(), e.detail())).and_then(|v| {
            seat_for(&v, token).map(|q| (v, q)).map_err(|(n, d)| (n, d))
        }) {
            Ok((v, q)) => r.judges.push(JudgeEl { judge: a.clone(), amanah_hash: v.amanah.clone(), quote: q, source: None }),
            Err((n, d)) => r.excluded.push(Excluded { address: a.clone(), name: n.into(), detail: d }),
        }
    }
    if !sets.is_empty() {
        let pointers: Vec<String> = sets.iter().map(|(p, _)| p.clone()).collect();
        let chain = eravon_roster::pool::standard_chain_id();
        let set = eravon_roster::pool::candidate_set(logs, chain, &pointers, &[], &[me.to_ascii_lowercase()]);
        let left: Vec<String> = set.list.into_iter().filter(|a| !named.contains(a) && !r.judges.iter().any(|j| j.judge == *a)).collect();
        let held = std::sync::Arc::new((w.clone(), logs.clone()));
        let views = eravon_chain::read::side_by_side(left.clone(), move |a| read_judge(&held.0, &held.1, &a));
        for (a, view) in left.into_iter().zip(views) {
            let source = sets.iter().find(|(_, ms)| ms.contains(&a)).map(|(p, _)| p.clone());
            match view.map_err(|e| (e.name(), e.detail())).and_then(|v| seat_for(&v, token).map(|q| (v, q))) {
                Ok((v, q)) => r.judges.push(JudgeEl { judge: a.clone(), amanah_hash: v.amanah.clone(), quote: q, source }),
                Err((n, d)) => r.excluded.push(Excluded { address: a, name: n.into(), detail: d }),
            }
        }
    }
    r
}

fn pool_one(w: &Wire, logs: &Logs, me: &str, g: &eravon_cands::guild::GuildView, token: &str) -> Result<GuildEl, (&'static str, String)> {
    if let Some(Ok(q)) = g.quotes.iter().find(|q| q.token.eq_ignore_ascii_case(token)).map(|q| &q.quote) {
        eravon_cands::quote::paced(eravon_roster::pool::standard_chain_id(), q).map_err(|why| (eravon_roster::pool::PoolRefusal::Pace.name(), why))?;
    }
    let text = match g.terms.as_ref().map(|t| &t.text) {
        Some(Ok(t)) => t.clone(),
        Some(Err(e)) => return Err(("E_POOL_TERMS_DIGEST", format!("无法获取行会规则:{e}"))),
        None => return Err(("E_POOL_NO_GUILDTERMS", "行会未提供规则".into())),
    };
    let book = crate::pool::LiveBook { w };
    let chain = eravon_roster::pool::standard_chain_id();
    match eravon_roster::pool::pool_candidate(logs, &book, chain, &g.pointer, &text, token, &[me.to_ascii_lowercase()], w.now, u64::MAX) {
        Ok(pc) => {
            eravon_cands::gas::fits(pc.panel_len as u64, !pc.router.trim_start_matches("0x").bytes().all(|b| b == b'0')).map_err(|w| ("E_POOL_GAS", w))?;
            Ok(GuildEl {
            pointer: g.pointer.clone(),
            amanah_hash: pc.amanah_hash,
            quote: pc.quote,
            seq: pc.seq,
            panel_hash: pc.panel_hash,
            text,
            })
        }
        Err(r) => Err((r.name(), format!("池的读法四核没过:{}", r.name()))),
    }
}


#[cfg(test)]
mod fee_tests {
    use super::*;
    use eravon_standard::precheck::{Quoted, Rate};
    use eravon_terms::u256::U256;

    fn fee(candidate: &str, quote: &str, amount: u64, priced: Result<Quoted, (String, String)>) -> Fee {
        Fee { candidate: candidate.into(), quote: quote.into(), token: format!("0x{}", "c2".repeat(20)), decimals: Some(18), amount: U256::from_u64(amount), priced }
    }

    fn q(p: u64, lf: u64, rate: Option<Rate>) -> Quoted {
        Quoted { premium: U256::from_u64(p), lf: U256::from_u64(lf), deposit: U256::from_u64(p + lf), rate }
    }

    #[test]
    fn recorded_fees_read_back_whole() {
        let rate = Rate { price_token: U256::from_u64(200_000_000), price_unit: U256::from_u64(108_000_000), rate_at: 1_790_000_000 };
        let list = vec![
            fee("0xa1", "0xq1", 1200, Ok(q(12, 60, None))),
            fee("galeed:guild:1:0xb3", "0xq2", 1200, Ok(q(12, 65, Some(rate)))),
            fee("0xd4", "0xq3", 1200, Err(("E_A1_PRECHECK".into(), String::new()))),
        ];
        assert_eq!(fees_of_text(&fees_text(&list)), Some(list.clone()));
        assert_eq!(fees_of_text(""), Some(Vec::new()));
        assert_eq!(fees_of_text(&format!("{};0xbroken|1", fees_text(&list[..1]))), None);
    }

    #[test]
    fn fees_compare_by_quote_and_amount() {
        let a = vec![fee("0xa1", "0xq1", 1200, Ok(q(12, 60, None)))];
        assert!(same_fees(&a, &[fee("0xA1", "0xq1", 1200, Ok(q(12, 61, None)))]));
        assert!(!same_fees(&a, &[fee("0xa1", "0xq9", 1200, Ok(q(24, 60, None)))]));
        assert!(!same_fees(&a, &[fee("0xa1", "0xq1", 1300, Ok(q(13, 65, None)))]));
        assert!(!same_fees(&a, &[]));
    }
}
