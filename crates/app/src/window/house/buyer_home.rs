use super::*;
use crate::house::action::Action as Step;

impl Win {
    fn bhome_now(&mut self) -> Option<crate::house::buyer_home::BHome> {
        let me = self.shell.desk.addr.clone();
        match self.shell.desk.sh.buyer_home.clone() {
            Some(h) if h.fresh_for(&me) => Some(h),
            _ => {
                if !self.shell.house_in_flight(crate::task::Kind::BHome) {
                    self.shell.spawn_bhome();
                }
                None
            }
        }
    }

    pub(super) fn wallet_now(&mut self) -> Option<crate::house::wallet::Wallet> {
        let me = self.shell.desk.addr.clone();
        match self.shell.desk.sh.wallet.clone() {
            Some(w) if w.me.eq_ignore_ascii_case(&me) => Some(w),
            _ => {
                if !self.shell.page_in_flight(crate::task::Kind::Wallet) {
                    self.shell.load_wallet();
                }
                None
            }
        }
    }

    pub(super) fn buyer_home_page(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(h) = self.bhome_now() else {
            Self::home_skeleton(ui, 3);
            return;
        };
        if h.stopped {
            eravon_ui::states::banner(ui, eravon_ui::states::Banner::Warn, &t(crate::house::i18n::Key::PinLockedTitle), |_| {});
            return;
        }
        use crate::house::seller::Page;
        let unsynced = t(crate::house::i18n::Key::SyncNever);
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let unread_of = |v: &Option<Result<u64, crate::house::seller::Said>>| matches!(v, Some(Err(e)) if super::unread(&e.name));
        let num = |v: &Option<Result<u64, crate::house::seller::Said>>| match v {
            Some(Ok(n)) => n.to_string(),
            _ if unread_of(v) => t(crate::house::i18n::Key::HomeNoFigure),
            _ => unsynced.clone(),
        };
        let (whole, cut): (Vec<String>, bool) = match &h.balance {
            Some(Ok(b)) => {
                let held = b.coins.iter().filter_map(|c| c.have.as_ref().ok().filter(|x| *x != "0").map(|x| (x, c)));
                let cut = held.clone().any(|(x, c)| crate::house::coin::cut(lang, x, c.decimals, &c.token).is_some()) || b.native.as_ref().ok().is_some_and(|w| crate::house::coin::eth_cut(lang, w).is_some());
                let whole = held
                    .map(|(x, c)| crate::house::coin::amount(lang, x, c.decimals, &c.token, crate::house::coin::Form::Full))
                    .chain(b.native.as_ref().ok().map(|w| crate::house::coin::eth(lang, w, crate::house::coin::Form::Full)))
                    .collect();
                (whole, cut)
            }
            _ => (Vec::new(), false),
        };
        let (amounts, eth): (Vec<(String, String)>, String) = match &h.balance {
            Some(Ok(b)) => {
                let held: Vec<_> = b.coins.iter().filter(|c| c.have.as_ref().is_ok_and(|x| x != "0")).collect();
                let coins: Vec<String> = held.iter().map(|c| crate::house::coin::amount(lang, c.have.as_deref().unwrap_or("0"), c.decimals, &c.token, crate::house::coin::Form::Short)).collect();
                let eth = b.native.as_ref().map(|w| crate::house::coin::eth(lang, w, crate::house::coin::Form::Short)).unwrap_or_else(|_| unsynced.clone());
                let unread = b.coins.iter().filter(|c| c.have.is_err()).count();
                let sub = if unread == 0 { eth } else { format!("{eth} · {unread} {}", t(crate::house::i18n::Key::BhUnreadN)) };
                match coins.as_slice() {
                    [] => (vec![("0".to_string(), String::new())], sub),
                    [_] => {
                        let c = held[0];
                        (vec![crate::house::coin::parts(lang, c.have.as_deref().unwrap_or("0"), c.decimals, &c.token, crate::house::coin::Form::Short)], sub)
                    }
                    many => (many.iter().map(|x| (x.clone(), String::new())).collect(), sub),
                }
            }
            Some(Err(e)) if super::unread(&e.name) => (vec![(t(crate::house::i18n::Key::HomeNoFigure), String::new())], super::unread_word(syncing)),
            _ => (vec![(unsynced.clone(), String::new())], String::new()),
        };
        let mut go: Option<Option<Page>> = None;
        let mut row: Option<String> = None;
        let since = self.hx.page_at.1;
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 0)), since.to_bits(), 0, |ui| {
            eravon_ui::grid::tiles(ui, "tiles-1", 3, |ui, i| {
                let hit = match i {
                    0 => card::tile(ui, &card::Tile { title: &t(crate::house::i18n::Key::OrdersActive), figure: &num(&h.active), mark: None, figure_colour: None, sub: &if unread_of(&h.active) { super::unread_word(syncing) } else { format!("{} {}", eravon_home::notes::orders_to_handle(h.notes.as_ref().and_then(|r| r.as_deref().ok()).unwrap_or_default(), &h.marks), t(crate::house::i18n::Key::HomeTodo)) }, mono_figure: false, clickable: true }).clicked(),
                    1 => {
                        let (title, figure) = super::money_tile(ui, &t(crate::house::i18n::Key::PlStepBalance), &amounts);
                        let r = card::tile(ui, &card::Tile { title: &title, figure: &figure, mark: None, figure_colour: None, sub: &eth, mono_figure: false, clickable: true });
                        if cut || figure.ends_with('…') { eravon_ui::layer::tip(r, whole.join("\n")) } else { r }.clicked()
                    }
                    _ => card::tile(ui, &card::Tile { title: &t(crate::house::i18n::Key::Done), figure: &num(&h.done), mark: None, figure_colour: None, sub: &t(crate::house::i18n::Key::OrdersWordLapsed), mono_figure: false, clickable: true }).clicked(),
                };
                if hit {
                    go = Some(Some(if i == 1 { Page::Wallet } else { Page::Orders }));
                    if i != 1 {
                        self.shell.desk.sh.buyer_orders.get_or_insert_with(Default::default).done_tab = i == 2;
                    }
                }
            });
        });
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 1)), since.to_bits(), 1, |ui| {
            let heads = [t(crate::house::i18n::Key::IdSeatAuthor), t(crate::house::i18n::Key::PlTitle), t(crate::house::i18n::Key::OrdersColDeadline), t(crate::house::i18n::Key::OrdersAmount), t(crate::house::i18n::Key::BoColTurn), t(crate::house::i18n::Key::SetZkState)];
            let auto = |min: f32| table::Col::Auto { min, max: f32::MAX };
            let cols = [
                table::col(&heads[0], auto(eravon_ui::house::COL_HOME_WHO)),
                table::col(&heads[1], eravon_ui::house::REST),
                table::col(&heads[2], auto(eravon_ui::house::COL_HOME_DEADLINE)),
                table::col(&heads[4], auto(eravon_ui::house::COL_PILL)),
                table::col(&heads[5], super::STATE_COL),
                table::col_r(&heads[3], auto(eravon_ui::house::COL_HOME_AMOUNT)),
            ];
            let generation = self.shell.desk.rows_gen;
            let make = |i: usize| {
                    let r = &h.orders[i];
                    let name = if r.product.is_empty() { crate::house::orders::row_label(&r.row) } else { r.product.clone() };
                    let (say, tone) = match (super::buyer_orders::pre_state(r, lang), r.row.status.as_ref()) {
                        (Some(x), _) => x,
                        (None, Ok(st)) => (crate::house::i18n::order_word(lang, st.word), bh_tone(st.word)),
                        (None, Err(e)) => super::orders::unread_say(lang, e),
                    };
                    let turn = r.row.status.as_ref().ok().map(|st| bh_turn(st.word, &t)).unwrap_or_default();
                    let cells = vec![
                        table::Cell::Text(eravon_orders::table::short(&r.seller)),
                        table::Cell::Text(name),
                        table::Cell::Mono(super::day_or(r.row.deadline)),
                        table::Cell::Text(turn),
                        table::Cell::Pill(say, tone),
                        table::Cell::Amount(crate::house::coin::amount_tip(lang, &r.row.amount, r.row.decimals, &r.row.token)),
                    ];
                    table::Row { cells, click: true, gone: false, on: false }
            };
            card::section(ui, &t(crate::house::i18n::Key::NavOrders), "", |ui| {
                let none = t(crate::house::i18n::Key::HomeNone);
                if let Some(i) = table::table_with(ui, "bhome-orders", table::Spec::new(&cols, generation).head(true).empty(&none), h.orders.len(), make).clicked {
                    row = Some(h.orders[i].row.instance.clone());
                }
            });
        });
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 2)), since.to_bits(), 2, |ui| self.buyer_activity(ui, &h));
        if let Some(i) = row {
            self.step(Step::HomeOrder, &[("i", &i)]);
        } else if let Some(p) = go {
            self.house_go(p);
        }
    }

    fn buyer_activity(&mut self, ui: &mut egui::Ui, h: &crate::house::buyer_home::BHome) {
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let cols = [table::col("", table::Col::Px(eravon_ui::house::COL_MARK)), table::col("", eravon_ui::house::REST), table::col_r("", table::Col::Px(eravon_ui::house::COL_TIME))];
        let generation = self.shell.desk.rows_gen;
        card::section(ui, &t(crate::house::i18n::Key::HomeActivity), "", |ui| match &h.activity {
            Some(Ok(v)) => {
                let make = |i: usize| {
                    let a = &v[i];
                    match &a.unread {
                        Some((n, d)) => table::Row { cells: vec![table::Cell::Mark(mark::Mark::Bad), table::Cell::Text(format!("{} {}{}", crate::house::orders::order_label(&a.order), t(crate::house::i18n::Key::HomeActUnread), crate::house::refusal::line(lang, n, d))), table::Cell::Empty], ..Default::default() },
                        None => table::Row { cells: vec![table::Cell::Mark(mark::Mark::Ok), table::Cell::Text(format!("{} {}", crate::house::orders::order_label(&a.order), crate::house::i18n::library(lang, &a.say))), table::Cell::Mono(super::short_at(a.at))], click: !a.tx.is_empty(), gone: false, on: false },
                    }
                };
                let none = t(crate::house::i18n::Key::HomeNone);
                if let Some(i) = table::table_with(ui, "bhome-activity", table::Spec::new(&cols, generation).empty(&none), v.len(), make).clicked {
                    let a = &v[i];
                    self.shell.desk.act_open = Some((format!("{} {}", crate::house::orders::order_label(&a.order), crate::house::i18n::library(lang, &a.say)), a.at, a.tx.clone()));
                }
            }
            Some(Err(e)) if super::unread(&e.name) => super::unread_block(ui, syncing, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
            Some(Err(e)) => states::err_box(ui, "bhome-activity", &t(crate::house::i18n::Key::SyncNever), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail)),
            None => eravon_ui::states::skeleton_lines(ui, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
        });
        self.activity_sheet(ui.ctx());
    }

    pub(super) fn buyer_alerts_page(&mut self, ui: &mut egui::Ui) {
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(h) = self.bhome_now() else {
            Self::home_skeleton(ui, 0);
            return;
        };
        if h.stopped {
            eravon_ui::states::banner(ui, eravon_ui::states::Banner::Warn, &t(crate::house::i18n::Key::PinLockedTitle), |_| {});
            return;
        }
        let mut handled: Option<String> = None;
        let mut jump: Option<super::NoteGo> = None;
        let since = self.hx.page_at.1;
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 0)), since.to_bits(), 0, |ui| match &h.notes {
            Some(Ok(v)) => {
                let (h_, j_) = super::home::notes_list(ui, v, &h.marks, "bh.item");
                handled = h_;
                jump = j_;
            }
            Some(Err(e)) if super::unread(&e.name) => super::unread_block(ui, syncing, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
            Some(Err(e)) => states::err_box(ui, "balerts", &t(crate::house::i18n::Key::SyncNever), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail)),
            None => eravon_ui::states::skeleton_lines(ui, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
        });
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 1)), since.to_bits(), 1, |ui| self.buyer_activity(ui, &h));
        if let Some(g @ (super::NoteGo::Goods | super::NoteGo::Account)) = jump {
            self.note_jump(g);
        } else if let Some(super::NoteGo::Order(i)) = jump {
            self.step(Step::HomeOrder, &[("i", &i)]);
        }
        if let Some(key) = handled {
            self.step(Step::NoteHandle, &[("key", &key)]);
        }
    }

    pub(super) fn wallet_page(&mut self, ui: &mut egui::Ui) {
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(w) = self.wallet_now() else {
            Self::home_skeleton(ui, 2);
            return;
        };
        let can_send = self.write_banner(ui, crate::house::shell::Write::Chain);
        if let Some(e) = &w.fill_said {
            said_hint(ui, "buyer_home-said-1", lang, &e.name, &e.detail, true);
        }
        let busy = self.shell.house_in_flight(crate::task::Kind::Wallet);
        let mut sheet = false;
        let mut receive = false;
        let mut open_tx: Option<String> = None;
        let key_name = match self.shell.identities.as_ref().and_then(|r| r.now().map(|(x, _)| x.label.clone())).filter(|l| !l.trim().is_empty()) {
            Some(l) => l,
            None => short(&w.me),
        };
        match &w.balances {
            Some(Ok(b)) => {
                let mut cols: Vec<(String, String)> = b
                    .coins
                    .iter()
                    .filter(|c| c.have.as_ref().is_ok_and(|x| x != "0"))
                    .map(|c| (crate::house::coin::name(lang, &c.token), crate::house::coin::figure(lang, c.have.as_deref().unwrap_or("0"), c.decimals, crate::house::coin::Form::Short)))
                    .collect();
                if cols.is_empty() {
                    cols.push((t(crate::house::i18n::Key::GoodsToken), "0".to_string()));
                }
                let eth = b.native.as_ref().map(|x| crate::house::coin::figure(lang, x, Some(u64::from(eravon_literals::constants::NATIVE_DECIMALS)), crate::house::coin::Form::Short)).unwrap_or_else(|_| t(crate::house::i18n::Key::SyncNever));
                cols.push((t(crate::house::i18n::Key::WlEth), eth));
                let failed: Vec<String> = b.coins.iter().filter(|c| c.have.is_err()).map(|c| if c.symbol.is_empty() { super::short(&c.token) } else { c.symbol.clone() }).collect();
                let exact = b.native.as_ref().map(|x| crate::house::coin::eth(lang, x, crate::house::coin::Form::Full)).unwrap_or_else(|_| t(crate::house::i18n::Key::SyncNever));
                let sub = format!("{} · {key_name}", t(crate::house::i18n::Key::WlCurrent));
                card::form(ui, |ui, f| {
                    f.summary(ui, Glyph::Vault, card::Tint::Blue, &t(crate::house::i18n::Key::PlStepBalance), None, &sub, |ui| {
                        receive = key::key(ui, &t(crate::house::i18n::Key::WlReceive), Role::Secondary, true).clicked();
                        sheet = key::key(ui, &t(crate::house::i18n::Key::WlSend), Role::Guide, !busy && can_send).clicked();
                    });
                    let shown: Vec<(&str, card::Fact)> = cols.iter().take(3).map(|(n, v)| (n.as_str(), card::Fact::Text(v.as_str()))).collect();
                    f.facts(ui, &shown);
                    for (n, v) in cols.iter().skip(3) {
                        f.info(ui, n, card::Show::Text(v));
                    }
                    if !failed.is_empty() {
                        f.info(ui, &t(crate::house::i18n::Key::WlUnread), card::Show::Mark(mark::Mark::Warn, &failed.join(&t(crate::house::i18n::Key::ListJoin))));
                    }
                    details_row(ui, f, "wl-detail", &[(&t(crate::house::i18n::Key::Address), Val::mono(w.me.clone())), (&t(crate::house::i18n::Key::WlEth), Val::mono(exact.clone()))]);
                });
            }
            Some(Err(e)) if super::unread(&e.name) => super::unread_block(ui, syncing, &[eravon_ui::house::SKEL_FIG, eravon_ui::house::SKEL_MID]),
            Some(Err(e)) => states::err_box(ui, "wl-bal", &t(crate::house::i18n::Key::SyncNever), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail)),
            None => eravon_ui::states::skeleton_lines(ui, &[eravon_ui::house::SKEL_FIG, eravon_ui::house::SKEL_MID]),
        }
        self.formers_section(ui, &w, busy, can_send);
        let heads = [t(crate::house::i18n::Key::WlColTime), t(crate::house::i18n::Key::OrdersOrder), t(crate::house::i18n::Key::WlColKind), t(crate::house::i18n::Key::OrdersAmount)];
        let generation = self.shell.desk.rows_gen;
        card::section(ui, &t(crate::house::i18n::Key::ExRows), "", |ui| match &w.txs {
            Some(Ok(v)) => {
                let amount = |x: &eravon_wallet::tx::Tx| {
                    let dec = match &w.balances {
                        Some(Ok(b)) => b.coin(&x.token).and_then(|c| c.decimals),
                        _ => None,
                    };
                    if x.amount.is_empty() { eravon_ui::layer::Amount::new("—", "—") } else if x.token.is_empty() { crate::house::coin::eth_tip(lang, &x.amount) } else { crate::house::coin::amount_tip(lang, &x.amount, dec, &x.token) }
                };
                let cols = [table::col(&heads[0], table::Col::Px(eravon_ui::house::COL_TIME)), table::col(&heads[1], eravon_ui::house::REST), table::col(&heads[2], table::Col::Px(eravon_ui::house::COL_KIND)), table::col_r(&heads[3], table::Col::Auto { min: eravon_ui::house::COL_AMOUNT, max: f32::MAX }), table::CHEV];
                let make = |i: usize| {
                    let x = &v[i];
                    let kind = crate::house::wallet::kind_words(lang, &x.kind);
                    let order = if x.order.is_empty() { "—".to_string() } else { super::short(&x.order) };
                    table::Row { cells: vec![table::Cell::Mono(super::short_at(x.at)), table::Cell::Text(order), table::Cell::Text(kind), table::Cell::Amount(amount(x)), table::Cell::Chev], click: true, gone: false, on: false }
                };
                let none = t(crate::house::i18n::Key::HomeNone);
                if let Some(i) = table::table_with(ui, "wl-txs", table::Spec::new(&cols, generation).head(true).empty(&none), v.len(), make).clicked {
                    open_tx = Some(v[i].hash.clone());
                }
            }
            Some(Err(e)) if super::unread(&e.name) => super::unread_block(ui, syncing, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
            Some(Err(e)) => states::err_box(ui, "wl-txs", &t(crate::house::i18n::Key::SyncNever), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail)),
            None => eravon_ui::states::skeleton_lines(ui, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
        });
        if w.open_tx.is_some() {
            self.tx_sheet(ui.ctx(), &w);
        }
        if receive || self.shell.desk.wl_receive {
            self.shell.desk.wl_receive = true;
            self.receive_sheet(ui.ctx(), &w, crate::house::i18n::Key::WlReceive);
        }
        if w.sheet {
            self.wallet_sheet(ui.ctx(), &w, false);
        }
        if sheet {
            self.step(Step::WlSheet, &[("open", "true")]);
        }
        if let Some(h) = open_tx {
            self.step(Step::WlTx, &[("hash", &h)]);
        }
    }

    pub(super) fn formers_section(&mut self, ui: &mut egui::Ui, w: &crate::house::wallet::Wallet, busy: bool, can_send: bool) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let mut move_from: Option<String> = None;
        if !w.formers.is_empty() {
            card::section(ui, &t(crate::house::i18n::Key::WlFormerHead), "", |ui| {
                card::form(ui, |ui, f| {
                    for b in &w.formers {
                        let unread = |name: String, f: &eravon_wallet::fault::Fault| format!("{name} {}", crate::house::refusal::line(lang, &f.token, &f.detail));
                        let mut held: Vec<String> = b
                            .coins
                            .iter()
                            .filter_map(|c| match &c.have {
                                Ok(x) if x == "0" => None,
                                Ok(x) => Some(format!("{} {}", crate::house::coin::figure(lang, x, c.decimals, crate::house::coin::Form::Short), crate::house::coin::name(lang, &c.token))),
                                Err(f) => Some(unread(crate::house::coin::name(lang, &c.token), f)),
                            })
                            .collect();
                        match &b.native {
                            Ok(x) if x == "0" => {}
                            Ok(x) => held.push(format!("{} {}", crate::house::coin::figure(lang, x, Some(u64::from(eravon_literals::constants::NATIVE_DECIMALS)), crate::house::coin::Form::Short), t(crate::house::i18n::Key::WlEth))),
                            Err(f) => held.push(unread(t(crate::house::i18n::Key::WlEth), f)),
                        }
                        let label = t(crate::house::i18n::Key::WlFormerMove);
                        let k = key::Key::new(&label, Role::Secondary).enabled(!busy && can_send);
                        if f.act(ui, &short(&b.address), &held.join(&t(crate::house::i18n::Key::ListJoin)), k).clicked() {
                            move_from = Some(b.address.clone());
                        }
                    }
                });
            });
        }
        if let Some(a) = move_from {
            self.step(Step::WlFormerMove, &[("a", &a)]);
        }
    }

    fn tx_sheet(&mut self, ctx: &egui::Context, w: &crate::house::wallet::Wallet) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(x) = w.open_tx.as_ref().and_then(|h| w.tx(h)).cloned() else { return };
        let out = eravon_ui::sheet::show(ctx, eravon_ui::sheet::Spec::new("wl-tx", eravon_ui::metrics::MODAL_W), &mut (), |ui, _| {
            let kind = crate::house::wallet::kind_words(lang, &x.kind);
            let order = if x.order.is_empty() { "—".to_string() } else { super::short(&x.order) };
            eravon_ui::sheet::title(ui, &format!("{kind} · {order}"), "");
            paint::text(ui, &eravon_orders::when::minute(x.at), eravon_ui::tokens::Type::Mono, c(C::Ink2));
            let st = match x.status.as_str() {
                "1" => t(crate::house::i18n::Key::WlMined),
                "0" => t(crate::house::i18n::Key::WlReverted),
                _ => t(crate::house::i18n::Key::OrdersWordAwaitConfirm),
            };
            let dec = match &w.balances {
                Some(Ok(b)) => b.coin(&x.token).and_then(|c| c.decimals),
                _ => None,
            };
            let amt = if x.amount.is_empty() { "—".to_string() } else if x.token.is_empty() { crate::house::coin::eth(lang, &x.amount, crate::house::coin::Form::Full) } else { crate::house::coin::amount(lang, &x.amount, dec, &x.token, crate::house::coin::Form::Full) };
            let fee = if x.fee.is_empty() { t(crate::house::i18n::Key::OrdersWordAwaitConfirm) } else { crate::house::coin::eth(lang, &x.fee, crate::house::coin::Form::Full) };
            kv::kv(ui, &[(&t(crate::house::i18n::Key::OrdersAmount), kv::Val::Text(amt)), (&t(crate::house::i18n::Key::WlFee), kv::Val::Mono(fee)), (&t(crate::house::i18n::Key::SetZkState), kv::Val::Text(st))]);
            eravon_ui::fold::fold(ui, "wl-tx-detail", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                kv::kv(ui, &[(&t(crate::house::i18n::Key::WlHash), kv::Val::Mono(x.hash.clone())), (&t(crate::house::i18n::Key::WlFrom), kv::Val::Mono(x.from.clone()))]);
            });
        }, |ui, _| {
            let close = key::key(ui, &t(crate::house::i18n::Key::ToastClose), Role::Primary, true).clicked();
            if key::key(ui, &t(crate::house::i18n::Key::WlCopyHash), Role::Secondary, true).clicked() {
                ui.ctx().copy_text(x.hash.clone());
                tell_copied(ui.ctx(), Some(x.hash.clone()));
            }
            close
        });
        if out.foot || out.esc {
            if let Some(v) = self.shell.desk.sh.wallet.as_mut() {
                v.open_tx = None;
            }
            self.shell.desk.flush();
        }
    }

    pub(super) fn receive_sheet(&mut self, ctx: &egui::Context, w: &crate::house::wallet::Wallet, head: crate::house::i18n::Key) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let (title, key, scan, copy, close_word) = (t(head), t(crate::house::i18n::Key::Address), String::new(), t(crate::house::i18n::Key::IdDoCopy), t(crate::house::i18n::Key::ToastClose));
        let me = w.me.clone();
        let out = eravon_ui::sheet::show(
            ctx,
            eravon_ui::sheet::Spec::new("wl-receive", eravon_ui::house::address_face_w(ctx, qr_plate_w())),
            &mut (),
            |ui, _| {
                eravon_ui::sheet::title(ui, &title, "");
                let code = format!("ethereum:{me}");
                eravon_ui::house::address_face(ui, &key, &me, &scan, qr_plate_w(), |ui| qr_plate(ui, &code));
            },
            |ui, _| {
                let close = key::key(ui, &close_word, Role::Primary, true).clicked();
                let copied = key::key(ui, &copy, Role::Secondary, true).clicked();
                (close, copied)
            },
        );
        let (close, copied) = out.foot;
        if copied {
            ctx.copy_text(w.me.clone());
            tell_copied(ctx, Some(w.me.clone()));
            self.step(Step::WlCopy, &[("item", "address")]);
        }
        if close || out.esc {
            self.shell.desk.wl_receive = false;
        }
    }

    pub(super) fn wallet_sheet(&mut self, ctx: &egui::Context, w: &crate::house::wallet::Wallet, seller: bool) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let choices = if seller { seller_choices(w) } else { buyer_choices() };
        let (mut pick, mut amount, mut to) = self.shell.desk.wl_form.clone();
        let mut again = self.shell.desk.wl_again.clone();
        let differ = seller && !again.trim().is_empty() && !again.trim().eq_ignore_ascii_case(to.trim());
        let mut again_focus = false;
        if seller && self.shell.desk.wl_again_focus {
            let pasted = ctx.input_mut(|i| {
                let had = i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)));
                i.events.retain(|e| !matches!(e, egui::Event::Paste(_)));
                had
            });
            if pasted {
                self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: t(crate::house::i18n::Key::WlNoPaste), next: String::new(), detail: None });
            }
        }
        pick = pick.min(choices.len().saturating_sub(1));
        let busy = self.shell.house_in_flight(crate::task::Kind::Wallet);
        let picked = choices.get(pick).and_then(|(_, x)| x.clone());
        let decimals_of = |tok: &Option<String>| match (&w.balances, tok) {
            (_, None) => Some(eravon_literals::constants::NATIVE_DECIMALS),
            (Some(Ok(b)), Some(t)) => b.coin(t).and_then(|c| c.decimals).and_then(|d| u8::try_from(d).ok()),
            _ => None,
        };
        let can = w.all.as_ref().filter(|(tok, _)| *tok == picked).map(|(_, got)| got.clone());
        if self.shell.desk.wl_all_fill && !busy {
            if let (Some(Ok(raw)), Some(d)) = (&can, decimals_of(&picked)) {
                amount = eravon_literals::units::units_text(raw, d, false);
            }
            self.shell.desk.wl_all_fill = false;
        }
        let mut all_pressed = false;
        let all_word = t(crate::house::i18n::Key::WlMax);
        let (_, send, close) = Self::sheet_with(ctx, "wl-send", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::WlSend), "");
            field(ui, &t(crate::house::i18n::Key::SetSecTokens), None, |ui| {
                let refs: Vec<&str> = choices.iter().map(|(n, _)| n.as_str()).collect();
                let rows: Vec<eravon_ui::menu::Item> = refs.iter().enumerate().map(|(i, l)| eravon_ui::menu::Item::Row(eravon_ui::menu::Row { label: l, check: Some(i == pick), ..Default::default() })).collect();
                if let Some(i) = eravon_ui::menu::menu_key(ui, "wl-coin", refs.get(pick).copied().unwrap_or_default(), false, eravon_ui::house::DD_MENU_MIN_W, &rows).filter(|i| *i != pick) {
                    pick = i;
                }
            });
            field(ui, &t(crate::house::i18n::Key::OrdersAmount), None, |ui| {
                let unit = choices.get(pick).map(|(n, _)| n.clone()).unwrap_or_default();
                let (_, all) = width::then(
                    ui,
                    |ui| {
                        let all = key::key(ui, &all_word, Role::Secondary, !busy).clicked();
                        paint::text(ui, &unit, Type::Note, c(C::Ink2));
                        all
                    },
                    |ui, room| input::line_w(ui, &mut amount, "0", room),
                );
                all_pressed |= all;
            });
            field(ui, &t(crate::house::i18n::Key::WlTo), None, |ui| {
                input::field(ui, &mut to, "0x…", ui.available_width(), input::Look { mono: true, ..input::Look::default() });
            });
            if seller {
                field(ui, &t(crate::house::i18n::Key::WlToAgain), None, |ui| {
                    again_focus = input::field(ui, &mut again, "0x…", ui.available_width(), input::Look { mono: true, bad: differ, ..input::Look::default() }).has_focus();
                    if differ {
                        states::hint_ex(ui, &t(crate::house::i18n::Key::ErrEP7AGAIN), true);
                    }
                });
            }
            let token = choices.get(pick).and_then(|(_, x)| x.clone());
            let (have, decimals) = match (&w.balances, &token) {
                (Some(Ok(b)), None) => (b.native.as_ref().ok().cloned(), Some(u64::from(eravon_literals::constants::NATIVE_DECIMALS))),
                (Some(Ok(b)), Some(tok)) => b.coin(tok).map_or((None, None), |c| (c.have.as_ref().ok().cloned(), c.decimals)),
                _ => (None, None),
            };
            let word = |raw: &str, form| match &token {
                None => crate::house::coin::eth(lang, raw, form),
                Some(tok) => crate::house::coin::amount(lang, raw, decimals, tok, form),
            };
            let empty = have.as_deref().is_none_or(|h| h == "0");
            if empty {
                states::hint(ui, &t(crate::house::i18n::Key::WlNoBalance));
            }
            if let Some(h) = &have {
                kv::kv(ui, &[(&t(crate::house::i18n::Key::WlHave), kv::Val::Text(word(h, crate::house::coin::Form::Short)))]);
            }
            match &can {
                Some(Ok(raw)) => {
                    kv::kv(ui, &[(&t(crate::house::i18n::Key::WlCanSend), kv::Val::Text(word(raw, crate::house::coin::Form::Full)))]);
                    if token.is_none() {
                        states::hint(ui, &t(crate::house::i18n::Key::WlFeeAside));
                    }
                }
                Some(Err(e)) => said_hint(ui, "buyer_home-said-2", lang, &e.name, &e.detail, false),
                None => {}
            }
            Self::sheet_said(ui, "wl-send-said", w.said.as_ref());
            let ready = !busy && !empty && !amount.trim().is_empty() && !to.trim().is_empty() && (!seller || (!again.trim().is_empty() && !differ));
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::WlSend), Role::Commit, ready)));
        });
        self.shell.desk.wl_form = (pick, amount.clone(), to.clone());
        self.shell.desk.wl_again = again.clone();
        self.shell.desk.wl_again_focus = again_focus;
        let token = choices.get(pick).and_then(|(_, t)| t.clone()).unwrap_or_default();
        if close {
            self.step(Step::WlSheet, &[("open", "false")]);
        } else if all_pressed {
            self.step(Step::WlAll, &[("token", &token), ("to", to.trim())]);
        } else if send && seller {
            self.step(Step::WlSend, &[("token", &token), ("amount", amount.trim()), ("to", to.trim()), ("again", again.trim())]);
        } else if send {
            self.step(Step::WlSend, &[("token", &token), ("amount", amount.trim()), ("to", to.trim())]);
        }
    }
}

fn buyer_choices() -> Vec<(String, Option<String>)> {
    let mut choices: Vec<(String, Option<String>)> = vec![(eravon_literals::constants::NATIVE_SYMBOL.to_string(), None)];
    for r in eravon_vault::tokens::on_chain(&crate::house::coin::rows(), eravon_vault::tokens::chain()) {
        choices.push((r.label.clone(), Some(r.address.clone())));
    }
    choices
}

pub(super) fn seller_choices(w: &crate::house::wallet::Wallet) -> Vec<(String, Option<String>)> {
    let lang = crate::lang::lang();
    let mut choices: Vec<(String, Option<String>)> = match &w.balances {
        Some(Ok(b)) => b.coins.iter().filter(|c| c.have.as_ref().is_ok_and(|x| x != "0")).map(|c| (crate::house::coin::name(lang, &c.token), Some(c.token.clone()))).collect(),
        _ => Vec::new(),
    };
    choices.push((eravon_literals::constants::NATIVE_SYMBOL.to_string(), None));
    choices
}

fn bh_tone(w: eravon_orders::status::Word) -> eravon_ui::palette::Tone {
    use eravon_orders::status::Word;
    match w {
        _ if w.done() => eravon_ui::palette::Tone::Grey,
        Word::Disputed | Word::AwaitRedraw => eravon_ui::palette::Tone::Warn,
        _ => eravon_ui::palette::Tone::Ok,
    }
}

fn bh_turn(w: eravon_orders::status::Word, t: &dyn Fn(crate::house::i18n::Key) -> String) -> String {
    use eravon_orders::status::Word;
    match w {
        Word::AwaitConfirm => t(crate::house::i18n::Key::BoTurnYou),
        Word::AwaitDelivery | Word::AwaitAccept => t(crate::house::i18n::Key::IdSeatAuthor),
        Word::Disputed => t(crate::house::i18n::Key::PlCands),
        Word::AwaitRedraw => t(crate::house::i18n::Key::BoTurnAnyone),
        _ => String::new(),
    }
}
