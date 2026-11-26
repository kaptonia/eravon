use eravon_literals::json::Out;

use crate::endpoints::Table;
use crate::fault::{Fault, R};
use crate::read::{self, Reading};
use crate::seam;
use crate::transport::Transport;

#[derive(Clone, Debug)]
pub struct Facts {
    pub estimated: u64,
    pub gas_price: String,
    pub base_fees: Vec<String>,
}

pub fn estimate<T: Transport>(
    t: &T,
    table: &Table,
    from: Option<&str>,
    to: &str,
    value: Option<&str>,
    data: Option<&str>,
) -> R<(u64, Reading)> {
    let tx = estimate_tx(from, to, value, data)?;
    let head = read::height(t, table)?.0;
    estimate_at(t, table, head, tx)
}

pub fn estimate_pinned<T: Transport>(t: &T, table: &Table, head: u64, from: Option<&str>, to: &str, value: Option<&str>, data: Option<&str>) -> R<(u64, Reading)> {
    estimate_at(t, table, head, estimate_tx(from, to, value, data)?)
}

fn estimate_tx(from: Option<&str>, to: &str, value: Option<&str>, data: Option<&str>) -> R<Vec<(&'static str, Out)>> {
    seam::need_address(to)?;
    let mut tx: Vec<(&'static str, Out)> = vec![("to", Out::S(to.to_string()))];
    if let Some(f) = from {
        seam::need_address(f)?;
        tx.push(("from", Out::S(f.to_string())));
    }
    if let Some(v) = value {
        seam::need_quantity("value", v)?;
        tx.push(("value", Out::S(v.to_string())));
    }
    if let Some(d) = data {
        seam::need_hex_bytes("calldata", d)?;
        tx.push(("data", Out::S(d.to_string())));
    }
    Ok(tx)
}

fn estimate_at<T: Transport>(t: &T, table: &Table, head: u64, tx: Vec<(&'static str, Out)>) -> R<(u64, Reading)> {
    let mut kept: Option<Reading> = None;
    let n = crate::send::estimate_gas(
        || Ok(head),
        |_: &eravon::json::Value| {
            let r = read::read_agreed(t, table, seam::M_ESTIMATE_GAS, vec![Out::O(tx.clone()), Out::S(at_block(head))])?;
            let said = read::as_str(&r)?;
            kept = Some(r);
            Ok(eravon::json::Value::Str(said))
        },
        crate::ask::value_of(&Out::O(tx.clone())),
        |e: &Fault| matches!(e, Fault::Transport(_)),
    )
    .map_err(|e| match e {
        crate::send::NoGas::Network(f) | crate::send::NoGas::Refused(f) => by_the_table(f),
        crate::send::NoGas::NotText(v) => Fault::Endpoint(format!("{} 该答一个数量串:{v:?}", seam::M_ESTIMATE_GAS)),
        crate::send::NoGas::Unreadable(x) => Fault::Endpoint(format!("{} 答的不是数量:{x}", seam::M_ESTIMATE_GAS)),
        crate::send::NoGas::OverCap(n) => Fault::Execution(format!("估出的气 {n} 超过一笔交易的上限 {}", crate::send::GAS_LIMIT)),
    })?;
    let r = kept.ok_or_else(|| Fault::Endpoint(format!("{} 没有读数", seam::M_ESTIMATE_GAS)))?;
    Ok((n, r))
}

fn by_the_table(f: Fault) -> Fault {
    match crate::fault::refusal(&f) {
        Some(r) if r.about_the_call() => Fault::Execution(f.detail()),
        Some(_) => Fault::Endpoint(f.detail()),
        None => f,
    }
}

fn at_block(head: u64) -> String {
    seam::quantity(head)
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipFrom {
    Chain,
    Fallback,
}

impl TipFrom {
    pub const fn word(self) -> &'static str {
        match self {
            TipFrom::Chain => "chain",
            TipFrom::Fallback => "fallback",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fees {
    pub cap: String,
    pub tip: String,
    pub tip_from: TipFrom,
}

impl Fees {
    pub fn of(base_fee: Option<u128>, tip: Option<u128>) -> Fees {
        let base = base_fee.and_then(|b| u64::try_from(b).ok());
        let tip_read = tip.map(|x| u64::try_from(x).unwrap_or(u64::MAX));
        let f = crate::send::Fees::of(base, tip_read);
        let tip_from = if base.is_some() && tip_read.is_some() { TipFrom::Chain } else { TipFrom::Fallback };
        Fees { cap: f.max_fee.to_string(), tip: f.priority.min(f.max_fee).to_string(), tip_from }
    }
}

fn tip_read(history: &eravon::json::Value) -> Option<u128> {
    crate::send::tip_of(history).map(u128::from)
}


pub fn suggested_fees<T: Transport>(t: &T, table: &Table) -> R<Fees> {
    let (head, _) = read::height(t, table)?;
    suggested_fees_at(t, table, head)
}

pub fn suggested_fees_at<T: Transport>(t: &T, table: &Table, head: u64) -> R<Fees> {
    let (t1, b1, t2, b2) = (t.clone(), table.clone(), t.clone(), table.clone());
    let (block, history) = read::both(
        move || read::read_agreed(&t1, &b1, seam::M_GET_BLOCK, vec![Out::S(seam::quantity(head)), Out::B(false)]).ok(),
        move || read::read_agreed_value(&t2, &b2, seam::M_FEE_HISTORY, &crate::send::tip_params(head)).ok(),
    );
    let base = block.as_ref().and_then(|r| crate::ask::core_of(&r.value)).and_then(|b| crate::send::base_fee_of(&b)).map(u128::from);
    let tip = history.as_ref().and_then(|r| crate::ask::core_of(&r.value)).and_then(|h| tip_read(&h));
    Ok(Fees::of(base, tip))
}

pub fn price<T: Transport>(t: &T, table: &Table) -> R<(String, Reading)> {
    let r = read::read_agreed(t, table, seam::M_GAS_PRICE, vec![])?;
    let dec = seam::read_big_decimal(&read::as_str(&r)?)?;
    Ok((dec, r))
}

pub fn recent_base_fees<T: Transport>(t: &T, table: &Table, n: u64) -> R<Vec<String>> {
    let r = read::read_agreed(t, table, seam::M_BLOCK_NUMBER, vec![])?;
    let head = seam::read_quantity(&read::as_str(&r)?)?;
    read::ahead_blocks(t, table, &(0..n).map_while(|i| head.checked_sub(i)).collect::<Vec<_>>());
    let mut out = Vec::new();
    for i in 0..n {
        let Some(h) = head.checked_sub(i) else { break };
        let b = read::read_agreed(
            t,
            table,
            seam::M_GET_BLOCK,
            vec![Out::S(seam::quantity(h)), Out::B(false)],
        )?;
        if let Some(f) = b.value.get("baseFeePerGas").and_then(|x| x.as_str()) {
            out.push(seam::read_big_decimal(f)?);
        }
    }
    Ok(out)
}

pub fn verdict(_f: &Facts) -> Option<bool> {
    None
}


#[cfg(test)]
mod tests {
    use super::*;
    use eravon_literals::constants as k7;

    fn read(s: &str) -> eravon::json::Value {
        crate::ask::params_text(s).unwrap()
    }

    #[test]
    fn the_tip_is_the_median_paid_and_zero_when_none_paid() {
        assert_eq!(tip_read(&read(r#"{"reward":[["0x0"],["0x64"],["0x12c"],["0xc8"]]}"#)), Some(200), "空块不计,其余取中位数");
        assert_eq!(tip_read(&read(r#"{"reward":[["0x0"],["0x0"]]}"#)), Some(0), "全零即零");
        for unread in [r#"{"reward":[]}"#, r#"{"oldestBlock":"0x1"}"#, r#"{"reward":"x"}"#, r#"{"reward":[["0x1","0x2"]]}"#, r#"{"reward":[["zz"]]}"#] {
            assert_eq!(tip_read(&read(unread)), None, "{unread}");
        }
    }

    #[test]
    fn the_fees_follow_the_one_rule_and_fall_back_by_name() {
        assert_eq!(Fees::of(Some(16), Some(200)), Fees { cap: "232".into(), tip: "200".into(), tip_from: TipFrom::Chain });
        assert_eq!(Fees::of(Some(16), Some(0)), Fees { cap: "32".into(), tip: "0".into(), tip_from: TipFrom::Chain }, "全零小费答 0");
        assert_eq!(Fees::of(Some(1), Some(10 * k7::TIP_CAP_WEI)).tip, k7::TIP_CAP_WEI.to_string(), "封顶");
        assert_eq!(Fees::of(Some(16), None), Fees { cap: (32 + k7::TIP_FALLBACK_WEI).to_string(), tip: k7::TIP_FALLBACK_WEI.to_string(), tip_from: TipFrom::Fallback });
        assert_eq!(Fees::of(None, Some(5)), Fees { cap: k7::FEE_CAP_FALLBACK_WEI.to_string(), tip: k7::TIP_FALLBACK_WEI.to_string(), tip_from: TipFrom::Fallback }, "底费读不出:整对保底");
        assert_eq!(crate::send::base_fee_of(&read(r#"{"baseFeePerGas":"0x10"}"#)), Some(16));
        assert_eq!(crate::send::base_fee_of(&read(r#"{"number":"0x10"}"#)), None);
    }

    #[test]
    fn the_fee_literals_are_the_familys_numbers() {
        assert_eq!(k7::TIP_CAP_WEI, u128::from(crate::send::PRIORITY_FEE));
        assert_eq!(k7::TIP_FALLBACK_WEI, u128::from(crate::send::PRIORITY_FEE));
        assert_eq!(k7::FEE_CAP_FALLBACK_WEI, u128::from(crate::send::MAX_FEE));
        assert_eq!(k7::TIP_HISTORY_BLOCKS, crate::send::TIP_BLOCKS);
    }

    #[derive(Clone)]
    struct Node(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl Node {
        fn answer_one(&self, _url: &crate::seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            self.0.lock().unwrap().push(b.clone());
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let result = if b.contains(seam::M_CHAIN_ID) {
                r#""0x7a69""#
            } else if b.contains(seam::M_BLOCK_NUMBER) {
                r#""0x30""#
            } else if b.contains(seam::M_FEE_HISTORY) {
                r#"{"oldestBlock":"0x1d","gasUsedRatio":[0.0,0.25],"reward":[["0x0"],["0x64"]]}"#
            } else {
                r#"{"baseFeePerGas":"0x10","number":"0x30","timestamp":"0x1"}"#
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    impl Transport for Node {
        fn post(&self, url: &crate::seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            crate::ask::each_question(body, |b| self.answer_one(url, b))
        }
    }

    #[test]
    fn the_fees_are_read_at_the_pinned_head() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let n = Node(Default::default());
        let table = Table::new(vec!["http://127.0.0.1:1".into()]).unwrap();
        assert_eq!(suggested_fees(&n, &table).unwrap(), Fees { cap: (32 + 100).to_string(), tip: "100".into(), tip_from: TipFrom::Chain });
        let asked = n.0.lock().unwrap().clone();
        let history = asked.iter().find(|b| b.contains(seam::M_FEE_HISTORY)).expect("问了费史");
        assert!(history.contains("\"0x30\"") && !history.contains("latest"), "费史钉头块:{history}");
        assert!(asked.iter().any(|b| b.contains(seam::M_GET_BLOCK) && b.contains("\"0x30\"")), "底费钉头块");
    }

    #[derive(Clone)]
    struct Est(&'static str, std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl Est {
        fn answer_one(&self, _url: &crate::seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            self.1.lock().unwrap().push(b.clone());
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let result = if b.contains(seam::M_CHAIN_ID) {
                r#""0x7a69""#
            } else if b.contains(seam::M_BLOCK_NUMBER) {
                r#""0x30""#
            } else {
                self.0
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    impl Transport for Est {
        fn post(&self, url: &crate::seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            crate::ask::each_question(body, |b| self.answer_one(url, b))
        }
    }

    #[test]
    fn the_estimate_is_the_familys_rule_at_the_pinned_head() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let table = Table::new(vec!["http://127.0.0.1:1".into()]).unwrap();
        let to = "0x00000000000000000000000000000000000000c0";
        let from = "0x0000000000000000000000000000000000000001";
        let n = Est(r#""0x5208""#, Default::default());
        assert_eq!(estimate(&n, &table, Some(from), to, Some("0x1"), Some("0xabcd")).unwrap().0, 21_000);
        let asked = n.1.lock().unwrap().clone();
        let q = asked.iter().find(|b| b.contains(seam::M_ESTIMATE_GAS)).expect("问了估气");
        assert!(q.contains("\"0x30\"") && q.contains("\"value\":\"0x1\"") && q.contains("\"data\":\"0xabcd\"") && q.contains(from), "{q}");
        let over = estimate(&Est(r#""0x1000001""#, Default::default()), &table, None, to, None, None).unwrap_err();
        assert!(over.name() == "E_CHAIN_EXECUTION" && over.detail().contains("16777217"), "{over:?}");
        assert_eq!(estimate(&Est(r#""0x1000000""#, Default::default()), &table, None, to, None, None).unwrap().0, crate::send::GAS_LIMIT, "恰在上限照收");
        for bad in ["5", r#""0xzz""#] {
            assert!(estimate(&Est(bad, Default::default()), &table, None, to, None, None).is_err(), "{bad}");
        }
    }
}
