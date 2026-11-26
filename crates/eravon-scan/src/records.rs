use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed::Value;
use eravon_link::identity::{Rec, Records};
use eravon_literals::constants as k7;
use eravon_standard as a1;
use eravon_chain as a2;
use eravon_standard::order::hex_of;

use crate::fault::R;
use crate::log::Log;

fn s(x: &str) -> Value {
    Value::Str(x.to_ascii_lowercase())
}

fn rec(l: &Log, members: Vec<(&str, Value)>) -> Rec {
    let mut m: Vec<(String, Value)> = vec![("blockNumber".into(), Value::Int(l.block)), ("logIndex".into(), Value::Int(l.log_index))];
    m.extend(members.into_iter().map(|(k, v)| (k.to_string(), v)));
    m.sort_by(|a, b| a.0.cmp(&b.0));
    Rec { pos: (l.block, l.log_index), value: Value::Obj(m) }
}

fn word_addr(w: &[u8]) -> String {
    hex_of(&w[12..32])
}

fn word_dec(w: &[u8]) -> String {
    eravon_terms::u256::U256::from_be(w.try_into().unwrap_or(&[0u8; 32])).to_dec()
}

pub fn book<T: Transport>(t: &T, table: &Table) -> R<eravon_galeed::book::Logs> {
    let (code, _) = eravon_chain::read::code(t, table, k7::SUCCESSION_BOOK, eravon_chain::seam::TAG_LATEST)?;
    let hash = hex_of(&eravon_terms::derive::keccak(&code));
    if code.is_empty() || !hash.eq_ignore_ascii_case(k7::BOOK_CODE_HASH) {
        return Ok(eravon_galeed::book::Logs::default());
    }
    crate::succession::logs(t, table)
}

fn posted_rec(p: &crate::board::Post, l: &Log) -> Rec {
    rec(l, vec![("poster", s(&p.poster)), ("key", s(&hex_of(&p.key))), ("hash", s(&hex_of(&p.hash))), ("document", s(&hex_of(&p.document)))])
}

fn post_log(p: &crate::board::Post) -> Log {
    Log { address: String::new(), topics: Vec::new(), data: Vec::new(), block: p.block, log_index: p.log_index, tx: p.tx.clone() }
}

pub fn for_listing<T: Transport>(t: &T, table: &Table, chain_id: u64, terms_hash: &[u8; 32]) -> R<Records> {
    crate::mark::mark(crate::mark::FEATURE);
    let posts = crate::board::by_key(t, table, terms_hash, 0, eravon_chain::seam::Upto::Latest)?;
    let posted = posts.iter().map(|p| posted_rec(p, &post_log(p))).collect();
    Ok(Records { chain_id, posted, book: book(t, table)?, ..Records::default() })
}

pub fn for_record<T: Transport>(t: &T, table: &Table, chain_id: u64, logic: &str, a: &str) -> R<Records> {
    crate::mark::mark(crate::mark::FEATURE);
    let book = book(t, table)?;
    let mut keys = eravon_galeed::book::record_keys(&book, a).into_iter().map(|k| k.to_ascii_lowercase()).collect::<Vec<_>>();
    if keys.is_empty() {
        keys.push(a.to_ascii_lowercase());
    }
    let mut out = Records { chain_id, book, ..Records::default() };
    let ctx = std::sync::Arc::new((t.clone(), table.clone(), logic.to_string(), keys.clone()));
    let posts = {
        let c = ctx.clone();
        move || crate::board::by_poster(&c.0, &c.1, &c.3, 0, eravon_chain::seam::Upto::Latest)
    };
    let logic_recs = {
        let c = ctx.clone();
        move || crate::scan::logic_records(&c.0, &c.1, &c.2, &c.3, a2::seam::Upto::Latest)
    };
    let scans = {
        let c = ctx.clone();
        move || a2::read::side_by_side(vec![crate::scan::Side::Seller, crate::scan::Side::Buyer], move |side| crate::scan::by_keys(&c.0, &c.1, &c.2, side, &c.3, 0, a2::seam::Upto::Latest, &[]))
    };
    let (posts, (records, sides)) = if a2::recall::current().is_none() { a2::read::both(posts, || a2::read::both(logic_recs, scans)) } else { (posts(), (logic_recs(), scans())) };
    for p in posts? {
        out.posted.push(posted_rec(&p, &post_log(&p)));
    }
    let records = records?;
    for (ev, kind) in [(a1::sol::CONSENTED_EV, 0u8), (a1::sol::REVOKED_EV, 1), (a1::sol::PULLED_EV, 2)] {
        for l in crate::scan::picked(&records, &ev.topic0(), &[]) {
            if !l.address.eq_ignore_ascii_case(logic) || l.topics.len() < 3 {
                continue;
            }
            let r = match kind {
                0 => rec(&l, vec![("who", s(&l.topic_address(1)?)), ("digest", s(&hex_of(&l.topics[2])))]),
                1 => rec(&l, vec![("seller", s(&l.topic_address(1)?)), ("termsHash", s(&hex_of(&l.topics[2])))]),
                _ if l.data.len() >= 96 => rec(
                    &l,
                    vec![
                        ("seller", s(&l.topic_address(1)?)),
                        ("fund", s(&l.topic_address(2)?)),
                        ("token", s(&word_addr(&l.data[0..32]))),
                        ("instance", s(&word_addr(&l.data[32..64]))),
                        ("amount", Value::Str(word_dec(&l.data[64..96]))),
                    ],
                ),
                _ => continue,
            };
            match kind {
                0 => out.consented.push(r),
                1 => out.revoked.push(r),
                _ => out.pulled.push(r),
            }
        }
    }
    let mut instances: Vec<(String, u64)> = Vec::new();
    for sw in sides {
        for c in sw?.cases {
            if !instances.iter().any(|(i, _)| *i == c.instance.to_ascii_lowercase()) {
                instances.push((c.instance.to_ascii_lowercase(), c.block));
            }
        }
    }
    instances.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let reads: Vec<(Vec<String>, u64, a2::seam::Upto)> = instances
        .chunks(a2::seam::FILTER_FAN)
        .map(|part| (part.iter().map(|(i, _)| i.clone()).collect(), part.iter().map(|(_, b)| a2::read::instance_from(*b)).min().unwrap_or(0), a2::seam::Upto::Latest))
        .collect();
    a2::read::logs_at_any_ahead(t, table, &reads);
    for part in instances.chunks(a2::seam::FILTER_FAN) {
        let addrs: Vec<String> = part.iter().map(|(i, _)| i.clone()).collect();
        let from = part.iter().map(|(_, b)| a2::read::instance_from(*b)).min().unwrap_or(0);
        let (rows, _) = a2::read::logs_at_any(t, table, &addrs, from, a2::seam::Upto::Latest)?;
        for row in &rows {
            let l = Log::read(row)?;
            if !part.iter().any(|(i, b)| i.eq_ignore_ascii_case(&l.address) && l.block >= a2::read::instance_from(*b)) {
                continue;
            }
            let Ok(d) = a1::event::decode(&l.topics, &l.data) else { continue };
            if d.name == "Opened" && l.topics.len() == 4 && l.data.len() >= 96 {
                out.opened.push(rec(
                    &l,
                    vec![
                        ("emitter", s(&l.address)),
                        ("caseId", s(&hex_of(&l.topics[1]))),
                        ("seller", s(&l.topic_address(2)?)),
                        ("buyer", s(&l.topic_address(3)?)),
                        ("judge", s(&word_addr(&l.data[0..32]))),
                        ("amount", Value::Str(word_dec(&l.data[32..64]))),
                        ("termsHash", s(&hex_of(&l.data[64..96]))),
                    ],
                ));
            } else {
                out.events.push(rec(&l, vec![("emitter", s(&l.address)), ("name", Value::Str(d.name.to_string()))]));
            }
        }
    }
    for list in [&mut out.posted, &mut out.consented, &mut out.revoked, &mut out.pulled, &mut out.opened, &mut out.events] {
        list.sort_by_key(|r| r.pos);
        list.dedup_by_key(|r| r.pos);
    }
    Ok(out)
}

pub fn pay_check<T: Transport>(t: &T, table: &Table, chain_id: u64, listing: &[u8]) -> R<Result<eravon_link::identity::Pay, eravon_link::token::Verdict>> {
    let Ok(p) = eravon_link::listing::listing(listing) else {
        return Ok(eravon_link::identity::pay_check(&Records { chain_id, ..Records::default() }, listing));
    };
    let th = a1::order::b32_of("termsHash", &p.terms_hash)?;
    let rs = for_listing(t, table, chain_id, &th)?;
    Ok(eravon_link::identity::pay_check(&rs, listing))
}
