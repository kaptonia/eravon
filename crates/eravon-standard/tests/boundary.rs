use eravon_standard::abi::{self, Val};
use eravon_standard::{call, event, order, sol};
use eravon_terms as k2;

fn word(n: u128) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[16..].copy_from_slice(&n.to_be_bytes());
    w
}

fn order_with(f: impl Fn(&mut order::Order)) -> order::Order {
    let a = |c: char| format!("0x{}", c.to_string().repeat(40));
    let mut o = order::Order {
        offer: k2::derive::Offer {
            seller_key: a('1'),
            judge: a('2'),
            guild: a('0'),
            seller_payout: a('3'),
            seller_fund: a('4'),
            token: a('5'),
            amount: k2::u256::U256::from_dec("1").expect("十进制"),
            delivery_window: 0,
            confirm_window: 0,
            open_window: 0,
            buyer_may_open: false,
            ladder: vec![k2::doc::Rung { buyer_bps: 0, seller_bps: 10_000, fee_bearer: 0 }],
            terms_hash: [0; 32],
            amanah_hash: [0; 32],
            quote_hash: [0; 32],
            nonce: 0,
            expiry: 0,
        },
        buyer_key: a('6'),
        buyer_payout: a('7'),
        open_by: 0,
        prev: order::NO_PREV.to_string(),
    };
    f(&mut o);
    o
}

const INST: &str = "0x9999999999999999999999999999999999999999";

fn init_of(o: &order::Order) -> Result<call::Call, String> {
    call::init(o, INST, &[], &[], &[], &[], &[]).map_err(|f| f.detail())
}

#[test]
fn every_width_passes_at_its_edges_and_refuses_one_past() {
    for (t, max) in [("uint8", 255u128), ("uint16", 65_535), ("uint32", u32::MAX as u128), ("uint64", u64::MAX as u128)] {
        let list = format!("({t})");
        assert!(abi::fits(&list, &[Val::U256(word(0))]).is_ok(), "{t} 零");
        assert!(abi::fits(&list, &[Val::U256(word(max))]).is_ok(), "{t} 最大");
        assert!(abi::fits(&list, &[Val::U256(word(max + 1))]).is_err(), "{t} 过一");
    }
    assert!(abi::fits("(uint256)", &[Val::U256([0xff; 32])]).is_ok(), "uint256 无上界可越");
    assert!(abi::fits("(bool)", &[Val::U256(word(1))]).is_ok());
    assert!(abi::fits("(bool)", &[Val::U256(word(2))]).is_err());
    assert!(abi::fits("((uint16,uint8)[])", &[Val::Arr(vec![Val::Tuple(vec![Val::U256(word(1)), Val::U256(word(256))])])]).is_err(), "元组列里的一位过宽也拒");
}

#[test]
fn init_and_cancel_refuse_a_member_past_its_width() {
    let at_max = order_with(|o| {
        o.offer.delivery_window = u32::MAX as u64;
        o.offer.confirm_window = u32::MAX as u64;
        o.offer.open_window = u32::MAX as u64;
        o.offer.ladder = vec![k2::doc::Rung { buyer_bps: 65_535, seller_bps: 0, fee_bearer: 255 }];
        o.open_by = u64::MAX;
    });
    assert!(init_of(&at_max).is_ok());
    assert!(call::cancel(&at_max, INST, &[]).is_ok());
    let past: [Box<dyn Fn(&mut order::Order)>; 5] = [
        Box::new(|o| o.offer.delivery_window = 1 << 32),
        Box::new(|o| o.offer.confirm_window = 1 << 32),
        Box::new(|o| o.offer.open_window = 1 << 32),
        Box::new(|o| o.offer.ladder[0].buyer_bps = 65_536),
        Box::new(|o| o.offer.ladder[0].fee_bearer = 256),
    ];
    for f in past.iter() {
        let o = order_with(f);
        let why = init_of(&o).expect_err("过宽该拒");
        assert!(why.contains("字宽"), "{why}");
        assert!(call::cancel(&o, INST, &[]).is_err());
    }
}

#[test]
fn revoke_accept_funding_and_post_take_their_edges() {
    let logic = eravon_literals::constants::active_standard().logic;
    let seller = format!("0x{}", "1".repeat(40));
    for th in [[0u8; 32], [0xff; 32]] {
        let c = call::revoke(logic, &seller, &th).expect("写得出");
        assert_eq!(&c.data[..4], &sol::REVOKE.selector());
        assert_eq!(&c.data[16..36], &[0x11u8; 20], "第一参是卖家");
        assert_eq!(&c.data[36..], &th);
    }
    for who in ["0x0000000000000000000000000000000000000000", "0xffffffffffffffffffffffffffffffffffffffff"] {
        let c = call::accept_funding(logic, who).expect("写得出");
        assert_eq!(&c.data[..4], &sol::ACCEPT_FUNDING.selector());
    }
    for doc in [Vec::new(), vec![0x7b; 200_000]] {
        let c = call::post("0x62d4040839c4d4b9480f0fb60028b257acef6420", &[0x11; 32], &doc).expect("写得出");
        let back = abi::decode(&abi::args_of(sol::POST.sig).expect("签名"), &c.data[4..]).expect("读得回");
        assert_eq!(back[1].bytes().expect("bytes"), doc.as_slice());
    }
}

fn refunded(cause: u128) -> Result<event::Log, String> {
    let ev = sol::INSTANCE_EVENTS.iter().find(|e| e.name == "Refunded").expect("Refunded");
    event::decode(&[ev.topic0()], &word(cause)).map_err(|f| f.detail())
}

#[test]
fn refunded_reads_causes_zero_to_four_and_the_one_past() {
    for c in 0..=5u128 {
        let l = refunded(c).expect("在字宽之内");
        assert_eq!(l.at(0).expect("因").small().expect("小数"), c as u64);
    }
    assert!(refunded(256).expect_err("过宽").contains("字宽"));
}

#[test]
fn the_new_events_carry_the_source_indexed_rows() {
    let row = |n: &str| sol::all_events().into_iter().chain([sol::POSTED]).find(|e| e.name == n).unwrap_or_else(|| panic!("{n}")).indexed.to_vec();
    assert_eq!(row("Opened"), vec![true, true, true, false, false, false]);
    assert_eq!(row("Pulled"), vec![true, true, false, false, false]);
    assert_eq!(row("FundBound"), vec![true, true]);
    assert_eq!(row("Posted"), vec![true, true, true, false]);
    assert!(row("Disputed").is_empty());
}

#[test]
fn the_caller_rule_of_init() {
    use eravon_standard::consent::Sender;
    use eravon_standard::precheck::{caller, GATE_CALLER};
    let seller = format!("0x{}", "1".repeat(40));
    let moved = format!("0x{}", "c".repeat(40));
    let buyer = format!("0x{}", "6".repeat(40));
    let closed = order_with(|_| {});
    assert!(caller(&closed, Sender { from: Some(&seller), via_multicall: false }, &seller, &buyer).is_ok());
    assert!(caller(&closed, Sender { from: Some(&moved), via_multicall: false }, &moved, &buyer).is_ok(), "现行钥照收");
    assert_eq!(caller(&closed, Sender { from: Some(&seller), via_multicall: false }, &moved, &buyer).expect_err("冻结钥换过即拒").precheck_gate(), Some(GATE_CALLER));
    for s in [Sender { from: Some(&buyer), via_multicall: false }, Sender { from: Some(&seller), via_multicall: true }, Sender { from: None, via_multicall: false }] {
        assert_eq!(caller(&closed, s, &seller, &buyer).expect_err("该拒").precheck_gate(), Some(GATE_CALLER));
    }
    let open = order_with(|o| o.offer.buyer_may_open = true);
    assert!(caller(&open, Sender { from: Some(&buyer), via_multicall: false }, &seller, &buyer).is_ok());
    assert!(caller(&open, Sender { from: Some(&buyer), via_multicall: true }, &seller, &buyer).is_err(), "经 Multicall3 仍拒");
    assert_eq!(caller(&open, Sender { from: Some(&buyer), via_multicall: false }, &seller, &moved).expect_err("买家钥换过即拒").precheck_gate(), Some(GATE_CALLER));
    let stranger = format!("0x{}", "e".repeat(40));
    assert!(caller(&open, Sender { from: Some(&stranger), via_multicall: false }, &seller, &buyer).is_err(), "外人不开");
}

#[test]
fn init_carries_prev_and_reads_it_back() {
    let stage = format!("0x{}", "a".repeat(40));
    let o = order_with(|o| o.prev = stage.clone());
    let c = init_of(&o).expect("写得出");
    let back = order::decode_init(&c.data).expect("读得回");
    assert_eq!(back.order.prev, stage);
}

#[test]
fn the_renewed_events_carry_the_source_rows() {
    let row = |n: &str| sol::all_events().into_iter().chain([sol::BOOK_ENGAGED]).find(|e| e.name == n).unwrap_or_else(|| panic!("{n}"));
    assert_eq!(row("PanelFrozen").indexed.to_vec(), vec![true, false, false, false, false]);
    assert_eq!(row("Lapsed").sig, "Lapsed(uint16)");
    assert_eq!(row("Engaged").indexed.to_vec(), vec![true, true, false, false, false, false, false, false, false, false]);
}

#[test]
fn a_merged_call_never_carries_init() {
    let o = order_with(|_| {});
    let init = init_of(&o).expect("写得出");
    let other = call::revoke(eravon_literals::constants::active_standard().logic, &format!("0x{}", "1".repeat(40)), &[1; 32]).expect("写得出");
    let e = call::merged(&[&other, &init]).expect_err("合并里有 init 该拒");
    assert_eq!(e.precheck_gate(), Some(eravon_standard::precheck::GATE_CALLER));
    let dep = call::deploy_instance(&[7; 32], eravon_literals::constants::active_standard().logic).expect("写得出");
    let permit_like = call::erc20_transfer(&format!("0x{}", "5".repeat(40)), &format!("0x{}", "9".repeat(40)), &[0; 32]).expect("写得出");
    let by = call::BySig::Dispute { sig: vec![1; 65] }.call(&format!("0x{}", "9".repeat(40))).expect("写得出");
    assert!(call::merged(&[&dep, &permit_like]).is_ok(), "部署与无门的币调用可合");
    assert_eq!(call::merged(&[&permit_like, &by]).expect_err("disputeBySig 有当事人门").precheck_gate(), Some(eravon_standard::precheck::GATE_CALLER));
    assert!(call::merged(&[&dep, &other]).is_err(), "revoke 有当事人门");
}

#[test]
fn status_unpacks_into_sixteen_named_columns() {
    use eravon_standard::chain::{status_col, status_of};
    let cols: Vec<Val> = (0..16u128)
        .map(|i| match i {
            8 => Val::Bool(true),
            14.. => Val::Addr([i as u8; 20]),
            _ => Val::U256(word(i)),
        })
        .collect();
    let out = abi::encode(&[Val::Tuple(cols)]);
    let v = status_of(&out).expect("读得回");
    assert_eq!(v.len(), 16);
    assert_eq!(v[status_col::SEATS].small().expect("数"), 1);
    assert!(v[status_col::PENDING_REDRAW].boolean().expect("真假"));
    assert_eq!(v[status_col::PANEL_SIZE].small().expect("数"), 13);
    assert_eq!(v[status_col::ROOT].addr().expect("地址"), [14u8; 20]);
    assert_eq!(v[status_col::ROUTER].addr().expect("地址"), [15u8; 20]);
}
