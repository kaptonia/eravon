use super::*;
use crate::house::action::Action as Step;

pub(super) enum NoteGo {
    Order(String),
    Goods,
    Account,
}

pub(super) fn note_go(n: &eravon_home::notes::Note) -> Option<(&'static str, NoteGo)> {
    use eravon_home::notes::Kind;
    match n.kind {
        Kind::Backup => Some(("home.goSettings", NoteGo::Account)),
        Kind::Link if n.instance.is_empty() => Some(("home.goGoods", NoteGo::Goods)),
        _ if !n.instance.is_empty() => Some(("home.goOrder", NoteGo::Order(n.instance.clone()))),
        _ => None,
    }
}

pub(super) fn notes_list(ui: &mut egui::Ui, notes: &[eravon_home::notes::Note], marks: &[(String, bool, bool)], item_words: &str) -> (Option<String>, Option<NoteGo>) {
    let lang = crate::lang::lang();
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    let open: Vec<&eravon_home::notes::Note> = notes.iter().filter(|n| !marks.iter().any(|(k, d, _)| *k == n.key && *d)).collect();
    let (mut handled, mut jump) = (None, None);
    if open.is_empty() {
        card::card_pad(ui, egui::vec2(ALL_CLEAR_PAD_X, ALL_CLEAR_PAD_Y), |ui| all_clear(ui, &t(crate::house::i18n::Key::AlertsNone)));
        return (handled, jump);
    }
    card::form(ui, |ui, f| {
        for n in open {
            let target = note_go(n);
            let sub = if n.instance.is_empty() || n.say.contains(&n.subject) { String::new() } else { crate::house::orders::order_label(&n.subject) };
            let hit = f
                .row_with(ui, Some(mark::Mark::Warn), &note_line(lang, n), &sub, if target.is_some() { card::Value::Nav } else { card::Value::None }, |ui| {
                    if key::key(ui, &t(crate::house::i18n::Key::HomeHandle), Role::Plain, true).clicked() {
                        handled = Some(n.key.clone());
                    }
                })
                .clicked();
            if hit {
                jump = target.map(|x| x.1);
            }
            for (k, val) in &n.items {
                f.row_with(ui, None, &crate::house::i18n::get_named(lang, &format!("{item_words}.{k}")).0, &eravon_ui::width::elide_chars(val, eravon_ui::house::RAIL_NOTE_CHARS), card::Value::None, |ui| {
                    if key::key(ui, &t(crate::house::i18n::Key::OrdersCopy), Role::Secondary, !val.is_empty()).clicked() {
                        ui.ctx().copy_text(val.clone());
                        tell_copied(ui.ctx(), Some(val.clone()));
                    }
                });
            }
        }
    });
    (handled, jump)
}

pub(super) fn note_line(lang: crate::lang::Lang, n: &eravon_home::notes::Note) -> String {
    let say = crate::house::i18n::library(lang, &n.say);
    if n.instance.is_empty() || n.subject.is_empty() {
        return say;
    }
    say.replace(&n.subject, &crate::house::orders::order_label(&n.subject))
}

use eravon_ui::house::{ALL_CLEAR_GAP, ALL_CLEAR_MARK as MARK, ALL_CLEAR_PAD_X, ALL_CLEAR_PAD_Y};

fn all_clear(ui: &mut egui::Ui, s: &str) {
    let w = ui.available_width();
    let text = paint::galley(ui, s, Type::Body, c(C::Ink2));
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, MARK + ALL_CLEAR_GAP + text.size().y), egui::Sense::hover());
    mark::paint_mark(ui.ctx(), ui.painter(), egui::pos2(rect.center().x, rect.top() + MARK / 2.0), mark::Mark::Ok, MARK);
    ui.painter().galley(egui::pos2(rect.center().x - text.size().x / 2.0, rect.top() + MARK + ALL_CLEAR_GAP), text, c(C::Ink2));
}

impl Win {
    pub(super) fn home_now(&mut self) -> Option<crate::house::home::Home> {
        let fresh = self.shell.desk.sh.seller.home.as_ref().is_some_and(|h| h.fresh_for(&self.shell.desk.addr));
        if !fresh {
            self.shell.spawn_home();
        }
        self.shell.desk.sh.seller.home.clone().filter(|h| h.fresh_for(&self.shell.desk.addr))
    }

    pub(super) fn home_page(&mut self, ui: &mut egui::Ui) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(h) = self.home_now() else {
            Self::home_skeleton(ui, 4);
            return;
        };
        use crate::house::seller::Page;
        let unsynced = t(crate::house::i18n::Key::SyncNever);
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let fig_of = |name: &str| if super::unread(name) { t(crate::house::i18n::Key::HomeNoFigure) } else { unsynced.clone() };
        let active_unread = h.cards.active.as_ref().err().is_some_and(|e| super::unread(&e.0));
        let income_unread = h.cards.income.as_ref().err().is_some_and(|e| super::unread(&e.0));
        let active = h.cards.active.as_ref().map(|n| n.to_string()).unwrap_or_else(|e| fig_of(&e.0));
        let (whole, cut): (Vec<String>, bool) = match &h.cards.income {
            Ok(v) => (
                v.iter().map(|x| crate::house::coin::amount(lang, &x.total, x.decimals, &x.token, crate::house::coin::Form::Full)).collect(),
                v.iter().any(|x| crate::house::coin::cut(lang, &x.total, x.decimals, &x.token).is_some()),
            ),
            Err(_) => (Vec::new(), false),
        };
        let (income, count): (Vec<(String, String)>, u64) = match &h.cards.income {
            Ok(v) if v.is_empty() => (vec![("0".to_string(), String::new())], 0),
            Ok(v) => {
                let split = match v.as_slice() {
                    [one] => vec![crate::house::coin::parts(lang, &one.total, one.decimals, &one.token, crate::house::coin::Form::Short)],
                    many => many.iter().map(|x| (crate::house::coin::amount(lang, &x.total, x.decimals, &x.token, crate::house::coin::Form::Short), String::new())).collect(),
                };
                (split, v.iter().map(|x| x.count).sum())
            }
            Err(e) => (vec![(fig_of(&e.0), String::new())], 0),
        };
        let goods = h.goods.as_ref().map(|n| n.to_string()).unwrap_or_else(|_| unsynced.clone());
        let cands = h.cands.as_ref().map(|n| n.to_string()).unwrap_or_else(|_| unsynced.clone());
        let mut go: Option<Option<Page>> = None;
        let (mut row, mut cand): (Option<String>, Option<String>) = (None, None);
        let since = self.hx.page_at.1;
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 0)), since.to_bits(), 0, |ui| {
            eravon_ui::grid::tiles(ui, "tiles-1", 4, |ui, i| {
                let hit = match i {
                    0 => card::tile(ui, &card::Tile { title: &t(crate::house::i18n::Key::OrdersActive), figure: &active, mark: None, figure_colour: None, sub: &if active_unread { super::unread_word(syncing) } else { format!("{} {}", eravon_home::notes::orders_to_handle(h.notes.as_deref().unwrap_or_default(), &h.marks), t(crate::house::i18n::Key::HomeTodo)) }, mono_figure: false, clickable: true }).clicked(),
                    1 => {
                        let (title, figure) = super::money_tile(ui, &t(crate::house::i18n::Key::HomeIncome), &income);
                        let r = card::tile(ui, &card::Tile { title: &title, figure: &figure, mark: None, figure_colour: None, sub: &if income_unread { super::unread_word(syncing) } else { crate::house::i18n::filln(crate::house::i18n::Key::HomeCount, &[&count.to_string()]) }, mono_figure: false, clickable: true });
                        if cut || figure.ends_with('…') { eravon_ui::layer::tip(r, whole.join("\n")) } else { r }.clicked()
                    }
                    2 => card::tile(ui, &card::Tile { title: &t(crate::house::i18n::Key::NavGoods), figure: &goods, mark: None, figure_colour: None, sub: &t(crate::house::i18n::Key::HomeOnSale), mono_figure: false, clickable: true }).clicked(),
                    _ => card::tile(ui, &card::Tile { title: &t(crate::house::i18n::Key::NavCands), figure: &cands, mark: None, figure_colour: None, sub: &t(crate::house::i18n::Key::HomeChoosable), mono_figure: false, clickable: true }).clicked(),
                };
                if hit {
                    go = Some(Some(if i == 2 { Page::Goods } else if i == 3 { Page::Cands } else { Page::Orders }));
                }
            });
        });
        ui.add_space(eravon_ui::metrics::CARD_GAP - ui.spacing().item_spacing.y);
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 1)), since.to_bits(), 1, |ui| {
            card::two_cols(ui, eravon_ui::tokens::SIDE_W, eravon_ui::tokens::MAIN_MIN_W, |ui, side| {
                if side == card::Side::Main {
                    let heads = [t(crate::house::i18n::Key::OrdersColNo), t(crate::house::i18n::Key::IdSeatGrantee), t(crate::house::i18n::Key::PlTitle), t(crate::house::i18n::Key::OrdersColDeadline), t(crate::house::i18n::Key::SetZkState)];
                    let auto = |min: f32| table::Col::Auto { min, max: f32::MAX };
                    let cols = [
                        table::col(&heads[0], auto(eravon_ui::house::COL_HOME_NO)),
                        table::col(&heads[1], auto(eravon_ui::house::COL_NARROW)),
                        table::col(&heads[2], eravon_ui::house::REST),
                        table::col(&heads[3], auto(eravon_ui::house::COL_HOME_DEADLINE)),
                        table::col(&heads[4], STATE_COL),
                    ];
                    let generation = self.shell.desk.rows_gen;
                    let make = |i: usize| {
                            let r = &h.orders[i];
                            let buyer = if r.label.is_empty() { "—".to_string() } else { r.label.clone() };
                            let cells = vec![
                                table::Cell::Mono(if r.number.is_empty() { "·".into() } else { r.number.clone() }),
                                table::Cell::Text(buyer),
                                table::Cell::Text(if r.product.is_empty() { t(crate::house::i18n::Key::BoTermsMissing) } else { r.product.clone() }),
                                table::Cell::Mono(day_or(r.deadline)),
                                table::Cell::Pill(state_say(r, lang), state_tone(r)),
                            ];
                            table::Row { cells, click: true, gone: false, on: false }
                    };
                    card::section(ui, &t(crate::house::i18n::Key::NavOrders), "", |ui| {
                        let none = t(crate::house::i18n::Key::HomeNone);
                        if let Some(i) = table::table_with(ui, "home-orders", table::Spec::new(&cols, generation).head(true).empty(&none), h.orders.len(), make).clicked {
                            row = Some(h.orders[i].instance.clone());
                        }
                    });
                } else {
                    let lang = crate::lang::lang();
                    let page = self.shell.desk.sh.seller.cands.as_ref();
                    let cols = [table::col("", eravon_ui::house::REST), table::col_r("", table::Col::Auto { min: eravon_ui::house::COL_PILL, max: f32::MAX })];
                    let generation = self.shell.desk.rows_gen;
                    let make = |i: usize| {
                        let (label, kind, a) = &h.cand_rows[i];
                        let pill = super::cands::home_pill(page, &h.me, a, lang);
                        let kind_word = t(if kind == "guild" { crate::house::i18n::Key::CandsGuild } else { crate::house::i18n::Key::PlCands });
                        table::Row { cells: vec![table::Cell::Text(format!("{label} · {kind_word}")), table::Cell::Pill(pill.0, pill.1)], click: true, gone: false, on: false }
                    };
                    card::section(ui, &t(crate::house::i18n::Key::NavCands), "", |ui| {
                        let none = t(crate::house::i18n::Key::HomeNone);
                        if let Some(i) = table::table_with(ui, "home-cands", table::Spec::new(&cols, generation).empty(&none), h.cand_rows.len(), make).clicked {
                            cand = Some(h.cand_rows[i].2.clone());
                        }
                    });
                }
            });
        });
        ui.add_space(eravon_ui::metrics::CARD_GAP - ui.spacing().item_spacing.y);
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 2)), since.to_bits(), 2, |ui| self.activity_card(ui, &h));
        if let Some(i) = row {
            self.step(Step::HomeOrder, &[("i", &i)]);
        } else if let Some(a) = cand {
            self.step(Step::HomeCand, &[("a", &a)]);
        } else if let Some(p) = go {
            self.house_go(p);
        }
    }

    pub(super) fn activity_card(&mut self, ui: &mut egui::Ui, h: &crate::house::home::Home) {
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let cols = [table::col("", table::Col::Px(eravon_ui::house::COL_MARK)), table::col("", eravon_ui::house::REST), table::col_r("", table::Col::Px(eravon_ui::house::COL_TIME))];
        let generation = self.shell.desk.rows_gen;
        card::section(ui, &t(crate::house::i18n::Key::HomeActivity), "", |ui| match &h.activity {
            Ok(v) => {
                let make = |i: usize| {
                    let a = &v[i];
                    match &a.unread {
                        Some((n, d)) => table::Row { cells: vec![table::Cell::Mark(mark::Mark::Bad), table::Cell::Text(format!("{} {}{}", crate::house::orders::order_label(&a.order), t(crate::house::i18n::Key::HomeActUnread), crate::house::refusal::line(lang, n, d))), table::Cell::Empty], ..Default::default() },
                        None => table::Row { cells: vec![table::Cell::Mark(mark::Mark::Ok), table::Cell::Text(format!("{} {}", crate::house::orders::order_label(&a.order), crate::house::i18n::library(lang, &a.say))), table::Cell::Mono(short_at(a.at))], click: !a.tx.is_empty(), gone: false, on: false },
                    }
                };
                let none = t(crate::house::i18n::Key::HomeNone);
                if let Some(i) = table::table_with(ui, "home-activity", table::Spec::new(&cols, generation).empty(&none), v.len(), make).clicked {
                    let a = &v[i];
                    self.shell.desk.act_open = Some((format!("{} {}", crate::house::orders::order_label(&a.order), crate::house::i18n::library(lang, &a.say)), a.at, a.tx.clone()));
                }
            }
            Err(e) if super::unread(&e.name) => super::unread_block(ui, syncing, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
            Err(e) => states::err_box(ui, "home-activity", &t(crate::house::i18n::Key::SyncNever), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail)),
        });
        self.activity_sheet(ui.ctx());
    }

    pub(crate) fn activity_sheet(&mut self, ctx: &egui::Context) {
        let Some((say, at, tx)) = self.shell.desk.act_open.clone() else { return };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let out = eravon_ui::sheet::show(ctx, eravon_ui::sheet::Spec::new("home-act", eravon_ui::metrics::MODAL_W), &mut (), |ui, _| {
            eravon_ui::sheet::title(ui, &t(crate::house::i18n::Key::HomeActivity), "");
            let (event, time) = (t(crate::house::i18n::Key::BoEvent), t(crate::house::i18n::Key::WlColTime));
            let mut rows: Vec<(&str, kv::Val)> = vec![(&event, kv::Val::Text(say.clone()))];
            if at > 0 {
                rows.push((&time, kv::Val::Mono(eravon_orders::when::minute(at))));
            }
            kv::kv(ui, &rows);
            eravon_ui::fold::fold(ui, "home-act-detail", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                kv::kv(ui, &[(&t(crate::house::i18n::Key::WlHash), kv::Val::Mono(tx.clone()))]);
            });
        }, |ui, _| {
            let close = key::key(ui, &t(crate::house::i18n::Key::ToastClose), Role::Primary, true).clicked();
            if key::key(ui, &t(crate::house::i18n::Key::WlCopyHash), Role::Secondary, true).clicked() {
                ui.ctx().copy_text(tx.clone());
                tell_copied(ui.ctx(), Some(tx.clone()));
            }
            close
        });
        if out.foot || out.esc {
            self.shell.desk.act_open = None;
        }
    }

    pub(super) fn home_skeleton(ui: &mut egui::Ui, tiles: usize) {
        if tiles > 0 {
            eravon_ui::grid::tiles(ui, "tiles-2", tiles, |ui, _| {
                card::card(ui, |ui| eravon_ui::states::skeleton_lines(ui, &[eravon_ui::house::SKEL_SHORT, eravon_ui::house::SKEL_FIG, eravon_ui::house::SKEL_SHORT]));
            });
        }
        card::card(ui, |ui| eravon_ui::states::skeleton_lines(ui, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID, eravon_ui::house::SKEL_LONG]));
    }

    pub(super) fn alerts_page(&mut self, ui: &mut egui::Ui) {
        let syncing = self.shell.house_in_flight(crate::task::Kind::Sync);
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(h) = self.home_now() else {
            Self::home_skeleton(ui, 0);
            return;
        };
        let mut handled: Option<String> = None;
        let mut jump: Option<NoteGo> = None;
        let since = self.hx.page_at.1;
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 0)), since.to_bits(), 0, |ui| match &h.notes {
            Ok(v) => {
                let (h_, j_) = notes_list(ui, v, &h.marks, "orders");
                handled = h_;
                jump = j_;
            }
            Err(e) if super::unread(&e.name) => super::unread_block(ui, syncing, &[eravon_ui::house::SKEL_LONG, eravon_ui::house::SKEL_MID]),
            Err(e) => states::err_box(ui, "alerts", &t(crate::house::i18n::Key::SyncNever), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail)),
        });
        eravon_ui::motion::stagger(ui, egui::Id::new(("eravon-stagger", 1)), since.to_bits(), 1, |ui| self.activity_card(ui, &h));
        match jump {
            Some(NoteGo::Order(i)) => self.step(Step::HomeOrder, &[("i", &i)]),
            Some(g) => self.note_jump(g),
            None => {}
        }
        if let Some(key) = handled {
            self.step(Step::NoteHandle, &[("key", &key)]);
        }
    }

    pub(super) fn note_jump(&mut self, g: NoteGo) {
        match g {
            NoteGo::Goods => self.house_go(Some(crate::house::seller::Page::Goods)),
            NoteGo::Account => self.go(Place::Settings(Section::Data), self.ux.now),
            NoteGo::Order(_) => {}
        }
    }
}
