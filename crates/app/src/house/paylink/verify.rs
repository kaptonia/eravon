use super::*;

pub use eravon_cli::deskops::paypage::NOT_LISTING;

pub fn open_plan(text: &str) -> Result<(), Said> {
    k3::transport::decode_link(text.trim().as_bytes())
        .map(|_| ())
        .map_err(|e| Said::bad(e.token.name(), crate::house::i18n::t(crate::house::i18n::Key::PlNotPayLink)))
}

pub fn verify(text: &str, buyer: &str, chain: Result<&eravon_pay::check::Chain, &eravon_pay::fault::Fault>) -> Result<Opened, Said> {
    crate::house::mark::mark(crate::house::mark::FEATURE_P1);
    let (link, _) = k3::transport::decode_link(text.trim().as_bytes())
        .map_err(|e| Said::bad(e.token.name(), crate::house::i18n::t(crate::house::i18n::Key::PlNotPayLink)))?;
    let listing = match chain {
        Ok(c) => eravon_cli::buyer::posted(&c.w, &link).map_err(|f| Said::bad(f.name(), f.detail()))?,
        Err(f) => return Err(Said::bad(&f.token, f.detail.clone())),
    };
    verify_listing(&link, &listing, buyer, chain)
}

pub fn verify_listing(link: &[u8], listing: &[u8], buyer: &str, chain: Result<&eravon_pay::check::Chain, &eravon_pay::fault::Fault>) -> Result<Opened, Said> {
    let words = words();
    let hooks = hooks(&words);
    match chain {
        Ok(c) => eravon_cli::deskops::paypage::verify_listing(link, listing, buyer, Ok(&c.view()), &hooks),
        Err(f) => eravon_cli::deskops::paypage::verify_listing(link, listing, buyer, Err(&(f.token.clone(), f.detail.clone())), &hooks),
    }
}

fn words() -> [String; 10] {
    use crate::house::i18n::{t, Key};
    [t(Key::PlLinkBroken), t(Key::PlNotPayLink), t(Key::PlRevoked), t(Key::PlAllHiddenDocs), t(Key::PlPriceStale), t(Key::PlAllHiddenUnregistered), t(Key::PlAllHiddenRetired), t(Key::PlDepositShort), t(Key::PlAllHiddenAct), t(Key::PlGoodsPrivate)]
}

fn hooks(words: &[String; 10]) -> eravon_cli::deskops::paypage::PayHooks<'_> {
    eravon_cli::deskops::paypage::PayHooks {
        candidates: eravon_pay::check::candidates_on,
        standard: eravon_pay::check::standard_on,
        price: eravon_pay::check::price_on,
        gap: eravon_pay::check::gap,
        gate,
        words: eravon_cli::deskops::paypage::PayWords {
            link_broken: &words[0],
            not_pay_link: &words[1],
            revoked: &words[2],
            all_hidden_docs: &words[3],
            price_stale: &words[4],
            all_hidden_unregistered: &words[5],
            all_hidden_retired: &words[6],
            deposit_short: &words[7],
            all_hidden_act: &words[8],
            private_goods: &words[9],
        },
    }
}

#[cfg(test)]
use eravon_cli::deskops::paypage::{act_note, hidden_of};

#[cfg(test)]
fn deposit_step(open: &[usize], notes: &[Note]) -> (&'static str, State) {
    eravon_cli::deskops::paypage::deposit_step(open, notes, &hooks(&words()))
}

#[cfg(test)]
fn act_step(rest: &[usize], notes: &[Note]) -> (&'static str, State) {
    eravon_cli::deskops::paypage::act_step(rest, notes, &hooks(&words()))
}

#[cfg(test)]
use eravon_cli::deskops::paypage::Note;

pub(super) fn gate(q: Option<&k2::quote::Quote>, amount: &k2::u256::U256, who: &str, parties: &[String], kind: &str) -> (bool, &'static str, String) {
    eravon_cli::deskops::paypage::gate(q, amount, who, parties, kind)
}


pub(super) use eravon_cli::deskops::paypage::opened_rows;

pub(super) fn done_rows(d: &eravon_import::import::Done) -> Out {
    let cases = std::iter::once(&d.order)
        .map(|o| {
            let mut r = vec![
                ("caseId", Out::S(o.case_id.clone())),
                ("instance", Out::S(o.instance.clone())),
                ("termsHash", Out::S(o.terms_hash.clone())),
                ("termsSaved", Out::B(o.terms_saved)),
                ("tx", Out::S(o.tx.clone())),
            ];
            if let Some(f) = &o.why {
                r.push(("why", Out::O(vec![("detail", Out::S(f.detail.clone())), ("name", Out::S(f.token.clone()))])));
            }
            if let Some(d) = o.decimals {
                r.push(("decimals", Out::N(d)));
            }
            r.sort_by(|a, b| a.0.cmp(b.0));
            Out::O(r)
        })
        .collect();
    Out::O(vec![
        ("access", Out::S(d.mode.name().into())),
        ("address", Out::S(d.address.clone())),
        ("cases", Out::A(cases)),
        ("label", Out::S(d.label.clone())),
        ("readOnly", Out::S(d.mode.note().into())),
    ])
}

pub(super) fn paid_view(r: eravon_pay::pay::Paid) -> PaidView {
    use eravon_pay::pay::Stop;
    let mut v = PaidView {
        buyer_key: r.buyer_key,
        bumped: r.bumped,
        instance: r.instance,
        case_id: r.case_id,
        id: r.id,
        stop: "opened",
        pay_tx: String::new(),
        open_tx: String::new(),
        short_address: String::new(),
        gap: String::new(),
        decimals: None,
        stale: None,
    };
    match r.stop {
        Stop::Opened { pay_tx, open_tx } => {
            v.pay_tx = pay_tx;
            v.open_tx = open_tx;
        }
        Stop::Short { address, gap, decimals } => {
            v.stop = "short";
            v.short_address = address;
            v.gap = gap;
            v.decimals = decimals;
        }
        Stop::Stale(f) => {
            v.stop = "stale";
            v.stale = Some(Said::bad(&f.token, f.detail));
        }
        Stop::Revoked(f) => {
            v.stop = "revoked";
            v.stale = Some(Said::bad(&f.token, f.detail));
        }
        Stop::DepositShort(f) => {
            v.stop = "depositShort";
            v.stale = Some(Said::bad(&f.token, f.detail));
        }
        Stop::AwaitSeller { pay_tx } => {
            v.stop = "awaitSeller";
            v.pay_tx = pay_tx;
        }
        Stop::AwaitStage { pay_tx, .. } => {
            v.stop = "awaitStage";
            v.pay_tx = pay_tx;
        }
        Stop::Locked { pay_tx, funded } => {
            v.stop = "locked";
            v.stale = Some(Said::bad("E_VAULT_LOCKED", if funded { crate::house::seam::PL_LOCKED_BETWEEN } else { crate::house::seam::PL_LOCKED_BEFORE }));
            v.pay_tx = pay_tx;
        }
    }
    v
}

pub(super) fn paid_rows(p: &PaidView) -> Out {
    let mut m = vec![
        ("buyerKey", Out::S(p.buyer_key.clone())),
        ("bumped", Out::B(p.bumped)),
        ("caseId", Out::S(p.case_id.clone())),
        ("id", Out::S(p.id.clone())),
        ("instance", Out::S(p.instance.clone())),
        ("stop", Out::S(p.stop.into())),
    ];
    if p.stop == "opened" {
        m.push(("openTx", Out::S(p.open_tx.clone())));
    }
    if !p.pay_tx.is_empty() {
        m.push(("payTx", Out::S(p.pay_tx.clone())));
    }
    if p.stop == "short" {
        m.push(("address", Out::S(p.short_address.clone())));
        m.push(("gap", Out::S(p.gap.clone())));
        if let Some(d) = p.decimals {
            m.push(("decimals", Out::N(d)));
        }
    }
    if let Some(s) = &p.stale {
        m.push(("stale", s.out()));
    }
    m.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(m)
}

pub fn land_link(sh: &mut crate::house::shell::Shell, data: &str, text: &str, d: &crate::house::link::Dispatched) -> &'static str {
    use crate::house::link::Route;
    use crate::house::seller::{Leave, Page};
    use crate::house::shell::Seat;
    let route = crate::house::link::route(d);
    if route != Route::Pay {
        return "toast.linkUnknownNext";
    }
    if !sh.go(Leave::Seat(Seat::Buyer)) || !sh.go(Leave::Page(Some(Page::PayLink))) {
        return "goods.askSave";
    }
    let owner = crate::house::shell::current_key(data, Seat::Buyer).unwrap_or_default();
    let mut pl = PayLink::open(&sh.seller.field.home, data, &owner);
    pl.link = text.trim().to_string();
    pl.wants_open = true;
    sh.paylink = Some(pl);
    "toast.linkPay"
}

pub fn deposit_gap(why: &str) -> Option<(String, String, String)> {
    let (who, rest) = why.split_once(" 上有 ")?;
    let (have, need) = rest.split_once(",这一笔要 ")?;
    Some((who.trim().to_string(), have.trim().to_string(), need.trim().to_string()))
}

#[cfg(test)]
mod gap_tests {
    #[test]
    fn a_deposit_shortfall_names_the_vault_and_the_gap() {
        let why = "0x1850000000000000000000000000000000000001 上有 0 WETH,这一笔要 0.000006 WETH";
        assert_eq!(super::deposit_gap(why), Some(("0x1850000000000000000000000000000000000001".into(), "0 WETH".into(), "0.000006 WETH".into())));
        assert_eq!(super::deposit_gap("授权了 0 WETH,这一步要 1 WETH"), None);
        assert_eq!(super::deposit_gap("未绑定"), None);
    }
}

#[cfg(test)]
mod hidden_tests {
    use super::{hidden_of, State};

    #[test]
    fn a_failed_step_says_the_one_reason_every_candidate_is_hidden_by() {
        let n = |i: usize, name: &str| (i, name.to_string(), format!("句{i}"), true);
        let paused = [n(0, "E_ACT_JUDGE_PAUSED"), n(1, "E_ACT_JUDGE_PAUSED"), n(1, "E_P3_QUOTE")];
        assert_eq!(hidden_of(2, &paused, "E_P3_QUOTE", "定话"), State::Fail("E_ACT_JUDGE_PAUSED".into(), "句0".into()));
        let mixed = [n(0, "E_ACT_JUDGE_PAUSED"), n(1, "E_ACT_JUDGE_AMANAH")];
        assert_eq!(hidden_of(2, &mixed, "E_P3_NO_CANDIDATE", "定话"), State::Fail("E_P3_NO_CANDIDATE".into(), "定话".into()));
        assert_eq!(hidden_of(2, &[n(0, ""), n(1, "")], "E_P3_NO_CANDIDATE", "定话"), State::Fail("E_P3_NO_CANDIDATE".into(), "定话".into()));
        assert_eq!(hidden_of(2, &[n(0, "E_ACT_JUDGE_PAUSED")], "E_P3_NO_CANDIDATE", "定话"), State::Pass);
        assert_eq!(hidden_of(1, &[n(0, "E_ACT_JUDGE_PAUSED")], "E_P3_NO_CANDIDATE", "定话"), State::Fail("E_ACT_JUDGE_PAUSED".into(), "句0".into()));
    }

    #[test]
    fn the_deposit_is_a_step_and_a_page_it_empties_does_not_pass() {
        let short = |i: usize| (i, "E_A1_PRECHECK".to_string(), format!("funds:金库 {i} 不足"), true);
        let passed = vec![(crate::house::seam::PL_STEP_PAYLOAD, State::Pass), (crate::house::seam::PL_STEP_CURRENT, State::Pass)];
        let mut all = passed.clone();
        all.push(super::deposit_step(&[0, 1], &[short(0), short(1)]));
        assert_eq!(all.last(), Some(&(crate::house::seam::PL_STEP_DEPOSIT, State::Fail("E_A1_PRECHECK".into(), "funds:金库 0 不足".into()))));
        assert!(!crate::house::paylink::checks_pass(&all), "not passed");
        let mut one = passed;
        one.push(super::deposit_step(&[0, 1], &[short(1)]));
        assert_eq!(one.last(), Some(&(crate::house::seam::PL_STEP_DEPOSIT, State::Pass)));
        assert!(crate::house::paylink::checks_pass(&one), "one left to choose: passed");
        let mut none = vec![(crate::house::seam::PL_STEP_CURRENT, State::Fail("E_P3_OFFER_RETIRED".into(), "x".into()))];
        none.push(super::deposit_step(&[], &[(0, "E_P3_OFFER_RETIRED".to_string(), "x".to_string(), false)]));
        assert_eq!(none.last(), Some(&(crate::house::seam::PL_STEP_DEPOSIT, State::Pass)));
    }

    #[test]
    fn a_candidate_is_choosable_where_the_payment_goes_on_and_the_act_says_why_not() {
        use eravon_cli::buyer::Gate;
        use eravon_standard::precheck as g;
        let r = |gate: &'static str, name: &str| eravon_cands::act::Refused { gate, name: name.to_string(), why: format!("{gate} 不过") };
        assert!(super::act_note(0, &Gate::Go).is_none(), "every gate passed");
        assert!(super::act_note(0, &Gate::AwaitStage(r(g::GATE_STAGE, "E_A1_PRECHECK"))).is_none(), "a later stage, paid now");
        for gate in [g::GATE_BOND, g::GATE_FUNDS, g::GATE_ALLOWANCE] {
            let (note, deposit) = super::act_note(3, &Gate::DepositShort(r(gate, "E_A1_PRECHECK"))).expect("hidden");
            assert!(deposit, "{gate}: the deposit's");
            assert_eq!(note, (3, "E_A1_PRECHECK".to_string(), format!("{gate}:{gate} 不过"), true));
        }
        for (gate, name, refused) in [
            (g::GATE_PRICE, "E_A1_PRICE_STALE", Gate::Stale(r(g::GATE_PRICE, "E_A1_PRICE_STALE"))),
            (g::GATE_REVOKED, "E_A1_REVOKED", Gate::Revoked(r(g::GATE_REVOKED, "E_A1_REVOKED"))),
            (g::GATE_CURRENT, "E_ACT_CURRENT", Gate::Refused(r(g::GATE_CURRENT, "E_ACT_CURRENT"))),
            (g::GATE_WINDOW, "E_ACT_WINDOW", Gate::Refused(r(g::GATE_WINDOW, "E_ACT_WINDOW"))),
            (g::GATE_VERIFIER, "E_ACT_VERIFIER", Gate::Refused(r(g::GATE_VERIFIER, "E_ACT_VERIFIER"))),
            (g::GATE_TERMS, "E_ACT_TERMS", Gate::Refused(r(g::GATE_TERMS, "E_ACT_TERMS"))),
            (g::GATE_SIG, "E_ACT_SIG", Gate::Refused(r(g::GATE_SIG, "E_ACT_SIG"))),
            (g::GATE_CALLER, "E_ACT_CALLER", Gate::Refused(r(g::GATE_CALLER, "E_ACT_CALLER"))),
            ("judge", "E_ACT_JUDGE_PAUSED", Gate::Refused(r("judge", "E_ACT_JUDGE_PAUSED"))),
        ] {
            let (note, deposit) = super::act_note(1, &refused).expect("hidden");
            assert!(!deposit, "{gate}: not the deposit's");
            assert_eq!((note.1.as_str(), note.2.as_str()), (name, format!("{gate}:{gate} 不过").as_str()), "{gate}: the act's own name and sentence");
        }
        let paced = |i: usize| (i, "E_ACT_JUDGE_PAUSED".to_string(), format!("judge:句{i}"), true);
        assert_eq!(super::act_step(&[0, 1], &[paced(0), paced(1)]), (crate::house::seam::PL_STEP_ACT, State::Fail("E_ACT_JUDGE_PAUSED".into(), "judge:句0".into())));
        let mixed = [paced(0), (1, "E_A1_PRICE_STALE".to_string(), "price:陈旧".to_string(), true)];
        assert!(matches!(super::act_step(&[0, 1], &mixed), (_, State::Fail(n, _)) if n == "E_P3_NO_CANDIDATE"));
        assert_eq!(super::act_step(&[0, 1], &[paced(0)]).1, State::Pass, "one left to choose");
        assert_eq!(super::act_step(&[], &[]).1, State::Pass, "nothing left for the act: the step that hid them said why");
        let short = (0, "E_A1_PRECHECK".to_string(), "funds:金库 0 不足".to_string(), true);
        assert_eq!(super::deposit_step(&[0, 1], std::slice::from_ref(&short)).1, State::Pass);
        assert!(matches!(super::act_step(&[1], &[short, paced(1)]).1, State::Fail(n, _) if n == "E_ACT_JUDGE_PAUSED"));
    }
}
