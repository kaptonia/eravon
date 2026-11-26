use super::*;


pub(super) fn bad(f: eravon_after::fault::Fault) -> Said {
    Said::bad(&f.token, f.detail)
}

pub fn acts(r: &BRow) -> Vec<&'static str> {
    eravon_cli::deskops::buyerpage::acts(r, read_only)
}

#[cfg(test)]
pub fn acts_under(r: &BRow, switch: bool) -> Vec<&'static str> {
    eravon_cli::deskops::buyerpage::acts_under(r, switch, read_only)
}

pub fn past_lacks(r: &BRow) -> Vec<&'static str> {
    eravon_cli::deskops::buyerpage::past_lacks(r, read_only)
}

pub fn shown_acts(r: &BRow) -> &'static [&'static str] {
    eravon_cli::deskops::buyerpage::shown_acts(r, read_only)
}

#[cfg(test)]
pub(super) use eravon_cli::deskops::buyerpage::budget_acts;

#[cfg(test)]
mod budget_acts_tests {
    use super::*;

    fn row(word: eravon_orders::status::Word, access: Access) -> BRow {
        let row = eravon_orders::table::Row {
            instance: String::new(),
            case_id: String::new(),
            record: "kept".into(),
            number: String::new(),
            buyer: String::new(),
            label: String::new(),
            product: String::new(),
            amount: "0".into(),
            token: String::new(),
            status: Ok(eravon_orders::status::Status { word, reason: String::new() }),
            deadline: 0,
            placed: 0,
            block: None,
            chain: None,
            local: None,
            handed: false,
            note: String::new(),
            terms_hash: String::new(),
            decimals: None,
            past: false,
            logic: String::new(),
            series: None,
        };
        BRow { row, owner: String::new(), record: String::new(), access, pack: None, seller: String::new(), product: String::new(), lf_gap: None, budget: None, receipt: None }
    }

    #[test]
    fn a_handed_confirmation_offers_no_refund_and_a_by_call_order_has_no_action() {
        use eravon_orders::status::Word;
        assert_eq!(acts(&row(Word::AwaitConfirm, Access::Key)), &["confirm", "confirmHand", "dispute", "advance"]);
        assert_eq!(acts(&row(Word::ConfirmHanded, Access::Key)), &["confirm", "advance"]);
        assert_eq!(acts(&row(Word::AwaitConfirm, Access::Company)), &["confirm", "dispute", "advance"]);
        assert!(acts(&row(Word::AwaitConfirm, Access::ByCall)).is_empty());
    }

    #[test]
    fn only_a_keyed_order_is_signed_for_by_this_seat() {
        use eravon_orders::status::Word;
        assert_eq!(super::super::acts::key_party(&row(Word::Disputed, Access::Key)), Ok(()));
        assert!(super::super::acts::key_party(&row(Word::Disputed, Access::ByCall)).is_err());
        assert!(super::super::acts::key_party(&row(Word::Disputed, Access::Company)).is_err());
    }

    #[test]
    fn a_case_on_an_older_contract_acts_only_with_the_switch_on() {
        use eravon_orders::status::Word;
        for (w, a) in [(Word::AwaitConfirm, Access::Key), (Word::AwaitConfirm, Access::Company), (Word::Disputed, Access::Company), (Word::Disputed, Access::Key)] {
            let mut r = row(w, a);
            assert!(!acts(&r).is_empty(), "对照:不是往代即有动作 {w:?} {a:?}");
            r.row.past = true;
            assert!(acts_under(&r, false).is_empty(), "开关关:往代之案不给动作:{w:?} {a:?}");
            assert_eq!(acts_under(&r, true), shown_acts(&r).iter().copied().filter(|x| !eravon_after::act::PAST_LACKS_ACTS.contains(x)).collect::<Vec<_>>());
        }
        let mut r = row(Word::Disputed, Access::Key);
        r.row.past = true;
        r.budget = None;
        assert!(past_lacks(&r).is_empty(), "争议中各键往代都有");
    }

    fn b(owe: &str, lapsed: bool, may: bool) -> Budget {
        Budget { have: "0".into(), need: "10".into(), owe: owe.into(), until: 0, lapsed, revoked: false, emptied: None, buyer_may_open: may, payout: String::new(), receipt: String::new(), pay_uri: String::new(), deposit_short: false }
    }

    #[test]
    fn each_budget_state_has_its_own_keys() {
        assert_eq!(budget_acts(&b("0", true, true)), &["expire"]);
        assert_eq!(budget_acts(&b("10", true, false)), &["expire"]);
        assert_eq!(budget_acts(&b("0", false, true)), &["openCase", "cancel"]);
        assert_eq!(budget_acts(&b("0", false, false)), &["cancel"]);
        assert_eq!(budget_acts(&b("3", false, true)), &["cancel"]);
    }

    #[test]
    fn an_emptied_revoked_order_offers_nothing() {
        let mut e = b("10", false, false);
        e.emptied = Some(eravon_after::act::Take::Revoked);
        assert!(budget_acts(&e).is_empty());
        let mut r = row(eravon_orders::status::Word::Revoked, Access::Key);
        r.budget = Some(Ok(e.clone()));
        assert!(shown_acts(&r).is_empty());
        e.emptied = None;
        e.lapsed = true;
        e.revoked = true;
        r.budget = Some(Ok(e));
        assert_eq!(shown_acts(&r), &["expire"]);
    }
}

pub(super) fn read_only(r: &BRow) -> bool {
    eravon_cli::deskops::buyerpage::read_only(r)
}


pub(super) fn row_out(r: &BRow) -> Out {
    eravon_cli::deskops::buyerpage::row_out(r, &crate::house::coin::rows(), read_only)
}

pub(super) fn detail_rows(data: &str, r: &BRow, w: &eravon_cli::shared::Wire, owners: &[String]) -> Out {
    eravon_cli::deskops::buyerpage::detail_out(data, r, w, owners, fee_line)
}

pub(super) fn fee_line(row: &Row, earns: bool, s2: String) -> String {
    eravon_cli::deskops::buyerpage::fee_line(row, earns, s2)
}


pub(super) fn due_word(d: &eravon_watch::pool::Due) -> &'static str {
    eravon_cli::deskops::orderpage::due_word(d)
}

impl BOrders {
    pub fn rows(&self) -> Out {
        let mut m: Vec<(&'static str, Out)> = vec![("doneTab", Out::B(self.done_tab))];
        let mut sr = vec![("from", Out::S(self.from.clone())), ("q", Out::S(self.q.clone())), ("to", Out::S(self.to.clone()))];
        if let Some(e) = self.search_said() {
            sr.push(("said", e.out()));
        }
        m.push(("search", Out::O(sr)));
        match &self.rows {
            Some(Ok(_)) => m.push(("list", Out::A(self.shown().into_iter().map(row_out).collect()))),
            Some(Err(s)) => m.push(("list", s.out())),
            None => {}
        }
        if let Some(o) = &self.open {
            let mut v: Vec<(&'static str, Out)> = vec![("instance", Out::S(o.instance.clone())), ("tab", Out::S(o.tab.word().into()))];
            if let Some(r) = self.find(&o.instance) {
                v.push(("acts", Out::A(acts(r).iter().map(|a| Out::S((*a).into())).collect())));
                v.push(("pastLacks", Out::A(past_lacks(r).iter().map(|a| Out::S((*a).into())).collect())));
                let locked = r.row.past && !eravon_literals::constants::past_switch();
                v.push(("readOnly", Out::S(if locked { eravon_standard::fault::PAST_CONTRACT_SAYS.into() } else { r.access.note().into() })));
                v.push(("exportMissing", Out::A(act::export_gate(&r.case()).into_iter().map(|x| Out::S(x.into())).collect())));
            }
            if let Some(d) = &o.detail {
                v.push(("detail", d.clone()));
            }
            if let Some(c) = &o.card {
                v.push((
                    "confirmCard",
                    match c {
                        Ok(c) => Out::O(vec![
                            ("amount", Out::S(c.amount.clone())),
                            ("earns", Out::B(c.earns)),
                            ("judgeFee", Out::S(c.judge_fee.clone().unwrap_or_default())),
                            ("toFund", Out::S(c.to_fund.clone().unwrap_or_default())),
                            ("toSeller", Out::S(c.to_seller.clone().unwrap_or_default())),
                        ]),
                        Err(s) => s.out(),
                    },
                ));
            }
            if let Some(c) = &o.advance {
                v.push((
                    "advanceCard",
                    match c {
                        Ok(c) => {
                            let mut m = vec![("due", Out::S(due_word(&c.due).into()))];
                            if let Some((b, s, j)) = &c.shares {
                                m.extend([("toBuyer", Out::S(b.clone())), ("toJudge", Out::S(j.clone())), ("toSeller", Out::S(s.clone()))]);
                            }
                            Out::O(m)
                        }
                        Err(s) => s.out(),
                    },
                ));
            }
            if let Some(c) = &o.refund {
                v.push((
                    "refundSheet",
                    match c {
                        Ok(c) => Out::O(vec![
                            ("lapse", match &c.lapse {
                                Ok((b, sb, fee)) => Out::O(vec![("buyerBps", Out::N(*b)), ("feeBearer", Out::N(*fee)), ("sellerBps", Out::N(*sb))]),
                                Err(e) => Out::O(vec![("why", Out::S(e.clone()))]),
                            }),
                            ("lf", Out::S(c.lf.clone())),
                            ("need", Out::S(c.need.clone())),
                            ("pull", Out::S(c.pull.into())),
                            ("rungs", Out::N(c.rungs.len() as u64)),
                            ("stray", Out::S(c.stray.clone())),
                        ]),
                        Err(s) => s.out(),
                    },
                ));
            }
            if let Some(h) = &o.handed {
                v.push(("handed", Out::O(vec![("data", Out::S(h.data.clone())), ("recorded", Out::B(h.recorded)), ("to", Out::S(h.to.clone()))])));
            }
            if let Some(c) = &o.owed {
                v.push((
                    "owed",
                    match c {
                        Ok(c) => Out::O(vec![
                            ("amount", Out::S(c.amount.clone())),
                            ("balance", Out::S(c.balance.clone())),
                            ("payout", Out::S(c.payout.clone())),
                            ("token", Out::S(c.token.clone())),
                        ]),
                        Err(s) => s.out(),
                    },
                ));
            }
            if let Some(k) = &o.first_kits {
                v.push((
                    "firstKits",
                    match k {
                        Ok(ks) => Out::A(ks.iter().map(|(id, note, n)| Out::O(vec![("contents", Out::N(*n)), ("id", Out::S(id.clone())), ("note", Out::S(note.clone()))])).collect()),
                        Err(s) => s.out(),
                    },
                ));
            }
            if let Some(k) = &o.kits {
                v.push((
                    "kits",
                    match k {
                        Ok(ks) => Out::A(ks.iter().map(|(id, note, n)| Out::O(vec![("contents", Out::N(*n)), ("id", Out::S(id.clone())), ("note", Out::S(note.clone()))])).collect()),
                        Err(s) => s.out(),
                    },
                ));
            }
            if let Some(c) = &o.offer {
                v.push((
                    "offer",
                    match c {
                        Ok(x) => Out::O(vec![
                            ("buyerBps", Out::N(x.st.buyer_bps)),
                            ("expiry", Out::N(x.st.expiry)),
                            ("sellerByCall", Out::B(x.seller_by_call)),
                        ]),
                        Err(s) => s.out(),
                    },
                ));
            }
            if let Some(j) = &o.judge {
                v.push(("judge", crate::house::judge_card::card_out(j)));
                v.push(("judgeOpen", Out::B(o.judge_open)));
            }
            if let Some(h) = crate::house::history::out(&o.history) {
                v.push(("history", h));
            }
            v.push(("sealable", Out::B(o.sealable.is_some())));
            if let Some(x) = &o.sealed {
                v.push(("sealed", crate::house::judge_card::sealed_out(x)));
            }
            if let Some(c) = &o.complaint {
                v.push(("complaint", eravon_cli::deskops::complaint::complaint_out(c)));
            }
            if let Some(i) = o.inner {
                v.push(("inner", Out::S(i.word().into())));
                v.push(("innerTab", Out::N(u64::from(i.tab()))));
            }
            if let Some(b) = &o.bench {
                v.push(("bench", crate::house::case::bench_out(b)));
                if let (Some(crate::house::orders::Inner::Lapse), Ok(x)) = (o.inner, b) {
                    if let Ok(q) = &x.quote {
                        v.push(("lapse", crate::house::case::lapse_out(&q.clocks, matches!(x.track, crate::house::case::Track::Guild(_)))));
                    }
                }
            }
            if let Some(p) = &o.papers {
                v.push(("papers", crate::house::case::papers_out(p)));
            }
            v.sort_by(|a, b| a.0.cmp(b.0));
            m.push(("open", Out::O(v)));
        }
        match &self.exported {
            Some(Ok(p)) => m.push(("exported", Out::S(p.clone()))),
            Some(Err(s)) => m.push(("exported", s.out())),
            None => {}
        }
        if let Some(x) = crate::house::ask::out(&self.ask) {
            m.push(("timeAsk", x));
        }
        if let Some(s) = self.said_here() {
            m.push(("said", s.out()));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}

#[cfg(test)]
mod said_and_fee_tests {
    use super::*;

    fn open(instance: &str) -> Open {
        Open { instance: instance.to_string(), ..Open::default() }
    }

    #[test]
    fn an_outcome_is_said_only_on_its_own_order() {
        let mut b = BOrders { open: Some(open("0xaa")), ..BOrders::default() };
        b.tell(Said::ok("refund-asked-on-aa".to_string()));
        assert!(b.said_here().is_some(), "said on the order it belongs to");
        assert!(String::from_utf8(eravon_literals::json::write(&b.rows())).unwrap_or_default().contains("refund-asked-on-aa"), "and answered there");
        b.open = Some(open("0xbb"));
        assert!(b.said_here().is_none(), "another order's page draws none of it");
        assert!(!String::from_utf8(eravon_literals::json::write(&b.rows())).unwrap_or_default().contains("refund-asked-on-aa"), "nor does the answer give it there");
        b.open = Some(open("0xaa"));
        b.leave();
        b.open = None;
        assert!(b.said.is_none() && b.said_here().is_none(), "leaving the order drops what was said on it");
        b.tell(Said::ok("the-list-own".to_string()));
        assert!(b.said_here().is_some(), "the list's own outcome stands on the list");
    }
}
