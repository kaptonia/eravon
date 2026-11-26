use super::*;

pub(super) fn task_key(k: crate::task::Kind) -> Key {
    use crate::task::Kind;
    match k {
        Kind::SelfCheck => Key::TaskSelfCheck,
        Kind::Archive => Key::TaskArchive,
        Kind::Keystore => Key::TaskKeystore,
        Kind::Vault => Key::TaskVault,
        Kind::Backup => Key::WizBackupTitle,
        Kind::Path => Key::TaskPath,
        Kind::CliPath => Key::TaskCliPath,
        Kind::Sync => Key::TaskSync,
        Kind::Orders => Key::NavOrders,
        Kind::Goods => Key::NavGoods,
        Kind::Cands => Key::NavCands,
        Kind::Home => Key::NavHome,
        Kind::Remind => Key::TaskRemind,
        Kind::PayLink => Key::TaskPaylink,
        Kind::Bo => Key::TaskBo,
        Kind::History => Key::PlHistory,
        Kind::BHome => Key::TaskBhome,
        Kind::Wallet => Key::TaskWallet,
        Kind::Settings => Key::NavSettings,
        Kind::Export => Key::IdExportGroup,
        Kind::First => Key::TaskFirst,
        Kind::Door => Key::TaskDoor,
        Kind::Seat => Key::TaskSeat,
        Kind::Keep => Key::TaskKeep,
        Kind::OrdersRead | Kind::CandsRead | Kind::BoRead | Kind::WalletRead | Kind::SettingsRead | Kind::ExportRead | Kind::GoodsRead | Kind::CandsLocal => task_key(k.page_of()),
    }
}

pub(super) fn task_long(k: crate::task::Kind) -> bool {
    use crate::task::Kind;
    !matches!(k, Kind::Vault | Kind::Sync | Kind::Home | Kind::BHome | Kind::Remind | Kind::Archive | Kind::Path | Kind::Keep)
}

impl Win {
    pub(super) fn own_turn(&mut self, ctx: &egui::Context, now: f64) {
        let _ = now;
        self.house_open_settings();
        self.shell.house_tick();
        if self.shell.house_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if let Some(d) = self.shell.house_wake_in() {
            ctx.request_repaint_after(d);
        }
    }

    pub(super) fn own_tag(&self, row: &crate::identity::Row) -> Option<&'static str> {
        let _ = row;
        None
    }
}

impl Win {

    pub(super) fn own_card_more(&self, seat: crate::roles::Role) -> Option<String> {
        let _ = seat;
        self.house_gas_line()
    }

}

pub(super) const APP_ID: &str = "eravon-desk";

pub(super) fn icon() -> egui::IconData {
    egui::IconData { rgba: crate::about::ICON.to_vec(), width: 256, height: 256 }
}

pub(super) const MENU_BAR: bool = true;

pub(super) const GATE_KEY: Option<Key> = None;

pub(super) fn own_stage(win: &mut Win, start: &Start) {
    let _ = (win, start);
}

impl Win {
    pub(super) fn own_after_rest(&mut self, ui: &mut egui::Ui, now: f64) {
        let _ = (ui, now);
    }

    pub(super) fn own_net_overview(&mut self, ui: &mut egui::Ui, now: f64) -> bool {
        self.house_network(ui, now);
        true
    }

    pub(super) fn own_net_details(&self) -> Option<Vec<(&'static str, Val)>> {
        self.house_net_rows()
    }

    pub(super) fn own_home_chain(&self) -> Option<u64> {
        Some(eravon_vault::store::chain_of(&self.shell.settings))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum OwnBk {}

impl Win {
    pub(super) fn own_bk_sheet(&mut self, ctx: &egui::Context, own: OwnBk, trouble: Option<crate::fault::Fault>, now: f64) -> (Option<Action>, bool, bool) {
        let _ = (ctx, trouble, now);
        match own {}
    }

    pub(super) fn own_bk_done(&self, r: &Applied) -> bool {
        let _ = r;
        false
    }

    pub(super) fn own_backup_rows(&self, ui: &mut egui::Ui, f: &mut card::Form) -> Option<OwnBk> {
        let last = self.shell.machine.backup.as_ref();
        let behind = self.backup_behind();
        let backup_say = match last {
            Some(b) => crate::when::when(b.at),
            None => t(Key::DataBackupNever).to_string(),
        };
        let _ = f.info(ui, t(Key::DataBackupLast), if last.is_some() { card::Show::Text(&backup_say) } else { card::Show::Empty(&backup_say) });
        let _ = match (last, behind) {
            (Some(_), Some(n)) if n > 0 => f.info(ui, t(Key::IdNotBacked), card::Show::Mark(Mark::Warn, &fill1(Key::U3EntriesCount, &n.to_string()))),
            (Some(_), Some(_)) => f.info(ui, t(Key::IdNotBacked), card::Show::Text(t(Key::Nothing))),
            _ => f.info(ui, t(Key::IdNotBacked), card::Show::Empty("\u{2014}")),
        };
        None
    }

    pub(super) fn own_backup_after(&mut self, ui: &mut egui::Ui, now: f64) {
        let _ = (ui, now);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum OwnIdPress {
    Rotate,
    Consent,
}

pub(super) fn own_delete_effect(seat: crate::roles::Role) -> Key {
    match seat {
        crate::roles::Role::Author => Key::IdDeleteAuthorEffect,
        crate::roles::Role::Grantee => Key::IdDeleteGranteeEffect,
    }
}

impl Win {
    pub(super) fn own_id_rows(&self, ui: &mut egui::Ui, f: &mut card::Form, current: bool, unseated: bool, can_sign: bool) -> Option<OwnIdPress> {
        let live = self.shell.unlocked();
        let rotate = f.go(ui, &crate::house::i18n::t(crate::house::i18n::Key::IdRotate), "", live && current && !unseated, "").clicked();
        let consent = f.go(ui, &crate::house::i18n::t(crate::house::i18n::Key::SetConsentRow), "", live && current && can_sign, "").clicked();
        if rotate {
            Some(OwnIdPress::Rotate)
        } else if consent {
            Some(OwnIdPress::Consent)
        } else {
            None
        }
    }

    pub(super) fn own_id_pressed(&mut self, p: OwnIdPress, now: f64) {
        let _ = now;
        match p {
            OwnIdPress::Rotate => self.house_rotate_open(),
            OwnIdPress::Consent => self.house_consent_open(),
        }
    }

    pub(super) fn own_key_rows(&self, row: Option<&crate::identity::Row>, seat: crate::roles::Role) -> Vec<(&'static str, Val)> {
        let _ = (row, seat);
        Vec::new()
    }

    pub(super) fn own_details(&self, seated: bool, seat: crate::roles::Role) -> Vec<(&'static str, Val)> {
        let _ = (seated, seat);
        Vec::new()
    }

    pub(super) fn own_hold(&self, a: &Action) -> Option<crate::fault::Fault> {
        let _ = a;
        None
    }

    pub(super) fn own_network_row(&mut self, f: &mut card::Form, ui: &mut egui::Ui, salt: &str) {
        let on = super::house::first_run::chain_on(self.shell.desk.new_chain, None);
        if let Some(c) = super::house::first_run::chain_rows(ui, f, salt, on) {
            self.shell.desk.new_chain = Some(c);
        }
    }

    pub(super) fn own_gas_fact(&self) -> Option<(Mark, eravon_ui::layer::Amount)> {
        self.house_gas_fact()
    }

    pub(super) fn own_primary_how(&self, primary: Option<&crate::identity::Row>) -> Option<(String, String)> {
        self.house_primary_how(primary)
    }

    pub(super) fn own_keys_after(&mut self, ui: &mut egui::Ui, now: f64) {
        self.house_keys(ui, now);
    }
}

pub(super) const IMPORT_TABS: &[super::identity::ImportTab] = &[super::identity::ImportTab::Words];

pub(super) const IMPORT_SEAT_ROW: bool = false;

impl Win {
    pub(super) fn own_rail_extra(&self, ui: &mut egui::Ui) -> bool {
        let _ = ui;
        false
    }

    pub(super) fn own_rail_go(&mut self, now: f64) {
        let _ = now;
    }

    pub(super) fn own_finished_words(&self) -> Option<String> {
        None
    }

    pub(super) fn own_wiz_enter(&mut self, step: crate::nav::Step) {
        let _ = step;
    }

    pub(super) fn own_pushed(&self, place: Place, hist: &mut History) {
        let _ = (place, hist);
    }
}
