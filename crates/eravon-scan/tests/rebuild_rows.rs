use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::constants::{current_network, pretend_chain, StandardTable, NETWORKS};
use eravon_scan::scan::{rebuild, rebuild_among, Rebuild};
use eravon_standard::{call, order};
use eravon_terms as k2;

fn past() -> StandardTable {
    StandardTable { logic: "0x00000000000000000000000000000000000f1a57", ..current_network().standard }
}

fn each_row(t: impl Fn()) {
    for n in NETWORKS {
        pretend_chain(Some(n.standard.chain_id));
        t();
    }
    pretend_chain(None);
}

const TX: &str = "0x2222222222222222222222222222222222222222222222222222222222222222";

#[derive(Clone)]
struct Chain(String, String);

impl Chain {
    fn answer_one(&self, _url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let result = if b.contains("eth_chainId") {
            format!("\"0x{:x}\"", current_network().standard.chain_id)
        } else if b.contains("eth_blockNumber") {
            "\"0x10\"".to_string()
        } else if b.contains("eth_getTransactionByHash") {
            format!(r#"{{"blockNumber":"0x5","from":"0x{}","hash":"{TX}","input":"{}","to":"{}","value":"0x0"}}"#, "6".repeat(40), self.0, self.1)
        } else {
            "null".to_string()
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }
}

impl Transport for Chain {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn table() -> Table {
    Table::new(vec!["http://127.0.0.1:1".to_string()]).expect("端点表")
}

fn the_order() -> order::Order {
    let a = |c: char| format!("0x{}", c.to_string().repeat(40));
    order::Order {
        offer: k2::derive::Offer {
            seller_key: a('1'),
            judge: a('2'),
            guild: a('0'),
            seller_payout: a('3'),
            seller_fund: a('4'),
            token: a('5'),
            amount: k2::u256::U256::from_dec("1000000000").expect("十进制"),
            delivery_window: 604_800,
            confirm_window: 259_200,
            open_window: 86_400,
            buyer_may_open: false,
            ladder: vec![k2::doc::Rung { buyer_bps: 0, seller_bps: 10_000, fee_bearer: 0 }, k2::doc::Rung { buyer_bps: 10_000, seller_bps: 0, fee_bearer: 1 }],
            terms_hash: [0x11; 32],
            amanah_hash: [0x22; 32],
            quote_hash: [0x33; 32],
            nonce: 7,
            expiry: 1_800_000_000,
        },
        buyer_key: a('6'),
        buyer_payout: a('7'),
        open_by: 1_700_086_400,
        prev: order::NO_PREV.to_string(),
    }
}

fn case_under(logic: &str) -> (String, Chain) {
    let o = the_order();
    let chain = current_network().standard.chain_id;
    let d = o.derive(chain, logic).expect("派生");
    let case_id = order::hex_of(&d.salt);
    let c = call::init(&o, &d.instance, &[0xab; 96], &[0xcd; 65], &[], &[], &[]).expect("写得出");
    (case_id, Chain(order::hex_of(&c.data), d.instance))
}

#[test]
fn a_case_made_under_a_past_row_is_rebuilt_under_that_row() {
    each_row(|| {
        let chain = current_network().standard.chain_id;
        let current = current_network().standard.logic;
        let (case_id, t) = case_under(past().logic);
        let Rebuild::Done(r) = rebuild_among(&t, &table(), TX, chain, current, &[past()], &case_id).expect("往代行重建得出") else {
            panic!("顶层 init,不该要追踪");
        };
        assert!(r.past && r.logic.eq_ignore_ascii_case(past().logic), "{} {}", r.logic, r.past);
        assert!(r.case_id.eq_ignore_ascii_case(&case_id));
        let o = the_order();
        assert_eq!(r.init.order.buyer_payout, o.buyer_payout);
        assert_eq!(r.token, o.offer.token);
        assert_eq!(r.init.order.open_by, o.open_by);
        assert_eq!(r.init.quote, vec![0xab; 96]);
        assert_eq!((r.amount.as_str(), r.buyer.as_str(), r.seller.as_str(), r.members, r.path), ("1000000000", o.buyer_key.as_str(), o.offer.seller_key.as_str(), 0, "top"));
        assert_eq!(r.terms_hash, order::hex_of(&o.offer.terms_hash));
    });
}

#[test]
fn a_case_made_under_the_current_row_is_rebuilt_as_before() {
    each_row(|| {
        let chain = current_network().standard.chain_id;
        let current = current_network().standard.logic;
        let (case_id, t) = case_under(current);
        let Rebuild::Done(r) = rebuild_among(&t, &table(), TX, chain, current, &[past()], &case_id).expect("现行行重建得出") else {
            panic!("顶层 init");
        };
        assert!(!r.past && r.logic.eq_ignore_ascii_case(current));
        let Rebuild::Done(same) = rebuild(&t, &table(), TX, chain, current, &case_id).expect("本构建的表") else { panic!() };
        assert!(!same.past && same.case_id.eq_ignore_ascii_case(&r.case_id));
    });
}

#[test]
fn the_row_is_told_by_where_init_went_and_a_case_id_mismatch_is_refused() {
    each_row(|| {
        let chain = current_network().standard.chain_id;
        let current = current_network().standard.logic;
        let (case_id, t) = case_under(past().logic);
        let Rebuild::Done(r) = rebuild(&t, &table(), TX, chain, current, &case_id).expect("本构建的表") else { panic!() };
        assert!(!r.past && r.logic.eq_ignore_ascii_case(current), "{} {}", r.logic, r.past);
        let (stranger, t2) = case_under("0x00000000000000000000000000000000000beef1");
        let Rebuild::Done(r) = rebuild_among(&t2, &table(), TX, chain, current, &[past()], &stranger).expect("照现行行读") else { panic!() };
        assert!(!r.past);
        let e = rebuild_among(&t, &table(), TX, chain, current, &[past()], &format!("0x{}", "9".repeat(64))).expect_err("案号对不上");
        assert_eq!(e.name(), "E_A3_REBUILD");
    });
}
