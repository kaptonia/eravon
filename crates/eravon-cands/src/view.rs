use eravon_store::record::Candidate;

use crate::fault::{Absent, Fault};
use crate::guild::{self, GuildView};
use crate::judge::{self, JudgeView};
use crate::registry;
use crate::table::{self, Side};
use crate::wire::Wire;

#[derive(Clone, Debug)]
pub struct Brief {
    pub row: Candidate,
    pub state: Result<Summary, Absent>,
    pub warn: Option<String>,
    pub flaw: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub version: Option<u64>,
    pub unread: Option<judge::Unread>,
    pub accepting: bool,
    pub selectable: bool,
    pub unpaced: bool,
    pub needs_resign: bool,
    pub resign_items: Vec<String>,
    pub members: usize,
    pub pool: bool,
    pub lapses: Vec<(String, crate::lapse::Clocks)>,
    pub current: String,
}

#[derive(Clone, Debug)]
pub enum Detail {
    Judge(Box<JudgeView>),
    Guild(Box<GuildView>),
}

fn lapses_of(rows: &[crate::quote::Row]) -> Vec<(String, crate::lapse::Clocks)> {
    rows.iter().filter(|r| r.usable).filter_map(|r| r.quote.as_ref().ok().and_then(|q| crate::lapse::clocks(q).ok()).map(|c| (r.token.clone(), c))).collect()
}

pub fn summary_of(d: &Detail) -> Summary {
    match d {
        Detail::Judge(j) => Summary {
            version: j.version,
            unread: j.law().err(),
            accepting: j.accepting,
            selectable: j.selectable(),
            unpaced: false,
            needs_resign: j.needs_resign,
            resign_items: j.resign_items.clone(),
            members: 0,
            pool: false,
            lapses: lapses_of(&j.quotes),
            current: j.address.clone(),
        },
        Detail::Guild(g) => Summary {
            version: g.terms.as_ref().and_then(|t| t.version),
            unread: None,
            accepting: true,
            selectable: g.selectable(),
            unpaced: g.unpaced(),
            needs_resign: g.needs_resign,
            resign_items: g.resign_items.clone(),
            members: g.members.len(),
            pool: g.pool,
            lapses: lapses_of(&g.quotes),
            current: g.address.clone(),
        },
    }
}

fn word_of(d: &Detail) -> &str {
    match d {
        Detail::Judge(j) => &j.bound,
        Detail::Guild(g) => &g.bound,
    }
}

fn read_one(w: &Wire, logs: &eravon_roster::adapter::Logs, row: &Candidate) -> Result<Detail, Absent> {
    match Side::of(&row.kind) {
        Some(Side::Judge) => judge::open(w, logs, &row.address, &row.seen).map(|j| Detail::Judge(Box::new(j))),
        Some(Side::Guild) => guild::open(w, logs, &row.address).map(|g| Detail::Guild(Box::new(g))),
        None => Err(Absent::Unverified(format!("表上这一行的类是 {:?}", row.kind))),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signed {
    pub address: String,
    pub word: String,
    pub item: String,
    pub quote: String,
    pub token: String,
}

pub fn owing(signed: &[Signed], address: &str, bound: &str, quotes: &[(String, String)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if bound.is_empty() {
        return out;
    }
    let now = |t: &str| quotes.iter().find(|(x, _)| x.eq_ignore_ascii_case(t)).map(|(_, h)| h.as_str());
    for s in signed.iter().filter(|s| s.address.eq_ignore_ascii_case(address) && crate::guild::guild_due(&s.word, &s.quote, bound, now(&s.token))) {
        if !out.contains(&s.item) {
            out.push(s.item.clone());
        }
    }
    out
}

fn settle(mut d: Detail, signed: &[Signed], address: &str) -> Detail {
    match &mut d {
        Detail::Judge(j) => {
            j.resign_items = owing(signed, address, &j.bound, &[]);
            j.needs_resign = !j.resign_items.is_empty();
        }
        Detail::Guild(g) => {
            g.resign_items = owing(signed, address, &g.bound, &g.quote_hashes());
            g.needs_resign = !g.resign_items.is_empty();
        }
    }
    d
}

pub fn list(data: &str, me: &str, registries: &[String], now: u64, writer: &str, signed: &[Signed]) -> Result<Vec<Brief>, Fault> {
    list_over(data, me, || Wire::of(data, me, registries, now), now, Some(writer), signed)
}

pub fn list_on(data: &str, me: &str, route: &Result<(eravon_chain::transport::Auto, eravon_chain::endpoints::Table), (String, String)>, registries: &[String], now: u64, signed: &[Signed]) -> Result<Vec<Brief>, Fault> {
    list_over(data, me, || Wire::on(route, registries, now), now, None, signed)
}

fn list_over(data: &str, me: &str, wire: impl FnOnce() -> Result<Wire, Absent>, now: u64, writer: Option<&str>, signed: &[Signed]) -> Result<Vec<Brief>, Fault> {
    let rows = table::read(data, me)?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let asked: Vec<String> = rows.iter().map(|r| r.row.address.clone()).collect();
    let ready = wire().and_then(|w| registry::logs_of(&w, &asked).map(|l| (w, l)));
    let reads: Vec<Result<Detail, Absent>> = match ready {
        Ok(ctx) => {
            let ctx = std::sync::Arc::new(ctx);
            eravon_chain::read::side_by_side(rows.iter().map(|r| r.row.clone()).collect::<Vec<_>>(), move |row| read_one(&ctx.0, &ctx.1, &row))
        }
        Err(a) => rows.iter().map(|_| Err(a.clone())).collect(),
    };
    let mut out = Vec::with_capacity(rows.len());
    for (table::Row { row, flaw }, read) in rows.into_iter().zip(reads) {
        let mut warn = None;
        let state = match read {
            Err(a) => Err(a),
            Ok(d) => Ok(d)
                .map(|d| settle(d, signed, &row.address))
                .inspect(|d| warn = writer.and_then(|w| baseline(data, w, now, me, &row, d)))
                .map(|d| summary_of(&d)),
        };
        out.push(Brief { row, state, warn, flaw });
    }
    Ok(out)
}

#[allow(clippy::type_complexity)]
pub fn detail(data: &str, me: &str, address: &str, registries: &[String], now: u64, writer: &str, signed: &[Signed]) -> Result<(Candidate, Result<Detail, Absent>, Option<String>), Fault> {
    detail_over(data, me, address, || Wire::of(data, me, registries, now), now, Some(writer), signed)
}

#[allow(clippy::type_complexity)]
pub fn detail_on(
    data: &str,
    me: &str,
    address: &str,
    route: &Result<(eravon_chain::transport::Auto, eravon_chain::endpoints::Table), (String, String)>,
    registries: &[String],
    now: u64,
    signed: &[Signed],
) -> Result<(Candidate, Result<Detail, Absent>, Option<String>), Fault> {
    detail_over(data, me, address, || Wire::on(route, registries, now), now, None, signed)
}

#[allow(clippy::type_complexity)]
fn detail_over(data: &str, me: &str, address: &str, wire: impl FnOnce() -> Result<Wire, Absent>, now: u64, writer: Option<&str>, signed: &[Signed]) -> Result<(Candidate, Result<Detail, Absent>, Option<String>), Fault> {
    let row = table::list(data, me)?
        .into_iter()
        .find(|r| r.address == address)
        .ok_or_else(|| Fault::Missing(address.to_string()))?;
    let got = wire()
        .and_then(|w| registry::logs_of(&w, std::slice::from_ref(&row.address)).and_then(|logs| read_one(&w, &logs, &row)))
        .map(|d| settle(d, signed, &row.address));
    let warn = match &got {
        Ok(d) => writer.and_then(|w| baseline(data, w, now, me, &row, d)),
        Err(_) => None,
    };
    Ok((row, got, warn))
}

pub fn seen_now(route: &Result<(eravon_chain::transport::Auto, eravon_chain::endpoints::Table), (String, String)>, side: Side, address: &str, registries: &[String], now: u64) -> String {
    let row = Candidate { kind: side.word().to_string(), address: address.to_string(), label: String::new(), added_at: now, seen: String::new() };
    Wire::on(route, registries, now)
        .and_then(|w| registry::logs_of(&w, std::slice::from_ref(&row.address)).and_then(|logs| read_one(&w, &logs, &row)))
        .map(|d| word_of(&d).to_string())
        .unwrap_or_default()
}

fn baseline(data: &str, writer: &str, now: u64, me: &str, row: &Candidate, d: &Detail) -> Option<String> {
    if !row.seen.is_empty() {
        return None;
    }
    table::remember(data, writer, now, me, &row.address, word_of(d))
        .err()
        .map(|e| format!("凭据保存失败:{}:{}", e.name(), e.detail()))
}

pub fn peek(data: &str, me: &str, address: &str, registries: &[String], now: u64) -> (Candidate, Result<Detail, Absent>) {
    let row = Candidate { kind: Side::Judge.word().to_string(), address: address.to_ascii_lowercase(), label: String::new(), added_at: 0, seen: String::new() };
    let got = Wire::of(data, me, registries, now).and_then(|w| registry::logs_of(&w, std::slice::from_ref(&row.address)).and_then(|logs| read_one(&w, &logs, &row)));
    (row, got)
}

#[cfg(test)]
mod owing_tests {
    use super::{owing, Signed};

    fn s(address: &str, word: &str, item: &str) -> Signed {
        Signed { address: address.into(), word: word.into(), item: item.into(), quote: String::new(), token: String::new() }
    }

    #[test]
    fn a_candidate_owes_re_signing_only_for_items_still_on_an_older_word() {
        let (j, k) = ("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000b2");
        let all_new = [s(j, "0x03", "甲品"), s(j, "0x03", "乙品"), s(k, "0x01", "丙品")];
        assert!(owing(&all_new, j, "0x03", &[]).is_empty(), "every item re-signed");
        let upper = j.to_ascii_uppercase().replacen("0X", "0x", 1);
        let one_old = [s(j, "0x03", "甲品"), s(&upper, "0x02", "乙品"), s(j, "0x02", "乙品"), s(k, "0x01", "丙品")];
        assert_eq!(owing(&one_old, j, "0x03", &[]), vec!["乙品".to_string()]);
        assert!(owing(&one_old, k, "0x01", &[]).is_empty());
        assert!(owing(&one_old, j, "", &[]).is_empty(), "no word offered: nothing owed");
    }
}
