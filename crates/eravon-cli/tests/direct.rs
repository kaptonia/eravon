mod common;

use common::{error, node, ok, Home, Scene};

#[test]
fn a_nonce_that_is_not_a_number_signs_nothing_and_says_it_as_it_came() {
    let h = Home::new("direct-nonce");
    let n = node();
    n.set(Scene { pending: None, ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(r.code, 4, "{}", r.text);
    assert!(r.get("detail").contains("not a number"), "the answer as it came: {}", r.text);
    assert_eq!((n.asked("eth_getTransactionCount"), n.broadcasts()), (1, 0), "asked, nothing sent");
    assert!(!h.record().exists(), "nothing signed, so nothing recorded");
}

#[test]
fn estimates_that_differ_take_the_largest() {
    let h = Home::new("direct-estimate");
    let (a, b) = (node(), node());
    a.set(Scene { estimate: ok("\"0x5208\""), ..Scene::default() });
    b.set(Scene { estimate: ok("\"0x7530\""), ..Scene::default() });
    let mut args = h.send_args(&a);
    args.extend(["--endpoint", b.url.as_str()]);
    let r = h.run(&args, |_| {});
    assert_eq!(r.code, 0, "{}", r.text);
    assert_eq!(r.out.member("gasLimit"), Some(&eravon::json::Value::Int(eravon_chain::send::limit_for(30_000))), "{}", r.text);
    assert_eq!(a.broadcasts() + b.broadcasts(), 2, "the same bytes to both");
}

#[test]
fn the_command_line_asks_fees_then_the_estimate_at_the_pinned_head_then_sends_once() {
    let h = Home::new("direct-order");
    let n = node();
    n.set(Scene { estimate: ok("\"0x7530\""), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(r.code, 0, "{}", r.text);
    assert_eq!(r.out.member("gasLimit"), Some(&eravon::json::Value::Int(eravon_chain::send::limit_for(30_000))), "the limit the estimate gives");
    let q = n.questions.lock().expect("questions").clone();
    let at = |m: &str| q.iter().position(|(x, _)| x == m).unwrap_or_else(|| panic!("{m} asked: {q:?}"));
    assert_eq!(q.iter().filter(|(m, _)| m == "eth_blockNumber").count(), 1, "the head is read once: {q:?}");
    for m in ["eth_estimateGas", "eth_getBlockByNumber"] {
        assert!(q.iter().filter(|(x, _)| x == m).all(|(_, p)| p.contains("\"0x40\"")), "{m} at the pinned head: {q:?}");
    }
    assert!(at("eth_blockNumber") < at("eth_estimateGas") && at("eth_estimateGas") < at("eth_getTransactionCount") && at("eth_getTransactionCount") < at("eth_sendRawTransaction"), "{q:?}");
    assert_eq!(n.broadcasts(), 1, "the bytes go out once");
}

#[test]
fn an_estimate_that_gives_no_figure_sends_nothing() {
    let reverted = error(200, "{\"code\":3,\"message\":\"execution reverted\"}");
    let limited = error(200, "{\"code\":-32005,\"message\":\"rate limit exceeded\"}");
    for (form, estimate, code) in [("refusedAboutTheCall", reverted, 1), ("rateLimited", limited, 4), ("pastTheCeiling", ok("\"0x1000001\""), 1), ("notText", ok("16"), 4)] {
        let h = Home::new(&format!("direct-nogas-{form}"));
        let n = node();
        n.set(Scene { estimate, ..Scene::default() });
        let r = h.send(&n);
        assert_eq!(r.code, code, "{form}: {}", r.text);
        assert_eq!((n.asked("eth_getTransactionCount"), n.broadcasts()), (0, 0), "{form}: nothing sent");
        assert!(!h.record().exists(), "{form}: nothing recorded");
    }
}
