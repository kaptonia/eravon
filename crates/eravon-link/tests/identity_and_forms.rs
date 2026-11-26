use eravon_galeed as k1;
use eravon_link as k3;
use k3::identity::{self, Pay, Records, Source};
use k3::token::Token;

fn corpus(name: &str) -> Vec<u8> {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base/eravon-conformance/link-corpus").join(name);
    std::fs::read(&at).unwrap_or_else(|e| panic!("{}: {e}", at.display()))
}

fn records(name: &str) -> Records {
    let v = k1::canon::read_open(&corpus(name)).expect("the corpus record set reads");
    Records::read(&v).expect("a record set of its form")
}

const SELLER: &str = "0xd2869cb9ca7046104af35d42678da7f8b53bf0c5";
const TERMS: &str = "0xf44db2165e72662499b472548965aac9cb5a7d940a0a677f7830a278018f8615";

#[test]
fn a_receipt_carries_prev_as_a_hex20() {
    for bad in ["receipts/r032_prev_short.json", "receipts/r033_prev_upper.json", "receipts/r034_prev_int.json", "receipts/r065_adj_prev_before_sig.json"] {
        assert_eq!(k3::receipt::receipt(&corpus(bad)).expect_err(bad).token, Token::ReceiptPrev, "{bad}");
    }
}

#[test]
fn a_pack_with_a_key_is_no_pack() {
    assert_eq!(k3::pack::pack(&corpus("packs/p002_closed_key.json")).expect_err("key").token, Token::PackClosed);
}

#[test]
fn a_document_is_placed_by_its_own_posting_then_its_case_then_its_revocation() {
    let at = |n: &str| identity::place(&records(n), SELLER, TERMS);
    assert_eq!(at("records/r001_place_self.json"), Some(((10, 0), Source::Own)));
    assert_eq!(at("records/r002_place_opened.json"), Some(((20, 1), Source::Opened)));
    assert_eq!(at("records/r003_place_revoked.json"), Some(((25, 4), Source::Revoked)));
    assert_eq!(at("records/r007_place_opened_first_self_wins.json"), Some(((10, 0), Source::Own)));
    assert_eq!(at("records/r008_place_least_posting.json"), Some(((10, 7), Source::Own)));
    for none in ["records/r005_place_none.json", "records/r010_place_repost.json", "records/r012_place_document_invalid.json", "records/r013_place_key_mismatch.json"] {
        assert_eq!(at(none), None, "{none}");
    }
}

#[test]
fn the_check_before_paying_answers_from_the_book() {
    let sb = corpus("listings/l3896_identity_a.json");
    let pay = |n: &str| identity::pay_check(&records(n), &sb).expect("a listing that passes");
    assert_eq!(pay("records/r015_pay_payable.json"), Pay::Payable);
    assert_eq!(pay("records/r016_pay_unplaced.json"), Pay::Unplaced);
    assert_eq!(pay("records/r018_pay_retired.json"), Pay::SellerKeyRetired);
    assert_eq!(pay("records/r019_pay_pending.json"), Pay::Payable, "a declaration alone moves nothing");
    assert_eq!(pay("records/r031_pay_same_block_after.json"), Pay::SellerKeyRetired);
    assert_eq!(pay("records/r033_pay_acceded_key.json"), Pay::Payable);
    let v = identity::pay_check(&records("records/r015_pay_payable.json"), b"not a listing").expect_err("invalid");
    assert_eq!(v.name, "LISTING_INVALID");
}

#[test]
fn a_history_is_the_whole_record_whichever_key_asks() {
    let first = identity::history(&records("records/r047_history_three_keys_first.json"), SELLER);
    assert_eq!(first.keys.len(), 3);
    assert_eq!(first.keys[0], SELLER, "the root first");
    assert_eq!(first.documents.len(), 4);
    assert_eq!(first.attempts().count(), 1);
    let second = identity::history(&records("records/r049_history_three_keys_second.json"), "0x00944bb8234cb8afb40c13eaee835089cafbc86d");
    assert_eq!(second.keys, first.keys);
    assert_eq!(second.documents.len(), first.documents.len());
}

#[test]
fn a_record_set_is_read_in_its_one_form() {
    let v = k1::canon::read_open(&corpus("records/r001_place_self.json")).expect("reads");
    assert!(Records::read(&v).is_some());
    let k1::Value::Obj(mut m) = v.clone() else { panic!("an object") };
    m.push(("extra".into(), k1::Value::Arr(Vec::new())));
    assert!(Records::read(&k1::Value::Obj(m)).is_none(), "a member beyond the nine");
    let k1::Value::Obj(m) = v else { panic!("an object") };
    let without: Vec<_> = m.into_iter().filter(|(k, _)| k != "accessions").collect();
    assert!(Records::read(&k1::Value::Obj(without)).is_none(), "a list missing");
}
