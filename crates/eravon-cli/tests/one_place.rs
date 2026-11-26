mod common;

use common::{Home, PHRASE, PIN};

const NOW: &str = "1758412800";

#[test]
fn a_seller_home_enters_the_buyer_seat_with_its_own_words_key() {
    let fresh = Home::new("bseat-words");
    let h = Home::bare("bseat-seller");
    let made = h.run(&["seller-vault-init", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--phrase", PHRASE], |_| {});
    assert_eq!(made.code, 0, "{}", made.text);
    let got = h.run(&["buyer-vault-init", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN], |_| {});
    assert_eq!(got.code, 0, "{}", got.text);
    assert_eq!(got.get("address"), fresh.address, "the buyer key is the one the same words give a fresh home");
    let list = h.run(&["buyer-vault-list", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN], |_| {});
    assert!(list.text.contains("\"count\":2"), "no key was added apart from the words' own: {}", list.text);
}

#[test]
fn a_new_passcode_without_its_second_entry_is_refused() {
    let h = Home::new("again");
    let got = h.run(&["buyer-vault-passcode", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--new-passcode", "71639482"], |_| {});
    assert_ne!(got.code, 0, "{}", got.text);
    assert!(got.text.contains("--again"), "the missing entry is named: {}", got.text);
    let still = h.run(&["buyer-vault-unlock", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN], |_| {});
    assert_eq!(still.code, 0, "the old passcode still opens the vault: {}", still.text);
}

#[test]
fn an_order_takes_no_typed_prev() {
    let h = Home::new("noprev");
    let got = h.run(&["buyer-order-sign", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--prev", "0x0000000000000000000000000000000000000001"], |_| {});
    assert_eq!(got.code, 2, "a flag this verb does not take is misuse: {} {}", got.text, got.err);
}

#[test]
fn the_order_list_is_refused_while_locked_rather_than_read_on_one_key() {
    let h = Home::new("listlocked");
    let got = h.run(&["buyer-list", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--endpoint", "http://127.0.0.1:9"], |_| {});
    assert_eq!(got.code, 1, "{}", got.text);
    assert_eq!(got.get("downToken"), "E_VAULT_LOCKED", "{}", got.text);
}

#[test]
fn a_transfer_takes_the_wallet_pages_checks() {
    let good = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
    let bad = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAeD";
    assert_eq!(eravon_cli::shared::send_plan(None, None, good).unwrap(), good.to_ascii_lowercase());
    assert!(eravon_cli::shared::send_plan(None, None, bad).is_err(), "a wrong checksum is refused");
    assert!(eravon_cli::shared::send_plan(Some("0x12"), None, good).is_err(), "a token address of another form is refused");
    assert!(eravon_cli::shared::send_plan(None, Some("abc"), good).is_err(), "an amount that is no number is refused");
    let zero = eravon_terms::u256::U256::from_dec("0").unwrap();
    assert_eq!(eravon_cli::shared::send_amount(&zero, "0").unwrap_err().0, "E_P7_AMOUNT");
    assert!(eravon_cli::shared::send_amount(&eravon_terms::u256::U256::from_dec("1").unwrap(), "1").is_ok());
}

#[test]
fn an_export_answer_keeps_both_refusals() {
    use eravon_cli::deskops::export::{Kind, Outcome};
    use eravon_cli::deskops::No;
    let o = Outcome {
        kind: Kind::Dossier,
        files: vec!["d".into()],
        report: Some(Err(No::new("E_RECHECK_READ", vec!["node down".into()]))),
        selfcheck: None,
        shape: None,
        statement: None,
        compliance: Some(Err(No::new("E_EXPORT_NO_COMPLIANCE", vec!["none".into()]))),
    };
    let text = String::from_utf8(eravon_literals::json::write(&o.out())).unwrap();
    assert!(text.contains("\"report\":{\"detail\":\"node down\",\"name\":\"E_RECHECK_READ\"}"), "{text}");
    assert!(text.contains("\"compliance\":{\"detail\":\"none\",\"name\":\"E_EXPORT_NO_COMPLIANCE\"}"), "{text}");
}

#[test]
fn a_sweep_without_a_refund_address_is_refused() {
    assert_eq!(eravon_cli::after_company::payout_of(None, None).unwrap_err().0, "E_P6_NO_PAYOUT");
}

#[test]
fn the_zikaron_verbs_follow_the_identitys_switch() {
    let h = Home::bare("zk");
    let made = h.run(&["seller-vault-init", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--phrase", PHRASE], |_| {});
    assert_eq!(made.code, 0, "{}", made.text);
    let off = h.run(&["seller-settings-zikaron-link", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--on", "false", "--passcode", PIN], |_| {});
    assert_eq!(off.code, 0, "{}", off.text);
    for verb in ["seller-zikaron-status", "seller-zikaron-kits"] {
        let got = h.run(&[verb, "--data", h.data_s(), "--writer", "t", "--now", NOW, "--home", "/nonexistent", "--passcode", PIN], |_| {});
        assert_eq!(got.code, 0, "{}", got.text);
        assert_eq!(got.get("absent"), "E_S9_ZK_OFF", "{verb}: {}", got.text);
    }
}

#[test]
fn the_payment_gate_reads_alike_for_every_payer() {
    use eravon_cli::buyer::Gate;
    let r = |gate: &'static str| eravon_cands::act::Refused { gate, name: "N".into(), why: String::new() };
    assert!(Gate::Go.signs() && Gate::Go.pays());
    assert!(Gate::AwaitStage(r("stage")).signs() && Gate::AwaitStage(r("stage")).pays());
    assert!(Gate::DepositShort(r("funds")).signs() && !Gate::DepositShort(r("funds")).pays());
    for g in [Gate::Revoked(r("revoked")), Gate::Stale(r("price")), Gate::Refused(r("judge"))] {
        assert!(!g.signs() && !g.pays());
    }
}

#[test]
fn a_rebuild_keeps_the_current_key() {
    let h = Home::new("rebuild-current");
    let out = h.root.join("backup");
    std::fs::create_dir_all(&out).expect("an out dir");
    let file_pin = "filepass8";
    let b = h.run(&["buyer-vault-backup", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--file-passcode", file_pin, "--again", file_pin, "--out", out.to_str().expect("utf-8")], |_| {});
    assert_eq!(b.code, 0, "{}", b.text);
    let file = b.get("file");
    let k = h.run(&["buyer-vault-new-key", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN], |_| {});
    assert_eq!(k.code, 0, "{}", k.text);
    let other = k.get("address");
    let sw = h.run(&["buyer-vault-switch", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--address", &other], |_| {});
    assert_eq!(sw.code, 0, "{}", sw.text);
    let current = |h: &Home| {
        let a = h.run(&["buyer-vault-accounts", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", "39517264"], |_| {});
        assert_eq!(a.code, 0, "{}", a.text);
        a.get("current").to_ascii_lowercase()
    };
    let r = h.run(&["buyer-vault-rebuild", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--file", &file, "--file-passcode", file_pin, "--new-passcode", "39517264", "--again", "39517264"], |_| {});
    assert_eq!(r.code, 0, "{}", r.text);
    assert_eq!(r.get("address").to_ascii_lowercase(), h.address.to_ascii_lowercase(), "the file rebuilt the primary key: {}", r.text);
    assert_eq!(current(&h), other.to_ascii_lowercase(), "the current key is still the one switched to");
}

fn code_of(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} 读不出:{e}", path.display()));
    text.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

fn body_of<'a>(code: &'a str, name: &str) -> &'a str {
    let at = code.find(&format!("fn {name}(")).unwrap_or_else(|| panic!("没有 fn {name}"));
    let rest = &code[at..];
    let end = rest[1..].find("\n}").map_or(rest.len(), |e| e + 3);
    &rest[..end]
}

#[test]
fn a_rotation_registers_unless_the_registry_holds_it_on_two_endpoints() {
    let rotate = code_of("eravon-cli/src/vaultverbs/rotate.rs");
    let register = body_of(&rotate, "register");
    assert!(register.contains("a1::consent::registered(") && register.contains("Registration::Held"), "the skip reads `consent::registered`: {register}");
    assert!(!code_of("eravon-standard/src/chain.rs").contains("fn consented("), "no second registry reading stands in the chain module");
}

#[test]
fn the_buyer_page_asks_who_acts_by_generation_before_any_money_moves() {
    let act = code_of("eravon-after/src/act.rs");
    let acts = Some(act.as_str()).map(|x| body_of(x, "acts")).unwrap_or_default();
    assert!(acts.contains("acting_key(") && acts.contains("c.past()") && !acts.contains("chain::current("), "acts reads acting_key by generation: {acts}");
    let dispute = body_of(&act, "dispute");
    let gate = dispute.find("acting(w, b, c)?").expect("the refund request asks who acts");
    let fee = dispute.find("erc20_transfer(").expect("the fee transfer");
    assert!(gate < fee, "who acts is asked before the fee moves: {dispute}");
}

#[test]
fn a_pair_of_sends_is_gated_whole_before_the_first() {
    let shared = code_of("eravon-cli/src/shared.rs");
    let two = body_of(&shared, "deliver_two");
    let gate = two.find("sender_current(").expect("the pair is gated");
    let series = two.find("series(").expect("the pair is sent as a series");
    assert!(gate < series, "both calls are gated before the series: {two}");
}

#[test]
fn a_company_seller_opening_asks_the_opening_gates_from_one_place() {
    let company = code_of("eravon-watch/src/company.rs");
    let open = body_of(&company, "open");
    assert!(open.contains("crate::act::opening_slots(") && open.contains("crate::act::opening("), "the company opening asks the gates: {open}");
    let watch = code_of("eravon-cli/src/seller/watch.rs");
    let gates = Some(watch.as_str()).map(|x| body_of(x, "gates")).unwrap_or_default();
    assert!(gates.contains("eravon_watch::act::opening("), "the seller's own init asks the same function: {gates}");
    assert!(body_of(&watch, "open_calls").contains("eravon_watch::act::opening_slots("), "both slots are read by the same function");
}

#[test]
fn every_payment_asks_the_one_stop() {
    assert!(body_of(&code_of("eravon-cli/src/buyer/company.rs"), "co_draft").contains("must_pay("), "the company order export asks must_pay");
    assert!(body_of(&code_of("eravon-cli/src/buyer/order.rs"), "order_check").contains("must_pay("), "buyer-pay --id asks must_pay");
}

#[test]
fn a_hand_out_is_recorded_only_when_the_paste_would_take_it() {
    let actions = code_of("eravon-cli/src/buyer/actions.rs");
    let confirm = body_of(&actions, "confirm");
    let check = confirm.find("check_pasted(").expect("the hand-out is checked");
    let record = confirm.find("record_confirm(").expect("the hand-out is recorded");
    assert!(check < record, "checked before it is recorded: {confirm}");
}

#[test]
fn every_company_proposal_passes_the_company_sender_gate() {
    let cli = code_of("eravon-cli/src/company.rs");
    let write = body_of(&cli, "write");
    let gate = write.find("proposal_senders(").expect("write asks the exit gate");
    let assemble = write.find("proposal::assemble(").expect("write assembles");
    assert!(gate < assemble, "the gate is asked before anything is assembled: {write}");
    let goods = code_of("eravon-goods/src/company.rs");
    for f in ["export", "revoke_export"] {
        let body = body_of(&goods, f);
        let gate = body.find("senders(").unwrap_or_else(|| panic!("goods {f} asks the exit gate: {body}"));
        let assemble = body.find("proposal::assemble(").expect("assembles");
        assert!(gate < assemble, "goods {f} asks the gate before assembling: {body}");
    }
    assert!(body_of(&goods, "senders").contains("gate::proposal_senders("), "goods asks the same gate");
    for (path, n) in [("eravon-cli/src/company.rs", 1), ("eravon-goods/src/company.rs", 2)] {
        assert_eq!(code_of(path).matches("proposal::assemble(").count(), n, "{path}");
    }
}

#[test]
fn both_signing_roads_ask_the_one_gate_when_they_sign() {
    let order_src = code_of("eravon-cli/src/buyer/order.rs");
    let order = body_of(&order_src, "order_sign");
    let gate = order.find("sign_gate(").expect("order-sign asks the sign gate");
    assert!(gate < order.find("sign_order(").expect("signs"), "order-sign asks before it signs: {order}");
    let pay_src = code_of("eravon-pay/src/pay.rs");
    let pay = body_of(&pay_src, "pay");
    let gate = pay.find("sign_gate(").expect("the window payment asks the sign gate");
    assert!(gate < pay.find("sign_order(").expect("signs"), "the window asks before it signs: {pay}");
    let standing = code_of("eravon-cli/src/buyer/standing.rs");
    let g = body_of(&standing, "sign_gate");
    assert!(g.contains("candidate_gate(Some(w)") && g.contains("stands("), "the gate is the candidate gate and the standing: {g}");
}

#[test]
fn the_company_vault_setup_is_formed_in_one_place() {
    let fund = code_of("eravon-cli/src/seller/fund.rs");
    assert!(body_of(&fund, "fund_setup_company").contains("company_setup("), "the command line takes the builder");
    let wallet = code_of("app/src/house/wallet.rs");
    let setup = body_of(&wallet, "vault_setup");
    assert!(setup.contains("company_mode(") && setup.contains("eravon_cli::seller::company_setup(") && setup.contains("eravon_cli::company::write("), "the desktop takes the builder: {setup}");
    let read = body_of(&wallet, "read_vault");
    assert!(read.contains("company_mode(") && read.contains("views.wallet"), "the desktop reads the company pair: {read}");
}
