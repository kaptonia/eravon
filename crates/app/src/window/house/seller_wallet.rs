use super::*;
use crate::house::action::Action as Step;
use crate::house::i18n::Key;

impl Win {
    pub(super) fn seller_wallet_page(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: Key| crate::house::i18n::get(lang, k).0;
        let Some(w) = self.wallet_now() else {
            Self::home_skeleton(ui, 2);
            return;
        };
        let busy = self.shell.house_in_flight(crate::task::Kind::Wallet);
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let (mut set_vault, mut adjust): (bool, Option<String>) = (false, None);
        let mut flip: Option<bool> = None;
        let (mut own_in, mut own_out): (bool, Option<Option<String>>) = (false, None);
        let can_send = self.shell.write_why(crate::house::shell::Write::Chain).is_none();
        let own_on = w.vault.as_ref().and_then(|v| v.as_ref().ok()).is_some_and(|v| v.own && v.company.is_empty());
        let held: Vec<(String, String)> = match &w.balances {
            Some(Ok(b)) => b.coins.iter().filter(|c| c.have.as_ref().is_ok_and(|x| x != "0")).map(|c| (c.token.clone(), crate::house::coin::figure(lang, c.have.as_deref().unwrap_or("0"), c.decimals, crate::house::coin::Form::Full))).collect(),
            _ => Vec::new(),
        };
        match &w.vault {
            None => Self::home_skeleton(ui, 1),
            Some(Err(e)) if unread(&e.name) => card::card(ui, |ui| unread_block(ui, syncing, &eravon_ui::house::SKELETON_TWO)),
            Some(Err(e)) => said_banner(ui, "seller_wallet-said-2", lang, &e.name, &e.detail),
            Some(Ok(v)) if v.fund.is_empty() => {
                card::form(ui, |ui, f| {
                    set_vault = f.act(ui, &t(Key::OrdersKvBond), &t(Key::WlVaultSay), key::Key::new(&t(Key::WlVaultSetKey), Role::Primary).enabled(!busy)).clicked();
                });
                if v.company.is_empty() {
                    flip = own_card(ui, v.own, &w.me, &held, busy, can_send).0;
                }
            }
            Some(Ok(v)) => {
                if v.rows.is_empty() {
                    card::form(ui, |ui, f| {
                        let copied = f.info(ui, &t(Key::WlVault), card::Show::Short(&v.fund));
                        f.note(ui, &t(Key::WlVaultNoGoods));
                        tell_copied(ui.ctx(), copied);
                    });
                }
                for (token, st) in &v.rows {
                    match st {
                        Err(e) if unread(&e.name) => card::card(ui, |ui| unread_block(ui, syncing, &eravon_ui::house::SKELETON_TWO)),
                        Err(e) => said_banner(ui, "seller_wallet-said-3", lang, &e.name, &e.detail),
                        Ok(s) => {
                            let decimals = v.decimals.get(token).copied().flatten();
                            let word = |raw: &eravon_terms::u256::U256| crate::house::coin::word(&raw.to_dec(), decimals, token, crate::house::coin::Form::Short);
                            let (stood, waiting) = v.stands(token, &s.available);
                            let (state, tint) = if !s.bound {
                                (t(Key::GoodsRegNotHeld), card::Tint::Warn)
                            } else if s.allowance.is_zero() {
                                (t(Key::WlNoAllowance), card::Tint::Warn)
                            } else if waiting > 0 && stood < waiting {
                                (crate::house::i18n::filln(Key::WlStandsShort, &[&stood.to_string(), &waiting.to_string()]), card::Tint::Warn)
                            } else if waiting > 0 {
                                (crate::house::i18n::filln(Key::WlStands, &[&stood.to_string()]), card::Tint::Ok)
                            } else {
                                (t(Key::WlReady), card::Tint::Ok)
                            };
                            let title = format!("{} · {}", t(Key::OrdersKvBond), crate::house::coin::name(lang, token));
                            let copied = card::form(ui, |ui, f| {
                                f.summary(ui, Glyph::Vault, tint, &title, None, &state, |ui| {
                                    if key::show(ui, key::Key::new(&t(Key::WlAdjust), Role::Secondary).enabled(!busy)).clicked() {
                                        adjust = Some(token.clone());
                                    }
                                });
                                let three = [(t(Key::WlAvailable), word(&s.available)), (t(Key::PlStepBalance), word(&s.balance)), (t(Key::WlAllowance), word(&s.allowance))];
                                if decimals.is_some() {
                                    f.facts(ui, &three.iter().map(|(k, x)| (k.as_str(), card::Fact::Text(x.as_str()))).collect::<Vec<_>>());
                                } else {
                                    for (k, x) in &three {
                                        f.info(ui, k, card::Show::Text(x));
                                    }
                                }
                                let mut copied = None;
                                let _ = f.line(ui, None, &t(Key::WlVault), "", false, false, c(C::Ink), |ui| {
                                    if v.company.is_empty() && !v.own && key::key(ui, &t(Key::ExChange), Role::Secondary, !busy).clicked() {
                                        set_vault = true;
                                    }
                                    copied = super::casepages::copy_mono(ui, &s.fund);
                                });
                                let bound_say = if v.own { Key::WlOwnNoBind } else if s.bound { Key::WlBoundSay } else { Key::GoodsRegNotHeld };
                                f.info(ui, &t(Key::WlBinding), card::Show::Mark(if s.bound { Mark::Ok } else { Mark::Warn }, &t(bound_say)));
                                let each: Vec<(String, kv::Val)> = v
                                    .waiting
                                    .iter()
                                    .filter(|o| o.token.eq_ignore_ascii_case(token))
                                    .map(|o| (short(&o.id), match &o.deposit {
                                        Ok(d) => kv::Val::Text(word(d)),
                                        Err(e) => kv::Val::Text(said_line(&e.name, &crate::house::refusal::line(lang, &e.name, &e.detail))),
                                    }))
                                    .collect();
                                if !each.is_empty() {
                                    let refs: Vec<(&str, kv::Val)> = each.iter().map(|(k, x)| (k.as_str(), x.clone())).collect();
                                    let (detail, widest) = detail_rows_of(ui, &refs);
                                    copied = f.details(ui, &format!("wallet-waiting-{token}"), &t(Key::WlWaiting), &detail, widest).or(copied);
                                }
                                copied
                            });
                            tell_copied(ui.ctx(), copied);
                        }
                    }
                }
                if v.company.is_empty() {
                    let (fl, i, o) = own_card(ui, v.own, &w.me, &held, busy, can_send);
                    flip = fl.or(flip);
                    own_in |= i;
                    if o {
                        own_out = Some(None);
                    }
                }
                if !v.rows.is_empty() {
                    states::banner(ui, states::Banner::Note, &t(Key::WlVaultSay), |_| ());
                }
                for h in &v.hints {
                    let say = match h {
                        eravon_goods::fund::Hint::NewStandard { logic } => crate::house::i18n::filln(Key::WlHintNewStandard, &[logic]),
                        eravon_goods::fund::Hint::LargeDeposit { deposit, amount } => crate::house::i18n::filln(Key::WlHintLargeDeposit, &[&deposit.to_dec(), &amount.to_dec()]),
                    };
                    states::banner(ui, states::Banner::Warn, &say, |_| {});
                }
            }
        }
        if set_vault {
            self.shell.desk.vault_fund = Some(String::new());
        }
        if let Some(on) = flip {
            self.step(Step::WlFundOwn, &[("on", if on { "true" } else { "false" })]);
        }
        if let Some(token) = adjust {
            self.shell.desk.vault_adjust = Some((token, String::new()));
            self.step(Step::WlVaultClose, &[]);
        }
        if self.shell.desk.sh.seller.pay.is_none() {
            self.shell.open_pay();
        }
        let payout = self.shell.desk.sh.seller.pay.as_ref().map(|p| p.payout.clone()).unwrap_or_default();
        let mut change = false;
        card::section(ui, &t(Key::WlReceive), "", |ui| {
            let copied = card::form(ui, |ui, f| {
                let word = if payout.is_empty() { Key::PaySetPayout } else { Key::ExChange };
                let mut copied = None;
                let _ = f.line(ui, None, &t(Key::SetPayout), "", false, false, c(C::Ink), |ui| {
                    change = key::key(ui, &t(word), Role::Secondary, true).clicked();
                    if payout.is_empty() {
                        ui.label(egui::RichText::new(t(Key::SetNone)).font(Type::Note.font()).color(c(C::Ink3)));
                    } else {
                        copied = super::casepages::copy_mono(ui, &payout);
                    }
                });
                copied
            });
            tell_copied(ui.ctx(), copied);
        });
        if change {
            self.shell.desk.pay_payout = Some((String::new(), String::new()));
        }
        let key_say = if own_on { Key::WlTradeKeyOwnSay } else { Key::WlTradeKeySay };
        card::section(ui, &t(Key::WlTradeKey), &t(key_say), |ui| {
            let copied = card::form(ui, |ui, f| {
                let eth = match &w.balances {
                    Some(Ok(b)) => b.native.as_ref().map(|x| crate::house::coin::eth(lang, x, crate::house::coin::Form::Full)).unwrap_or_else(|_| t(Key::SyncNever)),
                    _ => t(Key::SyncNever),
                };
                let shown = f.info(ui, &t(Key::WlTradeKey), card::Show::Short(&w.me));
                f.info(ui, &t(Key::WlEth), card::Show::Text(&eth));
                if !own_on {
                    for (token, figure) in &held {
                        let (_, out) = f.line(ui, None, &crate::house::coin::name(lang, token), "", false, false, c(C::Ink), |ui| {
                            let out = key::key(ui, &t(Key::WlSend), Role::Guide, !busy && can_send).clicked();
                            paint::text(ui, figure, Type::Body, c(C::Ink));
                            out
                        });
                        if out {
                            own_out = Some(Some(token.clone()));
                        }
                    }
                }
                shown
            });
            tell_copied(ui.ctx(), copied);
        });
        self.formers_section(ui, &w, busy, can_send);
        if let Some(pre) = own_out {
            let choices = super::buyer_home::seller_choices(&w);
            let i = pre.and_then(|t| choices.iter().position(|(_, x)| x.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(&t)))).unwrap_or(0);
            self.shell.desk.wl_form = (i, String::new(), String::new());
            self.shell.desk.wl_again.clear();
            self.step(Step::WlSheet, &[("open", "true")]);
        }
        if own_in || self.shell.desk.wl_receive {
            self.shell.desk.wl_receive = true;
            self.receive_sheet(ui.ctx(), &w, Key::WlIn);
        }
        if w.sheet {
            self.wallet_sheet(ui.ctx(), &w, true);
        }
        self.payout_card(ui.ctx());
        self.vault_fund_card(ui.ctx(), busy);
        self.vault_adjust_card(ui.ctx(), &w, busy);
    }

    fn vault_fund_card(&mut self, ctx: &egui::Context, busy: bool) {
        let Some(mut a) = self.shell.desk.vault_fund.take() else { return };
        let t = |k: Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        let (_, save, cancel) = Self::sheet_with(ctx, "wl-fund", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &t(Key::WlVaultSetKey), "");
            field(ui, &t(Key::WlVaultAddr), None, |ui| {
                let _ = input::field(ui, &mut a, "0x…", ui.available_width(), input::Look { mono: true, ..input::Look::default() });
            });
            Self::sheet_keys(keys, &t(Key::CfBack), Some((&t(Key::ReadNetSave), Role::Primary, !busy && !a.trim().is_empty())));
        });
        if cancel {
            return;
        }
        if save {
            self.step(Step::WlFund, &[("v", a.trim())]);
            return;
        }
        self.shell.desk.vault_fund = Some(a);
    }

    fn vault_adjust_card(&mut self, ctx: &egui::Context, w: &crate::house::wallet::Wallet, busy: bool) {
        let Some((token, mut amount)) = self.shell.desk.vault_adjust.take() else { return };
        let lang = crate::lang::lang();
        let t = |k: Key| crate::house::i18n::get(lang, k).0;
        let chain = eravon_literals::constants::active_standard().chain_id;
        let set = w.vault_set.as_ref().filter(|s| s.token.eq_ignore_ascii_case(&token) && s.amount == amount.trim()).cloned();
        let mut open_uri: Option<String> = None;
        let fund = w.vault.as_ref().and_then(|v| v.as_ref().ok()).map(|v| v.fund.clone()).unwrap_or_default();
        let own = w.vault.as_ref().and_then(|v| v.as_ref().ok()).is_some_and(|v| v.own && v.company.is_empty());
        let (hit, back) = {
            let (_, hit, back) = Self::sheet_pick(ctx, "wl-vault", eravon_ui::metrics::MODAL_WIDE_W, |ui, keys| {
                Self::sheet_head(ui, &t(Key::WlAdjust), "");
                let coin = crate::house::coin::name(lang, &token);
                let (vault, tokens, allowance) = (t(Key::WlVault), t(Key::SetSecTokens), t(Key::WlAllowance));
                let key_w = kv::key_width([&vault, &tokens, &allowance].iter().map(|k| ui.painter().layout_no_wrap(k.to_string(), Type::Body.font(), ui.visuals().text_color()).size().x).fold(0.0, f32::max));
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = eravon_ui::tokens::KV_ROW_GAP;
                    for (k, v) in [(&vault, kv::Val::Mono(short(&fund))), (&tokens, kv::Val::Text(coin.clone()))] {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = eravon_ui::tokens::KV_COL_GAP;
                            kv_label(ui, k, key_w);
                            kv::value(ui, &v);
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.set_min_height(eravon_ui::tokens::INPUT_H);
                        ui.spacing_mut().item_spacing.x = eravon_ui::tokens::KV_COL_GAP;
                        kv_label(ui, &allowance, key_w);
                        let unit = crate::house::i18n::filln(Key::WlUnit, &[&coin]);
                        let unit_w = ui.painter().layout_no_wrap(unit.clone(), Type::Body.font(), ui.visuals().text_color()).size().x.ceil();
                        let w = (ui.available_width() - unit_w - eravon_ui::tokens::S2).max(0.0);
                        ui.spacing_mut().item_spacing.x = eravon_ui::tokens::S2;
                        let _ = input::line_w(ui, &mut amount, "0", w);
                        paint::text(ui, &unit, Type::Body, c(C::Ink2));
                    });
                });
                match &set {
                    Some(s) => match &s.calls {
                        Err(e) => said_hint(ui, "seller_wallet-said-1", lang, &e.name, &e.detail, true),
                        Ok(crate::house::wallet::Setup::Proposals(ps)) => {
                            for (path, face) in ps {
                                card::card(ui, |ui| {
                                    kv::kv(ui, &[(t(Key::GoodsProposalAt).as_str(), kv::Val::Mono(path.clone()))]);
                                    for line in face {
                                        states::hint(ui, line);
                                    }
                                });
                            }
                        }
                        Ok(crate::house::wallet::Setup::Sent(_)) => {}
                        Ok(crate::house::wallet::Setup::Calls(calls)) if calls.is_empty() => states::okline(ui, eravon_ui::mark::Mark::Ok, &t(Key::WlVaultNothing)),
                        Ok(crate::house::wallet::Setup::Calls(calls)) => {
                            for (i, call) in calls.iter().enumerate() {
                                let uri = crate::house::seam::wallet_uri(chain, &call.to, &call.data, "0");
                                card::card(ui, |ui| {
                                    paint::text(ui, &crate::house::i18n::filln(Key::WlVaultCall, &[&(i + 1).to_string()]), Type::Strong, c(C::Ink));
                                    qr_plate(ui, &uri);
                                    eravon_ui::fold::fold(ui, &format!("wl-vault-call-{i}"), &t(Key::ToastDetail), |ui| {
                                        kv::kv(ui, &[("to", kv::Val::Mono(call.to.clone())), ("data", kv::Val::Mono(call.hex()))]);
                                    });
                                    if card_keys(ui, |ui| key::key(ui, &t(Key::WlOpenVault), Role::Secondary, true).clicked()) {
                                        open_uri = Some(uri.clone());
                                    }
                                });
                            }
                        }
                    },
                    None => {}
                }
                let formed = set.as_ref().is_some_and(|s| s.calls.as_ref().is_ok_and(|c| match c { crate::house::wallet::Setup::Calls(c) => !c.is_empty(), crate::house::wallet::Setup::Proposals(p) => !p.is_empty(), crate::house::wallet::Setup::Sent(_) => false }));
                if own {
                    Self::sheet_keys_n(keys, &t(Key::CfBack), &[(&t(Key::WlAdjust), Role::Commit, !busy && !amount.trim().is_empty())]);
                } else if formed {
                    Self::sheet_keys_n(keys, &t(Key::CfBack), &[(&t(Key::WlVaultReread), Role::Primary, !busy)]);
                } else {
                    Self::sheet_keys_n(keys, &t(Key::CfBack), &[(&t(Key::WlAdjust), Role::Primary, !busy && !amount.trim().is_empty())]);
                }
            });
            (hit, back)
        };
        if let Some(u) = open_uri {
            if let Err(e) = crate::house::platform::open_url(&u) {
                self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: e, next: String::new(), detail: Some("E_APP_BROWSER".into()) });
            }
        }
        if back {
            self.step(Step::WlVaultClose, &[]);
            return;
        }
        let formed = set.as_ref().is_some_and(|s| s.calls.as_ref().is_ok_and(|c| match c { crate::house::wallet::Setup::Calls(c) => !c.is_empty(), crate::house::wallet::Setup::Proposals(p) => !p.is_empty(), crate::house::wallet::Setup::Sent(_) => false }));
        if hit == Some(0) {
            if formed && !own {
                self.step(Step::WlVaultClose, &[]);
                self.step(Step::WlReload, &[]);
                return;
            }
            self.step(Step::WlVaultSetup, &[("token", &token), ("amount", amount.trim())]);
        }
        self.shell.desk.vault_adjust = Some((token, amount));
    }
}

fn kv_label(ui: &mut egui::Ui, k: &str, w: f32) {
    ui.allocate_ui_with_layout(egui::vec2(w, Type::Body.line()), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_min_width(w);
        ui.set_max_width(w);
        ui.add(egui::Label::new(egui::RichText::new(k).font(Type::Body.font()).color(c(C::Ink2)).line_height(Some(Type::Body.line()))).wrap());
    });
}

fn own_card(ui: &mut egui::Ui, own: bool, me: &str, held: &[(String, String)], busy: bool, can_send: bool) -> (Option<bool>, bool, bool) {
    let lang = crate::lang::lang();
    let t = |k: Key| crate::house::i18n::get(lang, k).0;
    let (mut flip, mut sent_in, mut sent_out) = (None, false, false);
    card::form(ui, |ui, f| {
        f.keep(ui);
        f.key("wl-own-switch");
        if f.switch(ui, &t(Key::WlOwnSwitch), "", own, !busy) {
            flip = Some(!own);
        }
        if own {
            f.key("wl-own-vault");
            let sub = format!("{} · {}", t(Key::WlTradeKey), short(me));
            let (_, (o, i)) = f.line(ui, Some(card::Lead::Glyph(Glyph::Vault)), &t(Key::WlOwnVault), &sub, false, false, c(C::Ink), |ui| {
                let i = key::key(ui, &t(Key::WlIn), Role::Secondary, true).clicked();
                let o = key::key(ui, &t(Key::WlSend), Role::Guide, !busy && can_send).clicked();
                (o, i)
            });
            sent_in = i;
            sent_out = o;
            for (token, figure) in held {
                f.key(("wl-own-coin", token.as_str()));
                f.info(ui, &crate::house::coin::name(lang, token), card::Show::Text(figure));
            }
        }
    });
    (flip, sent_in, sent_out)
}
