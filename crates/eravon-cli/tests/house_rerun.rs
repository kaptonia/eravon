mod common;

use common::{node, recorded, Home, Scene};

fn included(status: u64) -> Option<String> {
    Some(format!("{{\"blockNumber\":\"0x40\",\"gasUsed\":\"0x5208\",\"status\":\"0x{status:x}\",\"transactionHash\":\"0x00\"}}"))
}

#[test]
fn one_signed_again_and_included_leaves_nothing_at_its_nonce_to_ask_after() {
    let h = Home::new("rerun-again-included");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    assert_eq!(h.send(&n).code, 4, "refused broadcast: unanswered with its hash");
    n.set(Scene { receipt: included(1), only: Some(1), ..Scene::default() });
    let mut args = h.send_args(&n);
    let at = args.iter().position(|a| *a == "--wait").expect("the send waits") + 1;
    args[at] = "5";
    let r = h.run(&args, |_| {});
    assert_eq!((r.code, r.get("txHash")), (0, n.hash(1)), "signed again and included: {}", r.text);
    assert_eq!(recorded(&h.record()), vec![], "the one included is done, the earlier one at its nonce void");
    n.set(Scene { pending: Some(1), latest: 1, ..Scene::default() });
    let r = h.send(&n);
    assert_eq!((r.code, n.broadcasts()), (0, 3), "sent afresh: {}", r.text);
    assert_eq!(r.get("txHash"), n.hash(2));
}

#[test]
fn unknown_with_its_nonce_used_it_is_void_and_the_next_run_sends_afresh() {
    let h = Home::new("rerun-void");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    let _ = h.send(&n);
    n.set(Scene { latest: 1, pending: Some(1), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!((r.code, r.get("reason")), (1, "E_V_SENT_VOID".to_string()), "void: {}", r.text);
    assert_eq!(n.broadcasts(), 1, "void: nothing sent now");
    assert_eq!(recorded(&h.record()), vec![], "the void one no longer asked after");
    n.set(Scene { latest: 1, pending: Some(1), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(n.broadcasts(), 2, "sent afresh: {}", r.text);
    let second = n.hash(1);
    assert_eq!(r.get("txHash"), second);
    let mut tx2 = [0u8; 32];
    tx2.copy_from_slice(&eravon::hexfmt::decode(&second).expect("hex"));
    assert_eq!(recorded(&h.record()), vec![(1, tx2)], "its receipt not waited for: open, to be asked by its hash");
    let account = h.address.trim_start_matches("0x").to_ascii_lowercase();
    for e in std::fs::read_dir(h.record()).expect("the record's folder") {
        let bytes = std::fs::read(e.expect("an item").path()).expect("reads");
        let low = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
        assert!(!low.contains(second.trim_start_matches("0x")) && !low.contains(&account), "neither hash nor account in the clear");
        assert!(!bytes.windows(32).any(|w| w == tx2), "no raw hash");
    }
    let other = eravon_send::sent::Seal::of(&[0x11u8; 32], &h.record());
    assert!(eravon_send::sent::read(&h.record(), &other).is_err(), "another key reads nothing");
}
