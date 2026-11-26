use super::*;
use crate::house::action::Action as Step;
use eravon_ui::sheetparts as parts;

fn choice_row(ui: &mut egui::Ui, f: &mut card::Form, salt: &str, title: &str, sub: &str, on: bool) -> bool {
    let (resp, _) = f.line(ui, None, title, sub, false, false, c(C::Ink), |ui| parts::radio(ui, salt, on));
    let hit = ui.interact(resp.rect, egui::Id::new(("eravon-wiz-choice", salt)), egui::Sense::click());
    let pressed = hit.clicked();
    let _ = hit.on_hover_cursor(egui::CursorIcon::PointingHand);
    pressed
}

pub(in crate::window) fn chain_rows(ui: &mut egui::Ui, f: &mut card::Form, salt: &str, on: u64) -> Option<u64> {
    let mut chose = None;
    for n in eravon_literals::constants::NETWORKS {
        let chain = n.standard.chain_id;
        let title = crate::house::i18n::get_named(crate::lang::lang(), n.word).0;
        if choice_row(ui, f, &format!("{salt}-{}", n.name), &title, "", on == chain) {
            chose = Some(chain);
        }
    }
    chose
}

pub(in crate::window) fn chain_on(chosen: Option<u64>, home: Option<&eravon_vault::settings::Settings>) -> u64 {
    crate::house::first::chain_on(chosen, home)
}

impl Win {
    pub(in crate::window) fn house_wiz_network(&mut self, ui: &mut egui::Ui) {
        let on = chain_on(self.shell.desk.wiz_chain, self.shell.home.as_ref().map(|_| &self.shell.settings));
        let mut chose = None;
        eravon_ui::fields::rows(ui, |ui, f| chose = chain_rows(ui, f, "wiz-network", on));
        if chose.is_some() {
            self.shell.desk.wiz_chain = chose;
        }
        self.first_said(ui);
    }

    pub(in crate::window) fn house_wiz_network_keys(&mut self, ui: &mut egui::Ui) {
        let chain = chain_on(self.shell.desk.wiz_chain, self.shell.home.as_ref().map(|_| &self.shell.settings));
        if page::Page::new().primary_with(ui, &crate::house::i18n::t(crate::house::i18n::Key::WizardNext), !self.first_busy()).1.clicked() {
            self.step(Step::Network, &[("chain", &chain.to_string())]);
        }
    }

    pub(in crate::window) fn house_wiz_role(&mut self, ui: &mut egui::Ui) {
        use crate::house::i18n::Key;
        let waiting = !self.shell.desk.pending_url.is_empty();
        let now = self.shell.desk.wiz_role.unwrap_or(if waiting { crate::house::shell::Seat::Buyer } else { self.shell.desk.seat() });
        let word = |k: Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        if waiting {
            states::note_box(ui, &word(Key::WizLinkWaiting));
        }
        let mut chose = None;
        eravon_ui::fields::rows(ui, |ui, f| {
            for (key, sub, seat) in [(Key::IdSeatAuthor, Key::IdSeatAuthorSub, crate::house::shell::Seat::Seller), (Key::IdSeatGrantee, Key::IdSeatGranteeSub, crate::house::shell::Seat::Buyer)] {
                if choice_row(ui, f, &format!("wiz-role-{}", seat.name()), &word(key), &word(sub), now == seat) {
                    chose = Some(seat);
                }
            }
        });
        if let Some(seat) = chose {
            self.shell.desk.wiz_role = Some(seat);
        }
        self.first_said(ui);
    }

    pub(in crate::window) fn house_wiz_card(&mut self, ui: &mut egui::Ui, st: &mut eravon_ui::fields::Stack, s: crate::nav::SeatStep, role: crate::roles::Role) {
        match s {
            crate::nav::SeatStep::Network => self.house_wiz_network(ui),
            crate::nav::SeatStep::Role => self.house_wiz_role(ui),
            crate::nav::SeatStep::Tail => self.house_wiz_tail(ui, st, role),
        }
    }

    pub(in crate::window) fn own_wiz_card(&mut self, ui: &mut egui::Ui, st: &mut eravon_ui::fields::Stack, o: crate::own::OwnStep, now: f64) {
        let _ = now;
        let role = self.shell.settings.role;
        self.house_wiz_card(ui, st, o, role);
    }

    pub(in crate::window) fn own_wiz_keys(&mut self, ui: &mut egui::Ui, o: crate::own::OwnStep) -> (Option<Action>, bool) {
        let role = self.shell.settings.role;
        self.house_wiz_keys(ui, o, role);
        (None, false)
    }

    pub(in crate::window) fn own_wiz_next(&self, o: crate::own::OwnStep) -> Option<Action> {
        let _ = o;
        None
    }

    pub(in crate::window) fn own_wiz_readable(&self, o: crate::own::OwnStep) -> bool {
        let _ = o;
        false
    }

    pub(in crate::window) fn own_wiz_made(&self, o: crate::own::OwnStep) -> Option<(Key, String)> {
        if o == crate::nav::SeatStep::Network {
            let on = eravon_literals::constants::network_of(eravon_vault::store::chain_of(&self.shell.settings)).unwrap_or_else(eravon_literals::constants::default_network);
            return Some((crate::house::i18n::Key::SetNetwork, crate::house::i18n::get_named(crate::lang::lang(), on.word).0));
        }
        if o != crate::nav::SeatStep::Tail || self.shell.settings.role == crate::roles::Role::Grantee {
            return None;
        }
        let payout = Some(self.shell.desk.ready.payout.clone()).filter(|p| !p.is_empty())?;
        Some((crate::house::i18n::Key::SetPayout, payout))
    }

    pub(in crate::window) fn own_wiz_turn(&mut self, ctx: &egui::Context, step: crate::nav::Step, done: bool, now: f64) -> bool {
        let _ = (ctx, step, done, now);
        false
    }

    pub(in crate::window) fn house_wiz_keys(&mut self, ui: &mut egui::Ui, s: crate::nav::SeatStep, role: crate::roles::Role) {
        match s {
            crate::nav::SeatStep::Network => self.house_wiz_network_keys(ui),
            crate::nav::SeatStep::Role => self.house_wiz_role_keys(ui),
            crate::nav::SeatStep::Tail => self.house_wiz_tail_keys(ui, role),
        }
    }

    pub(in crate::window) fn house_wiz_role_keys(&mut self, ui: &mut egui::Ui) {
        let waiting = !self.shell.desk.pending_url.is_empty();
        let seat = self.shell.desk.wiz_role.unwrap_or(if waiting { crate::house::shell::Seat::Buyer } else { self.shell.desk.seat() });
        if page::Page::new().primary_with(ui, &crate::house::i18n::t(crate::house::i18n::Key::WizardNext), !self.first_busy()).1.clicked() {
            self.step(Step::Role, &[("to", seat.name())]);
        }
    }

    pub(in crate::window) fn house_wiz_tail(&mut self, ui: &mut egui::Ui, st: &mut eravon_ui::fields::Stack, _role: crate::roles::Role) {
        let data = self.shell.desk.data.clone();
        let f = self.shell.desk.first.get_or_insert_with(|| crate::house::first::First::new(&data));
        let (mut v, mut again) = (f.payout.clone(), f.again.clone());
        st.field(ui, &crate::house::i18n::t(crate::house::i18n::Key::SetPayout), None, None, |ui, w| eravon_ui::fields::line(ui, &mut v, crate::house::seam::PAYOUT_PLACEHOLDER, w, eravon_ui::fields::Look::default()));
        st.field(ui, &crate::house::i18n::t(crate::house::i18n::Key::SetAgain), None, None, |ui, w| eravon_ui::fields::line(ui, &mut again, crate::house::seam::PAYOUT_PLACEHOLDER, w, eravon_ui::fields::Look::default()));
        if v != f.payout {
            f.set_payout(&v);
        }
        if again != f.again {
            f.set_again(&again);
        }
        self.first_said(ui);
    }

    pub(in crate::window) fn house_wiz_tail_keys(&mut self, ui: &mut egui::Ui, _role: crate::roles::Role) {
        let busy = self.first_busy();
        let filled = self.shell.desk.first.as_ref().is_some_and(|f| !f.payout.trim().is_empty() && !f.again.trim().is_empty());
        if page::Page::new().primary_with(ui, &crate::house::i18n::t(crate::house::i18n::Key::WizardNext), filled && !busy).1.clicked() {
            self.step(Step::Payout, &[]);
        }
    }

    fn first_said(&mut self, ui: &mut egui::Ui) {
        let says = self.shell.desk.first.as_ref().map(|f| f.line(crate::lang::lang())).unwrap_or_default();
        if !says.is_empty() {
            states::err_box(ui, "wiz-house", &says, "", "", "");
        }
        if self.first_busy() {
            let line = crate::house::i18n::t(crate::house::i18n::Key::VaultBusy);
            ui.vertical_centered(|ui| {
                eravon_ui::mark::mark_line(ui, eravon_ui::mark::Mark::Busy, &line, eravon_ui::tokens::Type::Small, c(C::Ink2));
            });
        }
    }

    pub(super) fn first_busy(&self) -> bool {
        self.shell.house_in_flight(crate::task::Kind::First)
    }
}
