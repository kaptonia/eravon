use super::*;

impl Win {
    pub(super) fn any_sheet(&self) -> bool {
        self.ux.id_modal.is_some()
            || self.ux.bk.is_some()
            || self.ux.wiz_exit_ask
            || self.ux.cli_sheet
            || self.ux.dir_sheet.is_some()
            || self.ux.pick_which.is_some()
            || self.ux.pass_open
    }

    pub(super) fn sheets_clear(&mut self) {
        self.ux.pass_open = false;
        if self.ux.id_modal.is_some() {
            self.ux.id_close();
        }
        self.ux.sheet_shake = None;
        self.ux.bk_clear();
        self.ux.dir_sheet = None;
        self.cli_close();
        self.ux.pick_which = None;
    }

    pub(super) fn layers_clear(&mut self) {
        self.sheets_clear();
        self.ux.wizard = None;
    }

    pub(super) fn id_layer_open(&mut self, m: IdModal) {
        self.sheets_clear();
        if let IdModal::Delete(id) = &m {
            let row = self.shell.identities.clone().unwrap_or_default().find(id).cloned();
            let (seats, troubles) = row.as_ref().map(crate::action::seats_with_entries).unwrap_or_default();
            self.ux.id_delete_ledgers = seats;
            self.shell.faults.extend(troubles);
            self.ux.id_delete_backup = row.as_ref().map(crate::identity::backup_seen);
        }
        self.ux.id_open(m);
    }

    pub(super) fn wizard_open(&mut self, step: crate::nav::Step) {
        self.layers_clear();
        if self.ux.wizard.is_none() {
            self.ux.wiz_from = Some((self.shell.settings.role, self.ux.nav.stack, self.ux.nav.place, self.ux.nav.hist.get(&self.ux.nav.stack).cloned()));
        }
        self.ux.wizard = Some(step);
        self.own_wiz_enter(step);
    }

    pub(super) fn wizard_exit_shown(&self) -> bool {
        let identity = self.shell.identities.as_ref().map(|r| !r.rows.is_empty()).unwrap_or(false);
        !self.shell.vault.absent() && identity
    }

    pub(super) fn wizard_exit(&mut self, now: f64) {
        if self.shell.new_words.is_some() {
            self.act(Action::DropFresh, now);
        }
        self.ux.wiz_exit_ask = false;
        self.ux.wizard = None;
        if let Some((role, stack, place, hist)) = self.ux.wiz_from.take() {
            let seated = self.shell.identities.as_ref().and_then(|r| r.now()).map(|(row, _)| row.address(role).is_some()).unwrap_or(false);
            if self.shell.settings.role != role && seated {
                self.act(Action::SwitchRole, now);
            }
            self.ux.nav.stack = stack;
            if let Some(p) = place {
                self.go(p, now);
            }
            if let Some(h) = hist {
                self.ux.nav.hist.insert(stack, h);
            }
        }
    }

    pub(super) fn sheets(&mut self, ctx: &egui::Context, now: f64) {
        self.bk_sheets(ctx, now);
        if self.shell.vault.gate_up() {
            return;
        }
        self.dir_sheet(ctx, now);
        self.id_sheets(ctx, now);
        self.cli_sheet(ctx, now);
        self.pass_sheet(ctx, now);
        self.pick_which_sheet(ctx);
    }

    fn pick_which_sheet(&mut self, ctx: &egui::Context) {
        let Some(site) = self.ux.pick_which else { return };
        let (mut kind, mut close) = (None, false);
        let out = sheet::show(
            ctx,
            sheet::Spec::new("pick-which", tk::SHEET_W),
            self,
            |ui, _me| sheet::title(ui, t(Key::PickWhich), ""),
            |ui, _me| {
                if page::Page::new().primary(ui, t(Key::PickFile)).1.clicked() {
                    kind = Some(crate::platform::Pick::File);
                }
                if key::key(ui, t(Key::PickFolder), Role::Secondary, true).clicked() {
                    kind = Some(crate::platform::Pick::Folder);
                }
                close = ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| key::key(ui, t(Key::CfBack), Role::Secondary, true).clicked()).inner;
            },
        );
        if close || out.esc {
            self.ux.pick_which = None;
        } else if let Some(k) = kind {
            self.ux.pick_which = None;
            ctx.data_mut(|d| d.get_temp_mut_or_default::<PathMail>(path_mail()).asked = Some((site, k)));
        }
    }

    pub(super) fn said_back(&mut self, k: crate::task::Kind, a: Applied, now: f64) {
        let _ = k;
        self.told(a, None, now);
    }

}
