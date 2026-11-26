use super::*;
use eravon_ui::covers;
use eravon_ui::sheetparts as parts;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WizRow {
    Generate,
    Import,
    Restore,
}

impl Win {
    pub(super) fn wizard_layer(&mut self, ctx: &egui::Context, now: f64) {
        use crate::nav::Step;
        let Some(want) = self.ux.wizard else { return };
        let pr = self.progress();
        let role = self.shell.settings.role;
        let step = pr.gate(want);
        self.ux.wizard = Some(step);
        let walk = Step::walk(role);
        let at = walk.iter().position(|s| *s == step).unwrap_or(0);
        let last = at + 1 == walk.len();
        let done = pr.done(step);
        let dir_id = egui::Id::new("eravon-wizard-dir");
        let (was, dir) = ctx.data(|d| d.get_temp::<(usize, f32)>(dir_id)).unwrap_or((at, 0.0));
        let dir = if was == at { dir } else if at > was { 1.0 } else { -1.0 };
        ctx.data_mut(|d| d.insert_temp(dir_id, (at, dir)));
        let (mut move_to, mut close, mut copied) = (None, false, false);
        let (mut open_export, mut exit, mut rail_extra) = (false, false, false);
        let mut row: Option<WizRow> = None;
        let exit_shown = self.wizard_exit_shown();
        let mut act: Option<Action> = None;
        let fresh = self.shell.new_words.as_ref().map(|f| (f.words(), f.picks));
        let busy = self.shell.tasks.in_flight(crate::task::Kind::Vault);
        let sub = if step == Step::Key && fresh.is_some() && self.ux.id_confirming { crate::nav::Sub::Check } else { crate::nav::Sub::First };
        let back = crate::nav::prev(step, sub, role);
        let mut go_back = false;
        let mut lang_to: Option<crate::lang::Lang> = None;
        let mut main_rect = egui::Rect::NOTHING;
        let (_, over) = covers::cover(ctx, "wizard", covers::Ground::Clear, |ui| {
            let screen = ui.max_rect();
            let wid = egui::Id::new(("eravon-wizard-steps-w", ui.ctx().pixels_per_point().to_bits()));
            let steps_w = match ui.ctx().data(|d| d.get_temp::<f32>(wid)) {
                Some(w) => w,
                None => {
                    let names: Vec<&str> = Step::ALL.iter().flat_map(|s| {
                        let (zh, en) = crate::lang::both(s.title_for(role));
                        [zh, en]
                    }).collect();
                    let w = covers::steps_width(ui, &names);
                    ui.ctx().data_mut(|d| d.insert_temp(wid, w));
                    w
                }
            };
            let side_w = eravon_ui::tokens::RAIL_W.max(steps_w + 2.0 * covers::RAIL_PAD_X);
            let side = egui::Rect::from_min_size(screen.min, egui::vec2(side_w, screen.height()));
            main_rect = egui::Rect::from_min_max(egui::pos2(side.right(), screen.top()), screen.max);
            covers::rail_ground(ui.painter(), side, 1.0);
            ui.painter().rect_filled(main_rect, 0.0, c(C::Ground));
            let inner = egui::Rect::from_min_max(egui::pos2(side.left() + covers::RAIL_PAD_X, side.top() + tk::RAIL_TOP), egui::pos2(side.right() - covers::RAIL_PAD_X, side.bottom() - 14.0));
            let mut left = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
            left.spacing_mut().item_spacing.y = 0.0;
            covers::rail_head(&mut left, t(Key::PageFirstRun), t(Key::AppName));
            let rows: Vec<covers::Step> = walk
                .iter()
                .enumerate()
                .map(|(i, s)| covers::Step { label: t(s.title_for(role)), done: pr.done(*s), reachable: i != at && (pr.done(*s) || i < at) })
                .collect();
            if let Some(i) = covers::steps(&mut left, &rows, at) {
                move_to = Some(i);
            }
            left.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                exit = covers::exit_item(ui, t(Key::WizExit), "esc", exit_shown).clicked();
                rail_extra = self.own_rail_extra(ui);
                lang_to = lang_pick(ui, "wiz-lang");
            });
        });
        let dir_now = if was == at { 0.0 } else { dir };
        covers::pane(ctx, over, at as u64, dir_now, main_rect, |ui| {
            let main = ui.max_rect().shrink(32.0);
            full::centered(ui, "wizard-main", main, WIZ_CARD_W, 0.0, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                covers::step_head(ui, &fill2(Key::WizCount, &(at + 1).to_string(), &walk.len().to_string()), t(step.title_for(role)));
                let open = self.ux.id_words_open;
                card::card_pad(ui, egui::vec2(24.0, 24.0), |ui| {
                    parts::bare(ui, |ui| {
                        fields::stack(ui, |ui, st| {
                            ui.spacing_mut().item_spacing.y = 18.0;
                            row = ui
                                .vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 18.0;
                                    self.wizard_body(ui, st, step, done, fresh.as_ref(), busy, now)
                                })
                                .inner;
                            let later = t(if last { Key::WizLaterDo } else { Key::WizLater });
                            let later_on = step.deferrable(role) && !done;
                            let words_shown = step == Step::Key && !done && fresh.is_some() && !self.ux.id_confirming;
                            let has_keys = done || step != Step::Pin || busy;
                            if !has_keys {
                                return;
                            }
                            ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), tk::KEY_H), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = tk::S2;
                                if done {
                                    if page::Page::new().primary(ui, t(if last { Key::WizFinish } else { Key::WizardNext })).1.clicked() {
                                        if let Step::Own(o) = step {
                                            if let Some(a) = self.own_wiz_next(o) {
                                                act = Some(a);
                                            }
                                        }
                                        if last {
                                            close = true;
                                        } else {
                                            move_to = Some(at + 1);
                                        }
                                    }
                                } else {
                                    match step {
                                        Step::Pin => {}
                                        Step::Key => match (fresh.as_ref(), self.ux.id_confirming) {
                                            (None, _) => {}
                                            (Some(_), false) => copied = page::Page::new().primary_with(ui, t(Key::IdDoCopied), self.ux.id_words_seen).1.clicked(),
                                            (Some((_, picks)), true) => {
                                                if page::Page::new().primary_with(ui, t(Key::IdDoConfirm), !busy).1.clicked() {
                                                    let answers = picks.iter().zip(self.ux.id_confirm.iter()).map(|(p, w)| (*p, w.clone())).collect();
                                                    act = Some(Action::ConfirmIdentity { answers, label: self.ux.id_new_label.clone() });
                                                }
                                            }
                                        },
                                        Step::Own(o) => {
                                            let (asked, next) = self.own_wiz_keys(ui, o);
                                            if asked.is_some() {
                                                act = asked;
                                            }
                                            if next {
                                                move_to = Some(at + 1);
                                            }
                                        }
                                        Step::Backup => open_export = page::Page::new().primary_with(ui, t(Key::BackupExportTitle), self.shell.unlocked()).1.clicked(),
                                    }
                                }
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing.x = tk::S2;
                                    if words_shown && open {
                                        if key::key(ui, t(Key::IdHideNewWords), Role::Secondary, true).clicked() {
                                            self.ux.id_words_open = false;
                                        }
                                    } else if back.is_some() && key::key(ui, t(Key::WizPrev), Role::Secondary, true).clicked() {
                                        go_back = true;
                                    }
                                    if later_on && key::key(ui, later, Role::Secondary, true).clicked() {
                                        if last {
                                            close = true;
                                        } else {
                                            move_to = Some(at + 1);
                                        }
                                    }
                                    if busy && step != Step::Pin {
                                        sheet::busy_note(ui, t(Key::VaultBusy), false);
                                    }
                                });
                            });
                        });
                    });
                });
            });
        });
        match row {
            Some(WizRow::Generate) => act = Some(Action::NewIdentity),
            Some(WizRow::Import) => {
                let label = std::mem::take(&mut self.ux.id_new_label);
                self.ux.id_open(IdModal::Import);
                self.ux.id_new_label = label;
            }
            Some(WizRow::Restore) => self.bk_open(Bk::Restore(RestoreFrom::FirstRun)),
            None => {}
        }
        if let Some(lang) = lang_to {
            let _ = self.act(Action::SetLang { lang }, now);
        }
        if let (true, Some((to, to_sub))) = (go_back, back) {
            if to == step {
                self.ux.id_confirming = to_sub == crate::nav::Sub::Check;
                for c in self.ux.id_confirm.iter_mut() {
                    *c = crate::secret::Secret::default();
                }
            } else {
                move_to = walk.iter().position(|s| *s == to);
            }
        }
        if copied {
            self.ux.id_confirming = true;
            self.ux.id_words_open = false;
        }
        if open_export {
            self.bk_open(Bk::Export);
        }
        if rail_extra {
            self.own_rail_go(now);
        }
        let esc = exit_shown && eravon_ui::layer::esc(ctx, egui::Id::new("eravon-wizard")) && !self.any_sheet();
        let mut asked_now = false;
        if (exit || esc) && !self.ux.wiz_exit_ask {
            if self.shell.new_words.is_some() {
                self.ux.wiz_exit_ask = true;
                asked_now = true;
            } else {
                self.wizard_exit(now);
                return;
            }
        }
        if self.ux.wiz_exit_ask {
            let (mut keep, mut leave) = (false, false);
            let out = sheet::show(
                ctx,
                sheet::Spec::new("wiz-exit", tk::SHEET_W),
                self,
                |ui, _me| {
                    sheet::title(ui, t(Key::WizExitTitle), "");
                },
                |ui, _me| {
                    keep = page::Page::new().primary(ui, t(Key::WizExitKeep)).1.clicked();
                    leave = key::key(ui, t(Key::WizExitGo), Role::Secondary, true).clicked();
                },
            );
            if keep || (out.esc && !asked_now) {
                self.ux.wiz_exit_ask = false;
            } else if leave {
                self.wizard_exit(now);
                return;
            }
        }
        if self.own_wiz_turn(ctx, step, done, now) {
            move_to = move_to.or(Some(at + 1));
        }
        if let Some(a) = act {
            let was_pin = matches!(a, Action::SetPin { .. });
            let r = self.act(a, now);
            if was_pin {
                self.vault_or(VaultSite::Wizard, r, now);
            }
        }
        if close {
            self.ux.wizard = None;
            let said = self.own_finished_words().unwrap_or_else(|| t(Key::WizFinished).to_string());
            self.toasts.say(said, Tone::Note, now);
        } else if let Some(i) = move_to {
            self.ux.wizard = walk.get(i).copied();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn wizard_body(&mut self, ui: &mut egui::Ui, st: &mut fields::Stack, step: crate::nav::Step, done: bool, fresh: Option<&(Vec<crate::secret::Secret>, [usize; 3])>, busy: bool, now: f64) -> Option<WizRow> {
        use crate::nav::Step;
        let len = crate::keybox::PIN_LEN;
        let readable = match step {
            Step::Own(o) => self.own_wiz_readable(o),
            _ => false,
        };
        if done && !readable {
            let nth = Step::ALL.iter().position(|s| *s == step).unwrap_or(0);
            parts::done(ui, &format!("wizard-done-{nth}"), t(Key::Done));
            let made = match step {
                Step::Key => self.shell.anchor.map(|a| (Key::IdAddress, a.hex())),
                Step::Own(o) => self.own_wiz_made(o),
                Step::Backup => self.shell.machine.backup.as_ref().map(|b| (Key::DataBackupFile, b.path.clone())),
                Step::Pin => None,
            };
            if let Some((k, v)) = made {
                let copied = eravon_ui::fields::rows(ui, |ui, f| f.info(ui, t(k), card::Show::Copy(&v)));
                tell_copied(ui.ctx(), copied);
            }
            return None;
        }
        let mut row = None;
        match step {
            Step::Pin => {
                let again = !self.ux.pin_again.is_empty();
                let full = ui
                    .vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        if again {
                            fields::name(ui, t(Key::PinAgain), None);
                        }
                        let r = parts::pins(ui, "wiz-pin", &mut self.ux.pin, len, self.ux.pin_shake, !busy);
                        if !again && !busy {
                            paint::text(ui, t(Key::PinRules), Type::Small, c(C::Ink2));
                        }
                        r
                    })
                    .inner
                    .full;
                if full && !again {
                    self.take_first_pin(now, PinSaid::Toast);
                } else if full {
                    let a = Action::SetPin { pin: self.ux.pin_again.clone(), again: std::mem::take(&mut self.ux.pin) };
                    let r = self.act(a, now);
                    self.vault_or(VaultSite::Wizard, r, now);
                }
                if busy {
                    sheet::busy_note(ui, t(Key::VaultBusy), true);
                }
            }
            Step::Key => match fresh {
                None => {
                    st.field(ui, t(Key::IdLabel), Some(t(Key::ProxyCellOptional)), None, |ui, w| fields::line(ui, &mut self.ux.id_new_label, t(Key::IdLabelExample), w, fields::Look::default()));
                    eravon_ui::fields::rows(ui, |ui, f| {
                        if f.go(ui, t(Key::WizDoGenerate), "", true, "").clicked() {
                            row = Some(WizRow::Generate);
                        }
                        if f.go(ui, t(Key::IdDoImportExisting), "", true, "").clicked() {
                            row = Some(WizRow::Import);
                        }
                        if f.go(ui, t(Key::BackupRestoreTitle), "", !busy, "").clicked() {
                            row = Some(WizRow::Restore);
                        }
                    });
                }
                Some((words, _)) if !self.ux.id_confirming => {
                    if parts::words(ui, "wiz-words", Some(words.as_slice()), self.ux.id_words_open, t(Key::IdShowNewWords)).clicked() {
                        self.ux.id_words_open = true;
                        self.ux.id_words_seen = true;
                    }
                }
                Some((_, picks)) => {
                    for (k, p) in picks.iter().enumerate() {
                        st.field(ui, &fill1(Key::IdWordN, &(p + 1).to_string()), None, None, |ui, w| input::secret_line_w_marked(ui, &mut self.ux.id_confirm[k], "", w, false));
                    }
                }
            },
            Step::Own(o) => self.own_wiz_card(ui, st, o, now),
            Step::Backup => {
                self.measure_if_due(now);
                let content = self.data_content();
                eravon_ui::fields::rows(ui, |ui, f| {
                    super::backup::content_row(f, ui, &content);
                });
            }
        }
        row
    }

}

const WIZ_CARD_W: f32 = 520.0;
