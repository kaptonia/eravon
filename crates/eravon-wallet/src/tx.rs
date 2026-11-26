use eravon_cli::shared::Wire;
use eravon_literals::json::{In, Out};
use eravon_standard as a1;
use eravon_store::record::Kind as K;
use eravon_terms::u256::U256;

use crate::fault::{Fault, R};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pay,
    Open,
    Fee,
    Approve,
    Dispute,
    Confirm,
    Commit,
    Advance,
    Settle,
    Expire,
    Sweep,
    Send,
    Cancel,
}

impl Kind {
    pub const ALL: [Kind; 13] = [
        Kind::Pay,
        Kind::Open,
        Kind::Fee,
        Kind::Approve,
        Kind::Dispute,
        Kind::Confirm,
        Kind::Commit,
        Kind::Advance,
        Kind::Settle,
        Kind::Expire,
        Kind::Sweep,
        Kind::Send,
        Kind::Cancel,
    ];

    pub const fn word(self) -> &'static str {
        match self {
            Kind::Pay => "pay",
            Kind::Open => "open",
            Kind::Fee => "fee",
            Kind::Approve => "approve",
            Kind::Dispute => "dispute",
            Kind::Confirm => "confirm",
            Kind::Commit => "commit",
            Kind::Advance => "advance",
            Kind::Settle => "settle",
            Kind::Expire => "expire",
            Kind::Sweep => "sweep",
            Kind::Send => "send",
            Kind::Cancel => "cancel",
        }
    }

    pub const fn say(self) -> &'static str {
        match self {
            Kind::Pay => "付款",
            Kind::Open => "开案",
            Kind::Fee => "诉讼费",
            Kind::Approve => "授权",
            Kind::Dispute => "申请退款",
            Kind::Confirm => "确认收货",
            Kind::Commit => "举证",
            Kind::Advance => "到期戳",
            Kind::Settle => "和解",
            Kind::Expire => "过期退款",
            Kind::Sweep => "取回",
            Kind::Send => "转出",
            Kind::Cancel => "撤单",
        }
    }

    pub fn of(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.word() == s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub kind: Kind,
    pub order: String,
    pub token: String,
    pub amount: String,
}

impl Tag {
    pub fn of(kind: Kind, order: &str) -> Tag {
        Tag { kind, order: order.to_ascii_lowercase(), token: String::new(), amount: String::new() }
    }

    pub fn moving(kind: Kind, order: &str, token: &str, amount: &str) -> Tag {
        Tag { kind, order: order.to_ascii_lowercase(), token: token.to_ascii_lowercase(), amount: amount.to_string() }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Keep<'a> {
    pub data: &'a str,
    pub writer: &'a str,
    pub owner: &'a str,
}

pub use eravon_cli::deskops::receipts::{fee_total, Tx};
use eravon_cli::deskops::receipts::CHAIN_ID;

pub fn record_id(hash: &str) -> String {
    let h: String = hash.trim_start_matches("0x").to_ascii_lowercase().chars().filter(|c| c.is_ascii_hexdigit()).take(60).collect();
    format!("tx-{h}")
}

pub fn post(
    k: &Keep<'_>,
    w: &Wire,
    from: &str,
    key_of: &dyn Fn() -> eravon_cli::fault::R<[u8; 32]>,
    o: &eravon_cli::send::Opts,
    call: &a1::call::Call,
    value: &str,
    tag: &Tag,
) -> R<String> {
    crate::mark::mark(crate::mark::FEATURE);
    eravon_store::mirror::need_writable(k.data)?;
    let d = eravon_cli::send::deliver_key(key_of, o, w, from, call, value)?;
    let kept = keep(k, w, from, &call.to, &d.tx_hash, tag);
    match (d.outcome(), kept) {
        (Ok(tx), Ok(())) => Ok(tx),
        (Ok(tx), Err(e)) => Err(Fault::new("E_P7_KEEP", format!("{tx} 已上链,本机交易回执没记下:{}:{}", e.token, e.detail))),
        (Err((n, why)), _) => Err(Fault::new(n, why)),
    }
}

fn facts(w: &Wire, hash: &str) -> R<Option<(u64, u64, u64, U256, u64)>> {
    use eravon_chain::seam;
    let r = eravon_chain::read::read_agreed(&w.t, &w.table, seam::M_GET_RECEIPT, vec![Out::S(hash.to_string())])?;
    if matches!(r.value, In::Null) {
        return Ok(None);
    }
    let q = |v: &In, k: &str| v.get(k).and_then(|x| x.as_str()).and_then(|x| seam::read_quantity(x).ok());
    let shape = || Fault::new("E_P7_RECEIPT", format!("交易 {hash} 的回执缺栏位"));
    let block = q(&r.value, "blockNumber").ok_or_else(shape)?;
    let status = q(&r.value, "status").ok_or_else(shape)?;
    let gas_used = q(&r.value, "gasUsed").ok_or_else(shape)?;
    let price = r.value.get("effectiveGasPrice").and_then(|x| x.as_str()).and_then(|x| seam::read_big_decimal(x).ok()).and_then(|d| U256::from_dec(&d)).ok_or_else(shape)?;
    let hash_at = r.value.get("blockHash").and_then(|x| x.as_str()).map(str::to_string);
    let at = eravon_scan::scan::block_stamp_at(&w.t, &w.table, block, hash_at.as_deref()).map_err(|e| Fault::new(&e.name(), e.detail()))?;
    Ok(Some((block, status, gas_used, price, at)))
}

fn ahead_facts(w: &Wire, hashes: &[String]) {
    use eravon_chain::seam;
    if !eravon_chain::read::holds_ahead(&w.table) {
        return;
    }
    eravon_chain::read::ahead_receipts(&w.t, &w.table, hashes);
    let blocks: Vec<u64> = hashes
        .iter()
        .filter_map(|h| eravon_chain::read::read_agreed(&w.t, &w.table, seam::M_GET_RECEIPT, vec![Out::S(h.to_string())]).ok())
        .filter_map(|r| r.value.get("blockNumber").and_then(|x| x.as_str()).and_then(|x| seam::read_quantity(x).ok()))
        .collect();
    eravon_scan::scan::ahead_stamps(&w.t, &w.table, &blocks);
}

fn fee_of(gas_used: u64, price: &U256) -> String {
    let w = price.mul_wide(&U256::from_u64(gas_used));
    U256([w.0[0], w.0[1], w.0[2], w.0[3]]).to_dec()
}

fn keep(k: &Keep<'_>, w: &Wire, from: &str, to: &str, hash: &str, tag: &Tag) -> R<()> {
    write(k, w, from, to, hash, &tag.kind.word().to_string(), &tag.order, &tag.token, &tag.amount)
}


#[allow(clippy::too_many_arguments)]
fn write(k: &Keep<'_>, w: &Wire, from: &str, to: &str, hash: &str, kind: &str, order: &str, token: &str, amount: &str) -> R<()> {
    let f = facts(w, hash).ok().flatten();
    let now = match &f {
        Some(x) => x.4,
        None => eravon_chain::clock::now(&w.t, &w.table)?.0.timestamp,
    };
    let s = |x: &str| x.to_string();
    let mut fields = vec![
        (s("amount"), s(amount)),
        (s(CHAIN_ID), w.chain_id.to_string()),
        (s("from"), from.to_ascii_lowercase()),
        (s("hash"), hash.to_ascii_lowercase()),
        (s("kind"), s(kind)),
        (s("order"), order.to_ascii_lowercase()),
        (s("to"), to.to_ascii_lowercase()),
        (s("token"), token.to_ascii_lowercase()),
    ];
    if let Some((block, status, gas_used, price, at)) = &f {
        fields.push((s("at"), at.to_string()));
        fields.push((s("block"), block.to_string()));
        fields.push((s("fee"), fee_of(*gas_used, price)));
        fields.push((s("gasPrice"), price.to_dec()));
        fields.push((s("gasUsed"), gas_used.to_string()));
        fields.push((s("status"), status.to_string()));
    }
    fields.sort();
    let st = open_write_waiting(k, now)?;
    st.init_identity(k.owner, now)?;
    st.put(k.owner, K::TxReceipt, &record_id(hash), &fields, &[], now)?;
    Ok(())
}

fn open_write_waiting(k: &Keep<'_>, now: u64) -> R<eravon_vault::store::Store> {
    let mut left = 30;
    loop {
        match eravon_vault::store::open_write(k.data, k.writer, now) {
            Err(eravon_store::fault::Fault::Locked { .. }) if left > 0 => {
                left -= 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            r => return r.map_err(Fault::from),
        }
    }
}

pub fn list(data: &str, owner: &str) -> R<Vec<Tx>> {
    crate::mark::mark(crate::mark::FEATURE);
    eravon_cli::deskops::receipts::list(data, owner).map_err(|s| Fault::new(s.name, s.detail))
}

pub fn fill(k: &Keep<'_>, w: &Wire) -> R<(usize, Option<Fault>)> {
    let mut n = 0;
    let mut refused: Option<Fault> = None;
    let open: Vec<Tx> = list(k.data, k.owner)?.into_iter().filter(|t| !t.settled()).collect();
    ahead_facts(w, &open.iter().map(|t| t.hash.clone()).collect::<Vec<_>>());
    for t in open {
        match facts(w, &t.hash) {
            Ok(None) => continue,
            Ok(Some(_)) => {}
            Err(e) => {
                refused.get_or_insert(Fault::new(e.token, format!("交易 {}:{}", t.hash, e.detail)));
                continue;
            }
        }
        write(k, w, &t.from, &t.to, &t.hash, &t.kind, &t.order, &t.token, &t.amount)?;
        n += 1;
    }
    Ok((n, refused))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_ids_fit_the_store_name_gate() {
        let id = record_id(&format!("0x{}", "ab".repeat(32)));
        assert!(eravon_store::seam::name_ok(&id), "{id}");
        assert_eq!(id.len(), 63);
    }

    #[test]
    fn kinds_round_trip() {
        for k in Kind::ALL {
            assert_eq!(Kind::of(k.word()), Some(k));
        }
    }

    #[test]
    fn receipts_are_listed_by_the_network_they_were_sent_on() {
        let data = eravon_vault::testing::fresh();
        let now = 1_790_000_000;
        let mut v = eravon_vault::vault::create("48261537", eravon_store::vault::Seat::Buyer, "主钥", None, now).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut session = eravon_vault::session::Session::locked(&data);
        session.unlock("48261537").expect("开得了");
        let me = v.address.clone();
        let st = eravon_vault::store::open_write(&data, "eravon-test", now).expect("可写");
        st.init(&eravon_store::settings::Global::default()).expect("铺得下");
        st.init_identity(&me, now).expect("登记身份");
        let row = |st: &eravon_vault::store::Store, chain: Option<&str>, hash: &str| {
            let mut f = vec![("hash".to_string(), hash.to_string()), ("kind".to_string(), "pay".to_string())];
            if let Some(c) = chain {
                f.push((CHAIN_ID.to_string(), c.to_string()));
            }
            f.sort();
            st.put(&me, K::TxReceipt, &record_id(hash), &f, &[], now).expect("落档");
        };
        let (sep, main) = (format!("0x{}", "a1".repeat(32)), format!("0x{}", "f6".repeat(32)));
        row(&st, Some("11155111"), &sep);
        row(&st, Some("1"), &main);
        drop(st);
        for (chain, want) in [(11_155_111u64, &sep), (1, &main)] {
            assert!(eravon_literals::constants::select_chain(chain));
            let got: Vec<String> = list(&data, &me).expect("列得出").into_iter().map(|t| t.hash).collect();
            assert_eq!(got, vec![want.clone()], "链 {chain} 只见本网的那一条");
        }
        let st = eravon_vault::store::open_write(&data, "eravon-test", now).expect("可写");
        assert_eq!(Kind::of("gas"), None);
        let gas = format!("0x{}", "d4".repeat(32));
        let mut f = vec![("hash".to_string(), gas.clone()), ("kind".to_string(), "gas".to_string()), (CHAIN_ID.to_string(), "1".to_string())];
        f.sort();
        st.put(&me, K::TxReceipt, &record_id(&gas), &f, &[], now).expect("落档");
        assert!(list(&data, &me).expect("列得出").iter().any(|t| t.hash == gas && t.kind == "gas"));
        let old = format!("0x{}", "c3".repeat(32));
        row(&st, None, &old);
        drop(st);
        let e = list(&data, &me).unwrap_err();
        assert!(e.token == "E_P7_RECORD" && e.detail.contains(&record_id(&old)), "{e:?}");
    }

    #[test]
    fn fee_total_counts_only_settled_rows_of_that_order() {
        let t = |order: &str, fee: &str| Tx {
            id: String::new(),
            hash: String::new(),
            from: String::new(),
            to: String::new(),
            kind: "pay".into(),
            order: order.into(),
            token: String::new(),
            amount: String::new(),
            block: String::new(),
            status: "1".into(),
            gas_used: String::new(),
            gas_price: String::new(),
            fee: fee.into(),
            at: 0,
        };
        let v = vec![t("0xaa", "100"), t("0xaa", "23"), t("0xaa", ""), t("0xbb", "7")];
        assert_eq!(fee_total(&v, "0xAA"), Some(("123".into(), 2)));
        assert_eq!(fee_total(&v, "0xcc"), None);
        assert_eq!(fee_of(21_000, &U256::from_u64(2)), "42000");
    }
}
