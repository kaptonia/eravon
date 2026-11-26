use super::*;
use crate::house::action::Action as Step;
use crate::house::judge_card::{Card, Pick};

impl Win {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn seal_card(
        &mut self,
        ui: &mut egui::Ui,
        judge: Option<&Result<Card, crate::house::seller::Said>>,
        sealable: bool,
        sealed: Option<&Result<eravon_dispute::seal::Sealed, crate::house::seller::Said>>,
        read: Step,
        seal: Step,
    ) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let (mut ask_read, mut ask_dir) = (false, false);
        let one = matches!(judge, Some(Ok(c)) if matches!(c.pick, Pick::One(_)));
        let who = match judge {
            Some(Ok(c)) => c.name.clone().unwrap_or_else(|| eravon_orders::table::short(&c.address)),
            _ => t(crate::house::i18n::Key::PlCands),
        };
        card::section(ui, &t(crate::house::i18n::Key::SealSection), "", |ui| {
            card::form(ui, |ui, f| match judge {
                None => {
                    if f.act(ui, &who, "", key::Key::new(&t(crate::house::i18n::Key::JudgeReadKey), Role::Secondary)).clicked() {
                        ask_read = true;
                    }
                }
                Some(Err(e)) => {
                    let _ = f.line(ui, None, &who, &crate::house::refusal::line(lang, &e.name, &e.detail), false, false, c(C::Ink), |_| ());
                }
                Some(Ok(card)) => {
                    let sub = format!("{} · {}", t(crate::house::i18n::Key::JudgeRecipient), crate::house::judge_card::pick_said(card));
                    if sealable {
                        if f.act(ui, &who, &sub, key::Key::new(&t(crate::house::i18n::Key::SealExportKey), Role::Primary).enabled(one)).clicked() {
                            ask_dir = true;
                        }
                    } else {
                        let _ = f.line(ui, None, &who, &sub, false, false, c(C::Ink), |_| ());
                        f.note(ui, &t(crate::house::i18n::Key::SealNothing));
                    }
                }
            });
        });
        match sealed {
            Some(Ok(x)) => eravon_ui::states::okline(ui, Mark::Ok, &crate::house::i18n::filln(crate::house::i18n::Key::SealDone, &[&x.files.len().to_string()])),
            Some(Err(e)) => said_banner(ui, "judge-said-1", lang, &e.name, &e.detail),
            None => {}
        }
        if ask_read {
            self.step(read, &[("sheet", "false")]);
        }
        let id: &'static str = if seal == Step::EvidenceSeal { "seal-export" } else { "bo-seal-export" };
        if ask_dir {
            self.hx.place_open = Some(id);
        }
        let home = self.shell.desk.sh.seller.field.home.clone();
        let data = self.shell.desk.data.clone();
        match self.place_sheet(ui.ctx(), id, &t(crate::house::i18n::Key::SealExportKey), &[(t(crate::house::i18n::Key::PlCands), who.clone())], &t(crate::house::i18n::Key::IdExportGroup), || crate::house::seam::downloads(&home, &data).unwrap_or_default()) {
            Placed::Go(dir) => self.step(seal, &[("dir", &dir)]),
            Placed::Close | Placed::Open => {}
        }
    }
}
