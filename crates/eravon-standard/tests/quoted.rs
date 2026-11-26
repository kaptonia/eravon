use std::sync::atomic::{AtomicUsize, Ordering};

use eravon_chain::endpoints::Table;
use eravon_chain::fault::{Fault, R};
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_standard::precheck;
use eravon_terms as k2;

#[derive(Clone)]
struct Down(std::sync::Arc<AtomicUsize>);

impl Down {
    fn answer_one(&self, _url: &Url, _body: &[u8]) -> R<Answer> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(Fault::Transport("connect refused".into()))
    }
}

impl Transport for Down {
    fn post(&self, url: &Url, body: &[u8]) -> R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn a(c: char) -> String {
    format!("0x{}", c.to_string().repeat(40))
}

fn quote(unit: &str) -> Vec<u8> {
    let addr = |x: &str| format!("{:0>64}", x.trim_start_matches("0x"));
    let num = |n: u64| format!("{n:064x}");
    let mut q = addr(&a('5')) + &num(100) + &addr(unit);
    let feed = if unit.trim_start_matches("0x").chars().all(|c| c == '0') { a('0') } else { a('8') };
    q += &(addr(&feed) + &addr(&feed));
    for n in [3600, 0, 500, 0, 0, 0, 14 * 86_400, 3 * 86_400, 0, 0] {
        q += &num(n);
    }
    eravon_standard::order::bytes_of("报价", &format!("0x{q}")).expect("十六进制")
}

fn offer(amount: &str) -> k2::derive::Offer {
    k2::derive::Offer {
        seller_key: a('1'),
        judge: a('2'),
        guild: a('0'),
        seller_payout: a('3'),
        seller_fund: a('4'),
        token: a('5'),
        amount: k2::u256::U256::from_dec(amount).expect("十进制"),
        delivery_window: 7 * 86_400,
        confirm_window: 3 * 86_400,
        open_window: 86_400,
        buyer_may_open: false,
        ladder: vec![k2::doc::Rung { buyer_bps: 0, seller_bps: 10_000, fee_bearer: 0 }],
        terms_hash: [0; 32],
        amanah_hash: [0; 32],
        quote_hash: [0; 32],
        nonce: 1,
        expiry: 0,
    }
}

#[test]
fn a_settlement_token_quote_is_priced_on_the_order_amount_offline() {
    let t = Down(std::sync::Arc::new(AtomicUsize::new(0)));
    let table = Table::new(vec!["http://127.0.0.1:1".into()]).expect("端点表");
    let q = precheck::quoted(&t, &table, &offer("12500000000000000000"), &quote(&a('0'))).expect("离线即得");
    assert_eq!(q.premium.to_dec(), "125000000000000000");
    assert_eq!(q.lf.to_dec(), "625000000000000000");
    assert_eq!(q.deposit.to_dec(), "750000000000000000");
    assert_eq!(q.rate, None);
    assert_eq!(t.0.load(Ordering::SeqCst), 0, "结算币报价不读链");
}

#[test]
fn a_reference_unit_quote_is_priced_only_by_the_verifier() {
    let t = Down(std::sync::Arc::new(AtomicUsize::new(0)));
    let table = Table::new(vec!["http://127.0.0.1:1".into()]).expect("端点表");
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let got = precheck::quoted(&t, &table, &offer("12500000000000000000"), &quote(&a('9')));
    assert!(got.is_err(), "{got:?}");
    assert!(t.0.load(Ordering::SeqCst) > 0, "参考币报价问验证器");
}
