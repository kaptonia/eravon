use eravon_chain::send::{self, Unheld};

#[test]
fn transactions_no_node_holds_are_judged_by_the_nonce_on_chain() {
    assert_eq!(send::unheld(None, 5, || true), Unheld::Wait, "not read");
    assert_eq!(send::unheld(Some(5), 5, || panic!("not asked")), Unheld::Unused, "at the sent nonce");
    assert_eq!(send::unheld(Some(0), 5, || panic!("not asked")), Unheld::Unused, "below it");
    assert_eq!(send::unheld(Some(6), 5, || true), Unheld::Void, "past it, none included");
    assert_eq!(send::unheld(Some(u64::MAX), 5, || false), Unheld::Wait, "past it, a receipt heard");
    assert_eq!(send::unheld(Some(1), 0, || true), Unheld::Void, "nonce zero used");
}

