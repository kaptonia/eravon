use eravon_galeed as k1;
use eravon_terms as k2;
use k2::token::Token;

fn corpus(name: &str) -> Vec<u8> {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base/eravon-conformance/terms-corpus").join(name);
    std::fs::read(&at).unwrap_or_else(|e| panic!("{}: {e}", at.display()))
}

fn offer() -> (k2::doc::Terms, k2::derive::Offer) {
    let t = k2::doc::terms(&corpus("docs/d001_ok_full.md")).expect("the corpus's full document passes");
    let o = k2::derive::offer(&t, 0).expect("candidate 0");
    (t, o)
}

const BUYER: &str = "0x00000000000000000000000000000000000000b1";
const PAYOUT: &str = "0x00000000000000000000000000000000000000b2";
const ZERO: &str = "0x0000000000000000000000000000000000000000";

#[test]
fn prev_is_a_hex20_zero_admitted_and_enters_the_salt() {
    let (t, o) = offer();
    let first = k2::derive::order(&o, t.chain_id, &t.logic, BUYER, PAYOUT, 1_850_000_000, ZERO).expect("zero prev");
    let later = k2::derive::order(&o, t.chain_id, &t.logic, BUYER, PAYOUT, 1_850_000_000, &first.instance).expect("a stage before");
    assert_ne!(first.salt, later.salt, "prev enters the salt");
    assert_ne!(first.instance, later.instance, "a later stage is its own instance");
    for bad in ["0x12", "0X0000000000000000000000000000000000000000", "0x00000000000000000000000000000000000000B1", ""] {
        let f = k2::derive::order(&o, t.chain_id, &t.logic, BUYER, PAYOUT, 1_850_000_000, bad).expect_err("not a hex20");
        assert_eq!(f.token, Token::OrderPrev, "{bad:?}");
    }
}

#[test]
fn series_reads_in_one_form_and_refuses_every_other_by_name() {
    let ok = k2::doc::terms(&corpus("docs/d3716_ok_series.md")).expect("a series document");
    let s = ok.series.expect("its series is read");
    assert!(s.stage >= 1 && s.stages >= s.stage);
    assert!(k2::doc::terms(&corpus("docs/d001_ok_full.md")).expect("no series").series.is_none());
    for bad in [
        "docs/d3718_series_array.md",
        "docs/d3719_series_id_zero.md",
        "docs/d3720_series_id_short.md",
        "docs/d3721_series_stage_zero.md",
        "docs/d3723_series_stages_below_stage.md",
        "docs/d3724_series_missing_member.md",
        "docs/d3725_series_extra_member.md",
        "docs/d3735_series_id_upper.md",
    ] {
        let f = k2::doc::terms(&corpus(bad)).expect_err(bad);
        assert_eq!(f.token, Token::TermsSeries, "{bad}");
    }
    let f = k2::doc::terms(&corpus("docs/d3726_order_compliance_before_series.md")).expect_err("compliance first");
    assert_eq!(f.token, Token::TermsCompliance);
}

fn quote() -> Vec<u8> {
    let mut q = vec![0u8; k2::quote::LEN];
    q[31] = 1;
    q
}

#[test]
fn a_quote_is_fifteen_words_and_lapse_takes_the_half() {
    assert_eq!(k2::quote::LEN, 480);
    assert_eq!(k2::quote::decode(&vec![0u8; 512]).expect_err("512 bytes").token, Token::QuoteLen);
    for lapse in 0u8..=2 {
        let mut q = quote();
        q[13 * 32 + 31] = lapse;
        assert_eq!(k2::quote::decode(&q).expect("0, 1 or 2").lapse_default, u64::from(lapse));
    }
    let mut q = quote();
    q[13 * 32 + 31] = 3;
    let f = k2::quote::decode(&q).expect_err("3 is no share");
    assert_eq!((f.token, f.field), (Token::QuoteField, Some(13)));
    let mut q = quote();
    q[14 * 32 + 31] = 2;
    assert_eq!(k2::quote::decode(&q).expect_err("autoAccept 2").field, Some(14));
}

#[test]
fn an_object_signature_is_no_buyer_form() {
    let obj = k1::Value::Obj(vec![("x".into(), k1::Value::Str(format!("0x{}", "1".repeat(64))))]);
    let d = [7u8; 32];
    assert_eq!(k2::sig::verify_buyer(&d, &obj, BUYER).expect_err("object").token, Token::SigForm);
    assert!(!k2::sig::is_signature_shape(&obj));
    assert!(matches!(k2::sig::verify_buyer(&d, &k1::Value::Str(String::new()), BUYER), Ok(k2::sig::Form::Call)));
}

fn trial_quote() -> Vec<u8> {
    let mut q = vec![0u8; k2::quote::TRIAL_WORDS * 32];
    q[31] = 1;
    q[32 + 31] = 7;
    q
}

#[test]
fn a_trial_quote_is_read_in_its_own_sixteen_words() {
    use eravon_literals::constants::Shape;
    let mut q = trial_quote();
    q[2 * 32 + 31] = 25;
    q[14 * 32 + 31] = 2;
    let d = k2::quote::decode_under(Shape::Trial, &q).expect("sixteen words");
    assert_eq!((d.premium_bps, d.lapse_default, d.bytes.len()), (25, 2, 512));
    assert_eq!(k2::quote::decode_under(Shape::Trial, &quote()).expect_err("fifteen words").token, Token::QuoteLen);
    assert_eq!(k2::quote::decode_under(Shape::Current, &trial_quote()).expect_err("sixteen words").token, Token::QuoteLen);
    let mut q = trial_quote();
    q[32] = 1;
    assert_eq!(k2::quote::decode_under(Shape::Trial, &q).expect_err("payout word").field, Some(1));
    let mut q = trial_quote();
    q[14 * 32 + 31] = 1;
    assert_eq!(k2::quote::decode_under(Shape::Trial, &q).expect_err("half share").field, Some(14));
    let mut q = trial_quote();
    q[2 * 32 + 30] = 0x40;
    assert_eq!(k2::quote::decode_under(Shape::Trial, &q).expect_err("premium over 10000").field, Some(2));
    assert_eq!(k2::quote::decode_under(Shape::Current, &quote()).expect("current"), k2::quote::decode(&quote()).expect("current"));
}
