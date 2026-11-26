#![forbid(unsafe_code)]

use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed as k1;
use eravon_literals::json::Out;
use eravon_recheck::read::{self, Event};
use eravon_standard as a1;
use eravon_terms as k2;
use eravon_watch::{Why, why};

pub const V_STATEMENT: &str = "eravon.statement/1";
pub const V_COMPLIANCE: &str = "eravon.compliance/1";
pub const E_NO_COMPLIANCE: &str = "E_R2_NO_COMPLIANCE";

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::R2;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub case_id: String,
    pub instance: String,
    pub state: String,
    pub cause: Option<u64>,
    pub token: String,
    pub price: String,
    pub premium: String,
    pub deposit: String,
    pub lf: String,
    pub lf_source: &'static str,
    pub judge_fee: String,
    pub split: Option<[String; 4]>,
    pub received: Vec<(&'static str, String, String)>,
    pub rows: Vec<(String, String, u64, u64)>,
}

pub fn cause_word(c: u64) -> &'static str {
    match c {
        0 => "T1 ran out",
        1 => "offer clock ran out",
        2 => "expiry branch",
        3 => "seller refund",
        4 => "buyer cancel",
        _ => "",
    }
}

fn kv<'a>(v: &'a k1::Value, k: &str) -> Option<&'a k1::Value> {
    v.get(k)
}

fn s_of(v: &k1::Value, k: &str) -> String {
    kv(v, k).and_then(k1::Value::as_str).unwrap_or_default().to_string()
}

fn n_of(v: &k1::Value, k: &str) -> u64 {
    kv(v, k).and_then(k2::form::as_int).unwrap_or_default()
}

pub fn statement<T: Transport>(t: &T, table: &Table, index: &[u8], listing: &[u8], receipt: &[u8]) -> Result<Statement, Why> {
    mark::mark(mark::FEATURE);
    let idx = eravon_dossier::index::index(index).map_err(|f| why("E_R2_DOSSIER", String::from_utf8_lossy(&eravon_galeed::schema::answer_bytes(&f.to_value(true))).into_owned()))?;
    let v = k1::canon::accept(index).map_err(|e| why("E_R2_DOSSIER", format!("{e:?}")))?;
    let p = eravon_link::listing::listing(listing).map_err(|f| why("E_R2_LISTING", format!("{:?}", f.to_value(true))))?;
    let rows: Vec<(String, String, u64, u64)> = kv(&v, "transactions")
        .and_then(k2::form::as_arr)
        .unwrap_or_default()
        .iter()
        .map(|r| (s_of(r, "kind"), s_of(r, "tx"), n_of(r, "block"), n_of(r, "timestamp")))
        .collect();
    let split = kv(&v, "terminal").map(|t| [s_of(t, "buyerPaid"), s_of(t, "sellerPaid"), s_of(t, "judgePaid"), s_of(t, "bondToBuyer")]);
    let opened = rows.iter().find(|r| r.0 == "init").map_or(0, |r| eravon_chain::read::instance_from(r.2));
    let evs = read::instance_events(t, table, &idx.instance, opened).map_err(|e| why(&e.name(), e.detail()))?;
    let (lf, lf_source) = match evs.iter().find(|e| e.ev.name == "Priced") {
        Some(e) => (word(e, 1)?, "Priced"),
        None => match a1::chain::fee_quoted(t, table, &idx.instance) {
            Ok(r) => {
                let lf = r.value.get(a1::sol::src_abi::ERAVON_FEE_QUOTED_COL_LF).ok_or_else(|| why("E_A1_ABI", "feeQuoted 缺 lf"))?;
                (k2::u256::U256::from_be(&lf.word().map_err(|e| why(&e.name(), e.detail()))?).to_dec(), "feeQuoted")
            }
            Err(_) => ("0".to_string(), "feeQuoted"),
        },
    };
    let cause = match evs.iter().find(|e| e.ev.name == "Refunded") {
        Some(e) => Some(e.ev.at(0).and_then(|v| v.small()).map_err(|f| why(&f.name(), f.detail()))?),
        None => None,
    };
    let buyer = eravon_link::receipt::receipt(receipt).map(|r| r.buyer_key).unwrap_or_default();
    let deposit = pulled(t, table, &p, &buyer, &idx.instance)?;
    let premium = match (&deposit, k2::u256::U256::from_dec(&lf)) {
        (Some(d), Some(l)) if d.cmp_u(&l) != std::cmp::Ordering::Less => d.overflowing_sub(&l).0.to_dec(),
        _ => String::new(),
    };
    let judge = kv(&v, "seat").map(|s| s_of(s, "judge")).unwrap_or_default();
    let receipt = eravon_link::receipt::receipt(receipt).map_err(|f| why("E_R2_RECEIPT", format!("{:?}", f.to_value(true))))?;
    let parties: [(&'static str, String); 4] = [
        ("buyerPayout", receipt.buyer_payout.to_ascii_lowercase()),
        ("sellerPayout", p.order.seller_payout.to_ascii_lowercase()),
        ("sellerFund", p.order.seller_fund.to_ascii_lowercase()),
        ("judge", judge.to_ascii_lowercase()),
    ];
    let mut received = Vec::new();
    for (role, who) in parties {
        if who.is_empty() {
            continue;
        }
        received.push((role, who.clone(), swept_to(&evs, &who, &p.token)?));
    }
    let state = state_of(idx.terminal.as_deref(), &rows);
    Ok(Statement {
        case_id: idx.case_id.clone(),
        instance: idx.instance.clone(),
        state,
        cause,
        token: p.token.to_ascii_lowercase(),
        price: p.order.amount.to_dec(),
        premium,
        deposit: deposit.map(|d| d.to_dec()).unwrap_or_default(),
        lf,
        lf_source,
        judge_fee: split.as_ref().map(|m| m[2].clone()).unwrap_or_default(),
        split: split_of(split),
        received,
        rows,
    })
}

fn pulled<T: Transport>(t: &T, table: &Table, p: &eravon_link::listing::Listing, buyer: &str, instance: &str) -> Result<Option<k2::u256::U256>, Why> {
    let topic = |a: &str| eravon_scan::log::address_topic(a).map_err(|e| why(&e.name(), e.detail()));
    let topics = [Some(topic(&p.order.seller_key)?), Some(topic(&p.order.seller_fund)?)];
    let keys: Vec<String> = [p.order.seller_key.clone(), buyer.to_string()].into_iter().filter(|k| !k.is_empty()).collect();
    for l in read::logic_logs(t, table, &p.logic, &keys, &a1::sol::PULLED_EV.topic0(), &topics).map_err(|e| why(&e.name(), e.detail()))? {
        let Ok(e) = a1::event::decode(&l.topics, &l.data) else { continue };
        let names = e.at(3).ok().and_then(|v| v.addr().ok()).is_some_and(|a| a1::order::hex_of(&a).eq_ignore_ascii_case(instance));
        if names {
            return Ok(Some(k2::u256::U256::from_be(&e.at(4).and_then(|v| v.word()).map_err(|f| why(&f.name(), f.detail()))?)));
        }
    }
    Ok(None)
}

fn swept_to(evs: &[Event], who: &str, token: &str) -> Result<String, Why> {
    let mut acc = k2::u256::U256::ZERO;
    for e in evs.iter().filter(|e| e.ev.name == "Swept") {
        let to = e.log.topic_address(1).map_err(|f| why(&f.name(), f.detail()))?;
        let tk = e.ev.at(1).and_then(|v| v.addr()).map_err(|f| why(&f.name(), f.detail()))?;
        if !to.eq_ignore_ascii_case(who) || !a1::order::hex_of(&tk).eq_ignore_ascii_case(token) {
            continue;
        }
        let w = k2::u256::U256::from_be(&e.ev.at(2).and_then(|v| v.word()).map_err(|f| why(&f.name(), f.detail()))?);
        let (n, over) = acc.overflowing_add(&w);
        if over {
            return Err(why("E_R2_FORM", "Swept 之和溢出".to_string()));
        }
        acc = n;
    }
    Ok(acc.to_dec())
}

fn word(e: &Event, i: usize) -> Result<String, Why> {
    Ok(k2::u256::U256::from_be(&e.ev.at(i).and_then(|v| v.word()).map_err(|f| why(&f.name(), f.detail()))?).to_dec())
}

fn split_of(m: Option<[String; 4]>) -> Option<[String; 4]> {
    m
}


fn state_of(terminal: Option<&str>, rows: &[(String, String, u64, u64)]) -> String {
    match terminal {
        Some(t) => t.to_string(),
        None if rows.iter().any(|r| r.0 == "expire") => "expired".to_string(),
        None => "open".to_string(),
    }
}


pub fn statement_bytes(s: &Statement) -> Vec<u8> {
    let mut m = vec![
        ("caseId", Out::S(s.case_id.clone())),
        ("deposit", Out::S(s.deposit.clone())),
        ("instance", Out::S(s.instance.clone())),
        ("judgeFee", Out::S(s.judge_fee.clone())),
        ("lf", Out::S(s.lf.clone())),
        ("lfSource", Out::S(s.lf_source.to_string())),
        ("premium", Out::S(s.premium.clone())),
        ("price", Out::S(s.price.clone())),
        (
            "received",
            Out::A(s.received.iter().map(|(role, who, amount)| Out::O(vec![("address", Out::S(who.clone())), ("amount", Out::S(amount.clone())), ("role", Out::S((*role).to_string()))])).collect()),
        ),
        ("state", Out::S(s.state.clone())),
        ("token", Out::S(s.token.clone())),
        (
            "transactions",
            Out::A(s.rows.iter().map(|r| Out::O(vec![("block", Out::N(r.2)), ("kind", Out::S(r.0.clone())), ("timestamp", Out::N(r.3)), ("tx", Out::S(r.1.clone()))])).collect()),
        ),
        ("v", Out::S(V_STATEMENT.to_string())),
    ];
    if let Some(c) = s.cause {
        m.push(("cause", Out::O(vec![("code", Out::N(c)), ("word", Out::S(cause_word(c).to_string()))])));
    }
    if let Some(x) = &s.split {
        m.push((
            "split",
            Out::O(vec![
                ("bondToBuyer", Out::S(x[3].clone())),
                ("buyerPaid", Out::S(x[0].clone())),
                ("judgePaid", Out::S(x[2].clone())),
                ("sellerPaid", Out::S(x[1].clone())),
            ]),
        ));
    }
    eravon_literals::json::write(&Out::O(m))
}

pub fn compliance_bytes(index: &[u8]) -> Result<Vec<u8>, Why> {
    let v = k1::canon::accept(index).map_err(|e| why("E_R2_DOSSIER", format!("{e:?}")))?;
    let Some(c) = v.get("compliance") else {
        return Err(why(E_NO_COMPLIANCE, "条款里没有合规块:卷宗不带合规页"));
    };
    let mut m = match c {
        k1::Value::Obj(m) => m.clone(),
        _ => return Err(why("E_R2_DOSSIER", "卷宗的合规块不是对象")),
    };
    m.push(("caseId".into(), k1::Value::Str(s_of(&v, "caseId"))));
    m.push(("v".into(), k1::Value::Str(V_COMPLIANCE.into())));
    k1::canon::canonical(&k1::Value::Obj(m)).map_err(|t| why("E_R2_DOSSIER", format!("{t:?}")))
}


pub fn write(dest: &str, name: &str, bytes: &[u8]) -> Result<String, Why> {
    if !std::path::Path::new(dest).is_dir() {
        return Err(why(eravon_export::export::E_UNWRITABLE, format!("{dest} 不是一个目录")));
    }
    let at = eravon_export::seam::join(dest, name);
    std::fs::write(&at, bytes).map_err(|e| why(eravon_export::export::E_UNWRITABLE, format!("{dest} 写不进:{e}")))?;
    Ok(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_expired_order_without_a_terminal_is_expired() {
        let rows = vec![("expire".to_string(), "0x".to_string(), 1, 1)];
        assert_eq!(state_of(None, &rows), "expired");
        assert_eq!(state_of(None, &[]), "open");
        assert_eq!(state_of(Some("Ruled"), &rows), "Ruled");
    }

    #[test]
    fn every_refunded_cause_has_its_word() {
        for c in 0..=4 {
            assert!(!cause_word(c).is_empty(), "{c}");
        }
        assert_eq!(cause_word(4), "buyer cancel");
        assert_eq!(cause_word(5), "");
    }

    #[test]
    fn the_export_words_carry_no_guarantee() {
        for w in [V_STATEMENT, V_COMPLIANCE, E_NO_COMPLIANCE] {
            assert!(!w.contains("保证") && !w.to_ascii_lowercase().contains("guarantee"), "{w}");
        }
    }
}
