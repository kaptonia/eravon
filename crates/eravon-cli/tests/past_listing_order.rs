mod common;

use common::{Home, Node, Ran, node, tree};
use eravon_galeed as k1;
use eravon_literals::constants as k7;
use eravon_terms as k2;
use k1::Value;

const NOW: &str = "1758412800";
const NOW_INT: u64 = 1_758_412_800;
const SEPOLIA: &str = "11155111";

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
        ("buyerMayOpen", Value::Bool(true)),
        ("ladder", ladder),
        ("nonce", Value::Int(1)),
        ("expiry", Value::Int(NOW_INT + 600)),
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

fn pack(logic: &str, buyer: &str) -> Vec<u8> {
    let l = listing(logic);
    let read = eravon_link::listing::listing(&l).expect("the listing reads");
    let receipt = k1::canon::canonical(&obj(&[
        ("v", text("eravon.receipt/1")),
        ("termsHash", text(&read.terms_hash)),
        ("candidate", Value::Int(0)),
        ("buyerKey", text(buyer)),
        ("buyerPayout", text(buyer)),
        ("openBy", Value::Int(NOW_INT + 60)),
        ("prev", addr("0")),
        ("sig", text("")),
    ]))
    .expect("canonical receipt");
    let (_, _, bound) = eravon_link::receipt::bind_receipt_bytes(&l, &receipt).expect("the receipt binds");
    let chain = Value::Int(k7::TRIAL_SEPOLIA.chain_id);
    let value = |b: &[u8]| k1::canon::accept(b).expect("canonical");
    k1::canon::canonical(&obj(&[
        ("v", text("eravon.pack/1")),
        ("chainId", chain.clone()),
        ("instance", text(&bound.instance)),
        ("caseId", text(&k1::hash::hex_encode(&bound.salt))),
        ("link", obj(&[("v", text("eravon.link/1")), ("chainId", chain), ("tx", word("7")), ("hash", text(&k1::hash::hex_encode(&k1::hash::sha256(&l))))])),
        ("listing", value(&l)),
        ("receipt", value(&receipt)),
    ]))
    .expect("canonical pack")
}

fn import(h: &Home, n: &Node, id: &str, logic: &str) {
    let path = h.root.join(format!("{id}.json"));
    std::fs::write(&path, pack(logic, &h.address)).expect("the pack file");
    let r = h.run(&["buyer-import", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", common::PIN, "--endpoint", &n.url, "--chain", SEPOLIA, "--id", id, "--pack", path.to_str().expect("utf-8")], |_| {});
    assert_eq!(r.code, 0, "imported: {} {}", r.text, r.err);
}

fn pay(h: &Home, n: &Node, id: &str) -> Ran {
    h.run(&["buyer-pay", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", common::PIN, "--endpoint", &n.url, "--chain", SEPOLIA, "--id", id, "--to", common::TO, "--token", common::TOKEN, "--amount", common::AMOUNT, "--send", "true"], |_| {})
}

fn deploy(h: &Home, n: &Node, id: &str) -> Ran {
    h.run(&["buyer-deploy", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", common::PIN, "--endpoint", &n.url, "--chain", SEPOLIA, "--id", id, "--send", "true"], |_| {})
}

#[test]
fn a_past_listings_order_is_neither_paid_nor_opened() {
    let h = Home::new("past-listing");
    let n = node();
    assert!(k7::select_chain(k7::TRIAL_SEPOLIA.chain_id), "Sepolia carries the trial row");
    import(&h, &n, "past", k7::TRIAL_SEPOLIA.logic);
    import(&h, &n, "now", k7::active_standard().logic);

    let refused = || {
        let before = tree(&h.data);
        for (verb, r) in [("buyer-pay", pay(&h, &n, "past")), ("buyer-deploy", deploy(&h, &n, "past"))] {
            assert_eq!(r.code, 1, "{verb}: {} {}", r.text, r.err);
            assert_eq!((r.get("reason"), r.get("downToken")), ("E_V_DOWN".to_string(), "E_A1_PAST_LISTING".to_string()), "{verb}: {}", r.text);
            assert!(r.get("detail").ends_with(eravon_standard::fault::PAST_LISTING_SAYS), "{verb}: {}", r.text);
        }
        assert_eq!(n.broadcasts(), 0, "nothing sent");
        assert_eq!(n.asked("eth_call"), 0, "refused before the chain is read");
        assert_eq!(tree(&h.data), before, "nothing kept");
    };
    refused();

    let ers = std::path::Path::new(common::BIN).with_file_name("ers");
    if ers.exists() {
        let set = std::process::Command::new(&ers)
            .args(["settings-put", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", common::PIN, "--past-cases", "true"])
            .env(eravon_os::HOME_VAR, &h.home)
            .output()
            .expect("ers runs");
        assert!(set.status.success(), "the switch written: {}", String::from_utf8_lossy(&set.stdout));
        refused();
    }

    for (verb, r) in [("buyer-pay", pay(&h, &n, "now")), ("buyer-deploy", deploy(&h, &n, "now"))] {
        assert_ne!(r.get("downToken"), "E_A1_PAST_LISTING", "{verb} on a current order: {}", r.text);
    }
}
