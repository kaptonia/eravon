mod common;

use common::{Home, PIN, Ran};
use eravon_galeed as k1;
use k1::Value;

const NOW: &str = "1758412800";

fn obj(members: &[(&str, Value)]) -> Value {
    let mut m: Vec<(String, Value)> = members.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect();
    m.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Value::Obj(m)
}

fn text(s: &str) -> Value {
    Value::Str(s.to_string())
}

fn block(h: &Home, seller: &str) -> String {
    let ladder = vec![("驳回".to_string(), 0), ("全额退款".to_string(), 10_000)];
    let judges = vec![eravon_goods::resolve::JudgeEl {
        judge: "0x00000000000000000000000000000000000000b1".into(),
        amanah_hash: format!("0x{}", "11".repeat(32)),
        quote: "0x0102".into(),
        source: None,
    }];
    let g = eravon_goods::terms::generate(&eravon_goods::terms::Parts {
        name: "海报",
        description: "一张 A2 海报",
        delivery_note: "PNG 一份",
        goods_private: false,
        delivery_private: false,
        seller_key: seller,
        seller_payout: seller,
        seller_fund: seller,
        token: "0x00000000000000000000000000000000000000a4",
        amount: "1000",
        t1_secs: 86_400,
        t2_secs: 3_600,
        open_window: 7_200,
        buyer_may_open: false,
        ladder: &ladder,
        judges: &judges,
        guilds: &[],
        nonce: 1,
        expiry: 2_000_000_000,
        series: None,
    });
    let doc = String::from_utf8(g.bytes).expect("utf-8");
    let p = h.root.join("block.json");
    std::fs::write(&p, doc.lines().nth(1).expect("the block line")).expect("a block file");
    p.display().to_string()
}

fn archived(h: &Home) -> (String, String) {
    let terms = h.root.join("terms.md").display().to_string();
    let listing = h.root.join("listing.json").display().to_string();
    let b = block(h, &h.address);
    let t = h.run(&["seller-terms-build", "--data", h.data_s(), "--now", NOW, "--writer", "t", "--passcode", PIN, "--address", &h.address, "--block", &b, "--out", &terms], |_| {});
    assert_eq!(t.code, 0, "the terms are built: {}", t.text);
    let l = h.run(&["seller-product-link", "--data", h.data_s(), "--now", NOW, "--writer", "t", "--passcode", PIN, "--address", &h.address, "--terms", &terms, "--judge-sig", "", "--out", &listing], |_| {});
    assert_eq!(l.code, 0, "the listing is built: {}", l.text);
    (listing, terms)
}

fn receipt(h: &Home, listing: &str) -> String {
    let l = std::fs::read(listing).expect("the listing");
    let read = eravon_link::listing::listing(&l).expect("the listing reads");
    let buyer = "0x00000000000000000000000000000000000000c1";
    let r = k1::canon::canonical(&obj(&[
        ("v", text("eravon.receipt/1")),
        ("termsHash", text(&read.terms_hash)),
        ("candidate", Value::Int(0)),
        ("buyerKey", text(buyer)),
        ("buyerPayout", text(buyer)),
        ("openBy", Value::Int(1_758_412_800 + 3_600)),
        ("prev", text(&format!("0x{}", "0".repeat(40)))),
        ("sig", text("")),
    ]))
    .expect("canonical receipt");
    let p = h.root.join("receipt.json");
    std::fs::write(&p, r).expect("a receipt file");
    p.display().to_string()
}

fn paste(h: &Home, receipt: &str, id: &str, files: &[(&str, &str)]) -> Ran {
    let mut a = vec!["seller-receipt-paste", "--data", h.data_s(), "--now", NOW, "--writer", "t", "--passcode", PIN, "--address", &h.address, "--receipt", receipt, "--id", id];
    for (flag, path) in files {
        a.push(flag);
        a.push(path);
    }
    h.run(&a, |_| {})
}

fn face(r: &Ran) -> Vec<(String, String)> {
    ["budget", "buyerKey", "caseId", "instance", "verdict"].iter().map(|k| (k.to_string(), r.get(k))).chain(["candidate", "openBy"].iter().map(|k| (k.to_string(), r.out.member(k).map(|v| format!("{v:?}")).unwrap_or_default()))).collect()
}

#[test]
fn a_receipt_pasted_without_the_files_takes_them_from_the_archive() {
    let h = Home::new("paste-archive");
    let (listing, terms) = archived(&h);
    let r = receipt(&h, &listing);
    let with = paste(&h, &r, "with", &[("--listing", &listing), ("--terms", &terms)]);
    assert_eq!(with.code, 0, "with both files: {}", with.text);
    let without = paste(&h, &r, "without", &[]);
    assert_eq!(without.code, 0, "with neither file: {}", without.text);
    assert_eq!(face(&without), face(&with), "the same order either way");
    assert_eq!(without.get("id"), "without", "recorded under its own id");
}

#[test]
fn a_receipt_whose_product_this_machine_never_saved_is_refused_naming_the_verbs_flags() {
    let maker = Home::new("paste-maker");
    let (listing, _) = archived(&maker);
    let h = Home::new("paste-elsewhere");
    let r = receipt(&maker, &listing);
    let out = paste(&h, &r, "x", &[]);
    assert_ne!(out.code, 0, "{}", out.text);
    assert_eq!((out.get("reason"), out.get("downToken")), ("E_V_DOWN".to_string(), "E_WATCH_ARCHIVE".to_string()), "{}", out.text);
    let said = out.get("detail");
    assert!(said.contains("seller-receipt-paste") && said.contains("--listing") && said.contains("--terms"), "the verb and flags as they are: {said}");
    assert!(!said.contains("--payload") && !said.contains(" receipt-paste"), "no former names: {said}");
}

#[test]
fn one_file_alone_is_refused_naming_the_other() {
    let h = Home::new("paste-one");
    let (listing, terms) = archived(&h);
    let r = receipt(&h, &listing);
    for (given, path, missing) in [("--listing", &listing, "--terms"), ("--terms", &terms, "--listing")] {
        let out = paste(&h, &r, "one", &[(given, path)]);
        assert_eq!((out.code, out.get("reason")), (1, "E_V_ARG".to_string()), "{given} alone: {}", out.text);
        assert!(out.get("detail").contains(missing), "{given} alone names {missing}: {}", out.text);
    }
    let shown = h.run(&["seller-order-list", "--data", h.data_s(), "--now", NOW, "--writer", "t", "--passcode", PIN, "--address", &h.address], |_| {});
    assert!(!shown.text.contains("\"one\""), "nothing recorded: {}", shown.text);
}
