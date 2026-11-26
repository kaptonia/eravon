use eravon_cands::fault::Absent;
use eravon_cands::view::{Brief, Detail};
use eravon_literals::json::Out;

const BLANK: &str = "";

pub fn absent_out(a: &Absent) -> Out {
    let mut m = vec![("class", Out::S(a.class().to_string())), ("detail", Out::S(a.detail())), ("name", Out::S(a.name().to_string()))];
    if let Absent::Succeeded { to, block } = a {
        m.push(("successor", Out::O(vec![("address", Out::S(to.clone())), ("block", Out::N(*block))])));
    }
    Out::O(m)
}

pub fn opt_n(v: Option<u64>) -> Out {
    match v {
        Some(n) => Out::N(n),
        None => Out::S(String::new()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pill {
    Flawed,
    Absent,
    Succeeded,
    Closed,
    Resign,
    Choosable,
    Unpaced,
    NoQuote,
}

impl Pill {
    pub fn word(self) -> &'static str {
        match self {
            Pill::Flawed => "flawed",
            Pill::Absent => "absent",
            Pill::Succeeded => "succeeded",
            Pill::Closed => "closed",
            Pill::Resign => "resign",
            Pill::Choosable => "choosable",
            Pill::Unpaced => "unpaced",
            Pill::NoQuote => "noQuote",
        }
    }
}

pub fn pill_of(b: &Brief) -> Pill {
    if b.flaw.is_some() {
        return Pill::Flawed;
    }
    match &b.state {
        Err(eravon_cands::fault::Absent::Succeeded { .. }) => Pill::Succeeded,
        Err(_) => Pill::Absent,
        Ok(s) if !s.accepting => Pill::Closed,
        Ok(s) if s.needs_resign => Pill::Resign,
        Ok(s) if s.selectable => Pill::Choosable,
        Ok(s) if s.unpaced => Pill::Unpaced,
        Ok(_) => Pill::NoQuote,
    }
}

fn unread_out(u: Option<&eravon_cands::judge::Unread>) -> Out {
    Out::S(u.map_or(BLANK, |u| u.name()).to_string())
}

pub fn brief_out(b: &Brief) -> Out {
    let mut m = vec![
        ("addedAt", Out::N(b.row.added_at)),
        ("address", Out::S(b.row.address.clone())),
        ("kind", Out::S(b.row.kind.clone())),
        ("label", Out::S(b.row.label.clone())),
        ("pill", Out::S(pill_of(b).word().to_string())),
        ("seen", Out::S(b.row.seen.clone())),
    ];
    match &b.state {
        Ok(s) => m.push((
            "state",
            Out::O(vec![
                ("accepting", Out::B(s.accepting)),
                ("current", Out::S(s.current.clone())),
                ("members", Out::N(s.members as u64)),
                ("needsResign", Out::B(s.needs_resign)),
                ("pool", Out::B(s.pool)),
                ("resignItems", Out::A(s.resign_items.iter().map(|x| Out::S(x.clone())).collect())),
                ("selectable", Out::B(s.selectable)),
                ("unread", unread_out(s.unread.as_ref())),
                ("version", opt_n(s.version)),
            ]),
        )),
        Err(a) => m.push(("absent", absent_out(a))),
    }
    if let Some(w) = &b.warn {
        m.push(("warn", Out::S(w.clone())));
    }
    if let Some(f) = &b.flaw {
        m.push(("flaw", Out::S(f.clone())));
    }
    Out::O(m)
}

pub fn quote_out(q: &eravon_cands::quote::Row, decimals: &[(String, Option<u64>)]) -> Out {
    let mut m = vec![("token", Out::S(q.token.clone())), ("usable", Out::B(q.usable))];
    match &q.quote {
        Ok(x) => {
            let c = eravon_cands::quote::cells(x, &q.token, decimals, eravon_vault::lang::Lang::Zh);
            m.push((
                "cells",
                Out::O(vec![
                    ("accept", Out::S(c.accept)),
                    ("cap", Out::S(c.cap)),
                    ("exampleAmount", Out::S(c.example_amount)),
                    ("exampleLawsuit", Out::S(c.example_lawsuit)),
                    ("examplePremium", Out::S(c.example_premium)),
                    ("lapse", Out::S(c.lapse)),
                    ("lawsuit", Out::S(c.lawsuit)),
                    ("premium", Out::S(c.premium)),
                    ("reference", Out::S(c.reference)),
                    ("rulingDays", Out::S(c.ruling_days)),
                ]),
            ));
        }
        Err(e) => m.push(("undecodable", Out::S(e.clone()))),
    }
    Out::O(m)
}

pub fn judge_out(j: &eravon_cands::judge::JudgeView) -> Out {
    let mut m = vec![
        ("accepting", Out::B(j.accepting)),
        ("address", Out::S(j.address.clone())),
        ("amanah", Out::S(j.amanah.clone())),
        ("attempts", attempts_out(&j.attempts)),
        ("beyond", beyond_out(&j.beyond)),
        ("block", Out::N(j.block)),
        ("home", Out::S(j.home.clone().unwrap_or_default())),
        ("listed", Out::S(j.listed.clone())),
        ("name", Out::S(j.name.clone().unwrap_or_default())),
        ("needsResign", Out::B(j.needs_resign)),
        ("noteMd", Out::S(j.note_md.clone().unwrap_or_default())),
        ("offers", Out::B(j.offers())),
        ("published", opt_n(j.published)),
        ("quotes", Out::A(j.quotes.iter().map(|q| quote_out(q, &j.decimals)).collect())),
        ("resignItems", Out::A(j.resign_items.iter().map(|x| Out::S(x.clone())).collect())),
        ("selectable", Out::B(j.selectable())),
        ("text", Out::S(j.text.clone().unwrap_or_default())),
        ("trail", Out::A(j.trail.iter().map(|a| Out::S(a.clone())).collect())),
        ("unread", unread_out(j.law().err().as_ref())),
        ("version", opt_n(j.version)),
        ("word", Out::S(j.word.clone())),
    ];
    m.push((
        "ledger",
        match &j.ledger {
            Ok(l) => Out::O(vec![
                ("amanahFound", Out::B(l.amanah_found)),
                ("discarded", Out::A(l.discarded.iter().map(|n| Out::S(n.clone())).collect())),
                ("entries", Out::N(l.entries as u64)),
                ("inLineage", Out::B(l.in_lineage)),
                ("label", Out::S(l.label.clone())),
                ("narrow", Out::A(l.narrow.iter().map(|n| Out::S(n.clone())).collect())),
                ("ok", Out::B(true)),
                ("unchecked", Out::A(l.unchecked.iter().map(|n| Out::S(n.clone())).collect())),
            ]),
            Err(e) => Out::O(vec![("ok", Out::B(false)), ("why", Out::S(e.clone()))]),
        },
    ));
    if let Some(records) = &j.records {
    m.push((
        "records",
        match records {
            Ok(list) => Out::A(
                list.iter()
                    .map(|r| {
                        let mut x = vec![
                            ("at", opt_n(r.at)),
                            ("instance", Out::S(r.instance.clone())),
                            ("label", Out::S(r.label.clone())),
                            ("tx", Out::S(r.tx.clone())),
                        ];
                        if let Some(red) = &r.red {
                            x.push(("red", Out::S(red.clone())));
                        }
                        Out::O(x)
                    })
                    .collect(),
            ),
            Err(e) => Out::O(vec![("unavailable", Out::S(e.clone()))]),
        },
    ));
    }
    Out::O(m)
}

pub fn guild_out(g: &eravon_cands::guild::GuildView) -> Out {
    let mut m = vec![
        ("address", Out::S(g.address.clone())),
        ("beyond", beyond_out(&g.beyond)),
        ("card", Out::S(g.card.into())),
        ("cardUnread", Out::B(g.card_unread())),
        ("home", Out::S(g.home.clone().unwrap_or_default())),
        ("issued", Out::N(g.issued)),
        (
            "members",
            Out::A(
                g.members
                    .iter()
                    .map(|x| {
                        Out::O(vec![
                            ("attempts", attempts_out(&x.attempts)),
                            ("beyond", beyond_out(&x.beyond)),
                            ("home", Out::S(x.home.clone().unwrap_or_default())),
                            ("listed", Out::S(x.listed.clone())),
                            ("name", Out::S(x.name.clone().unwrap_or_default())),
                            ("noteMd", Out::S(x.note_md.clone().unwrap_or_default())),
                            ("resolved", Out::S(x.resolved.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("name", Out::S(g.name.clone().unwrap_or_default())),
        ("needsResign", Out::B(g.needs_resign)),
        ("pointer", Out::S(g.pointer.clone())),
        ("pool", Out::B(g.pool)),
        ("published", opt_n(g.published)),
        ("quotes", Out::A(g.quotes.iter().map(|q| quote_out(q, &g.decimals)).collect())),
        ("resignItems", Out::A(g.resign_items.iter().map(|x| Out::S(x.clone())).collect())),
        ("selectable", Out::B(g.selectable())),
        ("word", Out::S(g.word.clone())),
        ("words", Out::S(g.words.clone().unwrap_or_default())),
    ];
    if let Some(t) = &g.terms {
        m.push((
            "terms",
            Out::O(vec![
                ("sha256", Out::S(t.sha256.clone())),
                (
                    "text",
                    match &t.text {
                        Ok(s) => Out::O(vec![("ok", Out::B(true)), ("text", Out::S(s.clone()))]),
                        Err(e) => Out::O(vec![("ok", Out::B(false)), ("why", Out::S(e.clone()))]),
                    },
                ),
                ("version", opt_n(t.version)),
            ]),
        ));
    }
    Out::O(m)
}

fn attempts_out(a: &[eravon_cands::judge::Attempt]) -> Out {
    Out::A(a.iter().map(|x| Out::O(vec![("block", Out::N(x.block)), ("to", Out::S(x.to.clone()))])).collect())
}

fn beyond_out(b: &[(String, String)]) -> Out {
    Out::A(b.iter().map(|(k, v)| Out::O(vec![("key", Out::S(k.clone())), ("value", Out::S(v.clone()))])).collect())
}

pub fn shown(d: &Result<Detail, Absent>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    match d {
        Ok(Detail::Judge(j)) => seen.extend(std::iter::once(j.listed.clone()).chain(j.trail.iter().cloned()).chain(std::iter::once(j.address.clone()))),
        Ok(Detail::Guild(g)) => {
            seen.push(g.address.clone());
            for m in &g.members {
                seen.push(m.listed.clone());
                seen.push(m.resolved.clone());
            }
        }
        Err(_) => {}
    }
    let mut out: Vec<String> = Vec::new();
    for a in seen.into_iter().map(|a| a.to_ascii_lowercase()) {
        if !a.is_empty() && !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

pub fn codes<T: eravon_chain::transport::Transport>(t: &T, table: &eravon_chain::endpoints::Table, addrs: &[String]) -> Vec<(String, Result<bool, (String, String)>)> {
    eravon_chain::read::ahead_codes(t, table, addrs, eravon_chain::seam::TAG_LATEST);
    addrs
        .iter()
        .map(|a| (a.clone(), eravon_chain::read::code(t, table, a, eravon_chain::seam::TAG_LATEST).map(|(c, _)| !c.is_empty()).map_err(|e| (e.name().to_string(), e.detail()))))
        .collect()
}

pub fn codes_out(codes: &[(String, Result<bool, (String, String)>)]) -> Out {
    Out::A(
        codes
            .iter()
            .map(|(a, r)| match r {
                Ok(b) => Out::O(vec![("address", Out::S(a.clone())), ("code", Out::B(*b))]),
                Err((n, d)) => Out::O(vec![("address", Out::S(a.clone())), ("unread", Out::O(vec![("detail", Out::S(d.clone())), ("name", Out::S(n.clone()))]))]),
            })
            .collect(),
    )
}

pub fn detail_out(row: &eravon_store::record::Candidate, got: &Result<Detail, Absent>, warn: Option<&String>, codes: &[(String, Result<bool, (String, String)>)]) -> Vec<(&'static str, Out)> {
    let mut d = vec![("address", Out::S(row.address.clone())), ("label", Out::S(row.label.clone()))];
    match got {
        Ok(Detail::Judge(j)) => d.push(("judge", judge_out(j))),
        Ok(Detail::Guild(g)) => d.push(("guild", guild_out(g))),
        Err(a) => d.push(("absent", absent_out(a))),
    }
    if let Some(w) = warn {
        d.push(("warn", Out::S(w.clone())));
    }
    d.push(("codes", codes_out(codes)));
    d
}

#[cfg(test)]
mod page_tests {
    use super::*;
    use eravon_cands::fault::Absent;
    use eravon_cands::view::{Brief, Summary};

    fn row(addr: &str) -> eravon_store::record::Candidate {
        eravon_store::record::Candidate { kind: "judge".into(), address: addr.into(), label: "x".into(), added_at: 1, seen: String::new() }
    }

    fn brief(addr: &str, state: Result<Summary, Absent>, flaw: Option<String>) -> Brief {
        Brief { row: row(addr), state, warn: None, flaw }
    }

    fn summary(accepting: bool, resign: bool, selectable: bool, current: &str) -> Summary {
        Summary { version: Some(1), unread: None, accepting, selectable, unpaced: false, needs_resign: resign, resign_items: if resign { vec!["goods".into()] } else { Vec::new() }, members: 0, pool: false, lapses: Vec::new(), current: current.into() }
    }

    fn key(o: &Out, k: &str) -> Option<Out> {
        match o {
            Out::O(m) => m.iter().find(|(x, _)| *x == k).map(|(_, v)| v.clone()),
            _ => None,
        }
    }

    fn said(o: Option<Out>) -> Option<String> {
        o.map(|x| String::from_utf8(eravon_literals::json::write(&x)).expect("utf8"))
    }

    #[test]
    fn every_candidate_state_is_said_by_the_one_build() {
        let a = "0x00000000000000000000000000000000000000a1";
        let cases = [
            (brief(a, Ok(summary(true, false, true, a)), None), "choosable"),
            (brief(a, Ok(summary(true, true, true, a)), None), "resign"),
            (brief(a, Ok(summary(false, false, true, a)), None), "closed"),
            (brief(a, Err(Absent::Unreachable("端点表还没配".into())), None), "absent"),
            (brief(a, Ok(summary(true, false, false, a)), None), "noQuote"),
            (brief(a, Err(Absent::Succeeded { to: "0x00000000000000000000000000000000000000b2".into(), block: 7 }), None), "succeeded"),
            (brief(a, Ok(summary(true, false, true, a)), Some("kind".into())), "flawed"),
        ];
        for (b, want) in &cases {
            assert_eq!(pill_of(b).word(), *want);
            assert_eq!(said(key(&brief_out(b), "pill")), said(Some(Out::S((*want).into()))));
        }
        let moved = brief(a, Ok(summary(true, false, true, "0x00000000000000000000000000000000000000c3")), None);
        let state = key(&brief_out(&moved), "state").expect("state");
        assert_eq!(said(key(&state, "current")), said(Some(Out::S("0x00000000000000000000000000000000000000c3".into()))), "已换钥:现读的新钥");
        let gone = absent_out(&Absent::Succeeded { to: "0x00000000000000000000000000000000000000b2".into(), block: 7 });
        assert!(key(&gone, "successor").is_some(), "已交继任:说出新钥与块");
    }
}
