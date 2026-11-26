use eravon::json::Value;
use eravon_chain::rpc::{Endpoint, Trouble};
use eravon_chain::wire::{self, W};
use eravon_chain::send;

fn w(json: &str) -> W {
    wire::parse(json.as_bytes()).expect("样本不是 JSON")
}

#[test]
fn the_transport_reader_takes_what_the_law_does_not() {
    let v = wire::parse(b"{\"code\":-32000,\"ratio\":1.5}").unwrap();
    assert_eq!(v.member("code").unwrap().as_u64(), None, "负数不是 §3 的整数,也不冒充一个");
    assert!(wire::to_core(&v).is_none(), "落不进 §3 值域的值不许被折进去");
    let ok = wire::parse(b"{\"a\":1}").unwrap();
    assert!(wire::to_core(&ok).is_some());
}

#[test]
fn a_value_keeps_the_bytes_it_was_written_as() {
    let src = b"{\"basis\":{\"b\":1,\"a\":2}}";
    let v = wire::parse(src).unwrap();
    assert_eq!(v.member("basis").unwrap().raw(src), b"{\"b\":1,\"a\":2}", "原文照留:重写一遍会把形上的病治好");
}

#[test]
fn a_reading_stands_only_when_the_endpoints_agree() {
    use eravon_chain::endpoints::agree;
    let one = Value::Obj(vec![("anchors".into(), Value::Arr(vec![]))]);
    let two = Value::Obj(vec![("anchors".into(), Value::Arr(vec![Value::Null]))]);
    let r = agree(vec![("a".into(), one.clone()), ("b".into(), one.clone())]).ok().unwrap();
    assert!(!r.single_source);
    let single = agree(vec![("a".into(), one.clone())]).ok().unwrap();
    assert!(single.single_source, "单源那一栏必须随读数走");
    assert!(agree(vec![("a".into(), one), ("b".into(), two)]).is_err(), "不一致不取多数");
}

#[test]
fn one_endpoint_on_one_chain_is_a_single_source_however_many_rounds_ran() {
    use eravon_chain::endpoints::agree_over;
    let f = Value::Obj(vec![("anchors".into(), Value::Arr(vec![]))]);
    let r = agree_over(
        vec![("1=A,2=C".into(), f.clone()), ("1=B,2=C".into(), f.clone())],
        vec![2],
    )
    .ok()
    .unwrap();
    assert!(r.single_source, "跑了两趟不等于有两个来源");
    assert_eq!(r.single_source_chains, vec![2]);
    let both = agree_over(vec![("1=A".into(), f.clone()), ("1=B".into(), f)], vec![]).ok().unwrap();
    assert!(!both.single_source);
}

#[test]
fn an_incomplete_receipt_is_an_answer_so_the_deadline_says_not_yet() {
    struct Half;
    impl Endpoint for Half {
        fn call(&mut self, method: &str, _params: &Value) -> Result<W, Trouble> {
            match method {
                "eth_getTransactionReceipt" => Ok(w("{\"transactionHash\":\"0x0000000000000000000000000000000000000000000000000000000000000001\"}")),
                _ => Err(Trouble::Transport(eravon_chain::rpc::Broke::Said("not asked here".into()))),
            }
        }
        fn name(&self) -> String {
            "half".into()
        }
        fn place(&self) -> String {
            self.name()
        }
    }
    let mut ep = Half;
    let got = send::confirm_each(&mut [&mut ep], &[1u8; 32], std::time::Duration::from_millis(0), &[]);
    assert!(matches!(got, send::Confirm::NotYet), "收据不全也是答了");
}

#[test]
fn a_broadcast_transaction_keeps_its_hash_even_when_the_endpoint_goes_quiet() {
    struct Flaky {
        sent: bool,
    }
    impl Endpoint for Flaky {
        fn call(&mut self, method: &str, _params: &Value) -> Result<W, Trouble> {
            match method {
                "eth_sendRawTransaction" => {
                    self.sent = true;
                    Ok(w("\"0x0000000000000000000000000000000000000000000000000000000000000000\""))
                }
                _ => Err(Trouble::Transport(eravon_chain::rpc::Broke::Said("the endpoint broke off".into()))),
            }
        }
        fn name(&self) -> String {
            "flaky".into()
        }
        fn place(&self) -> String {
            self.name()
        }
    }
    let mut ep = Flaky { sent: false };
    let raw = [0x02u8, 0xc0];
    let hash = eravon::cryptox::keccak256(&raw);
    let out = send::submit(&mut ep, &raw, &hash);
    assert!(matches!(out, Err(Trouble::Transport(eravon_chain::rpc::Broke::EchoNotOurs))), "节点回的哈希不是我们签的那一笔,当场停:{out:?}");
    assert!(ep.sent, "那一笔确实广播过");
}

#[test]
fn every_public_verb_of_this_crate_marks_the_trace() {
    let src = |f: &str| std::fs::read_to_string(format!("{}/src/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    for (file, sig) in [("send.rs", "pub fn submit("), ("endpoints.rs", "pub fn agree_over(")] {
        let text = src(file);
        let at = text.find(sig).unwrap_or_else(|| panic!("{file} 里没有 {sig}"));
        let body = &text[at..];
        let head: String = body.chars().take(600).collect();
        assert!(head.contains("crate::seam()"), "{file} 的 {sig} 体首没有吐记号");
    }
}
