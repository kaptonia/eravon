use super::*;
use crate::house::action::Action as Step;
use crate::house::paylink::{State, Tab};

impl Win {
    pub(super) fn paylink_page(&mut self, ui: &mut egui::Ui) {
        let Some(pl) = self.shell.desk.sh.paylink.clone() else { return };
        if let Some(inner) = pl.inner.clone() {
            self.paylink_inner(ui, &pl, &inner);
            return;
        }
        match pl.tab {
            Tab::Pay => self.paylink_pay(ui, &pl),
            Tab::Import => self.paylink_import(ui, &pl),
        }
    }

    fn paylink_inner(&mut self, ui: &mut egui::Ui, pl: &crate::house::paylink::PayLink, inner: &crate::house::paylink::PlInner) {
        use super::casepages::Asked;
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(Ok(o)) = pl.verified() else { return };
        let item = if o.title.is_empty() { t(crate::house::i18n::Key::BoUntitled) } else { o.title.clone() };
        match inner {
            crate::house::paylink::PlInner::Doc(goods) => {
                let seg = pl.segment(*goods);
                let name = t(if *goods { crate::house::i18n::Key::PlSegGoods } else { crate::house::i18n::Key::GoodsDeliveryNote });
                let state = seg.as_ref().map(|s| text_state(s)).unwrap_or_default();
                let digest = seg.as_ref().filter(|s| !s.on_board).map(|s| s.hash.clone());
                let text = seg.as_ref().and_then(|s| s.text.clone());
                let asked = super::casepages::text_page(ui, &item, &format!("{name} · {state}"), *goods, text.as_deref(), digest.as_deref(), pl.checked.as_ref());
                let mut pick = false;
                if text.is_none() {
                    card::form(ui, |ui, f| {
                        pick = f.act(ui, &t(crate::house::i18n::Key::PlSegCheck), "", key::Key::new(&t(crate::house::i18n::Key::PickFile), Role::Secondary)).clicked();
                    });
                }
                let image = matches!(asked, Some(Asked::CheckImage));
                if let Some(Asked::Url(k)) = asked {
                    self.open_foreign(k);
                }
                let dropped = eravon_ui::drop::take_dropped(ui.ctx()).into_iter().next();
                let seg_file = path_answer(ui.ctx(), egui::Id::new("eravon-pl-seg-check"), pick, crate::platform::Pick::File);
                let image_file = path_answer(ui.ctx(), egui::Id::new("eravon-pl-image-check"), image, crate::platform::Pick::File);
                match (text.is_none(), seg_file.or(image_file).or(dropped)) {
                    (true, Some(p)) => self.step(Step::PlAttach, &[("path", &p)]),
                    (false, Some(p)) if *goods => self.step(Step::PdImageCheck, &[("path", &p)]),
                    _ => {}
                }
            }
            crate::house::paylink::PlInner::Lapse(a) => {
                if let Some((_, guild, Ok(ck))) = pl.lapses().into_iter().find(|(x, _, _)| x.eq_ignore_ascii_case(a)) {
                    super::casepages::lapse_page(ui, &pl.cand_label(a), guild, &ck);
                }
            }
            crate::house::paylink::PlInner::History(a) => super::history::page(ui, lang, "pl-history", a, &pl.history, self.shell.house_in_flight(crate::task::Kind::History)),
        }
    }

    fn paylink_pay(&mut self, ui: &mut egui::Ui, pl: &crate::house::paylink::PayLink) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let mut link = pl.link.clone();
        let mut open = false;
        eravon_ui::fields::card(ui, |ui, st| {
            st.field(ui, &t(crate::house::i18n::Key::PlPasteLabel), None, None, |ui, _w| {
                eravon_ui::scroll::nested(ui, egui::ScrollArea::vertical().id_salt("pl-link").max_height(eravon_ui::house::PASTE_MAX_H), |ui| {
                    input::area_hint(ui, &mut link, 2, "eravon://open?e=…");
                });
            });
            st.foot(ui, |ui| {
                open = key::key(ui, &t(crate::house::i18n::Key::PlOpen), if pl.verified().is_some() { Role::Secondary } else { Role::Primary }, !link.trim().is_empty()).clicked();
            });
        });
        if link != pl.link {
            if let Some(x) = self.shell.desk.sh.paylink.as_mut() {
                x.link = link.clone();
            }
        }
        if open {
            self.step(Step::PlOpen, &[]);
            return;
        }
        let o = match pl.verified() {
            None => return,
            Some(Err(s)) => {
                states::err_box(ui, "pl-open", &t(crate::house::i18n::Key::PlOpenFailed), &crate::house::refusal::line(lang, &s.name, &s.detail), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail));
                return;
            }
            Some(Ok(o)) => o.clone(),
        };
        let all = crate::house::paylink::checks_pass(&o.steps);
        let summary = t(if all { crate::house::i18n::Key::PlAllPass } else { crate::house::i18n::Key::PlNotPass });
        card::section(ui, "", "", |ui| {
            card::form(ui, |ui, f| {
              let _ = f.fold(ui, "pl-steps", &t(crate::house::i18n::Key::PlSteps), &summary, |ui, f| {
                for (name, s) in &o.steps {
                    let title = crate::house::i18n::get_named(lang, &format!("pl.step.{name}")).0;
                    let (w, tone, code, why) = match s {
                        State::Pass => (t(crate::house::i18n::Key::PlPass), eravon_ui::palette::Tone::Ok, String::new(), String::new()),
                        State::Fail(n, d) => (t(crate::house::i18n::Key::PlFail), eravon_ui::palette::Tone::Bad, raw_of(n, d), step_why(lang, name, n, d, o.series_held.as_deref())),
                    };
                    let _ = f.line(ui, None, &title, &why, false, false, c(C::Ink), |ui| {
                        let r = mark::pill(ui, &w, tone);
                        if !code.is_empty() {
                            let _ = eravon_ui::layer::tip(r, code.clone());
                        }
                    });
                }
              });
            });
        });
        let mut history_of: Option<String> = None;
        let mut seller_copied = None;
        let money = |v: &str| crate::house::coin::amount(lang, v, o.decimals, &o.token, crate::house::coin::Form::Full);
        card::section(ui, &t(crate::house::i18n::Key::GoodsBasics), "", |ui| {
            card::form(ui, |ui, f| {
                f.info(ui, &t(crate::house::i18n::Key::PlTitle), card::Show::Text(&o.title));
                f.info(ui, &t(crate::house::i18n::Key::OrdersAmount), card::Show::Text(&money(&o.amount)));
                let _ = f.row(ui, &t(crate::house::i18n::Key::GoodsT1), &t(crate::house::i18n::Key::GoodsT1From), card::Value::Text(&span_text(o.t1)));
                f.info(ui, &t(crate::house::i18n::Key::GoodsT2), card::Show::Text(&span_text(o.t2)));
                f.info(ui, &t(crate::house::i18n::Key::PlSource), card::Show::Short(&o.source));
                let balance = o.balance.as_deref().map(money);
                let unread = t(crate::house::i18n::Key::SetNotRead);
                f.info(ui, &t(crate::house::i18n::Key::PlStepBalance), match &balance {
                    Some(b) => card::Show::Text(b),
                    None => card::Show::Empty(&unread),
                });
                details_row(ui, f, "pl-goods-raw", &[(&t(crate::house::i18n::Key::IdSeatAuthor), Val::mono(o.seller_key.clone())), (&t(crate::house::i18n::Key::PlSource), Val::mono(o.source.clone())), (&t(crate::house::i18n::Key::PlToken), Val::mono(o.token.clone()))]);
                seller_copied = f.info(ui, &t(crate::house::i18n::Key::IdSeatAuthor), card::Show::Short(&o.seller_key));
                if f.row(ui, &t(crate::house::i18n::Key::PlHistory), "", card::Value::Nav).clicked() {
                    history_of = Some(o.seller_key.clone());
                }
            });
        });
        tell_copied(ui.ctx(), seller_copied);
        if let Some(a) = history_of {
            self.step(Step::PlHistory, &[("a", &a)]);
        }
        if let Some(g) = &o.gap {
            states::banner(ui, states::Banner::Warn, &format!("{} {} · {} {}", t(crate::house::i18n::Key::PlGapTo), eravon_orders::table::short(&o.source), t(crate::house::i18n::Key::PlGapShort), money(g)), |_| ());
        }
        let goods = pl.segment(true);
        let delivery = pl.segment(false);
        if goods.is_some() || delivery.is_some() {
            let st = |s: &Option<crate::house::case::Segment>| s.as_ref().map(text_state).unwrap_or_default();
            let page = goods.as_ref().and_then(|g| g.text.as_deref()).and_then(|x| eravon_goods::refs::read(x).refs.page);
            match super::casepages::description_section(ui, [&st(&goods), &st(&delivery)], page.as_deref()) {
                Some(super::casepages::Asked::Text(g)) => self.step(Step::PdDoc, &[("k", if g { "goods" } else { "delivery" })]),
                Some(super::casepages::Asked::Url(k)) => self.open_foreign(k),
                _ => {}
            }
        }
        card::section(ui, &t(crate::house::i18n::Key::GoodsLadder), "", |ui| {
            card::form(ui, |ui, f| {
                for (i, (b, s, fb)) in o.rungs.iter().enumerate() {
                    let bearer = t(match fb {
                        0 => crate::house::i18n::Key::PlBearerBuyer,
                        1 => crate::house::i18n::Key::PlBearerSeller,
                        _ => crate::house::i18n::Key::PlBearerSplit,
                    });
                    let say = format!("{} {} · {} {} · {bearer}", t(crate::house::i18n::Key::PlToBuyer), pct_word(*b), t(crate::house::i18n::Key::PlToSeller), pct_word(*s));
                    let _ = f.row(ui, &crate::house::i18n::filln(crate::house::i18n::Key::PlRungN, &[&crate::house::buyer_orders::rung_shown(i as u64)]), "", card::Value::Text(&say));
                }
            });
        });
        if let Some(md) = &o.terms_text {
            let human = crate::house::text::human(md);
            card::card(ui, |ui| {
                if !human.trim().is_empty() {
                    desc_block(ui, &human);
                }
                eravon_ui::fold::fold(ui, "pl-terms", &t(crate::house::i18n::Key::PlTerms), |ui| states::note_box(ui, md));
            });
        }
        let mut picked: Option<usize> = None;
        card::section(ui, &t(crate::house::i18n::Key::PlCands), "", |ui| {
            card::form(ui, |ui, f| {
                for c in &o.cands {
                    let on = pl.pick == Some(c.n);
                    let who = format!("{} · {}", t(if c.kind == "guild" { crate::house::i18n::Key::CandsGuild } else { crate::house::i18n::Key::PlCands }), c.label());
                    let m = |v: &str| crate::house::coin::amount(lang, v, o.decimals, &o.token, crate::house::coin::Form::Short);
                    let fees = if c.lf.is_empty() {
                        let why = if c.priced_why.0.is_empty() { String::new() } else { format!(" · {}", crate::house::refusal::line(lang, &c.priced_why.0, &c.priced_why.1)) };
                        format!("{} {} · {}{why}", t(crate::house::i18n::Key::CandsQPremium), m(&c.premium), t(crate::house::i18n::Key::CasePricedAtOpen))
                    } else {
                        let at_open = if c.priced_at_open { format!(" · {}", t(crate::house::i18n::Key::CasePricedAtOpen)) } else { String::new() };
                        format!("{} {} · {} · {} {}{at_open}", t(crate::house::i18n::Key::CandsQPremium), m(&c.premium), crate::house::i18n::filln(crate::house::i18n::Key::PlFeeLfNote, &[&m(&c.lf)]), t(crate::house::i18n::Key::PlSellerDeposit), m(&c.deposit))
                    };
                    let mut note = if c.note.is_empty() { format!("{} {} · {fees}", t(crate::house::i18n::Key::CandsQRuling), span_text(c.ruling_window)) } else { crate::house::paylink::cand_why(c, lang) };
                    if let Some((_, name, why)) = o.deposit_short.iter().find(|(n, ..)| *n == c.n) {
                        let gap = match crate::house::paylink::deposit_gap(why) {
                            Some((vault, have, need)) => crate::house::i18n::filln(crate::house::i18n::Key::PlDepositGap, &[&short(&vault), &have, &need]),
                            None => crate::house::refusal::line(lang, name, why),
                        };
                        note = format!("{} · {gap}", t(crate::house::i18n::Key::PlDepositShort));
                    }
                    let ink = if c.choosable { eravon_ui::palette::c(C::Ink) } else { eravon_ui::palette::c(C::Ink3) };
                    let (resp, _) = f.line(ui, None, &who, &note, false, false, ink, |ui| eravon_ui::sheetparts::radio(ui, &format!("pl-pick-{}", c.n), on));
                    if c.choosable {
                        let hit = ui.interact(resp.rect, egui::Id::new(("eravon-pl-pick", c.n)), egui::Sense::click());
                        if hit.clicked() && !on {
                            picked = Some(c.n);
                        }
                        let _ = hit.on_hover_cursor(egui::CursorIcon::PointingHand);
                    }
                }
                if o.cands.iter().any(|c| c.note.is_empty() && (c.lf.is_empty() || c.priced_at_open)) {
                    f.note(ui, &t(crate::house::i18n::Key::PlPricedAtOpen));
                }
            });
        });
        if let Some(n) = picked {
            self.step(Step::PlPick, &[("n", &n.to_string())]);
        }
        let mut cand_history: Option<String> = None;
        card::section(ui, &t(crate::house::i18n::Key::PlHistory), "", |ui| {
            card::form(ui, |ui, f| {
                for c in &o.cands {
                    if f.row(ui, &c.label(), "", card::Value::Nav).clicked() {
                        cand_history = Some(c.address.clone());
                    }
                }
            });
        });
        if let Some(a) = cand_history {
            self.step(Step::PlHistory, &[("a", &a)]);
        }
        if let Some(c) = pl.pick.and_then(|n| o.cands.iter().find(|c| c.n == n)).filter(|c| c.kind == "guild") {
            card::section(ui, &t(crate::house::i18n::Key::CandsGuildLaw), "", |ui| card::card(ui, |ui| covenant_block(ui, &o, c)));
        }
        let rows: Vec<super::casepages::LapseRow> = pl
            .lapses()
            .into_iter()
            .map(|(a, guild, ck)| super::casepages::LapseRow { key: a.clone(), name: pl.cand_label(&a), guild, clocks: ck })
            .collect();
        if let Some(who) = super::casepages::lapse_section(ui, &rows) {
            self.step(Step::LapseOpen, &[("a", &who)]);
        }
        if pl.company_on {
            self.paylink_company(ui, pl);
            return;
        }
        let mut payout = self.shell.desk.pl_payout.clone().unwrap_or_else(|| pl.payout.clone());
        let why = pl.payout_said.as_ref().map(|s| said_line(&s.name, &crate::house::refusal::line(lang, &s.name, &s.detail)));
        let refused = why.as_deref().map(|w| (pl.payout_tried.as_str(), w));
        let save = payout_card(ui, &mut payout, &pl.payout, refused, &t(crate::house::i18n::Key::PlPayout), &t(crate::house::i18n::Key::PlPayoutSave));
        self.shell.desk.pl_payout = (payout != pl.payout).then(|| payout.clone());
        if save {
            self.step(Step::PlPayout, &[("a", &payout)]);
        }
        let key = pl.pay_key();
        if o.stage_waits && pl.paid.is_none() {
            states::hint(ui, &t(crate::house::i18n::Key::PlStageWaits));
        }
        let can = self.write_banner(ui, crate::house::shell::Write::Chain);
        let mut go = false;
        let why = match &key {
            Err(why) => crate::house::i18n::library(lang, why),
            Ok(_) => String::new(),
        };
        match &pl.paid {
            Some(Ok(p)) => card::card(ui, |ui| {
                match p.stop {
                    "opened" => {
                        states::okline(ui, mark::Mark::Ok, &t(crate::house::i18n::Key::PlPaidOpened));
                        eravon_ui::fold::fold(ui, "pl-paid-raw", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                            kv::kv(ui, &[(&t(crate::house::i18n::Key::SetContractAddr), kv::Val::Mono(p.instance.clone())), (&t(crate::house::i18n::Key::WlHash), kv::Val::Mono(p.open_tx.clone()))]);
                        });
                    }
                    "awaitStage" => states::hint(ui, &t(crate::house::i18n::Key::PlPaidStage)),
                    "short" => states::hint_ex(ui, &format!("{} {} · {} {}", t(crate::house::i18n::Key::PlGapTo), p.short_address, t(crate::house::i18n::Key::PlGapShort), crate::house::coin::amount(lang, &p.gap, p.decimals, &o.token, crate::house::coin::Form::Full)), true),
                    "locked" => {
                        if let Some(s) = &p.stale {
                            said_hint(ui, "pl-paid-locked", lang, &s.name, &s.detail, true);
                        }
                    }
                    _ => {
                        if let Some(s) = &p.stale {
                            states::hint_ex(ui, &format!("{} · {}", t(crate::house::i18n::Key::PlPaidStale), crate::house::refusal::line(lang, &s.name, &s.detail)), true);
                        }
                    }
                }
            }),
            Some(Err(s)) => states::err_box(ui, "pl-pay", &t(crate::house::i18n::Key::PlPayFailed), &crate::house::refusal::line(lang, &s.name, &s.detail), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
            None => {}
        }
        eravon_ui::formparts::page_bar(ui, |ui| {
            go = key::show(ui, key::Key::new(&t(crate::house::i18n::Key::PlTabPay), Role::Primary).enabled(key.is_ok() && can).why(&why)).clicked();
        });
        if go && key.is_ok() && can {
            self.shell.desk.pl_confirm = true;
        }
        if self.shell.desk.pl_confirm && self.pay_card(ui.ctx(), &o, &t(crate::house::i18n::Key::PlTabPay)) {
            self.step(Step::PlPay, &[]);
        }
    }

    fn paylink_company(&mut self, ui: &mut egui::Ui, pl: &crate::house::paylink::PayLink) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let form = pl.form_of();
        let mut chose: Option<eravon_pay::company::Form> = None;
        card::section(ui, &t(crate::house::i18n::Key::SetCompanyMode), "", |ui| card::card(ui, |ui| {
            states::note_box(ui, &t(crate::house::i18n::Key::PlCompanyNote));
            let at = match form {
                Some(eravon_pay::company::Form::Direct) => 1,
                _ => 0,
            };
            let words = [t(crate::house::i18n::Key::PlFormRegistered), t(crate::house::i18n::Key::PlFormDirect)];
            let cells: Vec<eravon_ui::seg::Cell> = words.iter().map(|w| eravon_ui::seg::Cell { text: w, count: None }).collect();
            if let Some(i) = eravon_ui::seg::seg(ui, "pl-form", &cells, at) {
                chose = Some(if i == 1 { eravon_pay::company::Form::Direct } else { eravon_pay::company::Form::Registered });
            }
        }));
        if let Some(f) = chose {
            self.step(Step::PlForm, &[("v", f.name())]);
        }
        let key = pl.pay_key();
        let can = self.write_banner(ui, crate::house::shell::Write::Local);
        let mut go = false;
        card::card(ui, |ui| {
            keys_row(ui, |ui| {
                go = key::key(ui, &t(crate::house::i18n::Key::OrdersCompanyExportKey), Role::Primary, key.is_ok() && can).clicked();
            });
            if let Err(why) = key {
                states::hint(ui, &crate::house::i18n::library(lang, why));
            }
        });
        if go && key.is_ok() {
            self.shell.desk.pl_confirm = true;
        }
        let opened = pl.verified().as_ref().and_then(|x| x.as_ref().ok()).cloned();
        if let Some(o) = opened {
            if self.shell.desk.pl_confirm && self.pay_card(ui.ctx(), &o, &t(crate::house::i18n::Key::OrdersCompanyExportKey)) {
                self.step(Step::PlPay, &[]);
            }
        }
        let mut open = false;
        match &pl.company {
            Some(Ok(v)) => card::card(ui, |ui| {
                eravon_ui::states::okline(ui, eravon_ui::mark::Mark::Ok, &format!("{} · {}", t(crate::house::i18n::Key::PlCompanyExported), eravon_ui::width::file_name(&v.path)));
                for (i, line) in v.face.iter().enumerate() {
                    states::hint_ex(ui, &super::split_raw(line).0, v.time_short && i + 1 == v.face.len());
                }
                paint::text(ui, &t(crate::house::i18n::Key::PlCompanyReceipt), eravon_ui::tokens::Type::Note, c(C::Ink2));
                states::note_box(ui, &v.receipt);
                if key::key(ui, &t(crate::house::i18n::Key::PlCopyReceipt), Role::Secondary, !v.receipt.is_empty()).clicked() {
                    ui.ctx().copy_text(v.receipt.clone());
                    tell_copied(ui.ctx(), Some(v.receipt.clone()));
                }
                eravon_ui::fold::fold(ui, "pl-company-raw", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                    kv::kv(ui, &[(&t(crate::house::i18n::Key::PlCompanyExportedAt), kv::Val::Mono(v.path.clone())), (&t(crate::house::i18n::Key::PlOrderFile), kv::Val::Mono(v.order_file.clone())), (&t(crate::house::i18n::Key::SetContractAddr), kv::Val::Mono(v.instance.clone()))]);
                });
                if v.form == eravon_pay::company::Form::Registered.name() {
                    card_keys(ui, |ui| {
                        open = key::key(ui, &t(crate::house::i18n::Key::PlCompanyOpen), Role::Secondary, true).clicked();
                    });
                }
            }),
            Some(Err(s)) => states::err_box(ui, "pl-company", &t(crate::house::i18n::Key::PlPayFailed), &crate::house::refusal::line(lang, &s.name, &s.detail), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
            None => {}
        }
        if let Some(Err(s)) = &pl.paid {
            states::err_box(ui, "pl-company-fork", &t(crate::house::i18n::Key::PlPayFailed), &crate::house::refusal::line(lang, &s.name, &s.detail), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail));
        }
        match &pl.co_open {
            Some(Ok(tx)) => card::card(ui, |ui| {
                eravon_ui::states::okline(ui, eravon_ui::mark::Mark::Ok, &t(crate::house::i18n::Key::OrdersSentDeploy));
                eravon_ui::fold::fold(ui, "pl-co-open-raw", &t(crate::house::i18n::Key::ToastDetail), |ui| ui.add(egui::Label::new(egui::RichText::new(tx).font(eravon_ui::tokens::Type::Mono.font()).color(c(C::Ink2))).selectable(true).wrap()));
            }),
            Some(Err(s)) => states::err_box(ui, "pl-company-open", &t(crate::house::i18n::Key::PlPayFailed), &crate::house::refusal::line(lang, &s.name, &s.detail), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
            None => {}
        }
        if open {
            self.step(Step::PlCompanyOpen, &[]);
        }
    }

    fn paylink_import(&mut self, ui: &mut egui::Ui, pl: &crate::house::paylink::PayLink) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let i = &pl.import;
        let mut changed = false;
        eravon_ui::fields::card(ui, |ui, st| {
            let chosen = i.file.as_ref().map(|(p, bytes)| (p.as_str(), bytes.len() as u64));
            let d = super::pick_zone(ui, "pl-drop", &t(crate::house::i18n::Key::PlDropTitle), &t(crate::house::i18n::Key::DropClickFile), chosen, lang);
            if d.clicked {
                crate::house::platform::dialog_starts_in(&i.at);
            }
            if let Some(p) = self.drop_or_pick(&ui.ctx().clone(), "pl-drop", &d, crate::platform::Pick::File, self.ux.now) {
                self.step(Step::PlFile, &[("path", &p)]);
                changed = true;
            }
            if let Some(s) = &i.said {
                said_hint(ui, "paylink-said-1", lang, &s.name, &s.detail, s.tone == crate::house::seller::Tone::Bad);
            }
            let ready = i.file.is_some();
            let go = st.foot(ui, |ui| key::key(ui, &t(crate::house::i18n::Key::SetImport), Role::Primary, ready).clicked());
            if go && ready {
                self.step(Step::PlImport, &[]);
                changed = true;
            }
        });
        if changed {
            self.shell.refresh_addr();
        }
    }
}

fn text_state(s: &crate::house::case::Segment) -> String {
    crate::house::i18n::t(match (s.on_board, s.text.is_some()) {
        (true, _) => crate::house::i18n::Key::GoodsTextOnBoard,
        (false, true) => crate::house::i18n::Key::PlSegHeld,
        (false, false) => crate::house::i18n::Key::PlSegMissing,
    })
}

fn seg_name(lang: crate::lang::Lang, seg: &str) -> String {
    crate::house::i18n::get(lang, if seg == "goods" { crate::house::i18n::Key::PlSegGoods } else { crate::house::i18n::Key::GoodsDeliveryNote }).0
}

fn payout_card(ui: &mut egui::Ui, payout: &mut String, accepted: &str, refused: Option<(&str, &str)>, name: &str, use_it: &str) -> bool {
    let held = refused.filter(|(tried, _)| payout.as_str() == *tried);
    let why = held.map(|(_, w)| w).filter(|_| !payout.trim().is_empty());
    let mut save = false;
    eravon_ui::fields::card(ui, |ui, st| {
        let line = name_line(ui);
        let cell = st.field(ui, name, None, None, |ui, w| {
            let changed = payout.as_str() != accepted || refused.is_some();
            let look = eravon_ui::fields::Look { mono: true, bad: held.is_some(), ..eravon_ui::fields::Look::default() };
            let (cell, hit) = eravon_ui::fields::joined(ui, payout, "0x…", w, look, changed.then_some(|ui: &mut egui::Ui| eravon_ui::fields::inner_key(ui, use_it, None)));
            save = hit.is_some_and(|r| r.clicked());
            cell
        });
        if let Some(w) = why {
            field_why(ui, line, name, w, cell);
        }
    });
    save
}

fn covenant_block(ui: &mut egui::Ui, o: &crate::house::paylink::Opened, c: &crate::house::paylink::Cand) {
    let lang = crate::lang::lang();
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    if let Some(Some(q)) = o.quotes.get(c.n) {
        let x = eravon_cands::quote::cells(q, &o.token, &[(o.token.clone(), o.decimals)], lang);
        let rows = [
            (t(crate::house::i18n::Key::CandsQPremium), x.premium),
            (t(crate::house::i18n::Key::OrdersKvLf), x.lawsuit),
            (t(crate::house::i18n::Key::CandsQRuling), x.ruling_days),
            (t(crate::house::i18n::Key::CandsQLapse), x.lapse),
            (t(crate::house::i18n::Key::CandsQCap), x.cap),
            (t(crate::house::i18n::Key::CandsQAccept), x.accept),
        ];
        let refs: Vec<(&str, kv::Val)> = rows.iter().map(|(k, v)| (k.as_str(), kv::Val::Text(v.clone()))).collect();
        kv::kv(ui, &refs);
        if !x.reference.is_empty() {
            states::hint(ui, &x.reference);
        }
    }
    match &c.covenant {
        Some(Ok(text)) => {
            desc_block(ui, text);
            states::hint(ui, &t(crate::house::i18n::Key::PlCovenantSay));
        }
        Some(Err(why)) => states::hint_ex(ui, &format!("{} · {}", t(crate::house::i18n::Key::PlCovenantUnread), crate::house::i18n::library(lang, why)), true),
        None => eravon_ui::states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO),
    }
}

pub(super) fn pct_word(bps: u64) -> String {
    let (i, f) = (bps / 100, bps % 100);
    if f == 0 { format!("{i}%") } else if f % 10 == 0 { format!("{i}.{}%", f / 10) } else { format!("{i}.{f:02}%") }
}

impl Win {
    fn pay_card(&mut self, ctx: &egui::Context, o: &crate::house::paylink::Opened, go_word: &str) -> bool {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let money = |v: &str| crate::house::coin::amount(lang, v, o.decimals, &o.token, crate::house::coin::Form::Short);
        let full = crate::house::coin::amount(lang, &o.amount, o.decimals, &o.token, crate::house::coin::Form::Full);
        let chosen = self.shell.desk.sh.paylink.as_ref().and_then(|p| p.pick).and_then(|n| o.cands.iter().find(|c| c.n == n)).cloned();
        let held: Vec<&'static str> = self.shell.desk.sh.paylink.as_ref().map(|p| p.held.iter().map(|(s, _)| *s).collect()).unwrap_or_default();
        let missing: Vec<String> = o.segments.iter().filter(|(s, _)| !held.contains(s)).map(|(s, _)| seg_name(lang, s)).collect();
        let pick = chosen.as_ref().map(|c| format!("{} · {}", t(if c.kind == "guild" { crate::house::i18n::Key::CandsGuild } else { crate::house::i18n::Key::PlCands }), c.label())).unwrap_or_default();
        let lapses = self.shell.desk.sh.paylink.as_ref().map(|p| p.lapses()).unwrap_or_default();
        let lapse = chosen
            .as_ref()
            .and_then(|c| lapses.iter().find(|(a, _, _)| a.eq_ignore_ascii_case(&c.address)))
            .map(|(_, _, ck)| match ck {
                Ok(k) => super::casepages::lapse_word(k.lapse),
                Err(e) => e.clone(),
            })
            .unwrap_or_default();
        let fees: Vec<(String, kv::Val)> = match &chosen {
            Some(c) if c.lf.is_empty() => vec![(t(crate::house::i18n::Key::CandsQPremium), kv::Val::Text(money(&c.premium))), (t(crate::house::i18n::Key::OrdersKvLf), kv::Val::Text(t(crate::house::i18n::Key::PlPricedAtOpen)))],
            Some(c) => {
                let lf = crate::house::i18n::filln(crate::house::i18n::Key::PlFeeLfNote, &[&money(&c.lf)]);
                vec![
                    (t(crate::house::i18n::Key::CandsQPremium), kv::Val::Text(money(&c.premium))),
                    (t(crate::house::i18n::Key::OrdersKvLf), kv::Val::Text(if c.priced_at_open { format!("{lf} · {}", t(crate::house::i18n::Key::PlPricedAtOpen)) } else { lf })),
                    (t(crate::house::i18n::Key::PlSellerDeposit), kv::Val::Text(crate::house::i18n::filln(crate::house::i18n::Key::PlDepositSay, &[&money(&c.deposit)]))),
                ]
            }
            None => Vec::new(),
        };
        let (_, go, back) = Self::sheet_with(ctx, "pl-pay-card", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, go_word, "");
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            kv::kv(ui, &[
                (&t(crate::house::i18n::Key::PlTitle), kv::Val::Text(o.title.clone())),
                (&t(crate::house::i18n::Key::OrdersAmount), kv::Val::Text(money(&o.amount))),
                (&t(crate::house::i18n::Key::GoodsT1), kv::Val::Text(format!("{} · {} {}", span_text(o.t1), t(crate::house::i18n::Key::GoodsT2), span_text(o.t2)))),
                (&t(crate::house::i18n::Key::PlPickedCand), kv::Val::Text(pick.clone())),
                (&t(crate::house::i18n::Key::CandsQLapse), kv::Val::Text(lapse.clone())),
                (&t(crate::house::i18n::Key::PlSource), kv::Val::Text(eravon_orders::table::short(&o.source))),
            ]);
            let refs: Vec<(&str, kv::Val)> = fees.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            kv::kv(ui, &refs);
            for name in &missing {
                states::note_box(ui, &crate::house::i18n::filln(crate::house::i18n::Key::PlSegWithout, &[name]));
            }
            if let Some(c) = chosen.as_ref().filter(|c| c.kind == "guild") {
                eravon_ui::fold::fold(ui, "pl-pay-card-covenant", &t(crate::house::i18n::Key::CandsGuildLaw), |ui| covenant_block(ui, o, c));
            }
            eravon_ui::fold::fold(ui, "pl-pay-card-raw", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                kv::kv(ui, &[(&t(crate::house::i18n::Key::OrdersAmount), kv::Val::Mono(full.clone())), (&t(crate::house::i18n::Key::AmtRaw), kv::Val::Mono(o.amount.clone())), (&t(crate::house::i18n::Key::IdSeatAuthor), kv::Val::Mono(o.seller_key.clone())), (&t(crate::house::i18n::Key::PlSource), kv::Val::Mono(o.source.clone()))]);
            });
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((go_word, Role::Commit, true)));
        });
        if go || back {
            self.shell.desk.pl_confirm = false;
        }
        go
    }
}

fn step_why(lang: crate::lang::Lang, step: &str, name: &str, detail: &str, held: Option<&str>) -> String {
    let words = crate::house::refusal::line(lang, name, detail);
    match held.filter(|_| step == crate::house::seam::PL_STEP_SERIES) {
        Some(i) => format!("{words} · {}", crate::house::orders::order_label(i)),
        None => words,
    }
}

#[cfg(test)]
mod step_tests {
    #[test]
    fn a_failed_check_says_the_tables_words_and_the_orders_short_name() {
        let held = "0x5e1e5000000000000000000000000000000000b7";
        let detail = format!("这个系列的第 2 段已有订单 {held}:接着办那一单,不另付一单");
        for lang in [crate::lang::Lang::Zh, crate::lang::Lang::En] {
            let why = super::step_why(lang, crate::house::seam::PL_STEP_SERIES, eravon_cli::buyer::E_SERIES_SAME, &detail, Some(held));
            assert!(why.starts_with(&crate::house::refusal::line(lang, eravon_cli::buyer::E_SERIES_SAME, &detail)), "{lang:?}: {why}");
            assert!(why.ends_with(&eravon_orders::table::short(held)) && !why.contains(held), "{lang:?}: {why}");
            if lang == crate::lang::Lang::En {
                assert!(!why.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{why}");
            }
        }
        let book = super::step_why(crate::lang::Lang::En, crate::house::seam::PL_STEP_BOOK, "E_V_DOWN", "下层拒了", None);
        assert!(!book.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{book}");
    }
}

#[cfg(test)]
mod refund_cell {
    use super::*;

    const NAME: &str = "Refund address";
    const KEY: &str = "Use this address";
    const WHY: &str = "0x12 is not forty hexadecimal digits";
    const TAKEN: &str = "0x00000000000000000000000000000000000000aa";

    struct Read {
        foot: f32,
        rung: bool,
        texts: Vec<crate::window::faces::Drawn>,
    }

    fn rung_in(s: &egui::Shape, bad: egui::Color32) -> bool {
        match s {
            egui::Shape::Vec(v) => v.iter().any(|x| rung_in(x, bad)),
            egui::Shape::Rect(r) => r.stroke.color == bad && r.stroke.width > f32::default(),
            _ => false,
        }
    }

    fn read(width: f32, cell: &str, refused: Option<(&str, &str)>) -> Read {
        let ctx = egui::Context::default();
        let _ = eravon_ui::skin::dress(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, crate::window::H));
        let (mut foot, mut bad) = (f32::default(), None);
        let mut out = None;
        for _ in 0..2 {
            let mut text = cell.to_string();
            out = Some(ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |root| {
                egui::CentralPanel::default().show_inside(root, |ui| {
                    let _ = payout_card(ui, &mut text, TAKEN, refused, NAME, KEY);
                    foot = ui.cursor().min.y;
                    bad = Some(c(C::Bad));
                });
            }));
        }
        let out = out.expect("a frame was drawn");
        let bad = bad.expect("the frame read the warning colour");
        Read { foot, rung: out.shapes.iter().any(|cl| rung_in(&cl.shape, bad)), texts: crate::window::faces::texts_of(&out) }
    }

    fn slack() -> f32 {
        Type::Note.line() - Type::Small.line()
    }

    fn wide() -> f32 {
        crate::window::W
    }

    fn narrow() -> f32 {
        crate::window::W / f32::from(3u8)
    }

    fn named(r: &Read) -> &crate::window::faces::Drawn {
        r.texts.iter().find(|d| d.text == NAME).expect("the field's name is drawn")
    }

    #[test]
    fn a_refused_address_rings_the_cell_and_says_why_on_the_names_line() {
        let plain = read(wide(), TAKEN, None);
        let said = read(wide(), "0x12", Some(("0x12", WHY)));
        let empty = read(wide(), "", Some(("", WHY)));
        assert_eq!(plain.foot, said.foot, "the card is as tall with a reason as without");
        assert_eq!(plain.foot, empty.foot, "and with an empty cell refused");
        assert!(!plain.rung, "no ring without a refusal");
        assert!(said.rung && empty.rung, "a refused cell is rung, filled or empty");
        let why = said.texts.iter().find(|d| d.text == WHY).expect("the reason is drawn whole where it fits");
        let name = named(&said);
        assert_eq!(why.rows, 1);
        assert!((why.rect.center().y - name.rect.center().y).abs() <= slack(), "on the name's line: {:?} against {:?}", why.rect, name.rect);
        assert!(why.rect.left() > name.rect.right(), "after the name");
        let cell = said.texts.iter().find(|d| d.text == "0x12").expect("the cell's words are drawn");
        assert!(said.texts.iter().all(|d| d.rect.top() < cell.rect.bottom() + tk::S3), "nothing is drawn under the cell: {:?}", said.texts.iter().map(|d| (&d.text, d.rect)).collect::<Vec<_>>());
    }

    #[test]
    fn an_empty_cell_is_rung_without_words_and_an_edit_takes_both_away() {
        let empty = read(wide(), "", Some(("", WHY)));
        assert!(empty.rung);
        assert!(empty.texts.iter().all(|d| !d.text.contains("forty")), "no reason over an empty cell");
        let edited = read(wide(), "0x123", Some(("0x12", WHY)));
        assert!(!edited.rung, "the cell no longer holds what was refused");
        assert!(edited.texts.iter().all(|d| !d.text.contains("forty")));
        assert!(edited.texts.iter().any(|d| d.text == KEY), "the key stands to take the address again");
        assert!(read(wide(), TAKEN, Some(("0x12", WHY))).texts.iter().any(|d| d.text == KEY), "and with the accepted address back in the cell, to clear the refusal");
        assert!(read(wide(), TAKEN, None).texts.iter().all(|d| d.text != KEY), "no key while there is nothing to take");
    }

    #[test]
    fn a_long_reason_is_cut_short_on_its_line() {
        let long = "this address is not forty hexadecimal digits and its checksum does not read either, write it again";
        let plain = read(narrow(), TAKEN, None);
        let said = read(narrow(), "0x12", Some(("0x12", long)));
        assert_eq!(plain.foot, said.foot);
        let name = named(&said);
        let cut = said.texts.iter().find(|d| d.text.ends_with('…') && long.starts_with(d.text.trim_end_matches('…'))).expect("the reason is cut with an ellipsis");
        assert_eq!(cut.rows, 1);
        assert!(cut.rect.left() > name.rect.right(), "it does not run over the name");
        assert!((cut.rect.center().y - name.rect.center().y).abs() <= slack());
    }
}
