use super::*;
use crate::house::seller::Page as HPage;
use crate::house::shell::Seat;
use crate::nav::View;

impl Win {
    pub(in crate::window) fn house_body(&mut self, ui: &mut egui::Ui, route: &Route, now: f64) {
        let under = match route {
            Route::House(crate::nav::Detail::Inner(u, _)) => Route::House((**u).clone()),
            r => r.clone(),
        };
        let route = &under;
        let seat = self.shell.desk.seat();
        let no_key = self.shell.desk.addr.is_empty() || self.shell.anchor.is_none();
        if no_key || self.shell.home.is_none() {
            let (glyph, said, go) = if no_key { (eravon_ui::icons::Glyph::Key, t(Key::ChipNoKey), t(Key::PageFirstRun)) } else { (eravon_ui::icons::Glyph::Data, t(Key::FaultWhatNoHome), t(Key::SetChangeHome)) };
            states::empty(ui, glyph, said);
            let mut press = false;
            ui.vertical_centered(|ui| press = key::key(ui, go, Role::Secondary, true).clicked());
            if press && no_key {
                self.wizard_open(self.progress().first_open().unwrap_or(crate::nav::Step::Pin));
            } else if press {
                self.go(Place::Settings(crate::nav::Section::Data), now);
            }
            return;
        }
        match (route, seat) {
            (Route::Root(Place::Home) | Route::Root(Place::Page(_)), Seat::Seller) => self.home_page(ui),
            (Route::Root(Place::View(View::Alerts, _)), Seat::Seller) => self.alerts_page(ui),
            (Route::Root(Place::View(View::Orders, _)) | Route::House(crate::nav::Detail::Order(_)), Seat::Seller) => self.orders_page(ui),
            (Route::Root(Place::View(View::Goods, _)) | Route::House(crate::nav::Detail::Goods(_)), Seat::Seller) => self.goods_page(ui),
            (Route::Root(Place::View(View::Cands, _)) | Route::House(crate::nav::Detail::Cand(_)), Seat::Seller) => self.cands_page(ui),
            (Route::Root(Place::View(View::Payways, _)), Seat::Seller) => self.payways_page(ui),
            (Route::Root(Place::Home) | Route::Root(Place::Page(_)), Seat::Buyer) => self.buyer_home_page(ui),
            (Route::Root(Place::View(View::Alerts, _)), Seat::Buyer) => self.buyer_alerts_page(ui),
            (Route::Root(Place::View(View::Wallet, _)), Seat::Buyer) => self.wallet_page(ui),
            (Route::Root(Place::View(View::Wallet, _)), Seat::Seller) => self.seller_wallet_page(ui),
            (Route::Root(Place::View(View::PayLink, _)) | Route::House(crate::nav::Detail::PayLink), Seat::Buyer) => self.paylink_page(ui),
            (Route::Root(Place::View(View::Export, _)), _) => self.export_page(ui),
            (Route::Root(Place::View(View::Queue, _)), _) => self.queue_page(ui),
            (Route::Root(Place::View(View::Orders, _)) | Route::House(crate::nav::Detail::BuyerOrder(_)), Seat::Buyer) => self.buyer_orders_page(ui),
            _ => {}
        }
    }

    pub(in crate::window) fn house_title(&self, route: &Route) -> String {
        if let Route::House(crate::nav::Detail::Inner(u, _)) = route {
            return self.house_title(&Route::House((**u).clone()));
        }
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let d = &self.shell.desk.sh;
        match route {
            Route::Root(Place::Home) | Route::Root(Place::Page(_)) => t(crate::house::i18n::Key::NavHome),
            Route::Root(Place::View(v, _)) => crate::lang::t(v.key()).to_string(),
            Route::House(crate::nav::Detail::PayLink) => crate::lang::t(crate::nav::View::PayLink.key()).to_string(),
            Route::House(crate::nav::Detail::Order(sel)) => match d.seller.orders.as_ref().and_then(|o| o.table.as_ref()?.as_ref().ok()?.iter().find(|r| r.instance.eq_ignore_ascii_case(sel)).cloned()) {
                Some(r) => format!("{} {}", t(crate::house::i18n::Key::OrdersOrder), crate::house::orders::row_label(&r)),
                None => t(crate::house::i18n::Key::NavOrders),
            },
            Route::House(crate::nav::Detail::BuyerOrder(i)) => match d.buyer_orders.as_ref().and_then(|b| b.rows.as_ref()?.as_ref().ok()?.iter().find(|x| x.row.instance == *i).cloned()) {
                Some(r) => if r.product.is_empty() { t(crate::house::i18n::Key::BoUntitled) } else { r.product.clone() },
                None => t(crate::house::i18n::Key::NavOrders),
            },
            Route::House(crate::nav::Detail::Goods(_)) => match d.seller.goods.as_ref() {
                Some(g) => match (&g.edit, &g.shown) {
                    (Some(e), _) if !e.draft.name.trim().is_empty() => e.draft.name.clone(),
                    (Some(_), _) => t(crate::house::i18n::Key::GoodsNew),
                    (None, Some(s)) => match s.view.as_ref().map(|v| v.row.name.clone()) {
                        Ok(n) if !n.trim().is_empty() => n,
                        _ => t(crate::house::i18n::Key::GoodsUnnamed),
                    },
                    (None, None) => t(crate::house::i18n::Key::NavGoods),
                },
                None => t(crate::house::i18n::Key::NavGoods),
            },
            Route::House(crate::nav::Detail::Cand(_)) => match d.seller.cands.as_ref().and_then(|c| c.detail.as_ref()) {
                Some((row, _, _)) => format!("{} · {}", t(if row.kind == "guild" { crate::house::i18n::Key::CandsGuild } else { crate::house::i18n::Key::PlCands }), row.label),
                None => t(crate::house::i18n::Key::NavCands),
            },
            _ => String::new(),
        }
    }

    pub(in crate::window) fn house_toolbar(&mut self, ui: &mut egui::Ui, route: &Route, now: f64) {
        let _ = (route, now);
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let seat = self.shell.desk.seat();
        let page = self.shell.desk.sh.seller.page();
        if page == Some(HPage::Export) {
            self.export_seg(ui);
            return;
        }
        if seat == Seat::Buyer {
            if page == Some(HPage::Orders) {
                if let Some(b) = self.shell.desk.sh.buyer_orders.clone().filter(|b| b.open.is_none() && matches!(b.rows, Some(Ok(_)))) {
                    let (mut active, mut done) = (0usize, 0usize);
                    if let Some(Ok(rows)) = &b.rows {
                        for r in rows {
                            if eravon_orders::table::done(&r.row) {
                                done += 1;
                            } else {
                                active += 1;
                            }
                        }
                    }
                    let (a_word, d_word) = (t(crate::house::i18n::Key::OrdersActive), t(crate::house::i18n::Key::Done));
                    let cells = [seg::Cell { text: &a_word, count: Some(active) }, seg::Cell { text: &d_word, count: Some(done) }];
                    if let Some(i) = seg::seg(ui, "bo-seg", &cells, usize::from(b.done_tab)) {
                        self.step(Step::BoListTab, &[("v", if i == 1 { "done" } else { "active" })]);
                    }
                }
            }
            if page == Some(HPage::PayLink) {
                if let Some(pl) = self.shell.desk.sh.paylink.clone() {
                    let (p_word, i_word) = (t(crate::house::i18n::Key::PlTabPay), t(crate::house::i18n::Key::SetImport));
                    let at = if pl.tab == crate::house::paylink::Tab::Pay { 0 } else { 1 };
                    let cells = [seg::Cell { text: &p_word, count: None }, seg::Cell { text: &i_word, count: None }];
                    if let Some(i) = seg::seg(ui, "pl-seg", &cells, at) {
                        let tab = if i == 0 { crate::house::paylink::Tab::Pay } else { crate::house::paylink::Tab::Import };
                        self.step(Step::PlTab, &[("v", tab.name())]);
                    }
                }
            }
            return;
        }
        if seat == Seat::Seller && page == Some(HPage::Orders) {
            let Some(os) = self.shell.desk.sh.seller.orders.clone().filter(|o| o.sel.is_none()) else { return };
            ui.scope(|ui| {
                if key::key(ui, &t(crate::house::i18n::Key::OrdersPasteKey), Role::Secondary, true).clicked() {
                    self.step(Step::OrderPasteOpen, &[]);
                }
                let (mut active, mut done) = (0usize, 0usize);
                if let Some(Ok(rows)) = &os.table {
                    for r in rows {
                        if eravon_orders::table::done(r) {
                            done += 1;
                        } else {
                            active += 1;
                        }
                    }
                }
                let (a_word, d_word) = (t(crate::house::i18n::Key::OrdersActive), t(crate::house::i18n::Key::Done));
                let cells = [seg::Cell { text: &a_word, count: Some(active) }, seg::Cell { text: &d_word, count: Some(done) }];
                if let Some(i) = seg::seg(ui, "orders-seg", &cells, usize::from(os.done_tab)) {
                    self.step(Step::OrderListTab, &[("v", if i == 1 { "done" } else { "active" })]);
                }
            });
        }
        if seat == Seat::Seller && page == Some(HPage::Goods) {
            let Some(gd) = self.shell.desk.sh.seller.goods.clone() else { return };
            ui.scope(|ui| {
                if let Some(e) = &gd.edit {
                    let saving = self.shell.house_in_flight(crate::task::Kind::Goods);
                    let confirm = e.tab == 4;
                    let (formal, fees) = (eravon_goods::goods::local(&e.draft).formal(), e.fees_shown().is_some());
                    let lit = confirm && !saving && formal && fees;
                    let lang = crate::lang::lang();
                    let mut missing: Vec<String> = e.last.as_ref().map(|sv| sv.verdict.missing.iter().map(|m| crate::house::i18n::get_named(lang, &format!("goods.cell.{}", super::goods::cell_key(&m.cell))).0).collect()).unwrap_or_default();
                    missing.dedup();
                    if missing.is_empty() && !fees {
                        missing.push(t(crate::house::i18n::Key::BoTabFee));
                    }
                    if missing.is_empty() && !confirm {
                        missing.push(t(crate::house::i18n::Key::GoodsTabConfirm));
                    }
                    let why = crate::house::i18n::filln(crate::house::i18n::Key::GoodsPublishMissing, &[&missing.join(&t(crate::house::i18n::Key::ListJoin))]);
                    let publish = key::key(ui, &t(crate::house::i18n::Key::GoodsPublish), Role::Primary, lit);
                    let publish = if lit || missing.is_empty() { publish } else { publish.on_disabled_hover_text(&why) };
                    if publish.clicked() {
                        self.shell.desk.goods_publish = Some(true);
                    }
                    let bare = key::key(ui, &t(crate::house::i18n::Key::GoodsPublishBare), Role::Secondary, lit);
                    let bare = if lit || missing.is_empty() { bare } else { bare.on_disabled_hover_text(&why) };
                    if bare.clicked() {
                        self.shell.desk.goods_publish = Some(false);
                    }
                    if key::key(ui, &t(crate::house::i18n::Key::ReadNetSave), Role::Secondary, true).clicked() {
                        self.shell.desk.goods_bar = Some(true);
                    }
                    if key::key(ui, &t(crate::house::i18n::Key::CfBack), Role::Secondary, !saving).clicked() {
                        self.shell.desk.goods_bar = Some(false);
                    }
                    return;
                }
                if let Some(sh) = &gd.shown {
                    let draft = sh.view.as_ref().is_ok_and(|v| v.row.draft);
                    if key::key(ui, &t(crate::house::i18n::Key::GoodsEdit), if draft { Role::Primary } else { Role::Secondary }, sh.view.is_ok()).clicked() {
                        self.step(Step::PdEdit, &[]);
                    }
                    return;
                }
                if key::key(ui, &t(crate::house::i18n::Key::GoodsNew), Role::Primary, true).clicked() {
                    self.step(Step::PeNew, &[]);
                }
            });
        }
        if seat == Seat::Seller && page == Some(HPage::Cands) {
            use eravon_cands::table::Side;
            let Some(c) = self.shell.desk.sh.seller.cands.clone().filter(|c| c.detail.is_none()) else { return };
            ui.scope(|ui| {
                let add = if c.tab == Side::Judge { t(crate::house::i18n::Key::CandsAddJudge) } else { t(crate::house::i18n::Key::CandsAddGuild) };
                if key::key(ui, &add, Role::Primary, true).clicked() {
                    self.shell.desk.add_form = Some((c.tab, String::new(), String::new()));
                }
                let labels = [t(crate::house::i18n::Key::CandsJudges), t(crate::house::i18n::Key::CandsGuilds)];
                let at = if c.tab == Side::Judge { 0 } else { 1 };
                let cells = [seg::Cell { text: &labels[0], count: None }, seg::Cell { text: &labels[1], count: None }];
                if let Some(i) = seg::seg(ui, "cands-seg", &cells, at) {
                    self.step(Step::CandsTab, &[("t", if i == 0 { Side::Judge.word() } else { Side::Guild.word() })]);
                }
            });
        }
    }

    pub(in crate::window) fn house_whole_drop(&mut self, ctx: &egui::Context, open: bool, now: f64) {
        let on_import = self.shell.desk.sh.seller.page() == Some(HPage::PayLink)
            && self.shell.desk.sh.paylink.as_ref().is_some_and(|p| p.tab == crate::house::paylink::Tab::Import || (p.tab == crate::house::paylink::Tab::Pay && p.takes_drop()));
        let buyer = self.shell.desk.seat() == Seat::Buyer;
        let covered = self.any_sheet() || sheet::up(ctx) || self.shell.desk.set_card.is_some();
        let open = open && buyer && !on_import && !covered;
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        drop::veil(ctx, open && drop::dragging(ctx), &t(crate::house::i18n::Key::DropOrderTitle), &t(crate::house::i18n::Key::DropOrderNote));
        if !sheet::up(ctx) && ctx.memory(|m| m.focused().is_none()) {
            let pasted = ctx.input(|i| i.events.iter().find_map(|e| if let egui::Event::Paste(s) = e { Some(s.trim().to_string()) } else { None }));
            if let Some(u) = pasted.filter(|u| crate::house::paylink::open_plan(u).is_ok()) {
                self.shell.take_url(&u);
            }
        }
        let got = drop::take_dropped(ctx);
        if !open {
            if !got.is_empty() && self.rail_shown() {
                use crate::house::i18n::Key as K;
                let said = if covered {
                    t(K::DropCloseSheet)
                } else if buyer {
                    crate::lang::fill1(K::DropElsewhere, &t(K::TaskPaylink))
                } else {
                    t(K::DropNotHere)
                };
                self.toasts.say(said, Tone::Bad, now);
            }
            return;
        }
        match got.as_slice() {
            [] => {}
            [one] => {
                self.house_go(Some(HPage::PayLink));
                self.step(Step::PlTab, &[("v", "import")]);
                self.step(Step::PlFile, &[("path", one)]);
            }
            _ => self.toasts.say(t(crate::house::i18n::Key::DropOneAtATime), Tone::Bad, now),
        }
    }
}
