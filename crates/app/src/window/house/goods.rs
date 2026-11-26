use super::*;
use crate::house::action::Action as Step;

pub(super) fn cell_key(cell: &str) -> &str {
    match cell {
        c if c.starts_with("ladder") => "ladder",
        "t1h" | "t1m" => "t1",
        "t2h" | "t2m" => "t2",
        c => c,
    }
}

impl Win {
    pub(super) fn goods_page(&mut self, ui: &mut egui::Ui) {
        let Some(gd) = self.shell.desk.sh.seller.goods.clone() else {
            if self.shell.house_in_flight(crate::task::Kind::Goods) {
                Self::home_skeleton(ui, 0);
            }
            return;
        };
        if gd.edit.is_some() {
            self.goods_edit(ui);
            self.leave_sheet(ui.ctx());
            return;
        }
        if gd.shown.is_some() {
            self.goods_shown(ui);
            self.qr_card(ui.ctx());
            return;
        }
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let mut q = self.shell.desk.goods_search.clone().unwrap_or_default();
        let w = ui.available_width();
        if input::search(ui, &mut q, &t(crate::house::i18n::Key::GoodsSearchHint), w).changed() {
            self.step(Step::GoodsSearch, &[("q", &q)]);
        }
        let mut open: Option<String> = None;
        match &gd.rows {
            Err(s) => states::err_box(ui, "goods", &t(crate::house::i18n::Key::GoodsListFailed), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
            Ok(rows) => {
                let heads = [t(crate::house::i18n::Key::PlTitle), t(crate::house::i18n::Key::GoodsPrice), t(crate::house::i18n::Key::SetZkState), t(crate::house::i18n::Key::CandsJudges)];
                let cols = [
                    table::col(&heads[0], eravon_ui::house::REST),
                    table::col(&heads[2], table::Col::Auto { min: eravon_ui::house::COL_STATE, max: f32::MAX }),
                    table::col(&heads[3], table::Col::Px(eravon_ui::house::COL_JUDGES)),
                    table::col_r(&heads[1], table::Col::Px(eravon_ui::house::COL_PRICE)),
                    table::CHEV,
                ];
                let make = |i: usize| {
                        let r = &rows[i];
                        let name = if r.name.trim().is_empty() { t(crate::house::i18n::Key::GoodsUnnamed) } else { r.name.clone() };
                        let price = if r.price.is_empty() { "—".to_string() } else { format!("{} {}", r.price, token_shown(lang, &r.token)) };
                        let (word, tone) = if r.draft {
                            (t(crate::house::i18n::Key::GoodsDraft), eravon_ui::palette::Tone::Grey)
                        } else if !r.listed {
                            (t(crate::house::i18n::Key::GoodsLinkOff), eravon_ui::palette::Tone::Grey)
                        } else if !r.pending.is_empty() {
                            (t(crate::house::i18n::Key::GoodsPending), eravon_ui::palette::Tone::Warn)
                        } else {
                            (t(crate::house::i18n::Key::HomeOnSale), eravon_ui::palette::Tone::Ok)
                        };
                        let names = if r.cands.is_empty() { "—".to_string() } else { r.cands.join(&t(crate::house::i18n::Key::ListJoin)) };
                        table::Row { cells: vec![table::Cell::Text({
                            let body = crate::house::seller::goods::list_line(&eravon_goods::refs::read(&r.description).body);
                            let mut head = name.to_string();
                            if let Some(sr) = &r.series {
                                head = format!("{head} · {}", crate::house::i18n::filln(crate::house::i18n::Key::GoodsSeriesStage, &[&sr.stage.to_string(), &sr.stages.to_string()]));
                            }
                            if body.trim().is_empty() { head } else { format!("{} · {}", head, body.trim()) }
                        }), table::Cell::Pill(word, tone), table::Cell::Text(names), table::Cell::Mono(price), table::Cell::Chev], click: true, gone: false, on: false }
                };
                let none = t(crate::house::i18n::Key::HomeNone);
                if let Some(i) = table::table_with(ui, "goods", table::Spec::new(&cols, self.shell.desk.rows_gen).head(true).empty(&none), rows.len(), make).clicked {
                    open = Some(rows[i].id.clone());
                }
            }
        }
        if let Some(id) = open {
            self.step(Step::PdOpen, &[("id", &id)]);
        }
    }

    pub(super) fn goods_shown(&mut self, ui: &mut egui::Ui) {
        self.shell.goods_taken();
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(s) = self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.shown.clone()) else { return };
        let v = match &s.view {
            Err(x) => {
                states::err_box(ui, "goods-shown", &t(crate::house::i18n::Key::GoodsListFailed), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&x.name, &x.detail));
                return;
            }
            Ok(v) => v.clone(),
        };
        if let Some(inner) = s.inner.clone() {
            self.goods_inner(ui, &s, &v, inner);
            return;
        }
        let mut act: Option<&'static str> = None;
        let d = &v.draft;
        let name = if v.row.name.trim().is_empty() { t(crate::house::i18n::Key::GoodsUnnamed) } else { v.row.name.clone() };
        let price = if d.price.is_empty() { t(crate::house::i18n::Key::SetNone) } else { format!("{} {}", d.price, token_shown(lang, &d.token)) };
        let (word, tone) = if v.row.draft {
            (t(crate::house::i18n::Key::GoodsDraft), eravon_ui::palette::Tone::Warn)
        } else if v.awaiting_company() {
            (t(crate::house::i18n::Key::GoodsLinkAwaiting), eravon_ui::palette::Tone::Warn)
        } else if !v.row.listed {
            (t(crate::house::i18n::Key::GoodsLinkOff), eravon_ui::palette::Tone::Grey)
        } else if !v.pending.is_empty() {
            (t(crate::house::i18n::Key::GoodsPending), eravon_ui::palette::Tone::Warn)
        } else {
            (t(crate::house::i18n::Key::HomeOnSale), eravon_ui::palette::Tone::Ok)
        };
        self.hero_keys(ui, &name, &format!("{} · {price}", t(crate::house::i18n::Key::NavGoods)), &[hero_pill(&word, tone)]);
        if !v.row.draft && v.editing {
            states::banner(ui, states::Banner::Note, &t(crate::house::i18n::Key::GoodsEditingDraft), |_| ());
        }
        let why: Vec<String> = v.pending.iter().filter_map(|p| resign_why(p)).map(t).collect();
        if !why.is_empty() {
            states::banner(ui, states::Banner::Warn, &format!("{}{}", t(crate::house::i18n::Key::GoodsPendingNote), why.join(&t(crate::house::i18n::Key::ListJoin))), |_| ());
        }
        let judge_names: Vec<String> = d.cands.iter().map(|a| self.cand_label(a)).collect();
        card::section(ui, &t(crate::house::i18n::Key::GoodsBasics), "", |ui| {
            let copied = card::form(ui, |ui, f| {
                f.info(ui, &t(crate::house::i18n::Key::GoodsPrice), card::Show::Text(&price));
                for (cell, k) in [("t1", crate::house::i18n::Key::GoodsT1), ("t2", crate::house::i18n::Key::GoodsT2)] {
                    match window_secs(d, cell) {
                        Some(x) => f.info(ui, &t(k), card::Show::Text(&span_text(x))),
                        None => f.info(ui, &t(k), card::Show::Empty(&t(crate::house::i18n::Key::SetNone))),
                    };
                }
                let ladder: Vec<String> = d.ladder.iter().map(|l| format!("{} {}%", l.name, l.pct)).collect();
                let (unset, none, ladder_line) = (t(crate::house::i18n::Key::SetNone), t(crate::house::i18n::Key::PickNone), ladder.join(" · "));
                f.info(ui, &t(crate::house::i18n::Key::GoodsLadder), if ladder.is_empty() { card::Show::Empty(&unset) } else { card::Show::Text(&ladder_line) });
                let names = judge_names.join(&t(crate::house::i18n::Key::ListJoin));
                f.info(ui, &t(crate::house::i18n::Key::CandsJudges), if names.is_empty() { card::Show::Empty(&none) } else { card::Show::Text(&names) });
                let tl = t(crate::house::i18n::Key::GoodsToken);
                let labels: Vec<String> = (0..d.cands.len()).map(|i| crate::house::i18n::filln(crate::house::i18n::Key::GoodsCandidate, &[&(i + 1).to_string()])).collect();
                let mut raw: Vec<(&str, kv::Val)> = vec![(tl.as_str(), kv::Val::Mono(d.token.clone()))];
                for (i, a) in d.cands.iter().enumerate() {
                    raw.push((labels[i].as_str(), kv::Val::Mono(a.clone())));
                }
                let (detail, widest) = detail_rows_of(ui, &raw);
                f.details(ui, "goods-basics-raw", &t(crate::house::i18n::Key::SetEvidence), &detail, widest)
            });
            tell_copied(ui.ctx(), copied);
        });
        let page = s.texts().and_then(|(g, _)| eravon_goods::refs::read(&g).refs.page);
        let states_words = if v.row.draft {
            let filled = |x: &str| t(if x.trim().is_empty() { crate::house::i18n::Key::GoodsNotFilled } else { crate::house::i18n::Key::GoodsFilled });
            [filled(&d.description), filled(&d.delivery_note)]
        } else {
            [super::casepages::text_state(!d.goods_private), super::casepages::text_state(!d.delivery_private)]
        };
        match super::casepages::description_section(ui, [&states_words[0], &states_words[1]], page.as_deref()) {
            Some(super::casepages::Asked::Text(goods)) if v.row.draft => {
                self.step(Step::PdEdit, &[]);
                self.step(Step::PeTab, &[("i", if goods { "0" } else { "1" })]);
                return;
            }
            Some(super::casepages::Asked::Text(goods)) => self.step(Step::PdDoc, &[("k", if goods { "goods" } else { "delivery" })]),
            Some(super::casepages::Asked::Url(k)) => self.open_foreign(k),
            _ => {}
        }
        let rows: Vec<super::casepages::LapseRow> = s.lapses().into_iter().map(|(c, clocks)| super::casepages::LapseRow { name: self.cand_label(&c), guild: c.starts_with("galeed:"), clocks, key: c }).collect();
        if let Some(c) = super::casepages::lapse_section(ui, &rows) {
            self.step(Step::LapseOpen, &[("a", &c)]);
        }
        if !v.row.draft {
            let current = v.current_link();
            let post_due = current.is_none() && v.pending.iter().any(|x| x == "post") && v.revoked.is_none() && v.company.is_none();
            let (state, mark) = match current {
                Some(_) if v.row.listed && v.awaiting_company() => (t(crate::house::i18n::Key::GoodsLinkAwaiting), Mark::Warn),
                Some(_) if v.row.listed => (t(crate::house::i18n::Key::GoodsLinkLive), Mark::Ok),
                Some(_) => (t(crate::house::i18n::Key::GoodsLinkOff), Mark::Todo),
                None if !crate::house::refusal::line(lang, &v.link_state, "").is_empty() => (format!("{} · {}", t(crate::house::i18n::Key::GoodsNoLink), crate::house::refusal::line(lang, &v.link_state, "")), Mark::Warn),
                None => (t(crate::house::i18n::Key::GoodsNoLink), Mark::Warn),
            };
            let sub = current.filter(|l| !l.posted_tx.is_empty()).map(|l| crate::house::i18n::filln(crate::house::i18n::Key::GoodsPostedAt, &[&eravon_orders::table::short(&l.posted_tx)])).unwrap_or_else(|| if post_due { t(crate::house::i18n::Key::GoodsPostPending) } else { String::new() });
            card::section(ui, &t(crate::house::i18n::Key::TaskPaylink), "", |ui| {
                let copied = card::form(ui, |ui, f| {
                    let _ = f.line(ui, Some(card::Lead::Mark(mark)), &state, &sub, false, false, c(C::Ink), |ui| {
                        if post_due {
                            if key::key(ui, &t(crate::house::i18n::Key::GoodsPostKey), Role::Primary, true).clicked() {
                                act = Some(crate::house::seam::STEP_PD_POST);
                            }
                        } else {
                            if key::key(ui, &t(crate::house::i18n::Key::GoodsCopyLink), Role::Primary, current.is_some()).clicked() {
                                act = Some(crate::house::seam::STEP_PD_COPY);
                            }
                            if key::key(ui, &t(crate::house::i18n::Key::GoodsQr), Role::Secondary, current.is_some()).clicked() {
                                act = Some(crate::house::seam::STEP_PD_QR);
                            }
                        }
                    });
                    if !v.row.listed && v.revoked.is_none() {
                        f.note(ui, &t(crate::house::i18n::Key::GoodsUnlistHint));
                        match &s.taken {
                            Some(Ok(list)) if list.is_empty() => {
                                f.info(ui, &t(crate::house::i18n::Key::GoodsTaken), card::Show::Empty(&t(crate::house::i18n::Key::GoodsTakenNone)));
                            }
                            Some(Ok(list)) => {
                                f.info(ui, &t(crate::house::i18n::Key::GoodsTaken), card::Show::Text(&list.join(&t(crate::house::i18n::Key::ListJoin))));
                            }
                            Some(Err(e)) => f.note(ui, &said_line(&e.name, &crate::house::refusal::line(lang, &e.name, &e.detail))),
                            None => f.free(ui, |ui| eravon_ui::states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO)),
                        }
                    }
                    if !v.fees_new.is_empty() {
                        f.note(ui, &t(crate::house::i18n::Key::GoodsFeesNewHint));
                        for nf in &v.fees_new {
                            let who = self.cand_label(&nf.candidate);
                            if let Some(old) = v.fees_signed.iter().find(|o| o.candidate.eq_ignore_ascii_case(&nf.candidate)) {
                                f.info(ui, &format!("{who} · {}", t(crate::house::i18n::Key::GoodsFeesWas)), card::Show::Text(&fee_line(lang, old)));
                            }
                            f.info(ui, &format!("{who} · {}", t(crate::house::i18n::Key::GoodsFeesNow)), card::Show::Text(&fee_line(lang, nf)));
                        }
                        if priced_at_open(&v.fees_new) || priced_at_open(&v.fees_signed) {
                            f.note(ui, &t(crate::house::i18n::Key::PlPricedAtOpen));
                        }
                        if f.act(ui, &t(crate::house::i18n::Key::GoodsFeesNewTitle), "", key::Key::new(&t(crate::house::i18n::Key::GoodsFeesConfirmKey), Role::Commit)).clicked() {
                            act = Some(crate::house::seam::STEP_PD_FEES_CONFIRM);
                        }
                    }
                    let (ll, ol) = (t(crate::house::i18n::Key::TaskPaylink), t(crate::house::i18n::Key::GoodsOlderLinks));
                    let older: Vec<String> = v.links.iter().filter(|l| Some(*l) != current).map(|l| format!("{ol} v{} · {}", l.version, date_of(Some(l.expiry)))).collect();
                    let mut raw: Vec<(&str, kv::Val)> = Vec::new();
                    if let Some(l) = current {
                        raw.push((ll.as_str(), kv::Val::Mono(l.link.clone())));
                    }
                    for (i, l) in v.links.iter().filter(|l| Some(*l) != current).enumerate() {
                        raw.push((older[i].as_str(), kv::Val::Mono(l.link.clone())));
                    }
                    if raw.is_empty() {
                        return None;
                    }
                    let (detail, widest) = detail_rows_of(ui, &raw);
                    f.details(ui, "goods-link-det", &t(crate::house::i18n::Key::SetEvidence), &detail, widest)
                });
                tell_copied(ui.ctx(), copied);
            });
        }
        if let Some((_, address, digests)) = &v.company {
            card::section(ui, &t(crate::house::i18n::Key::GoodsCompany), "", |ui| {
                let copied = card::form(ui, |ui, f| {
                    let head = if v.awaiting_company() { t(crate::house::i18n::Key::GoodsLinkAwaiting) } else { t(crate::house::i18n::Key::GoodsCompanyPassed) };
                    if f.act(ui, &head, "", key::Key::new(&t(crate::house::i18n::Key::OrdersCompanyExportKey), Role::Primary)).clicked() {
                        self.shell.desk.goods_confirm = Some(crate::house::seam::STEP_PD_EXPORT_PROPOSAL);
                    }
                    for (i, dg) in digests.iter().enumerate() {
                        let st = match &s.regs {
                            Some(Ok(list)) => list.iter().find(|r| r.digest == *dg).map_or_else(|| t(crate::house::i18n::Key::SetCoNotChecked), |r| crate::house::i18n::get_named(lang, &format!("goods.reg.{}", r.state)).0),
                            _ => t(crate::house::i18n::Key::SetCoNotChecked),
                        };
                        let who = judge_names.get(i).cloned().unwrap_or_else(|| crate::house::i18n::filln(crate::house::i18n::Key::GoodsCandidate, &[&(i + 1).to_string()]));
                        f.info(ui, &who, card::Show::Text(&st));
                    }
                    match &s.exported {
                        Some(Ok(p)) => {
                            f.info(ui, &t(crate::house::i18n::Key::GoodsProposalAt), card::Show::Text(&eravon_ui::width::file_name(p)));
                        }
                        Some(Err(e)) => f.note(ui, &super_split(&crate::house::refusal::line(lang, &e.name, &e.detail))),
                        None => {}
                    }
                    if f.act(ui, &t(crate::house::i18n::Key::GoodsCheckRegistry), "", key::Key::new(&t(crate::house::i18n::Key::GoodsCheckRegistry), Role::Secondary).enabled(v.awaiting_company())).clicked() {
                        act = Some(crate::house::seam::STEP_PD_COMPANY_CHECK);
                    }
                    let al = t(crate::house::i18n::Key::SetCoAddress);
                    let labels: Vec<String> = (0..digests.len()).map(|i| crate::house::i18n::filln(crate::house::i18n::Key::GoodsCandidate, &[&(i + 1).to_string()])).collect();
                    let mut raw: Vec<(&str, kv::Val)> = vec![(al.as_str(), kv::Val::Mono(address.clone()))];
                    for (i, dg) in digests.iter().enumerate() {
                        raw.push((labels[i].as_str(), kv::Val::Mono(dg.clone())));
                    }
                    let pl = t(crate::house::i18n::Key::GoodsProposalAt);
                    if let Some(Ok(p)) = &s.exported {
                        raw.push((pl.as_str(), kv::Val::Mono(p.clone())));
                    }
                    let (detail, widest) = detail_rows_of(ui, &raw);
                    f.details(ui, "goods-company-raw", &t(crate::house::i18n::Key::SetEvidence), &detail, widest)
                });
                tell_copied(ui.ctx(), copied);
            });
        }
        card::form(ui, |ui, f| {
            if v.row.draft {
                if f.danger(ui, &t(crate::house::i18n::Key::GoodsDeleteTitle)).clicked() {
                    self.shell.desk.goods_confirm = Some(crate::house::seam::STEP_PD_DELETE);
                }
            } else if v.revoked.is_some() {
                if f.go_sub(ui, &t(crate::house::i18n::Key::GoodsNewVersion), &t(crate::house::i18n::Key::GoodsRevokedNote)).clicked() {
                    act = Some(crate::house::seam::STEP_PD_EDIT);
                }
            } else if v.row.listed {
                if f.danger(ui, &t(crate::house::i18n::Key::GoodsRevokeTitle)).clicked() {
                    self.shell.desk.goods_confirm = Some(crate::house::seam::STEP_PD_REVOKE);
                }
            } else if f.act(ui, &t(crate::house::i18n::Key::GoodsList), "", key::Key::new(&t(crate::house::i18n::Key::GoodsList), Role::Secondary)).clicked() {
                act = Some("pd-list-on");
            }
        });
        if let Some(c) = self.shell.desk.goods_confirm {
            let export = c == crate::house::seam::STEP_PD_EXPORT_PROPOSAL;
            if export {
                if self.hx.place_open != Some("goods-proposal") {
                    self.hx.place_open = Some("goods-proposal");
                }
                let rows = [(t(crate::house::i18n::Key::NavGoods), name.clone()), (t(crate::house::i18n::Key::GoodsConsentCount), format!("{} · {}", v.company.as_ref().map_or(0, |(_, _, dg)| dg.len()), judge_names.join(&t(crate::house::i18n::Key::ListJoin))))];
                let (home, data) = (self.shell.desk.sh.seller.field.home.clone(), self.shell.desk.data.clone());
                match self.place_sheet(ui.ctx(), "goods-proposal", &t(crate::house::i18n::Key::OrdersCompanyExportKey), &rows, &t(crate::house::i18n::Key::IdExportGroup), || crate::house::seam::downloads(&home, &data).unwrap_or_default()) {
                    Placed::Go(dir) => {
                        self.shell.desk.goods_confirm = None;
                        self.step(Step::PdExportProposal, &[("dir", &dir)]);
                    }
                    Placed::Close => self.shell.desk.goods_confirm = None,
                    Placed::Open => {}
                }
            } else {
                let (title, ok) = if c == crate::house::seam::STEP_PD_DELETE {
                    (t(crate::house::i18n::Key::GoodsDeleteTitle), t(crate::house::i18n::Key::SetRemove))
                } else {
                    (t(crate::house::i18n::Key::GoodsRevokeTitle), t(crate::house::i18n::Key::GoodsRevokeTitle))
                };
                let (_, go, back) = Self::sheet_with(ui.ctx(), "goods-confirm", eravon_ui::metrics::MODAL_W, |ui, keys| {
                    Self::sheet_head(ui, &title, "");
                    card::form(ui, |ui, f| {
                        if c == crate::house::seam::STEP_PD_REVOKE {
                            f.info(ui, &t(crate::house::i18n::Key::NavGoods), card::Show::Text(&format!("{name} · {price}")));
                            f.info(ui, &t(crate::house::i18n::Key::GoodsRevokeAfter), card::Show::Text(&t(crate::house::i18n::Key::GoodsRevokeAfterSay)));
                            f.info(ui, &t(crate::house::i18n::Key::OrdersSentDeploy), card::Show::Text(&t(crate::house::i18n::Key::GoodsRevokeOpenSay)));
                        } else {
                            f.info(ui, &t(crate::house::i18n::Key::NavGoods), card::Show::Text(&name));
                        }
                    });
                    Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&ok, Role::Commit, true)));
                });
                if go {
                    self.shell.desk.goods_confirm = None;
                    act = Some(c);
                } else if back {
                    self.shell.desk.goods_confirm = None;
                }
            }
        }
        let Some(a) = act else { return };
        match a {
            crate::house::seam::STEP_PD_COMPANY_CHECK => self.step(Step::PdCompanyCheck, &[]),
            crate::house::seam::STEP_PD_COPY => {
                self.step(Step::PdCopy, &[]);
                if let Some(l) = self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).and_then(|s| s.copied.clone()) {
                    ui.ctx().copy_text(l);
                }
            }
            crate::house::seam::STEP_PD_QR => self.step(Step::PdQr, &[]),
            crate::house::seam::STEP_PD_EDIT => self.step(Step::PdEdit, &[]),
            crate::house::seam::STEP_PD_DELETE => self.step(Step::PdDelete, &[]),
            crate::house::seam::STEP_PD_POST => self.step(Step::PdPost, &[]),
            crate::house::seam::STEP_PD_FEES_CONFIRM => self.step(Step::PdFeesConfirm, &[]),
            crate::house::seam::STEP_PD_REVOKE => self.step(Step::PdRevoke, &[]),
            "pd-list-on" => self.step(Step::PdList, &[("on", "true")]),
            _ => self.step(Step::PdList, &[("on", "false")]),
        }
    }

    fn goods_inner(&mut self, ui: &mut egui::Ui, s: &crate::house::seller::Shown, v: &eravon_goods::goods::View, inner: crate::house::seller::GoodsInner) {
        use super::casepages::Asked;
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let name = if v.row.name.trim().is_empty() { t(crate::house::i18n::Key::GoodsUnnamed) } else { v.row.name.clone() };
        match inner {
            crate::house::seller::GoodsInner::Doc(goods) => {
                let (g, dl) = s.texts().unwrap_or_default();
                let private = if goods { v.draft.goods_private } else { v.draft.delivery_private };
                let text = if goods { g } else { dl };
                let digest = private.then(|| eravon_goods::terms::segment_hash(&text));
                let sub = format!("{} · {}", t(if goods { crate::house::i18n::Key::PlSegGoods } else { crate::house::i18n::Key::GoodsDeliveryNote }), super::casepages::text_state(!private));
                match super::casepages::text_page(ui, &name, &sub, goods, Some(&text), digest.as_deref(), s.checked.as_ref()) {
                    Some(Asked::CheckImage) => self.hx.place_open = Some("goods-image-check"),
                    Some(Asked::Url(k)) => self.open_foreign(k),
                    _ => {}
                }
                let asked = self.hx.place_open == Some("goods-image-check");
                if asked {
                    self.hx.place_open = None;
                }
                let dropped = eravon_ui::drop::take_dropped(ui.ctx());
                let picked = path_answer(ui.ctx(), egui::Id::new("goods-image-check"), asked, crate::platform::Pick::File).or_else(|| dropped.into_iter().next());
                if let Some(p) = picked {
                    self.step(Step::PdImageCheck, &[("path", &p)]);
                }
            }
            crate::house::seller::GoodsInner::Lapse(c) => {
                if let Some((_, Ok(clocks))) = s.lapses().into_iter().find(|(x, _)| x.eq_ignore_ascii_case(&c)) {
                    super::casepages::lapse_page(ui, &self.cand_label(&c), c.starts_with("galeed:"), &clocks);
                }
            }
        }
    }

    pub(super) fn qr_card(&mut self, ctx: &egui::Context) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some((id, name, version, q)) = self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).and_then(|s| {
            let (name, version) = s.view.as_ref().map_or((String::new(), 0), |v| (v.row.name.clone(), v.row.version));
            s.qr.clone().map(|q| (s.id.clone(), name, version, q))
        }) else {
            return;
        };
        let (_, hit, back) = Self::sheet_pick(ctx, "goods-qr", eravon_ui::metrics::MODAL_W, |ui, keys| {
            eravon_ui::sheet::title(ui, &name, "");
            match &q.modules {
                Some((n, cells)) if q.fits => {
                    ui.vertical_centered(|ui| qr_plate(ui, &q.link));
                }
                _ => states::note_box(
                    ui,
                    &format!("{} · {} / {} B", t(crate::house::i18n::Key::GoodsQrOver), q.link.len(), eravon_literals::constants::QR_BYTE_BUDGET),
                ),
            }
            if let Some(Ok((path, _))) = &q.saved {
                states::hint(ui, path);
            }
            Self::sheet_keys_sides(
                keys,
                "",
                &[(&t(crate::house::i18n::Key::ToastClose), Role::Primary, true)],
                &[(&t(crate::house::i18n::Key::GoodsQrSave), Role::Secondary, q.fits), (&t(crate::house::i18n::Key::GoodsQrCopy), Role::Secondary, q.fits)],
            );
        });
        let (close, save, copy) = (hit == Some(0) || back, hit == Some(1), hit == Some(2));
        let site = egui::Id::new("goods-qr-save");
        if let Some(dir) = super::super::common::path_answer(ctx, site, save, crate::platform::Pick::Folder) {
            let path = crate::house::seam::qr_file(&dir, &id, version);
            self.step(Step::PdQrSave, &[("path", path.as_str())]);
        }
        if copy {
            self.step(Step::PdQrCopy, &[]);
            let image = self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).and_then(|s| s.qr.as_ref()).and_then(|q| q.image.clone());
            if let Some(Ok((side, px))) = image {
                let said = match put_image(&mut self.hx.clipboard, side, &px) {
                    Ok(()) => crate::house::seller::Said::ok(t(crate::house::i18n::Key::QrCopiedSaid)),
                    Err(e) => crate::house::seller::Said::bad("E_APP_CLIPBOARD", e),
                };
                self.shell.desk.sh.toast(crate::house::shell::Toast {
                    tone: said.tone,
                    head: if said.tone == crate::house::seller::Tone::Bad { crate::house::refusal::line(lang, &said.name, &said.detail) } else { said.detail.clone() },
                    next: if said.tone == crate::house::seller::Tone::Bad { crate::house::refusal::next(lang, &said.name).unwrap_or_default() } else { String::new() },
                    detail: (said.tone == crate::house::seller::Tone::Bad).then(|| said.detail.clone()),
                });
                if let Some(g) = self.shell.desk.sh.seller.goods.as_mut() {
                    g.said = Some(said);
                }
            }
        }
        if close {
            self.step(Step::PdQrClose, &[]);
        }
    }

    pub(super) fn goods_edit(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let bar = self.shell.desk.goods_bar.take();
        let (cancel, save) = (bar == Some(false), bar == Some(true));
        match self.shell.desk.goods_publish.take() {
            Some(true) => self.step(Step::PePublish, &[]),
            Some(false) => self.step(Step::PePublish, &[("board", "false")]),
            None => {}
        }
        let title = self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.edit.as_ref()).map(|e| {
            if !e.draft.name.trim().is_empty() {
                e.draft.name.clone()
            } else if e.id.is_none() {
                t(crate::house::i18n::Key::GoodsNew)
            } else {
                t(crate::house::i18n::Key::GoodsUnnamed)
            }
        });
        if let Some(title) = title {
            self.hero_keys(ui, &title, "", &[]);
        }
        let named: Vec<(String, String)> = self
            .shell
            .desk
            .sh
            .seller
            .goods
            .as_ref()
            .and_then(|g| g.edit.as_ref())
            .map(|e| e.draft.cands.iter().cloned().chain(e.fees_shown().unwrap_or_default().iter().map(|f| f.candidate.clone())).chain(e.left_out(lang).into_iter().map(|(a, _)| a)).collect::<Vec<String>>())
            .unwrap_or_default()
            .into_iter()
            .map(|a| {
                let n = self.cand_label(&a);
                (a, n)
            })
            .collect();
        let name_of = |a: &str| named.iter().find(|(x, _)| x.eq_ignore_ascii_case(a)).map_or_else(|| eravon_orders::table::short(a.rsplit(':').next().unwrap_or(a)), |(_, n)| n.clone());
        let known: Vec<eravon_goods::series::Known> = self.shell.desk.sh.seller.goods.as_ref().map(|g| g.series_known()).unwrap_or_default().into_iter().filter(|k| eravon_goods::series::next_stage(k).is_some()).collect();
        let mut series_pick: Option<String> = None;
        let Some(gd) = self.shell.desk.sh.seller.goods.as_mut() else { return };
        let Some(e) = gd.edit.as_mut() else { return };
        let bad: Vec<String> = e.last.as_ref().map(|s| s.verdict.missing.iter().map(|m| m.cell.clone()).collect()).unwrap_or_default();
        if let Some(sv) = &e.last {
            if sv.draft && !sv.verdict.missing.is_empty() {
                let cells: Vec<String> = sv.verdict.missing.iter().map(|m| crate::house::i18n::get_named(lang, &format!("goods.cell.{}", cell_key(&m.cell))).0).collect();
                states::note_box(ui, &format!("{}{}", t(crate::house::i18n::Key::GoodsStillNeed), cells.join(&t(crate::house::i18n::Key::ListJoin))));
            }
        }
        let labels = [t(crate::house::i18n::Key::GoodsBasics), t(crate::house::i18n::Key::BoTabDelivery), t(crate::house::i18n::Key::GoodsTabRefund), t(crate::house::i18n::Key::CandsJudges), t(crate::house::i18n::Key::GoodsTabConfirm)];
        let refs: Vec<&str> = labels.iter().map(|x| x.as_str()).collect();
        let mut judges_tab = false;
        let mut import_ask: Option<&'static str> = None;
        let mut image_ask = false;
        let mut page_bad = false;
        if let Some(i) = eravon_ui::seg::tabs(ui, &&refs.join("\u{1f}"), &refs, e.tab) {
            e.tab = i;
            judges_tab = i == 3;
        }
        let field_line = |ui: &mut egui::Ui, e: &mut crate::house::seller::GoodsEdit, k: &str, label: &str, hint: &str| {
            let mut v = e.draft.get(k).unwrap_or_default().to_string();
            let marked = bad.iter().any(|c| c == k);
            let mono = k == "token";
            let changed = field(ui, label, None, |ui| {
                let w = ui.available_width();
                input::field(ui, &mut v, hint, w, input::Look { mono, bad: marked, ..input::Look::default() }).changed()
            });
            if changed {
                e.draft.set(k, &v);
            }
        };
        match e.tab {
            0 => card::card(ui, |ui| {
                field_line(ui, e, "name", &t(crate::house::i18n::Key::GoodsName), "");
                if md_cell(ui, e, "description", &t(crate::house::i18n::Key::GoodsDescription)) {
                    import_ask = Some("description");
                }
                let read = e.draft.refs();
                let mut page = e.draft.page();
                let changed = field(ui, &t(crate::house::i18n::Key::GoodsPage), Some(&t(crate::house::i18n::Key::ProxyCellOptional)), |ui| {
                    let w = ui.available_width();
                    input::field(ui, &mut page, "https://…", w, input::Look { mono: true, ..input::Look::default() }).changed()
                });
                if changed && !e.draft.set(eravon_goods::draft::PAGE, &page) {
                    page_bad = true;
                }
                let images = read.refs.images.clone();
                let mut drop_image: Option<usize> = None;
                let mut add_image = false;
                field(ui, &t(crate::house::i18n::Key::GoodsImages), Some(&t(crate::house::i18n::Key::ProxyCellOptional)), |ui| {
                    add_image = key::key(ui, &t(crate::house::i18n::Key::GoodsImageAdd), Role::Secondary, true).clicked();
                });
                if !images.is_empty() {
                    card::form(ui, |ui, f| {
                        for (i, im) in images.iter().enumerate() {
                            let _ = f.line(ui, None, &im.name, &im.sha256, true, false, c(C::Ink), |ui| {
                                if key::key(ui, &t(crate::house::i18n::Key::KitRemove), Role::Secondary, true).clicked() {
                                    drop_image = Some(i);
                                }
                            });
                        }
                    });
                }
                if let Some(i) = drop_image {
                    let _ = e.draft.remove_image(i);
                }
                if add_image {
                    image_ask = true;
                }
                let mut v = e.draft.get("price").unwrap_or_default().to_string();
                let marked = bad.iter().any(|c| c == "price");
                let unit = token_shown(lang, &e.draft.token);
                let changed = field(ui, &t(crate::house::i18n::Key::GoodsPrice), None, |ui| {
                    width::then(ui, |ui| {
                        ui.add_space(eravon_ui::tokens::S2);
                        paint::text(ui, &unit, Type::Note, c(C::Ink2))
                    }, |ui, room| input::field(ui, &mut v, "0", room, input::Look { mono: true, bad: marked, ..input::Look::default() }).changed()).0
                });
                if changed {
                    e.draft.set("price", &v);
                }
                let listed = eravon_vault::tokens::on_chain(&crate::house::coin::rows(), eravon_vault::tokens::chain());
                let now = crate::house::coin::cell(&e.draft.token);
                let mut names: Vec<String> = listed.iter().map(|r| r.label.clone()).collect();
                let at = match now.as_deref().and_then(|a| listed.iter().position(|r| r.address.eq_ignore_ascii_case(a))) {
                    Some(i) => i,
                    None => {
                        names.insert(0, if e.draft.token.trim().is_empty() { t(crate::house::i18n::Key::TokPick) } else { token_shown(lang, &e.draft.token) });
                        usize::MAX
                    }
                };
                let lead = usize::from(at == usize::MAX);
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                field(ui, &t(crate::house::i18n::Key::GoodsToken), None, |ui| {
                    if let Some(i) = { let rows: Vec<eravon_ui::menu::Item> = refs.iter().enumerate().map(|(i, l)| eravon_ui::menu::Item::Row(eravon_ui::menu::Row { label: l, check: Some(i == if lead == 1 { 0 } else { at }), ..Default::default() })).collect(); eravon_ui::menu::menu_key(ui, "goods-token", refs.get(if lead == 1 { 0 } else { at }).copied().unwrap_or_default(), false, eravon_ui::house::DD_MENU_MIN_W, &rows).filter(|i| *i != if lead == 1 { 0 } else { at }) } {
                        if i >= lead {
                            e.draft.set("token", &listed[i - lead].address);
                        }
                    }
                });
                let picks: Vec<String> = [t(crate::house::i18n::Key::GoodsSeriesNone), t(crate::house::i18n::Key::GoodsSeriesNew)]
                    .into_iter()
                    .chain(known.iter().map(|k| crate::house::i18n::filln(crate::house::i18n::Key::GoodsSeriesOf, &[&k.name, &k.stages.to_string()])))
                    .collect();
                let id_now = e.draft.series_id.trim().to_ascii_lowercase();
                let at = if id_now.is_empty() { 0 } else { known.iter().position(|k| k.id == id_now).map_or(1, |i| i + 2) };
                let refs: Vec<&str> = picks.iter().map(String::as_str).collect();
                field(ui, &t(crate::house::i18n::Key::GoodsSeriesId), None, |ui| {
                    let rows: Vec<eravon_ui::menu::Item> = refs.iter().enumerate().map(|(i, l)| eravon_ui::menu::Item::Row(eravon_ui::menu::Row { label: l, check: Some(i == at), ..Default::default() })).collect();
                    if let Some(i) = eravon_ui::menu::menu_key(ui, "goods-series", refs[at], false, eravon_ui::house::DD_MENU_MIN_W, &rows).filter(|i| *i != at) {
                        series_pick = Some(match i {
                            0 => "none".to_string(),
                            1 => "new".to_string(),
                            n => known[n - 2].id.clone(),
                        });
                    }
                });
                if at != 0 {
                    field_line(ui, e, "stage", &t(crate::house::i18n::Key::GoodsStage), "");
                    field_line(ui, e, "stages", &t(crate::house::i18n::Key::GoodsStages), "");
                }
            }),
            1 => {
                card::card(ui, |ui| {
                    window_cells(ui, e, "t1", &t(crate::house::i18n::Key::GoodsT1), &bad);
                    window_cells(ui, e, "t2", &t(crate::house::i18n::Key::GoodsT2), &bad);
                    if md_cell(ui, e, "deliveryNote", &t(crate::house::i18n::Key::GoodsDeliveryNote)) {
                        import_ask = Some("deliveryNote");
                    }
                });
            }
            2 => card::card(ui, |ui| {
                card::card_title(ui, &t(crate::house::i18n::Key::GoodsLadder));
                let n = e.draft.ladder.len();
                let mut del: Option<usize> = None;
                let remove = t(crate::house::i18n::Key::SetRemove);
                let gap = eravon_ui::metrics::S2;
                let key_w = key::size_of(ui, &key::Key::new(&remove, Role::Secondary)).x;
                let pct_w = eravon_ui::house::LADDER_PCT_W;
                let bearers = [t(crate::house::i18n::Key::PlBearerBuyer), t(crate::house::i18n::Key::PlBearerSeller)];
                let bearer_w = bearers.iter().map(|b| paint::galley(ui, b, Type::Note, c(C::Ink3)).size().x).fold(0.0, f32::max);
                let inline = ui.available_width() - pct_w - bearer_w - key_w - gap * 3.0 >= pct_w * 2.0;
                let name_w = if inline { ui.available_width() - pct_w - bearer_w - key_w - gap * 3.0 } else { (ui.available_width() - pct_w - key_w - gap * 2.0).max(pct_w) };
                for i in 0..n {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        let mut name = e.draft.ladder[i].name.clone();
                        if input::field(ui, &mut name, &t(crate::house::i18n::Key::GoodsTierName), name_w, input::Look { bad: bad.iter().any(|c| c == &format!("ladder.{i}")), ..input::Look::default() }).changed() {
                            e.draft.set_tier(i, Some(&name), None);
                        }
                        let mut pct = e.draft.ladder[i].pct.clone();
                        let locked = i == 0 || i + 1 == n;
                        let resp = ui.add_enabled_ui(!locked, |ui| input::line_w(ui, &mut pct, "%", pct_w)).inner;
                        if resp.changed() {
                            e.draft.set_tier(i, None, Some(&pct));
                        }
                        if inline {
                            let said = paint::text(ui, &bearers[usize::from(eravon_literals::constants::fee_bearer_of_rung(i) == eravon_literals::constants::FEE_BEARER_SELLER)], Type::Note, c(C::Ink3));
                            ui.add_space((bearer_w - said.rect.width()).max(0.0));
                        }
                        if locked {
                            ui.add_space(key_w);
                        } else if key::key(ui, &remove, Role::Secondary, true).clicked() {
                            del = Some(i);
                        }
                    });
                    if !inline {
                        states::hint(ui, &bearers[usize::from(eravon_literals::constants::fee_bearer_of_rung(i) == eravon_literals::constants::FEE_BEARER_SELLER)]);
                    }
                }
                if let Some(i) = del {
                    e.draft.del_tier(i);
                }
                if n < eravon_literals::constants::LADDER_MAX_RUNGS
                    && key::key(ui, &t(crate::house::i18n::Key::GoodsTierAdd), Role::Secondary, true).clicked()
                {
                    e.draft.add_tier();
                }
                states::hint(ui, &t(crate::house::i18n::Key::GoodsLadderHint));
            }),
            4 => {
                if let Some(to) = goods_confirm(ui, e, &name_of) {
                    e.tab = to;
                }
                let rows: Vec<super::casepages::LapseRow> = e.fees_shown().unwrap_or_default().iter().map(|f| super::casepages::LapseRow { key: f.candidate.clone(), name: name_of(&f.candidate), guild: f.candidate.starts_with("galeed:"), clocks: eravon_cands::lapse::of_spelling(&f.quote) }).collect();
                let _ = super::casepages::lapse_section(ui, &rows);
            }
            _ => {
                let picked = e.draft.cands.clone();
                let mut toggled: Option<String> = None;
                match self.shell.desk.sh.seller.cands.as_ref().filter(|c| !c.unread).map(|c| &c.rows) {
                    None => card::card(ui, |ui| eravon_ui::states::skeleton_lines(ui, &[eravon_ui::house::SKEL_MID, eravon_ui::house::SKEL_MID, eravon_ui::house::SKEL_SHORT])),
                    Some(Err(s)) => states::err_box(ui, "goods-cands", &t(crate::house::i18n::Key::CandsTableFailed), &crate::house::refusal::line(lang, &s.name, &s.detail), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
                    Some(Ok(rows)) => {
                        for (side, title) in [("judge", t(crate::house::i18n::Key::CandsJudges)), ("guild", t(crate::house::i18n::Key::CandsGuilds))] {
                            card::card(ui, |ui| {
                                card::card_title(ui, &title);
                                if !rows.iter().any(|b| b.row.kind == side) {
                                    eravon_ui::states::empty_pad(ui, Glyph::Inbox, &t(crate::house::i18n::Key::HomeNone), eravon_ui::house::EMPTY_PAD_INLINE);
                                }
                                for b in rows.iter().filter(|b| b.row.kind == side) {
                                    let usable = b.state.as_ref().is_ok_and(|s| s.selectable);
                                    let mut on = picked.contains(&b.row.address);
                                    let label = format!("{}  {}", b.row.label, short(&b.row.address));
                                    let lapse = |s: &eravon_cands::view::Summary| {
                                        let tok = crate::house::coin::cell(&e.draft.token).unwrap_or_default();
                                        s.lapses.iter().find(|(x, _)| tok.is_empty() || x.eq_ignore_ascii_case(&tok)).map(|(_, c)| crate::house::i18n::filln(crate::house::i18n::Key::LapseTermsSay, &[&super::casepages::lapse_word(c.lapse)]))
                                    };
                                    let kind = b.state.as_ref().ok().filter(|_| side == "guild").map(|s| if s.pool { t(crate::house::i18n::Key::GoodsGuildPool) } else { format!("{} · {}", t(crate::house::i18n::Key::GoodsGuildSet), s.members) });
                                    let note = cand_cell(b, kind, b.state.as_ref().ok().and_then(lapse), lang);
                                    if eravon_ui::toggle::toggle(ui, &mut on, &label, &note, usable || picked.contains(&b.row.address)) {
                                        toggled = Some(b.row.address.clone());
                                    }
                                }
                            });
                        }
                    }
                }
                if let Some(a) = toggled {
                    e.draft.toggle(&a);
                }
            }
        }
        if page_bad {
            self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: t(crate::house::i18n::Key::GoodsPageBad), next: String::new(), detail: None });
        }
        if let Some(v) = series_pick {
            self.step(Step::PeSeries, &[("v", &v)]);
        }
        if let Some(k) = import_ask {
            self.shell.desk.goods_import = Some(k);
        }
        if image_ask {
            self.shell.desk.goods_import = Some("image");
        }
        if let Some(k) = self.shell.desk.goods_import {
            let site = egui::Id::new(("goods-import", k));
            if let Some(path) = path_answer(ui.ctx(), site, import_ask.is_some() || image_ask, crate::platform::Pick::File) {
                self.shell.desk.goods_import = None;
                if k == "image" {
                    self.step(Step::PeImage, &[("path", &path)]);
                } else {
                    self.step(Step::PeImport, &[("k", k), ("path", &path)]);
                }
            }
        }
        if judges_tab {
            let (data, addr, field, now) = (self.shell.desk.data.clone(), self.shell.goods_key(), self.shell.desk.sh.seller.field.clone(), self.shell.desk.now());
            self.shell.spawn_cands(crate::house::desk::Site::Page, false, move |_| crate::house::seller::Cands::open(&data, &addr, &field, now, eravon_cands::table::Side::Judge));
        }
        if let Some(g) = self.shell.desk.sh.seller.goods.as_mut() {
            g.mark_dirty();
        }
        if save {
            self.step(Step::PeSave, &[]);
        }
        if cancel {
            self.step(Step::PeLeave, &[]);
        }
    }

    pub(super) fn leave_sheet(&mut self, ctx: &egui::Context) {
        let Some(to) = self.shell.desk.sh.seller.goods.as_ref().and_then(|g| g.edit.as_ref()).and_then(|e| e.ask.clone()) else { return };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let (_, hit, back) = Self::sheet_pick(ctx, "goods-leave", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::GoodsAskSave), &t(crate::house::i18n::Key::GoodsAskSaveHint));
            Self::sheet_keys_n(keys, &t(crate::house::i18n::Key::GoodsStay), &[(&t(crate::house::i18n::Key::ReadNetSave), Role::Primary, true), (&t(crate::house::i18n::Key::GoodsDiscard), Role::Secondary, true)]);
        });
        let pick = match hit {
            Some(0) => Some("save"),
            Some(_) => Some("discard"),
            None => back.then_some("stay"),
        };
        let Some(a) = pick else { return };
        let _ = (ctx, to);
        self.step(Step::PeAsk, &[("a", a)]);
    }
}

fn token_shown(lang: crate::lang::Lang, cell: &str) -> String {
    match crate::house::coin::cell(cell) {
        Some(a) => crate::house::coin::name(lang, &a),
        None => cell.to_string(),
    }
}

fn window_secs(d: &eravon_goods::draft::Draft, cell: &str) -> Option<u64> {
    let mut v = eravon_goods::verdict::Verdict::default();
    let (a, b, c) = (d.get(cell).unwrap_or_default(), d.get(&format!("{cell}h")).unwrap_or_default(), d.get(&format!("{cell}m")).unwrap_or_default());
    eravon_goods::goods::secs_of(cell, a, b, c, &mut v).filter(|s| *s > 0)
}

fn windows_said(d: &eravon_goods::draft::Draft) -> String {
    let lang = crate::lang::lang();
    let t = |k| crate::house::i18n::get(lang, k).0;
    let gap = if lang == crate::lang::Lang::Zh { "" } else { " " };
    let parts: Vec<String> = [("t1", crate::house::i18n::Key::GoodsDaysDeliver), ("t2", crate::house::i18n::Key::GoodsDaysConfirm)]
        .into_iter()
        .filter_map(|(cell, k)| window_secs(d, cell).map(|s| format!("{}{gap}{}", span_text(s), t(k))))
        .collect();
    if parts.is_empty() { t(crate::house::i18n::Key::SetNone) } else { parts.join(" · ") }
}

fn window_cells(ui: &mut egui::Ui, e: &mut crate::house::seller::GoodsEdit, k: &str, label: &str, bad: &[String]) {
    let names = [k.to_string(), format!("{k}h"), format!("{k}m")];
    let mut cells = names.clone().map(|n| e.draft.get(&n).unwrap_or_default().to_string());
    let marked = names.clone().map(|n| bad.iter().any(|c| *c == n));
    field(ui, label, None, |ui| {
        let [d, h, m] = &mut cells;
        period_cells(ui, [d, h, m], marked);
    });
    for (n, v) in names.iter().zip(&cells) {
        if e.draft.get(n).unwrap_or_default() != v.as_str() {
            e.draft.set(n, v);
        }
    }
}

pub(super) fn period_cells(ui: &mut egui::Ui, cells: [&mut String; 3], bad: [bool; 3]) {
    let lang = crate::lang::lang();
    let t = |x| crate::house::i18n::get(lang, x).0;
    let units = [
        (crate::house::i18n::Key::CliLastsDays, eravon_goods::goods::DAYS_MAX),
        (crate::house::i18n::Key::CliLastsHours, eravon_goods::goods::HOURS_MAX),
        (crate::house::i18n::Key::CliLastsMinutes, eravon_goods::goods::MINUTES_MAX),
    ];
    input_row(ui, |ui| {
        let w = (ui.available_width() / 3.0 - eravon_ui::metrics::BUTTON_GAP * 3.0).max(eravon_ui::house::GOODS_TERM_MIN_W);
        for ((cell, (unit, max)), marked) in cells.into_iter().zip(units).zip(bad) {
            let mut v = cell.clone();
            if input::field(ui, &mut v, "0", w * 0.6, input::Look { mono: true, bad: marked, ..input::Look::default() }).changed() {
                let digits: String = v.chars().filter(char::is_ascii_digit).collect();
                *cell = match digits.parse::<u64>() {
                    Ok(n) if n > max => max.to_string(),
                    _ => digits,
                };
            }
            paint::text(ui, &t(unit), Type::Note, c(C::Ink2));
        }
    });
}

fn fee_line(lang: crate::lang::Lang, f: &eravon_goods::resolve::Fee) -> String {
    let t = |k| crate::house::i18n::get(lang, k).0;
    let m = |v: &eravon_terms::u256::U256| crate::house::coin::amount(lang, &v.to_dec(), f.decimals, &f.token, crate::house::coin::Form::Short);
    match &f.priced {
        Ok(q) => {
            let line = format!(
                "{} {} · {} · {} {}",
                t(crate::house::i18n::Key::CandsQPremium),
                m(&q.premium),
                crate::house::i18n::filln(crate::house::i18n::Key::PlFeeLfNote, &[&m(&q.lf)]),
                t(crate::house::i18n::Key::PlSellerDeposit),
                m(&q.deposit)
            );
            if q.rate.is_some() { format!("{line} · {}", t(crate::house::i18n::Key::CasePricedAtOpen)) } else { line }
        }
        Err((n, d)) => crate::house::refusal::line(lang, n, d),
    }
}

fn priced_at_open(fees: &[eravon_goods::resolve::Fee]) -> bool {
    fees.iter().any(|f| f.priced.as_ref().is_ok_and(|q| q.rate.is_some()))
}

fn goods_confirm(ui: &mut egui::Ui, e: &crate::house::seller::GoodsEdit, name_of: &dyn Fn(&str) -> String) -> Option<usize> {
    let lang = crate::lang::lang();
    let t = |k| crate::house::i18n::get(lang, k).0;
    let d = &e.draft;
    let unset = t(crate::house::i18n::Key::SetNone);
    let or_unset = |s: &str| if s.trim().is_empty() { unset.clone() } else { s.to_string() };
    let price = if d.price.trim().is_empty() { unset.clone() } else { format!("{} {}", d.price, token_shown(lang, &d.token)) };
    let ladder: Vec<String> = d.ladder.iter().map(|l| format!("{} {}%", l.name, l.pct)).collect();
    let cands: Vec<String> = d.cands.iter().map(|a| name_of(a)).collect();
    let read = eravon_goods::refs::read(&d.description);
    let r = &read.refs;
    let (page, images) = (r.page.clone().unwrap_or_else(|| unset.clone()), if r.images.is_empty() { unset.clone() } else { r.images.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join(&t(crate::house::i18n::Key::ListJoin)) });
    let sections: [(usize, String, Vec<(String, String)>, String, String); 4] = [
        (
            0,
            t(crate::house::i18n::Key::GoodsBasics),
            vec![
                (t(crate::house::i18n::Key::GoodsName), or_unset(&d.name)),
                (t(crate::house::i18n::Key::GoodsPrice), price),
                (t(crate::house::i18n::Key::GoodsPage), page),
                (t(crate::house::i18n::Key::GoodsImages), images),
            ],
            read.body.clone(),
            t(crate::house::i18n::Key::PlSegGoods),
        ),
        (1, t(crate::house::i18n::Key::BoTabDelivery), vec![(t(crate::house::i18n::Key::BoTabDelivery), windows_said(d))], d.delivery_note.clone(), t(crate::house::i18n::Key::GoodsDeliveryNote)),
        (2, t(crate::house::i18n::Key::GoodsTabRefund), vec![(t(crate::house::i18n::Key::GoodsLadder), if ladder.is_empty() { unset.clone() } else { ladder.join(" · ") })], String::new(), String::new()),
        (3, t(crate::house::i18n::Key::CandsJudges), vec![(t(crate::house::i18n::Key::CandsJudges), if cands.is_empty() { unset.clone() } else { cands.join(&t(crate::house::i18n::Key::ListJoin)) })], String::new(), String::new()),
    ];
    let mut go = None;
    for (tab, title, rows, text, head) in sections {
        card::section(ui, &title, "", |ui| {
            card::form(ui, |ui, f| {
                for (k, v) in rows.iter().filter(|(_, v)| !v.is_empty()) {
                    f.info(ui, k, card::Show::Text(v));
                }
                if !text.trim().is_empty() {
                    f.free(ui, |ui| titled_text(ui, &head, &text));
                }
                if f.go(ui, &t(crate::house::i18n::Key::GoodsChange), "", true, "").clicked() {
                    go = Some(tab);
                }
            });
        });
    }
    let fees: Vec<(String, String)> = e
        .fees_shown()
        .unwrap_or_default()
        .iter()
        .map(|f| (name_of(&f.candidate), fee_line(lang, f)))
        .chain(e.left_out(lang).into_iter().map(|(a, why)| (name_of(&a), why)))
        .collect();
    card::section(ui, &t(crate::house::i18n::Key::BoTabFee), "", |ui| {
        card::form(ui, |ui, f| {
            if fees.is_empty() {
                f.note(ui, &t(crate::house::i18n::Key::GoodsFeesAfterSave));
            }
            for (k, v) in &fees {
                f.info(ui, k, card::Show::Text(v));
            }
            if priced_at_open(e.fees_shown().unwrap_or_default()) {
                f.note(ui, &t(crate::house::i18n::Key::PlPricedAtOpen));
            }
        });
    });
    go
}

fn md_cell(ui: &mut egui::Ui, e: &mut crate::house::seller::GoodsEdit, k: &str, label: &str) -> bool {
    let lang = crate::lang::lang();
    let t = |x| crate::house::i18n::get(lang, x).0;
    use crate::house::i18n::Key as K;
    let tools = [K::TextToolHeading, K::TextToolBold, K::TextToolItalic, K::ModeList, K::TextToolNumbered, K::TextToolQuote, K::TextToolCode].map(t);
    let words: [&str; 7] = std::array::from_fn(|i| tools[i].as_str());
    let read = |d: &eravon_goods::draft::Draft| if k == "description" { d.body() } else { d.get(k).unwrap_or_default().to_string() };
    let mut import = false;
    let mut v = String::new();
    field(ui, label, None, |ui| {
        card::card_pad(ui, egui::vec2(0.0, 0.0), |ui| {
            v = held_editor(ui, &format!("goods.{k}"), read(&e.draft), words, |ui| {
                import = key::key(ui, &t(K::TextImport), Role::Plain, true).clicked();
            }, |w| {
                if k == "description" {
                    e.draft.set_body(w);
                } else {
                    e.draft.set(k, w);
                }
                read(&e.draft)
            });
            eravon_ui::paint::rule(ui, 0.0);
            egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 8)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                paint::text(ui, &crate::fill!(K::TextChars; crate::house::text::char_count(&v).to_string()), eravon_ui::tokens::Type::Tiny, c(C::Ink3));
                eravon_ui::fold::fold(ui, &format!("goods.{k}.digest"), &t(K::ToastDetail), |ui| {
                    kv::kv(ui, &[(t(K::TextDigest).as_str(), kv::Val::Mono(crate::house::text::text_sha256(&v)))]);
                });
            });
        });
    });
    import
}

#[derive(Clone, Default)]
struct Held {
    text: String,
    seen: Option<String>,
}

fn held_editor(ui: &mut egui::Ui, id: &str, from_draft: String, tools: [&str; 7], extra: impl FnOnce(&mut egui::Ui), write: impl FnOnce(&str) -> String) -> String {
    let key = egui::Id::new((eravon_ui::mdedit::ID_HEAD, "held", id));
    let mut held: Held = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    if held.seen.as_deref() != Some(from_draft.as_str()) {
        held.text = from_draft.clone();
    }
    let before = held.text.clone();
    let _ = eravon_ui::mdedit::md_editor(ui, id, &mut held.text, tools, eravon_ui::house::GOODS_EDITOR_H, extra);
    let now = if held.text != before { write(&held.text) } else { from_draft };
    held.seen = Some(now.clone());
    ui.data_mut(|d| d.insert_temp(key, held));
    now
}

pub(super) fn resign_why(word: &str) -> Option<crate::house::i18n::Key> {
    use crate::house::i18n::Key as K;
    use eravon_goods::resign::Reason;
    match Reason::of(word) {
        Some(Reason::Expiry) => Some(K::GoodsWhyExpiry),
        Some(Reason::Candidate) => Some(K::GoodsWhyCandidate),
        Some(Reason::Payout) => Some(K::SetPayoutSaved),
        Some(Reason::Fund) => Some(K::GoodsWhyFund),
        None if word == eravon_goods::goods::PRESIGN => Some(K::GoodsWhyPresign),
        None => None,
    }
}

fn put_image(held: &mut Option<arboard::Clipboard>, side: usize, px: &[u8]) -> Result<(), String> {
    if held.is_none() {
        *held = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
    }
    let rgba: Vec<u8> = px.iter().flat_map(|&g| [g, g, g, 255]).collect();
    let image = arboard::ImageData { width: side, height: side, bytes: std::borrow::Cow::Owned(rgba) };
    held.as_mut().map_or(Ok(()), |c| c.set_image(image).map_err(|e| e.to_string()))
}

#[cfg(test)]
mod held_editor_tests {
    use eravon_ui::egui;
    use egui::text::{CCursor, CCursorRange};
    use eravon_goods::draft::Draft;

    const ID: &str = "goods.description";

    fn frame(ctx: &egui::Context, d: &mut Draft, events: Vec<egui::Event>, set: Option<(usize, usize)>) {
        let raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(crate::window::W, crate::window::H))), ..Default::default() };
        let _ = ctx.run_ui(raw, |root| {
            egui::CentralPanel::default().show_inside(root, |ui| {
                let edit = ui.make_persistent_id((eravon_ui::mdedit::ID_HEAD, ID));
                ui.memory_mut(|m| m.request_focus(edit));
                if let Some((a, b)) = set {
                    let mut st = egui::text_edit::TextEditState::load(ui.ctx(), edit).unwrap_or_default();
                    st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
                    st.store(ui.ctx(), edit);
                }
                let from = d.body();
                let _ = super::held_editor(ui, ID, from, ["h", "b", "i", "l", "n", "q", "c"], |_| {}, |w| {
                    d.set_body(w);
                    d.body()
                });
            });
        });
    }

    fn ctx() -> egui::Context {
        let ctx = egui::Context::default();
        let _ = eravon_ui::fonts::install(&ctx, &eravon_ui::fonts::Found::none());
        ctx
    }

    fn cmd(k: egui::Key) -> egui::Event {
        egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND }
    }

    #[test]
    fn the_shortcuts_write_the_draft_with_the_text_taken_from_it_every_frame() {
        let ctx = ctx();
        let mut d = Draft::default();
        d.set_body("say hello now");
        frame(&ctx, &mut d, Vec::new(), Some((4, 9)));
        frame(&ctx, &mut d, vec![cmd(egui::Key::B)], None);
        assert_eq!(d.body(), "say **hello** now", "bold reaches the draft");
        frame(&ctx, &mut d, vec![cmd(egui::Key::I)], None);
        assert_eq!(d.body(), "say ***hello*** now", "italic joins it");
        frame(&ctx, &mut d, Vec::new(), None);
        assert_eq!(d.body(), "say ***hello*** now", "and a frame with nothing pressed keeps it");
    }

    #[test]
    fn a_space_or_a_new_line_typed_at_the_end_stays_for_the_next_word() {
        let ctx = ctx();
        let mut d = Draft::default();
        d.set_body("hello");
        frame(&ctx, &mut d, Vec::new(), Some((5, 5)));
        frame(&ctx, &mut d, vec![egui::Event::Text(" ".into())], None);
        assert_eq!(d.description, "hello", "the draft keeps the saved form, the space off");
        frame(&ctx, &mut d, vec![egui::Event::Text("world".into())], None);
        assert_eq!(d.body(), "hello world");
        let enter = egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
        frame(&ctx, &mut d, vec![enter], None);
        assert_eq!(d.body(), "hello world");
        frame(&ctx, &mut d, vec![egui::Event::Text("next".into())], None);
        assert_eq!(d.body(), "hello world\nnext", "the new line begun at the end is kept");
    }

    #[test]
    fn a_draft_changed_elsewhere_is_what_the_editor_holds_next() {
        let ctx = ctx();
        let mut d = Draft::default();
        d.set_body("old");
        frame(&ctx, &mut d, Vec::new(), Some((3, 3)));
        frame(&ctx, &mut d, vec![egui::Event::Text(" ".into())], None);
        d.set_body("imported text");
        frame(&ctx, &mut d, Vec::new(), Some((13, 13)));
        frame(&ctx, &mut d, vec![egui::Event::Text("!".into())], None);
        assert_eq!(d.body(), "imported text!", "the editor took the imported text, the space it held dropped");
    }
}

#[cfg(test)]
mod resign_why_tests {
    #[test]
    fn every_to_do_word_is_said_by_its_closed_table() {
        use eravon_goods::resign::Reason;
        for r in [Reason::Expiry, Reason::Candidate, Reason::Payout, Reason::Fund] {
            let k = super::resign_why(r.word()).expect("a trigger has its words");
            for lang in [crate::lang::Lang::Zh, crate::lang::Lang::En] {
                let (w, hit) = crate::house::i18n::get(lang, k);
                assert!(hit && !w.is_empty() && !w.contains('?') && !w.contains("goods."), "{} {lang:?}: {w}", r.word());
            }
        }
        assert!(super::resign_why(eravon_goods::goods::PRESIGN).is_some());
        assert!(super::resign_why(eravon_goods::goods::POST).is_none() && super::resign_why(eravon_goods::goods::FEES_DUE).is_none());
        assert!(super::resign_why("unlisted").is_none());
        let src = include_str!("goods.rs");
        assert!(!src.contains(concat!("goods.why.", "{")), "no key is built from a to-do word");
        for (n, k) in [(eravon_goods::link::E_GOODS_LINK_UNPOSTED, crate::house::i18n::Key::GoodsPostPending), (eravon_goods::goods::E_GOODS_FEES_CHANGED, crate::house::i18n::Key::GoodsFeesNewTitle)] {
            for lang in [crate::lang::Lang::Zh, crate::lang::Lang::En] {
                assert_eq!(crate::house::refusal::line(lang, n, ""), crate::house::i18n::get(lang, k).0);
            }
        }
    }
}

fn cand_cell(b: &eravon_cands::view::Brief, kind: Option<String>, lapse: Option<String>, lang: crate::lang::Lang) -> String {
    let said = match crate::house::seller::pill_of(b) {
        crate::house::seller::Pill::Choosable => lapse.unwrap_or_default(),
        _ => super::cands::cand_pill(b, lang).0,
    };
    match kind {
        Some(k) if said.is_empty() => k,
        Some(k) => format!("{k} · {said}"),
        None => said,
    }
}

#[cfg(test)]
mod cand_cell_tests {
    use eravon_cands::view::{Brief, Summary};

    fn brief(state: Result<Summary, eravon_cands::fault::Absent>) -> Brief {
        let row = eravon_store::record::Candidate { kind: "judge".into(), address: "0x00000000000000000000000000000000000000b1".into(), label: "甲".into(), added_at: 0, seen: String::new() };
        Brief { row, state, warn: None, flaw: None }
    }

    fn summary(accepting: bool, selectable: bool) -> Summary {
        Summary { version: Some(1), unread: None, accepting, selectable, unpaced: false, needs_resign: false, resign_items: Vec::new(), members: 0, pool: false, lapses: Vec::new(), current: String::new() }
    }

    #[test]
    fn the_editor_says_the_candidates_pages_state() {
        let lang = crate::lang::Lang::Zh;
        let t = |k| crate::house::i18n::get(lang, k).0;
        assert_eq!(super::cand_cell(&brief(Ok(summary(false, true))), None, None, lang), t(crate::house::i18n::Key::CandsClosedPill));
        assert_eq!(super::cand_cell(&brief(Ok(summary(true, false))), None, None, lang), t(crate::house::i18n::Key::CandsNoQuote));
        assert_eq!(super::cand_cell(&brief(Ok(summary(true, true))), None, Some("条款".into()), lang), "条款");
        let absent = brief(Err(eravon_cands::fault::Absent::Unreachable("连不上".into())));
        assert_eq!(super::cand_cell(&absent, None, None, lang), super::super::cands::cand_pill(&absent, lang).0);
        assert_ne!(super::cand_cell(&absent, None, None, lang), t(crate::house::i18n::Key::CandsNoQuote));
        assert_eq!(super::cand_cell(&brief(Ok(summary(false, true))), Some("池".into()), None, lang), format!("池 · {}", t(crate::house::i18n::Key::CandsClosedPill)));
        assert_eq!(super::cand_cell(&brief(Ok(summary(true, true))), Some("池".into()), None, lang), "池");
    }
}
