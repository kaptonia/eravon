mod common;

use common::*;

const OFFICE: &str = "0x00000000000000000000000000000000000000a1";
const FORMER: &str = "0x00000000000000000000000000000000000000a2";

#[derive(Debug, Clone)]
struct Posted {
    from: String,
    to: String,
    data: Vec<u8>,
    value: String,
    gas_limit: Option<u64>,
    cap: Option<String>,
}

fn wire(scene: Scene) -> (Node, Node, eravon_cli::shared::Wire) {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let (a, b) = (node(), node());
    a.set(scene.clone());
    b.set(scene);
    let w = eravon_cli::shared::Wire {
        t: eravon_chain::transport::Auto::web_roots().expect("a transport"),
        table: eravon_chain::table::Table::new(vec![a.url.clone(), b.url.clone()]).expect("a table"),
        source: eravon_cli::home::Source::Inline,
        chain_id: CHAIN,
        logic: String::new(),
        registries: Vec::new(),
    };
    (a, b, w)
}

#[test]
fn a_former_keys_token_goes_whole_to_the_key_in_office_its_fee_paid_in_by_the_key_in_office() {
    let (_a, _b, w) = wire(Scene { natives: vec![(FORMER.to_string(), "\"0x0\"".to_string())], ..Scene::default() });
    let mut posted: Vec<Posted> = Vec::new();
    let mut post = |p: &eravon_cli::former::Post<'_>| -> eravon_cli::fault::R<String> {
        posted.push(Posted {
            from: p.from.to_string(),
            to: p.call.to.clone(),
            data: p.call.data.clone(),
            value: p.value.to_string(),
            gas_limit: p.opts.gas_limit,
            cap: p.opts.fees.as_ref().map(|(c, _)| c.clone()),
        });
        Ok(format!("0x{:064x}", posted.len()))
    };
    let m = eravon_cli::former::run(&w, FORMER, OFFICE, &[TOKEN.to_string()], true, &eravon_cli::send::Opts::default(), &mut post, &|| true);
    assert!(m.stop.is_none(), "nothing stops it: {:?}", m.stop);
    assert_eq!(posted.len(), 2, "a fee paid in, then the token: {posted:?}");
    let (fee, tok) = (&posted[0], &posted[1]);
    assert_eq!((fee.from.as_str(), fee.to.as_str()), (OFFICE, FORMER));
    assert!(fee.data.is_empty());
    assert_eq!(tok.from, FORMER);
    let want = eravon_standard::call::erc20_transfer(TOKEN, OFFICE, &eravon_terms::u256::U256::from_u64(1_000_000).to_be()).expect("a transfer");
    assert_eq!((tok.to.as_str(), &tok.data, tok.value.as_str()), (want.to.as_str(), &want.data, "0"));
    let (gas, cap) = (tok.gas_limit.expect("pinned gas"), tok.cap.clone().expect("pinned fees"));
    let most = eravon_terms::u256::U256::from_dec(&cap).expect("a cap").mul_wide(&eravon_terms::u256::U256::from_u64(gas));
    assert_eq!(fee.value, eravon_terms::u256::U256([most.0[0], most.0[1], most.0[2], most.0[3]]).to_dec());
    assert_eq!(m.paid, vec![eravon_cli::former::Leg { token: String::new(), amount: fee.value.clone(), tx: format!("0x{:064x}", 1) }]);
    assert_eq!(m.moved, vec![eravon_cli::former::Leg { token: TOKEN.to_string(), amount: "1000000".into(), tx: format!("0x{:064x}", 2) }]);
    assert_eq!(m.sent, vec![format!("0x{:064x}", 1), format!("0x{:064x}", 2)]);
}

#[test]
fn a_refused_token_stops_the_move_and_nothing_after_it_goes() {
    let (_a, _b, w) = wire(Scene { natives: vec![(FORMER.to_string(), "\"0x0\"".to_string())], ..Scene::default() });
    let mut asked = 0usize;
    let mut post = |_: &eravon_cli::former::Post<'_>| -> eravon_cli::fault::R<String> {
        asked += 1;
        Err(eravon_cli::fault::Fault::Reverted { tx_hash: "0x01".into() })
    };
    let m = eravon_cli::former::run(&w, FORMER, OFFICE, &[TOKEN.to_string()], true, &eravon_cli::send::Opts::default(), &mut post, &|| true);
    assert_eq!(asked, 1, "the fee's refusal stops it before the token and the ETH");
    assert!(matches!(m.stop, Some(eravon_cli::former::Stop::Token(eravon_cli::fault::Fault::Reverted { .. }))), "{:?}", m.stop);
    assert!(m.moved.is_empty() && m.sent.is_empty());
}

#[test]
fn the_verb_refuses_a_key_that_is_not_a_former_key_of_the_seat_and_sends_nothing() {
    let h = Home::new("former-not");
    let n = node();
    for former in [FORMER, h.address.as_str()] {
        let r = h.run(
            &["buyer-wallet-former-move", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--endpoint", &n.url, "--chain", "31337", "--former", former, "--token", TOKEN, "--send", "true", "--passcode", PIN],
            |_| {},
        );
        assert_eq!(r.code, 1, "{former}: {} {}", r.text, r.err);
        assert!(r.text.contains("E_VAULT_NO_IDENTITY"), "{former}: refused by name: {}", r.text);
        assert_eq!(n.broadcasts(), 0, "nothing sent");
        assert_eq!(n.asked("eth_call") + n.asked("eth_getBalance"), 0, "no balance read: {}", r.text);
    }
}
