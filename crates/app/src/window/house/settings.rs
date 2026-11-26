use super::*;
use crate::house::cards::SetCard;
use crate::house::settings::{Settings, ZkState};

impl Win {
    fn house_settings(&mut self) -> Option<Settings> {
        if self.shell.desk.sh.seller.settings.is_none() {
            self.shell.open_settings();
        }
        self.shell.desk.sh.seller.settings.clone()
    }

    fn house_section_at(&mut self, s: &Settings, name: &str) {
        if s.section.map(|x| x.name()) != Some(name) {
            self.step(Step::SetSection, &[("s", name)]);
        }
    }

    pub(in crate::window) fn house_section(&mut self, ui: &mut egui::Ui, sec: crate::nav::Own, _now: f64) {
        let Some(s) = self.house_settings() else { return };
        self.house_section_at(&s, sec.as_str());
        let busy = self.shell.house_in_flight(crate::task::Kind::Settings);
        let live = self.shell.unlocked();
        let (mut open, mut act) = (None, None);
        match sec {
            crate::nav::Own::Opening => self.set_opening(ui),
            crate::nav::Own::Company => self.set_company(ui, &s, busy, &mut open, &mut act),
            crate::nav::Own::Tokens => self.set_tokens(ui, &s, busy, live, &mut open, &mut act),
            crate::nav::Own::Zikaron => self.set_zikaron(ui, &s, live, &mut open, &mut act),
        }
        self.house_settle(ui.ctx(), open, act);
    }

    pub(in crate::window) fn house_gas_fact(&self) -> Option<(eravon_ui::mark::Mark, eravon_ui::layer::Amount)> {
        use eravon_ui::mark::Mark;
        let s = self.shell.desk.sh.seller.settings.as_ref()?;
        let lang = crate::lang::lang();
        let t = move |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let eth = |wei: &str| crate::house::coin::eth(lang, wei, crate::house::coin::Form::Full);
        let empty = |wei: &str| wei.trim_start_matches('0').is_empty();
        let (mark, said) = match (&s.gas, &s.gas_book) {
            (Some(Ok((wei, _, _))), _) => (if empty(wei) { Mark::Bad } else { Mark::Ok }, eth(wei)),
            (Some(Err(e)), _) => (Mark::Bad, crate::house::refusal::line(lang, &e.name, &e.detail)),
            (None, Some((wei, _, at))) => {
                let stale = crate::house::book::stale(*at, crate::house::desk::wall_now());
                (if stale { Mark::Warn } else if empty(wei) { Mark::Bad } else { Mark::Ok }, eth(wei))
            }
            (None, None) => (Mark::Todo, t(crate::house::i18n::Key::SetNotRead)),
        };
        Some((mark, eravon_ui::layer::Amount::new(said.clone(), said)))
    }

    pub(in crate::window) fn house_gas_line(&self) -> Option<String> {
        let s = self.shell.desk.sh.seller.settings.as_ref()?;
        let lang = crate::lang::lang();
        let t = move |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let read = match (&s.gas, &s.gas_book) {
            (Some(Ok((_, a, n))), _) => format!("{} · {}", t(if a == "agreed" { crate::house::i18n::Key::SyncAgreed } else { crate::house::i18n::Key::SyncSingle }), crate::fill!(crate::house::i18n::Key::SetNodesN; n.to_string())),
            (None, Some((_, a, at))) => {
                let mut line = crate::fill!(crate::house::i18n::Key::SyncReadAt; eravon_orders::when::hm(*at));
                if a == "single" {
                    line.push_str(&format!(" · {}", t(crate::house::i18n::Key::SyncSingle)));
                }
                if crate::house::book::stale(*at, crate::house::desk::wall_now()) {
                    line.push_str(&format!(" · {}", t(crate::house::i18n::Key::SyncStale)));
                }
                line
            }
            _ => return None,
        };
        Some(format!("{} {read}", t(crate::house::i18n::Key::IdGas)))
    }

    pub(in crate::window) fn house_primary_how(&self, primary: Option<&crate::identity::Row>) -> Option<(String, String)> {
        let p = primary?;
        let (cur, _) = self.shell.identities.as_ref()?.now()?;
        if !p.id.eq_ignore_ascii_case(&cur.id) {
            return None;
        }
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        let how = match self.shell.primary.as_ref()?.1 {
            crate::keybox::PrimaryKind::Words => t(crate::house::i18n::Key::IdPrimaryByWords),
            crate::keybox::PrimaryKind::KeyFile => t(crate::house::i18n::Key::IdPrimaryByFile),
        };
        Some((how, String::new()))
    }

    pub(in crate::window) fn house_open_settings(&mut self) {
        if matches!(self.ux.nav.place, Some(Place::Settings(_))) {
            let _ = self.house_settings();
        }
    }

    pub(in crate::window) fn house_keys(&mut self, ui: &mut egui::Ui, now: f64) {
        let mut act = None;
        let live = self.shell.unlocked();
        let ask_gas = live && !self.shell.page_in_flight(crate::task::Kind::Settings) && self.shell.desk.sh.seller.settings.as_ref().is_some_and(|s| s.gas.is_none() && !s.gas_asked && !s.me.is_empty());
        if ask_gas {
            if let Some(s) = self.shell.desk.sh.seller.settings.as_mut() {
                s.gas_asked = true;
            }
            act = Some(Act::of(Step::SetGas));
        }
        self.house_settle(ui.ctx(), None, act);
        self.house_consent(ui.ctx(), now);
    }

    pub(in crate::window) fn house_rotate_open(&mut self) {
        self.run(Act::of(Step::SetRotate));
    }

    pub(in crate::window) fn house_notify(&mut self, ui: &mut egui::Ui, now: f64) {
        let stored = if self.shell.refresh_every == 0 { String::new() } else { self.shell.refresh_every.to_string() };
        let check = |x: &str| if crate::house::general::every_parse(x).is_some() { Ok(()) } else { Err(t(Key::U3EveryMustBeNumber)) };
        let key_w = key::size_of(ui, &key::Key::new(t(Key::ReadNetSave), Role::Secondary)).x.max(92.0);
        let words = card::SettingWords { save: t(Key::ReadNetSave), saved: t(Key::SettingSaved), unit: t(Key::SetEverySecs), hint: t(Key::SetEveryOff), key_w };
        let mut save = None;
        let edit = &mut self.hx.every;
        stagger(ui, 0, |ui| {
            card::form(ui, |ui, f| {
                save = f.setting(ui, t(Key::SetRefresh), edit, &stored, 110.0, words, check);
            });
        });
        if let Some(x) = save {
            if !matches!(self.act(Action::SetRefreshEvery { typed: x }, now), Applied::Trouble(_)) {
                self.hx.every.land();
            }
        }
    }

    pub(in crate::window) fn house_network(&mut self, ui: &mut egui::Ui, _now: f64) {
        let Some(s) = self.house_settings() else { return };
        let live = self.shell.unlocked();
        let mut act = None;
        self.set_net_extras(ui, &s, live, &mut act);
        self.house_settle(ui.ctx(), None, act);
    }

    pub(in crate::window) fn house_version_rows(&self) -> Vec<(String, kv::Val)> {
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        let (_, lock) = crate::house::settings::about();
        let network = crate::house::i18n::get_named(crate::lang::lang(), eravon_literals::constants::current_network().word).0;
        vec![(t(crate::house::i18n::Key::SetNetwork), kv::Val::text(network)), (t(crate::house::i18n::Key::SetBaseLock), kv::Val::mono(short(&lock)))]
    }

    fn house_settle(&mut self, ctx: &egui::Context, open: Option<SetCard>, act: Option<Act>) {
        if let Some(c) = open {
            if let Some(s) = self.shell.desk.sh.seller.settings.as_mut() {
                s.said = None;
            }
            self.shell.desk.set_card = Some(c);
        }
        if let Some(a) = act {
            self.run(a);
        }
        self.house_set_card(ctx);
    }

    fn token_sheet(&mut self, ctx: &egui::Context) {
        let Some(s) = self.shell.desk.sh.seller.settings.clone() else { return };
        let Some(sh) = s.token_sheet.clone() else { return };
        let Some(r) = eravon_vault::tokens::find(&s.tokens, eravon_vault::tokens::chain(), &sh.address).cloned() else { return };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let kind = t(if eravon_vault::tokens::builtin(&r) { crate::house::i18n::Key::SetTokensBuiltin } else { crate::house::i18n::Key::SetTokensCustom });
        let manual = matches!(r.source.as_str(), eravon_vault::tokens::FROM_MANUAL | eravon_vault::tokens::FROM_PENDING);
        let out = eravon_ui::sheet::show(ctx, eravon_ui::sheet::Spec::new("set-token-sheet", eravon_ui::metrics::MODAL_W), &mut (), |ui, _| {
            Self::sheet_head(ui, &r.label, &kind);
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            let used = match &sh.used {
                Ok(v) if v.is_empty() => kv::Val::Text(t(crate::house::i18n::Key::SetTokensUnused)),
                Ok(v) => kv::Val::Mark(mark::Mark::Ok, format!("{} · {}", t(crate::house::i18n::Key::IdIsCurrent), v.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>().join(&t(crate::house::i18n::Key::ListJoin)))),
                Err(e) => kv::Val::Text(crate::house::refusal::line(lang, &e.name, &e.detail)),
            };
            let from = crate::house::i18n::get_named(lang, &format!("set.tokens.from.{}", r.source)).0;
            let decimals = match r.decimals {
                Some(d) => format!("{d} · {from}"),
                None => from,
            };
            let read = s.read_of(&r.address);
            let mut rows = vec![(t(crate::house::i18n::Key::GoodsName), kv::Val::Text(r.label.clone())), (t(crate::house::i18n::Key::WlColKind), kv::Val::Text(kind.clone())), (t(crate::house::i18n::Key::SetTokensUsed), used), (t(crate::house::i18n::Key::SetTokensDecimals), kv::Val::Text(decimals))];
            if !read.is_empty() {
                rows.push((t(crate::house::i18n::Key::SetTokensRead), kv::Val::Text(read)));
            }
            let rows: Vec<(&str, kv::Val)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            kv::kv(ui, &rows);
            ui.add_space(eravon_ui::house::FOLD_GAP);
            eravon_ui::fold::fold(ui, "set-token-detail", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                kv::kv(ui, &[(&t(crate::house::i18n::Key::SetTokensAddress), kv::Val::Mono(r.address.clone())), (&t(crate::house::i18n::Key::SetChainId), kv::Val::Mono(r.chain.to_string()))]);
            });
        }, |ui, _| {
            let close = key::key(ui, &t(crate::house::i18n::Key::ToastClose), Role::Primary, true).clicked();
            let (remove, edit) = ui
                .with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = eravon_ui::metrics::S2;
                    let remove = key::key(ui, &t(crate::house::i18n::Key::CandsRemoveDots), Role::Guide, true).clicked();
                    (remove, manual && key::key(ui, &t(crate::house::i18n::Key::SetTokensDecimalsEdit), Role::Secondary, true).clicked())
                })
                .inner;
            (remove, edit, close)
        });
        let (remove, edit, close) = (out.foot.0, out.foot.1, out.foot.2 || out.esc);
        if remove || edit || close {
            self.step(Step::SetTokenClose, &[]);
        }
        if remove {
            self.shell.desk.set_card = Some(SetCard::TokenRemove { address: r.address.clone(), label: r.label.clone() });
        } else if edit {
            self.shell.desk.set_card = Some(SetCard::TokenDecimals { address: r.address.clone(), text: r.decimals.map(|d| d.to_string()).unwrap_or_default() });
        }
    }

    fn house_set_card(&mut self, ctx: &egui::Context) {
        let Some(card) = self.shell.desk.set_card.clone() else {
            if self.shell.desk.sh.seller.settings.as_ref().is_some_and(|s| s.token_sheet.is_some()) {
                self.token_sheet(ctx);
            }
            return;
        };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let said = self.shell.desk.sh.seller.settings.as_ref().and_then(|s| s.said.clone());
        let mut next = card.clone();
        let manual_at = |a: &str| self.shell.desk.sh.seller.settings.as_ref().is_some_and(|s| s.token_form.manual_at(a));
        let manual = match &next {
            SetCard::TokenAdd { address, .. } => manual_at(address),
            _ => false,
        };
        let zk_site = "set-zk-dir";
        let busy = self.shell.house_in_flight(crate::task::Kind::Settings);
        let rotation = self.shell.desk.sh.seller.settings.as_ref().and_then(|s| s.rotate.clone());
        let mut unveil = false;
        let mut held = false;
        let (_, hit, back) = Self::sheet_pick(ctx, "set-card", eravon_ui::metrics::MODAL_W, |ui, keys| {
            match &mut next {
                SetCard::Company { addr } => {
                    card::card_title(ui, &t(crate::house::i18n::Key::SetCompanyMode));
                    field(ui, &t(crate::house::i18n::Key::SetCoAddress), None, |ui| {
                        let _ = input::field(ui, addr, "0x…", ui.available_width(), input::Look { mono: true, ..input::Look::default() });
                    });
                    Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::SetCoOpenBtn), Role::Primary, !addr.trim().is_empty())));
                }
                SetCard::Zikaron { dir } => {
                    card::card_title(ui, &t(crate::house::i18n::Key::SetChangeZk));
                    if let Some(p) = Self::sheet_picked(ui.ctx(), zk_site) {
                        *dir = p;
                    }
                    card::form(ui, |ui, f| f.folder(ui, &t(crate::house::i18n::Key::SetZkDir), dir, &t(crate::house::i18n::Key::PickNone)));
                    Self::sheet_keys_left(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::ReadNetSave), Role::Primary, true)), Some((zk_site, crate::platform::Pick::Folder)));
                }
                SetCard::TokenAdd { address, label, decimals } => {
                    Self::sheet_head(ui, &t(crate::house::i18n::Key::SetTokensAdd), "");
                    ui.add_space(eravon_ui::metrics::CARD_GAP);
                    field(ui, &t(crate::house::i18n::Key::SetTokensAddress), None, |ui| {
                        let _ = input::field(ui, address, "0x…", ui.available_width(), input::Look { mono: true, ..input::Look::default() });
                    });
                    field(ui, &t(crate::house::i18n::Key::GoodsName), None, |ui| {
                        let _ = input::line(ui, label, "");
                    });
                    if manual {
                        field(ui, &t(crate::house::i18n::Key::SetTokensDecimals), None, |ui| {
                            let _ = input::line(ui, decimals, "0–77");
                        });
                        states::hint(ui, &t(crate::house::i18n::Key::SetTokensManualHint));
                    }
                    Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::ReadNetAdd), Role::Primary, !address.trim().is_empty() && !label.trim().is_empty())));
                }
                SetCard::TokenRemove { address, label } => {
                    Self::sheet_head(ui, &crate::fill_in!(lang, crate::house::i18n::Key::SetTokensRemoveTitle; label), "");
                    ui.add_space(eravon_ui::metrics::CARD_GAP);
                    kv::kv(ui, &[(&t(crate::house::i18n::Key::SetSecTokens), kv::Val::Text(label.clone())), (&t(crate::house::i18n::Key::IdDeleteEffect), kv::Val::Text(t(crate::house::i18n::Key::SetTokensEffectSays)))]);
                    ui.add_space(eravon_ui::house::FOLD_GAP);
                    eravon_ui::fold::fold(ui, "set-token-remove-detail", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                        kv::kv(ui, &[(&t(crate::house::i18n::Key::SetTokensAddress), kv::Val::Mono(address.clone())), (&t(crate::house::i18n::Key::SetChainId), kv::Val::Mono(eravon_vault::tokens::chain().to_string()))]);
                    });
                    Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::KitRemove), Role::Commit, true)));
                }
                SetCard::TokenDecimals { text, .. } => {
                    card::card_title(ui, &t(crate::house::i18n::Key::SetTokensDecimalsEdit));
                    field(ui, &t(crate::house::i18n::Key::SetTokensDecimals), None, |ui| {
                        let _ = input::line(ui, text, "0–77");
                    });
                    Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::ReadNetSave), Role::Primary, !text.trim().is_empty())));
                }
                SetCard::Rotate { keep, let_expire, revoke } => {
                    card::card_title(ui, &t(crate::house::i18n::Key::IdRotate));
                    let mut flip = false;
                    card::form(ui, |ui, f| {
                        f.row_with(ui, None, &t(crate::house::i18n::Key::IdRotateKeep), "", card::Value::None, |ui| {
                            flip = eravon_ui::toggle::switch(ui, *keep, !busy).clicked();
                        });
                    });
                    if flip {
                        *keep = !*keep;
                    }
                    let st = |w: &str| crate::house::i18n::get_named(lang, &format!("id.rotSt.{w}")).0;
                    let mut commit = (t(crate::house::i18n::Key::IdRotate), false);
                    match &rotation {
                        None => states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO),
                        Some(Err(e)) => {
                            states::err_box(ui, "set-rotate-err", &crate::house::refusal::line(lang, &e.name, &e.detail), &crate::house::refusal::next(lang, &e.name).unwrap_or_default(), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail));
                            held = e.name == eravon_cli::vaultverbs::rotate::E_ROTATE_HELD && *keep;
                        }
                        Some(Ok(v)) => {
                            let net_of = |c: u64| net_name(lang, c);
                            let (cases, unread) = cases_said(lang, v);
                            let mut rows: Vec<(String, kv::Val)> = v
                                .chains
                                .iter()
                                .map(|(n, _, w, d, seat)| {
                                    let seat = t(if *seat == eravon_store::vault::Seat::Seller { crate::house::i18n::Key::IdSeatAuthor } else { crate::house::i18n::Key::IdSeatGrantee });
                                    let net = format!("{} · {seat}", crate::house::i18n::get_named(lang, &format!("set.chain{}", if n == "mainnet" { "Main" } else { "Sepolia" })).0);
                                    let said = match unread_why(lang, w, d) {
                                        Some(why) => format!("{} · {why}", st(w)),
                                        None => st(w),
                                    };
                                    (net, kv::Val::Text(said))
                                })
                                .collect();
                            if v.new.is_empty() && *keep {
                                let txs = if v.txs_most > v.txs { crate::house::i18n::filln(crate::house::i18n::Key::IdRotateTxsMost, &[&v.txs.to_string(), &v.txs_most.to_string()]) } else { v.txs.to_string() };
                                rows.push((t(crate::house::i18n::Key::IdRotateTxs), kv::Val::Figure(txs)));
                                rows.push((t(crate::house::i18n::Key::IdRotateCases), cases));
                                rows.push((t(crate::house::i18n::Key::IdRotateOrders), kv::Val::Figure(v.orders.to_string())));
                                rows.push((t(crate::house::i18n::Key::IdRotateOffers), kv::Val::Figure(v.offers.to_string())));
                                rows.push((t(crate::house::i18n::Key::IdRotateDeposits), kv::Val::Figure(v.deposits.to_string())));
                                if v.handed > 0 {
                                    rows.push((t(crate::house::i18n::Key::IdRotateHanded), kv::Val::Figure(v.handed.to_string())));
                                }
                            }
                            if !v.new.is_empty() {
                                rows.push((t(crate::house::i18n::Key::IdRotateNew), kv::Val::Mono(v.new.clone())));
                            }
                            let refs: Vec<(&str, kv::Val)> = rows.iter().map(|(k, x)| (k.as_str(), x.clone())).collect();
                            kv::kv(ui, &refs);
                            if v.new.is_empty() && *keep {
                                for (i, (words, why)) in unread.iter().enumerate() {
                                    let at = ui.scope(|ui| states::okline(ui, eravon_ui::mark::Mark::Warn, words)).response.rect;
                                    let _ = eravon_ui::layer::tip(ui.interact(at, ui.id().with(("set-rotate-unread", i)), egui::Sense::hover()), why.as_str());
                                }
                                for (seat, name, why) in &v.refused {
                                    let seat = t(if *seat == eravon_store::vault::Seat::Seller { crate::house::i18n::Key::IdSeatAuthor } else { crate::house::i18n::Key::IdSeatGrantee });
                                    states::okline(ui, eravon_ui::mark::Mark::Warn, &format!("{seat} · {}", crate::house::refusal::line(lang, name, why)));
                                }
                            }
                            if v.opens_on_resume() {
                                states::hint(ui, &t(crate::house::i18n::Key::IdRotateUnfinished));
                            }
                            if v.new.is_empty() && !*keep {
                                states::hint(ui, &t(crate::house::i18n::Key::IdRotateFreshSays));
                            }
                            if v.new.is_empty() && *keep && (!v.order_ids.is_empty() || !v.product_ids.is_empty()) {
                                card::form(ui, |ui, f| {
                                    for (ids, on, k) in [(&v.order_ids, &mut *let_expire, crate::house::i18n::Key::IdRotateLetExpire), (&v.product_ids, &mut *revoke, crate::house::i18n::Key::IdRotateRevoke)] {
                                        for id in ids {
                                            let set = on.contains(id);
                                            f.row_with(ui, None, id, &t(k), card::Value::None, |ui| {
                                                if eravon_ui::toggle::switch(ui, set, !busy).clicked() {
                                                    if set {
                                                        on.retain(|x| x != id);
                                                    } else {
                                                        on.push(id.clone());
                                                    }
                                                }
                                            });
                                        }
                                    }
                                });
                            }
                            if v.new.is_empty() && *keep {
                                for id in &v.refunds {
                                    states::hint(ui, &crate::house::i18n::filln(crate::house::i18n::Key::IdRotateRefund, &[&eravon_orders::table::short(id)]));
                                }
                                for (id, have, all) in &v.series_short {
                                    states::hint(ui, &crate::house::i18n::filln(crate::house::i18n::Key::IdRotateSeries, &[&eravon_orders::table::short(id), &have.to_string(), &all.to_string()]));
                                }
                                states::hint(ui, &t(crate::house::i18n::Key::IdRotateVoid));
                                states::hint(ui, &t(crate::house::i18n::Key::IdRotateHistory));
                            }
                            if v.backup_again {
                                states::hint(ui, &t(crate::house::i18n::Key::IdRotateBackupAgain));
                            }
                            if !v.left_on.is_empty() {
                                states::hint(ui, &t(crate::house::i18n::Key::IdRotateLeftOn));
                            }
                            if !v.phrase.is_empty() {
                                states::hint(ui, &t(crate::house::i18n::Key::IdRotateWords));
                                if eravon_ui::sheetparts::words(ui, "set-rotate-words", Some(&v.phrase), v.phrase_open, crate::lang::t(crate::lang::Key::IdShowNewWords)).clicked() {
                                    unveil = true;
                                }
                            }
                            let eth = |wei: &str| crate::house::coin::eth(lang, wei, crate::house::coin::Form::Full);
                            for x in &v.swept {
                                let line = match &x.how {
                                    eravon_cli::vaultverbs::rotate::Sweep::Moved { amount, .. } => crate::house::i18n::filln(crate::house::i18n::Key::IdRotateSwept, &[&net_of(x.chain), &eth(amount)]),
                                    eravon_cli::vaultverbs::rotate::Sweep::Short { balance, fee } => crate::house::i18n::filln(crate::house::i18n::Key::IdRotateShort, &[&net_of(x.chain), &eth(balance), &eth(fee)]),
                                };
                                states::hint(ui, &line);
                            }
                            for c in &v.unswept {
                                states::hint(ui, &crate::house::i18n::filln(crate::house::i18n::Key::IdRotateUnswept, &[&net_of(*c)]));
                            }
                            for (k, c) in &v.stranded {
                                states::hint(ui, &crate::house::i18n::filln(crate::house::i18n::Key::IdRotateStranded, &[&net_of(*c), &eravon_orders::table::short(k)]));
                            }
                            for h in &v.owed {
                                let k = if h.how == eravon_store::settings::hand_on::REVOKE { crate::house::i18n::Key::IdRotateOwedRevoke } else { crate::house::i18n::Key::IdRotateOwedRegister };
                                states::hint(ui, &crate::house::i18n::filln(k, &[&net_of(h.chain), &eravon_orders::table::short(&h.id)]));
                            }
                            if v.owed_other > 0 {
                                states::hint(ui, &crate::house::i18n::filln(crate::house::i18n::Key::IdRotateOwedOther, &[&v.owed_other.to_string()]));
                            }
                            for l in &v.stops {
                                let net = l.chain.map(net_of);
                                let why = crate::house::refusal::line(lang, &l.name, &l.detail);
                                let head = [Some(eravon_orders::table::short(&l.what)), net, Some(why)].into_iter().flatten().collect::<Vec<String>>().join(" · ");
                                states::hint_ex(ui, &head, true);
                            }
                            if !v.left.is_empty() {
                                eravon_ui::fold::fold(ui, "set-rotate-left", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                                    for l in &v.left {
                                        paint::text(ui, l, Type::MonoSmall, c(C::Ink3));
                                    }
                                });
                            }
                            let resume = v.awaiting() && *keep;
                            commit = (t(if resume { crate::house::i18n::Key::IdRotateResume } else { crate::house::i18n::Key::IdRotate }), v.new.is_empty() || resume);
                        }
                    }
                    let go = (commit.0.as_str(), Role::Commit, commit.1 && !busy && rotation.as_ref().is_some_and(|r| r.is_ok()));
                    let anyway_key = t(crate::house::i18n::Key::IdRotateAnyway);
                    let left: &[(&str, Role, bool)] = if held { &[(anyway_key.as_str(), Role::Secondary, !busy)] } else { &[] };
                    let done = matches!(rotation.as_ref(), Some(Ok(v)) if !v.new.is_empty() && !v.awaiting());
                    Self::sheet_keys_sides(keys, &t(if done { crate::house::i18n::Key::WizFinish } else { crate::house::i18n::Key::CfBack }), &[go], left);
                }
            }
            Self::sheet_said(ui, "set-card-said", said.as_ref());
        });
        let go = match (&next, hit) {
            (SetCard::Rotate { keep, let_expire, revoke }, Some(1)) => Some(Act::with(Step::SetRotateGo, &[("v", if *keep { "on" } else { "off" }), ("letExpire", &let_expire.join(",")), ("revoke", &revoke.join(",")), ("leave", "on")])),
            (_, None) => None,
            (SetCard::Company { addr }, Some(_)) => Some(Act::with(Step::SetCompany, &[("on", "on"), ("a", addr.trim())])),
            (SetCard::Zikaron { dir }, Some(_)) => Some(Act::with(Step::SetZikaron, &[("dir", dir)])),
            (SetCard::TokenAdd { address, label, decimals }, Some(_)) => Some(Act::with(Step::SetTokenAdd, &[("a", address.trim()), ("label", label), ("d", decimals)])),
            (SetCard::TokenRemove { address, .. }, Some(_)) => Some(Act::with(Step::SetTokenRemove, &[("a", address)])),
            (SetCard::TokenDecimals { address, text }, Some(_)) => Some(Act::with(Step::SetTokenDecimals, &[("a", address), ("d", text)])),
            (SetCard::Rotate { keep, let_expire, revoke }, Some(_)) => Some(Act::with(Step::SetRotateGo, &[("v", if *keep { "on" } else { "off" }), ("letExpire", &let_expire.join(",")), ("revoke", &revoke.join(","))])),
        };
        if unveil && let Some(Ok(v)) = self.shell.desk.sh.seller.settings.as_mut().and_then(|s| s.rotate.as_mut()) {
            v.phrase_open = true;
        }
        let close = back;
        self.shell.desk.set_card = if close { None } else { Some(next) };
        if let Some(a) = go {
            self.run(a);
        }
    }

    fn set_opening(&mut self, ui: &mut egui::Ui) {
        use crate::house::i18n::Key;
        let t = |k: Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        if self.shell.desk.sh.seller.pay.is_none() {
            self.shell.open_pay();
        }
        let Some(pay) = self.shell.desk.sh.seller.pay.clone() else { return };
        let (auto, buyer, window) = match &pay.ways {
            Ok(w) => (w.auto_open, w.buyer_may_open, w.open_window),
            Err(_) => (pay.auto_open == "true", pay.buyer_may_open == "true", pay.open_window.parse().unwrap_or(0)),
        };
        if self.shell.desk.open_cells.is_none() {
            let (d, h, m) = eravon_goods::goods::span(window);
            self.shell.desk.open_cells = Some((d.to_string(), h.to_string(), m.to_string()));
        }
        let mut flip: Option<(Step, bool)> = None;
        card::form(ui, |ui, f| {
            f.row_with(ui, None, &t(Key::SetOpenAuto), "", card::Value::None, |ui| {
                if eravon_ui::toggle::switch(ui, auto, true).clicked() {
                    flip = Some((Step::OpenAuto, !auto));
                }
            });
            f.row_with(ui, None, &t(Key::SetOpenBuyer), "", card::Value::None, |ui| {
                if eravon_ui::toggle::switch(ui, buyer, true).clicked() {
                    flip = Some((Step::OpenBuyer, !buyer));
                }
            });
        });
        card::group_title(ui, &t(Key::SetOpenWindow));
        let mut save = false;
        let mut cells = self.shell.desk.open_cells.clone().unwrap_or_default();
        card::card(ui, |ui| {
            super::goods::period_cells(ui, [&mut cells.0, &mut cells.1, &mut cells.2], [false; 3]);
            states::hint(ui, &t(Key::SetOpenNewVersions));
            save = card_keys(ui, |ui| key::key(ui, &t(Key::ReadNetSave), Role::Primary, true).clicked());
        });
        self.shell.desk.open_cells = Some(cells.clone());
        if let Some((step, on)) = flip {
            self.step(step, &[("v", if on { "true" } else { "false" })]);
            self.step(Step::OpeningSave, &[]);
        }
        if save {
            let mut v = eravon_goods::verdict::Verdict::default();
            match eravon_goods::goods::secs_of("openWindow", &cells.0, &cells.1, &cells.2, &mut v) {
                Some(secs) => {
                    self.step(Step::OpenWindow, &[("v", &secs.to_string())]);
                    self.step(Step::OpeningSave, &[("what", "window")]);
                    self.shell.desk.open_cells = None;
                }
                None => {
                    let toast = crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: t(Key::SetOpenWindow), next: String::new(), detail: Some("E_PAYWAYS_OPEN_WINDOW".into()) };
                    self.shell.desk.sh.toast(toast);
                }
            }
        }
        if let Some(s) = self.shell.desk.sh.seller.pay.as_mut().and_then(|p| p.said.take()) {
            let toast = crate::house::shell::Toast { tone: s.tone, head: s.detail.clone(), next: String::new(), detail: (s.tone == crate::house::seller::Tone::Bad).then(|| s.name.clone()) };
            self.shell.desk.sh.toast(toast);
        }
    }

    fn set_company(&mut self, ui: &mut egui::Ui, s: &Settings, busy: bool, open: &mut Option<SetCard>, act: &mut Option<Act>) {
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        let c = &s.company;
        let note = match &c.gate {
            None if c.on => t(crate::house::i18n::Key::SetCoOnNote),
            None => t(crate::house::i18n::Key::SetCoOffNote),
            Some(Ok(o)) => {
                let single = if o.agreement == "single" { format!(" · {}", t(crate::house::i18n::Key::SetCoSingle)) } else { String::new() };
                format!("{} · {} {} s{single}", t(crate::house::i18n::Key::SetCoOpen), t(crate::house::i18n::Key::SetCoBound), o.bound)
            }
            Some(Err(e)) if c.on => format!("{} {} · {} · {}", t(crate::house::i18n::Key::SetCoKept), c.address, t(crate::house::i18n::Key::SetCoShut), crate::house::refusal::line(crate::lang::lang(), &e.name, &e.detail)),
            Some(Err(e)) => format!("{} · {}", t(crate::house::i18n::Key::SetCoShut), crate::house::refusal::line(crate::lang::lang(), &e.name, &e.detail)),
        };
        card::form(ui, |ui, f| {
            let mut on = c.on;
            f.row_with(ui, None, &t(crate::house::i18n::Key::SetCompanyMode), &note, card::Value::None, |ui| {
                if eravon_ui::toggle::switch(ui, on, !busy).clicked() {
                    on = !on;
                    if on {
                        *open = Some(SetCard::Company { addr: c.address.clone() });
                    } else {
                        *act = Some(Act::with(Step::SetCompany, &[("on", "off")]));
                    }
                }
            });
            let mut tc = c.time_check;
            f.row_with(ui, None, &t(crate::house::i18n::Key::SetCoTime), &t(if c.time_check { crate::house::i18n::Key::SetCoTimeOnNote } else { crate::house::i18n::Key::SetCoTimeOffNote }), card::Value::None, |ui| {
                if eravon_ui::toggle::switch(ui, tc, true).clicked() {
                    tc = !tc;
                    *act = Some(Act::with(Step::SetTimeCheck, &[("v", if tc { "on" } else { "off" })]));
                }
            });
        });
        let addr = if c.address.is_empty() { t(crate::house::i18n::Key::SetNone) } else { eravon_orders::table::short(&c.address) };
        card::form(ui, |ui, f| {
            if f.act(ui, &t(crate::house::i18n::Key::SetCoAddress), &addr, key::Key::new(&t(crate::house::i18n::Key::SetCoChange), Role::Secondary).enabled(!busy)).clicked() {
                *open = Some(SetCard::Company { addr: c.address.clone() });
            }
        });
        if c.on || c.gate.is_some() {
            card::card(ui, |ui| {
                eravon_ui::fold::fold(ui, "set-co-detail", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                    let mut rows: Vec<(String, kv::Val)> = Vec::new();
                    if !c.address.is_empty() {
                        rows.push((t(crate::house::i18n::Key::SetCoAddress), kv::Val::Mono(c.address.clone())));
                    }
                    match &c.gate {
                        Some(Ok(o)) => {
                            rows.push((t(crate::house::i18n::Key::SetCoBound), kv::Val::Text(format!("{} s", o.bound))));
                            rows.push((t(crate::house::i18n::Key::SetCoWallet), kv::Val::Mono(o.wallet.clone())));
                            rows.push((t(crate::house::i18n::Key::SetCoLedger), kv::Val::Mono(o.ledger.clone())));
                            rows.push((t(crate::house::i18n::Key::SetCoAgreement), kv::Val::Text(format!("{} · {}", o.agreement, o.endpoints))));
                        }
                        Some(Err(e)) => rows.push((t(crate::house::i18n::Key::SetCoShut), kv::Val::Text(crate::house::refusal::line(crate::lang::lang(), &e.name, &e.detail)))),
                        None => rows.push((t(crate::house::i18n::Key::SetZkState), kv::Val::Text(t(crate::house::i18n::Key::SetCoNotChecked)))),
                    }
                    let refs: Vec<(&str, kv::Val)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                    kv::kv(ui, &refs);
                });
                if card_keys(ui, |ui| key::key(ui, &t(crate::house::i18n::Key::SetCoRecheck), Role::Secondary, !busy && !c.address.is_empty()).clicked()) {
                    *act = Some(Act::of(Step::SetCompanyCheck));
                }
            });
        }
    }

    fn set_tokens(&mut self, ui: &mut egui::Ui, s: &Settings, busy: bool, live: bool, open: &mut Option<SetCard>, act: &mut Option<Act>) {
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        let rows = &s.tokens[..];
        card::section(ui, &t(crate::house::i18n::Key::SetTokensRegistered), "", |ui| {
            card::form(ui, |ui, f| {
                if rows.is_empty() {
                    let _ = f.info(ui, &t(crate::house::i18n::Key::SetSecTokens), card::Show::Empty(&t(crate::house::i18n::Key::SetTokensNone)));
                }
                for r in rows {
                    let kind = t(if eravon_vault::tokens::builtin(r) { crate::house::i18n::Key::SetTokensBuiltin } else { crate::house::i18n::Key::SetTokensCustom });
                    if f.row(ui, &r.label, "", card::Value::NavWith(&kind)).clicked() {
                        *act = Some(Act::with(Step::SetToken, &[("a", &r.address)]));
                    }
                }
                if f.go(ui, &t(crate::house::i18n::Key::SetTokensAddNav), "", live && !busy, "").clicked() {
                    *open = Some(SetCard::TokenAdd { address: s.token_form.address.clone(), label: s.token_form.label.clone(), decimals: String::new() });
                }
            });
        });
    }

    fn set_zikaron(&mut self, ui: &mut egui::Ui, s: &Settings, live: bool, open: &mut Option<SetCard>, act: &mut Option<Act>) {
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        let note = if !s.zk_link {
            t(crate::house::i18n::Key::SetZkNotLinked)
        } else {
            match &s.zk.1 {
                ZkState::Linked { layer, .. } => format!("{} · {}{}", t(crate::house::i18n::Key::SetZkLinked), t(crate::house::i18n::Key::SetZkVia), crate::house::i18n::get_named(crate::lang::lang(), &format!("set.zkLayer.{layer}")).0),
                ZkState::Absent { name, .. } if name == "E_ZK_NOT_CONNECTED" => t(crate::house::i18n::Key::SetZkNotLinked),
                ZkState::Absent { name, .. } => format!("{} · {}", t(crate::house::i18n::Key::SetZkNotLinked), crate::house::i18n::get_named(crate::lang::lang(), &format!("set.zkAbsent.{name}")).0),
                ZkState::Bad { .. } => format!("{} · {}", t(crate::house::i18n::Key::SetZkNotLinked), t(crate::house::i18n::Key::SetZkUnreadable)),
            }
        };
        card::form(ui, |ui, f| {
            let mut on = s.zk_link;
            f.row_with(ui, None, &t(crate::house::i18n::Key::SetZkLink), &note, card::Value::None, |ui| {
                if eravon_ui::toggle::switch(ui, on, live).clicked() {
                    on = !on;
                    *act = Some(Act::with(Step::SetZkLink, &[("v", if on { "on" } else { "off" })]));
                }
            });
        });
        let shown = if s.zk.0.is_empty() { t(crate::house::i18n::Key::SetZkAuto) } else { s.zk.0.clone() };
        card::form(ui, |ui, f| {
            if f.act(ui, &t(crate::house::i18n::Key::SetZkDir), &shown, key::Key::new(&t(crate::house::i18n::Key::ExChange), Role::Secondary).enabled(s.zk_link && live)).clicked() {
                let at = match (&s.zk.0, &s.zk.1) {
                    (d, _) if !d.is_empty() => d.clone(),
                    (_, ZkState::Linked { at, .. }) => at.clone(),
                    _ => String::new(),
                };
                *open = Some(SetCard::Zikaron { dir: at });
            }
        });
        card::card(ui, |ui| {
            eravon_ui::fold::fold(ui, "set-zk-detail", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                let mut rows: Vec<(String, kv::Val)> = Vec::new();
                match &s.zk.1 {
                    ZkState::Linked { at, layer, kits, absent } => {
                        rows.push((t(crate::house::i18n::Key::SetZkDir), kv::Val::Mono(at.clone())));
                        rows.push((t(crate::house::i18n::Key::SetZkBy), kv::Val::Text(crate::house::i18n::get_named(crate::lang::lang(), &format!("set.zkLayer.{layer}")).0)));
                        rows.push((t(crate::house::i18n::Key::SetZkKits), kv::Val::Text(kits.to_string())));
                        if let Some((n, d)) = absent {
                            rows.push((t(crate::house::i18n::Key::SetZkState), kv::Val::Text(format!("{} · {d}", crate::house::i18n::get_named(crate::lang::lang(), &format!("set.zkAbsent.{n}")).0))));
                        }
                    }
                    ZkState::Absent { name, detail } => rows.push((t(crate::house::i18n::Key::SetZkState), kv::Val::Text(format!("{} · {detail}", crate::house::i18n::get_named(crate::lang::lang(), &format!("set.zkAbsent.{name}")).0)))),
                    ZkState::Bad { detail, .. } => rows.push((t(crate::house::i18n::Key::SetZkState), kv::Val::Text(detail.clone()))),
                }
                let refs: Vec<(&str, kv::Val)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                kv::kv(ui, &refs);
            });
        });
    }

    fn set_net_extras(&mut self, ui: &mut egui::Ui, _s: &Settings, live: bool, act: &mut Option<Act>) {
        use eravon_ui::mark::Mark;
        let lang = crate::lang::lang();
        let t = move |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let ver = eravon_literals::literals::DOMAIN_VERSION.to_string();
        let network = crate::house::i18n::get_named(crate::lang::lang(), eravon_literals::constants::current_network().word).0;
        let asked = crate::house::settings::asked(&self.shell.settings);
        let (tint, said) = match self.shell.desk.sh.sync.agreement {
            Some(a) if a == "agreed" => (card::Tint::Ok, t(crate::house::i18n::Key::SyncAgreed)),
            Some(_) => (card::Tint::Blue, t(crate::house::i18n::Key::SyncSingle)),
            None => (if asked == 0 { card::Tint::Warn } else { card::Tint::Blue }, t(crate::house::i18n::Key::SyncNotYet)),
        };
        let nodes = crate::fill!(crate::house::i18n::Key::SetNodesN; asked.to_string());
        card::form(ui, |ui, f| {
            f.summary(ui, Glyph::Globe, tint, &network, None, &said, |_| ());
            f.facts(ui, &[
                (&t(crate::house::i18n::Key::SetEndpoints), if asked == 0 { card::Fact::Warn(&nodes) } else { card::Fact::Text(&nodes) }),
                (&t(crate::house::i18n::Key::SetContract), card::Fact::Text(&ver)),
                (&t(crate::house::i18n::Key::SetSyncState), card::Fact::Text(&said)),
            ]);
        });
        let _ = Mark::Ok;
        let nets = eravon_literals::constants::NETWORKS;
        let cur = nets.iter().position(|n| n.name == eravon_literals::constants::current_network().name).unwrap_or(0);
        let words: Vec<String> = nets.iter().map(|n| crate::house::i18n::get_named(lang, n.word).0).collect();
        let cells: Vec<seg::Cell> = words.iter().map(|w| seg::Cell::from(w.as_str())).collect();
        let past = eravon_literals::constants::past_switch();
        card::form(ui, |ui, f| {
            f.row_with(ui, None, &t(crate::house::i18n::Key::SetNetwork), "", card::Value::None, |ui| {
                if let Some(i) = seg::seg(ui, "set-network", &cells, cur).filter(|i| *i != cur && live) {
                    *act = Some(Act::with(Step::SetNetwork, &[("chain", &nets[i].standard.chain_id.to_string())]));
                }
            });
            f.row_with(ui, None, &t(crate::house::i18n::Key::SetPastCases), "", card::Value::None, |ui| {
                if eravon_ui::toggle::switch(ui, past, live).clicked() {
                    *act = Some(Act::with(Step::SetPastCases, &[("v", if past { "off" } else { "on" })]));
                }
            });
        });
    }

    pub(in crate::window) fn house_net_rows(&self) -> Option<Vec<(&'static str, kv::Val)>> {
        let s = self.shell.desk.sh.seller.settings.clone()?;
        let logic = eravon_literals::constants::active_standard().logic.to_string();
        let none = || kv::Val::text(t(crate::house::i18n::Key::SetNone));
        let mut rows = vec![
            (t(crate::house::i18n::Key::SetEndpoints), if s.endpoints.is_empty() { none() } else { kv::Val::mono(s.endpoints.join("\n")) }),
            (t(crate::house::i18n::Key::SetContractAddr), if logic.is_empty() { none() } else { kv::Val::mono(logic) }),
        ];
        if let Some(b) = self.shell.desk.sh.sync.block {
            rows.push((t(crate::house::i18n::Key::SyncBlock), kv::Val::Figure(super::orders::thousands(b))));
        }
        Some(rows)
    }
}

fn net_name(lang: crate::lang::Lang, c: u64) -> String {
    match c {
        c if c == eravon_literals::constants::STANDARD_TABLE.chain_id => crate::house::i18n::get_named(lang, "set.chainMain").0,
        c if c == eravon_literals::constants::STANDARD_TABLE_SEPOLIA.chain_id => crate::house::i18n::get(lang, crate::house::i18n::Key::SetChainSepolia).0,
        other => other.to_string(),
    }
}

fn cases_said(lang: crate::lang::Lang, v: &crate::house::settings::RotView) -> (kv::Val, Vec<(String, String)>) {
    let lines = v
        .cases_unread
        .iter()
        .map(|(c, name, why)| (crate::house::i18n::filln_in(lang, crate::house::i18n::Key::IdRotateCasesUnread, &[&net_name(lang, *c)]), crate::house::refusal::line(lang, name, why)))
        .collect();
    (kv::Val::Figure(v.cases.to_string()), lines)
}

fn unread_why(lang: crate::lang::Lang, state: &str, detail: &str) -> Option<String> {
    if state != "unread" || detail.is_empty() {
        return None;
    }
    match detail.split_once(':') {
        Some((name, rest)) if name.starts_with("E_") => Some(crate::house::refusal::line(lang, name, rest)),
        _ if lang == crate::lang::Lang::Zh => Some(detail.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod rotate_row_tests {
    #[test]
    fn an_unread_network_says_why_and_a_read_one_says_its_state() {
        use crate::lang::Lang;
        assert_eq!(super::unread_why(Lang::Zh, "unread", "没有这条链的节点").as_deref(), Some("没有这条链的节点"));
        assert_eq!(super::unread_why(Lang::En, "unread", "没有这条链的节点"), None);
        let named = super::unread_why(Lang::En, "unread", "E_CHAIN_WRONG:节点在链 1,所选的网在链 11155111").expect("named");
        assert!(!named.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{named}");
        assert_eq!(super::unread_why(Lang::Zh, "attempt", ""), None);
    }

    #[test]
    fn the_cases_cell_says_an_unread_network_in_words() {
        use crate::lang::Lang;
        let drawn = |v: &crate::house::settings::RotView, lang: Lang| {
            let (cell, lines) = super::cases_said(lang, v);
            let mut words = vec![match cell {
                eravon_ui::kv::Val::Figure(s) => s,
                _ => panic!("the count is a figure"),
            }];
            words.extend(lines.into_iter().map(|(w, _)| w));
            words
        };
        let main = eravon_literals::constants::STANDARD_TABLE.chain_id;
        let whole = crate::house::settings::RotView { cases: 1, ..Default::default() };
        for lang in [Lang::Zh, Lang::En] {
            assert_eq!(drawn(&whole, lang), vec!["1".to_string()]);
        }
        let why = "S2 说 E_CHAIN_ENDPOINT:链 1 上本席的订单没读全:left 取日志每一家都出列了:http://127.0.0.1:61767:chain 1 eth_getLogs HTTP_503";
        let part = crate::house::settings::RotView { cases: 1, cases_unread: vec![(main, "E_V_DOWN".into(), why.into())], ..Default::default() };
        for lang in [Lang::Zh, Lang::En] {
            let words = drawn(&part, lang);
            assert_eq!(words.len(), 2, "{words:?}");
            assert_eq!(words[0], "1");
            assert!(words[1].contains(&super::net_name(lang, main)), "{words:?}");
            for w in &words {
                assert!(!w.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).any(|x| x.starts_with("E_")), "a refusal name is drawn: {w}");
                assert!(!w.contains("http") && !w.contains("127.0.0.1"), "a node address is drawn: {w}");
            }
        }
    }
}
