use eravon::json::Value;
use eravon_chain::judge::{self, Missing, NoReading};
use eravon_chain::rpc::Trouble;
use eravon_chain::send::{self, NoGas, GAS_LIMIT};
use eravon_chain::wire::{self, W};

fn w(s: &str) -> W {
    wire::parse(s.as_bytes()).expect("a wire value")
}

fn estimate(answer: Result<Value, Trouble>) -> Result<u64, NoGas<Trouble>> {
    send::estimate_gas(|| Ok(0x40), |_| answer, Value::Null, |t| matches!(t, Trouble::Transport(_)))
}

#[test]
fn an_estimate_is_read_once_by_its_shape_and_held_at_the_ceiling() {
    let q = |s: &str| estimate(Ok(Value::Str(s.into())));
    assert_eq!(q("0x0"), Ok(0), "zero is a quantity (its limit is zero too: `limit_for(0) == 0`)");
    assert_eq!(q("0x1000000"), Ok(GAS_LIMIT), "exactly the ceiling");
    assert_eq!(q("0x1000001"), Err(NoGas::OverCap(u128::from(GAS_LIMIT) + 1)), "one past it");
    assert_eq!(q(&format!("0x{}", "f".repeat(32))), Err(NoGas::OverCap(u128::MAX)), "the widest quantity read");
    assert_eq!(q("0x00010"), Ok(16), "leading zeros read");
    for bad in ["0X10", "0x", &format!("0x1{}", "0".repeat(32)), "10", "0xzz"] {
        assert_eq!(q(bad), Err(NoGas::Unreadable(bad.to_string())), "{bad}");
    }
    assert_eq!(estimate(Ok(Value::Int(16))), Err(NoGas::NotText(Value::Int(16))));
    let net = Trouble::Transport(eravon_chain::rpc::Broke::Said("broke".into()));
    let node = Trouble::Node("{\"code\":3,\"message\":\"execution reverted\"}".into());
    assert_eq!(estimate(Err(net.clone())), Err(NoGas::Network(net.clone())), "the estimate broke: the network's");
    assert_eq!(estimate(Err(node.clone())), Err(NoGas::Refused(node.clone())), "the estimate refused: the call's");
    let head_net = send::estimate_gas(|| Err(net.clone()), |_| Ok(Value::Str("0x1".into())), Value::Null, |t| matches!(t, Trouble::Transport(_)));
    assert_eq!(head_net, Err(NoGas::Network(net)), "the head broke: the network's, the estimate never asked");
    let head_node = send::estimate_gas(|| Err(node.clone()), |_| Ok(Value::Str("0x1".into())), Value::Null, |t| matches!(t, Trouble::Transport(_)));
    assert_eq!(head_node, Err(NoGas::Refused(node)));
    assert_eq!((send::limit_for(30_000), send::limit_for(11_184_811), send::limit_for(GAS_LIMIT)), (45_000, GAS_LIMIT, GAS_LIMIT));
}

#[test]
fn a_plain_estimate_is_read_the_same_way_without_the_anchor_ceiling() {
    let plain = |answer: Result<Value, Trouble>| send::estimate_plain_gas(|| Ok(0x40), |_| answer, Value::Null, |t| matches!(t, Trouble::Transport(_)));
    let q = |s: &str| plain(Ok(Value::Str(s.into())));
    assert_eq!(q("0x3c6f8"), Ok(247_544), "the book's declare on Sepolia");
    assert_eq!(q("0x1000001"), Ok(u64::from(GAS_LIMIT) + 1), "one past the anchor's ceiling");
    assert_eq!(q("0x0"), Ok(0));
    assert_eq!(q(&format!("0x{:x}", send::PLAIN_GAS_MAX)), Ok(send::PLAIN_GAS_MAX), "the field's bound");
    assert_eq!(q(&format!("0x{:x}", u128::from(send::PLAIN_GAS_MAX) + 1)), Err(NoGas::OverCap(u128::from(send::PLAIN_GAS_MAX) + 1)), "one past it");
    assert_eq!(q(&format!("0x{:x}", u64::MAX)), Err(NoGas::OverCap(u128::from(u64::MAX))), "the widest 64-bit quantity");
    assert_eq!(q(&format!("0x{}", "f".repeat(32))), Err(NoGas::OverCap(u128::MAX)), "the widest quantity read");
    assert_eq!(q("0X10"), Err(NoGas::Unreadable("0X10".into())));
    assert_eq!(plain(Ok(Value::Int(16))), Err(NoGas::NotText(Value::Int(16))));
    let net = Trouble::Transport(eravon_chain::rpc::Broke::Said("broke".into()));
    let node = Trouble::Node("{\"code\":3,\"message\":\"execution reverted\"}".into());
    assert_eq!(plain(Err(net.clone())), Err(NoGas::Network(net)));
    assert_eq!(plain(Err(node.clone())), Err(NoGas::Refused(node)));
    assert_eq!(send::plain_limit_for(247_544), 371_316, "one and a half times, the transaction's limit");
    assert_eq!(send::plain_limit_for(1), 2, "rounded up");
    assert_eq!(send::plain_limit_for(0), 0);
    assert_eq!(send::plain_limit_for(send::PLAIN_GAS_MAX), u64::MAX, "the bound's limit fills the field");
    assert_eq!(send::plain_limit_for(u64::MAX), u64::MAX, "past the bound: the field's largest, never wrapped");
    let f = eravon_chain::send::Fees::of(Some(10), Some(3)).with_plain_estimate(247_544);
    assert_eq!((f.gas_limit, f.cap_wei()), (371_316, 23 * 371_316), "the cap the balance check and the card read");
}

#[test]
fn estimates_that_differ_are_a_quantity_read_by_the_largest() {
    let reverted = || Trouble::Node("{\"code\":3,\"message\":\"execution reverted\"}".into());
    let judged = |list: Vec<(&str, Result<&str, Trouble>)>| {
        let answers = list.into_iter().map(|(n, a)| (n.to_string(), a.map(w))).collect();
        judge::judge("eth_estimateGas", &Value::Arr(vec![]), answers)
    };
    let read = |j: &judge::Judged| estimate(Ok(j.value.clone()));
    let j = judged(vec![("http://a", Ok("\"0x443a\"")), ("http://b", Ok("\"0x4398\""))]).expect("differing estimates are read");
    assert_eq!((read(&j), j.sources.len(), j.single_source), (Ok(0x443a), 2, false), "differ");
    let j = judged(vec![("http://a", Ok("\"0x4398\"")), ("http://b", Ok("\"0x443a\"")), ("http://c", Ok("\"0x5ce9\""))]).expect("three");
    assert_eq!(read(&j), Ok(0x5ce9), "the largest of three, wherever it stands");
    let j = judged(vec![("http://a", Ok("\"0x5c12\"")), ("http://b", Ok("\"0x5c12\""))]).expect("alike");
    assert_eq!((read(&j), j.single_source), (Ok(0x5c12), false), "alike");
    let j = judged(vec![("http://a", Err(reverted())), ("http://b", Ok("\"0x4398\""))]).expect("one answered");
    assert_eq!((read(&j), j.single_source), (Ok(0x4398), true), "one refused");
    assert_eq!(j.missing, vec![("http://a".to_string(), Missing::Trouble(reverted()))]);
    match judged(vec![("http://a", Err(reverted())), ("http://b", Err(reverted()))]) {
        Err(NoReading::NoneAnswered(m)) => assert_eq!(m.len(), 2, "all refused: {m:?}"),
        other => panic!("all refused: {other:?}"),
    }
    let j = judged(vec![("http://a", Ok("\"0xzz\"")), ("http://b", Ok("\"0x4398\""))]).expect("one read");
    assert_eq!((read(&j), j.missing.clone()), (Ok(0x4398), vec![("http://a".to_string(), Missing::Unreadable("\"0xzz\"".into()))]), "one not a quantity");
    match judged(vec![("http://a", Ok("\"0xzz\"")), ("http://b", Ok("16"))]) {
        Err(NoReading::NoneAnswered(m)) => assert!(m.len() == 2 && m.iter().all(|(_, x)| matches!(x, Missing::Unreadable(_))), "none a quantity: {m:?}"),
        other => panic!("none a quantity: {other:?}"),
    }
    let j = judged(vec![("http://a", Ok("\"0x1000000\"")), ("http://b", Ok("\"0x1000001\""))]).expect("read");
    assert_eq!(read(&j), Err(NoGas::OverCap(u128::from(GAS_LIMIT) + 1)), "the largest one past the ceiling");
    let j = judged(vec![("http://a", Ok("\"0x0\"")), ("http://b", Ok("\"0x1000000\""))]).expect("read");
    assert_eq!(read(&j), Ok(GAS_LIMIT), "zero beside the ceiling");
    let j = judged(vec![("http://a", Ok("\"0x0\"")), ("http://b", Ok("\"0x0\""))]).expect("read");
    assert_eq!(read(&j), Ok(0), "zero alone");
    let widest = format!("\"0x{}\"", "f".repeat(32));
    let j = judged(vec![("http://a", Ok(widest.as_str())), ("http://b", Ok("\"0x1\""))]).expect("read");
    assert_eq!(read(&j), Err(NoGas::OverCap(u128::MAX)), "the widest quantity");
    let past = format!("\"0x1{}\"", "0".repeat(32));
    let j = judged(vec![("http://a", Ok(past.as_str())), ("http://b", Ok("\"0x1\""))]).expect("read");
    assert_eq!((read(&j), j.missing.len()), (Ok(1), 1), "one digit past the widest is not a quantity");
}

#[test]
fn a_replacement_rises_by_the_pools_bump_at_least() {
    use eravon_chain::send::{Fees, REPLACE_BUMP_PERCENT};
    assert_eq!(REPLACE_BUMP_PERCENT, 10);
    let old = Fees { max_fee: 2_000_000_001, priority: 1_000_000_000, gas_limit: 90_000, from_chain: true };
    let low = Fees::of(Some(100), Some(5));
    let r = Fees::replacing(old, low);
    assert_eq!((r.max_fee, r.priority, r.gas_limit), (2_200_000_002, 1_100_000_000, 90_000));
    let high = Fees::of(Some(5_000_000_000), Some(1_000_000_000));
    let r = Fees::replacing(old, high);
    assert_eq!((r.max_fee, r.priority, r.gas_limit), (11_000_000_000, 1_100_000_000, 90_000));
    assert!(r.from_chain);
    let tiny = Fees { max_fee: 1, priority: 1, gas_limit: 21_000, from_chain: true };
    let r = Fees::replacing(tiny, Fees::of(Some(0), Some(0)));
    assert_eq!((r.max_fee, r.priority), (2, 2));
    assert!(r.priority <= r.max_fee);
}
