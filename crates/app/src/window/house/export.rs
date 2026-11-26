use super::*;
use crate::house::action::Action as Step;
use eravon_ui::states;
use crate::house::export::Tab;

impl Win {
    pub(super) fn export_seg(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(e) = self.shell.desk.sh.export.clone() else { return };
        let (d, s) = (t(crate::house::i18n::Key::ExTabDossier), t(crate::house::i18n::Key::ExTabStatement));
        let at = if e.tab == Tab::Dossier { 0 } else { 1 };
        if let Some(i) = { let cells: Vec<eravon_ui::seg::Cell> = [(&d, None), (&s, None)].iter().map(|(text, count)| eravon_ui::seg::Cell { text, count: *count }).collect(); eravon_ui::seg::seg(ui, "ex-seg", &cells, at) } {
            let tab = if i == 0 { Tab::Dossier } else { Tab::Statement };
            self.step(Step::ExTab, &[("v", tab.name())]);
        }
    }

    pub(super) fn export_page(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let buyer = self.shell.desk.sh.seat() == crate::house::shell::Seat::Buyer;
        let Some(e) = self.shell.desk.sh.export.clone().filter(|e| e.buyer == buyer) else {
            if !self.shell.page_in_flight(crate::task::Kind::Export) {
                self.shell.load_export();
            }
            Self::home_skeleton(ui, 2);
            return;
        };
        if let Some(s) = &e.said {
            said_hint(ui, "export-said-1", lang, &s.name, &s.detail, s.tone == crate::house::seller::Tone::Bad);
        }
        let busy = self.shell.house_acting(crate::task::Kind::Export);
        let reading = self.shell.page_in_flight(crate::task::Kind::Export) && !busy;
        let mut open_picker = false;
        let mut run = false;
        match &e.picks {
            Some(Ok(p)) if p.is_empty() => {
                states::empty(ui, Glyph::Ledger, &t(crate::house::i18n::Key::ExNoOrders));
                return;
            }
            Some(Err(x)) => {
                states::err_box(ui, "ex-picks", &t(crate::house::i18n::Key::SyncNever), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&x.name, &x.detail));
                return;
            }
            None => {
                Self::home_skeleton(ui, 2);
                return;
            }
            Some(Ok(_)) => {}
        }
        card::section(ui, &t(crate::house::i18n::Key::OrdersOrder), "", |ui| {
            card::form(ui, |ui, f| {
                let keys = |ui: &mut egui::Ui, choose: &str, open_picker: &mut bool, run: &mut bool| {
                    *run = key::show(ui, key::Key::new(&t(crate::house::i18n::Key::IdExportGroup), Role::Guide).enabled(!busy && !reading && e.sel.is_some())).clicked();
                    *open_picker = key::key(ui, choose, Role::Secondary, !busy).clicked();
                };
                match e.sel.as_deref().and_then(|i| e.pick(i)) {
                    Some(p) => {
                        let (head, sub) = pick_lines(p, e.buyer, lang);
                        let _ = f.line(ui, None, &head, &sub, false, false, c(C::Ink), |ui| keys(ui, &t(crate::house::i18n::Key::ExChange), &mut open_picker, &mut run));
                    }
                    None => {
                        let _ = f.line(ui, None, &t(crate::house::i18n::Key::ExNoPick), "", false, false, c(C::Ink3), |ui| keys(ui, &t(crate::house::i18n::Key::ExChoose), &mut open_picker, &mut run));
                    }
                }
            });
        });
        if busy {
            card::section(ui, &t(if e.tab == Tab::Dossier { crate::house::i18n::Key::ExResultDossier } else { crate::house::i18n::Key::ExTabStatement }), "", |ui| {
                card::card(ui, |ui| states::skeleton_lines(ui, &[eravon_ui::house::SKEL_MID, eravon_ui::house::SKEL_MID, eravon_ui::house::SKEL_FIG]));
            });
        } else {
            match &e.done {
                None => {}
                Some(Err(x)) => states::err_box(ui, "ex-done", &crate::house::refusal::line(lang, &x.name, &x.detail), &t(crate::house::i18n::Key::ErrEVAULTBACKUPNOTLANDEDNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&x.name, &x.detail)),
                Some(Ok(o)) => result_card(ui, o, &e, lang),
            }
        }
        if open_picker {
            self.step(Step::ExPicker, &[("open", "true")]);
        }
        if run {
            self.hx.place_open = Some("export-place");
            self.hx.places.insert("export-place", e.dir.clone());
        }
        let title = t(if e.tab == Tab::Dossier { crate::house::i18n::Key::ExTabDossier } else { crate::house::i18n::Key::ExTabStatement });
        let rows: Vec<(String, String)> = e.sel.as_deref().and_then(|i| e.pick(i)).map(|p| vec![(t(crate::house::i18n::Key::OrdersOrder), pick_cols(p, e.buyer, lang).0)]).unwrap_or_default();
        let dir0 = e.dir.clone();
        match self.place_sheet(ui.ctx(), "export-place", &title, &rows, &t(crate::house::i18n::Key::IdExportGroup), || dir0) {
            Placed::Go(dir) => {
                if dir != e.dir {
                    self.step(Step::ExDir, &[("dir", &dir)]);
                }
                self.step(Step::ExRun, &[]);
            }
            Placed::Close | Placed::Open => {}
        }
        self.export_picker(ui.ctx());
    }

    fn export_picker(&mut self, ctx: &egui::Context) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(e) = self.shell.desk.sh.export.clone() else { return };
        if !e.picker {
            return;
        }
        let picks = match &e.picks {
            Some(Ok(p)) => p.clone(),
            _ => Vec::new(),
        };
        let (mut q, mut range) = (e.q.clone(), (e.from.clone(), e.to.clone()));
        let kept = eravon_export::pick::search(&picks, &q, &range.0, &range.1).unwrap_or_default();
        let id = egui::Id::new("ex-picker-choice");
        let mut sel: Option<String> = ctx.data(|d| d.get_temp::<String>(id)).or_else(|| e.sel.clone());
        let mut pick_now: Option<String> = None;
        let (_, go, back) = Self::sheet_with(ctx, "ex-picker", PICK_WIDE_W, |ui, keys| {
            eravon_ui::sheet::title(ui, &t(crate::house::i18n::Key::ExChoose), "");
            search_row(ui, "ex-picker-search", &mut q, &t(crate::house::i18n::Key::OrdersSearchHint), &mut range, crate::house::desk::wall_now());
            if kept.is_empty() {
                states::empty(ui, Glyph::Search, &t(crate::house::i18n::Key::ExNoMatch));
            } else {
                card::form(ui, |ui, f| {
                    for i in &kept {
                        let p = &picks[*i];
                        let (no, name, at) = pick_cols(p, e.buyer, lang);
                        let why = (!p.ready()).then(|| format!("{} {}", t(crate::house::i18n::Key::ExMissing), p.missing.iter().map(|m| crate::house::i18n::library(lang, m)).collect::<Vec<_>>().join(&t(crate::house::i18n::Key::ListJoin))));
                        let chosen = sel.as_deref() == Some(p.row.instance.as_str());
                        let (resp, _) = f.choice(ui, why.is_none(), chosen, |ui| {
                            ui.label(egui::RichText::new(&at).font(Type::MonoSmall.font()).color(c(C::Ink3)));
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(&no).font(Type::MonoSmall.font()).color(c(C::Ink2)));
                                ui.label(egui::RichText::new(&name).font(Type::Note.font()).color(c(C::Ink)));
                                if let Some(w) = &why {
                                    mark::pill(ui, w, eravon_ui::palette::Tone::Bad);
                                }
                            });
                        }, |_| ());
                        if resp.clicked() {
                            sel = Some(p.row.instance.clone());
                            if resp.double_clicked() {
                                pick_now = sel.clone();
                            }
                        }
                    }
                });
            }
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::ExPickKey), Role::Primary, sel.is_some())));
        });
        ctx.data_mut(|d| match &sel {
            Some(s) => {
                d.insert_temp(id, s.clone());
            }
            None => {
                d.remove::<String>(id);
            }
        });
        if (q.as_str(), range.0.as_str(), range.1.as_str()) != (e.q.as_str(), e.from.as_str(), e.to.as_str()) {
            self.step(Step::ExSearch, &[("q", &q), ("from", &range.0), ("to", &range.1)]);
        }
        if let Some(i) = pick_now.or_else(|| if go { sel.clone() } else { None }) {
            ctx.data_mut(|d| d.remove::<String>(id));
            self.step(Step::ExPick, &[("instance", &i)]);
        } else if back {
            ctx.data_mut(|d| d.remove::<String>(id));
            self.step(Step::ExPicker, &[("open", "false")]);
        }
    }
}

fn pick_cols(p: &eravon_export::pick::Pick, buyer_seat: bool, lang: crate::lang::Lang) -> (String, String, String) {
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    let r = &p.row;
    let no = crate::house::orders::row_label(r);
    let product = if r.product.is_empty() { t(crate::house::i18n::Key::ExNoProduct) } else { r.product.clone() };
    let party = if buyer_seat { &r.label } else { &r.buyer };
    let at = if r.placed == 0 { t(crate::house::i18n::Key::OrdersStateUnread) } else { eravon_orders::when::when(r.placed) };
    (format!("#{no}"), format!("{product} · {}", eravon_orders::table::short(party)), at)
}

fn pick_lines(p: &eravon_export::pick::Pick, buyer_seat: bool, lang: crate::lang::Lang) -> (String, String) {
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    let r = &p.row;
    let no = crate::house::orders::row_label(r);
    let product = if r.product.is_empty() { t(crate::house::i18n::Key::ExNoProduct) } else { r.product.clone() };
    let party = if buyer_seat { &r.label } else { &r.buyer };
    let head = format!("{no} · {product} · {}", eravon_orders::table::short(party));
    let state = r.status.as_ref().map(|s| crate::house::i18n::order_word(lang, s.word)).unwrap_or_else(|e| super::orders::unread_say(lang, e).0);
    (head, format!("{state} · {}", eravon_orders::when::day(r.placed)))
}

fn result_card(ui: &mut egui::Ui, o: &crate::house::export::Outcome, e: &crate::house::export::Export, lang: crate::lang::Lang) {
    use eravon_recheck::report::State;
    use eravon_ui::mark::Mark;
    use eravon_ui::palette::Tone as PillTone;
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    let wrote = o.files.join("\n");
    let copied = card::form(ui, |ui, f| {
        let mut copied = None;
        match o.tab {
            Tab::Dossier => {
                let open: Vec<&eravon_recheck::report::Item> = match &o.report {
                    Some(Ok(items)) => items.iter().filter(|x| x.state != State::Pass).collect(),
                    _ => Vec::new(),
                };
                let clean = matches!(&o.report, Some(Ok(_))) && open.is_empty();
                let counts = match &o.report {
                    Some(Ok(items)) => {
                        let n = |s: State| items.iter().filter(|x| x.state == s).count().to_string();
                        crate::house::i18n::filln(crate::house::i18n::Key::ExCounts, &[&n(State::Pass), &n(State::Mismatch), &n(State::Unread)])
                    }
                    _ => String::new(),
                };
                f.summary(ui, if clean { Glyph::Ok } else { Glyph::Warn }, if clean { card::Tint::Ok } else { card::Tint::Warn }, &t(if clean { crate::house::i18n::Key::ExDoneDossier } else { crate::house::i18n::Key::ExDoneDossierDiff }), None, &counts, |_| ());
                if let Some(eravon_cli::deskops::export::Shape { rows: n, terminal: term, entry, compliance: cmp, posted, attachments, .. }) = &o.shape {
                    f.info(ui, &t(crate::house::i18n::Key::ExResTerms), card::Show::Mark(Mark::Ok, &t(crate::house::i18n::Key::ExHas)));
                    f.info(ui, &t(crate::house::i18n::Key::ExRows), card::Show::Mark(Mark::Ok, &crate::house::i18n::filln(crate::house::i18n::Key::BoTxs, &[&n.to_string()])));
                    match term {
                        Some(x) => f.info(ui, &t(crate::house::i18n::Key::ExTerminal), card::Show::Mark(Mark::Ok, x)),
                        None => f.info(ui, &t(crate::house::i18n::Key::ExTerminal), card::Show::Empty(&t(crate::house::i18n::Key::ExNoTerminal))),
                    };
                    if !posted.0.is_empty() {
                        copied = f.info(ui, &t(crate::house::i18n::Key::CaseBoardTx), card::Show::Short(&posted.0)).or(copied);
                    }
                    f.info(ui, &t(crate::house::i18n::Key::ExAttachments), card::Show::Text(&crate::house::i18n::filln(crate::house::i18n::Key::ExFilesN, &[&attachments.to_string()])));
                    let (has, no_entry, no_cmp) = (t(crate::house::i18n::Key::ExHas), t(crate::house::i18n::Key::ExResEntryNone), t(crate::house::i18n::Key::ExResCompNone));
                    f.info(ui, &t(crate::house::i18n::Key::OrdersEntryRuling), if *entry { card::Show::Text(&has) } else { card::Show::Empty(&no_entry) });
                    f.info(ui, &t(crate::house::i18n::Key::ExCompliance), if *cmp { card::Show::Text(&has) } else { card::Show::Empty(&no_cmp) });
                }
                match &o.report {
                    Some(Ok(items)) => {
                        let mismatch = items.iter().any(|x| x.state == State::Mismatch);
                        let (p, tone) = if clean { (t(crate::house::i18n::Key::PlPass), PillTone::Ok) } else if mismatch { (t(crate::house::i18n::Key::ExMismatch), PillTone::Bad) } else { (t(crate::house::i18n::Key::ExUnread), PillTone::Warn) };
                        f.info(ui, &t(crate::house::i18n::Key::ExChain), card::Show::Pill(&p, tone));
                        for x in &open {
                            let what = item_said(lang, x.item, x.row.as_ref());
                            let why = x.why.clone().unwrap_or_default();
                            let read = crate::house::i18n::library(lang, &x.read);
                            let detail = if why.is_empty() { read } else { format!("{read} · {}", crate::house::i18n::library(lang, &why)) };
                            let (p, tone) = if x.state == State::Mismatch { (t(crate::house::i18n::Key::ExMismatch), PillTone::Bad) } else { (t(crate::house::i18n::Key::ExUnread), PillTone::Warn) };
                            let _ = f.line(ui, None, &what, &detail, false, false, c(C::Ink), |ui| mark::pill(ui, &p, tone));
                        }
                    }
                    Some(Err(x)) => {
                        let _ = f.line(ui, None, &t(crate::house::i18n::Key::ExChain), &crate::house::refusal::line(lang, &x.name, &x.detail), false, false, c(C::Ink), |ui| mark::pill(ui, &t(crate::house::i18n::Key::ExUnread), PillTone::Warn));
                    }
                    None => {}
                }
                if o.selfcheck.is_some() {
                    f.info(ui, &t(crate::house::i18n::Key::TaskSelfCheck), card::Show::Pill(&t(crate::house::i18n::Key::PlPass), PillTone::Ok));
                }
            }
            Tab::Statement => {
                f.summary(ui, Glyph::Ok, card::Tint::Ok, &t(crate::house::i18n::Key::ExDoneStatement), None, "", |_| ());
                if let Some(s) = &o.statement {
                    let p = e.pick(&s.instance);
                    let money = |a: &str| crate::house::coin::amount(lang, a, p.and_then(|p| p.row.decimals), &s.token, crate::house::coin::Form::Full);
                    let state = p.and_then(|p| p.row.status.as_ref().ok()).map(|x| crate::house::i18n::order_word(lang, x.word)).unwrap_or_else(|| crate::house::i18n::library(lang, &s.state));
                    f.info(ui, &t(crate::house::i18n::Key::ExTerminal), card::Show::Text(&state));
                    f.info(ui, &t(crate::house::i18n::Key::ExPrice), card::Show::Text(&money(&s.price)));
                    if !s.judge_fee.is_empty() {
                        let _ = f.row(ui, &t(crate::house::i18n::Key::CandsQPremium), &t(crate::house::i18n::Key::OrdersFeeFromDeposit), card::Value::Text(&money(&s.judge_fee)));
                    }
                    if !s.deposit.is_empty() {
                        f.info(ui, &t(crate::house::i18n::Key::OrdersKvBond), card::Show::Text(&money(&s.deposit)));
                    }
                    f.info(ui, &t(crate::house::i18n::Key::OrdersKvLf), card::Show::Text(&money(&s.lf)));
                    f.info(ui, &t(crate::house::i18n::Key::ExLfFrom), card::Show::Empty(&t(if s.lf_source == "Priced" { crate::house::i18n::Key::ExLfFromPriced } else { crate::house::i18n::Key::ExLfFromQuoted })));
                    for (k, v) in shares(s.split.as_ref()) {
                        match v {
                            Some(v) => f.info(ui, &t(k), card::Show::Text(&money(v))),
                            None => f.info(ui, &t(k), card::Show::Empty(&t(crate::house::i18n::Key::OrdersSplitLater))),
                        };
                    }
                    f.info(ui, &t(crate::house::i18n::Key::ExRows), card::Show::Text(&crate::house::i18n::filln(crate::house::i18n::Key::BoTxs, &[&s.rows.len().to_string()])));
                }
                match &o.compliance {
                    Some(Ok(_)) => f.info(ui, &t(crate::house::i18n::Key::ExCompliance), card::Show::Text(&t(crate::house::i18n::Key::ExHas))),
                    Some(Err(x)) => f.info(ui, &t(crate::house::i18n::Key::ExCompliance), card::Show::Empty(&crate::house::refusal::line(lang, &x.name, &x.detail))),
                    None => None,
                };
            }
        }
        if !wrote.is_empty() {
            details_row(ui, f, "ex-wrote", &[(t(crate::house::i18n::Key::ExWrote).as_str(), Val::mono(wrote.clone()))]);
        }
        copied
    });
    tell_copied(ui.ctx(), copied);
}

fn shares(split: Option<&[String; 4]>) -> [(crate::house::i18n::Key, Option<&str>); 4] {
    use crate::house::i18n::Key;
    let at = |i: usize| split.map(|m| m[i].as_str());
    [(Key::OrdersSplitBuyer, at(0)), (Key::OrdersSplitSeller, at(1)), (Key::OrdersSplitJudge, at(2)), (Key::OrdersSplitBond, at(3))]
}

#[cfg(test)]
mod share_rows {
    use crate::house::i18n::Key;

    #[test]
    fn each_party_reads_its_own_share_from_the_statement() {
        let split = ["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()];
        assert_eq!(super::shares(Some(&split)), [(Key::OrdersSplitBuyer, Some("1")), (Key::OrdersSplitSeller, Some("2")), (Key::OrdersSplitJudge, Some("3")), (Key::OrdersSplitBond, Some("4"))]);
        assert!(super::shares(None).iter().all(|(_, v)| v.is_none()));
    }
}

pub(super) fn item_said(lang: crate::lang::Lang, item: eravon_recheck::report::Name, row: Option<&(usize, String)>) -> String {
    let named = crate::house::i18n::get_named(lang, &format!("ex.item.{}", item.word())).0;
    match row {
        Some((i, kind)) => {
            let place = crate::house::i18n::fill_phrase(&named, &[&(i + 1).to_string()]);
            match crate::house::i18n::get_named(lang, &format!("ex.kind.{kind}")) {
                (k, true) => format!("{place} · {k}"),
                _ => place,
            }
        }
        None => named,
    }
}

#[cfg(test)]
mod item_tests {
    #[test]
    fn every_check_row_is_named_in_words() {
        use crate::lang::Lang;
        for lang in [Lang::Zh, Lang::En] {
            for n in eravon_recheck::report::Name::ALL {
                assert!(crate::house::i18n::get_named(lang, &format!("ex.item.{}", n.word())).1, "{}", n.word());
            }
            for k in eravon_literals::literals::TRANSACTION_KINDS {
                assert!(crate::house::i18n::get_named(lang, &format!("ex.kind.{k}")).1, "{k}");
            }
        }
        let said = super::item_said(Lang::Zh, eravon_recheck::report::Name::RowKind, Some(&(2, "init".to_string())));
        assert_eq!(said, "第 3 笔的类别 · 开案");
        assert!(!said.contains("row.kind") && !said.contains("init"));
    }
}
