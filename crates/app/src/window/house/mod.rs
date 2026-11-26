use super::*;
use crate::house::act::Act;
use crate::house::action::Action as Step;
use crate::house::seller::Page as HPage;
use crate::house::shell::Seat;
pub(super) use crate::house::shell::Toast;

mod widgets;
pub(in crate::window) mod said;
use widgets::*;
mod home;
use home::*;
mod orders;
use orders::*;
mod order_cards;
mod complaint;
mod judge;
mod casepages;
mod goods;
mod cands;
mod payways;
mod seller_wallet;
mod buyer_home;
mod export;
mod queue;
mod buyer_orders;
mod paylink;
mod history;
pub(in crate::window) mod first_run;
mod consent;
mod settings;
pub(crate) mod windrive;
pub(crate) mod pageframes;
mod pages;

#[derive(Default)]
pub(super) struct Hx {
    pub(super) every: fields::SavedEdit,
    pub(super) hush: eravon_ui::hush::Hush,
    pub(super) toast_fed: Option<(String, String, u64)>,
    pub(super) page_at: (String, f64),
    pub(super) place_open: Option<&'static str>,
    pub(super) places: std::collections::BTreeMap<&'static str, String>,
    pub(super) home_seen: String,
    pub(super) goods_open: Option<String>,
    pub(super) published: Option<Option<eravon_goods::goods::Saved>>,
    pub(super) route_was: Option<Route>,
    pub(super) rail_landing: bool,
    pub(super) inner_open: Vec<(Route, Step, Vec<(&'static str, String)>)>,
    pub(super) entering: Option<Route>,
    pub(super) reopen: Option<Route>,
    pub(super) reopen_tried: Option<Route>,
    pub(super) clipboard: Option<arboard::Clipboard>,
}

pub(super) enum Placed {
    Go(String),
    Close,
    Open,
}

#[derive(Default)]
pub(super) struct Keys {
    pub(super) cancel: String,
    pub(super) go: Vec<(String, Role, bool)>,
    pub(super) pick: Option<(String, crate::platform::Pick)>,
    pub(super) picked: String,
    pub(super) left: Vec<(String, Role, bool)>,
}

impl Win {
    pub(super) fn house(&mut self, a: Act) -> crate::house::act::Said {
        if a.step == Step::PePublish {
            self.hx.published = Some(self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.edit.as_ref()).and_then(|e| e.last.clone()));
        }
        match apply(&mut self.shell, Action::House(a)) {
            Applied::House(said) => said,
            Applied::Trouble(f) => Err(f.said().to_string()),
            _ => Ok(String::new()),
        }
    }

    pub(super) fn step(&mut self, step: Step, args: &[(&str, &str)]) {
        self.run(Act::with(step, args));
    }

    pub(super) fn run(&mut self, a: Act) {
        let before = self.shell.desk.sh.toasts.clone();
        if let Err(e) = self.house(a) {
            if self.shell.desk.sh.toasts == before && !before.iter().any(|t| t.head == e) {
                self.shell.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: e, next: String::new(), detail: None });
            }
        }
    }

    pub(super) fn house_go(&mut self, p: Option<HPage>) {
        let to = p.map_or(crate::house::seam::PAGE_HOME, |p| p.name());
        self.step(Step::Page, &[("to", to)]);
    }

    pub(super) fn place_of(p: Option<HPage>) -> Place {
        use crate::nav::View;
        match p {
            None => Place::Home,
            Some(HPage::Settings) => Place::SettingsHome,
            Some(HPage::Orders) => Place::View(View::Orders, 0),
            Some(HPage::Queue) => Place::View(View::Queue, 0),
            Some(HPage::Goods) => Place::View(View::Goods, 0),
            Some(HPage::Cands) => Place::View(View::Cands, 0),
            Some(HPage::Export) => Place::View(View::Export, 0),
            Some(HPage::Payways) => Place::View(View::Payways, 0),
            Some(HPage::Alerts) => Place::View(View::Alerts, 0),
            Some(HPage::PayLink) => Place::View(View::PayLink, 0),
            Some(HPage::Wallet) => Place::View(View::Wallet, 0),
        }
    }

    pub(super) fn page_of(place: Place) -> Option<Option<HPage>> {
        use crate::nav::View;
        Some(match place {
            Place::Home => None,
            Place::SettingsHome | Place::Settings(_) => Some(HPage::Settings),
            Place::View(v, _) => Some(match v {
                View::Orders => HPage::Orders,
                View::Queue => HPage::Queue,
                View::Goods => HPage::Goods,
                View::Cands => HPage::Cands,
                View::Export => HPage::Export,
                View::Payways => HPage::Payways,
                View::Alerts => HPage::Alerts,
                View::PayLink => HPage::PayLink,
                View::Wallet => HPage::Wallet,
            }),
            Place::Page(_) => return None,
        })
    }

    pub(super) fn house_detail(&self) -> Option<Route> {
        let d = &self.shell.desk.sh;
        let under = self.house_detail_under()?;
        let inner = match (d.seat(), d.seller.page()) {
            (Seat::Seller, Some(HPage::Orders)) => d.seller.orders.as_ref().and_then(|o| o.inner).map(|i| i.word().to_string()),
            (Seat::Seller, Some(HPage::Goods)) => d.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).and_then(|s| s.inner.as_ref()).map(|i| i.word().to_string()),
            (Seat::Seller, Some(HPage::Cands)) => d.seller.cands.as_ref().and_then(|c| c.lapse.as_ref()).map(|_| "lapse".to_string()),
            (Seat::Buyer, Some(HPage::Orders)) => d.buyer_orders.as_ref().and_then(|b| b.open.as_ref()).and_then(|o| o.inner).map(|i| i.word().to_string()),
            (Seat::Buyer, Some(HPage::PayLink)) => d.paylink.as_ref().and_then(|p| p.inner.as_ref()).map(|i| i.word().to_string()),
            _ => None,
        };
        Some(match (under, inner) {
            (Route::House(u), Some(w)) => Route::House(crate::nav::Detail::Inner(Box::new(u), w)),
            (r, _) => r,
        })
    }

    fn house_detail_under(&self) -> Option<Route> {
        let d = &self.shell.desk.sh;
        match (d.seat(), d.seller.page()) {
            (Seat::Seller, Some(HPage::Orders)) => d.seller.orders.as_ref().and_then(|o| o.sel.clone()).map(|i| Route::House(crate::nav::Detail::Order(i))),
            (Seat::Buyer, Some(HPage::Orders)) => d.buyer_orders.as_ref().and_then(|b| b.open.as_ref()).map(|o| Route::House(crate::nav::Detail::BuyerOrder(o.instance.clone()))),
            (Seat::Seller, Some(HPage::Goods)) => d.seller.goods.as_ref().and_then(|g| match (&g.edit, &g.shown) {
                (Some(e), _) => Some(Route::House(crate::nav::Detail::Goods(e.id.clone().unwrap_or_default()))),
                (None, Some(s)) => Some(Route::House(crate::nav::Detail::Goods(s.id.clone()))),
                _ => None,
            })
            .or_else(|| self.hx.goods_open.clone().map(|id| Route::House(crate::nav::Detail::Goods(id)))),
            (Seat::Seller, Some(HPage::Cands)) => d.seller.cands.as_ref().and_then(|c| c.detail.as_ref()).map(|(row, _, _)| Route::House(crate::nav::Detail::Cand(row.address.clone()))),
            (Seat::Buyer, Some(HPage::PayLink)) => d.paylink.as_ref().and_then(|p| p.inner.as_ref()).map(|_| Route::House(crate::nav::Detail::PayLink)),
            _ => None,
        }
    }

    pub(super) fn house_detail_open(&mut self, d: &crate::nav::Detail) {
        match d {
            crate::nav::Detail::Order(i) => {
                self.hx.entering = Some(Route::House(d.clone()));
                self.step(Step::OrderRow, &[("instance", i)]);
            }
            crate::nav::Detail::BuyerOrder(i) => {
                self.hx.entering = Some(Route::House(d.clone()));
                self.step(Step::BoOpen, &[("instance", i)]);
            }
            crate::nav::Detail::Goods(id) if !id.is_empty() && !self.goods_ready() => self.hx.goods_open = Some(id.clone()),
            crate::nav::Detail::Goods(id) if !id.is_empty() => self.step(Step::PdOpen, &[("id", id)]),
            crate::nav::Detail::Cand(a) => {
                self.hx.entering = Some(Route::House(d.clone()));
                self.step(Step::CandOpen, &[("a", a)]);
            }
            crate::nav::Detail::Goods(_) | crate::nav::Detail::PayLink => {}
            crate::nav::Detail::Inner(under, _) => {
                let below = Route::House((**under).clone());
                let at = Route::House(d.clone());
                let open = self.under_open(&below);
                if !open || self.shell.page_opening() {
                    if !open {
                        self.house_detail_open(&under.clone());
                    }
                    self.hx.entering = Some(at.clone());
                    self.hx.reopen = Some(at);
                    self.hx.reopen_tried = None;
                    return;
                }
                self.hx.reopen = Some(at.clone());
                self.hx.reopen_tried = None;
                self.reopen_next(&at);
            }
        }
    }

    fn reopen_next(&mut self, route: &Route) {
        let Route::House(crate::nav::Detail::Inner(under, _)) = route else {
            self.hx.reopen = None;
            return;
        };
        let below = Route::House((**under).clone());
        let have = self.house_detail().map_or(0, |d| depth(&d));
        if !self.under_open(&below) {
            self.hx.reopen = None;
            self.hx.reopen_tried = None;
            return;
        }
        let over = |r: &Route| matches!(r, Route::House(crate::nav::Detail::Inner(u, _)) if **u == **under);
        let mut way: Vec<Route> = self.hist().back.iter().rev().take_while(|r| over(r) && depth(r) > have).cloned().collect();
        way.reverse();
        way.push(route.clone());
        let next = way[0].clone();
        if self.house_detail().as_ref() == Some(route) || self.hx.reopen_tried.as_ref() == Some(&next) {
            self.hx.reopen = None;
            self.hx.reopen_tried = None;
            return;
        }
        self.hx.reopen_tried = Some(next.clone());
        self.inner_reopen(&next);
    }

    fn under_open(&self, below: &Route) -> bool {
        match below {
            Route::House(crate::nav::Detail::PayLink) => self.shell.desk.seat() == Seat::Buyer && self.shell.desk.sh.seller.page() == Some(HPage::PayLink) && self.shell.desk.sh.paylink.is_some(),
            _ => self.house_detail_under().as_ref() == Some(below),
        }
    }

    fn inner_reopen(&mut self, at: &Route) {
        if let Some((step, args)) = self.hx.inner_open.iter().find(|(r, ..)| r == at).map(|(_, s, a)| (*s, a.clone())) {
            self.hx.entering = Some(at.clone());
            let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
            self.step(step, &args);
        }
    }

    fn goods_ready(&self) -> bool {
        self.shell.desk.sh.seller.goods.is_some() && !self.shell.house_in_flight(crate::task::Kind::Goods)
    }

    fn goods_follow(&mut self) {
        let on_goods = self.shell.desk.seat() == Seat::Seller && self.shell.desk.sh.seller.page() == Some(HPage::Goods);
        if !on_goods {
            self.hx.goods_open = None;
            self.hx.published = None;
            return;
        }
        if !self.goods_ready() {
            return;
        }
        if let Some(id) = self.hx.goods_open.take() {
            self.step(Step::PdOpen, &[("id", &id)]);
        }
        if let Some(before) = self.hx.published.take() {
            let last = self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.edit.as_ref()).and_then(|e| e.last.clone());
            if let Some(l) = last.filter(|l| Some(l) != before.as_ref() && !l.draft) {
                self.step(Step::PdOpen, &[("id", &l.id)]);
            }
        }
    }

    pub(super) fn house_rail_root(&mut self) -> bool {
        self.house_leave_to(None)
    }

    pub(super) fn house_leave_to(&mut self, keep: Option<&Route>) -> bool {
        let editing = |w: &Self| w.shell.desk.sh.seller.goods.as_ref().is_some_and(|g| g.edit.is_some());
        if let Some(id) = self.hx.goods_open.clone()
            && keep != Some(&Route::House(crate::nav::Detail::Goods(id)))
        {
            self.hx.goods_open = None;
        }
        loop {
            let Some(r) = self.house_detail().filter(|d| Some(d) != keep) else { return true };
            let was = editing(self) && matches!(r, Route::House(crate::nav::Detail::Goods(_)));
            let Some(s) = (if was { Some(Step::PeLeave) } else { Self::back_step(&r) }) else { return true };
            self.step(s, &[]);
            if self.house_detail().as_ref() == Some(&r) && was == editing(self) {
                return false;
            }
        }
    }

    fn back_step(route: &Route) -> Option<Step> {
        match route {
            Route::House(crate::nav::Detail::Order(_)) => Some(Step::OrderBack),
            Route::House(crate::nav::Detail::BuyerOrder(_)) => Some(Step::OrderBack),
            Route::House(crate::nav::Detail::Goods(_)) => Some(Step::PdBack),
            Route::House(crate::nav::Detail::Cand(_)) => Some(Step::CandBack),
            Route::House(crate::nav::Detail::Inner(..)) => Some(Step::InnerBack),
            _ => None,
        }
    }

    pub(super) fn house_follow(&mut self, now: f64) {
        self.hx.home_seen = self.shell.home.as_ref().map(|h| h.root().display().to_string()).unwrap_or_default();
        if let Some(p) = self.shell.desk.goto.take() {
            let place = Self::place_of(p);
            let on_section = matches!(self.route(), Route::Section(_)) && p == Some(HPage::Settings);
            if !on_section && (self.ux.nav.place != Some(place) || !self.route().is_root()) {
                self.go(place, now);
            }
        }
        if let Some(sec) = self.shell.desk.goto_section.take() {
            self.go(Place::Settings(sec), now);
        }
        self.goods_follow();
        let route = self.route();
        if self.hx.reopen.as_ref() == Some(&route) && !self.shell.page_opening() {
            self.reopen_next(&route);
        } else if self.hx.reopen.as_ref().is_some_and(|r| *r != route) {
            self.hx.reopen = None;
            self.hx.reopen_tried = None;
        }
        let want = self.house_detail();
        let held = self.shell.page_opening() && (self.hx.entering.as_ref() == Some(&route) || self.hx.reopen.as_ref() == Some(&route));
        match (&want, route.is_root()) {
            (Some(r), true) => self.push(r.clone(), now),
            (Some(r), false) if Self::back_step(&route).is_some() && *r != route && !held => {
                let behind = self.hist().back.last().cloned();
                match follow(r, &route, behind.as_ref()) {
                    Follow::Push => self.push(r.clone(), now),
                    Follow::Back => self.back(now),
                    Follow::Replace => self.hist_mut().cur = r.clone(),
                }
            }
            (None, false) if Self::back_step(&route).is_some() && !held => {
                let root = self.hist().bottom().clone();
                if let Route::Root(p) = root {
                    self.go(p, now);
                }
            }
            _ => {}
        }
        let shown = self.route();
        if self.hx.entering.as_ref().is_some_and(|e| *e != shown || self.house_detail().as_ref() == Some(e) || !self.shell.page_opening()) {
            self.hx.entering = None;
        }
        if self.house_detail().as_ref() == Some(&shown)
            && let Some((step, args)) = self.house_inner_step()
        {
            self.hx.inner_open.retain(|(r, ..)| *r != shown);
            self.hx.inner_open.push((shown.clone(), step, args));
        }
        self.hx.route_was = Some(shown);
        self.house_toasts(now);
    }

    fn house_inner_step(&self) -> Option<(Step, Vec<(&'static str, String)>)> {
        use crate::house::orders::Inner;
        let d = &self.shell.desk.sh;
        let case = |i: Inner, history: Step| match i {
            Inner::Judge(t) => (Step::CaseJudge, vec![("tab", t.to_string())]),
            Inner::Docs(t) => (Step::CaseDocs, vec![("tab", t.to_string())]),
            Inner::Lapse => (Step::LapseOpen, Vec::new()),
            Inner::History => (history, Vec::new()),
        };
        let text = |goods: bool| (Step::PdDoc, vec![("k", if goods { "goods" } else { "delivery" }.to_string())]);
        let lapse = |who: &str| (Step::LapseOpen, vec![("a", who.to_string())]);
        match (d.seat(), d.seller.page()) {
            (Seat::Seller, Some(HPage::Orders)) => d.seller.orders.as_ref().and_then(|o| o.inner).map(|i| case(i, Step::OrderHistory)),
            (Seat::Buyer, Some(HPage::Orders)) => d.buyer_orders.as_ref().and_then(|b| b.open.as_ref()).and_then(|o| o.inner).map(|i| case(i, Step::BoHistory)),
            (Seat::Seller, Some(HPage::Goods)) => d.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).and_then(|s| s.inner.as_ref()).map(|i| match i {
                crate::house::seller::GoodsInner::Doc(g) => text(*g),
                crate::house::seller::GoodsInner::Lapse(who) => lapse(who),
            }),
            (Seat::Seller, Some(HPage::Cands)) => d.seller.cands.as_ref().and_then(|c| c.lapse.as_deref()).map(lapse),
            (Seat::Buyer, Some(HPage::PayLink)) => d.paylink.as_ref().and_then(|p| p.inner.as_ref()).map(|i| match i {
                crate::house::paylink::PlInner::Doc(g) => text(*g),
                crate::house::paylink::PlInner::Lapse(who) => lapse(who),
                crate::house::paylink::PlInner::History(who) => (Step::PlHistory, vec![("a", who.clone())]),
            }),
            _ => None,
        }
    }

    fn house_toasts(&mut self, now: f64) {
        self.hx.hush.set(&mut self.toasts, self.shell.covered(), now);
        let ms = std::time::Duration::from_secs_f64(now.max(0.0)).as_millis() as u64;
        self.shell.desk.sh.at(ms);
        let lang = crate::lang::lang();
        match self.shell.desk.sh.toasts.first().cloned() {
            None => self.hx.toast_fed = None,
            Some(t) => {
                let key = (t.head.clone(), t.next.clone(), self.shell.desk.sh.toast_bumps);
                if self.hx.toast_fed.as_ref() != Some(&key) {
                    let tone = match t.tone {
                        crate::house::seller::Tone::Bad => Tone::Bad,
                        crate::house::seller::Tone::Ok | crate::house::seller::Tone::Note => Tone::Note,
                    };
                    let refusal = match (&t.tone, t.detail.as_deref()) {
                        (crate::house::seller::Tone::Bad, Some(name)) if !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') => Some(name),
                        _ => None,
                    };
                    let t_head = match refusal {
                        Some(name) => crate::house::refusal::line(lang, name, &t.head),
                        None => crate::house::i18n::library(lang, &t.head),
                    };
                    let t_next = crate::house::i18n::library(lang, &t.next);
                    let (head, raw_h) = split_raw(&t_head);
                    let (next, raw_n) = split_raw(&t_next);
                    let mut detail = match refusal {
                        Some(name) if t_head != t.head && !t.head.is_empty() => format!("{name} · {}", t.head),
                        _ => t.detail.as_deref().map(|d| crate::house::i18n::library(lang, d)).unwrap_or_default(),
                    };
                    for r in raw_h.iter().chain(&raw_n) {
                        if !detail.is_empty() {
                            detail.push_str(" · ");
                        }
                        detail.push_str(r);
                    }
                    let head = if head.is_empty() { t_head } else { head };
                    self.hx.hush.say_keys(&mut self.toasts, head, &next, &detail, tone, now, Vec::new());
                    self.hx.toast_fed = Some(key);
                }
            }
        }
        if self.toast_gone() {
            self.shell.desk.sh.toasts.clear();
            self.shell.desk.sh.toast_bumps = 0;
            self.hx.toast_fed = None;
        }
    }

    fn toast_gone(&self) -> bool {
        self.hx.toast_fed.is_some() && self.toasts.live() == 0 && self.hx.hush.held() == 0
    }


    pub(super) fn sheet_said(ui: &mut egui::Ui, id: &str, said: Option<&crate::house::seller::Said>) {
        if let Some(x) = said.filter(|x| x.tone == crate::house::seller::Tone::Bad) {
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            states::err_box(ui, id, &crate::house::refusal::line(crate::lang::lang(), &x.name, &x.detail), "", &crate::house::i18n::t(crate::house::i18n::Key::U3RawError), &raw_of(&x.name, &x.detail));
        }
    }

    pub(super) fn sheet_head(ui: &mut egui::Ui, title: &str, sub: &str) {
        sheet::title(ui, title, sub);
    }

    pub(super) fn sheet_keys(keys: &mut Keys, cancel: &str, go: Option<(&str, Role, bool)>) {
        Self::sheet_keys_n(keys, cancel, go.as_slice());
    }

    pub(super) fn sheet_keys_n(keys: &mut Keys, cancel: &str, go: &[(&str, Role, bool)]) {
        *keys = Keys { cancel: cancel.to_string(), go: go.iter().map(|(l, t, on)| (l.to_string(), *t, *on)).collect(), pick: None, picked: String::new(), left: Vec::new() };
    }

    pub(super) fn sheet_keys_sides(keys: &mut Keys, cancel: &str, go: &[(&str, Role, bool)], left: &[(&str, Role, bool)]) {
        Self::sheet_keys_n(keys, cancel, go);
        keys.left = left.iter().map(|(l, t, on)| (l.to_string(), *t, *on)).collect();
    }

    pub(super) fn sheet_keys_left(keys: &mut Keys, cancel: &str, go: Option<(&str, Role, bool)>, pick: Option<(&str, crate::platform::Pick)>) {
        Self::sheet_keys_n(keys, cancel, go.as_slice());
        keys.pick = pick.map(|(s, k)| (s.to_string(), k));
    }

    pub(super) fn sheet_picked(ctx: &egui::Context, salt: &str) -> Option<String> {
        ctx.data_mut(|d| d.remove_temp::<String>(egui::Id::new(("eravon-sheet-picked", salt))))
    }

    pub(super) fn sheet_pick<B>(ctx: &egui::Context, id: &str, width: f32, body: impl FnOnce(&mut egui::Ui, &mut Keys) -> B) -> (B, Option<usize>, bool) {
        let mut keys = Keys::default();
        let out = sheet::show(ctx, sheet::Spec::new(id, width), &mut keys, body, |ui, k| {
            let mut hit = None;
            for (i, (label, tone, on)) in k.go.iter().enumerate() {
                if key::key(ui, label, *tone, *on).clicked() {
                    hit = Some(i);
                }
            }
            let back = !k.cancel.is_empty() && key::key(ui, &k.cancel, Role::Secondary, true).clicked();
            if !k.left.is_empty() {
                let n = k.go.len();
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    for (i, (label, tone, on)) in k.left.iter().enumerate() {
                        if key::key(ui, label, *tone, *on).clicked() {
                            hit = Some(n + i);
                        }
                    }
                });
            }
            if let Some((salt, kind)) = &k.pick {
                let got = ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let got = foot_pick(ui, salt, *kind);
                    picked_beside(ui, &k.picked);
                    got
                });
                if let Some(p) = got.inner {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("eravon-sheet-picked", salt.as_str())), p));
                }
            }
            (hit, back)
        });
        (out.body, out.foot.0, out.foot.1 || out.esc)
    }

    pub(super) fn sheet_take<B>(&mut self, ctx: &egui::Context, id: &str, width: f32, kind: crate::platform::Pick, body: impl FnOnce(&mut egui::Ui, &mut Keys) -> B) -> (B, bool, bool, Option<String>) {
        let mut keys = Keys::default();
        let mut picked = None;
        let out = sheet::show(ctx, sheet::Spec::new(id, width), &mut keys, body, |ui, k| {
            let mut hit = false;
            for (label, tone, on) in &k.go {
                if key::key(ui, label, *tone, *on).clicked() {
                    hit = true;
                }
            }
            let back = !k.cancel.is_empty() && key::key(ui, &k.cancel, Role::Secondary, true).clicked();
            picked = foot_pick(ui, id, kind);
            (hit, back)
        });
        let taken = self.sheet_dropped(ctx).or(picked);
        (out.body, out.foot.0, out.foot.1 || out.esc, taken)
    }

    pub(super) fn sheet_dropped(&mut self, ctx: &egui::Context) -> Option<String> {
        let dropped = eravon_ui::drop::take_dropped(ctx);
        let now = self.ux.now;
        match dropped.as_slice() {
            [] => None,
            [one] => Some(one.clone()),
            _ => {
                self.toasts.say(t(Key::U3OneAtATime), Tone::Bad, now);
                None
            }
        }
    }

    pub(super) fn sheet_with<B>(ctx: &egui::Context, id: &str, width: f32, body: impl FnOnce(&mut egui::Ui, &mut Keys) -> B) -> (B, bool, bool) {
        let (b, hit, back) = Self::sheet_pick(ctx, id, width, body);
        (b, hit == Some(0), back)
    }

    pub(super) fn place_sheet(&mut self, ctx: &egui::Context, id: &'static str, title: &str, rows: &[(String, String)], commit: &str, start: impl FnOnce() -> String) -> Placed {
        if self.hx.place_open != Some(id) {
            return Placed::Open;
        }
        let at = match self.hx.places.get(id) {
            Some(p) => p.clone(),
            None => {
                let p = start();
                self.hx.places.insert(id, p.clone());
                p
            }
        };
        let label = crate::house::i18n::t(crate::house::i18n::Key::ExSaveTo);
        let none = crate::house::i18n::t(crate::house::i18n::Key::ExNoDir);
        let cancel = crate::house::i18n::t(crate::house::i18n::Key::CfBack);
        let (mut go, mut close, mut picked) = (false, false, None);
        let out = sheet::show(
            ctx,
            sheet::Spec::new(id, eravon_ui::tokens::SHEET_W),
            &mut (),
            |ui, _| {
                sheet::title(ui, title, "");
                card::form(ui, |ui, f| {
                    for (k, v) in rows {
                        f.info(ui, k, eravon_ui::card::Show::Text(v));
                    }
                    f.folder(ui, &label, &at, &none);
                });
            },
            |ui, _| {
                go = key::show(ui, key::Key::new(commit, Role::Primary).enabled(!at.trim().is_empty())).clicked();
                close = key::key(ui, &cancel, Role::Secondary, true).clicked();
                picked = foot_pick(ui, id, crate::platform::Pick::Folder);
            },
        );
        if let Some(p) = picked {
            self.hx.places.insert(id, p);
        }
        if close || out.esc {
            self.hx.place_open = None;
            return Placed::Close;
        }
        if go {
            self.hx.place_open = None;
            return Placed::Go(at);
        }
        Placed::Open
    }

    pub(super) fn hero(ui: &mut egui::Ui, title: &str, sub: &str, pills: &[card::HeroPill]) {
        card::hero(ui, title, sub, false, pills);
    }

    pub(super) fn hero_beside(ui: &mut egui::Ui, title: &str, sub: &str, pills: &[card::HeroPill], right: impl FnOnce(&mut egui::Ui)) {
        let w = ui.available_width();
        let gap = ui.spacing().item_spacing.x;
        ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            egui::Frame::new().inner_margin(egui::Margin { left: 0, right: 0, top: HERO_TOP, bottom: 0 }).show(ui, |ui| {
                ui.allocate_ui_with_layout(egui::vec2(0.0, HERO_TITLE_H), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = tk::S2;
                    right(ui);
                    for p in pills {
                        if p.live {
                            mark::pill_live(ui, &p.words, p.tone);
                        } else {
                            mark::pill(ui, &p.words, p.tone);
                        }
                    }
                });
            });
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                card::hero(ui, title, sub, false, &[])
            });
        });
    }

    pub(super) fn hero_keys(&mut self, ui: &mut egui::Ui, title: &str, sub: &str, pills: &[card::HeroPill]) {
        let (route, now) = (self.route(), self.ux.now);
        Self::hero_beside(ui, title, sub, pills, |ui| {
            ui.spacing_mut().item_spacing.x = tk::S2;
            self.house_toolbar(ui, &route, now);
        });
    }

    pub(super) fn write_banner(&mut self, ui: &mut egui::Ui, w: crate::house::shell::Write) -> bool {
        let Some(cause) = self.shell.write_cause(w) else { return true };
        let why = crate::house::i18n::get_named(crate::lang::lang(), cause).0;
        let network = matches!(cause, "write.noChain" | "write.badChain" | "write.wrongChain");
        let go = states::banner(ui, states::Banner::Warn, &why, |ui| network && key::key(ui, t(Key::SetNetwork), Role::Secondary, true).clicked());
        if go {
            self.shell.desk.goto_section = Some(crate::nav::Section::Network);
        }
        false
    }

    pub(super) fn said_box(&self, ui: &mut egui::Ui, id: &str, s: &crate::house::seller::Said, next: &str) {
        let lang = crate::lang::lang();
        let text = crate::house::refusal::line(lang, &s.name, &s.detail);
        let (head, raw) = split_raw(&text);
        if s.tone == crate::house::seller::Tone::Bad {
            let mut detail = if text != s.detail && !s.detail.is_empty() { format!("{} · {}", s.name, s.detail) } else { s.name.clone() };
            for r in &raw {
                detail.push_str(" · ");
                detail.push_str(r);
            }
            let next = if next.is_empty() { crate::house::i18n::t(crate::house::i18n::Key::SaidNext) } else { next.to_string() };
            states::err_box(ui, id, if head.is_empty() { &text } else { &head }, &next, &crate::house::i18n::t(crate::house::i18n::Key::U3RawError), &detail);
            return;
        }
        states::okline(ui, Mark::Ok, if head.is_empty() { &text } else { &head });
        if !raw.is_empty() {
            fold::fold(ui, &format!("{id}-raw"), &crate::house::i18n::t(crate::house::i18n::Key::ToastDetail), |ui| {
                for r in &raw {
                    ui.add(egui::Label::new(egui::RichText::new(r).font(Type::Mono.font()).color(c(C::Ink2))).selectable(true).wrap());
                }
            });
        }
    }
}

impl Win {
    pub(in crate::window) fn house_layers(&mut self, ctx: &egui::Context) {
        self.door_ask_modal(ctx);
        self.time_ask_modal(ctx);
    }

    fn door_ask_modal(&mut self, ctx: &egui::Context) {
        let t = |k: crate::house::i18n::Key| crate::house::i18n::t(k);
        let Some(q) = crate::house::approve::awaiting(&self.shell).cloned() else { return };
        if !self.shell.vault.absent() && crate::window::chrome::chord(ctx, egui::Key::L) {
            self.lock_now(eravon_ui::motion::now(ctx));
            return;
        }
        let flags = q.args.iter().map(|(k, v)| format!("--{k} {v}")).collect::<Vec<_>>().join("\n");
        let (_, go, back) = Self::sheet_with(ctx, "door-ask", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::DoorAskTitle), &t(crate::house::i18n::Key::DoorAskNote));
            details_list(ui, "door-ask-details", &[(t(crate::house::i18n::Key::DoorAskVerb).as_str(), Val::mono(q.verb.clone())), (t(crate::house::i18n::Key::DoorAskFlags).as_str(), Val::mono(flags.clone()))]);
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::DoorAskDecline), Some((&t(crate::house::i18n::Key::DoorAskApprove), Role::Commit, true)));
        });
        if go {
            self.step(Step::DoorApprove, &[]);
        } else if back {
            self.step(Step::DoorDecline, &[]);
        }
    }

    fn time_ask_modal(&mut self, ctx: &egui::Context) {
        let t = |k: crate::house::i18n::Key| crate::house::i18n::t(k);
        let d = &self.shell.desk.sh;
        let say = [d.seller.goods.as_ref().and_then(|x| x.ask.clone()), d.seller.orders.as_ref().and_then(|x| x.ask.clone()), d.buyer_orders.as_ref().and_then(|x| x.ask.clone()), d.paylink.as_ref().and_then(|x| x.ask.clone())]
            .into_iter()
            .flatten()
            .next()
            .map(|a| a.say);
        let Some(say) = say else { return };
        use crate::task::Kind;
        if self.shell.house_in_flight(Kind::Goods) || self.shell.house_acting(Kind::Orders) || self.shell.house_acting(Kind::Bo) || self.shell.house_in_flight(Kind::PayLink) {
            return;
        }
        let (_, go, back) = Self::sheet_with(ctx, "time-ask", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::SetCoTime), "");
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            states::note_box(ui, &say);
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::AskGo), Role::Commit, true)));
        });
        if go || back {
            self.step(Step::TimeAsk, &[("v", if go { "go" } else { "cancel" })]);
        }
    }
}

pub fn form_answer() -> eravon_literals::json::Out {
    use eravon_literals::json::Out;
    use eravon_ui::report;
    let num = |v: f32| Out::S(if (v - v.round()).abs() < f32::EPSILON { format!("{}", v as i64) } else { format!("{v}") });
    let rows = |t: Vec<(&'static str, f32)>| {
        Out::O(t.into_iter().map(|(k, v)| (k, num(v))).collect::<Vec<_>>())
    };
    let ctx = egui::Context::default();
    eravon_ui::skin::apply(&ctx);
    Out::O(vec![
        ("floorOk", Out::B(report::floor_ok(&ctx))),
        ("form", rows(report::metrics_rows())),
        (
            "frames",
            Out::O(
                eravon_ui::frames::read_all()
                    .into_iter()
                    .map(|(k, r)| {
                        let v = match r {
                            eravon_ui::frames::Reading::Nums(v) => Out::A(v.into_iter().map(num).collect()),
                            eravon_ui::frames::Reading::Words(v) => Out::A(v.into_iter().map(Out::S).collect()),
                            eravon_ui::frames::Reading::Flags(v) => Out::O(v.into_iter().map(|(k, b)| (k, Out::B(b))).collect()),
                        };
                        (k, v)
                    })
                    .collect::<Vec<_>>(),
            ),
        ),
        ("installed", rows(report::installed_rows(&ctx))),
        ("ok", Out::B(true)),
    ])
}

pub fn shell_headless() -> crate::shell::Shell {
    let ctx = egui::Context::default();
    crate::shell::Shell::boot(eravon_ui::skin::dress(&ctx))
}

pub(super) fn unread(name: &str) -> bool {
    name == eravon_chain::fault::Fault::Unread(String::new()).name()
}

pub(super) fn money_tile(ui: &egui::Ui, title: &str, amounts: &[(String, String)]) -> (String, String) {
    let room = ui.available_width() - 2.0 * eravon_ui::tokens::CARD_PAD_X;
    let fits = |s: &str| eravon_ui::width::elide_to(ui, s, eravon_ui::tokens::Type::Figure.font(), room) == s;
    let whole = |(f, u): &(String, String)| if u.is_empty() { f.clone() } else { format!("{f} {u}") };
    let all: Vec<String> = amounts.iter().map(whole).collect();
    match amounts {
        [] => (title.to_string(), String::new()),
        [one] if !fits(&all[0]) && !one.1.is_empty() => (format!("{title} · {}", one.1), one.0.clone()),
        [_] => (title.to_string(), all[0].clone()),
        _ => {
            let every = all.join(" · ");
            let shown = if fits(&every) { None } else { (1..all.len()).rev().map(|n| format!("{} · …", all[..n].join(" · "))).find(|s| fits(s)) };
            (title.to_string(), shown.unwrap_or(every))
        }
    }
}

pub(super) fn unread_word(syncing: bool) -> String {
    crate::house::i18n::t(if syncing { crate::house::i18n::Key::SyncSyncing } else { crate::house::i18n::Key::SyncNotYet })
}

pub(super) fn unread_block(ui: &mut egui::Ui, syncing: bool, widths: &[f32]) {
    if syncing {
        eravon_ui::states::skeleton_lines(ui, widths);
    } else {
        eravon_ui::states::hint(ui, &unread_word(false));
    }
}

#[cfg(test)]
mod unread_tests {
    #[test]
    fn not_read_yet_is_no_failed_read() {
        assert!(super::unread(eravon_chain::fault::Fault::Unread("x".into()).name()));
        assert!(!super::unread("E_WATCH_NO_ENDPOINT"));
        assert!(!super::unread(eravon_chain::fault::Fault::Disagree("x".into()).name()));
        assert_ne!(super::unread_word(true), super::unread_word(false));
        assert_eq!(super::unread_word(true), crate::house::i18n::t(crate::house::i18n::Key::SyncSyncing));
    }
}

#[cfg(test)]
mod money_tile_tests {
    use eravon_ui::egui;
    fn laid(w: f32, amounts: &[(&str, &str)]) -> (String, String) {
        let ctx = egui::Context::default();
        let _ = eravon_ui::skin::dress(&ctx);
        let amounts: Vec<(String, String)> = amounts.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let mut out = (String::new(), String::new());
        for _ in 0..2 {
            let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))), ..Default::default() };
            #[allow(deprecated)]
            let _ = ctx.run(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.allocate_ui(egui::vec2(w, 100.0), |ui| out = super::money_tile(ui, "余额", &amounts));
                });
            });
        }
        out
    }

    #[test]
    fn a_short_token_name_is_never_cut_again_by_the_tile() {
        let unit = "未登载 0xdc64…f6c9";
        assert_eq!(laid(640.0, &[("12.5", unit)]), ("余额".to_string(), format!("12.5 {unit}")), "with room, beside its figure");
        let (title, figure) = laid(180.0, &[("12.5", unit)]);
        assert_eq!((title.as_str(), figure.as_str()), (format!("余额 · {unit}").as_str(), "12.5"), "without, up beside the title");
        let many = [("12.5 USDC", ""), ("3 DAI", ""), ("1000 USDT", ""), ("7 EURC", "")];
        let (title, figure) = laid(260.0, &many);
        assert_eq!(title, "余额");
        let shown: Vec<&str> = figure.split(" · ").collect();
        assert_eq!(shown.last(), Some(&"…"), "fewer shown, said: {figure}");
        assert!(shown[..shown.len() - 1].iter().all(|x| many.iter().any(|(m, _)| m == x)), "each shown amount whole: {figure}");
        assert_eq!(laid(900.0, &many).1, "12.5 USDC · 3 DAI · 1000 USDT · 7 EURC", "with room, every one");
    }
}

const HERO_TOP: i8 = 2;
const HERO_TITLE_H: f32 = tk::S6;

pub(super) fn hero_pill(words: &str, tone: eravon_ui::palette::Tone) -> card::HeroPill {
    card::HeroPill { words: words.to_string(), tone, live: false }
}

#[derive(Debug, PartialEq, Eq)]
enum Follow {
    Push,
    Back,
    Replace,
}

fn follow(to: &Route, at: &Route, behind: Option<&Route>) -> Follow {
    match depth(to).cmp(&depth(at)) {
        std::cmp::Ordering::Greater => Follow::Push,
        std::cmp::Ordering::Less if behind == Some(to) => Follow::Back,
        _ => Follow::Replace,
    }
}

fn depth(r: &Route) -> usize {
    match r {
        Route::House(crate::nav::Detail::Inner(under, w)) => {
            let over_order = matches!(**under, crate::nav::Detail::Order(_) | crate::nav::Detail::BuyerOrder(_));
            1 + if over_order { crate::house::orders::Inner::depth_of(w) } else { 1 }
        }
        Route::House(_) => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod follow_tests {
    use super::*;
    use crate::nav::Detail;

    fn over(d: Detail, w: &str) -> Route {
        Route::House(Detail::Inner(Box::new(d), w.into()))
    }

    #[test]
    fn the_history_goes_as_deep_as_the_desk_and_back_the_way_it_came() {
        let order = Route::House(Detail::Order("0xa".into()));
        let mine = Route::House(Detail::BuyerOrder("0xa".into()));
        let item = Route::House(Detail::Goods("g1".into()));
        let cand = Route::House(Detail::Cand("0xc".into()));
        let list = Route::Root(Place::Home);
        for (under, d) in [(&order, Detail::Order("0xa".into())), (&mine, Detail::BuyerOrder("0xa".into()))] {
            let bench = over(d.clone(), "caseJudge");
            let lapse = over(d.clone(), "lapse");
            for page in [bench.clone(), over(d.clone(), "caseDocs"), over(d.clone(), "history")] {
                assert_eq!(follow(&page, under, Some(&list)), Follow::Push, "{page:?}");
                assert_eq!(follow(under, &page, Some(under)), Follow::Back, "{page:?}");
            }
            assert_eq!(follow(&lapse, &bench, Some(under)), Follow::Push);
            assert_eq!(follow(&bench, &lapse, Some(&bench)), Follow::Back);
            assert_eq!(follow(under, &lapse, Some(&bench)), Follow::Replace, "two pages up at once, the bench not behind");
        }
        for w in ["goodsText", "deliveryText", "lapse"] {
            assert_eq!(follow(&over(Detail::Goods("g1".into()), w), &item, Some(&list)), Follow::Push, "{w}");
        }
        assert_eq!(follow(&over(Detail::Cand("0xc".into()), "lapse"), &cand, Some(&list)), Follow::Push);
        assert_eq!(follow(&over(Detail::PayLink, "history"), &list, None), Follow::Push);
        assert_eq!(follow(&Route::House(Detail::Order("0xb".into())), &order, Some(&list)), Follow::Replace, "another order, same depth");
    }
}
