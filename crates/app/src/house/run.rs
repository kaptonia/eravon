use std::sync::Arc;

use crate::house::buyer_orders::BOrders;
use crate::house::desk::Site;
use crate::house::flow::{pass_kind, Ask};
use crate::house::merge::Pass;
use crate::house::export::Export;
use crate::house::landed::Landed;
use crate::house::seller::{Said, Tone};
use crate::house::shell::{Seat, Toast};
use crate::house::wallet::Wallet;
use crate::house::Ticket;
use crate::shell::Shell;
use crate::task::Kind;

impl Shell {
    pub(crate) fn spawn_home(&mut self) {
        if self.desk.seat() != Seat::Seller || self.desk.addr.is_empty() || self.desk.data.is_empty() || self.house_in_flight(Kind::Home) {
            return;
        }
        let (data, addr) = (self.desk.data.clone(), self.desk.addr.clone());
        let shown = self.desk.shown();
        let from = self.desk.sh.seller.home.clone();
        self.house_spawn(Kind::Home, Site::Page, false, move |_| Landed::Home(Box::new(Pass::new(from, crate::house::refresh::draw(&shown, || crate::house::home::Home::open(&data, &addr)), true))));
    }

    pub(crate) fn spawn_cands(&mut self, site: Site, ask: impl Into<Ask>, f: impl FnOnce(Option<crate::house::seller::Cands>) -> crate::house::seller::Cands + Send + 'static) -> bool {
        let ask = ask.into();
        let act = ask.writes();
        let read = ask == Ask::Read;
        let mut now = self.desk.sh.seller.cands.clone();
        if let Some(c) = now.as_mut() {
            c.said = None;
        }
        let from = now.clone();
        let go = self.house_spawn(Kind::Cands, site, ask, move |t| {
            let mut c = f(now);
            if act && !t.live() && c.said.as_ref().is_some_and(|s| s.tone != Tone::Ok) {
                c.said = Some(crate::house::stale());
            }
            Landed::Cands(Box::new(Pass::new(from, c, read)))
        });
        if !go {
            self.desk.retry_next = None;
            return false;
        }
        self.desk.launched(pass_kind(Kind::Cands, ask));
        true
    }

    pub(crate) fn goods_key(&self) -> String {
        self.desk.sh.seller.goods.as_ref().map(|g| g.me.clone()).filter(|m| !m.is_empty()).unwrap_or_else(|| self.desk.addr.clone())
    }

    pub(crate) fn orders_key(&self) -> String {
        self.desk.sh.seller.orders.as_ref().map(|o| o.me.clone()).filter(|m| !m.is_empty()).unwrap_or_else(|| self.desk.addr.clone())
    }

    pub(crate) fn cands_key(&self) -> String {
        self.desk.sh.seller.cands.as_ref().map(|c| c.me.clone()).filter(|m| !m.is_empty()).unwrap_or_else(|| self.desk.addr.clone())
    }

    pub(crate) fn spawn_goods(&mut self, f: impl FnOnce(&mut crate::house::seller::Goods, &str, &str, &crate::house::seller::Field, &eravon_goods::sign::Signer) + Send + 'static) {
        self.spawn_goods_at(Site::Page, f);
    }

    pub(crate) fn spawn_goods_at(&mut self, site: Site, f: impl FnOnce(&mut crate::house::seller::Goods, &str, &str, &crate::house::seller::Field, &eravon_goods::sign::Signer) + Send + 'static) -> bool {
        let Some(mut gd) = self.desk.sh.seller.goods.clone() else { return false };
        gd.said = None;
        let from = Some(gd.clone());
        let (data, addr, field) = (self.desk.data.clone(), self.goods_key(), self.desk.sh.seller.field.clone());
        let signer = eravon_goods::sign::Signer::of(Some(&self.desk.session), &addr);
        let go = self.house_spawn(Kind::Goods, site, true, move |t| {
            let t = *t;
            let signer = signer.bound(Arc::new(move || t.live()));
            f(&mut gd, &data, &addr, &field, &signer);
            Landed::Goods(Box::new(Pass::new(from, gd, false)))
        });
        if !go {
            self.desk.retry_next = None;
            self.desk.busy_said();
            return false;
        }
        self.desk.launched(Kind::Goods);
        true
    }

    pub(crate) fn goods_taken(&mut self) {
        let wants = self.desk.sh.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).is_some_and(|s| s.wants_taken() && s.taken.is_none());
        if !wants {
            return;
        }
        let fresh = self.desk.sh.seller.orders.as_ref().is_some_and(|o| o.fresh_for(&self.desk.addr) && o.table.is_some());
        if fresh {
            let rows = self.desk.sh.seller.orders.as_ref().and_then(|o| o.complete_table());
            if let Some(s) = self.desk.sh.seller.goods.as_mut().and_then(|g| g.shown.as_mut()) {
                s.fill_taken(rows.as_ref());
            }
        } else if !self.house_in_flight(crate::task::Kind::Orders) {
            let (data, addr) = (self.desk.data.clone(), self.desk.addr.clone());
            self.spawn_orders(crate::house::desk::Site::Page, false, move |cur, _| {
                if cur.fresh_for(&addr) && !cur.me.is_empty() {
                    let mut o = cur;
                    o.list(&data, &addr);
                    o
                } else {
                    crate::house::orders::Orders::table_only(&data, &addr)
                }
            });
        }
    }

    pub(crate) fn spawn_orders(&mut self, site: Site, ask: impl Into<Ask>, f: impl FnOnce(crate::house::orders::Orders, &Ticket) -> crate::house::orders::Orders + Send + 'static) -> bool {
        let ask = ask.into();
        let read = ask == Ask::Read;
        let had = self.desk.sh.seller.orders.is_some();
        let mut now = self.desk.sh.seller.orders.clone().unwrap_or_default();
        now.said = None;
        let from = had.then(|| now.clone());
        let data = self.desk.data.clone();
        let go = self.house_spawn(Kind::Orders, site, ask, move |t| {
            now.ticket = *t;
            let mut o = f(now, t);
            o.company = crate::house::orders::company_mode(&data, &o.me).is_some();
            Landed::Orders(Box::new(Pass::new(from, o, read)))
        });
        if !go {
            self.desk.retry_next = None;
            return false;
        }
        self.desk.launched(pass_kind(Kind::Orders, ask));
        true
    }

    pub(crate) fn orders_load(&mut self) {
        self.orders_load_as(false);
    }

    pub(crate) fn orders_load_as(&mut self, reread: bool) {
        if self.desk.seat() != Seat::Seller || self.desk.addr.is_empty() || self.desk.data.is_empty() || (self.covered() && self.unlocked()) {
            return;
        }
        let (data, addr) = (self.desk.data.clone(), self.desk.addr.clone());
        let shown = self.desk.shown();
        self.spawn_orders(Site::Page, false, move |cur, _t| {
            let mut o = if cur.fresh_for(&addr) { cur } else { crate::house::orders::Orders { me: addr.clone(), ..Default::default() } };
            let before = o.open_follow();
            crate::house::refresh::draw(&shown, || o.list_table(&data, &addr));
            let (whole, shape) = (o.read_whole(), o.open_follow());
            if o.follow.moved(whole, before, shape) && reread {
                o.reread_detail(&data, &addr);
            } else {
                o.readvance(&data, &addr);
            }
            o
        });
    }

    pub(crate) fn orders_open_row(&mut self, i: String) -> bool {
        let (data, addr, now) = (self.desk.data.clone(), self.desk.addr.clone(), self.desk.now());
        let shown = self.desk.shown();
        let go = self.spawn_orders(Site::Page, Ask::Asked, move |cur, _t| {
            let mut o = if cur.fresh_for(&addr) { cur } else { crate::house::orders::Orders { me: addr.clone(), ..Default::default() } };
            if o.table.is_none() {
                crate::house::refresh::draw(&shown, || o.list_table(&data, &addr));
            }
            o.open_row(&data, &addr, now, &i);
            o
        });
        go
    }

    pub(crate) fn orders_backstop(&mut self) {
        if self.desk.seat() != Seat::Seller || self.desk.addr.is_empty() || self.desk.data.is_empty() || (self.covered() && self.unlocked()) {
            return;
        }
        let (data, addr) = (self.desk.data.clone(), self.desk.addr.clone());
        let key = crate::house::orders::Key::of(&self.desk.session, &self.desk.addr);
        let shown = self.desk.shown();
        let now = self.desk.now();
        self.spawn_orders(Site::Page, false, move |cur, t| {
            let key = key.bound(t);
            let mut o = crate::house::orders::Orders::backstop(&data, &addr, now, &key, &shown);
            if let Some(id) = cur.order.clone().filter(|_| cur.fresh_for(&addr)).map(|x| x.id) {
                o.open_one(&data, &addr, now, &id);
            }
            o
        });
    }

    pub(crate) fn orders_catch_up(&mut self) {
        if self.desk.seat() != Seat::Seller || self.desk.addr.is_empty() || self.desk.data.is_empty() || !self.unlocked() {
            return;
        }
        let (data, addr, now) = (self.desk.data.clone(), self.desk.addr.clone(), self.desk.now());
        let key = crate::house::orders::Key::of(&self.desk.session, &self.desk.addr);
        let shown = self.desk.shown();
        let round = self.desk.sh.seller.page() == Some(crate::house::seller::Page::Orders) && self.desk.sh.seller.orders.as_ref().is_some_and(|o| o.fresh_for(&addr) && o.order.is_some());
        self.spawn_orders(Site::Page, true, move |cur, t| {
            let key = key.bound(t);
            if !cur.fresh_for(&addr) {
                return crate::house::orders::Orders::backstop(&data, &addr, now, &key, &shown);
            }
            let mut o = cur;
            o.scan(&data, &addr, now, &key);
            if round {
                o.poll(&data, &addr, now, &key);
            }
            o
        });
        self.desk.tr.push(crate::house::action::Action::OrderScan, eravon_orders::mark::FEATURE, if round { "同步 · 补扫与本单一轮" } else { "同步 · 补扫" }.into());
    }

    pub(crate) fn orders_said(&mut self, s: Said) {
        let t = Toast { tone: s.tone, head: s.detail.clone(), next: String::new(), detail: (s.tone == Tone::Bad).then(|| s.name.clone()) };
        if let Some(o) = self.desk.sh.seller.orders.as_mut() {
            o.said = Some(s);
        }
        self.desk.sh.toast(t);
    }

    pub(crate) fn spawn_paylink_open(&mut self) {
        let Some(mut pl) = self.desk.sh.paylink.clone() else { return };
        if let Err(e) = crate::house::asked(crate::house::paylink::open_plan(&pl.link)) {
            if let Some(x) = self.desk.sh.paylink.as_mut() {
                x.wants_open = false;
                x.opened = Some(Err(e));
                x.opened_for = x.link.trim().to_string();
            }
            return;
        }
        if let Some(x) = self.desk.sh.paylink.as_mut() {
            x.wants_open = false;
        }
        self.desk.pl_payout = None;
        let (data, registries, now) = (self.desk.data.clone(), self.desk.sh.seller.field.registries.clone(), self.desk.now());
        let buyer = crate::house::shell::current_key(&data, Seat::Buyer).unwrap_or_default();
        let from = Some(pl.clone());
        if !self.house_spawn(Kind::PayLink, Site::Page, Ask::Asked, move |_| {
            let text = pl.link.clone();
            pl.open_link(&text, &buyer, &data, &registries, now);
            pl.company_on = crate::house::orders::company_mode(&data, &buyer).is_some();
            Landed::PayLink(Box::new(Pass::new(from, pl, false)))
        }) {
            self.desk.busy_said();
        }
    }

    pub(crate) fn spawn_pl_act(&mut self, site: Site, f: impl FnOnce(&mut crate::house::paylink::PayLink, &eravon_vault::session::Session, &str, &Ticket) + Send + 'static) -> bool {
        let Some(mut pl) = self.desk.sh.paylink.clone() else { return false };
        if self.house_in_flight(Kind::PayLink) {
            self.desk.retry_next = None;
            self.desk.busy_said();
            return false;
        }
        let fork = match self.desk.session.fork() {
            Ok(s) => s,
            Err(e) => {
                self.desk.retry_next = None;
                self.pl_said(site, crate::house::said_of(&e));
                return false;
            }
        };
        let buyer = crate::house::shell::current_key(&self.desk.data, Seat::Buyer).unwrap_or_default();
        let data = self.desk.data.clone();
        let from = Some(pl.clone());
        let go = self.house_spawn(Kind::PayLink, site, true, move |t| {
            let mut s = fork;
            pl.ticket = *t;
            f(&mut pl, &s, &buyer, t);
            pl.company_on = crate::house::orders::company_mode(&data, &buyer).is_some();
            s.lock();
            Landed::PayLink(Box::new(Pass::new(from, pl, false)))
        });
        if go {
            self.desk.launched(Kind::PayLink);
        }
        self.desk.retry_next = None;
        go
    }

    pub(crate) fn pl_said(&mut self, site: Site, s: Said) {
        self.desk.sh.toast(Toast { tone: Tone::Bad, head: s.detail.clone(), next: crate::house::sync::next_of(&s), detail: Some(s.name.clone()) });
        let Some(x) = self.desk.sh.paylink.as_mut() else { return };
        match site.name() {
            "pl-co" => x.co_open = Some(Err(s)),
            "pl-import" => x.import.said = Some(s),
            "pl-company" => x.company = Some(Err(s)),
            _ => x.paid = Some(Err(s)),
        }
    }

    pub(crate) fn spawn_bo_at(&mut self, site: Site, ask: impl Into<Ask>, f: impl FnOnce(&eravon_vault::session::Session, &str, &str, BOrders) -> BOrders + Send + 'static) -> bool {
        let ask = ask.into();
        let act = ask.writes();
        let read = ask == Ask::Read;
        if self.house_in_flight(pass_kind(Kind::Bo, ask)) {
            self.busy(ask);
            return false;
        }
        let had = self.desk.sh.buyer_orders.is_some();
        let mut cur = self.desk.sh.buyer_orders.clone().unwrap_or_default();
        cur.said = None;
        cur.said_on = None;
        let fork = match self.desk.session.fork() {
            Ok(s) => s,
            Err(e) => {
                let mut b = cur;
                b.tell(crate::house::said_of(&e));
                self.refused(ask, &crate::house::said_of(&e));
                if b.rows.is_none() {
                    b.rows = Some(Err(crate::house::said_of(&e)));
                }
                b.me = self.desk.addr.to_ascii_lowercase();
                self.desk.sh.buyer_orders = Some(b);
                self.desk.retry_next = None;
                return false;
            }
        };
        let data = self.desk.data.clone();
        let me = match act && !cur.me.is_empty() {
            true => cur.me.clone(),
            false => crate::house::shell::current_key(&data, Seat::Buyer).unwrap_or_default(),
        };
        let from = had.then(|| cur.clone());
        let go = self.house_spawn(Kind::Bo, site, ask, move |t| {
            let mut s = fork;
            cur.ticket = *t;
            let mut b = f(&s, &data, &me, cur);
            s.lock();
            if b.said.is_some() && b.said_on.is_none() {
                b.said_on = b.open.as_ref().map(|o| o.instance.to_ascii_lowercase());
            }
            Landed::Bo(Box::new(Pass::new(from, b, read)))
        });
        if go {
            self.desk.launched(pass_kind(Kind::Bo, ask));
        }
        self.desk.retry_next = None;
        go
    }

    pub(crate) fn load_bo(&mut self) {
        let shown = self.desk.shown();
        self.spawn_bo_at(Site::Page, false, move |s, data, me, cur| {
            let mut b = crate::house::refresh::draw(&shown, || BOrders::load(s, data, me));
            if cur.fresh_for(me) {
                b.open = cur.open;
                b.readvance(data, me);
            }
            b.draw_due(s, data, me);
            b
        });
    }

    pub(crate) fn spawn_bhome(&mut self) {
        if self.desk.seat() != Seat::Buyer || self.desk.data.is_empty() || self.house_in_flight(Kind::BHome) {
            return;
        }
        let fork = self.desk.session.fork().unwrap_or_else(|_| eravon_vault::session::Session::locked(&self.desk.data));
        let data = self.desk.data.clone();
        let me = crate::house::shell::current_key(&data, Seat::Buyer).unwrap_or_default();
        if !self.desk.seat_reads() {
            return;
        }
        let shown = self.desk.shown();
        let from = self.desk.sh.buyer_home.clone();
        self.house_spawn(Kind::BHome, Site::Page, false, move |_| {
            let mut s = fork;
            let h = crate::house::refresh::draw(&shown, || crate::house::buyer_home::BHome::open(&s, &data, &me));
            s.lock();
            Landed::BHome(Box::new(Pass::new(from, h, true)))
        });
    }

    pub(crate) fn spawn_wallet_at(&mut self, site: Site, ask: impl Into<Ask>, f: impl FnOnce(&eravon_vault::session::Session, &str, &str, Wallet, &Ticket) -> Wallet + Send + 'static) -> bool {
        let ask = ask.into();
        let read = ask == Ask::Read;
        if self.house_in_flight(pass_kind(Kind::Wallet, ask)) {
            self.busy(ask);
            return false;
        }
        let had = self.desk.sh.wallet.is_some();
        let mut cur = self.desk.sh.wallet.clone().unwrap_or_default();
        cur.said = None;
        let fork = match self.desk.session.fork() {
            Ok(s) => s,
            Err(e) => {
                let mut w = cur;
                w.me = crate::house::shell::current_key(&self.desk.data, self.desk.seat()).unwrap_or_default().to_ascii_lowercase();
                w.said = Some(crate::house::said_of(&e));
                self.refused(ask, &crate::house::said_of(&e));
                self.desk.sh.wallet = Some(w);
                self.desk.retry_next = None;
                return false;
            }
        };
        let data = self.desk.data.clone();
        let me = crate::house::shell::current_key(&data, self.desk.seat()).unwrap_or_default();
        let from = had.then(|| cur.clone());
        let go = self.house_spawn(Kind::Wallet, site, ask, move |t| {
            let mut s = fork;
            let w = f(&s, &data, &me, cur, t);
            s.lock();
            Landed::Wallet(Box::new(Pass::new(from, w, read)))
        });
        if go {
            self.desk.launched(pass_kind(Kind::Wallet, ask));
        }
        self.desk.retry_next = None;
        go
    }

    pub(crate) fn load_wallet(&mut self) {
        let shown = self.desk.shown();
        let seller = self.desk.seat() == Seat::Seller;
        self.spawn_wallet_at(Site::Page, false, move |s, data, me, _, _| {
            crate::house::refresh::draw(&shown, || {
                let mut w = Wallet::open(s, data, me, seller);
                if seller {
                    w.read_vault(data, me);
                }
                w
            })
        });
    }

    pub(crate) fn spawn_export_at(&mut self, site: Site, ask: impl Into<Ask>, f: impl FnOnce(&eravon_vault::session::Session, &str, &str, Export, &Ticket) -> Export + Send + 'static) -> bool {
        let ask = ask.into();
        let read = ask == Ask::Read;
        if self.house_in_flight(pass_kind(Kind::Export, ask)) {
            self.busy(ask);
            return false;
        }
        let buyer = self.desk.seat() == Seat::Buyer;
        let had = self.desk.sh.export.as_ref().is_some_and(|e| e.buyer == buyer);
        let mut cur = self.desk.sh.export.clone().filter(|e| e.buyer == buyer).unwrap_or(Export { buyer, ..Export::default() });
        cur.said = None;
        let fork = match self.desk.session.fork() {
            Ok(s) => s,
            Err(e) => {
                cur.said = Some(crate::house::said_of(&e));
                self.refused(ask, &crate::house::said_of(&e));
                self.desk.sh.export = Some(cur);
                self.desk.retry_next = None;
                return false;
            }
        };
        let data = self.desk.data.clone();
        let me = crate::house::shell::current_key(&data, self.desk.seat()).unwrap_or_default();
        let from = had.then(|| cur.clone());
        let go = self.house_spawn(Kind::Export, site, ask, move |t| {
            let mut s = fork;
            let e = f(&s, &data, &me, cur, t);
            s.lock();
            Landed::Export(Box::new(Pass::new(from, e, read)))
        });
        if go {
            self.desk.launched(pass_kind(Kind::Export, ask));
        }
        self.desk.retry_next = None;
        go
    }

    pub(crate) fn load_export(&mut self) {
        let pend = self.desk.pending_export.take();
        let buyer = self.desk.seat() == Seat::Buyer;
        let keep_tab = self.desk.sh.export.as_ref().map(|e| e.tab).unwrap_or_default();
        let shown = self.desk.shown();
        let ask = if pend.is_some() { Ask::Asked } else { Ask::Read };
        self.spawn_export_at(Site::Page, ask, move |s, data, me, cur, _| {
            let mut e = crate::house::refresh::draw(&shown, || Export::open(s, data, me, buyer));
            e.tab = keep_tab;
            if let Some(i) = pend.or(cur.sel) {
                if let Err(x) = e.choose(&i) {
                    e.said = Some(x);
                }
            }
            e
        });
    }

    pub(crate) fn open_settings(&mut self) {
        let env = crate::house::settings::Env { home: self.desk.sh.seller.field.home.clone() };
        let mut st = crate::house::settings::Settings::open(&self.desk.data, self.desk.seat(), &env);
        st.from_book(self.desk.sh.book.as_ref());
        self.desk.sh.seller.settings = Some(st);
        self.desk.set_card = None;
    }

    pub(crate) fn spawn_settings(&mut self, step: crate::house::action::Action, ask: impl Into<Ask>, f: impl FnOnce(&mut crate::house::settings::Settings, &str) + Send + 'static) {
        if self.held(step) {
            return;
        }
        let Some(mut s) = self.desk.sh.seller.settings.clone() else { return };
        s.said = None;
        let ask = ask.into();
        let read = ask == Ask::Read;
        let from = Some(s.clone());
        let data = self.desk.data.clone();
        if !self.house_spawn(Kind::Settings, Site::Sheet("settings"), ask, move |t| {
            s.ticket = *t;
            f(&mut s, &data);
            if !t.live() && s.said.as_ref().is_some_and(|x| x.tone != Tone::Ok) {
                s.said = Some(crate::house::stale());
            }
            Landed::Settings(Box::new(Pass::new(from, s, read)))
        }) {
            return;
        }
        self.desk.tr.push(step, eravon_literals::trace::Feature::S9, String::new());
        self.desk.flush();
    }

    pub(crate) fn spawn_settings_keyed(&mut self, step: crate::house::action::Action, f: impl FnOnce(&mut crate::house::settings::Settings, &eravon_vault::session::Session, &str) + Send + 'static) {
        if self.held(step) {
            return;
        }
        let Some(mut s) = self.desk.sh.seller.settings.clone() else { return };
        let fork = match self.desk.session.fork() {
            Ok(x) => x,
            Err(e) => {
                let said = crate::house::said_of(&e);
                self.desk.sh.toast(crate::house::shell::Toast { tone: said.tone, head: said.detail.clone(), next: String::new(), detail: Some(said.name) });
                return;
            }
        };
        s.said = None;
        let from = Some(s.clone());
        let data = self.desk.data.clone();
        if !self.house_spawn(Kind::Settings, Site::Sheet("settings"), true, move |t| {
            let mut ses = fork;
            s.ticket = *t;
            f(&mut s, &ses, &data);
            ses.lock();
            Landed::Settings(Box::new(Pass::new(from, s, false)))
        }) {
            return;
        }
        self.desk.tr.push(step, eravon_literals::trace::Feature::S9, String::new());
        self.desk.flush();
    }

    pub(crate) fn held(&mut self, act: crate::house::action::Action) -> bool {
        let Some(h) = crate::house::action::held_back(act, !self.unlocked()) else { return false };
        let (name, t) = h.said(crate::lang::lang());
        self.desk.sh.toast(t);
        self.desk.tr.push(act, eravon_literals::trace::Feature::H1, format!("{} · {name}", act.name()));
        self.desk.flush();
        true
    }
}
