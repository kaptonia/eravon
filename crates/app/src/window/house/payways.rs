use super::*;
use crate::house::action::Action as Step;

impl Win {
    pub(super) fn payways_page(&mut self, ui: &mut egui::Ui) {
        let Some(pay) = self.shell.desk.sh.seller.pay.as_mut() else { return };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let mut change = false;
        let copied = card::form(ui, |ui, f| {
            f.row_with(ui, None, &t(crate::house::i18n::Key::PayCrypto), "", card::Value::None, |ui| {
                mark::pill(ui, &t(crate::house::i18n::Key::PayDefault), eravon_ui::palette::Tone::Ok);
            });
            let word = if pay.payout.is_empty() { crate::house::i18n::Key::PaySetPayout } else { crate::house::i18n::Key::ExChange };
            let mut copied = None;
            let _ = f.line(ui, None, &t(crate::house::i18n::Key::SetPayout), "", false, false, c(C::Ink), |ui| {
                change = key::key(ui, &t(word), Role::Secondary, true).clicked();
                if pay.payout.is_empty() {
                    ui.label(egui::RichText::new(t(crate::house::i18n::Key::SetNone)).font(Type::Note.font()).color(c(C::Ink3)));
                } else {
                    copied = super::casepages::copy_mono(ui, &pay.payout);
                }
            });
            f.note(ui, &t(crate::house::i18n::Key::SetPayoutHint));
            copied
        });
        tell_copied(ui.ctx(), copied);
        if change {
            self.shell.desk.pay_payout = Some((String::new(), String::new()));
        }
        self.payout_card(ui.ctx());
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        if let Some(s) = self.shell.desk.sh.seller.pay.as_mut().and_then(|p| p.said.take()) {
            let next = if s.tone != crate::house::seller::Tone::Bad { String::new() } else if s.detail.contains("https") { t(crate::house::i18n::Key::PayHttpsNext) } else { t(crate::house::i18n::Key::SaidNext) };
            self.shell.desk.sh.toast(crate::house::shell::Toast { tone: s.tone, head: s.detail.clone(), next, detail: if s.tone == crate::house::seller::Tone::Bad { Some(s.name.clone()) } else { None } });
            self.shell.desk.flush();
        }
    }

    pub(super) fn payout_card(&mut self, ctx: &egui::Context) {
        let Some((mut a, mut again)) = self.shell.desk.pay_payout.take() else { return };
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(crate::lang::lang(), k).0;
        let (_, save, cancel) = Self::sheet_with(ctx, "pay-payout", eravon_ui::metrics::MODAL_W, |ui, keys| {
            eravon_ui::sheet::title(ui, &t(crate::house::i18n::Key::SetChangePayout), "");
            field(ui, &t(crate::house::i18n::Key::SetPayout), None, |ui| {
                let _ = input::field(ui, &mut a, "0x…", ui.available_width(), input::Look { mono: true, ..input::Look::default() });
            });
            field(ui, &t(crate::house::i18n::Key::SetAgain), None, |ui| {
                let _ = input::field(ui, &mut again, "0x…", ui.available_width(), input::Look { mono: true, ..input::Look::default() });
            });
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::ReadNetSave), Role::Primary, !a.is_empty() && !again.is_empty())));
        });
        if cancel {
            return;
        }
        if !save {
            self.shell.desk.pay_payout = Some((a, again));
            return;
        }
        self.step(Step::SetPayout, &[("a", &a), ("again", &again)]);
        let said = self.shell.desk.sh.seller.pay.as_mut().and_then(|p| p.said.take());
        let bad = said.as_ref().is_some_and(|s| s.tone == crate::house::seller::Tone::Bad);
        if bad {
            self.shell.desk.pay_payout = Some((a, again));
        }
        if let Some(s) = said {
            let next = if s.tone != crate::house::seller::Tone::Bad { String::new() } else if s.detail.contains("https") { t(crate::house::i18n::Key::PayHttpsNext) } else { t(crate::house::i18n::Key::SaidNext) };
            self.shell.desk.sh.toast(crate::house::shell::Toast { tone: s.tone, head: s.detail.clone(), next, detail: if s.tone == crate::house::seller::Tone::Bad { Some(s.name.clone()) } else { None } });
            self.shell.desk.flush();
        }
    }
}
