use eravon_galeed as k1;
use eravon_link as k3;
use k3::token::Token;

const TERMS: &str = "0xf44db2165e72662499b472548965aac9cb5a7d940a0a677f7830a278018f8615";

#[test]
fn a_receipt_is_read_in_its_listings_generation() {
    use eravon_literals::constants as k7;
    assert!(k7::select_chain(k7::TRIAL_SEPOLIA.chain_id), "Sepolia carries the trial row");
    let zero = "0x0000000000000000000000000000000000000000";
    let doc = |prev: Option<&str>| {
        let mut m = vec![
            ("buyerKey".to_string(), k1::Value::Str(format!("0x{}", "1".repeat(40)))),
            ("buyerPayout".to_string(), k1::Value::Str(format!("0x{}", "2".repeat(40)))),
            ("candidate".to_string(), k1::Value::Int(0)),
            ("openBy".to_string(), k1::Value::Int(9)),
            ("sig".to_string(), k1::Value::Str(String::new())),
            ("termsHash".to_string(), k1::Value::Str(TERMS.to_string())),
            ("v".to_string(), k1::Value::Str("eravon.receipt/1".into())),
        ];
        if let Some(p) = prev {
            m.push(("prev".to_string(), k1::Value::Str(p.to_string())));
        }
        m.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        k1::canon::canonical(&k1::Value::Obj(m)).expect("canonical")
    };
    let trial = k7::TRIAL_SEPOLIA.logic;
    let current = k7::active_standard().logic;
    let r = k3::receipt::receipt_under(trial, &doc(None)).expect("the trial form");
    assert_eq!(r.prev, zero);
    assert_eq!(r.current_form(), doc(Some(zero)), "kept with the zero prev");
    let kept = k3::receipt::receipt_under(trial, &r.current_form()).expect("the kept form");
    assert_eq!(kept.current_form(), r.current_form(), "byte for byte");
    assert!(k3::receipt::receipt_under(trial, &doc(Some(&format!("0x{}", "3".repeat(40))))).is_err(), "a trial order knows no other prev");
    assert_eq!(k3::receipt::receipt_under(current, &doc(None)).expect_err("current form").token, Token::ReceiptMissing);
    assert!(k3::receipt::receipt_under(current, &doc(Some(zero))).is_ok());
}
