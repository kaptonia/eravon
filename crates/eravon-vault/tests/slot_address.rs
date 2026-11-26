mod common;

use eravon_vault::fault::Known;
use eravon_vault::{key, keybox, places};

const A: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
const B: &str = "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba";

#[test]
fn a_slot_holding_another_addresss_key_is_refused_for_that_slot_alone() {
    let (_data, _s) = common::home("slot-address");
    let (a, b) = (key::from_hex(A).expect("A"), key::from_hex(B).expect("B"));
    let (addr_a, addr_b) = (a.address().expect("A's address"), b.address().expect("B's address"));
    let (slot_a, slot_b) = (places::key_slot(&addr_a), places::key_slot(&addr_b));
    assert_eq!(places::slot_address(&slot_b), Some(addr_b), "a key slot's name reads back as its address");
    assert_eq!(places::slot_address(places::key_account()), None, "the original slot is named by no address");
    assert_eq!(places::slot_address(&places::seed_slot(&addr_a.hex())), None, "nor is a seed slot");
    key::install(Some(&slot_a), &a).expect("A in its own slot");
    key::install(Some(&slot_b), &a).expect("A in B's slot");
    let refused = key::load(Some(&slot_b)).err().expect("another address's key is refused");
    assert_eq!(refused.which(), Some(Known::KeyMalformed));
    assert!(refused.tail().contains(&slot_b), "the slot is named: {}", refused.tail());
    assert_eq!(key::load(Some(&slot_a)).expect("its own key reads").and_then(|k| k.address()), Some(addr_a), "the other slot reads as before");
    key::install(Some(&slot_b), &b).expect("B back in its slot");
    assert_eq!(key::load(Some(&slot_b)).expect("reads now").and_then(|k| k.address()), Some(addr_b));
    keybox::put(places::key_account(), &[7u8; 32]).expect("the original slot");
    assert!(key::load(Some(places::key_account())).expect("the original slot reads").is_some(), "unchecked: named by no address");
}
