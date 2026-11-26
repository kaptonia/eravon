use eravon_orders::detail;
use eravon_orders::table::{self, Table};

use crate::{Why, Wire};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    pub at: u64,
    pub order: String,
    pub say: String,
    pub tx: String,
    pub unread: Option<Why>,
}

impl Act {
    pub fn order_of(a: &Act, b: &Act) -> std::cmp::Ordering {
        b.unread.is_some().cmp(&a.unread.is_some()).then(b.at.cmp(&a.at)).then(b.tx.cmp(&a.tx))
    }
}

fn rows_of(w: &Wire<'_>, data: &str, me: &str, r: &table::Row) -> Result<Vec<Act>, Why> {
    let o = detail::order_of(data, me, r);
    let mut out: Vec<Act> = detail::record(w, r, o.as_ref())?
        .into_iter()
        .filter(|m| !m.tx.is_empty())
        .map(|m| Act { at: m.at, order: table::name(r), say: m.say, tx: m.tx, unread: None })
        .collect();
    let mine: Vec<Act> = local(w, r, o.as_ref())?.into_iter().filter(|x| !out.iter().any(|y| y.tx.eq_ignore_ascii_case(&x.tx))).collect();
    out.extend(mine);
    Ok(out)
}


fn local(w: &Wire<'_>, r: &table::Row, o: Option<&eravon_watch::order::Order>) -> Result<Vec<Act>, Why> {
    let Some(o) = o else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for (say, tx) in [("部署", &o.deploy_tx), ("过期分支,退回买家", &o.expire_tx)] {
        if tx.is_empty() {
            continue;
        }
        let Some(rc) = eravon_chain::sending::receipt(w.t, w.table, tx).map_err(|e| crate::why(e.name(), e.detail()))? else { continue };
        out.push(Act { at: detail::chain_time(w, rc.block)?, order: table::name(r), say: say.to_string(), tx: tx.clone(), unread: None });
    }
    Ok(out)
}

fn ahead_local(w: &Wire<'_>, data: &str, me: &str, rows: &[table::Row]) {
    if !eravon_chain::read::holds_ahead(w.table) {
        return;
    }
    let txs: Vec<String> = rows
        .iter()
        .filter_map(|r| detail::order_of(data, me, r))
        .flat_map(|o| [o.deploy_tx, o.expire_tx])
        .filter(|tx| !tx.is_empty())
        .collect();
    eravon_chain::read::ahead_receipts(w.t, w.table, &txs);
    detail::ahead_stamps(w, txs.iter().filter_map(|tx| eravon_chain::sending::receipt(w.t, w.table, tx).ok().flatten()).map(|rc| rc.block));
}

pub fn recent(w: &Wire<'_>, data: &str, me: &str, t: &Result<Table, Why>, n: usize) -> Result<Vec<Act>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let t = t.as_ref().map_err(Clone::clone)?;
    let mut all: Vec<Act> = Vec::new();
    let mut unread: Vec<Act> = Vec::new();
    ahead_local(w, data, me, &t.rows);
    for r in &t.rows {
        match rows_of(w, data, me, r) {
            Ok(v) => all.extend(v),
            Err(e) => unread.push(Act { at: 0, order: table::name(r), say: String::new(), tx: String::new(), unread: Some(e) }),
        }
    }
    all.sort_by(Act::order_of);
    all.truncate(n);
    unread.extend(all);
    Ok(unread)
}
