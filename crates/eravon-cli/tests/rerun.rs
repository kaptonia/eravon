mod common;

use common::{node, recorded, seal, Home, Scene};

fn included(status: u64) -> Option<String> {
    Some(format!("{{\"blockNumber\":\"0x40\",\"gasUsed\":\"0x5208\",\"status\":\"0x{status:x}\",\"transactionHash\":\"0x00\"}}"))
}

#[test]
fn a_rerun_asks_about_what_was_sent_before_signing_anything_new() {
    let h = Home::new("rerun-story");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    let r = h.send(&n);
    assert_eq!((r.code, r.get("reason")), (4, "E_UNREACHABLE".to_string()), "{}", r.text);
    let first = n.hash(0);
    assert_eq!(r.get("tx"), first, "the broadcast's trouble carries the hash");
    let mut tx1 = [0u8; 32];
    tx1.copy_from_slice(&eravon::hexfmt::decode(&first).expect("hex"));
    assert_eq!(recorded(&h.record()), vec![(0, tx1)], "landed before it was handed");
    n.set(Scene { holds: true, ..Scene::default() });
    let r = h.send(&n);
    assert_eq!((r.code, r.get("txHash")), (0, first.clone()), "in the pool: {}", r.text);
    assert_eq!((n.broadcasts(), n.asked("eth_estimateGas"), n.asked("eth_getTransactionCount")), (1, 0, 0), "in the pool: nothing new");
    n.set(Scene { receipt: Some("\"0x1\"".into()), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!((r.code, r.get("reason"), r.get("tx")), (4, "E_UNREACHABLE".to_string(), first.clone()), "another shape: {}", r.text);
    assert_eq!((n.broadcasts(), n.asked("eth_estimateGas")), (1, 0), "another shape: nothing new");
    n.set(Scene { receipt: included(1), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!((r.code, r.get("txHash")), (0, first.clone()), "in the block: {}", r.text);
    assert_eq!(r.out.member("receipt").and_then(|x| x.member("block")), Some(&eravon::json::Value::Int(64)));
    assert_eq!(n.broadcasts(), 1, "in the block: nothing new");
    assert_eq!(recorded(&h.record()), vec![], "done: its end is known");
    n.set(Scene { pending: Some(1), latest: 1, ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(n.broadcasts(), 2, "sent afresh: {}", r.text);
    assert_eq!(r.get("txHash"), n.hash(1));
}

#[test]
fn a_void_then_a_send_refused_again_answers_with_the_new_hash() {
    let h = Home::new("rerun-void-again");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    let _ = h.send(&n);
    n.set(Scene { latest: 1, pending: Some(1), ..Scene::default() });
    assert_eq!(h.send(&n).get("reason"), "E_V_SENT_VOID");
    n.set(Scene { refuse: true, latest: 1, pending: Some(1), ..Scene::default() });
    let r = h.send(&n);
    let second = n.hash(1);
    assert_eq!((r.code, r.get("reason"), r.get("tx")), (4, "E_UNREACHABLE".to_string(), second), "{}", r.text);
    assert_eq!(recorded(&h.record()).len(), 1);
}

#[test]
fn of_two_sent_at_one_nonce_the_one_in_a_block_is_the_answer() {
    let h = Home::new("rerun-one-included");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    let _ = h.send(&n);
    n.set(Scene { refuse: true, latest: 0, pending: Some(0), ..Scene::default() });
    let _ = h.send(&n);
    assert_eq!(recorded(&h.record()).len(), 2, "both asked after");
    n.set(Scene { receipt: included(1), latest: 1, pending: Some(1), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!((r.code, r.get("txHash")), (0, n.hash(0)), "{}", r.text);
    assert_eq!(n.broadcasts(), 2, "nothing new");
    assert_eq!(recorded(&h.record()), vec![], "the other at its nonce can never be included");
}

#[test]
fn held_by_no_node_with_its_nonce_unused_it_is_signed_again_at_that_nonce() {
    let h = Home::new("rerun-unused");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    let _ = h.send(&n);
    n.set(Scene { refuse: true, latest: 0, pending: Some(5), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(n.broadcasts(), 2, "{}", r.text);
    let got = recorded(&h.record());
    assert_eq!(got.iter().map(|(nonce, _)| *nonce).collect::<Vec<_>>(), vec![0, 0], "both at nonce zero");
}

#[cfg(unix)]
#[allow(clippy::permissions_set_readonly_false)]
#[test]
fn a_hash_that_cannot_be_landed_sends_nothing() {
    let h = Home::new("rerun-unlanded");
    let n = node();
    let at = h.record();
    std::fs::create_dir_all(&at).expect("the record's folder");
    let mut p = std::fs::metadata(&at).expect("meta").permissions();
    p.set_readonly(true);
    std::fs::set_permissions(&at, p.clone()).expect("read-only");
    let r = h.send(&n);
    p.set_readonly(false);
    std::fs::set_permissions(&at, p).expect("back");
    assert_eq!((r.code, r.get("reason")), (4, "E_V_IO".to_string()), "{}", r.text);
    assert_eq!(n.broadcasts(), 0, "nothing broadcast");
}

#[test]
fn a_record_that_does_not_read_signs_nothing() {
    let n = node();
    for (form, name, bytes) in [("notJson", "0", &b"not json"[..]), ("notNumbered", "x", &b"{}"[..]), ("noTx", "0", &b"{\"nonce\":\"0x0\"}"[..]), ("nonceNotAQuantity", "0", &b"{\"nonce\":7,\"tx\":\"0x00\"}"[..])] {
        let h = Home::new(&format!("rerun-{form}"));
        let at = h.record();
        std::fs::create_dir_all(&at).expect("the record's folder");
        std::fs::write(at.join(name), bytes).expect("a record that does not read");
        let r = h.send(&n);
        assert_eq!((r.code, r.get("reason")), (4, "E_V_IO".to_string()), "{form}: {}", r.text);
        assert!(r.get("detail").contains(&at.display().to_string()), "{form}: the record named: {}", r.get("detail"));
        assert_eq!(n.broadcasts(), 0, "{form}: nothing signed or sent");
    }
}

#[test]
fn a_machine_folder_that_does_not_read_signs_nothing() {
    let h = Home::new("rerun-pointer");
    let n = node();
    let ptr = eravon_os::machine::pointer_dir(&h.home).join(eravon_os::machine::POINTER);
    std::fs::create_dir_all(ptr.parent().expect("a parent")).expect("the pointer's folder");
    std::fs::write(&ptr, "../not a path inside\n").expect("a pointer that does not read");
    let r = h.send(&n);
    assert_ne!(r.code, 0, "{}", r.text);
    assert_eq!(n.broadcasts(), 0, "nothing signed or sent");
}

#[test]
fn a_record_keeps_any_nonce() {
    let h = Home::bare("rerun-nonces");
    let at = h.root.join("record");
    eravon_send::sent::land(&at, &seal(&at), 0, &[1u8; 32]).expect("landed");
    eravon_send::sent::land(&at, &seal(&at), u64::MAX, &[2u8; 32]).expect("landed");
    assert_eq!(recorded(&at), vec![(0, [1u8; 32]), (u64::MAX, [2u8; 32])]);
}

#[test]
fn the_record_is_sealed_to_the_sending_key() {
    let h = Home::bare("rerun-sealed");
    let at = h.root.join("record");
    let tx = [0xabu8; 32];
    eravon_send::sent::land(&at, &seal(&at), 7, &tx).expect("landed");
    let hex_tx = eravon::hexfmt::encode(&tx);
    for e in std::fs::read_dir(&at).expect("the folder") {
        let bytes = std::fs::read(e.expect("an item").path()).expect("reads");
        assert!(!bytes.windows(hex_tx.len() - 2).any(|w| w == hex_tx[2..].as_bytes()), "no hash in the clear");
    }
    let other = eravon_send::sent::Seal::of(&[0x11u8; 32], &at);
    assert!(eravon_send::sent::read(&at, &other).is_err(), "another key reads nothing");
    assert_eq!(eravon_send::sent::read(&at, &seal(&at)).expect("read"), vec![(7, tx)], "the same key reads it back");
    let plain = h.root.join("plain");
    std::fs::create_dir_all(&plain).expect("a folder");
    std::fs::write(plain.join("0"), format!("{{\"nonce\":\"0x3\",\"tx\":\"{hex_tx}\"}}")).expect("written");
    assert_eq!(eravon_send::sent::read(&plain, &seal(&plain)).expect("read"), vec![(3, tx)], "a 0.1.x record still reads");
}

#[test]
fn a_reverted_inclusion_is_said_once_and_the_next_run_sends_afresh() {
    let h = Home::new("rerun-reverted");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    let _ = h.send(&n);
    let first = n.hash(0);
    n.set(Scene { receipt: included(0), latest: 1, pending: Some(1), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(r.code, 1, "{}", r.text);
    assert!(r.text.contains("E_V_REVERTED") && r.text.contains(&first), "{}", r.text);
    assert_eq!(n.broadcasts(), 1, "nothing sent on the run that found it");
    assert!(recorded(&h.record()).is_empty(), "marked done");
    n.set(Scene { latest: 1, pending: Some(1), ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(n.broadcasts(), 2, "sent afresh: {}", r.text);
}

#[test]
fn a_hidden_file_beside_the_record_is_not_a_record() {
    let h = Home::new("rerun-hidden");
    let n = node();
    let at = h.record();
    std::fs::create_dir_all(&at).expect("the record's folder");
    std::fs::write(at.join(".DS_Store"), b"\0\0\0\x01Bud1").expect("a hidden file");
    n.set(Scene { refuse: true, ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(n.broadcasts(), 1, "signed and sent: {}", r.text);
    assert_eq!(recorded(&at).len(), 1);
}

#[test]
fn a_home_handed_through_the_systems_variable_keeps_its_record_apart() {
    let h = Home::new("rerun-apart");
    let n = node();
    n.set(Scene { refuse: true, ..Scene::default() });
    let r = h.send(&n);
    assert_eq!(n.broadcasts(), 1, "signed and sent: {}", r.text);
    assert_eq!(recorded(&h.record()).len(), 1, "recorded under the home it was handed");
    let other = Home::new("rerun-apart-other");
    assert!(!other.record().exists(), "nothing under another home");
    let r = other.send(&n);
    assert_eq!(n.broadcasts(), 2, "the other home starts from nothing and sends: {}", r.text);
    assert_eq!(n.asked("eth_getTransactionByHash"), 0, "and asks about nothing it did not send");
}

#[cfg(unix)]
#[allow(clippy::permissions_set_readonly_false)]
#[test]
fn a_home_that_cannot_be_written_sends_nothing() {
    let h = Home::new("rerun-unwritable");
    let n = node();
    let m = h.machine();
    let mut p = std::fs::metadata(&m).expect("meta").permissions();
    p.set_readonly(true);
    std::fs::set_permissions(&m, p.clone()).expect("read-only");
    let r = h.send(&n);
    p.set_readonly(false);
    std::fs::set_permissions(&m, p).expect("back");
    assert_ne!(r.code, 0, "{}", r.text);
    assert_eq!(n.broadcasts(), 0, "nothing broadcast");
}

#[cfg(unix)]
#[test]
fn no_home_sends_nothing() {
    let h = Home::bare("rerun-nohome");
    let n = node();
    for (form, r) in [
        ("unset", h.send_where(&n, |c| {
            c.env_remove(eravon_os::HOME_VAR);
        })),
        ("empty", h.send_where(&n, |c| {
            c.env(eravon_os::HOME_VAR, "");
        })),
    ] {
        assert_ne!(r.code, 0, "{form}: {}", r.text);
    }
    assert_eq!(n.broadcasts(), 0, "nothing broadcast");
}
