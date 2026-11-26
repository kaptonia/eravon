use super::*;
use crate::house::action::Action as Step;
use crate::house::buyer_orders::Tab;
use eravon_literals::json::Out;

fn pick(o: &Out, path: &[&str]) -> String {
    let mut cur = o;
    for k in path {
        match cur {
            Out::O(m) => match m.iter().find(|(n, _)| n == k) {
                Some((_, v)) => cur = v,
                None => return String::new(),
            },
            _ => return String::new(),
        }
    }
    match cur {
        Out::S(s) => s.clone(),
        Out::N(n) => n.to_string(),
        Out::B(b) => b.to_string(),
        Out::O(m) if m.iter().any(|(n, _)| *n == "tone") => {
            let at = |k: &str| m.iter().find(|(n, _)| *n == k).and_then(|(_, v)| if let Out::S(s) = v { Some(s.clone()) } else { None }).unwrap_or_default();
            let (name, detail) = (at("name"), at("detail"));
            said_line(&name, &crate::house::refusal::line(crate::lang::lang(), &name, &detail))
        }
        _ => String::new(),
    }
}

impl Win {
    pub(super) fn buyer_orders_page(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let me = self.shell.desk.addr.clone();
        let Some(b) = self.shell.desk.sh.buyer_orders.clone().filter(|b| b.fresh_for(&me)) else {
            if !self.shell.page_in_flight(crate::task::Kind::Bo) {
                self.shell.load_bo();
            }
            Self::home_skeleton(ui, 0);
            return;
        };
        if let Some(s) = b.said_here() {
            self.said_box(ui, "bo-said", s, "");
        }
        if let Some(o) = &b.open {
            self.buyer_order_detail(ui, &b, o);
            return;
        }
        if let (Some(Ok(_)), Some(u)) = (&b.rows, &b.unscanned) {
            states::err_box(ui, "bo-unscanned", &t(crate::house::i18n::Key::OrdersScanRefused), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&u.name, &u.detail));
        }
        match &b.rows {
            None => Self::home_skeleton(ui, 0),
            Some(Err(s)) => states::err_box(ui, "bo-list", &t(crate::house::i18n::Key::BoListFailed), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
            Some(Ok(_)) => {
                let (mut q, mut from, mut to) = (b.q.clone(), b.from.clone(), b.to.clone());
                let w = ui.available_width();
                let (line, _) = ui.allocate_exact_size(egui::vec2(w, eravon_ui::metrics::INPUT_H), egui::Sense::hover());
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(line).layout(egui::Layout::left_to_right(egui::Align::Center)));
                child.spacing_mut().item_spacing.x = eravon_ui::metrics::S2;
                let qw = eravon_ui::house::search_beside_dates(w);
                input::search(&mut child, &mut q, &t(crate::house::i18n::Key::BoSearchHint), qw);
                super::date_box(&mut child, "bo-from", &mut from, &t(crate::house::i18n::Key::DateFrom), crate::house::desk::wall_now());
                super::date_box(&mut child, "bo-to", &mut to, &t(crate::house::i18n::Key::DateTo), crate::house::desk::wall_now());
                if (q.as_str(), from.as_str(), to.as_str()) != (b.q.as_str(), b.from.as_str(), b.to.as_str()) {
                    self.step(Step::BoSearch, &[("q", &q), ("from", &from), ("to", &to)]);
                }
                if let Some(e) = self.shell.desk.sh.buyer_orders.as_ref().and_then(|x| x.search_said()) {
                    said_hint(ui, "buyer_orders-said-1", lang, &e.name, &e.detail, true);
                }
                let shown = self.shell.desk.sh.buyer_orders.clone().unwrap_or_default();
                let rows = shown.shown();
                let mut open: Option<String> = None;
                let co_col = rows.iter().any(|r| r.lf_gap.is_some());
                let heads = [t(crate::house::i18n::Key::IdSeatAuthor), t(crate::house::i18n::Key::PlTitle), t(crate::house::i18n::Key::OrdersColDeadline), t(crate::house::i18n::Key::OrdersAmount), t(crate::house::i18n::Key::BoColLfGap), t(crate::house::i18n::Key::SetZkState)];
                let tip = |v: &str, r: &crate::house::buyer_orders::BRow| crate::house::coin::amount_tip(lang, v, r.row.decimals, &r.row.token);
                let auto = |min: f32| table::Col::Auto { min, max: f32::MAX };
                let mut cols = vec![
                    table::col(&heads[0], auto(eravon_ui::house::COL_WHO)),
                    table::col(&heads[1], eravon_ui::house::REST),
                    table::col(&heads[2], table::Col::Px(eravon_ui::house::COL_DEADLINE)),
                    table::col(&heads[5], super::STATE_COL),
                    table::col_r(&heads[3], auto(eravon_ui::house::COL_AMOUNT)),
                ];
                if co_col {
                    cols.push(table::col_r(&heads[4], auto(eravon_ui::house::COL_AMOUNT)));
                }
                cols.push(table::CHEV);
                let make = |i: usize| {
                        let r = &rows[i];
                        let mut name = if r.product.is_empty() { super::short(&r.row.instance) } else { r.product.clone() };
                        if let Some(x) = &r.row.series {
                            name = crate::fill_in!(lang, crate::house::i18n::Key::OrdersSeriesStage; name, x.stage.to_string(), x.stages.to_string());
                        }
                        if r.access == eravon_after::act::Access::ByCall {
                            name = format!("{name} · {}", t(crate::house::i18n::Key::BoReadOnly));
                        }
                        let (say, tone) = match (pre_state(r, lang), r.row.status.as_ref()) {
                            _ if r.row.past => (super::orders::past_pill(&r.row, lang).unwrap_or_default(), eravon_ui::palette::Tone::Grey),
                            (Some(x), _) => x,
                            (None, Ok(st)) if st.word.done() => (crate::house::i18n::order_word(lang, st.word), eravon_ui::palette::Tone::Grey),
                            (None, Ok(st)) if matches!(st.word, eravon_orders::status::Word::Disputed | eravon_orders::status::Word::AwaitRedraw) => (crate::house::i18n::order_word(lang, st.word), eravon_ui::palette::Tone::Warn),
                            (None, Ok(st)) => (crate::house::i18n::order_word(lang, st.word), eravon_ui::palette::Tone::Ok),
                            (None, Err(e)) => super::orders::unread_say(lang, e),
                        };
                        let mut cells = vec![
                            table::Cell::Text(eravon_orders::table::short(&r.seller)),
                            table::Cell::Text(name),
                            table::Cell::Mono(super::day_or(r.row.deadline)),
                            table::Cell::Pill(say, tone),
                            table::Cell::Amount(tip(&r.row.amount, r)),
                        ];
                        if co_col {
                            cells.push(match &r.lf_gap {
                                Some(Ok(g)) => table::Cell::Amount(tip(g, r)),
                                Some(Err(_)) => table::Cell::Text(t(crate::house::i18n::Key::SetNotRead)),
                                None => table::Cell::Text("·".into()),
                            });
                        }
                        cells.push(table::Cell::Chev);
                        table::Row { cells, click: true, gone: false, on: false }
                };
                let none = t(crate::house::i18n::Key::OrdersNone);
                if let Some(i) = table::table_with(ui, "bo-list", table::Spec::new(&cols, self.shell.desk.rows_gen).head(true).empty(&none), rows.len(), make).clicked {
                    open = Some(rows[i].row.instance.clone());
                }
                if let Some(i) = open {
                    self.step(Step::BoOpen, &[("instance", &i)]);
                }
            }
        }
    }

    fn buyer_order_detail(&mut self, ui: &mut egui::Ui, b: &crate::house::buyer_orders::BOrders, o: &crate::house::buyer_orders::Open) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let d = o.detail.clone().unwrap_or(Out::O(Vec::new()));
        let row = b.rows.as_ref().and_then(|x| x.as_ref().ok()).and_then(|x| x.iter().find(|x| x.row.instance == o.instance)).cloned();
        if let (Some(inner), Some(r)) = (o.inner, row.as_ref()) {
            self.buyer_order_inner(ui, o, r, inner);
            return;
        }
        if let Some(r) = &row {
            let goods = if r.product.is_empty() { t(crate::house::i18n::Key::BoUntitled) } else { r.product.clone() };
            let mut sub = format!("{} · {} {}", crate::house::coin::amount(lang, &r.row.amount, r.row.decimals, &r.row.token, crate::house::coin::Form::Short), t(crate::house::i18n::Key::BoKvPlaced), day_or(r.row.placed));
            if r.row.chain.as_ref().is_some_and(|v| v.phase != 0) {
                if let Some(w) = state_why(&r.row, lang) {
                    sub = format!("{sub} · {w}");
                }
            }
            let (say, tone) = pre_state(r, lang).unwrap_or_else(|| (state_say(&r.row, lang), state_tone(&r.row)));
            Self::hero(ui, &goods, &sub, &[hero_pill(&say, tone)]);
        }
        let tabs = [Tab::Overview, Tab::Delivery, Tab::Dispute, Tab::Record, Tab::Fee];
        let labels: Vec<String> = tabs.iter().map(|x| crate::house::i18n::get_named(lang, &format!("bo.tab.{}", x.word())).0).collect();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let at = tabs.iter().position(|x| *x == o.tab).unwrap_or(0);
        if let Some(i) = eravon_ui::seg::tabs(ui, &&refs.join("\u{1f}"), &refs, at) {
            self.step(Step::BoTab, &[("v", tabs[i].word())]);
        }
        let mut hit: Option<&'static str> = None;
        let money = |v: &str| match &row {
            Some(r) if !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()) => crate::house::coin::amount(lang, v, r.row.decimals, &r.row.token, crate::house::coin::Form::Full),
            _ => v.to_string(),
        };
        let co = row.as_ref().is_some_and(|r| r.access == eravon_after::act::Access::Company);
        let reading = self.shell.house_in_flight(crate::task::Kind::Bo);
        self.complaint_sheet(ui.ctx(), &o.instance, o.complaint.as_ref(), b.said.as_ref(), reading);
        match o.tab {
            Tab::Overview => {
                if let Some(r) = &row {
                    if let Some(x) = self.buyer_overview(ui, r, &d, co, &judge_named(o, &pick(&d, &["overview", "judge"])), o.advance.as_ref()) {
                        hit = Some(x);
                    }
                }
            }
            Tab::Delivery => {
                let done = pick(&d, &["delivery", "done"]) == "true";
                let mut rows = vec![(t(crate::house::i18n::Key::SetZkState), if done { kv::Val::Mark(mark::Mark::Ok, t(crate::house::i18n::Key::BoDelivered)) } else { kv::Val::Mark(mark::Mark::Todo, t(crate::house::i18n::Key::BoNotDelivered)) })];
                if done {
                    let note = pick(&d, &["delivery", "note"]);
                    let files = pick(&d, &["delivery", "files"]);
                    if !files.is_empty() {
                        rows.push((t(crate::house::i18n::Key::OrdersFileSheet), kv::Val::Text(crate::house::i18n::filln(crate::house::i18n::Key::BoFilesN, &[&files]))));
                    }
                    rows.push((t(crate::house::i18n::Key::BoNote), if note.is_empty() { kv::Val::Quiet("·".into()) } else { kv::Val::Text(note) }));
                }
                let refs: Vec<(&str, kv::Val)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                rows_section(ui, &t(crate::house::i18n::Key::BoTabDelivery), &refs);
            }
            Tab::Dispute => {
                if pick(&d, &["dispute", "deadline"]).is_empty() {
                    card::card(ui, |ui| eravon_ui::states::empty_pad(ui, eravon_ui::icons::Glyph::House(eravon_ui::page_marks::PageMark::Scale), &t(crate::house::i18n::Key::BoNoDispute), eravon_ui::house::EMPTY_PAD_CARD));
                } else {
                    let deadline = pick(&d, &["dispute", "deadline"]).parse::<u64>().unwrap_or(0);
                    rows_section(ui, &t(crate::house::i18n::Key::OrdersClaimSection), &[
                        (&t(crate::house::i18n::Key::BoRuleBy), kv::Val::Figure(day_or(deadline))),
                        (&t(crate::house::i18n::Key::OrdersBuyerLf), kv::Val::Text(money(&pick(&d, &["dispute", "lf"])))),
                        (&t(crate::house::i18n::Key::BoEvidence), kv::Val::Figure(pick(&d, &["dispute", "evidence"]))),
                    ]);
                    self.seal_card(ui, o.judge.as_ref(), o.sealable.is_some(), o.sealed.as_ref(), Step::BoEvidenceJudge, Step::BoEvidenceSeal);
                }
            }
            Tab::Record => {
                let marks: Vec<(u64, String)> = match &d {
                    Out::O(m) => match m.iter().find(|(k, _)| *k == "record").map(|(_, v)| v) {
                        Some(Out::A(v)) => v.iter().map(|x| (pick(x, &["at"]).parse::<u64>().unwrap_or(0), pick(x, &["say"]))).collect(),
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                let failed = pick(&d, &["record", "detail"]);
                let heads = [t(crate::house::i18n::Key::WlColTime), t(crate::house::i18n::Key::BoEvent)];
                let cols = [table::col(&heads[0], table::Col::Px(eravon_ui::house::COL_RECORD_TIME)), table::col(&heads[1], eravon_ui::house::REST)];
                let make = |i: usize| {
                    let (at, say) = &marks[i];
                    table::Row { cells: vec![table::Cell::Mono(short_at(*at)), table::Cell::Text(say.clone())], ..Default::default() }
                };
                let empty = if failed.is_empty() { t(crate::house::i18n::Key::BoRecordNone) } else { failed.clone() };
                table::table_with(ui, "bo-record", table::Spec::new(&cols, self.shell.desk.rows_gen).head(true).empty(&empty), marks.len(), make);
            }
            Tab::Fee => {
                let mut v = vec![(t(crate::house::i18n::Key::ExPrice), kv::Val::Text(money(&pick(&d, &["fee", "amount"])))), (t(crate::house::i18n::Key::CandsQPremium), kv::Val::Text(money(&pick(&d, &["fee", "judgeFee"]))))];
                let mine = pick(&d, &["fee", "mineFee"]);
                if !mine.is_empty() {
                    let said = pick(&d, &["fee", "mineTxs"]).is_empty();
                    v.push((t(crate::house::i18n::Key::BoMineFee), if said { kv::Val::Text(mine) } else { kv::Val::Text(format!("{} · {}", crate::house::coin::eth(lang, &mine, crate::house::coin::Form::Full), crate::house::i18n::filln(crate::house::i18n::Key::BoTxs, &[&pick(&d, &["fee", "mineTxs"])]))) }));
                }
                let refs: Vec<(&str, kv::Val)> = v.iter().map(|(k, x)| (k.as_str(), x.clone())).collect();
                rows_section(ui, &t(crate::house::i18n::Key::BoTabFee), &refs);
                let later = t(crate::house::i18n::Key::OrdersSplitLater);
                let part = |k: &str| { let x = pick(&d, &["fee", "split", k]); if x.is_empty() { later.clone() } else { money(&x) } };
                let split = [(t(crate::house::i18n::Key::OrdersSplitBuyer), part("buyer")), (t(crate::house::i18n::Key::OrdersSplitSeller), part("seller")), (t(crate::house::i18n::Key::OrdersSplitJudge), part("judge")), (t(crate::house::i18n::Key::OrdersSplitBond), part("bondToBuyer"))];
                let refs: Vec<(&str, kv::Val)> = split.iter().map(|(k, x)| (k.as_str(), kv::Val::Text(x.clone()))).collect();
                rows_section(ui, &t(crate::house::i18n::Key::OrdersEvClosed), &refs);
            }
        }
        let ctx = ui.ctx().clone();
        let mut close: Option<&'static str> = None;
        let mut input = self.shell.desk.bo_input.clone();
        if let Some(Ok(c)) = &o.card {
            let (tags, go, back) = Self::sheet_pick(&ctx, "bo-confirm", eravon_ui::metrics::MODAL_W, |ui, keys| {
                let tags: (Vec<&'static str>, Option<&'static str>);
                Self::sheet_head(ui, &t(crate::house::i18n::Key::BoActConfirm), "");
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                if co {
                    states::note_box(ui, &t(crate::house::i18n::Key::BoCompanyConfirm));
                }
                let short = |v: &str| match &row {
                    Some(r) if !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()) => crate::house::coin::amount(lang, v, r.row.decimals, &r.row.token, crate::house::coin::Form::Short),
                    _ => v.to_string(),
                };
                kv::kv(ui, &[(&t(crate::house::i18n::Key::BoToSeller), kv::Val::Text(c.to_seller.as_deref().map_or_else(|| t(crate::house::i18n::Key::BoFeeLater), short))), (&t(crate::house::i18n::Key::CandsQPremium), kv::Val::Text(c.judge_fee.as_deref().map_or_else(|| t(crate::house::i18n::Key::BoFeeLater), short)))]);
                let raw = |v: Option<&str>| v.map_or_else(|| t(crate::house::i18n::Key::BoFeeLater), |v| format!("{} · {v}", money(v)));
                eravon_ui::fold::fold(ui, "bo-confirm-det", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                    kv::kv(ui, &[(&t(crate::house::i18n::Key::BoToSeller), kv::Val::Mono(raw(c.to_seller.as_deref()))), (&t(crate::house::i18n::Key::CandsQPremium), kv::Val::Mono(raw(c.judge_fee.as_deref())))]);
                });
                let send = if co { t(crate::house::i18n::Key::OrdersCompanyExportKey) } else { t(crate::house::i18n::Key::BoConfirmSend) };
                Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&send, Role::Commit, true)));
                tags = (vec!["confirmSend"], Some("card"));
                tags
            });
            if let Some(k) = go {
                hit = tags.0.get(k).copied();
            } else if back {
                close = tags.1;
            }
        }
        if let Some(Ok(rf)) = &o.refund {
            let (tags, go, back) = Self::sheet_pick(&ctx, "bo-dispute", eravon_ui::metrics::MODAL_W, |ui, keys| {
                let tags: (Vec<&'static str>, Option<&'static str>);
                Self::sheet_head(ui, &t(crate::house::i18n::Key::BoActDispute), "");
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                let short = |v: &str| match &row {
                    Some(r) if !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()) => crate::house::coin::amount(lang, v, r.row.decimals, &r.row.token, crate::house::coin::Form::Short),
                    _ => v.to_string(),
                };
                let judge = pick(&d, &["overview", "judge"]);
                let range = rf.rungs.iter().map(|(b, _, _)| super::paylink::pct_word(*b)).collect::<Vec<_>>().join(" · ");
                let by = row.as_ref().and_then(|r| r.row.chain.as_ref()).map(|v| v.window_end).unwrap_or(0);
                let mut v = vec![
                    (t(crate::house::i18n::Key::PlCands), kv::Val::Mono(if judge.is_empty() { "·".into() } else { judge_named(o, &judge) })),
                    (t(crate::house::i18n::Key::BoLf), kv::Val::Text(crate::house::i18n::filln(crate::house::i18n::Key::BoLfNow, &[&short(&rf.lf)]))),
                ];
                if !range.is_empty() {
                    v.push((t(crate::house::i18n::Key::BoRulingRange), kv::Val::Text(format!("{} {range}", t(crate::house::i18n::Key::PlToBuyer)))));
                }
                v.push((t(crate::house::i18n::Key::BoLapse), kv::Val::Text(match &rf.lapse {
                    Ok((b, _, _)) => crate::house::i18n::filln(crate::house::i18n::Key::BoLapseSay, &[&format!("{} {}", t(crate::house::i18n::Key::PlToBuyer), super::paylink::pct_word(*b))]),
                    Err(e) => e.clone(),
                })));
                if by != 0 {
                    v.push((t(crate::house::i18n::Key::BoApplyBy), kv::Val::Text(crate::house::i18n::filln(crate::house::i18n::Key::BoApplyBySay, &[&short_at(by)]))));
                }
                let refs: Vec<(&str, kv::Val)> = v.iter().map(|(k, x)| (k.as_str(), x.clone())).collect();
                kv::kv(ui, &refs);
                let raw = |v: &str| format!("{} · {v}", money(v));
                eravon_ui::fold::fold(ui, "bo-dispute-det", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                    kv::kv(ui, &[(&t(crate::house::i18n::Key::BoLf), kv::Val::Mono(raw(&rf.lf))), (&t(crate::house::i18n::Key::BoStray), kv::Val::Mono(raw(&rf.stray))), (&t(crate::house::i18n::Key::BoNeed), kv::Val::Mono(raw(&rf.need)))]);
                });
                if co {
                    paint::text(ui, &t(crate::house::i18n::Key::BoFirstEvidence), eravon_ui::tokens::Type::Note, c(C::Ink2));
                    match &o.first_kits {
                        Some(Ok(ks)) if !ks.is_empty() => {
                            card::form(ui, |ui, f| {
                                for (id, note, n) in ks {
                                    let on = self.shell.desk.bo_kit.as_deref() == Some(id.as_str());
                                    if f.row_with(ui, on.then_some(mark::Mark::Ok), id, &format!("{note} · {n}"), card::Value::Nav, |_| ()).clicked() {
                                        self.shell.desk.bo_kit = if on { None } else { Some(id.clone()) };
                                    }
                                }
                            });
                        }
                        Some(Err(e)) => said_hint(ui, "buyer_orders-said-2", lang, &e.name, &e.detail, false),
                        _ => {}
                    }
                    chosen_row(ui, &self.shell.desk.bo_first, lang);
                }
                if co {
                    Self::sheet_keys_n(keys, &t(crate::house::i18n::Key::CfBack), &[(&t(crate::house::i18n::Key::BoCompanyWallet), Role::Commit, true), (&t(crate::house::i18n::Key::BoCompanyPersonal), Role::Secondary, true)]);
                    keys.pick = Some(("bo-first".into(), crate::platform::Pick::FileOrFolder));
                    tags = (vec!["disputeSend", "disputePull"], Some("refund"));
                } else {
                    Self::sheet_keys_n(keys, &t(crate::house::i18n::Key::CfBack), &[(&t(crate::house::i18n::Key::BoPrepayApply), Role::Commit, true)]);
                    tags = (vec!["disputeSend"], Some("refund"));
                }
                tags
            });
            if co {
                if let Some(p) = Self::sheet_picked(&ctx, "bo-first").or_else(|| self.sheet_dropped(&ctx)) {
                    self.shell.desk.bo_first = p;
                }
            }
            if let Some(k) = go {
                hit = tags.0.get(k).copied();
            } else if back {
                close = tags.1;
            }
        }
        if let Some(k) = &o.kits {
            let (tags, go, back) = Self::sheet_pick(&ctx, "bo-evidence", eravon_ui::metrics::MODAL_WIDE_W, |ui, keys| {
                let tags: (Vec<&'static str>, Option<&'static str>);
                Self::sheet_head(ui, &t(crate::house::i18n::Key::OrdersEvidenceKey), "");
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                if co {
                    states::note_box(ui, &t(crate::house::i18n::Key::BoCompanyConfirm));
                }
                match k {
                    Ok(ks) if ks.is_empty() => eravon_ui::states::empty_pad(ui, eravon_ui::icons::Glyph::Kit, &t(crate::house::i18n::Key::BoKitsNone), eravon_ui::house::EMPTY_PAD_INLINE),
                    Ok(ks) => {
                        card::form(ui, |ui, f| {
                            for (id, note, n) in ks {
                                let on = self.shell.desk.bo_kit.as_deref() == Some(id.as_str());
                                if f.row_with(ui, on.then_some(mark::Mark::Ok), id, &format!("{note} · {n}"), card::Value::Nav, |_| ()).clicked() {
                                    self.shell.desk.bo_kit = if on { None } else { Some(id.clone()) };
                                }
                            }
                        });
                    }
                    Err(e) => match crate::house::settings::zk_away(e) {
                        Some(line) => states::note_box(ui, &line),
                        None => said_hint(ui, "buyer_orders-said-3", lang, &e.name, &e.detail, true),
                    },
                }
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                chosen_row(ui, &input, lang);
                field(ui, &t(crate::house::i18n::Key::BoNote), None, |ui| {
                    input::line(ui, &mut self.shell.desk.bo_note, "");
                });
                let send = if co { t(crate::house::i18n::Key::OrdersCompanyExportKey) } else { t(crate::house::i18n::Key::OrdersEvidenceKey) };
                Self::sheet_keys_left(keys, &t(crate::house::i18n::Key::CfBack), Some((&send, Role::Commit, self.shell.desk.bo_kit.is_some() || !input.trim().is_empty())), Some(("bo-evidence", crate::platform::Pick::FileOrFolder)));
                tags = (vec!["commitSend"], Some("kits"));
                tags
            });
            if let Some(p) = Self::sheet_picked(&ctx, "bo-evidence").or_else(|| self.sheet_dropped(&ctx)) {
                input = p;
            }
            if let Some(k) = go {
                hit = tags.0.get(k).copied();
            } else if back {
                close = tags.1;
            }
        }
        let closed = row.as_ref().is_some_and(|r| eravon_orders::table::done(&r.row));
        if self.shell.desk.bo_sheet == Some("settle") || (o.offer.is_some() && !closed) {
            let (tags, go, back) = Self::sheet_pick(&ctx, "bo-settle", eravon_ui::metrics::MODAL_W, |ui, keys| {
                let tags: (Vec<&'static str>, Option<&'static str>);
                Self::sheet_head(ui, &t(crate::house::i18n::Key::BoActSettle), "");
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                match &o.offer {
                    Some(Ok(of)) => {
                        let seller = t(if of.seller_by_call { crate::house::i18n::Key::BoOfferSellerByCall } else { crate::house::i18n::Key::BoOfferSellerSigned });
                        kv::kv(ui, &[(&t(crate::house::i18n::Key::BoBuyerBps), kv::Val::Text(bps_word(u64::from(of.st.buyer_bps)))), (&t(crate::house::i18n::Key::BoExpiry), kv::Val::Mono(day_or(of.st.expiry))), (&t(crate::house::i18n::Key::BoOfferSeller), kv::Val::Text(seller))]);
                        if co {
                            states::note_box(ui, &t(crate::house::i18n::Key::BoCompanyConfirm));
                        }
                        if co {
                            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::OrdersCompanyExportKey), Role::Commit, true)));
                            tags = (vec!["settleCall"], Some("offer"));
                        } else {
                            Self::sheet_keys_n(keys, &t(crate::house::i18n::Key::CfBack), &[(&t(crate::house::i18n::Key::BoSettleSign), Role::Commit, true), (&t(crate::house::i18n::Key::BoSettleCall), Role::Secondary, true)]);
                            tags = (vec!["settleSign", "settleCall"], Some("offer"));
                        }
                    }
                    other => {
                        if let Some(Err(e)) = other {
                            said_hint(ui, "buyer_orders-said-4", lang, &e.name, &e.detail, true);
                        }
                        input::area_hint(ui, &mut input, 3, "eravon://open?e=…");
                        Self::sheet_keys_n(keys, &t(crate::house::i18n::Key::CfBack), &[(&t(crate::house::i18n::Key::BoOffersScan), Role::Primary, true), (&t(crate::house::i18n::Key::BoOfferRead), Role::Secondary, !input.trim().is_empty())]);
                        tags = (vec!["offersScan", "settle"], Some("offer"));
                    }
                }
                tags
            });
            if let Some(k) = go {
                hit = tags.0.get(k).copied();
            } else if back {
                close = tags.1;
            }
        }
        if let Some(a @ ("advance" | "expire" | "cancel" | "openCase" | "confirmHand")) = self.shell.desk.bo_sheet {
            let (tags, go, back) = Self::sheet_pick(&ctx, "bo-timeout", eravon_ui::metrics::MODAL_W, |ui, keys| {
                let tags: (Vec<&'static str>, Option<&'static str>);
                Self::sheet_head(ui, &crate::house::i18n::get_named(lang, &format!("bo.act.{a}")).0, "");
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                if let Some(r) = &row {
                    let goods = if r.product.is_empty() { t(crate::house::i18n::Key::BoTermsMissing) } else { r.product.clone() };
                    let (say, tone) = pre_state(r, lang).unwrap_or_else(|| (state_say(&r.row, lang), state_tone(&r.row)));
                    let mut v = vec![(t(crate::house::i18n::Key::PlTitle), kv::Val::Text(goods)), (t(crate::house::i18n::Key::OrdersAmount), kv::Val::Text(money(&r.row.amount))), (t(crate::house::i18n::Key::SetZkState), kv::Val::Mark(tone_mark(tone), say))];
                    if let (Some(Ok(bg)), "cancel" | "expire") = (&r.budget, a) {
                        v.push((t(crate::house::i18n::Key::BoArrived), kv::Val::Text(money(&bg.have))));
                        v.push((t(crate::house::i18n::Key::BoRefundTo), kv::Val::Mono(bg.payout.clone())));
                    }
                    if let (Some(Ok(c)), "advance") = (&o.advance, a) {
                        if let Some((b, s, j)) = &c.shares {
                            v.push((t(crate::house::i18n::Key::OrdersSplitBuyer), kv::Val::Text(money(b))));
                            v.push((t(crate::house::i18n::Key::OrdersSplitSeller), kv::Val::Text(money(s))));
                            v.push((t(crate::house::i18n::Key::OrdersSplitJudge), kv::Val::Text(money(j))));
                        }
                    }
                    let refs: Vec<(&str, kv::Val)> = v.iter().map(|(k, x)| (k.as_str(), x.clone())).collect();
                    kv::kv(ui, &refs);
                }
                let send = crate::house::i18n::get_named(lang, &format!("bo.act.{a}")).0;
                Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&send, Role::Commit, true)));
                tags = (vec![match a {
                    "advance" => "advanceSend",
                    "cancel" => "cancelSend",
                    "openCase" => "openSend",
                    "confirmHand" => "confirmHandSend",
                    _ => "expireSend",
                }], Some("timeout"));
                tags
            });
            if let Some(k) = go {
                hit = tags.0.get(k).copied();
            } else if back {
                close = tags.1;
            }
        }
        let mut copied_next = false;
        if let Some(h) = &o.handed {
            let (_, _, back) = Self::sheet_pick(&ctx, "bo-handed", eravon_ui::metrics::MODAL_W, |ui, keys| {
                Self::sheet_head(ui, &t(crate::house::i18n::Key::BoHandedTitle), "");
                kv::kv(ui, &[("to", kv::Val::Mono(h.to.clone())), ("data", kv::Val::Mono(h.data.clone()))]);
                if card_keys(ui, |ui| key::key(ui, &t(crate::house::i18n::Key::OrdersCopy), Role::Secondary, true).clicked()) {
                    ui.ctx().copy_text(h.data.clone());
                    copied_next = true;
                }
                Self::sheet_keys_n(keys, &t(crate::house::i18n::Key::CfBack), &[]);
            });
            if back {
                close = Some("handed");
            }
            if copied_next {
                self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Ok, head: t(crate::house::i18n::Key::SaidCopied), next: t(crate::house::i18n::Key::BoHandedNext), detail: None });
            }
        }
        if self.shell.desk.bo_sheet == Some("receipt") {
            if let Some(Some(rc)) = row.as_ref().map(|r| &r.receipt) {
                let (_, _, back) = Self::sheet_pick(&ctx, "bo-receipt", eravon_ui::metrics::MODAL_W, |ui, keys| {
                    Self::sheet_head(ui, &t(crate::house::i18n::Key::BoReceiptQr), "");
                    qr_plate(ui, rc);
                    if card_keys(ui, |ui| key::key(ui, &t(crate::house::i18n::Key::PlCopyReceipt), Role::Secondary, true).clicked()) {
                        ui.ctx().copy_text(rc.clone());
                        tell_copied(ui.ctx(), Some(rc.clone()));
                    }
                    Self::sheet_keys_n(keys, &t(crate::house::i18n::Key::CfBack), &[]);
                });
                if back {
                    close = Some("timeout");
                }
            }
        }
        if self.shell.desk.bo_sheet == Some("sweep") || o.owed.is_some() {
            let (tags, go, back) = Self::sheet_pick(&ctx, "bo-sweep", eravon_ui::metrics::MODAL_W, |ui, keys| {
                let tags: (Vec<&'static str>, Option<&'static str>);
                Self::sheet_head(ui, &t(crate::house::i18n::Key::BoActSweep), "");
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                let listed = eravon_vault::tokens::on_chain(&crate::house::coin::rows(), eravon_vault::tokens::chain());
                let mut names = vec![t(crate::house::i18n::Key::BoTokenOwn)];
                names.extend(listed.iter().map(|r| r.label.clone()));
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let at = listed.iter().position(|r| r.address.eq_ignore_ascii_case(input.trim())).map_or(0, |i| i + 1);
                field(ui, &t(crate::house::i18n::Key::BoToken), None, |ui| {
                    if let Some(i) = { let rows: Vec<eravon_ui::menu::Item> = refs.iter().enumerate().map(|(i, l)| eravon_ui::menu::Item::Row(eravon_ui::menu::Row { label: l, check: Some(i == at), ..Default::default() })).collect(); eravon_ui::menu::menu_key(ui, "bo-sweep-token", refs.get(at).copied().unwrap_or_default(), false, eravon_ui::house::DD_MENU_MIN_W, &rows).filter(|i| *i != at) } {
                        input = if i == 0 { String::new() } else { listed[i - 1].address.clone() };
                    }
                });
                match &o.owed {
                    Some(Ok(ow)) => {
                        ui.add_space(eravon_ui::metrics::CARD_GAP);
                        let tok = input.trim().to_string();
                        let m = |v: &str| if tok.is_empty() { money(v) } else { crate::house::coin::amount(lang, v, None, &tok, crate::house::coin::Form::Full) };
                        kv::kv(ui, &[(&t(crate::house::i18n::Key::BoOwed), kv::Val::Text(m(&ow.amount))), (&t(crate::house::i18n::Key::BoBalance), kv::Val::Text(m(&ow.balance)))]);
                        Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::BoActSweep), Role::Commit, true)));
                        tags = (vec!["sweepSend"], Some("owed"));
                    }
                    other => {
                        if let Some(Err(e)) = other {
                            said_hint(ui, "buyer_orders-said-5", lang, &e.name, &e.detail, true);
                        }
                        Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::BoOwedRead), Role::Primary, true)));
                        tags = (vec!["sweep"], Some("owed"));
                    }
                }
                tags
            });
            if let Some(k) = go {
                hit = tags.0.get(k).copied();
            } else if back {
                close = tags.1;
            }
        }
        for c in [o.card.as_ref().and_then(|x| x.as_ref().err()), o.refund.as_ref().and_then(|x| x.as_ref().err())].into_iter().flatten() {
            said_hint(ui, "buyer_orders-said-6", lang, &c.name, &c.detail, true);
        }
        self.shell.desk.bo_input = input.clone();
        if let Some(w) = close {
            if let Some(x) = self.shell.desk.sh.buyer_orders.as_mut().and_then(|x| x.open.as_mut()) {
                match w {
                    "card" => x.card = None,
                    "refund" => x.refund = None,
                    "kits" => x.kits = None,
                    "offer" => x.offer = None,
                    "handed" => x.handed = None,
                    "timeout" => {}
                    _ => x.owed = None,
                }
            }
            self.shell.desk.bo_sheet = None;
            self.shell.desk.bo_input.clear();
            self.shell.desk.bo_kit = None;
            self.shell.desk.bo_note.clear();
            return;
        }
        if hit == Some("export") {
            self.step(Step::BoExport, &[]);
            return;
        }
        if let Some(a @ ("settle" | "sweep" | "advance" | "expire" | "cancel" | "openCase" | "confirmHand")) = hit {
            if self.shell.desk.bo_sheet != Some(a) {
                self.shell.desk.bo_sheet = Some(a);
                self.shell.desk.bo_input.clear();
                return;
            }
        }
        let Some(a) = hit else { return };
        let text = input.trim().to_string();
        let kit = if a == "commitSend" { self.shell.desk.bo_kit.take() } else { None };
        let (first_kit, first_path) = if matches!(a, "disputeSend" | "disputePull") { (self.shell.desk.bo_kit.clone(), self.shell.desk.bo_first.trim().to_string()) } else { (None, String::new()) };
        let kit = kit.or(first_kit).unwrap_or_default();
        let mut args: Vec<(&str, String)> = Vec::new();
        let step = match a {
            "confirm" => Step::BoConfirmOpen,
            "confirmSend" => Step::BoConfirm,
            "confirmHandSend" => Step::BoConfirmHand,
            "dispute" => Step::BoRefundOpen,
            "disputeSend" | "disputePull" => {
                args.push(("way", if a == "disputePull" { "pull" } else { "transfer" }.into()));
                if !first_path.is_empty() {
                    args.push(("path", first_path.clone()));
                }
                Step::BoDispute
            }
            "commit" => Step::BoEvidenceOpen,
            "commitSend" => {
                if !text.is_empty() {
                    args.push(("path", text.clone()));
                }
                let note = std::mem::take(&mut self.shell.desk.bo_note);
                if !note.trim().is_empty() {
                    args.push(("note", note.trim().to_string()));
                }
                Step::BoCommit
            }
            "settle" => {
                args.push(("v", text.clone()));
                Step::BoOffer
            }
            "offersScan" => Step::BoOffersScan,
            "settleSign" => Step::BoSettle,
            "settleCall" => {
                args.push(("mode", "call".into()));
                Step::BoSettle
            }
            "advanceSend" => Step::BoAdvance,
            "expireSend" => Step::BoExpire,
            "cancelSend" => Step::BoCancel,
            "openSend" => Step::BoOpenCase,
            "sweep" => {
                args.push(("token", text.clone()));
                Step::BoOwed
            }
            _ => {
                args.push(("token", text.clone()));
                Step::BoSweep
            }
        };
        if !kit.is_empty() && matches!(step, Step::BoCommit | Step::BoDispute) {
            args.push(("kit", kit));
        }
        let refs: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
        self.step(step, &refs);
    }
}

impl Win {
    fn buyer_order_inner(&mut self, ui: &mut egui::Ui, o: &crate::house::buyer_orders::Open, r: &crate::house::buyer_orders::BRow, inner: crate::house::orders::Inner) {
        use super::casepages::Asked;
        use crate::house::orders::Inner;
        let lang = crate::lang::lang();
        let coin = |x: &str| if x.is_empty() { String::new() } else { crate::house::coin::amount(lang, x, r.row.decimals, &r.row.token, crate::house::coin::Form::Full) };
        let names = |a: &str| eravon_orders::table::short(a.rsplit(':').next().unwrap_or(a));
        let goods = if r.product.is_empty() { crate::house::i18n::t(crate::house::i18n::Key::BoUntitled) } else { r.product.clone() };
        let asked = match inner {
            Inner::Judge(tab) => {
                let fee = eravon_orders::detail::fee(&r.row);
                let fees = super::casepages::Fees { premium: coin(&fee.judge_fee), lf: coin(&fee.lf.0), deposit: coin(&fee.deposit) };
                let guild = o.bench.as_ref().and_then(|b| b.as_ref().ok()).and_then(|b| match &b.track {
                    crate::house::case::Track::Guild(g) => Some(eravon_orders::table::short(g)),
                    crate::house::case::Track::Named(_) => None,
                });
                super::casepages::bench_page(ui, tab, o.bench.as_ref(), o.judge.as_ref(), guild.as_deref(), &fees, r.row.chain.as_ref().map_or("", |v| v.judge.as_str()))
            }
            Inner::Docs(tab) => super::casepages::papers_page(ui, tab, o.papers.as_ref(), &goods, &crate::house::orders::row_label(&r.row), &coin, &names),
            Inner::History => {
                super::history::page(ui, lang, "bo-history", &r.seller, &o.history, self.shell.house_in_flight(crate::task::Kind::History));
                None
            }
            Inner::Lapse => {
                if let Some(Ok(b)) = o.bench.as_ref() {
                    if let Ok(q) = &b.quote {
                        let (who, guild) = match &b.track {
                            crate::house::case::Track::Guild(g) => (eravon_orders::table::short(g), true),
                            crate::house::case::Track::Named(j) => (o.judge.as_ref().and_then(|c| c.as_ref().ok()).and_then(|c| c.name.clone()).unwrap_or_else(|| eravon_orders::table::short(j)), false),
                        };
                        super::casepages::lapse_page(ui, &who, guild, &q.clocks);
                    }
                }
                None
            }
        };
        match asked {
            Some(Asked::Tab(n)) => {
                let v = n.to_string();
                self.step(if matches!(inner, Inner::Judge(_)) { Step::CaseJudge } else { Step::CaseDocs }, &[("tab", &v)]);
            }
            Some(Asked::Lapse(_)) => self.step(Step::LapseOpen, &[]),
            Some(Asked::Url(k)) => self.open_foreign(k),
            Some(Asked::Export) => self.step(Step::BoExport, &[]),
            Some(Asked::Text(_)) | Some(Asked::CheckImage) | None => {}
        }
    }

    fn buyer_overview(&mut self, ui: &mut egui::Ui, r: &crate::house::buyer_orders::BRow, d: &Out, co: bool, named: &str, advance: Option<&Result<eravon_after::act::AdvanceCard, crate::house::seller::Said>>) -> Option<&'static str> {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let mut hit: Option<&'static str> = None;
        let locked = super::orders::past_locked(r.row.past);
        let can = !locked && self.write_banner(ui, if co { crate::house::shell::Write::Local } else { crate::house::shell::Write::Chain });
        let why = super::orders::past_why(locked, lang);
        let judge = pick(d, &["overview", "judge"]);
        let judge_state = pick(d, &["overview", "judgeState"]);
        let opened = r.row.chain.as_ref().is_some_and(|v| v.phase != 0);
        let judge_say = match (opened, judge_state.is_empty()) {
            (false, _) => t(crate::house::i18n::Key::BoJudgeAtOpen),
            (true, true) => named.to_string(),
            (true, false) => crate::house::i18n::library(lang, &judge_state),
        };
        let chain_now = self.shell.desk.now();
        let past_t2 = r.row.chain.as_ref().is_some_and(|v| v.window_end != 0 && v.window_end <= chain_now);
        let redraw = r.row.status.as_ref().is_ok_and(|s| s.word == eravon_orders::status::Word::AwaitRedraw);
        let due_now = advance.is_some_and(|c| c.as_ref().is_ok_and(|c| c.due.ran_out()));
        let stamp = redraw || advance.is_some_and(|c| c.as_ref().map_or(true, |c| c.due.moves()));
        let acts: Vec<&'static str> = crate::house::buyer_orders::shown_acts(r).iter().copied().filter(|a| !(past_t2 && *a == "dispute") && (*a != "advance" || stamp)).collect();
        let has = |a: &str| acts.contains(&a);
        let amount = crate::house::coin::amount(lang, &r.row.amount, r.row.decimals, &r.row.token, crate::house::coin::Form::Short);
        let mut used: Vec<&'static str> = Vec::new();
        match r.budget.as_ref() {
            Some(b) => {
                if let Some(x) = self.budget_card(ui, r, can) {
                    hit = Some(x);
                }
                if b.is_ok() {
                    used.extend(["openCase", "cancel", "expire"]);
                }
            }
            None => {
                let (say, tone) = pre_state(r, lang).unwrap_or_else(|| (state_say(&r.row, lang), state_tone(&r.row)));
                let word = r.row.status.as_ref().ok().map(|s| s.word);
                let done = eravon_orders::table::done(&r.row);
                let (glyph, tint) = match (done, word) {
                    (true, _) => (Glyph::Ok, card::Tint::Ok),
                    (_, Some(eravon_orders::status::Word::Disputed | eravon_orders::status::Word::AwaitRedraw)) => (Glyph::Warn, card::Tint::Warn),
                    _ => (Glyph::Receipt, card::Tint::Blue),
                };
                let _ = tone;
                let deadline = if r.row.deadline == 0 { String::new() } else { format!("{} {}", t(crate::house::i18n::Key::OrdersColDeadline), day_or(r.row.deadline)) };
                let main: Vec<&'static str> = if done {
                    vec!["export"]
                } else if due_now && has("advance") {
                    vec!["advance"]
                } else if has("commit") {
                    vec!["commit"]
                } else if has("dispute") && has("confirm") {
                    vec!["dispute", "confirm"]
                } else if has("confirm") {
                    vec!["confirm"]
                } else {
                    Vec::new()
                };
                used.extend(main.iter().copied());
                card::form(ui, |ui, f| {
                    f.summary(ui, glyph, tint, &say, super::orders::past_pill(&r.row, lang).as_deref().map(|p| (mark::Mark::Todo, p)), &deadline, |ui| {
                        for a in &main {
                            let (label, on) = if *a == "export" { (t(crate::house::i18n::Key::IdExportGroup), true) } else { (crate::house::i18n::get_named(lang, &format!("bo.act.{a}")).0, can) };
                            if key::show(ui, key::Key::new(&label, if *a == "export" { Role::Secondary } else { Role::Guide }).enabled(on).why(&why)).clicked() {
                                hit = Some(a);
                            }
                        }
                    });
                    f.facts(ui, &[
                        (&t(crate::house::i18n::Key::OrdersAmount), card::Fact::Text(&amount)),
                        (&t(crate::house::i18n::Key::PlCands), card::Fact::Text(&judge_say)),
                        (&t(crate::house::i18n::Key::BoKvPlaced), card::Fact::Mono(&eravon_orders::when::day(r.row.placed))),
                    ]);
                });
            }
        }
        let held = r.pack.is_some();
        let (mut bench, mut goods_doc, mut history) = (false, false, false);
        let (mut receipt_copy, mut receipt_qr) = (false, false);
        let mut seller_copied = None;
        card::section(ui, &t(crate::house::i18n::Key::GoodsBasics), "", |ui| {
            card::form(ui, |ui, f| {
                seller_copied = if r.seller.is_empty() { f.info(ui, &t(crate::house::i18n::Key::IdSeatAuthor), card::Show::Text("·")) } else { f.info(ui, &t(crate::house::i18n::Key::IdSeatAuthor), card::Show::Short(&r.seller)) };
                history = f.go(ui, &t(crate::house::i18n::Key::PlHistory), "", !r.seller.is_empty(), "").clicked();
                match &r.lf_gap {
                    Some(Ok(g)) => f.info(ui, &t(crate::house::i18n::Key::BoColLfGap), card::Show::Text(&crate::house::coin::amount(lang, g, r.row.decimals, &r.row.token, crate::house::coin::Form::Full))),
                    Some(Err(e)) => f.info(ui, &t(crate::house::i18n::Key::BoColLfGap), card::Show::Mark(mark::Mark::Warn, &crate::house::refusal::line(lang, &e.name, &e.detail))),
                    None => None,
                };
                f.info(ui, &t(crate::house::i18n::Key::OrdersColDeadline), card::Show::Mono(&day_or(r.row.deadline)));
                if opened {
                    bench = f.go(ui, &t(crate::house::i18n::Key::PlCands), &judge_say, true, "").clicked();
                } else {
                    f.info(ui, &t(crate::house::i18n::Key::PlCands), card::Show::Text(&judge_say));
                }
                if held {
                    goods_doc = f.go(ui, &t(crate::house::i18n::Key::OrdersGoodsDetail), &t(if r.budget.is_some() { crate::house::i18n::Key::BoTermsSaved } else { crate::house::i18n::Key::CaseFrozen }), true, "").clicked();
                } else {
                    f.info(ui, &t(crate::house::i18n::Key::OrdersGoodsDetail), card::Show::Mark(mark::Mark::Warn, &t(crate::house::i18n::Key::BoTermsMissing)));
                }
                if let Some(Ok(bg)) = &r.budget {
                    f.info(ui, &t(crate::house::i18n::Key::SetOpenWindow), card::Show::Text(&crate::house::i18n::filln(crate::house::i18n::Key::BoOpenByLapse, &[&short_at(bg.until)])));
                    f.info(ui, &t(crate::house::i18n::Key::BoRefundTo), card::Show::Short(&bg.payout));
                }
                if r.receipt.is_some() {
                    receipt_copy = f.go(ui, &t(crate::house::i18n::Key::PlCopyReceipt), "", true, "").clicked();
                    receipt_qr = f.go(ui, &t(crate::house::i18n::Key::BoReceiptQr), "", true, "").clicked();
                }
            });
        });
        tell_copied(ui.ctx(), seller_copied);
        if let (true, Some(rc)) = (receipt_copy, &r.receipt) {
            ui.ctx().copy_text(rc.clone());
            tell_copied(ui.ctx(), Some(rc.clone()));
        }
        if receipt_qr {
            self.shell.desk.bo_sheet = Some("receipt");
        }
        if bench {
            self.step(Step::CaseJudge, &[("tab", "0")]);
        }
        if history {
            self.step(Step::BoHistory, &[]);
        }
        if goods_doc {
            self.step(Step::CaseDocs, &[("tab", "0")]);
        }
        if !r.access.note().is_empty() {
            states::note_box(ui, &crate::house::i18n::library(lang, r.access.note()));
        }
        let ops: Vec<&'static str> = acts.iter().copied().filter(|a| !used.contains(a)).collect();
        let seated = crate::house::complaint::seated(&r.row).is_some();
        let mut complain = false;
        if !ops.is_empty() || seated {
            card::section(ui, &t(crate::house::i18n::Key::OrdersOps), "", |ui| {
                card::form(ui, |ui, f| {
                    for a in &ops {
                        if f.go(ui, &crate::house::i18n::get_named(lang, &format!("bo.act.{a}")).0, "", can, &why).clicked() {
                            hit = Some(a);
                        }
                    }
                    if seated && f.go(ui, &t(crate::house::i18n::Key::OrdersComplain), "", true, "").clicked() {
                        complain = true;
                    }
                });
            });
        }
        if complain {
            self.complaint_start();
        }
        let keys = [t(if r.budget.is_some() { crate::house::i18n::Key::BoPayAddr } else { crate::house::i18n::Key::SetContractAddr }), t(crate::house::i18n::Key::IdAddress), t(crate::house::i18n::Key::OrdersKvStateRaw), t(crate::house::i18n::Key::BoSellerAddr), t(crate::house::i18n::Key::OrdersKvJudgeAddr), t(crate::house::i18n::Key::PlToken), t(crate::house::i18n::Key::OrdersAmount)];
        let mut rows = vec![(keys[0].as_str(), kv::Val::Mono(r.row.instance.clone())), (keys[1].as_str(), kv::Val::Mono(r.owner.clone()))];
        if !r.seller.is_empty() {
            rows.push((keys[3].as_str(), kv::Val::Mono(r.seller.clone())));
        }
        if !judge.is_empty() {
            rows.push((keys[4].as_str(), kv::Val::Mono(judge.clone())));
        }
        rows.push((keys[5].as_str(), kv::Val::Mono(r.row.token.clone())));
        rows.push((keys[6].as_str(), kv::Val::Text(crate::house::coin::amount(lang, &r.row.amount, r.row.decimals, &r.row.token, crate::house::coin::Form::Full))));
        if let Some(x) = super::state_raw(&r.row) {
            rows.push((keys[2].as_str(), kv::Val::Figure(x)));
        }
        details_card(ui, "bo-details", &rows);
        hit
    }

    fn budget_card(&mut self, ui: &mut egui::Ui, r: &crate::house::buyer_orders::BRow, can: bool) -> Option<&'static str> {
        let lang = crate::lang::lang();
        let why = super::orders::past_why(super::orders::past_locked(r.row.past), lang);
        let lacks = |a: &str| r.row.past && eravon_after::act::PAST_LACKS_ACTS.contains(&a);
        let why_of = |a: &str| if lacks(a) { super::orders::lacks_why(true, lang) } else { why.clone() };
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let b = match r.budget.as_ref()? {
            Ok(b) => b,
            Err(e) => {
                states::err_box(ui, "bo-budget", &crate::house::refusal::line(lang, &e.name, &e.detail), "", &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail));
                return None;
            }
        };
        let m = |v: &str| crate::house::coin::amount(lang, v, r.row.decimals, &r.row.token, crate::house::coin::Form::Short);
        let mut hit: Option<&'static str> = None;
        let mut open_uri: Option<String> = None;
        let mut copy: Option<String> = None;
        let have = m(&b.have);
        let have_tip = crate::house::coin::amount_tip(lang, &b.have, r.row.decimals, &r.row.token);
        let until = short_at(b.until);
        let payout = super::short(&b.payout);
        card::form(ui, |ui, f| {
            if b.lapsed {
                let sub = if b.revoked { t(crate::house::i18n::Key::BoRevokedTake) } else { crate::house::i18n::filln(crate::house::i18n::Key::BoNotOpenedBy, &[&until]) };
                f.summary(ui, Glyph::Vault, card::Tint::Warn, &t(crate::house::i18n::Key::BoActExpire), None, &sub, |ui| {
                    if key::show(ui, key::Key::new(&t(crate::house::i18n::Key::BoActExpire), Role::Commit).enabled(can && !lacks("expire")).why(&why_of("expire"))).clicked() {
                        hit = Some("expire");
                    }
                });
                f.facts(ui, &[(&t(crate::house::i18n::Key::BoArrived), card::Fact::Amount(&have_tip)), (&t(crate::house::i18n::Key::BoRefundTo), card::Fact::Mono(&payout))]);
                return;
            }
            if b.emptied.is_some() {
                f.summary(ui, Glyph::Vault, card::Tint::Blue, &crate::house::i18n::order_word(lang, eravon_cli::deskops::buyerpage::ended_word(b)), None, &t(crate::house::i18n::Key::BoNothingLeft), |_| ());
                f.facts(ui, &[(&t(crate::house::i18n::Key::BoArrived), card::Fact::Amount(&have_tip)), (&t(crate::house::i18n::Key::BoRefundTo), card::Fact::Mono(&payout))]);
                return;
            }
            if !b.paid() && b.deposit_short {
                f.summary(ui, Glyph::Vault, card::Tint::Warn, &t(crate::house::i18n::Key::PlTabPay), None, &crate::house::i18n::filln(crate::house::i18n::Key::BhNoteDepositShort, &[&r.product]), |_| ());
                f.facts(ui, &[(&t(crate::house::i18n::Key::BoArrived), card::Fact::Amount(&have_tip)), (&t(crate::house::i18n::Key::BoRefundTo), card::Fact::Mono(&payout))]);
            } else if !b.paid() {
                f.summary(ui, Glyph::Vault, card::Tint::Blue, &t(crate::house::i18n::Key::PlTabPay), None, &crate::house::i18n::filln(crate::house::i18n::Key::BoPayScan, &[&m(&b.owe)]), |ui| {
                    if key::show(ui, key::Key::new(&t(crate::house::i18n::Key::BoOpenInWallet), Role::Primary).enabled(!r.row.past).why(&if r.row.past { crate::house::i18n::get_named(lang, "err.E_A1_PAST_LISTING").0 } else { why.clone() })).clicked() {
                        open_uri = Some(b.pay_uri.clone());
                    }
                    if key::show(ui, key::Key::new(&t(crate::house::i18n::Key::IdDoCopy), Role::Secondary)).clicked() {
                        copy = Some(r.row.instance.clone());
                    }
                });
                f.free(ui, |ui| ui.vertical_centered(|ui| qr_plate(ui, &b.pay_uri)));
                let copied = f.info(ui, &t(crate::house::i18n::Key::BoPayAddr), card::Show::Short(&r.row.instance));
                tell_copied(ui.ctx(), copied);
                f.info(ui, &t(crate::house::i18n::Key::BoArrived), card::Show::Mark(mark::Mark::Busy, &crate::house::i18n::filln(crate::house::i18n::Key::BoWaitArrival, &[&have, &m(&b.need)])));
            } else {
                f.summary(ui, Glyph::Vault, card::Tint::Blue, &t(crate::house::i18n::Key::WlKindOpen), None, &crate::house::i18n::filln(crate::house::i18n::Key::BoWaitOpenBy, &[&until]), |ui| {
                    if b.buyer_may_open && matches!(r.access, eravon_after::act::Access::Key | eravon_after::act::Access::Company) && key::show(ui, key::Key::new(&t(crate::house::i18n::Key::BoActOpenCase), Role::Commit).enabled(can && !lacks("openCase")).why(&why_of("openCase"))).clicked() {
                        hit = Some("openCase");
                    }
                });
                f.facts(ui, &[(&t(crate::house::i18n::Key::BoArrived), card::Fact::Amount(&have_tip)), (&t(crate::house::i18n::Key::SetOpenWindow), card::Fact::Mono(&until)), (&t(crate::house::i18n::Key::BoRefundTo), card::Fact::Mono(&payout))]);
            }
            let cancel = if b.paid() { crate::house::i18n::Key::BoActCancelRefund } else { crate::house::i18n::Key::BoActCancel };
            if f.go(ui, &t(cancel), "", can && !lacks("cancel"), &why_of("cancel")).clicked() {
                hit = Some("cancel");
            }
            if b.paid() {
                f.note(ui, &t(crate::house::i18n::Key::BoCancelNote));
            }
        });
        if let Some(v) = copy {
            ui.ctx().copy_text(v.clone());
            tell_copied(ui.ctx(), Some(v));
        }
        if let Some(u) = open_uri {
            match crate::house::platform::open_url(&u) {
                Ok(()) => self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Ok, head: crate::house::i18n::t(crate::house::i18n::Key::BoHandoffSay), next: String::new(), detail: None }),
                Err(e) => self.shell.desk.sh.toast(crate::house::shell::Toast {
                    tone: crate::house::seller::Tone::Bad,
                    head: crate::house::i18n::t(crate::house::i18n::Key::BoNoWallet),
                    next: crate::house::i18n::t(crate::house::i18n::Key::BoNoWalletNext),
                    detail: Some(format!("E_APP_BROWSER · {e}")),
                }),
            }
        }
        hit
    }
}

pub(super) fn pre_state(r: &crate::house::buyer_orders::BRow, lang: crate::lang::Lang) -> Option<(String, eravon_ui::palette::Tone)> {
    use eravon_cli::deskops::buyerpage::Pre;
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    Some(match eravon_cli::deskops::buyerpage::pre_state(r)? {
        Pre::RevokedOpen => (t(crate::house::i18n::Key::BoStateRevokedOpen), eravon_ui::palette::Tone::Warn),
        Pre::Ended(w) => (crate::house::i18n::order_word(lang, w), eravon_ui::palette::Tone::Grey),
        Pre::LapsedOpen => (t(crate::house::i18n::Key::BoStateLapsedOpen), eravon_ui::palette::Tone::Warn),
        Pre::AwaitOpen => (t(crate::house::i18n::Key::BoStateAwaitOpen), eravon_ui::palette::Tone::Warn),
        Pre::AwaitPay => (crate::house::i18n::order_word(lang, eravon_orders::status::Word::AwaitPay), eravon_ui::palette::Tone::Blue),
    })
}

fn bps_word(bps: u64) -> String {
    let (i, f) = (bps / 100, bps % 100);
    if f == 0 { format!("{i}%") } else if f % 10 == 0 { format!("{i}.{}%", f / 10) } else { format!("{i}.{f:02}%") }
}

fn judge_named(o: &crate::house::buyer_orders::Open, addr: &str) -> String {
    o.judge
        .as_ref()
        .and_then(|c| c.as_ref().ok())
        .filter(|c| addr.is_empty() || c.address.eq_ignore_ascii_case(addr))
        .and_then(|c| c.name.clone())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| eravon_orders::table::short(addr))
}
