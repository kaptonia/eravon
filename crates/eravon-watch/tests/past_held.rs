use eravon_galeed as k1;
use eravon_literals::constants as k7;
use eravon_store::vault::Seat;
use eravon_terms as k2;
use eravon_watch::order::{self, Order, State};
use eravon_watch::poll::past_held;
use k1::Value;

const NOW: u64 = 1_790_000_000;
const PIN: &str = "48261537";
const PHRASE: &str = "test test test test test test test test test test test junk";
const SELLER: &str = "0x98e503f35d0a019cb0a251ad243a4ccfcf371f46";

fn obj(members: &[(&str, Value)]) -> Value {
    let mut m: Vec<(String, Value)> = members.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect();
    m.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Value::Obj(m)
}

fn text(s: &str) -> Value {
    Value::Str(s.to_string())
}

fn addr(c: &str) -> Value {
    Value::Str(format!("0x{}", c.repeat(40)))
}

fn word(c: &str) -> Value {
    Value::Str(format!("0x{}", c.repeat(64)))
}

fn listing(logic: &str) -> Vec<u8> {
    let logic = logic.to_ascii_lowercase();
    let rung = |buyer: u64, seller: u64| obj(&[("buyerBps", Value::Int(buyer)), ("sellerBps", Value::Int(seller)), ("feeBearer", Value::Int(0))]);
    let ladder = Value::Arr(vec![rung(0, 10000), rung(10000, 0)]);
    let judge = [("judge", addr("4")), ("amanahHash", word("5")), ("quoteHash", word("6"))];
    let numbers = [
        ("sellerKey", addr("1")),
        ("sellerPayout", addr("2")),
        ("sellerFund", addr("1")),
        ("amount", text("1000")),
        ("deliveryWindow", Value::Int(60)),
        ("confirmWindow", Value::Int(60)),
        ("openWindow", Value::Int(60)),
        ("buyerMayOpen", Value::Bool(false)),
        ("ladder", ladder),
        ("nonce", Value::Int(1)),
        ("expiry", Value::Int(NOW + 600)),
    ];
    let chain = Value::Int(k7::TRIAL_SEPOLIA.chain_id);
    let mut block = vec![
        ("v", text("eravon.terms/1")),
        ("chainId", chain.clone()),
        ("logic", text(&logic)),
        ("token", addr("3")),
        ("goods_md", text("goods")),
        ("delivery_md", text("delivery")),
        ("judges", Value::Arr(vec![obj(&judge)])),
        ("guilds", Value::Arr(Vec::new())),
    ];
    block.extend(numbers.iter().cloned());
    let block = String::from_utf8(k1::canon::canonical(&obj(&block)).expect("canonical block")).expect("utf-8");
    let terms = format!("```eravon.terms/1\n{block}\n```");
    let mut signed = judge.to_vec();
    signed.push(("sig", text("")));
    let hash = k1::hash::hex_encode(&k1::hash::sha256(terms.as_bytes()));
    k1::canon::canonical(&obj(&[
        ("v", text("eravon.listing/1")),
        ("chainId", chain),
        ("token", addr("3")),
        ("logic", text(&logic)),
        ("proxyCodeHash", text(&k1::hash::hex_encode(&k2::derive::proxy_code_hash(&logic)))),
        ("terms", obj(&[("hash", text(&hash)), ("inline_md", text(&terms))])),
        ("order", obj(&numbers)),
        ("judges", Value::Arr(vec![obj(&signed)])),
        ("guilds", Value::Arr(Vec::new())),
    ]))
    .expect("canonical listing")
}

fn order_named(id: &str) -> Order {
    Order {
        id: id.into(),
        case_id: format!("0x{}", "11".repeat(32)),
        instance: format!("0x{}", "22".repeat(20)),
        buyer_key: format!("0x{}", "33".repeat(20)),
        buyer_payout: String::new(),
        verdict: "RECEIPT_BOUND".into(),
        token: String::new(),
        need: "1000".into(),
        open_by: NOW + 60,
        candidate: 0,
        expiry: NOW + 600,
        state: State::Deployed,
        balance: String::new(),
        gap: String::new(),
        note: String::new(),
        notice: String::new(),
        notice_at: 0,
        deploy_tx: String::new(),
        expire_tx: String::new(),
        source: "paste".into(),
        opened: 0,
        extra: Vec::new(),
    }
}

#[test]
fn the_round_holds_a_past_order_by_the_one_gate() {
    let dir = std::env::temp_dir().join(format!("watch-past-held-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the machine directory");
    let data = dir.display().to_string();
    eravon_vault::kdf::set_light(None);
    eravon_vault::records::bind(&data).expect("the run binds its machine directory");
    eravon_vault::onboard::land(&data, eravon_vault::onboard::Mode::First, Seat::Seller, "主钥", false, eravon_vault::onboard::Source::Words(PHRASE), PIN, None, None, NOW).expect("a first run");
    let mut session = eravon_vault::session::Session::locked(&data);
    session.unlock(PIN).expect("unlocks");
    let me = SELLER.to_string();

    assert!(k7::select_chain(k7::TRIAL_SEPOLIA.chain_id), "Sepolia carries the trial row");
    let keep = |id: &str, parts: &[(String, Vec<u8>)]| {
        let o = order_named(id);
        order::save(&data, "eravon-test", NOW, &me, &o, parts).expect("kept");
        o
    };
    let past = keep("past", &[(order::part_listing(), listing(k7::TRIAL_SEPOLIA.logic))]);
    let current = keep("current", &[(order::part_listing(), listing(k7::active_standard().logic))]);
    let unread = keep("unread", &[(order::part_listing(), b"not a listing".to_vec())]);

    k7::set_past_switch(false);
    assert!(past_held(&data, &me, &past), "off: a past order is held");
    assert!(!past_held(&data, &me, &current), "off: a current order is not");
    assert!(!past_held(&data, &me, &unread), "a listing that does not read is not judged here");
    k7::set_past_switch(true);
    assert!(!past_held(&data, &me, &past), "on: the round writes a past order in its generation's form");
    assert!(!past_held(&data, &me, &current), "on: a current order is not held");
    k7::set_past_switch(false);

    assert!(k7::select_chain(1), "mainnet");
    assert!(k7::past_rows().is_empty(), "mainnet carries no past row");
    assert!(!past_held(&data, &me, &past), "no past row: nothing is a past order");
    std::fs::remove_dir_all(&dir).ok();
}
