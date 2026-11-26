use super::*;

impl Win {
    pub(super) fn settings_home(&mut self, ui: &mut egui::Ui, now: f64) {
        let role = self.shell.settings.role;
        let mut open: Option<Section> = None;
        stagger(ui, 0, |ui| {
            card::form(ui, |ui, f| {
                for s in crate::nav::sections(role) {
                    if f.go_glyph(ui, s.glyph(), t(s.key())).clicked() {
                        open = Some(s);
                    }
                }
            });
        });
        if let Some(s) = open {
            self.push(Route::Section(s), now);
        }
    }

    pub(super) fn settings_section(&mut self, ui: &mut egui::Ui, s: Section, now: f64) {
        match s {
            Section::Language => self.set_language(ui, now),
            Section::Appearance => self.set_appearance(ui, now),
            Section::Keys => self.set_keys(ui, now),
            Section::Network => self.set_network(ui, now),
            Section::Notify => self.set_notify(ui, now),
            Section::Data => self.set_data(ui, now),
            Section::Cli => self.set_cli(ui, now),
            Section::About => self.set_about(ui, now),
            Section::House(o) => self.house_section(ui, o, now),
        }
    }

    fn set_language(&mut self, ui: &mut egui::Ui, now: f64) {
        let langs = crate::lang::Lang::ALL;
        let cur = langs.iter().position(|l| *l == crate::lang::lang()).unwrap_or(0);
        let zones = crate::when::Zone::ALL;
        let zone_now = zones.iter().position(|z| *z == crate::when::zone()).unwrap_or(0);
        let lang_cells: Vec<seg::Cell> = langs.iter().map(|l| seg::Cell::from(l.label())).collect();
        let zone_cells: Vec<seg::Cell> = zones.iter().map(|z| seg::Cell::from(zone_label(*z))).collect();
        let example = fill1(Key::ZoneExample, &crate::when::when(wall_secs()));
        let (mut pick_lang, mut pick_zone) = (None, None);
        let zone_say = if crate::when::zone() == crate::when::Zone::System && !crate::when::system_read() { t(Key::ZoneSystemUnread).to_string() } else { example };
        stagger(ui, 0, |ui| {
            card::form(ui, |ui, f| {
                f.row_with(ui, None, t(Key::Language), "", card::Value::None, |ui| pick_lang = seg::seg(ui, "set-lang", &lang_cells, cur));
                f.row_with(ui, None, t(Key::SetZoneTitle), &zone_say, card::Value::None, |ui| pick_zone = seg::seg(ui, "set-zone", &zone_cells, zone_now));
            });
        });
        if let Some(i) = pick_lang {
            if i != cur || self.shell.settings.lang != Some(langs[i]) {
                self.act(Action::SetLang { lang: langs[i] }, now);
            }
        }
        if let Some(i) = pick_zone {
            if i != zone_now || self.shell.settings.zone != Some(zones[i]) {
                self.act(Action::SetZone { zone: zones[i] }, now);
            }
        }
    }

    fn set_cli(&mut self, ui: &mut egui::Ui, now: f64) {
        self.set_cli_path(ui, now);
        self.set_cli_writes(ui, now);
        self.set_cli_anchor(ui, now);
    }

    fn set_cli_anchor(&mut self, ui: &mut egui::Ui, now: f64) {
        let all = crate::machine::CliAnchor::ALL;
        let cur = all.iter().position(|x| *x == self.shell.machine.cli_anchor).unwrap_or(0);
        let cells: Vec<seg::Cell> = all.iter().map(|x| seg::Cell::from(t(cli_anchor_label(*x)))).collect();
        let live = eravon_cli::home::anchor_live(&self.shell.machine);
        let mut pick = None;
        stagger(ui, 2, |ui| {
            card::section(ui, t(Key::SetCliAnchoring), "", |ui| {
                card::form(ui, |ui, f| {
                    pick = f.with(ui, t(Key::SetCliAnchor), "", |ui| {
                        let r = ui.add_enabled_ui(live, |ui| seg::seg(ui, "set-cli-anchor", &cells, cur));
                        if !live {
                            r.response.on_disabled_hover_text(t(Key::SetCliAnchorGrey));
                        }
                        r.inner.filter(|_| live)
                    });
                });
            });
        });
        if let Some(i) = pick.filter(|i| *i != cur) {
            self.act(Action::SetCliAnchor { to: all[i] }, now);
        }
    }

    fn set_cli_path(&mut self, ui: &mut egui::Ui, now: f64) {
        use eravon_os::cli_path::State;
        if now - self.ux.cli_path_seen > 1.0 || self.shell.cli_path.is_none() {
            self.act(Action::ReadCliPath, now);
        }
        self.ux.cli_path_seen = now;
        let state = self.shell.cli_path.clone().unwrap_or(State::Off);
        let busy = self.shell.tasks.in_flight(crate::task::Kind::CliPath);
        let (on, can, say) = match &state {
            State::On => (true, true, t(Key::CliPathOnSay).to_string()),
            State::Off => (false, true, String::new()),
            State::Taken(what) => (false, true, fill1(Key::CliPathTakenSay, what)),
            State::Provided => (true, false, t(Key::CliPathProvidedSay).to_string()),
            State::Unsupported(_) => (false, false, t(Key::CliPathUnsupportedSay).to_string()),
        };
        let config = self.shell.ai_config.clone();
        let mut flip = false;
        let mut copy = false;
        stagger(ui, 0, |ui| {
            card::section(ui, t(Key::SetCliTerminal), "", |ui| {
                card::form(ui, |ui, f| {
                    flip = f.switch(ui, t(Key::SetCliPath), &say, on, can && !busy);
                    if let State::Unsupported(why) = &state {
                        f.fold(ui, "cli-path-why", t(Key::SetEvidence), "", |ui, f| f.note(ui, why));
                    }
                    copy = f.with(ui, t(Key::SetCliAiApp), "", |ui| {
                        let k = key::Key::new(t(Key::DoCopyAiConfig), Role::Secondary).enabled(config.is_some());
                        let r = key::show(ui, if config.is_some() { k } else { k.why(t(Key::CliPathUnsupportedSay)) });
                        eravon_ui::probe::spot(ui.ctx(), "cli:copyAi", r.rect);
                        r.clicked()
                    });
                });
            });
        });
        if flip {
            self.act(Action::SetCliPath { on: !on }, now);
        }
        if let Some(c) = config.filter(|_| copy) {
            if self.shell.cli_alone_now() == crate::shell::CliAloneNow::Off {
                ui.ctx().copy_text(c);
                self.toasts.say(t(Key::SaidCopied), Tone::Note, now);
            } else {
                self.pass_open();
                self.ux.pass_for_ai = true;
                self.ux.pass_note = fill1(Key::CliPassAiNote, &crate::when::when(crate::when::wall_secs()));
            }
        }
    }

    fn set_cli_writes(&mut self, ui: &mut egui::Ui, now: f64) {
        let path = self.shell.machine.cli_path();
        let alone = path == crate::machine::CliPath::Alone;
        let desktop = path == crate::machine::CliPath::Desktop;
        let unlocked = self.shell.unlocked();
        let alone_say = match (alone, self.shell.credential) {
            (true, Some(c)) => cli_expiry_said(c.expires),
            _ => String::new(),
        };
        let desktop_say = match (alone, desktop, self.shell.door_why()) {
            (true, _, _) => t(Key::SetCliDesktopGrey).to_string(),
            (false, true, Some(crate::door::DoorWhy::TooLong)) => t(Key::SetCliDoorLong).to_string(),
            (false, true, Some(crate::door::DoorWhy::Refused(w))) => fill1(Key::SetCliDoorShut, &w),
            _ => String::new(),
        };
        let (mut flip_desktop, mut flip_alone) = (false, false);
        stagger(ui, 1, |ui| {
            card::section(ui, t(Key::SetCliLevel), "", |ui| {
                card::form(ui, |ui, f| {
                    flip_desktop = f.switch(ui, t(Key::SetCliDesktop), &desktop_say, desktop, unlocked && !alone);
                    flip_alone = f.switch(ui, t(Key::SetCliAlone), &alone_say, alone, unlocked);
                });
            });
        });
        let now_state = self.shell.cli_alone_now();
        let rows = self.shell.passes.clone();
        let (mut add, mut drop): (bool, Option<String>) = (false, None);
        stagger(ui, 2, |ui| {
            card::section(ui, t(Key::CliPasses), "", |ui| {
                card::form(ui, |ui, f| {
                    if now_state == crate::shell::CliAloneNow::Off {
                        let _ = f.info(ui, t(Key::CliPassFirstOn), card::Show::Empty(""));
                    }
                    for r in &rows {
                        let (_, hit) = f.line(ui, None, &r.note, &fill1(Key::SaidRestAdded, &crate::when::when(r.added)), false, false, c(C::Ink), |ui| {
                            let k = key::key(ui, t(Key::SetRemove), Role::Secondary, unlocked);
                            eravon_ui::probe::spot(ui.ctx(), &format!("cli:drop:{}", r.tag), k.rect);
                            k.clicked()
                        });
                        if hit {
                            drop = Some(r.tag.clone());
                        }
                    }
                    let on = now_state != crate::shell::CliAloneNow::Off && unlocked;
                    let r = f.go(ui, t(Key::CliPassAddGo), "", on, t(Key::CliPassFirstOn));
                    eravon_ui::probe::spot(ui.ctx(), "cli:add", r.rect);
                    add = r.clicked() && on;
                });
            });
        });
        if add {
            self.pass_open();
        }
        if let Some(tag) = drop {
            self.act(Action::CliPassDrop { tag }, now);
        }
        if flip_desktop {
            self.act(Action::SetCliDesktop { on: !desktop }, now);
        }
        if flip_alone {
            if alone {
                self.act(Action::CliAloneOff, now);
            } else {
                self.cli_open();
            }
        }
    }

    pub(super) fn cli_open(&mut self) {
        self.sheets_clear();
        self.ux.cli_sheet = true;
        self.ux.cli_span = Default::default();
        self.ux.cli_never = false;
    }

    pub(super) fn pass_open(&mut self) {
        self.sheets_clear();
        self.ux.pass_open = true;
        self.ux.pass_for_ai = false;
        self.ux.pass_note.clear();
        self.ux.pass_pin.clear();
        self.ux.pass_out = None;
        self.ux.pass_trouble = None;
    }

    pub(super) fn pass_sheet(&mut self, ctx: &egui::Context, now: f64) {
        if !self.ux.pass_open {
            self.ux.pass_out = None;
            return;
        }
        if self.ux.pass_for_ai {
            if let Some(pass) = self.ux.pass_out.take() {
                match self.shell.ai_config.as_deref().and_then(|c| crate::action::ai_config_with_pass(c, pass.expose())) {
                    Some(config) => {
                        ctx.copy_text(config);
                        self.toasts.say(t(Key::SaidCopiedAiConfigKeyed), Tone::Note, now);
                    }
                    None => self.toasts.say(t(Key::CliPathUnsupportedSay), Tone::Bad, now),
                }
                drop(pass);
                self.ux.pass_open = false;
                self.ux.pass_for_ai = false;
                self.ux.pass_pin.clear();
                return;
            }
        }
        let len = crate::keybox::PIN_LEN;
        let busy = self.shell.tasks.in_flight(crate::task::Kind::Vault);
        let trouble = self.ux.pass_trouble.clone();
        let made = self.ux.pass_out.as_ref().map(|p| p.expose().to_string());
        let (mut go, mut close) = (false, false);
        let out = sheet::show(
            ctx,
            sheet::Spec::new("cli-pass", tk::SHEET_NARROW),
            self,
            |ui, me| {
                sheet::title(ui, t(if me.ux.pass_for_ai { Key::DoCopyAiConfig } else { Key::CliPassAddGo }), "");
                match &made {
                    Some(text) => {
                        let mut copied = None;
                        fields::stack(ui, |ui, st| {
                            st.field(ui, t(Key::CliPasses), None, None, |ui, w| copied = card::whole(ui, text, w));
                        });
                        tell_copied(ui.ctx(), copied);
                        sheet::foot_note(ui, t(Key::CliPassOnce));
                    }
                    None => {
                        fields::stack(ui, |ui, st| {
                            st.field(ui, t(Key::BoNote), None, None, |ui, w| fields::line(ui, &mut me.ux.pass_note, "", w, fields::Look::default()));
                            let _ = pin_field(st, ui, t(Key::IdPinGate), None, "cli-pass-pin", &mut me.ux.pass_pin, me.ux.pin_shake, !busy);
                        });
                    }
                }
                if let Some(f) = &trouble {
                    states::err_box(ui, "cli-pass-trouble", f.human(), &f.next(), t(Key::U3RawError), &f.shown());
                }
            },
            |ui, me| {
                if made.is_some() {
                    close = page::Page::new().primary(ui, t(Key::WizFinish)).1.clicked();
                    return;
                }
                let ready = !me.ux.pass_note.trim().is_empty() && me.ux.pass_pin.chars() == len && !busy;
                let why = still_needs(&[(me.ux.pass_note.trim().is_empty(), Key::BoNote), (me.ux.pass_pin.chars() != len, Key::IdPinGate)]);
                let r = page::Page::new().primary_why(ui, t(Key::CliPassAddGo), ready, &why).1;
                eravon_ui::probe::spot(ui.ctx(), "cli:passGo", r.rect);
                go = r.clicked() && ready;
                close = key::key(ui, t(Key::CfBack), Role::Secondary, !busy).clicked();
            },
        );
        if close || (out.esc && !busy) {
            self.ux.pass_open = false;
            self.ux.pass_for_ai = false;
            self.ux.pass_out = None;
            self.ux.pass_pin.clear();
            return;
        }
        if go {
            self.ux.pass_trouble = None;
            let pin = std::mem::take(&mut self.ux.pass_pin);
            let r = self.act(Action::CliPassAdd { note: self.ux.pass_note.clone(), pin }, now);
            self.vault_or(VaultSite::CliPass, r, now);
        }
    }

    pub(super) fn cli_close(&mut self) {
        self.ux.cli_sheet = false;
        self.ux.cli_pin.clear();
        self.ux.cli_trouble = None;
    }

    pub(super) fn cli_sheet(&mut self, ctx: &egui::Context, now: f64) {
        if !self.ux.cli_sheet {
            return;
        }
        let len = crate::keybox::PIN_LEN;
        let busy = self.shell.tasks.in_flight(crate::task::Kind::Vault);
        let span = cli_span(&self.ux.cli_span, self.ux.cli_never);
        let lasts = match span {
            Span::Lasts(l) => Some(l),
            _ => None,
        };
        let pin_full = self.ux.cli_pin.chars() == len;
        let trouble = self.ux.cli_trouble.clone();
        let said = cli_span_said(span, self.shell.desk.pace_now());
        let (mut go, mut close) = (false, false);
        let out = sheet::show(
            ctx,
            sheet::Spec::new("cli-alone", tk::SHEET_NARROW).keys_above(tk::S5),
            self,
            |ui, me| {
                sheet::title(ui, t(Key::SetCliAlone), "");
                ui.add_space(tk::S4 - 6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let never = me.ux.cli_never;
                    let units: Vec<&str> = CLI_SPAN.iter().map(|(unit, _, _)| t(*unit)).collect();
                    let bad: Vec<bool> = (0..CLI_SPAN.len()).map(|i| matches!(span, Span::Over(o) if o[i])).collect();
                    let line = (!said.0.is_empty()).then(|| (said.0.as_str(), said.1));
                    fields::stack(ui, |ui, st| st.field(ui, t(Key::U3Window), None, line, |ui, _| eravon_ui::lasts::boxed(ui, "cli-span", &mut me.ux.cli_span, &units, "0", &bad, !never)));
                    ui.add_space(tk::S3);
                    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), Type::Note.line()), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let w = ui.add(egui::Label::new(egui::RichText::new(t(Key::CliLastsNever)).font(Type::Note.font()).color(c(C::Ink2))).selectable(false).sense(egui::Sense::click()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let s = toggle::switch(ui, me.ux.cli_never, true);
                            if s.clicked() || w.clicked() {
                                me.ux.cli_never = !me.ux.cli_never;
                            }
                        });
                    });
                    ui.add_space(tk::S5);
                    fields::name(ui, t(Key::IdPinGate), None);
                    ui.add_space(tk::S3);
                    let _ = eravon_ui::sheetparts::pins(ui, "cli-alone-pin", &mut me.ux.cli_pin, len, me.ux.pin_shake, !busy);
                });
                if let Some(f) = &trouble {
                    states::err_box(ui, "cli-trouble", f.human(), &f.next(), t(Key::U3RawError), &f.shown());
                }
            },
            |ui, _me| {
                let why = still_needs(&[(lasts.is_none(), Key::U3Window), (!pin_full, Key::IdPinGate)]);
                go = page::Page::new().primary_why(ui, t(Key::DoCliAloneOn), pin_full && lasts.is_some() && !busy, &why).1.clicked();
                close = key::key(ui, t(Key::CfBack), Role::Secondary, true).clicked();
                if busy {
                    sheet::foot_note(ui, t(Key::VaultBusy));
                }
            },
        );
        if close || out.esc {
            self.cli_close();
        } else if let (true, Some(lasts)) = (go, lasts) {
            let pin = std::mem::take(&mut self.ux.cli_pin);
            let r = self.act(Action::SetCliAlone { lasts, pin }, now);
            self.vault_or(VaultSite::Cli, r, now);
        }
    }

    fn set_appearance(&mut self, ui: &mut egui::Ui, now: f64) {
        let all = [skin::Appearance::Light, skin::Appearance::Dark, skin::Appearance::System];
        let cur = all.iter().position(|a| *a == self.appearance()).unwrap_or(0);
        let cells: Vec<seg::Cell> = all.iter().map(|a| seg::Cell::from(appearance_label(a.as_str()))).collect();
        let mut pick = None;
        stagger(ui, 0, |ui| {
            card::form(ui, |ui, f| {
                f.row_with(ui, None, &crate::house::i18n::t(crate::house::i18n::Key::SetTheme), "", card::Value::None, |ui| pick = seg::seg(ui, "set-appearance", &cells, cur));
            });
        });
        if let Some(i) = pick {
            if i != cur {
                self.act(Action::SetAppearance { appearance: all[i].as_str().to_string() }, now);
            }
        }
    }

    fn set_notify(&mut self, ui: &mut egui::Ui, now: f64) {
        self.house_notify(ui, now);
    }

    fn set_data(&mut self, ui: &mut egui::Ui, now: f64) {
        self.measure_if_due(now);
        let ids = self.shell.identities.as_ref().map(|r| r.rows.len()).unwrap_or(0);
        let archive = self.shell.archive.clone();
        let content = match archive.as_ref() {
            Some(a) => crate::lang::filln(Key::BackupContent, &[&ids.to_string(), "0", &a.records.to_string()]),
            None => t(Key::SetNotRead).to_string(),
        };
        let usage = archive.as_ref().map(|a| size_say(a.bytes)).unwrap_or_else(|| t(Key::SetNotRead).to_string());
        let seat_ok = !self.shell.seat_unseated();
        stagger(ui, 0, |ui| {
            card::form(ui, |ui, f| {
                let _ = f.info(ui, t(Key::DataContent), if archive.is_some() { card::Show::Text(&content) } else { card::Show::Empty(&content) });
                let _ = f.info(ui, t(Key::DataProtect), card::Show::Mark(Mark::Ok, t(Key::DataProtectOn)));
            });
        });
        if !seat_ok {
            eravon_ui::callout::callout(ui, t(Key::IdSeatEmptyNote));
        }
        self.backup_group(ui, now);
        let root = self.shell.home.as_ref().map(|h| h.root().display().to_string());
        let measure_phase = self.phase_of(crate::task::Kind::Archive);
        let (mut open, mut measure) = (None, false);
        stagger(ui, 2, |ui| {
            card::form(ui, |ui, f| {
                let path = root.clone().unwrap_or_else(|| t(Key::HomeNotOpen).to_string());
                if f.act_path(ui, t(Key::SetHomeOfSeat), &path, key::Key::new(t(Key::SetChangeHome), Role::Secondary).enabled(seat_ok)).clicked() {
                    open = Some(DirSheet::Home);
                }
                measure = f.act_value(ui, t(Key::SetUsage), "", &usage, key::Key::new(t(Key::DoMeasure), Role::Secondary).enabled(seat_ok).phase(measure_phase)).clicked();
            });
            self.stage_line(ui, crate::task::Kind::Archive);
        });
        let lock = match self.shell.lock.as_ref() {
            None => t(Key::HomeNoLock).to_string(),
            Some(l) => format!("{} \u{b7} {}", l.mode().as_str(), l.holder()),
        };
        let miss = self.shell.home.as_ref().map(|h| h.missing()).unwrap_or_default();
        let last = self.shell.machine.backup.clone();
        let none = || Val::text(t(Key::SetNone));
        stagger(ui, 4, |ui| {
            details_card(
                ui,
                "data-details",
                &[
                    (t(Key::SetHomeOfSeat), root.clone().map(Val::mono).unwrap_or_else(|| Val::text(t(Key::HomeNotOpen)))),
                    (t(Key::DataContent), Val::text(content.clone())),
                    (t(Key::DataEncrypt), Val::figure(t(Key::DataEncryptHow).to_string())),
                    (t(Key::DataBackupFile), last.as_ref().map(|b| Val::mono(b.path.clone())).unwrap_or_else(none)),
                    (t(Key::DataBackupHow), Val::figure(t(Key::DataBackupHowSay).to_string())),
                    (t(Key::HomeWhere), Val::text(t(Key::HomeWhereNote))),
                    (t(Key::HomeInstance), Val::mono(lock.clone())),
                    (t(Key::HomeMissingRooms), if miss.is_empty() { Val::text(t(Key::Nothing)) } else { Val::mono(miss.join(" ")) }),
                ],
            )
        });
        if let Some(d) = open {
            self.dir_open(d);
        }
        if measure {
            self.act(Action::Measure, now);
        }
    }

    pub(super) fn dir_open(&mut self, d: DirSheet) {
        let docs = self.shell.documents.clone();
        let slot = self.dir_slot(d);
        if slot.trim().is_empty() {
            *slot = docs;
        }
        self.ux.dir_sheet = Some(d);
    }

    fn dir_slot(&mut self, d: DirSheet) -> &mut String {
        match d {
            DirSheet::Home => &mut self.typed.home,
        }
    }

    pub(super) fn dir_sheet(&mut self, ctx: &egui::Context, now: f64) {
        let Some(d) = self.ux.dir_sheet else { return };
        let title = match d {
            DirSheet::Home => Key::SetChangeHome,
        };
        let at = self.dir_slot(d).clone();
        let ready = match d {
            DirSheet::Home => Self::landing_ok(&at),
        };
        let (mut go, mut close, mut picked) = (false, false, None);
        let out = sheet::show(
            ctx,
            sheet::Spec::new("data-folder", tk::SHEET_W),
            self,
            |ui, _me| {
                sheet::title(ui, t(title), "");
                card::form(ui, |ui, f| {
                    let label = match d {
                        DirSheet::Home => t(Key::SetHomeOfSeat),
                    };
                    f.folder(ui, label, &at, t(Key::PickNone));
                });
            },
            |ui, _me| {
                let k = match d {
                    DirSheet::Home => key::Key::new(t(Key::DoOpenHome), Role::Primary),
                };
                go = key::show(ui, k.enabled(ready)).clicked();
                close = key::key(ui, t(Key::CfBack), Role::Secondary, true).clicked();
                picked = foot_pick(ui, "data-folder", crate::platform::Pick::Folder);
            },
        );
        if let Some(p) = picked {
            *self.dir_slot(d) = p;
        }
        if close || out.esc {
            self.ux.dir_sheet = None;
            return;
        }
        if !go {
            return;
        }
        match d {
            DirSheet::Home => {
                let a = Action::ChangeHome { root: self.typed.home.clone() };
                self.act(a, now);
                self.ux.lang_applied = false;
                self.ux.dir_sheet = None;
            }
        }
    }

    pub(super) fn backup_behind(&self) -> Option<u64> {
        crate::machine::backup_behind(self.shell.machine.backup.as_ref(), self.shell.items_now)
    }

    pub(super) fn backup_point(&self) -> (Mark, String) {
        let last = self.shell.machine.backup.as_ref();
        match (crate::firstrun::backup_point(last, self.shell.items_now, self.shell.machine.backup_failed), last) {
            (crate::firstrun::Shade::Red, _) => (Mark::Bad, t(Key::GapBackupFailed).to_string()),
            (crate::firstrun::Shade::Amber, _) => (Mark::Warn, fill1(Key::GapBackupBehind, &self.backup_behind().unwrap_or(0).to_string())),
            (crate::firstrun::Shade::Green, Some(b)) => (Mark::Ok, crate::when::when(b.at)),
            (crate::firstrun::Shade::Grey, Some(b)) => (Mark::Todo, crate::when::when(b.at)),
            _ => (Mark::Todo, t(Key::GapBackupNever).to_string()),
        }
    }

    fn set_about(&mut self, ui: &mut egui::Ui, now: f64) {
        let unseated = self.shell.seat_unseated();
        let last = self.shell.last.as_ref().map(|x| format!("{} \u{b7} {} \u{b7} {}", x.found(), x.faces.iter().map(|f| f.bytes).sum::<u64>(), x.marks));
        let phase = self.phase_of(crate::task::Kind::SelfCheck);
        let title = format!("{} {}", t(Key::AppName), crate::about::VERSION);
        stagger(ui, 0, |ui| {
            card::form(ui, |ui, f| f.summary(ui, Glyph::Info, card::Tint::Blue, &title, None, build_kind(), |_| ()));
        });
        let points = self.checklist();
        let goes: Vec<Point> = vec![Point::Step(crate::nav::Step::Key), Point::Step(crate::nav::Step::Pin), Point::Backup];
        let mut go = None;
        let (mut again, mut self_check) = (false, false);
        stagger(ui, 1, |ui| {
            card::section(ui, t(Key::SetMissing), "", |ui| {
                card::form(ui, |ui, f| {
                    for ((m, name, _), p) in points.iter().zip(&goes) {
                        if *m == Mark::Ok {
                            let word = match p {
                                Point::Backup => t(Key::Done),
                                _ => "",
                            };
                            let _ = f.info(ui, name, card::Show::Mark(Mark::Ok, word));
                            continue;
                        }
                        let label = match p {
                            Point::Step(s) => t(s.title()),
                            Point::Backup => t(Key::BackupExportTitle),
                        };
                        if f.act(ui, name, "", key::Key::new(label, Role::Secondary).enabled(!unseated)).clicked() {
                            go = Some(*p);
                        }
                    }
                });
            });
        });
        stagger(ui, 2, |ui| {
            card::form(ui, |ui, f| {
                again = f.act(ui, t(Key::PageFirstRun), "", key::Key::new(t(Key::DoWizardAgain), Role::Secondary).enabled(!unseated)).clicked();
                self_check = f.act(ui, t(Key::TaskSelfCheck), last.as_deref().unwrap_or(""), key::Key::new(t(Key::DoSelfCheck), Role::Secondary).phase(phase)).clicked();
            });
            self.stage_line(ui, crate::task::Kind::SelfCheck);
        });
        if again {
            let step = self.progress().first_open().unwrap_or(crate::nav::Step::Pin);
            self.wizard_open(step);
        }
        match go {
            Some(Point::Step(s)) => self.wizard_open(s),
            Some(Point::Backup) => self.bk_open(Bk::Export),
            None => {}
        }
        let fonts: Vec<String> = eravon_ui::fonts::Role::ALL
            .iter()
            .map(|role| match self.shell.fonts.face(*role) {
                Some(f) => {
                    let place = t(if f.place == eravon_ui::fonts::Place::Embedded { Key::FontEmbedded } else { Key::FontFromSystem });
                    match f.licence() {
                        Some(l) => format!("{} {} \u{b7} {} \u{b7} {place} \u{b7} {l}", role.as_str(), f.file, f.index),
                        None => format!("{} {} \u{b7} {} \u{b7} {place}", role.as_str(), f.file, f.index),
                    }
                }
                None => format!("{} {}", role.as_str(), t(Key::Missing)),
            })
            .collect();
        let sink = match trace::sink() {
            Sink::Memory => t(Key::TraceMemory).to_string(),
            Sink::File(p) => width::file_name(&p.display().to_string()),
        };
        let flying = self.shell.tasks.flying();
        let extra = self.house_version_rows();
        let mut rows: Vec<(&str, Val)> = vec![
            (t(Key::BuildKind), Val::figure(build_kind().to_string())),
            (t(Key::FontGroup), Val::figure(fonts.join("\n"))),
            (t(Key::TraceSink), Val::mono(sink)),
            (t(Key::TaskInFlight), if flying.is_empty() { Val::text(t(Key::Nothing)) } else { Val::figure(flying.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(" ")) }),
        ];
        rows.extend(extra.iter().map(|(k, v)| (k.as_str(), v.clone())));
        stagger(ui, 2, |ui| details_card(ui, "about-details", &rows));
        stagger(ui, 3, |ui| {
            card::form(ui, |ui, f| {
                let _ = f.fold(ui, "about-notices", t(Key::SetNotices), &crate::about::count().to_string(), |ui, f| {
                    f.free(ui, |ui| {
                        let lines = crate::about::notice_lines();
                        let row_h = Type::MonoSmall.line();
                        ui.spacing_mut().item_spacing.y = 0.0;
                        eravon_ui::scroll::nested_rows(ui, egui::ScrollArea::both().id_salt("about-notices-text").max_height(320.0).auto_shrink([false, true]), row_h, lines.len(), |ui, range| {
                            for l in &lines[range] {
                                ui.add(egui::Label::new(egui::RichText::new(*l).font(Type::MonoSmall.font()).color(c(C::Ink2)).line_height(Some(row_h))).wrap_mode(egui::TextWrapMode::Extend));
                            }
                        });
                    })
                });
            });
        });
        if self_check {
            self.act(Action::SelfCheck, now);
        }
    }

    pub(super) fn checklist(&self) -> Vec<(Mark, &'static str, String)> {
        let key_mark = if self.shell.anchor.is_some() { Mark::Ok } else { Mark::Bad };
        let pin_mark = if self.shell.vault.absent() { Mark::Bad } else { Mark::Ok };
        let (backup, backup_say) = self.backup_point();
        vec![
            (key_mark, t(Key::PointAnchorKey), t(Key::GuideMakeKey).to_string()),
            (pin_mark, t(Key::IdPin), t(Key::GuideSetPin).to_string()),
            (backup, t(Key::WizBackupTitle), backup_say),
        ]
    }
}

#[derive(Clone, Copy, Debug)]
enum Point {
    Step(crate::nav::Step),
    Backup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DirSheet {
    Home,
}

const CLI_SPAN: [(Key, u64, u64); 3] = [(Key::CliLastsDays, 365, 86_400), (Key::CliLastsHours, 23, 3_600), (Key::CliLastsMinutes, 59, 60)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Span {
    Empty,
    Over([bool; 3]),
    Zero,
    PastYear,
    Lasts(crate::machine::Lasts),
}

fn cli_span(cells: &[String; 3], never: bool) -> Span {
    use crate::machine::{Lasts, LASTS_MAX};
    use eravon_ui::lasts::Read;
    if never {
        return Span::Lasts(Lasts::Never);
    }
    let parts = CLI_SPAN.map(|(_, most, unit)| (most, unit));
    match eravon_ui::lasts::read(cells, &parts) {
        Read::Empty => Span::Empty,
        Read::Over(over) => Span::Over(std::array::from_fn(|i| over.get(i).copied().unwrap_or(false))),
        Read::Total(0) => Span::Zero,
        Read::Total(s) if s > LASTS_MAX => Span::PastYear,
        Read::Total(s) => Span::Lasts(Lasts::Secs(s)),
    }
}

fn cli_span_said(span: Span, now: u64) -> (String, bool) {
    use crate::machine::Lasts;
    match span {
        Span::Empty | Span::Lasts(Lasts::Never) => (String::new(), false),
        Span::Over(over) => {
            let each: Vec<String> = CLI_SPAN.iter().zip(over).filter(|(_, o)| *o).map(|((unit, most, _), _)| crate::lang::filln(Key::CliLastsAtMost, &[t(*unit), &most.to_string()])).collect();
            (each.join(t(Key::ListJoin)), true)
        }
        Span::Zero => (t(Key::CliLastsZero).to_string(), true),
        Span::PastYear => (t(Key::CliLastsPastYear).to_string(), true),
        Span::Lasts(Lasts::Secs(s)) => (cli_expiry_said(Some(now + s)), false),
    }
}

pub(super) fn cli_expiry_said(expires: Option<u64>) -> String {
    match expires {
        Some(at) => fill1(Key::CliAloneUntil, &crate::when::when(at)),
        None => t(Key::CliLastsNever).to_string(),
    }
}

#[cfg(test)]
mod cli_lasts_tests {
    use super::*;

    fn cells(d: &str, h: &str, m: &str) -> [String; 3] {
        [d.to_string(), h.to_string(), m.to_string()]
    }

    #[test]
    fn a_validity_is_three_cells_within_a_minute_and_a_year_or_never() {
        use crate::machine::Lasts;
        assert_eq!(cli_span(&cells("", "", ""), false), Span::Empty);
        assert_eq!(cli_span(&cells("", "", "1"), false), Span::Lasts(Lasts::Secs(60)));
        assert_eq!(cli_span(&cells("1", "2", "3"), false), Span::Lasts(Lasts::Secs(86_400 + 7_200 + 180)));
        assert_eq!(cli_span(&cells("365", "", ""), false), Span::Lasts(Lasts::Secs(365 * 86_400)));
        assert_eq!(cli_span(&cells("365", "", "1"), false), Span::PastYear);
        assert_eq!(cli_span(&cells("0", "0", "0"), false), Span::Zero);
        assert_eq!(cli_span(&cells("366", "24", "59"), false), Span::Over([true, true, false]));
        assert_eq!(cli_span(&cells("", "", "99999999999999999999999"), false), Span::Over([false, false, true]));
        assert_eq!(cli_span(&cells("400", "x", ""), true), Span::Lasts(Lasts::Never));
        for s in [cli_span(&cells("1", "", ""), false), cli_span(&cells("", "", ""), true)] {
            if let Span::Lasts(l) = s {
                assert!(l.ok());
            }
        }
    }

    #[test]
    fn a_cell_keeps_its_digits() {
        assert_eq!(eravon_ui::lasts::digits("0012"), "12");
        assert_eq!(eravon_ui::lasts::digits("1a2"), "12");
        assert_eq!(eravon_ui::lasts::digits("000"), "0");
        assert_eq!(eravon_ui::lasts::digits("x"), "");
    }

    #[test]
    fn the_line_under_the_switch_says_the_expiry_or_why_not() {
        use crate::machine::Lasts;
        assert_eq!(cli_span_said(Span::Lasts(Lasts::Secs(60)), 1_000), (cli_expiry_said(Some(1_060)), false));
        assert_eq!(cli_span_said(Span::Zero, 0), (t(Key::CliLastsZero).to_string(), true));
        assert_eq!(cli_span_said(Span::PastYear, 0), (t(Key::CliLastsPastYear).to_string(), true));
        let over = cli_span_said(Span::Over([true, false, true]), 0);
        assert!(over.1 && over.0.contains("365") && over.0.contains("59") && !over.0.contains("23"), "{over:?}");
        assert_eq!(cli_span_said(Span::Empty, 0), (String::new(), false));
        assert_eq!(cli_span_said(Span::Lasts(Lasts::Never), 0), (String::new(), false));
    }
}
