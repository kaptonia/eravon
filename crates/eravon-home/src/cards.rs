use eravon_dispute::view::CaseView;
use eravon_orders::table::{self, Row, Table};

use crate::{Why, Wire};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Income {
    pub token: String,
    pub decimals: Option<u64>,
    pub total: String,
    pub count: u64,
}

#[derive(Clone, Debug)]
pub struct Cards {
    pub active: Result<u64, Why>,
    pub income: Result<Vec<Income>, Why>,
    pub month: String,
}

fn this_month(closed_at: u64, month: &str) -> bool {
    eravon_orders::day::of(closed_at).starts_with(month)
}


fn unsynced(e: &Why) -> (Result<u64, Why>, Result<Vec<Income>, Why>) {
    (Err(e.clone()), Err(e.clone()))
}


pub fn cards(w: &Wire<'_>, data: &str, me: &str, t: &Result<Table, Why>, now: u64) -> Cards {
    crate::mark::mark(crate::mark::FEATURE);
    let month = eravon_orders::day::of(now).get(..7).unwrap_or_default().to_string();
    let rows = match t {
        Ok(t) => &t.rows,
        Err(e) => {
            let (active, income) = unsynced(e);
            return Cards { active, income, month };
        }
    };
    let active = rows.iter().filter(|r| !table::done(r)).count() as u64;
    Cards { active: Ok(active), income: income(w, data, me, rows, &month), month }
}

fn income(w: &Wire<'_>, data: &str, me: &str, rows: &[Row], month: &str) -> Result<Vec<Income>, Why> {
    let mut income: Vec<Income> = Vec::new();
    if eravon_chain::read::holds_ahead(w.table) {
        let ended: Vec<&Row> = rows.iter().filter(|r| r.chain.as_ref().and_then(closed).is_some()).collect();
        eravon_orders::detail::ahead_stamps(w, ended.iter().filter_map(|r| Some(closed(r.chain.as_ref()?)?.block)));
        let now: Vec<String> = ended
            .iter()
            .filter(|r| r.chain.as_ref().and_then(closed).and_then(|c| eravon_orders::detail::chain_time(w, c.block).ok()).is_some_and(|at| this_month(at, month)))
            .map(|r| r.instance.clone())
            .collect();
        eravon_standard::chain::ahead_stages(w.t, w.table, &now);
    }
    for r in rows {
        let Some((at, seller)) = closed_of(w, data, me, r, month)? else { continue };
        if !this_month(at, month) {
            continue;
        }
        let Some(seller) = seller else {
            return Err(crate::why("E_HOME_SHARE_UNTOLD", format!("实例 {} 已分账,本机没有它的条款,裁决或逾期那一档的卖家份额算不出", r.instance)));
        };
        let tok = r.token.to_ascii_lowercase();
        match income.iter_mut().find(|x| x.token == tok) {
            Some(x) => {
                x.total = add(&x.total, &seller);
                x.count += 1;
            }
            None => income.push(Income { token: tok, decimals: r.decimals, total: seller, count: 1 }),
        }
    }
    Ok(income)
}

fn closed_of(w: &Wire<'_>, _data: &str, _me: &str, r: &Row, month: &str) -> Result<Option<(u64, Option<String>)>, Why> {
    let Some(v) = r.chain.as_ref() else { return Ok(None) };
    let Some(c) = closed(v) else { return Ok(None) };
    let at = eravon_orders::detail::chain_time(w, c.block)?;
    let seller = if this_month(at, month) { buyer_bps(w, &r.instance).and_then(|bps| seller_share(&v.amount, bps)) } else { None };
    Ok(Some((at, seller)))
}

fn buyer_bps(w: &Wire<'_>, instance: &str) -> Option<u64> {
    let s = eravon_standard::chain::stage(w.t, w.table, instance).ok()?.value;
    s.ended.then_some(u64::from(s.buyer_bps))
}

fn seller_share(amount: &eravon_terms::u256::U256, bps: u64) -> Option<String> {
    let buyer = amount.mul_div_small(bps, 10_000)?;
    Some(amount.overflowing_sub(&buyer).0.to_dec())
}

fn closed(v: &CaseView) -> Option<&eravon_dispute::view::Ev> {
    v.events.iter().rev().find(|e| e.name == "Closed")
}

fn add(a: &str, b: &str) -> String {
    let (x, y) = (eravon_terms::u256::U256::from_dec(a), eravon_terms::u256::U256::from_dec(b));
    match (x, y) {
        (Some(x), Some(y)) => x.overflowing_add(&y).0.to_dec(),
        _ => a.to_string(),
    }
}
