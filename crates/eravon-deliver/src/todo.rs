use eravon_link as k3;
use eravon_literals::literals as lit;
use eravon_standard as a1;
use eravon_terms::u256::U256;
use eravon_watch::order::{self, Order};

use crate::{Why, Wire, a1w, why};

pub const TODO_SAY: &str = "在 ZIKARON Desk 登记授权";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Todo {
    pub grantee: String,
    pub work: String,
    pub terms_hash: String,
    pub terminal: String,
}

fn due(terminal: &str, seller_paid: &U256) -> bool {
    (terminal == lit::TERMINAL_NAMES[1] || terminal == lit::TERMINAL_NAMES[3])
        && !seller_paid.is_zero()
}


fn word(v: &a1::abi::Val) -> Result<[u8; 32], Why> {
    v.word().map_err(a1w)
}

pub fn check(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    o: &Order,
    from: u64,
) -> Result<Option<Todo>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let (rows, _) = eravon_chain::read::logs(
        w.t,
        w.table,
        &o.instance,
        from.max(eravon_chain::read::instance_from(o.opened)),
        eravon_chain::seam::Upto::Latest,
    )
    .map_err(|e| why(e.name(), e.detail()))?;
    let mut closed: Option<(String, U256)> = None;
    let mut claimed: Option<String> = None;
    for row in &rows {
        let l = eravon_scan::log::Log::read(row).map_err(|e| why(&e.name(), e.detail()))?;
        let d = match a1::event::decode(&l.topics, &l.data) {
            Ok(d) => d,
            Err(_) => continue,
        };
        match d.name {
            "Closed" => {
                let t = U256::from_be(&word(d.at(0).map_err(a1w)?)?);
                let idx: usize = t.to_dec().parse().unwrap_or(usize::MAX);
                let name = lit::TERMINAL_NAMES
                    .get(idx)
                    .copied()
                    .unwrap_or("")
                    .to_string();
                closed = Some((name, U256::from_be(&word(d.at(2).map_err(a1w)?)?)));
            }
            "DeliveryClaimed" if claimed.is_none() => {
                claimed = Some(a1::order::hex_of(&word(d.at(0).map_err(a1w)?)?));
            }
            _ => {}
        }
    }
    let Some((terminal, seller_paid)) = closed else {
        return Ok(None);
    };
    if !due(&terminal, &seller_paid) {
        return Ok(None);
    }
    let work = crate::deliver::delivered(data, me, o)
        .map(|d| d.0)
        .filter(|m| !m.is_empty())
        .or(claimed)
        .unwrap_or_default();
    let receipt = order::part(data, me, &o.id, &order::part_receipt())?;
    let r = k3::receipt::receipt(&receipt)
        .map_err(|f| why(f.token.name(), "订单回执无效"))?;
    let t = Todo {
        grantee: o.buyer_key.clone(),
        work,
        terms_hash: r.terms_hash.clone(),
        terminal,
    };
    let mut o2 = o.clone();
    o2.set_extra("todo", TODO_SAY);
    o2.set_extra("todoGrantee", &t.grantee);
    o2.set_extra("todoWork", &t.work);
    o2.set_extra("todoTerms", &t.terms_hash);
    o2.set_extra("todoTerminal", &t.terminal);
    o2.set_extra("todoAt", &now.to_string());
    order::save(data, writer, now, me, &o2, &[])?;
    Ok(Some(t))
}

pub fn stored(o: &Order) -> Option<Todo> {
    o.extra("todo")?;
    Some(Todo {
        grantee: o.extra("todoGrantee").unwrap_or_default().to_string(),
        work: o.extra("todoWork").unwrap_or_default().to_string(),
        terms_hash: o.extra("todoTerms").unwrap_or_default().to_string(),
        terminal: o.extra("todoTerminal").unwrap_or_default().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_released_or_ruled_with_a_seller_share() {
        let n = |s: &str| U256::from_dec(s).expect("十进制");
        assert!(due("Released", &n("1")));
        assert!(due("Ruled", &n("1")));
        assert!(!due("Ruled", &n("0")));
        assert!(!due("Refunded", &n("1")));
        assert!(!due("Settled", &n("1")));
    }
}
