use eravon_link as k3;
use eravon_scan as a3;

use crate::order::{self, Order, State};
use crate::send::{self, Post};
use crate::{Why, Wire, why};

fn a3w(e: a3::fault::Fault) -> Why {
    why(&e.name(), e.detail())
}

#[derive(Default)]
pub struct Swept {
    pub enrolled: Vec<String>,
    pub marked: Vec<String>,
    pub ticked: Vec<(String, State)>,
    pub sent: Vec<(String, &'static str, String)>,
    pub failed: Vec<(String, Why)>,
    pub foreign: u64,
}

fn bound(p: &k3::listing::Listing, receipt: &[u8]) -> Result<String, Why> {
    let r = k3::receipt::receipt(receipt).map_err(|f| why(f.token.name(), "回执无效"))?;
    let b =
        k3::receipt::bind_receipt(p, &r).map_err(|v| why(v.name, "回执与订单不匹配"))?;
    Ok(b.verdict().to_string())
}


fn enroll_one(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    c: &a3::scan::Case,
) -> Result<String, Why> {
    let id = c.case_id[2..18].to_string();
    let mut o = Order {
        id: id.clone(),
        case_id: c.case_id.clone(),
        instance: c.instance.to_ascii_lowercase(),
        buyer_key: c.buyer.to_ascii_lowercase(),
        buyer_payout: String::new(),
        verdict: String::new(),
        token: String::new(),
        need: String::new(),
        open_by: 0,
        candidate: 0,
        expiry: 0,
        state: State::Deployed,
        balance: String::new(),
        gap: String::new(),
        note: String::new(),
        notice: String::new(),
        notice_at: 0,
        deploy_tx: c.tx.clone(),
        expire_tx: String::new(),
        source: "scan".into(),
        opened: c.block,
        extra: Vec::new(),
    };
    let r = match a3::scan::rebuild(w.t, w.table, &c.tx, w.chain_id, w.logic, &c.case_id)
        .map_err(a3w)?
    {
        a3::scan::Rebuild::Done(r) => r,
        a3::scan::Rebuild::Untraced => {
            o.note = a3::scan::SAY_UNTRACED.to_string();
            order::save(data, writer, now, me, &o, &[])?;
            return Ok(id);
        }
    };
    let ao = &r.init.order;
    o.buyer_payout = ao.buyer_payout.to_ascii_lowercase();
    o.token = ao.offer.token.to_ascii_lowercase();
    o.need = ao.offer.amount.to_dec();
    o.open_by = ao.open_by;
    o.expiry = ao.offer.expiry;
    crate::units::fill(data, me, Some(w), &mut o, &r.terms_hash);
    let (found, unread) = match crate::paste::archived(data, me, &r.terms_hash)? {
        Some(x) => (Some(x), None),
        None => match posted_back(w, &r.terms_hash, &ao.offer.seller_key) {
            Ok(x) => (x, None),
            Err((n, d)) => (None, Some(format!("{n}:{d}"))),
        },
    };
    let Some((p, pb, tb)) = found else {
        o.note = match unread {
            None => format!("商品归档里没有 termsHash {} 那一版的清单与条款,公告板上也没有,补不出回执", r.terms_hash),
            Some(why) => format!("商品归档里没有 termsHash {} 那一版的清单与条款,{BOARD_UNREAD}:{why}", r.terms_hash),
        };
        order::save(data, writer, now, me, &o, &[])?;
        return Ok(id);
    };
    let judges: Vec<String> = p.judges.iter().map(|j| j.judge.clone()).collect();
    let guilds: Vec<String> = p.guilds.iter().map(|g| g.address.clone()).collect();
    let seq = if eravon_terms::derive::zero_address().eq_ignore_ascii_case(&ao.offer.guild) {
        None
    } else {
        panel_index(w, &o.instance)
    };
    let receipt = match a3::scan::backfill(&r, w.chain_id, &judges, &guilds, seq) {
        Ok(b) => b,
        Err(e) => {
            o.note = format!("补回执不成:{}:{}", e.name(), e.detail());
            order::save(data, writer, now, me, &o, &[])?;
            return Ok(id);
        }
    };
    match bound(&p, &receipt) {
        Ok(v) => o.verdict = v,
        Err((n, d)) => {
            o.note = format!("{n}:{d}");
            order::save(data, writer, now, me, &o, &[])?;
            return Ok(id);
        }
    }
    if let Ok(rr) = k3::receipt::receipt(&receipt) {
        o.candidate = rr.candidate;
    }
    order::save(
        data,
        writer,
        now,
        me,
        &o,
        &[
            (order::part_receipt(), receipt),
            (order::part_listing(), pb),
            (order::part_terms(), tb),
        ],
    )?;
    Ok(id)
}

pub const BOARD_UNREAD: &str = "公告板这一回读不成,下一趟扫描再读";

fn posted_back(w: &Wire<'_>, terms_hash: &str, seller: &str) -> Result<Option<(k3::listing::Listing, Vec<u8>, Vec<u8>)>, Why> {
    let th = eravon_standard::order::b32_of("termsHash", terms_hash).map_err(|e| why(&e.name(), e.detail()))?;
    let Some((post, listing)) = a3::board::listing_by_key(w.t, w.table, &th, seller, 0).map_err(a3w)? else { return Ok(None) };
    let terms = listing.terms_text.clone().into_bytes();
    if eravon_terms::doc::terms_hash(&terms) != th {
        return Ok(None);
    }
    Ok(Some((listing, post.document, terms)))
}

fn panel_index(w: &Wire<'_>, instance: &str) -> Option<u64> {
    let s = eravon_standard::chain::status(w.t, w.table, instance).ok()?;
    s.value.get(eravon_standard::chain::status_col::PANEL_INDEX)?.small().ok()
}

fn enroll(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    c: &a3::scan::Case,
) -> Result<Option<String>, Why> {
    enroll_one(data, writer, now, me, w, c).map(Some)
}


fn catch_up(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    opens: send::Opens,
    mut post: Option<&mut dyn Post>,
    out: &mut Swept,
) -> Result<(), Why> {
    let all = order::all(data, me)?;
    let polled = |o: &order::Order| o.state.open() || (matches!(o.state, State::Expired | State::Revoked) && o.expire_tx.is_empty());
    let deals: Vec<(&order::Order, crate::poll::Deal)> = all.iter().filter(|o| polled(o)).filter_map(|o| crate::poll::deal(data, me, o, w).ok().map(|d| (o, d))).collect();
    crate::poll::ahead_rounds(w, &deals.iter().map(|(o, d)| (*o, d)).collect::<Vec<_>>());
    for o in all {
        if !polled(&o) {
            continue;
        }
        let t = match send::tick(
            data,
            writer,
            now,
            me,
            w,
            &o.id,
            opens,
            post.as_mut().map(|p| &mut **p as &mut dyn Post),
        ) {
            Ok(t) => t,
            Err(e) => {
                out.failed.push((o.id, e));
                continue;
            }
        };
        if let Some((what, tx)) = t.sent {
            out.sent.push((o.id.clone(), what, tx));
        }
        out.ticked.push((o.id, t.order.state));
    }
    Ok(())
}


pub(crate) fn party_keys(data: &str, me: &str) -> Result<Vec<String>, Why> {
    eravon_vault::keyset::seller(data, me).map_err(|e| why(e.name(), e.detail()))
}


#[allow(clippy::too_many_arguments)]
pub fn startup(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    from: u64,
    kept: &[a3::scan::Case],
    opens: send::Opens,
    post: Option<&mut dyn Post>,
) -> Result<Swept, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let mut cases: Vec<a3::scan::Case> = kept.to_vec();
    let mut foreign = 0;
    let parties = party_keys(data, me)?;
    let sweep = a3::scan::by_keys(w.t, w.table, w.logic, a3::scan::Side::Seller, &parties, from, eravon_chain::seam::Upto::Latest, &[]).map_err(a3w)?;
    foreign += sweep.foreign;
    for c in sweep.cases {
        if !cases.iter().any(|k| k.instance.eq_ignore_ascii_case(&c.instance)) {
            cases.push(c);
        }
    }
    let mut out = Swept {
        foreign,
        ..Swept::default()
    };
    let known = order::all(data, me)?;
    let enrolling: Vec<String> = cases
        .iter()
        .filter(|c| match known.iter().find(|o| o.instance.eq_ignore_ascii_case(&c.instance)) {
            None => true,
            Some(o) => o.state == State::Deployed && o.verdict.is_empty() && o.note.contains(BOARD_UNREAD),
        })
        .map(|c| c.tx.clone())
        .collect();
    eravon_chain::read::ahead_transactions(w.t, w.table, &enrolling);
    for c in &cases {
        match known
            .iter()
            .find(|o| o.instance.eq_ignore_ascii_case(&c.instance))
        {
            Some(o) if o.state == State::Deployed && o.verdict.is_empty() && o.note.contains(BOARD_UNREAD) => {
                if let Some(id) = enroll(data, writer, now, me, w, c)? {
                    out.enrolled.push(id);
                }
            }
            Some(o) if o.state == State::Deployed => {
                if o.opened != c.block {
                    let mut o = o.clone();
                    o.opened = c.block;
                    order::save(data, writer, now, me, &o, &[])?;
                }
            }
            Some(o) => {
                let mut o = o.clone();
                let tx = if o.deploy_tx.is_empty() { c.tx.clone() } else { String::new() };
                o.deployed(&tx);
                o.opened = c.block;
                order::save(data, writer, now, me, &o, &[])?;
                out.marked.push(o.id);
            }
            None => {
                if let Some(id) = enroll(data, writer, now, me, w, c)? {
                    out.enrolled.push(id);
                }
            }
        }
    }
    catch_up(data, writer, now, me, w, opens, post, &mut out)?;
    Ok(out)
}
