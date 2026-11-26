use super::*;
use crate::house::action::Action as Step;

pub(super) fn state_tone(r: &eravon_orders::table::Row) -> eravon_ui::palette::Tone {
    use eravon_ui::palette::Tone;
    use eravon_watch::order::State as L;
    match &r.status {
        Err(e) if crate::house::shell::not_read_yet(&e.0) => Tone::Grey,
        Err(_) => Tone::Bad,
        Ok(s) if s.word.done() => Tone::Grey,
        Ok(s) if matches!(s.word, eravon_orders::status::Word::Disputed | eravon_orders::status::Word::AwaitRedraw) => Tone::Warn,
        Ok(_) if !opened(r) => match r.local {
            Some(L::Blocked) => Tone::Bad,
            Some(L::PriceStale | L::DepositShort | L::PendingConsent | L::AwaitPrev) => Tone::Warn,
            Some(L::Ready) if r.note == eravon_watch::send::OPEN_OFF_NOTE => Tone::Blue,
            _ => Tone::Ok,
        },
        Ok(_) => Tone::Ok,
    }
}

fn opened(r: &eravon_orders::table::Row) -> bool {
    r.chain.as_ref().is_some_and(|v| v.phase != 0)
}

pub(super) const STATE_COL: table::Col = table::Col::Auto { min: eravon_ui::house::COL_STATE, max: f32::MAX };

pub(super) fn state_say(r: &eravon_orders::table::Row, lang: crate::lang::Lang) -> String {
    match &r.status {
        Ok(s) => crate::house::i18n::order_word(lang, s.word),
        Err(e) => unread_say(lang, e).0,
    }
}

pub(super) fn unread_say(lang: crate::lang::Lang, e: &(String, String)) -> (String, eravon_ui::palette::Tone) {
    if crate::house::shell::not_read_yet(&e.0) {
        (crate::house::i18n::get(lang, crate::house::i18n::Key::SyncNotYet).0, eravon_ui::palette::Tone::Grey)
    } else {
        (crate::house::i18n::get(lang, crate::house::i18n::Key::OrdersStateUnread).0, eravon_ui::palette::Tone::Bad)
    }
}

pub(super) fn state_why(r: &eravon_orders::table::Row, lang: crate::lang::Lang) -> Option<String> {
    use crate::house::i18n::Key as K;
    use eravon_watch::order::State as L;
    let t = |k: K| crate::house::i18n::get(lang, k).0;
    if opened(r) {
        if let Some(e) = r.chain.as_ref().and_then(eravon_orders::status::ended) {
            return Some(t(ended_key(e)));
        }
        return r.status.as_ref().ok().filter(|s| !s.reason.is_empty()).map(|s| crate::house::i18n::library(lang, &s.reason));
    }
    let before = r.series.as_ref().filter(|x| x.stage > 1).map(|x| (x.stage - 1).to_string());
    Some(match r.local? {
        L::PriceStale => t(K::OrdersWhyPrice),
        L::DepositShort => t(K::OrdersWhyDeposit),
        L::PendingConsent => t(K::OrdersWhyConsent),
        L::AwaitPrev => before.map_or_else(|| t(K::OrdersWhyPrevAny), |n| crate::house::i18n::filln_in(lang, K::OrdersWhyPrev, &[&n])),
        L::Blocked if r.note.contains(eravon_watch::poll::STAGE_LAPSED) => before.map_or_else(|| t(K::OrdersStateBlocked), |n| crate::house::i18n::filln_in(lang, K::OrdersWhyStageLapsed, &[&n])),
        L::Blocked => match (stage_share(&r.note), before) {
            (Some(10_000), Some(n)) => crate::house::i18n::filln_in(lang, K::OrdersWhyStageRefunded, &[&n]),
            (Some(bps), Some(n)) => crate::house::i18n::filln_in(lang, K::OrdersWhyStageShare, &[&n, &pct_of(bps)]),
            (Some(_), None) => t(K::OrdersWhyStageOff),
            (None, _) => named_in(&r.note).and_then(|n| crate::house::refusal::what(lang, n)).unwrap_or_else(|| t(K::OrdersStateBlocked)),
        },
        L::Ready if r.note == eravon_watch::send::OPEN_OFF_NOTE => t(K::OrdersAwaitOpen),
        _ => return None,
    })
}

fn ended_key(e: eravon_orders::status::Ended) -> crate::house::i18n::Key {
    use crate::house::i18n::Key as K;
    use eravon_orders::status::Ended as E;
    match e {
        E::DeliveryLate => K::OrdersEndDeliveryLate,
        E::NotAccepted => K::OrdersEndNotAccepted,
        E::SellerRefund => K::OrdersEndSellerRefund,
        E::Declined => K::OrdersEndDeclined,
        E::Resigned => K::OrdersEndResigned,
        E::NoneDrawn => K::OrdersEndNoneDrawn,
        E::ResignedInDispute => K::OrdersEndResignedInDispute,
        E::DisputedAfterResign => K::OrdersEndDisputedAfterResign,
        E::JudgeLate => K::OrdersEndJudgeLate,
    }
}

pub(super) fn stage_share(note: &str) -> Option<u64> {
    let head = note.split(STAGE_BPS).next().filter(|h| h.len() < note.len())?;
    let digits: String = head.trim_end().chars().rev().take_while(char::is_ascii_digit).collect::<Vec<_>>().into_iter().rev().collect();
    digits.parse().ok()
}

const STAGE_BPS: &str = " 基点:";

pub(super) fn named_in(note: &str) -> Option<&str> {
    note.split(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')).find(|w| w.starts_with("E_") && w.len() > 2)
}

pub(super) fn drew_tx(note: &str) -> Option<&str> {
    note.strip_prefix(DREW).filter(|tx| tx.starts_with("0x"))
}

const DREW: &str = "已抽签:";

pub(super) fn note_say(r: &eravon_orders::table::Row, note: &str, lang: crate::lang::Lang) -> String {
    if let Some(w) = state_why(r, lang).filter(|_| !opened(r)) {
        return w;
    }
    if let Some((name, detail)) = eravon_watch::send::missed_parts(note) {
        return crate::house::refusal::line(lang, name, detail);
    }
    if let Some(w) = named_in(note).and_then(|n| crate::house::refusal::what(lang, n)) {
        return w;
    }
    without_hex(&split_raw(&crate::house::i18n::library(lang, note)).0)
}

fn without_hex(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '0' && b.get(i + 1) == Some(&'x') {
            let n = b[i + 2..].iter().take_while(|c| c.is_ascii_hexdigit()).count();
            if n >= 20 {
                i += 2 + n;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out.replace("()", "").replace("\u{ff08}\u{ff09}", "").replace("  ", " ").trim().to_string()
}

pub(super) fn state_raw(r: &eravon_orders::table::Row) -> Option<String> {
    r.status.as_ref().err().map(|(n, d)| format!("{n} · {d}"))
}

pub(super) fn pct_of(bps: u64) -> String {
    let (i, f) = (bps / 100, bps % 100);
    if f == 0 { format!("{i}%") } else if f % 10 == 0 { format!("{i}.{}%", f / 10) } else { format!("{i}.{f:02}%") }
}

pub(super) fn super_split(s: &str) -> String {
    split_raw(s).0
}

pub(super) fn thousands(n: u64) -> String {
    let d = n.to_string();
    let mut out = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub(super) fn day_or(t: u64) -> String {
    if t == 0 { "—".to_string() } else { short_at(t) }
}

pub(super) fn short_at(t: u64) -> String {
    eravon_orders::when::short(t)
}

impl Win {
    pub(super) fn info_sheet(&mut self, ctx: &egui::Context) {
        let Some((title, rows)) = self.shell.desk.info.clone() else { return };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let (_, hit, back) = Self::sheet_pick(ctx, "order-info", eravon_ui::metrics::MODAL_W, |ui, keys| {
            eravon_ui::sheet::title(ui, &crate::house::i18n::get_named(lang, title).0, "");
            for (k, v, mono) in &rows {
                let copy = t(crate::house::i18n::Key::OrdersCopy);
                if value_row(ui, k, v, *mono, (*mono && !v.is_empty()).then_some((copy.as_str(), true))) {
                    ui.ctx().copy_text(v.clone());
                    tell_copied(ui.ctx(), Some(v.clone()));
                }
            }
            Self::sheet_keys_n(keys, "", &[(&t(crate::house::i18n::Key::ToastClose), Role::Primary, true)]);
        });
        if hit.is_some() || back {
            self.shell.desk.info = None;
        }
    }

    pub(super) fn orders_page(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        if self.shell.desk.sh.seller.orders.as_ref().is_some_and(|o| !o.fresh_for(&self.shell.desk.addr)) && !self.shell.page_in_flight(crate::task::Kind::Orders) {
            self.shell.orders_load();
        }
        let Some(os) = self.shell.desk.sh.seller.orders.clone() else {
            if self.shell.page_in_flight(crate::task::Kind::Orders) {
                Self::home_skeleton(ui, 0);
            }
            let _ = &t;
            return;
        };
        if os.sel.is_some() {
            self.order_detail(ui, &os);
        } else {
            self.order_list(ui, &os);
        }
        self.paste_card(ui.ctx());
        self.info_sheet(ui.ctx());
        self.deliver_card(ui.ctx());
        self.refund_card(ui.ctx());
        self.evidence_card(ui.ctx());
        self.settle_card(ui.ctx());
    }

    pub(super) fn order_list(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let (mut q, mut from, mut to) = (os.search.q.clone(), os.search.from.clone(), os.search.to.clone());
        let mut changed = false;
        let w = ui.available_width();
        let (row, _) = ui.allocate_exact_size(egui::vec2(w, eravon_ui::metrics::INPUT_H), egui::Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
        child.spacing_mut().item_spacing.x = eravon_ui::metrics::S2;
        let qw = eravon_ui::house::search_beside_dates(w);
        changed |= input::search(&mut child, &mut q, &t(crate::house::i18n::Key::OrdersSearchHint), qw).changed();
        changed |= date_box(&mut child, "orders-from", &mut from, &t(crate::house::i18n::Key::DateFrom), crate::house::desk::wall_now());
        changed |= date_box(&mut child, "orders-to", &mut to, &t(crate::house::i18n::Key::DateTo), crate::house::desk::wall_now());
        if changed {
            let say = format!("{q} · {from} · {to}");
            if q.is_empty() && from.is_empty() && to.is_empty() {
                self.step(Step::OrderSearchOff, &[]);
            } else {
                let _ = say;
                self.step(Step::OrderSearch, &[("q", &q), ("from", &from), ("to", &to)]);
            }
        }
        let mut open: Option<String> = None;
        if let (Some(Ok(_)), Some(u)) = (&os.table, &os.unscanned) {
            states::err_box(ui, "orders-unscanned", &t(crate::house::i18n::Key::OrdersScanRefused), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&u.name, &u.detail));
        }
        match &os.table {
            Some(Err(s)) => states::err_box(ui, "orders", &t(crate::house::i18n::Key::BoListFailed), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
            Some(Ok(_)) => {
                let shown = os.shown();
                let heads = [t(crate::house::i18n::Key::OrdersColNo), t(crate::house::i18n::Key::IdSeatGrantee), t(crate::house::i18n::Key::PlTitle), t(crate::house::i18n::Key::OrdersColDeadline), t(crate::house::i18n::Key::OrdersAmount), t(crate::house::i18n::Key::SetZkState)];
                let cols = [
                    table::col(&heads[0], table::Col::Px(eravon_ui::house::COL_ORDER_NO)),
                    table::col(&heads[1], table::Col::Px(eravon_ui::house::COL_WHO)),
                    table::col(&heads[2], eravon_ui::house::REST),
                    table::col(&heads[3], table::Col::Px(eravon_ui::house::COL_DEADLINE)),
                    table::col(&heads[5], STATE_COL),
                    table::col_r(&heads[4], table::Col::Auto { min: eravon_ui::house::COL_AMOUNT, max: f32::MAX }),
                    table::CHEV,
                ];
                let make = |i: usize| {
                        let r = &shown[i];
                        let buyer = if r.label.is_empty() { "—".to_string() } else { r.label.clone() };
                        let product = if r.product.is_empty() { t(crate::house::i18n::Key::BoTermsMissing) } else { r.product.clone() };
                        let product = match &r.series {
                            Some(x) => crate::fill_in!(lang, crate::house::i18n::Key::OrdersSeriesStage; product, x.stage.to_string(), x.stages.to_string()),
                            None => product,
                        };
                        let cells = vec![
                            table::Cell::Mono(if r.number.is_empty() { "·".into() } else { r.number.clone() }),
                            table::Cell::Text(buyer),
                            table::Cell::Text(product),
                            table::Cell::Mono(day_or(r.deadline)),
                            state_cell(r, lang),
                            table::Cell::Amount(crate::house::coin::amount_tip(lang, &r.amount, r.decimals, &r.token)),
                            table::Cell::Chev,
                        ];
                        table::Row { cells, click: true, gone: false, on: false }
                };
                let empty = if os.search.on { t(crate::house::i18n::Key::OrdersNoHit) } else { t(crate::house::i18n::Key::HomeNone) };
                if let Some(i) = table::table_with(ui, "orders", table::Spec::new(&cols, self.shell.desk.rows_gen).head(true).empty(&empty), shown.len(), make).clicked {
                    open = Some(shown[i].instance.clone());
                }
            }
            None => Self::home_skeleton(ui, 0),
        }
        if let Some(i) = open {
            let (v, now) = (i.clone(), self.shell.desk.now());
            let _ = (i, now);
            self.step(Step::OrderRow, &[("instance", &v)]);
        }
    }

    pub(super) fn order_detail(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let sel = os.sel.clone().unwrap_or_default();
        let r = match os.table.as_ref() {
            None => {
                Self::home_skeleton(ui, 0);
                return;
            }
            Some(Err(e)) => {
                states::err_box(ui, "order-table", &t(crate::house::i18n::Key::BoListFailed), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail));
                return;
            }
            Some(Ok(rows)) => match rows.iter().find(|r| r.instance.eq_ignore_ascii_case(&sel)) {
                Some(r) => r.clone(),
                None => {
                    states::err_box(ui, "order-none", &t(crate::house::i18n::Key::OrdersNotInTable), &t(crate::house::i18n::Key::OrdersNoneNext), &t(crate::house::i18n::Key::U3RawError), &format!("E_ORDERS_NONE · {sel}"));
                    return;
                }
            },
        };
        if let Some(inner) = os.inner {
            self.order_inner(ui, os, &r, inner);
            return;
        }
        {
            let goods = if r.product.is_empty() { t(crate::house::i18n::Key::BoTermsMissing) } else { r.product.clone() };
            let number = crate::house::orders::row_label(&r);
            let mut sub = format!("{number} · {}", crate::house::coin::amount(lang, &r.amount, r.decimals, &r.token, crate::house::coin::Form::Short));
            if let Some(w) = state_why(&r, lang) {
                sub = format!("{sub} · {w}");
            }
            Self::hero(ui, &goods, &sub, &[hero_pill(&state_say(&r, lang), state_tone(&r))]);
        }
        let has_dispute = os.detail.as_ref().is_some_and(|d| d.dispute.is_some());
        let tabs: Vec<crate::house::orders::Tab> = crate::house::orders::Tab::ALL.into_iter().filter(|x| *x != crate::house::orders::Tab::Dispute || has_dispute).collect();
        let labels: Vec<String> = tabs.iter().map(|x| crate::house::i18n::get_named(lang, x.key()).0).collect();
        let refs: Vec<&str> = labels.iter().map(|x| x.as_str()).collect();
        let at = tabs.iter().position(|x| *x == os.tab).unwrap_or(0);
        if let Some(i) = eravon_ui::seg::tabs(ui, &&refs.join("\u{1f}"), &refs, at) {
            let tab = tabs[i];
            self.step(Step::OrderTab, &[("v", tab.word())]);
        }
        let reading = self.shell.house_in_flight(crate::task::Kind::Orders);
        self.complaint_sheet(ui.ctx(), &r.instance, os.complaint.as_ref(), os.said.as_ref(), reading);
        match tabs.get(at).copied().unwrap_or_default() {
            crate::house::orders::Tab::Overview => self.overview_tab(ui, os, &r),
            crate::house::orders::Tab::Delivery => self.delivery_tab(ui, os, &r),
            crate::house::orders::Tab::Dispute => self.dispute_tab(ui, os),
            crate::house::orders::Tab::Record => self.record_tab(ui, os),
            crate::house::orders::Tab::Fee => self.fee_tab(ui, os, &r),
        }
    }

    fn order_inner(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders, r: &eravon_orders::table::Row, inner: crate::house::orders::Inner) {
        use super::casepages::Asked;
        use crate::house::orders::Inner;
        let lang = crate::lang::lang();
        let coin = |x: &str| crate::house::coin::amount(lang, x, r.decimals, &r.token, crate::house::coin::Form::Full);
        let names = |a: &str| self.cand_label(a);
        let asked = match inner {
            Inner::Judge(tab) => {
                let fee = os.detail.as_ref().map(|d| d.fee.clone()).unwrap_or_default();
                let fees = super::casepages::Fees { premium: coin(&fee.judge_fee), lf: coin(&fee.lf.0), deposit: coin(&fee.deposit) };
                let guild = os.bench.as_ref().and_then(|b| b.as_ref().ok()).and_then(|b| match &b.track {
                    crate::house::case::Track::Guild(g) => Some(self.cand_label(g)),
                    crate::house::case::Track::Named(_) => None,
                });
                super::casepages::bench_page(ui, tab, os.bench.as_ref(), os.judge.as_ref(), guild.as_deref(), &fees, r.chain.as_ref().map_or("", |v| v.judge.as_str()))
            }
            Inner::Docs(tab) => {
                let goods = if r.product.is_empty() { crate::house::i18n::t(crate::house::i18n::Key::BoTermsMissing) } else { r.product.clone() };
                let number = crate::house::orders::row_label(r);
                super::casepages::papers_page(ui, tab, os.papers.as_ref(), &goods, &number, &coin, &names)
            }
            Inner::History => {
                super::history::page(ui, lang, "order-history", &r.buyer, &os.history, self.shell.house_in_flight(crate::task::Kind::History));
                None
            }
            Inner::Lapse => {
                if let Some(Ok(b)) = os.bench.as_ref() {
                    if let Ok(q) = &b.quote {
                        let (who, guild) = match &b.track {
                            crate::house::case::Track::Guild(g) => (self.cand_label(g), true),
                            crate::house::case::Track::Named(j) => (os.judge.as_ref().and_then(|c| c.as_ref().ok()).and_then(|c| c.name.clone()).unwrap_or_else(|| self.cand_label(j)), false),
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
            Some(Asked::Export) => self.step(Step::OrderExport, &[]),
            Some(Asked::Text(_)) | Some(Asked::CheckImage) | None => {}
        }
    }

    pub(super) fn cand_label(&self, a: &str) -> String {
        let seller = &self.shell.desk.sh.seller;
        let paged: Vec<(String, String)> = seller.cands.as_ref().and_then(|c| c.rows.as_ref().ok()).map(|rows| rows.iter().filter(|b| !b.row.label.is_empty()).map(|b| (b.row.address.clone(), b.row.label.clone())).collect()).unwrap_or_default();
        seller
            .goods
            .as_ref()
            .and_then(|g| eravon_goods::goods::name_of(&g.names, a))
            .or_else(|| eravon_goods::goods::name_of(&paged, a))
            .unwrap_or_else(|| eravon_orders::table::short(a.rsplit(':').next().unwrap_or(a)))
    }

    pub(super) fn open_foreign(&mut self, k: &'static str) {
        self.open_foreign_of(k, None);
    }

    pub(super) fn open_foreign_of(&mut self, k: &'static str, member: Option<&str>) {
        match member {
            Some(a) => self.step(Step::UrlOpen, &[("k", k), ("a", a)]),
            None => self.step(Step::UrlOpen, &[("k", k)]),
        }
        let Some((u, v)) = self.shell.desk.sh.url_asked.clone() else { return };
        if v != "ok" {
            return;
        }
        let said = match crate::house::platform::open_foreign(&u) {
            Ok(()) => crate::house::shell::Toast { tone: crate::house::seller::Tone::Ok, head: crate::house::i18n::t(if k == "goods-page" { crate::house::i18n::Key::UrlPageOpened } else { crate::house::i18n::Key::UrlHomeOpened }), next: String::new(), detail: None },
            Err(e) => crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: crate::house::i18n::t(crate::house::i18n::Key::UrlNotOpened), next: String::new(), detail: Some(e) },
        };
        self.shell.desk.sh.toast(said);
    }

    pub(super) fn overview_tab(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders, r: &eravon_orders::table::Row) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(d) = &os.detail else { return };
        let ov = &d.overview;
        let order = os.order.clone();
        let co = os.company;
        let locked = past_locked(r.past);
        let can = !locked && self.write_banner(ui, if co { crate::house::shell::Write::Local } else { crate::house::shell::Write::Chain });
        let why = past_why(locked, lang);
        let acts = crate::house::orders::shown_acts(r);
        let has = |a: eravon_orders::status::Act| acts.contains(&a);
        let face = os.face.as_ref().and_then(|f| f.as_ref().ok());
        let live = face.is_some_and(|f| f.word != "closed");
        let disputed = face.is_some_and(|f| f.disputed) && live;
        let done = eravon_orders::table::done(r);
        let pending = os.order.as_ref().is_some_and(|o| o.pending_at().is_some());
        let named = self.cand_label(&ov.judge.addr);
        let judge = match (ov.judge.addr.is_empty(), d.chosen.as_deref()) {
            (true, Some(c)) => self.cand_label(c),
            (true, None) => t(crate::house::i18n::Key::OrdersFacePendingRedraw),
            (false, _) => format!("{} · {}", named, crate::house::i18n::judge_state(lang, ov.judge.state)),
        };
        #[derive(Clone, Copy, PartialEq)]
        enum Main {
            Deliver,
            Evidence,
            Export,
            Open,
            Nothing,
        }
        let ready = os.co.is_none() && order.as_ref().is_some_and(|o| o.state == eravon_watch::order::State::Ready) && r.chain.is_none();
        let main = if has(eravon_orders::status::Act::Deliver) {
            Main::Deliver
        } else if disputed {
            Main::Evidence
        } else if done {
            Main::Export
        } else if ready && !pending {
            Main::Open
        } else {
            Main::Nothing
        };
        let (glyph, tint) = match (done, disputed) {
            (true, _) => (Glyph::Ok, card::Tint::Ok),
            (_, true) => (Glyph::Warn, card::Tint::Warn),
            _ => (Glyph::Receipt, card::Tint::Blue),
        };
        let deadline = if r.deadline == 0 { String::new() } else { format!("{} {}", t(crate::house::i18n::Key::OrdersColDeadline), day_or(r.deadline)) };
        let mut main_hit = false;
        let watch = order.as_ref().map(crate::house::orders::line);
        card::form(ui, |ui, f| {
            f.summary(ui, glyph, tint, &state_say(r, lang), past_pill(r, lang).as_deref().map(|p| (Mark::Todo, p)), &deadline, |ui| {
                let (label, role, on) = match main {
                    Main::Deliver => (t(crate::house::i18n::Key::OrdersDeliverKey), Role::Guide, can),
                    Main::Evidence => (t(crate::house::i18n::Key::OrdersEvidenceKey), Role::Guide, can && os.order.is_some()),
                    Main::Export => (t(crate::house::i18n::Key::IdExportGroup), Role::Secondary, true),
                    Main::Open => (t(crate::house::i18n::Key::OrdersOpenKey), Role::Guide, can),
                    Main::Nothing => return,
                };
                main_hit = key::show(ui, key::Key::new(&label, role).enabled(on).why(&why)).clicked();
            });
            f.facts(ui, &[
                (&t(crate::house::i18n::Key::OrdersAmount), card::Fact::Amount(&crate::house::coin::amount_tip(lang, &r.amount, r.decimals, &r.token))),
                (&t(crate::house::i18n::Key::PlCands), card::Fact::Text(&judge)),
                (&t(crate::house::i18n::Key::BoKvPlaced), card::Fact::Mono(&eravon_orders::when::day(r.placed))),
            ]);
        });
        if main_hit {
            match main {
                Main::Deliver => self.step(Step::DeliverOpen, &[]),
                Main::Evidence => self.step(Step::EvidenceOpen, &[]),
                Main::Export => self.step(Step::OrderExport, &[]),
                Main::Open => self.step(Step::OrderDeploy, &[]),
                Main::Nothing => {}
            }
        }
        let who = if r.label.is_empty() { t(crate::house::i18n::Key::OrdersNoLabel) } else { r.label.clone() };
        let (mut rename, mut goods_doc, mut bench, mut history) = (false, false, false, false);
        card::section(ui, &t(crate::house::i18n::Key::GoodsBasics), "", |ui| {
            card::form(ui, |ui, f| {
                let product = if r.product.is_empty() { t(crate::house::i18n::Key::BoTermsMissing) } else { r.product.clone() };
                f.info(ui, &t(crate::house::i18n::Key::PlTitle), card::Show::Text(&product));
                if watch.is_some() && !opened(r) {
                    let say = state_why(r, lang).unwrap_or_else(|| state_say(r, lang));
                    f.info(ui, &t(crate::house::i18n::Key::SetZkState), card::Show::Mark(tone_mark(state_tone(r)), &say));
                }
                rename = f.go(ui, &t(crate::house::i18n::Key::IdSeatGrantee), &who, true, "").clicked();
                history = f.go(ui, &t(crate::house::i18n::Key::PlHistory), "", !r.buyer.is_empty(), "").clicked();
                goods_doc = f.go(ui, &t(crate::house::i18n::Key::OrdersGoodsDetail), &t(crate::house::i18n::Key::CaseFrozen), os.order.is_some(), "").clicked();
                bench = f.go(ui, &t(crate::house::i18n::Key::PlCands), &judge, os.order.is_some(), "").clicked();
            });
        });
        if rename {
            self.shell.desk.order_label = Some(r.label.clone());
        }
        if history {
            self.step(Step::OrderHistory, &[]);
        }
        if goods_doc {
            self.step(Step::OrderTerms, &[("v", "open")]);
        }
        if bench {
            self.step(Step::CaseJudge, &[("tab", "0")]);
        }
        self.label_sheet(ui.ctx(), r);
        let Some(o) = order else {
            states::banner(ui, states::Banner::Note, &t(crate::house::i18n::Key::OrdersNoRecord), |_| ());
            self.overview_tail(ui, os, r);
            return;
        };
        let (_, gap_row) = crate::house::orders::line(&o);
        let face_say = matches!(&os.face, Some(Ok(f)) if !f.say.is_empty() || f.redraw.is_some() || f.sweep.is_some()) || matches!(&os.face, Some(Err(_)));
        let pend = o.pending().zip(o.pending_at());
        let paste = has(eravon_orders::status::Act::PasteConfirm);
        if gap_row.is_some() || !o.note.is_empty() || (!o.notice.is_empty() && o.notice != o.note) || face_say || pend.is_some() || paste {
            let mut ask: Option<crate::house::action::Action> = None;
            let mut sig = os.sig_confirm.clone();
            let mut sig_go = false;
            let mut view_tx: Option<String> = None;
            card::section(ui, &t(crate::house::i18n::Key::OrdersProgress), "", |ui| {
                card::form(ui, |ui, f| {
                    let mut raw: Vec<String> = Vec::new();
                    let one = |f: &mut card::Form, ui: &mut egui::Ui, words: &str| {
                        if !words.is_empty() {
                            let _ = f.line(ui, None, words, "", false, false, c(C::Ink), |_| ());
                        }
                    };
                    if let Some(g) = &gap_row {
                        one(f, ui, &crate::house::i18n::library(lang, g));
                    }
                    if !o.note.is_empty() {
                        match drew_tx(&o.note) {
                            Some(tx) => {
                                let (_, hit) = f.line(ui, Some(card::Lead::Mark(Mark::Ok)), &t(crate::house::i18n::Key::OrdersDrew), "", false, false, c(C::Ink), |ui| key::show(ui, key::Key::new(&t(crate::house::i18n::Key::OrdersViewTx), Role::Secondary)).clicked());
                                if hit {
                                    view_tx = Some(tx.to_string());
                                }
                            }
                            None if o.note == eravon_watch::send::EXPIRED_NOTE && !o.expire_tx.is_empty() => {
                                let amount = crate::house::coin::amount(lang, &o.balance, eravon_watch::units::of(&o), &o.token, crate::house::coin::Form::Short);
                                one(f, ui, &crate::house::i18n::filln_in(lang, crate::house::i18n::Key::OrdersExpiredDone, &[&amount, &eravon_orders::table::short(&o.buyer_payout), &eravon_orders::table::short(&o.expire_tx)]));
                            }
                            None if !opened(r) && state_why(r, lang).is_some() => {}
                            None => one(f, ui, &note_say(r, &o.note, lang)),
                        }
                        raw.push(o.note.clone());
                    }
                    if !o.notice.is_empty() && o.notice != o.note {
                        raw.push(o.notice.clone());
                    }
                    match &os.face {
                        Some(Ok(fv)) => {
                            if !fv.say.is_empty() {
                                let m = if fv.word == "resigned" { Mark::Bad } else { Mark::Warn };
                                f.info(ui, &t(crate::house::i18n::Key::SetZkState), card::Show::Mark(m, &crate::house::i18n::library(lang, &fv.say)));
                            }
                            for s in [&fv.redraw, &fv.sweep].into_iter().flatten() {
                                one(f, ui, &said_line(&s.name, &crate::house::refusal::line(lang, &s.name, &s.detail)));
                                raw.push(raw_of(&s.name, &s.detail));
                            }
                        }
                        Some(Err(s)) => {
                            one(f, ui, &said_line(&s.name, &crate::house::refusal::line(lang, &s.name, &s.detail)));
                            raw.push(raw_of(&s.name, &s.detail));
                        }
                        None => {}
                    }
                    if !raw.is_empty() {
                        let head = t(crate::house::i18n::Key::ToastDetail);
                        let rows: Vec<(&str, kv::Val)> = raw.iter().map(|x| (head.as_str(), kv::Val::Mono(x.clone()))).collect();
                        let (detail, widest) = detail_rows_of(ui, &rows);
                        let copied = f.details(ui, "orders-progress-raw", &head, &detail, widest);
                        tell_copied(ui.ctx(), copied);
                    }
                    if let Some(((what, tx), _)) = &pend {
                        let title = if what.is_empty() { t(crate::house::i18n::Key::SendTx) } else { crate::house::i18n::library(lang, what) };
                        let sub = crate::house::i18n::filln(crate::house::i18n::Key::OrdersNoteSentWaiting, &[&eravon_orders::table::short(tx)]);
                        let _ = f.line(ui, Some(card::Lead::Mark(Mark::Busy)), &title, &sub, false, false, c(C::Ink), |ui| {
                            if key::show(ui, key::Key::new(&crate::house::i18n::get_named(lang, "orders.bumpKey").0, Role::Secondary).enabled(can).why(&why)).clicked() {
                                ask = Some(crate::house::action::Action::PendingBump);
                            }
                            if key::show(ui, key::Key::new(&t(crate::house::i18n::Key::OrdersVoidKey), Role::Secondary).enabled(can).why(&why)).clicked() {
                                ask = Some(crate::house::action::Action::PendingVoid);
                            }
                        });
                    }
                    if paste {
                        f.field(ui, &t(crate::house::i18n::Key::OrdersSigHint), None, None, |ui, w| {
                            let live = can && (!sig.trim().is_empty() || eravon_dispute::settle::takes_empty(eravon_dispute::settle::Paste::Confirm));
                            let _ = w;
                            sig_go = width::line_then(ui, &mut sig, &t(crate::house::i18n::Key::OrdersSigHint), false, |ui| key::key(ui, &t(crate::house::i18n::Key::OrdersSigConfirm), Role::Commit, live).clicked()).1;
                        });
                    }
                });
            });
            if let Some(x) = self.shell.desk.sh.seller.orders.as_mut() {
                if x.sig_confirm != sig {
                    x.sig_confirm = sig.clone();
                }
                if ask.is_some() {
                    x.pending_ask = ask;
                }
            }
            if sig_go {
                let v = sig.clone();
                self.step(Step::SigConfirm, &[("v", &v)]);
            }
            if let Some(tx) = view_tx {
                let at = self.shell.desk.sh.seller.home.as_ref().and_then(|h| h.activity.as_ref().ok()).and_then(|v| v.iter().find(|a| a.tx.eq_ignore_ascii_case(&tx))).map_or(0, |a| a.at);
                let number = crate::house::orders::row_label(r);
                self.shell.desk.act_open = Some((format!("{number} {}", t(crate::house::i18n::Key::OrdersDrew)), at, tx));
            }
        }
        self.activity_sheet(ui.ctx());
        self.pending_sheet(ui.ctx(), os, can);
        let unopened = !matches!(o.state, eravon_watch::order::State::Deployed | eravon_watch::order::State::Expired | eravon_watch::order::State::Revoked | eravon_watch::order::State::Cancelled);
        let mut go: Option<Step> = None;
        card::section(ui, &t(crate::house::i18n::Key::OrdersOps), "", |ui| {
            card::form(ui, |ui, f| {
                if os.co.is_some() && unopened && f.go(ui, &t(crate::house::i18n::Key::OrdersCompanyOpenKey), "", can, &why).clicked() {
                    go = Some(Step::OrderCompanyOpen);
                }
                if ready && main != Main::Open && f.go(ui, &t(crate::house::i18n::Key::OrdersOpenKey), "", can, &why).clicked() {
                    go = Some(Step::OrderDeploy);
                }
                if d.advance.as_ref().is_some_and(|a| a.as_ref().is_ok_and(|x| x.ran_out())) && f.go(ui, &t(crate::house::i18n::Key::BoActAdvance), "", can, &why).clicked() {
                    go = Some(Step::OrderAdvance);
                }
                if done && main != Main::Export && f.go(ui, &t(crate::house::i18n::Key::IdExportGroup), "", true, "").clicked() {
                    go = Some(Step::OrderExport);
                }
                if has(eravon_orders::status::Act::Deliver) && main != Main::Deliver && f.go(ui, &t(crate::house::i18n::Key::OrdersDeliverKey), "", can, &why).clicked() {
                    go = Some(Step::DeliverOpen);
                }
                if crate::house::complaint::seated(r).is_some() && f.go(ui, &t(crate::house::i18n::Key::OrdersComplain), "", true, "").clicked() {
                    go = Some(Step::ComplaintOpen);
                }
            });
        });
        match go {
            Some(Step::ComplaintOpen) => self.complaint_start(),
            Some(s) => self.step(s, &[]),
            None => {}
        }
        if let (Some(Ok(p)), false) = (&os.exported, os.originals.is_empty()) {
            details_card(ui, "order-proposal", &[(&t(crate::house::i18n::Key::GoodsProposalAt), kv::Val::Mono(p.clone())), (&t(crate::house::i18n::Key::OrdersOriginals), kv::Val::Mono(os.originals.join("\n")))]);
        }
        if !os.detail.as_ref().is_some_and(|d| d.dispute.is_some()) {
            self.entries_area(ui, os);
        }
        self.overview_tail(ui, os, r);
    }

    fn label_sheet(&mut self, ctx: &egui::Context, r: &eravon_orders::table::Row) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(mut label) = self.shell.desk.order_label.clone() else { return };
        let (_, save, back) = Self::sheet_with(ctx, "order-label", eravon_ui::metrics::MODAL_W, |ui, keys| {
            eravon_ui::sheet::title(ui, &t(crate::house::i18n::Key::CandsRename), "");
            eravon_ui::fields::stack(ui, |ui, st| {
                st.field(ui, &t(crate::house::i18n::Key::OrdersLabelHint), None, None, |ui, w| input::field(ui, &mut label, "", w, input::Look::default()));
            });
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::ReadNetSave), Role::Primary, true)));
        });
        if save {
            let (b, v) = (r.buyer.clone(), label.clone());
            self.shell.desk.order_label = None;
            self.step(Step::OrderLabel, &[("buyer", &b), ("v", &v)]);
        } else if back {
            self.shell.desk.order_label = None;
        } else {
            self.shell.desk.order_label = Some(label);
        }
    }

    fn pending_sheet(&mut self, ctx: &egui::Context, os: &crate::house::orders::Orders, can: bool) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let (Some(step), Some(o)) = (os.pending_ask, os.order.as_ref()) else { return };
        let (Some((_, tx)), Some(at)) = (o.pending(), o.pending_at()) else { return };
        let (title, go) = if step == crate::house::action::Action::PendingBump {
            (crate::house::i18n::Key::OrdersBumpTitle, crate::house::i18n::Key::U3BumpKey)
        } else {
            (crate::house::i18n::Key::OrdersVoidTitle, crate::house::i18n::Key::OrdersVoidKey)
        };
        let (_, ok, back) = Self::sheet_with(ctx, "orders-pending", eravon_ui::metrics::MODAL_W, |ui, keys| {
            eravon_ui::sheet::title(ui, &crate::house::i18n::get(lang, title).0, "");
            let copied = card::form(ui, |ui, f| {
                let c = f.info(ui, &t(crate::house::i18n::Key::SendTx), card::Show::Short(&tx));
                f.info(ui, "nonce", card::Show::Mono(&at.nonce.to_string()));
                c
            });
            tell_copied(ui.ctx(), copied);
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&crate::house::i18n::get(lang, go).0, Role::Primary, can)));
        });
        if ok || back {
            if let Some(x) = self.shell.desk.sh.seller.orders.as_mut() {
                x.pending_ask = None;
            }
        }
        if ok {
            self.step(step, &[]);
        }
    }

    pub(super) fn overview_tail(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders, r: &eravon_orders::table::Row) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let keys = [t(crate::house::i18n::Key::OrdersColNo), t(crate::house::i18n::Key::OrdersAmount), t(crate::house::i18n::Key::OrdersKvCase), t(crate::house::i18n::Key::SetContractAddr), t(crate::house::i18n::Key::OrdersKvBuyerAddr), t(crate::house::i18n::Key::OrdersKvJudgeAddr), t(crate::house::i18n::Key::PlToken), t(crate::house::i18n::Key::OrdersKvStateRaw)];
        let mut rows = vec![
            (keys[0].as_str(), kv::Val::Figure(if r.number.is_empty() { "·".into() } else { r.number.clone() })),
            (keys[1].as_str(), kv::Val::Text(crate::house::coin::amount(crate::lang::lang(), &r.amount, r.decimals, &r.token, crate::house::coin::Form::Full))),
            (keys[2].as_str(), kv::Val::Mono(r.case_id.clone())),
            (keys[3].as_str(), kv::Val::Mono(r.instance.clone())),
            (keys[4].as_str(), kv::Val::Mono(r.buyer.clone())),
            (keys[5].as_str(), kv::Val::Mono(os.detail.as_ref().map(|d| d.overview.judge.addr.clone()).unwrap_or_default())),
            (keys[6].as_str(), kv::Val::Mono(r.token.clone())),
        ];
        if let Some(x) = state_raw(r) {
            rows.push((keys[7].as_str(), kv::Val::Figure(x)));
        }
        let refund = matches!(&os.face, Some(Ok(f)) if f.refundable);
        let mut hit = false;
        let copied = card::form(ui, |ui, f| {
            let (detail, widest) = detail_rows_of(ui, &rows);
            let c = f.details(ui, "order-details", &t(crate::house::i18n::Key::SetEvidence), &detail, widest);
            c
        });
        tell_copied(ui.ctx(), copied);
        if refund && past_locked(r.past) {
            card::form(ui, |ui, f| {
                let _ = f.go(ui, &t(crate::house::i18n::Key::OrdersRefundKey), "", false, &past_why(true, lang));
            });
        } else if refund {
            card::form(ui, |ui, f| {
                hit = f.danger(ui, &t(crate::house::i18n::Key::OrdersRefundKey)).clicked();
            });
        }
        if hit {
            self.step(Step::RefundOpen, &[]);
        }
    }

    pub(super) fn delivery_tab(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders, r: &eravon_orders::table::Row) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(d) = &os.detail else { return };
        let mut open_file: Option<Vec<(String, String, bool)>> = None;
        let dl = &d.delivery;
        let mut save = false;
        let mut deliver = false;
        if dl.done {
            match &dl.hand_over {
                Some(Ok(h)) if h.ready => states::banner(ui, states::Banner::Ok, &t(crate::house::i18n::Key::OrdersHandReady), |_| ()),
                Some(Ok(_)) => states::banner(ui, states::Banner::Note, &t(crate::house::i18n::Key::OrdersHandWait), |_| ()),
                Some(Err((n, d))) => said_banner(ui, "orders-said-2", lang, n, d),
                None => {}
            }
            card::section(ui, &t(crate::house::i18n::Key::OrdersDeliverKey), "", |ui| {
                let copied = card::form(ui, |ui, f| {
                    let at = if dl.at == 0 { String::new() } else { eravon_orders::when::minute(dl.at) };
                    f.info(ui, &t(crate::house::i18n::Key::OrdersKvDelivered), card::Show::Mono(&at));
                    if !dl.note.is_empty() {
                        f.info(ui, &t(crate::house::i18n::Key::BoNote), card::Show::Text(&dl.note));
                    }
                    let mut rows: Vec<(&str, kv::Val)> = Vec::new();
                    let tx_label = t(crate::house::i18n::Key::SendTx);
                    let block_label = t(crate::house::i18n::Key::OrdersClaimBlock);
                    if !dl.tx.is_empty() {
                        rows.push((&tx_label, kv::Val::Mono(dl.tx.clone())));
                    }
                    if let Some(Ok(h)) = &dl.hand_over {
                        rows.push((&block_label, kv::Val::Figure(thousands(h.claim_block))));
                    }
                    if rows.is_empty() {
                        return None;
                    }
                    let (detail, widest) = detail_rows_of(ui, &rows);
                    f.details(ui, "order-delivery-raw", &t(crate::house::i18n::Key::SetEvidence), &detail, widest)
                });
                tell_copied(ui.ctx(), copied);
            });
            let heads = [t(crate::house::i18n::Key::OrdersFileSheet), t(crate::house::i18n::Key::OrdersColBytes)];
            let cols = [table::col(&heads[0], eravon_ui::house::REST), table::col_r(&heads[1], table::Col::Px(eravon_ui::house::COL_AMOUNT)), table::CHEV];
            let generation = self.shell.desk.rows_gen;
            let none = matches!(&dl.files, Ok(f) if f.is_empty());
            if !none {
                card::section(ui, &t(crate::house::i18n::Key::OrdersFileSheet), "", |ui| match &dl.files {
                    Ok(files) => {
                        let make = |i: usize| {
                            let (n, b, _) = &files[i];
                            table::Row { cells: vec![table::Cell::Text(eravon_ui::width::file_name(n)), table::Cell::Mono(size_say(*b as u64)), table::Cell::Chev], click: true, gone: false, on: false }
                        };
                        if let Some(i) = table::table_with(ui, "order-files", table::Spec::new(&cols, generation).head(true), files.len(), make).clicked {
                            let (n, b, h) = &files[i];
                            open_file = Some(vec![(t(crate::house::i18n::Key::OrdersFileSheet), n.clone(), false), (t(crate::house::i18n::Key::OrdersColBytes), b.to_string(), true), (t(crate::house::i18n::Key::OrdersColSha), h.clone(), true)]);
                        }
                    }
                    Err((n, d)) => said_banner(ui, "orders-said-3", lang, n, d),
                });
            }
            if matches!(&dl.files, Ok(f) if !f.is_empty()) {
                card::form(ui, |ui, f| {
                    save = f.act(ui, &t(crate::house::i18n::Key::OrdersSaveOriginals), "", key::Key::new(&t(crate::house::i18n::Key::OrdersSaveKey), Role::Secondary).enabled(os.order.is_some())).clicked();
                });
            }
            if let Some(p) = &os.saved {
                for x in p {
                    states::okline(ui, Mark::Ok, &eravon_ui::width::file_name(x));
                }
            }
        } else {
            card::section(ui, &t(crate::house::i18n::Key::OrdersDeliverKey), "", |ui| {
                card::form(ui, |ui, f| {
                    f.info(ui, &t(crate::house::i18n::Key::SetZkState), card::Show::Text(&t(crate::house::i18n::Key::BoNotDelivered)));
                    f.info(ui, &t(crate::house::i18n::Key::OrdersColDeadline), card::Show::Mono(&day_or(dl.deadline)));
                    if crate::house::orders::acts(r).contains(&eravon_orders::status::Act::Deliver) {
                        deliver = f.act(ui, &t(crate::house::i18n::Key::OrdersDeliverKey), "", key::Key::new(&t(crate::house::i18n::Key::OrdersDeliverKey), Role::Guide).enabled(!r.past).why(&past_why(r.past, lang))).clicked();
                    }
                });
            });
        }
        if deliver {
            self.step(Step::DeliverOpen, &[]);
        }
        if save {
            let id = os.sel.clone().unwrap_or_default();
            match crate::house::seam::downloads(&self.shell.desk.sh.seller.field.home, &self.shell.desk.data) {
                Ok(base) => {
                    let dir = format!("{base}/{}", id.trim_start_matches("0x"));
                    self.step(Step::DeliverSave, &[("dir", &dir)]);
                }
                Err(s) => self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: s.detail.clone(), next: crate::house::sync::next_of(&s), detail: Some(s.name.clone()) }),
            }
        }
        if os.sealable.is_some() || os.sealed.is_some() {
            self.seal_card(ui, os.judge.as_ref(), os.sealable.is_some(), os.sealed.as_ref(), Step::EvidenceJudge, Step::EvidenceSeal);
        }
        if let Some(td) = &os.todo {
            let mut copied_item: Option<&'static str> = None;
            card::section(ui, &crate::house::i18n::library(lang, eravon_deliver::todo::TODO_SAY), "", |ui| {
                card::form(ui, |ui, f| {
                    for (key, label, val) in [
                        ("grantee", t(crate::house::i18n::Key::OrdersGrantee), td.grantee.clone()),
                        ("work", t(crate::house::i18n::Key::OrdersWork), td.work.clone()),
                        ("terms", t(crate::house::i18n::Key::OrdersTerms), td.terms_hash.clone()),
                    ] {
                        if f.info(ui, &label, card::Show::Short(&val)).is_some() {
                            copied_item = Some(key);
                        }
                    }
                });
            });
            if let Some(key) = copied_item {
                let val = match key {
                    "grantee" => td.grantee.clone(),
                    "work" => td.work.clone(),
                    _ => td.terms_hash.clone(),
                };
                if let Some(x) = self.shell.desk.sh.seller.orders.as_mut() {
                    x.copied = Some(val.clone());
                }
                tell_copied(ui.ctx(), Some(val));
                self.shell.desk.tr.push(crate::house::action::Action::TodoCopy, eravon_deliver::mark::FEATURE, key.into());
                self.shell.desk.flush();
            }
        }
        if let Some(rows) = open_file {
            self.shell.desk.info = Some(("orders.fileSheet", rows));
            self.shell.desk.tr.push(crate::house::action::Action::OrderInfo, eravon_orders::mark::FEATURE, "文件".into());
            self.shell.desk.flush();
        }
    }

    pub(super) fn dispute_tab(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders) {
        let lang = crate::lang::lang();
        let past = past_locked(os.open_past());
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(x) = os.detail.as_ref().and_then(|d| d.dispute.clone()) else { return };
        let mut open_own: Option<u64> = None;
        let co = os.company;
        let can = !past && self.write_banner(ui, if co { crate::house::shell::Write::Local } else { crate::house::shell::Write::Chain });
        let why = past_why(past, lang);
        card::section(ui, &t(crate::house::i18n::Key::OrdersClaimSection), "", |ui| {
            card::form(ui, |ui, f| {
                f.info(ui, &t(crate::house::i18n::Key::OrdersKvDisputed), card::Show::Mono(&eravon_orders::when::minute(x.at)));
                f.info(ui, &t(crate::house::i18n::Key::BoRuleBy), card::Show::Mono(&day_or(x.deadline)));
                f.info(ui, &t(crate::house::i18n::Key::OrdersBuyerLf), card::Show::Text(&crate::house::orders::money(lang, os.order.as_ref(), &x.lf)));
            });
        });
        card::section(ui, &t(crate::house::i18n::Key::OrdersEvidence), "", |ui| {
            if x.evidence.is_empty() {
                card::form(ui, |ui, f| f.info(ui, &t(crate::house::i18n::Key::OrdersEvidence), card::Show::Empty(&t(crate::house::i18n::Key::OrdersNoEvidence))));
                return;
            }
            let copied = card::form(ui, |ui, f| {
                let mut own = 0u64;
                let mut copied = None;
                let (ml, tl, fl) = (t(crate::house::i18n::Key::OrdersManifest), t(crate::house::i18n::Key::SendTx), t(crate::house::i18n::Key::WlFrom));
                for (k, e) in x.evidence.iter().enumerate() {
                    let who = if e.ours { t(crate::house::i18n::Key::OrdersBySelf) } else { t(crate::house::i18n::Key::OrdersByOther) };
                    let title = format!("{who} · {}", eravon_orders::when::minute(e.at));
                    let sub = match &e.files {
                        Ok(f) => crate::house::i18n::filln(crate::house::i18n::Key::BoFilesN, &[&f.len().to_string()]),
                        Err((n, d)) => crate::house::refusal::line(lang, n, d),
                    };
                    if e.ours {
                        own += 1;
                        if f.act(ui, &title, &sub, key::Key::new(&t(crate::house::i18n::Key::OrdersOpenOwn), Role::Secondary).enabled(!e.source.is_empty())).clicked() {
                            open_own = Some(own);
                        }
                    } else {
                        let _ = f.line(ui, None, &title, &sub, false, false, c(C::Ink), |_| ());
                    }
                    let rows = [(ml.as_str(), kv::Val::Mono(e.manifest.clone())), (tl.as_str(), kv::Val::Mono(e.tx.clone())), (fl.as_str(), kv::Val::Mono(e.by.clone()))];
                    let (detail, widest) = detail_rows_of(ui, &rows);
                    copied = f.details(ui, &format!("order-ev-raw-{k}"), &t(crate::house::i18n::Key::SetEvidence), &detail, widest).or(copied);
                }
                copied
            });
            tell_copied(ui.ctx(), copied);
        });
        self.seal_card(ui, os.judge.as_ref(), os.sealable.is_some(), os.sealed.as_ref(), Step::EvidenceJudge, Step::EvidenceSeal);
        let live = matches!(&os.face, Some(Ok(f)) if f.word != "closed");
        let refundable = matches!(&os.face, Some(Ok(f)) if f.refundable);
        let mut go: Option<Step> = None;
        if live || refundable {
            card::section(ui, &t(crate::house::i18n::Key::OrdersOps), "", |ui| {
                card::form(ui, |ui, f| {
                    if live && f.go(ui, &t(crate::house::i18n::Key::OrdersEvidenceKey), "", os.order.is_some() && can, &why).clicked() {
                        go = Some(Step::EvidenceOpen);
                    }
                    if live && f.go(ui, &t(crate::house::i18n::Key::OrdersSettleKey), "", os.order.is_some() && can, &why).clicked() {
                        go = Some(Step::SettleOpen);
                    }
                    if refundable && f.go(ui, &t(crate::house::i18n::Key::OrdersRefundKey), "", can, &why).clicked() {
                        go = Some(Step::RefundOpen);
                    }
                });
            });
        }
        if let Some(s) = go {
            self.step(s, &[]);
        }
        if let Some((bps, expiry)) = os.offer.filter(|_| live) {
            let mut sig = os.sig_settle.clone();
            let mut sig_go = false;
            card::section(ui, &t(crate::house::i18n::Key::OrdersProposal), "", |ui| {
                card::form(ui, |ui, f| {
                    f.info(ui, &t(crate::house::i18n::Key::PlToBuyer), card::Show::Text(&pct_of(bps)));
                    f.info(ui, &t(crate::house::i18n::Key::BoExpiry), card::Show::Mono(&day_or(expiry)));
                    f.field(ui, &t(crate::house::i18n::Key::OrdersSigHint), None, None, |ui, _w| {
                        let ready = can && (!sig.trim().is_empty() || eravon_dispute::settle::takes_empty(eravon_dispute::settle::Paste::Settle { buyer_bps: bps, expiry }));
                        let (_typed, go) = width::line_then(ui, &mut sig, &t(crate::house::i18n::Key::OrdersSigHint), false, |ui| key::key(ui, &t(crate::house::i18n::Key::OrdersSigSettle), Role::Commit, ready).clicked());
                        sig_go = go;
                    });
                });
            });
            if let Some(x) = self.shell.desk.sh.seller.orders.as_mut() {
                if x.sig_settle != sig {
                    x.sig_settle = sig.clone();
                }
            }
            if sig_go {
                let v = sig.clone();
                self.step(Step::SigSettle, &[("v", &v)]);
            }
        }
        if let Some(n) = open_own {
            let url = self.shell.desk.sh.seller.orders.as_mut().and_then(|o| o.open_own(n));
            if let Some(u) = &url {
                match crate::house::platform::open_url(u) {
                    Ok(()) => self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Ok, head: t(crate::house::i18n::Key::OrdersOwnOpened), next: String::new(), detail: None }),
                    Err(e) => self.shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: e, next: String::new(), detail: None }),
                }
            }
            self.shell.desk.tr.push(crate::house::action::Action::EvidenceOwn, eravon_dispute::mark::FEATURE, url.unwrap_or_default());
            self.shell.desk.flush();
        }
        self.entries_area(ui, os);
    }

    pub(super) fn entries_area(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        match &os.entries {
            Some(Ok(es)) if !es.is_empty() => {
                card::section(ui, &t(crate::house::i18n::Key::OrdersEntries), "", |ui| {
                    for (k, e) in es.iter().enumerate() {
                        let (m, word) = match &e.red {
                            Some(r) => (Mark::Bad, r.clone()),
                            None => (Mark::Ok, t(if e.kind.contains("resign") { crate::house::i18n::Key::OrdersEntryResign } else { crate::house::i18n::Key::OrdersEntryRuling })),
                        };
                        card::form(ui, |ui, f| f.info(ui, &word, card::Show::Mark(m, "")));
                        super::casepages::text_box(ui, &format!("order-entry-{k}"), &e.text, eravon_ui::house::TEXT_BOX_MAX_H);
                        details_card(ui, &format!("order-entry-raw-{k}"), &[(&t(crate::house::i18n::Key::OrdersEntryWord), kv::Val::Mono(e.word.clone())), (&t(crate::house::i18n::Key::OrdersEntryKind), kv::Val::Figure(e.kind.clone())), (&t(crate::house::i18n::Key::WlFrom), kv::Val::Mono(e.signer.clone()))]);
                    }
                });
            }
            Some(Err(s)) => said_banner(ui, "orders-said-1", lang, &s.name, &s.detail),
            _ => {}
        }
    }

    pub(super) fn record_tab(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(d) = &os.detail else { return };
        let mut open_tx: Option<Vec<(String, String, bool)>> = None;
        match &d.record {
            Ok(rows) => {
                let heads = [t(crate::house::i18n::Key::WlColTime), t(crate::house::i18n::Key::BoEvent)];
                let cols = [table::col(&heads[0], table::Col::Px(eravon_ui::house::COL_WHEN)), table::col(&heads[1], eravon_ui::house::REST), table::CHEV];
                let at_of = |at: u64| if at == 0 { String::new() } else { eravon_orders::when::minute(at) };
                let make = |i: usize| {
                    let m = &rows[i];
                    table::Row { cells: vec![table::Cell::Mono(at_of(m.at)), table::Cell::Text(crate::house::i18n::library(lang, &m.say)), table::Cell::Chev], click: true, gone: false, on: false }
                };
                let none = t(crate::house::i18n::Key::HomeNone);
                if let Some(i) = table::table_with(ui, "order-record", table::Spec::new(&cols, self.shell.desk.rows_gen).head(true).empty(&none), rows.len(), make).clicked {
                    let m = &rows[i];
                    open_tx = Some(vec![(t(crate::house::i18n::Key::WlColTime), at_of(m.at), true), (t(crate::house::i18n::Key::BoEvent), crate::house::i18n::library(lang, &m.say), false), (t(crate::house::i18n::Key::SendTx), m.tx.clone(), true)]);
                }
            }
            Err(s) => states::err_box(ui, "orders-record", &t(crate::house::i18n::Key::BoListFailed), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
        }
        if let Some(n) = os.order.as_ref().and_then(|o| o.extra("sweepNote")) {
            states::banner(ui, states::Banner::Note, &crate::house::i18n::library(lang, n), |_| ());
        }
        if let Some(rows) = open_tx {
            self.shell.desk.info = Some(("orders.txSheet", rows));
            self.shell.desk.tr.push(crate::house::action::Action::OrderInfo, eravon_orders::mark::FEATURE, "交易".into());
            self.shell.desk.flush();
        }
    }

    pub(super) fn fee_tab(&mut self, ui: &mut egui::Ui, os: &crate::house::orders::Orders, r: &eravon_orders::table::Row) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(d) = &os.detail else { return };
        let f = &d.fee;
        let word = |x: &str| crate::house::coin::amount(lang, x, r.decimals, &r.token, crate::house::coin::Form::Full);
        card::section(ui, &t(crate::house::i18n::Key::BoTabFee), "", |ui| {
            card::form(ui, |ui, fm| {
                fm.info(ui, &t(crate::house::i18n::Key::ExPrice), card::Show::Text(&word(&f.amount)));
                let judge_sub = format!("{} · {}", t(crate::house::i18n::Key::OrdersFeeFromDeposit), t(crate::house::i18n::Key::OrdersFeeJudgeNote));
                let lf_sub = format!("{} · {}", t(crate::house::i18n::Key::OrdersFeeInDeposit), t(crate::house::i18n::Key::OrdersFeeLfNote));
                let _ = fm.line(ui, None, &t(crate::house::i18n::Key::CandsQPremium), &judge_sub, false, false, c(C::Ink), |ui| {
                    ui.label(egui::RichText::new(word(&f.judge_fee)).font(Type::Note.font()).color(c(C::Ink2)));
                });
                let _ = fm.line(ui, None, &t(crate::house::i18n::Key::OrdersKvLf), &lf_sub, false, false, c(C::Ink), |ui| {
                    ui.label(egui::RichText::new(crate::house::coin::pair(lang, &f.lf.1, &f.lf.0, r.decimals, &r.token, crate::house::coin::Form::Full)).font(Type::Note.font()).color(c(C::Ink2)));
                });
                let _ = fm.line(ui, None, &t(crate::house::i18n::Key::OrdersKvBond), &t(crate::house::i18n::Key::OrdersFeeDepositIs), false, false, c(C::Ink), |ui| {
                    ui.label(egui::RichText::new(word(&f.deposit)).font(Type::Note.font()).color(c(C::Ink2)));
                });
            });
        });
        let later = t(crate::house::i18n::Key::OrdersSplitLater);
        card::section(ui, &t(crate::house::i18n::Key::OrdersEvClosed), "", |ui| {
            card::form(ui, |ui, fm| {
                let rows = [
                    (t(crate::house::i18n::Key::OrdersSplitBuyer), f.split.as_ref().map(|s| word(&s.0))),
                    (t(crate::house::i18n::Key::OrdersSplitSeller), f.split.as_ref().map(|s| word(&s.1))),
                    (t(crate::house::i18n::Key::OrdersSplitJudge), f.split.as_ref().map(|s| word(&s.2))),
                    (t(crate::house::i18n::Key::OrdersSplitBond), f.split.as_ref().map(|s| word(&s.3))),
                ];
                for (k, v) in rows {
                    match v {
                        Some(v) => fm.info(ui, &k, card::Show::Text(&v)),
                        None => fm.info(ui, &k, card::Show::Empty(&later)),
                    };
                }
            });
        });
    }

    pub(super) fn order_route(&self) -> (bool, Option<crate::house::seller::Said>) {
        let o = self.shell.desk.sh.seller.orders.as_ref();
        (o.is_some_and(|o| o.co.is_some()), o.and_then(|o| o.co_err.clone()))
    }
}

pub(super) fn state_cell(r: &eravon_orders::table::Row, lang: crate::lang::Lang) -> eravon_ui::table::Cell {
    match past_pill(r, lang) {
        Some(p) => eravon_ui::table::Cell::Pill(p, eravon_ui::palette::Tone::Grey),
        None => eravon_ui::table::Cell::Pill(state_say(r, lang), state_tone(r)),
    }
}

pub(super) fn past_pill(r: &eravon_orders::table::Row, lang: crate::lang::Lang) -> Option<String> {
    r.past.then(|| crate::house::i18n::get(lang, crate::house::i18n::Key::OrdersPastPill).0)
}

pub(super) fn past_locked(past: bool) -> bool {
    past && !eravon_literals::constants::past_switch()
}

pub(super) fn lacks_why(past: bool, lang: crate::lang::Lang) -> String {
    if past { crate::house::i18n::get(lang, crate::house::i18n::Key::ErrEP6PASTLACKS).0 } else { String::new() }
}

pub(super) fn past_why(past: bool, lang: crate::lang::Lang) -> String {
    if past { crate::house::i18n::get_named(lang, "err.E_A1_PAST_CONTRACT").0 } else { String::new() }
}

#[cfg(test)]
mod past_tests {
    use super::*;

    fn row(past: bool) -> eravon_orders::table::Row {
        eravon_orders::table::Row {
            instance: String::new(),
            case_id: String::new(),
            record: String::new(),
            number: String::new(),
            buyer: String::new(),
            label: String::new(),
            product: String::new(),
            amount: "0".into(),
            token: String::new(),
            status: Ok(eravon_orders::status::Status { word: eravon_orders::status::Word::AwaitConfirm, reason: String::new() }),
            deadline: 0,
            placed: 0,
            block: None,
            chain: None,
            local: None,
            handed: false,
            note: String::new(),
            terms_hash: String::new(),
            decimals: None,
            past,
            logic: String::new(),
            series: None,
        }
    }

    #[test]
    fn a_case_on_an_older_contract_is_marked_read_only_and_its_keys_say_why() {
        for lang in [crate::lang::Lang::Zh, crate::lang::Lang::En] {
            let pill = crate::house::i18n::get(lang, crate::house::i18n::Key::OrdersPastPill).0;
            assert_eq!(past_pill(&row(true), lang).as_deref(), Some(pill.as_str()));
            assert!(past_pill(&row(false), lang).is_none());
            match state_cell(&row(true), lang) {
                eravon_ui::table::Cell::Pill(s, eravon_ui::palette::Tone::Grey) => assert_eq!(s, pill),
                other => panic!("{other:?}"),
            }
            assert!(matches!(state_cell(&row(false), lang), eravon_ui::table::Cell::Pill(s, _) if s == state_say(&row(false), lang)));
            assert!(past_why(false, lang).is_empty());
        }
        for lang in [crate::lang::Lang::Zh, crate::lang::Lang::En] {
            let switch = crate::house::i18n::get(lang, crate::house::i18n::Key::SetPastCases).0;
            assert!(past_why(true, lang).contains(switch.as_str()), "{lang:?}: {} names {switch}", past_why(true, lang));
        }
        assert!(crate::house::orders::acts(&row(true)).is_empty());
        assert_eq!(crate::house::orders::shown_acts(&row(true)), crate::house::orders::acts(&row(false)));
        assert_eq!(crate::house::i18n::get(crate::lang::Lang::Zh, crate::house::i18n::Key::OrdersPastPill).0, "往代合约");
    }
}

#[cfg(test)]
mod why_tests {
    use super::*;
    use crate::lang::Lang;
    use eravon_watch::order::State as L;

    fn row(local: Option<L>, note: &str, stage: Option<u64>) -> eravon_orders::table::Row {
        eravon_orders::table::Row {
            instance: "0x00000000000000000000000000000000000000c1".into(),
            case_id: String::new(),
            record: "r".into(),
            number: String::new(),
            buyer: String::new(),
            label: String::new(),
            product: String::new(),
            amount: "0".into(),
            token: String::new(),
            status: eravon_orders::status::judge_noted(local, None, false, note),
            deadline: 0,
            placed: 0,
            block: None,
            chain: None,
            local,
            handed: false,
            note: note.into(),
            terms_hash: String::new(),
            decimals: None,
            past: false,
            logic: String::new(),
            series: stage.map(|s| eravon_terms::doc::Series { id: "0x01".into(), stage: s, stages: 3 }),
        }
    }

    fn stage_note(ended: bool, bps: u16) -> (L, String) {
        let st = eravon_standard::chain::Stage { seller_key: String::new(), buyer_key: String::new(), ended, buyer_bps: bps, series_judge: String::new(), case_id: [0; 32], past: false, logic: String::new() };
        eravon_watch::poll::stage_waits(&Some(("0x00000000000000000000000000000000000000d1".into(), Ok(st))), false).expect("a later stage waits")
    }

    fn machine(s: &str) -> bool {
        s.contains("0x") || s.contains("E_") || s.contains("基点") || s.contains(':') || s.contains("RateStale")
    }

    #[test]
    fn each_held_state_says_its_own_sentence_and_is_never_green() {
        let (prev, prev_note) = stage_note(false, 0);
        let (full, full_note) = stage_note(true, 10_000);
        let (half, half_note) = stage_note(true, 5_000);
        assert_eq!((prev, full, half), (L::AwaitPrev, L::Blocked, L::Blocked));
        assert_eq!(stage_share(&full_note), Some(10_000));
        assert_eq!(stage_share(&half_note), Some(5_000));
        assert_eq!(stage_share(&prev_note), None);
        let cases = [
            (row(Some(L::PriceStale), "price:RateStale:feed 0x00000000000000000000000000000000000000e1;恢复后再发", None), crate::house::i18n::Key::OrdersWhyPrice),
            (row(Some(L::DepositShort), "卖家押金不足,开不了案:E_ACT_BOND:差 3", None), crate::house::i18n::Key::OrdersWhyDeposit),
            (row(Some(L::PendingConsent), "等待买家同意:unread", None), crate::house::i18n::Key::OrdersWhyConsent),
            (row(Some(L::Ready), eravon_watch::send::OPEN_OFF_NOTE, None), crate::house::i18n::Key::OrdersAwaitOpen),
            (row(Some(L::AwaitPrev), &prev_note, None), crate::house::i18n::Key::OrdersWhyPrevAny),
            (row(Some(L::Blocked), &half_note, None), crate::house::i18n::Key::OrdersWhyStageOff),
        ];
        for lang in [Lang::Zh, Lang::En] {
            for (r, k) in &cases {
                let w = state_why(r, lang).expect("a held state has its sentence");
                assert_eq!(w, crate::house::i18n::get(lang, *k).0, "{:?}", r.local);
                assert!(!machine(&w), "{w}");
                assert_eq!(state_say(r, lang), crate::house::i18n::order_word(lang, eravon_orders::status::Word::AwaitDeploy), "the pill is the word alone");
            }
            let w = state_why(&row(Some(L::AwaitPrev), &prev_note, Some(3)), lang).unwrap();
            assert_eq!(w, crate::house::i18n::filln_in(lang, crate::house::i18n::Key::OrdersWhyPrev, &["2"]));
            let w = state_why(&row(Some(L::Blocked), &full_note, Some(2)), lang).unwrap();
            assert_eq!(w, crate::house::i18n::filln_in(lang, crate::house::i18n::Key::OrdersWhyStageRefunded, &["1"]));
            let w = state_why(&row(Some(L::Blocked), &half_note, Some(2)), lang).unwrap();
            assert_eq!(w, crate::house::i18n::filln_in(lang, crate::house::i18n::Key::OrdersWhyStageShare, &["1", "50%"]));
            assert!(!machine(&w), "{w}");
            let w = state_why(&row(Some(L::Blocked), "开案前核不过:judge:E_ACT_JUDGE_PAUSED:0x00000000000000000000000000000000000000f1 暂停接单", None), lang).unwrap();
            assert_eq!(w, crate::house::refusal::what(lang, "E_ACT_JUDGE_PAUSED").unwrap());
            assert!(state_why(&row(Some(L::Ready), "", None), lang).is_none(), "a plain ready order says its word alone");
        }
        use eravon_ui::palette::Tone;
        assert_eq!(state_tone(&row(Some(L::Blocked), &half_note, None)), Tone::Bad);
        for l in [L::PriceStale, L::DepositShort, L::PendingConsent, L::AwaitPrev] {
            assert_eq!(state_tone(&row(Some(l), "x", None)), Tone::Warn, "{l:?}");
        }
        assert_eq!(state_tone(&row(Some(L::Ready), eravon_watch::send::OPEN_OFF_NOTE, None)), Tone::Blue);
        assert_eq!(state_tone(&row(Some(L::Ready), "", None)), Tone::Ok);
    }

    #[test]
    fn a_draw_is_said_with_its_transaction_and_a_note_never_raw() {
        let src = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-watch/src/send.rs")).expect("the writer's source");
        assert!(src.contains(&format!("format!(\"{DREW}{{tx}}\")")), "the draw's note is written as {DREW}{{tx}}");
        let tx = "0x8c42ed5cb5009606594223c1e123785923ca64a4d238f9201e65c44b4636c966";
        assert_eq!(drew_tx(&format!("{DREW}{tx}")), Some(tx));
        assert_eq!(drew_tx("抽签未发:E_V_DOWN:x"), None);
        let r = row(Some(L::Deployed), "", None);
        for lang in [Lang::Zh, Lang::En] {
            assert_eq!(note_say(&r, "抽签未发:E_POOL_GAS:差 0x00000000000000000000000000000000000000ab", lang), crate::house::refusal::what(lang, "E_POOL_GAS").unwrap());
            let w = note_say(&r, &format!("实例已部署({tx}),下一轮发 init"), Lang::Zh);
            assert!(!w.contains("0x"), "{w}");
        }
    }
}
