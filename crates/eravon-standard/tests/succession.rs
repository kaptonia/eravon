use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_standard::{chain, fault::Fault, order};

#[derive(Clone)]
struct Nodes {
    call: fn(u16) -> Option<String>,
    seen: std::sync::Arc<Mutex<Vec<String>>>,
}

impl Nodes {
    fn answer_one(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        self.seen.lock().expect("锁").push(b.clone());
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let method = b.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("");
        let result = match method {
            "eth_chainId" => format!("\"0x{:x}\"", eravon_literals::constants::active_standard().chain_id),
            "eth_blockNumber" => "\"0x40\"".to_string(),
            "eth_getBlockByNumber" => "{\"number\":\"0x40\",\"timestamp\":\"0x64\",\"baseFeePerGas\":\"0x1\"}".to_string(),
            "eth_call" => format!("\"{}\"", (self.call)(url.port).unwrap_or_else(|| "0x".into())),
            _ => "null".to_string(),
        };
        Ok(Answer { status: 200, location: None, body: format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{result}}}").into_bytes() })
    }
}

impl Transport for Nodes {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn word_of(addr_byte: u8) -> String {
    format!("0x{}{}", "00".repeat(12), format!("{addr_byte:02x}").repeat(20))
}

fn table() -> Table {
    Table::new(vec!["http://127.0.0.1:8001".into(), "http://127.0.0.1:8002".into()]).expect("两个端点")
}

const KEY: &str = "0x1111111111111111111111111111111111111111";

#[test]
fn current_is_the_books_answer_where_the_endpoints_agree() {
    let n = Nodes { call: |_| Some(word_of(0x22)), seen: std::sync::Arc::new(Mutex::new(Vec::new())) };
    let got = chain::current(&n, &table(), KEY).expect("两端一致");
    assert_eq!(got.value, format!("0x{}", "22".repeat(20)));
    let r = chain::root(&n, &table(), KEY).expect("两端一致");
    assert_eq!(r.value, format!("0x{}", "22".repeat(20)));
    let book = eravon_literals::constants::SUCCESSION_BOOK.to_ascii_lowercase();
    assert!(n.seen.lock().expect("锁").iter().any(|b| b.to_ascii_lowercase().contains(&book)), "问的是簿的地址");
}

#[test]
fn disagreeing_endpoints_and_an_absent_book_are_refused_by_name() {
    let split = Nodes { call: |port| Some(word_of(if port == 8001 { 0x22 } else { 0x33 })), seen: std::sync::Arc::new(Mutex::new(Vec::new())) };
    assert!(matches!(chain::current(&split, &table(), KEY), Err(Fault::Chain { .. })), "两端不一致即拒");
    let empty = Nodes { call: |_| None, seen: std::sync::Arc::new(Mutex::new(Vec::new())) };
    match chain::current(&empty, &table(), KEY) {
        Err(Fault::Precheck { gate, why }) => assert!(gate == eravon_standard::precheck::GATE_CURRENT && why.contains(eravon_literals::constants::BOOK_CODE_HASH), "{why}"),
        Err(Fault::Chain { .. }) => {}
        other => panic!("簿上无码即具名拒,不读成零地址:{other:?}"),
    }
}

#[test]
fn the_verifier_check_is_asked_as_from_the_logic() {
    let n = Nodes { call: |_| None, seen: std::sync::Arc::new(Mutex::new(Vec::new())) };
    let a = |c: char| format!("0x{}", c.to_string().repeat(40));
    let o = order::Order {
        offer: eravon_terms::derive::Offer {
            seller_key: a('1'),
            judge: a('2'),
            guild: a('0'),
            seller_payout: a('3'),
            seller_fund: a('4'),
            token: a('5'),
            amount: eravon_terms::u256::U256::from_dec("1").expect("十进制"),
            delivery_window: 0,
            confirm_window: 0,
            open_window: 0,
            buyer_may_open: false,
            ladder: vec![eravon_terms::doc::Rung { buyer_bps: 0, seller_bps: 10_000, fee_bearer: 0 }],
            terms_hash: [0; 32],
            amanah_hash: [0; 32],
            quote_hash: [0; 32],
            nonce: 0,
            expiry: 0,
        },
        buyer_key: a('6'),
        buyer_payout: a('7'),
        open_by: 0,
        prev: order::NO_PREV.to_string(),
    };
    let _ = chain::verifier_check(&n, &table(), &o, &[0u8; 480]);
    let logic = eravon_literals::constants::active_standard().logic.to_ascii_lowercase();
    let seen = n.seen.lock().expect("锁");
    let call = seen.iter().find(|b| b.contains("eth_call")).expect("问了一次 eth_call");
    assert!(call.to_ascii_lowercase().contains(&format!("\"from\":\"{logic}\"")), "{call}");
}

#[test]
fn a_stage_fault_is_refused_by_the_precheck_by_name() {
    use eravon_standard::abi::{self, Val};
    let token = format!("0x{}01", "00".repeat(19));
    let mut quote = vec![0u8; eravon_terms::quote::LEN];
    quote[31] = 1;
    let quote_hash = eravon_terms::derive::keccak(&quote);
    let a = |c: char| format!("0x{}", c.to_string().repeat(40));
    let o = order::Order {
        offer: eravon_terms::derive::Offer {
            seller_key: a('1'),
            judge: a('2'),
            guild: a('0'),
            seller_payout: a('3'),
            seller_fund: a('4'),
            token: token.clone(),
            amount: eravon_terms::u256::U256::from_dec("1").expect("十进制"),
            delivery_window: 0,
            confirm_window: 0,
            open_window: 0,
            buyer_may_open: false,
            ladder: vec![eravon_terms::doc::Rung { buyer_bps: 0, seller_bps: 10_000, fee_bearer: 0 }],
            terms_hash: [0; 32],
            amanah_hash: [0; 32],
            quote_hash,
            nonce: 0,
            expiry: 0,
        },
        buyer_key: a('6'),
        buyer_payout: a('7'),
        open_by: 0,
        prev: a('8'),
    };
    for name in ["BadPrev", "StageOpen"] {
        let mut stage = [0u8; 32];
        stage[..4].copy_from_slice(&eravon_terms::derive::keccak(format!("{name}()").as_bytes())[..4]);
        let zero = || Val::U256([0; 32]);
        let mut members = vec![zero(), Val::Bool(false), Val::Tuple((0..15).map(|_| zero()).collect())];
        members.extend((0..5).map(|_| zero()));
        members.extend([Val::Bool(false), Val::Bool(false), Val::Bool(false), Val::B32(stage), Val::Addr([0; 20])]);
        let body = order::hex_of(&abi::encode(&[Val::Tuple(members)]));
        static BODY: std::sync::OnceLock<Mutex<String>> = std::sync::OnceLock::new();
        *BODY.get_or_init(|| Mutex::new(String::new())).lock().expect("锁") = body;
        let n = Nodes { call: |_| Some(BODY.get().expect("已置").lock().expect("锁").clone()), seen: std::sync::Arc::new(Mutex::new(Vec::new())) };
        let e = eravon_standard::precheck::priced(&n, &table(), &o, &quote).expect_err("前段不让开案");
        assert_eq!(e.precheck_gate(), Some(eravon_standard::precheck::GATE_STAGE), "{name}");
        assert!(format!("{e:?}").contains(name), "{name}: {e:?}");
    }
}

#[test]
fn a_signature_by_the_frozen_key_after_it_moved_is_refused_by_name() {
    use eravon_standard::consent::{self, Form, Party, RegRead, Sender};
    let digest = [9u8; 32];
    let (old_sig, frozen) = eravon_terms::sig::sign_secp256k1(&[0x11; 32], &digest).expect("一把钥");
    let (new_sig, current) = eravon_terms::sig::sign_secp256k1(&[0x22; 32], &digest).expect("另一把钥");
    let raw = |s: &str| order::bytes_of("签名", s).expect("十六进制");
    let sender = Sender { from: None, via_multicall: false };
    let mut registry = |_: &str, _: &[u8; 32]| -> RegRead { panic!("签名形不问登记") };
    let e = consent::slot(Party::Buyer, &frozen, &current, &digest, &raw(&old_sig), sender, &mut registry).expect_err("冻结钥已换");
    assert_eq!(e.precheck_gate(), Some(eravon_standard::precheck::GATE_CURRENT));
    let ok = consent::slot(Party::Buyer, &frozen, &current, &digest, &raw(&new_sig), sender, &mut registry).expect("现行钥签");
    assert_eq!(ok, Form::Signature);
    let same = consent::slot(Party::Buyer, &frozen, &frozen, &digest, &raw(&old_sig), sender, &mut registry).expect("钥未换");
    assert_eq!(same, Form::Signature);
}
