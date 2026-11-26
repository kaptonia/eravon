use std::fmt::{Debug, Write as _};
use std::hash::Hasher as _;

pub struct Pass<M> {
    pub from: Option<M>,
    pub got: M,
    pub read: bool,
}

impl<M> Pass<M> {
    pub(crate) fn new(from: Option<M>, got: M, read: bool) -> Pass<M> {
        Pass { from, got, read }
    }
}

impl<M: Clone> Clone for Pass<M> {
    fn clone(&self) -> Self {
        Pass { from: self.from.clone(), got: self.got.clone(), read: self.read }
    }
}

struct Digest(std::collections::hash_map::DefaultHasher);

impl std::fmt::Write for Digest {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write(s.as_bytes());
        Ok(())
    }
}

fn digest<T: Debug + ?Sized>(x: &T) -> u64 {
    let mut d = Digest(std::collections::hash_map::DefaultHasher::new());
    let _ = write!(d, "{x:?}");
    d.0.finish()
}

pub(crate) fn same<T: Debug + ?Sized>(a: &T, b: &T) -> bool {
    digest(a) == digest(b)
}

pub(crate) trait Lands: Sized + Debug {
    fn land(live: &mut Self, from: &Self, got: Self, read: bool);
    fn fresh(live: &mut Self, got: Self, read: bool);
}

impl<T: Lands> Lands for Option<T> {
    fn land(live: &mut Self, from: &Self, got: Self, read: bool) {
        match (live.as_mut(), from, got) {
            (Some(l), Some(f), Some(g)) => T::land(l, f, g, read),
            (_, f, g) => {
                if !read && !same(f, &g) && same(&*live, f) {
                    *live = g;
                }
            }
        }
    }

    fn fresh(live: &mut Self, got: Self, read: bool) {
        match (live.as_mut(), got) {
            (Some(l), Some(g)) => T::fresh(l, g, read),
            (_, g) => {
                if !read {
                    *live = g;
                }
            }
        }
    }
}

pub(crate) fn land<M: Lands>(slot: &mut Option<M>, pass: Pass<M>) {
    let Pass { from, got, read } = pass;
    match (slot.as_mut(), from) {
        (Some(live), Some(from)) => M::land(live, &from, got, read),
        (Some(live), None) => M::fresh(live, got, read),
        (None, _) => *slot = Some(got),
    }
}

macro_rules! lands {
    ($T:ident { whole: [$($w:ident),*], data: [$($d:ident),*], page: [$($p:ident),*], keyed($key:expr): [$($g:ident),*], nested: [$($n:ident),*], said: [$($lead:ident $(, $more:ident)*)?] $(,)? }) => {
        impl crate::house::merge::Lands for $T {
            #[allow(unused_variables, unused_mut)]
            fn land(live: &mut Self, from: &Self, got: Self, read: bool) {
                use crate::house::merge::same;
                if false $(|| !same(&from.$w, &got.$w))* {
                    *live = got;
                    return;
                }
                let key = $key;
                let held = same(&key(&*live), &key(from));
                let $T { $($w: _,)* $($d,)* $($lead, $($more,)*)? $($p,)* $($g,)* $($n,)* } = got;
                $( if !same(&from.$d, &$d) { live.$d = $d; } )*
                $( if !read || $lead.is_some() { live.$lead = $lead; $( live.$more = $more; )* } )?
                $( if held && !same(&from.$g, &$g) { live.$g = $g; } )*
                $( crate::house::merge::Lands::land(&mut live.$n, &from.$n, $n, read); )*
                $( if !read && !same(&from.$p, &$p) && same(&live.$p, &from.$p) { live.$p = $p; } )*
            }

            #[allow(unused_variables, unused_mut)]
            fn fresh(live: &mut Self, got: Self, read: bool) {
                if !read || (false $(|| !crate::house::merge::same(&live.$w, &got.$w))*) {
                    *live = got;
                    return;
                }
                let $T { $($w: _,)* $($d,)* $($lead, $($more,)*)? $($p: _,)* $($g: _,)* $($n: _,)* } = got;
                $( live.$d = $d; )*
                $( if $lead.is_some() { live.$lead = $lead; $( live.$more = $more; )* } )?
            }
        }
    };
}

pub(crate) use lands;

#[cfg(test)]
mod tests {
    use super::{land, Pass};
    use crate::house::seller::Said;

    fn ran<M: Clone>(slot: &Option<M>, f: impl FnOnce(&mut M), read: bool) -> Pass<M> {
        let from = slot.clone().expect("a model");
        let mut got = from.clone();
        f(&mut got);
        Pass::new(Some(from), got, read)
    }

    #[test]
    fn an_orders_read_lands_its_readings_and_leaves_the_list_tab_and_a_row_opened_meanwhile() {
        let mut slot = Some(crate::house::orders::Orders { me: "0xa".into(), sel: Some("i1".into()), offer: Some((1, 1)), ..Default::default() });
        let pass = ran(&slot, |o| {
            o.foreign = 7;
            o.offer = Some((2, 2));
            o.done_tab = true;
        }, true);
        if let Some(o) = slot.as_mut() {
            o.sel = Some("i2".into());
        }
        land(&mut slot, pass);
        let o = slot.expect("kept");
        assert_eq!((o.foreign, o.sel.as_deref(), o.offer, o.done_tab), (7, Some("i2"), Some((1, 1)), false));
    }

    #[test]
    fn an_orders_read_refreshes_the_open_order_while_it_stays_open() {
        let mut slot = Some(crate::house::orders::Orders { me: "0xa".into(), sel: Some("i1".into()), offer: Some((1, 1)), ..Default::default() });
        let pass = ran(&slot, |o| o.offer = Some((2, 2)), true);
        if let Some(o) = slot.as_mut() {
            o.done_tab = true;
        }
        land(&mut slot, pass);
        let o = slot.expect("kept");
        assert_eq!((o.offer, o.done_tab), (Some((2, 2)), true));
    }

    #[test]
    fn an_orders_action_opens_its_row_and_its_sheet_unless_the_person_moved_them_meanwhile() {
        let mut slot = Some(crate::house::orders::Orders { me: "0xa".into(), ..Default::default() });
        let pass = ran(&slot, |o| {
            o.sel = Some("i3".into());
            o.offer = Some((3, 3));
            o.paste_open = true;
        }, false);
        land(&mut slot, pass.clone());
        let o = slot.clone().expect("kept");
        assert_eq!((o.sel.as_deref(), o.offer, o.paste_open), (Some("i3"), Some((3, 3)), true));
        let mut slot = Some(crate::house::orders::Orders { me: "0xa".into(), ..Default::default() });
        if let Some(o) = slot.as_mut() {
            o.paste = "typed".into();
            o.sel = Some("i9".into());
        }
        land(&mut slot, pass);
        let o = slot.expect("kept");
        assert_eq!((o.sel.as_deref(), o.offer, o.paste.as_str()), (Some("i9"), None, "typed"));
    }

    #[test]
    fn an_action_lands_its_answer_even_none_and_a_read_lands_only_its_own() {
        let old = Some(Said::bad("E_STORE_OTHER_MACHINE", "old"));
        let mut slot = Some(crate::house::orders::Orders { me: "0xa".into(), said: old.clone(), ..Default::default() });
        let read = ran(&slot, |o| o.said = None, true);
        land(&mut slot, read);
        assert_eq!(slot.as_ref().and_then(|o| o.said.clone()), old, "a read with nothing to say leaves the line");
        let act = ran(&slot, |o| o.said = None, false);
        land(&mut slot, act);
        assert_eq!(slot.as_ref().and_then(|o| o.said.clone()), None, "an action with nothing to say takes the old line away");
        let late = ran(&slot, |o| o.foreign = 2, true);
        let act = ran(&slot, |o| o.said = Some(Said::bad("E_V_PRECHECK", "fee")), false);
        land(&mut slot, act);
        land(&mut slot, late);
        assert_eq!(slot.as_ref().and_then(|o| o.said.as_ref()).map(|s| s.name.as_str()), Some("E_V_PRECHECK"), "a read that started before the action does not cover its answer");
        let own = ran(&slot, |o| o.said = Some(Said::bad("E_CHAIN_TRANSPORT", "down")), true);
        land(&mut slot, own);
        assert_eq!(slot.as_ref().and_then(|o| o.said.as_ref()).map(|s| s.name.as_str()), Some("E_CHAIN_TRANSPORT"), "a read lands its own words");
    }

    #[test]
    fn a_buyer_orders_answer_lands_with_the_order_it_was_said_on() {
        let mut slot = Some(crate::house::buyer_orders::BOrders { me: "0xa".into(), said: Some(Said::bad("E_X", "x")), said_on: Some("i1".into()), ..Default::default() });
        let read = ran(&slot, |b| {
            b.said = None;
            b.said_on = None;
        }, true);
        land(&mut slot, read);
        assert_eq!(slot.as_ref().and_then(|b| b.said_on.clone()).as_deref(), Some("i1"), "a read with nothing to say keeps both");
        let act = ran(&slot, |b| {
            b.said = None;
            b.said_on = None;
        }, false);
        land(&mut slot, act);
        assert!(slot.as_ref().is_some_and(|b| b.said.is_none() && b.said_on.is_none()), "an action's none takes both");
    }

    #[test]
    fn a_pass_for_another_key_replaces_the_page_whole() {
        let mut slot = Some(crate::house::orders::Orders { me: "0xa".into(), sel: Some("i1".into()), ..Default::default() });
        let pass = ran(&slot, |o| {
            *o = crate::house::orders::Orders { me: "0xb".into(), ..Default::default() };
        }, true);
        land(&mut slot, pass);
        let o = slot.expect("kept");
        assert_eq!((o.me.as_str(), o.sel.as_deref()), ("0xb", None));
    }

    #[test]
    fn a_goods_pass_leaves_the_search_typed_meanwhile_and_lands_its_rows() {
        let goods = crate::house::seller::Goods { rows: Ok(Vec::new()), names: Vec::new(), query: String::new(), stages: Vec::new(), shown: None, edit: None, said: None, swept: Vec::new(), ask: None, me: "0xa".into() };
        let mut slot = Some(goods);
        let pass = ran(&slot, |g| g.rows = Err(Said::bad("E_X", "unread")), false);
        if let Some(g) = slot.as_mut() {
            g.query = "lamp".into();
        }
        land(&mut slot, pass);
        let g = slot.expect("kept");
        assert_eq!((g.query.as_str(), g.rows.is_err()), ("lamp", true));
    }

    #[test]
    fn a_candidates_read_leaves_the_detail_tab_and_lands_the_table() {
        let cands = crate::house::seller::Cands { tab: eravon_cands::table::Side::Judge, rows: Ok(Vec::new()), detail: None, dtab: 0, said: None, me: "0xa".into(), codes: Vec::new(), lapse: None, unread: false };
        let mut slot = Some(cands);
        let pass = ran(&slot, |c| {
            c.rows = Err(Said::bad("E_X", "unread"));
            c.dtab = 0;
        }, true);
        if let Some(c) = slot.as_mut() {
            c.dtab = 2;
        }
        land(&mut slot, pass);
        let c = slot.expect("kept");
        assert_eq!((c.dtab, c.rows.is_err()), (2, true));
    }

    #[test]
    fn a_wallet_read_leaves_the_open_transaction_and_its_send_closes_the_sheet_it_was_sent_from() {
        let mut slot = Some(crate::house::wallet::Wallet { me: "0xa".into(), sheet: true, ..Default::default() });
        let read = ran(&slot, |w| {
            w.filled = 3;
            w.open_tx = None;
        }, true);
        if let Some(w) = slot.as_mut() {
            w.open_tx = Some("0xt".into());
        }
        land(&mut slot, read);
        let sent = ran(&slot, |w| w.sheet = false, false);
        land(&mut slot, sent);
        let w = slot.expect("kept");
        assert_eq!((w.filled, w.open_tx.as_deref(), w.sheet), (3, Some("0xt"), false));
    }

    #[test]
    fn a_buyer_orders_read_keeps_the_open_orders_tab_and_refreshes_its_readings() {
        let open = crate::house::buyer_orders::Open { instance: "i1".into(), ..Default::default() };
        let mut slot = Some(crate::house::buyer_orders::BOrders { me: "0xa".into(), open: Some(open), ..Default::default() });
        let pass = ran(&slot, |b| {
            b.owners = vec!["0xo".into()];
            if let Some(o) = b.open.as_mut() {
                o.judge_open = true;
                o.kits = Some(Ok(Vec::new()));
            }
        }, true);
        if let Some(b) = slot.as_mut() {
            b.q = "lamp".into();
        }
        land(&mut slot, pass);
        let b = slot.expect("kept");
        let o = b.open.expect("open");
        assert_eq!((b.owners.len(), b.q.as_str(), o.judge_open, o.kits.is_some()), (1, "lamp", false, true));
    }

    #[test]
    fn a_buyer_orders_read_lands_nothing_on_an_order_opened_meanwhile() {
        let open = crate::house::buyer_orders::Open { instance: "i1".into(), ..Default::default() };
        let mut slot = Some(crate::house::buyer_orders::BOrders { me: "0xa".into(), open: Some(open), ..Default::default() });
        let pass = ran(&slot, |b| {
            if let Some(o) = b.open.as_mut() {
                o.kits = Some(Ok(Vec::new()));
            }
        }, true);
        if let Some(b) = slot.as_mut() {
            b.open = Some(crate::house::buyer_orders::Open { instance: "i2".into(), ..Default::default() });
        }
        land(&mut slot, pass);
        let o = slot.expect("kept").open.expect("open");
        assert_eq!((o.instance.as_str(), o.kits.is_none()), ("i2", true));
    }

    #[test]
    fn a_payment_link_pass_leaves_the_refund_address_typed_meanwhile() {
        let mut slot = Some(crate::house::paylink::PayLink { owner: "0xa".into(), ..Default::default() });
        let pass = ran(&slot, |p| p.company_on = true, false);
        if let Some(p) = slot.as_mut() {
            p.payout = "0xr".into();
        }
        land(&mut slot, pass);
        let p = slot.expect("kept");
        assert_eq!((p.company_on, p.payout.as_str()), (true, "0xr"));
    }

    #[test]
    fn a_settings_read_leaves_the_section_and_lands_what_it_read() {
        let data = std::env::temp_dir().join(format!("eravon-merge-{}", std::process::id())).display().to_string();
        let st = crate::house::settings::Settings::open(&data, crate::house::shell::Seat::Seller, &crate::house::settings::Env { home: String::new() });
        let mut slot = Some(st);
        let pass = ran(&slot, |s| s.gas_asked = !s.gas_asked, true);
        let asked = !slot.as_ref().is_some_and(|s| s.gas_asked);
        if let Some(s) = slot.as_mut() {
            s.section = Some(crate::house::settings::Section::Look);
        }
        land(&mut slot, pass);
        let s = slot.expect("kept");
        assert_eq!((s.gas_asked, s.section), (asked, Some(crate::house::settings::Section::Look)));
    }

    #[test]
    fn an_export_read_leaves_the_search_and_the_choice_and_lands_the_picks() {
        let mut slot = Some(crate::house::export::Export { me: "0xa".into(), ..Default::default() });
        let pass = ran(&slot, |e| e.owners = vec!["0xo".into()], true);
        if let Some(e) = slot.as_mut() {
            e.q = "lamp".into();
            e.sel = Some("i1".into());
        }
        land(&mut slot, pass);
        let e = slot.expect("kept");
        assert_eq!((e.owners.len(), e.q.as_str(), e.sel.as_deref()), (1, "lamp", Some("i1")));
    }

    #[test]
    fn an_export_read_lands_the_remembered_folder_on_a_page_another_pass_made_first() {
        let mut slot = Some(crate::house::export::Export { me: "0xa".into(), ..Default::default() });
        let pass = ran(&slot, |e| e.dir = "/saved".into(), true);
        land(&mut slot, pass);
        assert_eq!(slot.as_ref().map(|e| e.dir.as_str()), Some("/saved"));
        let mut slot = Some(crate::house::export::Export { me: "0xa".into(), ..Default::default() });
        let got = crate::house::export::Export { me: "0xa".into(), dir: "/saved".into(), ..Default::default() };
        land(&mut slot, Pass::new(None, got, true));
        assert_eq!(slot.map(|e| e.dir), Some("/saved".to_string()));
    }

    #[test]
    fn a_buyer_home_read_lands_its_readings() {
        let mut slot = Some(crate::house::buyer_home::BHome { me: "0xa".into(), ..Default::default() });
        let pass = ran(&slot, |h| h.badge = 4, true);
        land(&mut slot, pass);
        assert_eq!(slot.expect("kept").badge, 4);
    }

    fn goods_row(id: &str) -> eravon_goods::goods::Row {
        eravon_goods::goods::Row { id: id.into(), name: "lamp".into(), description: String::new(), price: "1".into(), token: String::new(), draft: false, cands: Vec::new(), version: 1, updated: 0, listed: true, pending: Vec::new(), series: None, chain: None }
    }

    #[test]
    fn the_goods_list_in_hand_is_drawn_the_moment_the_page_is_entered() {
        let goods = crate::house::seller::Goods { rows: Ok(vec![goods_row("p1")]), names: Vec::new(), query: "lamp".into(), stages: Vec::new(), shown: None, edit: None, said: None, swept: Vec::new(), ask: None, me: "0xa".into() };
        let mut slot = Some(goods);
        crate::house::seller::Goods::entered(&mut slot, "0xA");
        let g = slot.expect("kept on entering");
        assert_eq!((g.rows.as_ref().map(Vec::len).ok(), g.query.as_str(), g.edit.is_none()), (Some(1), "lamp", true));
        let mut other = Some(g);
        crate::house::seller::Goods::entered(&mut other, "0xb");
        assert!(other.is_none(), "another key's list is not drawn");
    }

    #[test]
    fn the_candidates_in_hand_are_drawn_the_moment_the_page_is_entered() {
        let row = eravon_store::record::Candidate { kind: "judge".into(), address: "0xj".into(), label: "甲".into(), added_at: 0, seen: String::new() };
        let brief = eravon_cands::view::Brief { row, state: Err(eravon_cands::fault::Absent::NoRegistry), warn: None, flaw: None };
        let cands = crate::house::seller::Cands { tab: eravon_cands::table::Side::Guild, rows: Ok(vec![brief]), detail: None, dtab: 2, said: None, me: "0xa".into(), codes: Vec::new(), lapse: Some("0xt".into()), unread: false };
        let mut slot = Some(cands);
        crate::house::seller::Cands::entered(&mut slot, "0xa");
        let c = slot.expect("kept on entering");
        assert_eq!((c.rows.as_ref().map(Vec::len).ok(), c.tab, c.lapse.is_none()), (Some(1), eravon_cands::table::Side::Guild, true));
    }

    #[test]
    fn a_read_that_started_on_no_page_lands_its_readings_beside_a_detail_opened_meanwhile() {
        let opened = crate::house::orders::Orders { me: "0xa".into(), sel: Some("i1".into()), ..Default::default() };
        let mut slot = Some(opened);
        let got = crate::house::orders::Orders { me: "0xa".into(), foreign: 5, ..Default::default() };
        land(&mut slot, Pass::new(None, got, true));
        let o = slot.expect("kept");
        assert_eq!((o.foreign, o.sel.as_deref()), (5, Some("i1")));
    }
}
