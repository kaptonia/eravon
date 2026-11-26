use super::*;
use crate::house::action::Action as Step;
use crate::house::complaint::{Complaint, Form};
use eravon_dispute::complaint::Kind;

fn kind_word(k: Kind) -> String {
    use crate::house::i18n::Key as K;
    crate::house::i18n::t(match k {
        Kind::Dispatch => K::CpKindDispatch,
        Kind::Pass => K::CpKindPass,
        Kind::AuditInput => K::CpKindAuditInput,
        Kind::Other => K::CpKindOther,
    })
}

fn last_of(p: &str) -> String {
    std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string())
}

impl Win {
    pub(super) fn complaint_start(&mut self) {
        self.shell.desk.complaint = Some(Form::default());
        if let Some(o) = self.shell.desk.sh.seller.orders.as_mut() {
            o.complaint = None;
        }
        if let Some(o) = self.shell.desk.sh.buyer_orders.as_mut().and_then(|b| b.open.as_mut()) {
            o.complaint = None;
        }
        let before = self.shell.desk.sh.toasts.clone();
        if let Err(e) = self.house(crate::house::act::Act::with(Step::ComplaintOpen, &[])) {
            self.shell.desk.complaint = None;
            if self.shell.desk.sh.toasts == before && !before.iter().any(|t| t.head == e) {
                self.shell.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: e, next: String::new(), detail: None });
            }
        }
    }

    pub(super) fn complaint_sheet(&mut self, ctx: &egui::Context, instance: &str, c: Option<&Complaint>, said: Option<&crate::house::seller::Said>, reading: bool) {
        use crate::house::i18n::Key as K;
        let Some(mut form) = self.shell.desk.complaint.clone() else { return };
        let lang = crate::lang::lang();
        let t = |k: K| crate::house::i18n::get(lang, k).0;
        let c = c.filter(|c| c.instance.eq_ignore_ascii_case(instance));
        let written = c.and_then(|c| c.written.as_ref()).and_then(|w| w.as_ref().ok()).cloned();
        let list: &[crate::house::complaint::Guild] = c.and_then(|c| c.guild.as_ref().ok()).map_or(&[], Vec::as_slice);
        let shown = crate::house::complaint::picked(list, &form.guild).cloned();
        let can = !list.is_empty() && written.is_none() && !reading;
        let ready = can && shown.is_some() && !form.text.trim().is_empty() && !form.dir.trim().is_empty();
        let (mut add, mut home, mut drop_at, mut kind_at, mut pick) = (false, false, None::<usize>, None::<(usize, Kind)>, None::<String>);
        let (_, go, back) = Self::sheet_pick(ctx, "order-complaint", eravon_ui::metrics::MODAL_WIDE_W, |ui, keys| {
            Self::sheet_head(ui, &t(K::OrdersComplain), "");
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            let Some(c) = c else {
                match said.filter(|s| s.tone == crate::house::seller::Tone::Bad && !reading) {
                    Some(s) => said_hint(ui, "complaint-said", lang, &s.name, &s.detail, true),
                    None => states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO),
                }
                Self::sheet_keys(keys, &t(K::ToastClose), None);
                return;
            };
            let copied = card::form(ui, |ui, f| {
                let copied = f.info(ui, &t(K::CpJudge), card::Show::Short(&c.judge));
                match &c.guild {
                    Err(s) => {
                        f.info(ui, &t(K::CpTo), card::Show::Mark(mark::Mark::Warn, &crate::house::refusal::line(lang, &s.name, &s.detail)));
                    }
                    Ok(l) if l.is_empty() => {
                        f.info(ui, &t(K::CpTo), card::Show::Mark(mark::Mark::Warn, &t(K::ErrECOMPLAINTNOGUILD)));
                    }
                    Ok(_) => {}
                }
                copied
            });
            tell_copied(ui.ctx(), copied);
            if list.len() > 1 && written.is_none() {
                card::section(ui, &t(K::CpTo), &t(K::CpPickGuild), |ui| {
                    card::form(ui, |ui, f| {
                        for g in list {
                            let on = shown.as_ref().is_some_and(|x| x.address == g.address);
                            let name = g.card.as_ref().ok().and_then(|c| c.name.clone()).unwrap_or_else(|| eravon_orders::table::short(&g.address));
                            if f.row_with(ui, on.then_some(mark::Mark::Ok), &name, &eravon_orders::table::short(&g.address), card::Value::Nav, |_| ()).clicked() {
                                pick = Some(g.address.clone());
                            }
                        }
                    });
                });
            }
            let mut note: Option<String> = None;
            if let Some(g) = &shown {
                let copied = card::form(ui, |ui, f| {
                    let mut copied = None;
                    match &g.card {
                        Err(s) => {
                            copied = f.info(ui, &t(K::CpTo), card::Show::Short(&g.address));
                            f.info(ui, &t(K::GoodsName), card::Show::Mark(mark::Mark::Warn, &format!("{} · {}", t(K::CpCardUnread), crate::house::refusal::line(lang, &s.name, &s.detail))));
                        }
                        Ok(card) if !card.read() => {
                            copied = f.info(ui, &t(K::CpTo), card::Show::Short(&card.address));
                            let why = if card.none() { t(K::CpCardNone) } else { format!("{} · {}", t(K::CpCardUnread), card.card) };
                            f.info(ui, &t(K::GoodsName), card::Show::Mark(mark::Mark::Warn, &why));
                        }
                        Ok(card) => {
                            match &card.name {
                                Some(n) => f.info(ui, &t(K::CpTo), card::Show::Text(n)),
                                None => f.info(ui, &t(K::CpTo), card::Show::Empty(&t(K::CpNoName))),
                            };
                            if let Some(h) = &card.home {
                                if super::casepages::link_row(ui, f, &t(K::CandsHome), h) {
                                    home = true;
                                }
                            }
                            note = card.words.clone().filter(|w| !w.trim().is_empty());
                        }
                    }
                    f.info(ui, &t(K::CandsPointer), card::Show::Copy(&g.pointer)).or(copied)
                });
                tell_copied(ui.ctx(), copied);
            }
            if let Some(n) = &note {
                card::section(ui, &t(K::BoNote), "", |ui| super::casepages::text_box(ui, "complaint-guild-note", n, eravon_ui::house::TEXT_BOX_MAX_H));
            }
            if let Some(p) = &written {
                ui.add_space(eravon_ui::metrics::CARD_GAP);
                states::banner(ui, states::Banner::Ok, &crate::house::complaint::written_say(p), |_| ());
                states::banner(ui, states::Banner::Warn, &t(K::CpSendIt), |_| ());
                states::note_box(ui, &t(K::CpSendHow));
                Self::sheet_keys(keys, &t(K::WizFinish), None);
                return;
            }
            if list.is_empty() {
                Self::sheet_keys(keys, &t(K::ToastClose), None);
                return;
            }
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            field(ui, &t(K::CpText), None, |ui| {
                input::area_hint(ui, &mut form.text, 5, &t(K::CpTextHint));
            });
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            card::section(ui, &t(K::CpFiles), "", |ui| {
                card::form(ui, |ui, f| {
                    if form.files.is_empty() {
                        f.info(ui, &t(K::CpFiles), card::Show::Empty(&t(K::CpFilesNone)));
                    }
                    for (i, (k, p)) in form.files.iter().enumerate() {
                        f.with(ui, &last_of(p), "", |ui| {
                            if key::key(ui, &t(K::KitRemove), Role::Secondary, true).clicked() {
                                drop_at = Some(i);
                            }
                            let names: Vec<String> = Kind::ALL.iter().map(|x| kind_word(*x)).collect();
                            let rows: Vec<eravon_ui::menu::Item> = names.iter().enumerate().map(|(j, l)| eravon_ui::menu::Item::Row(eravon_ui::menu::Row { label: l, check: Some(Kind::ALL[j] == *k), ..Default::default() })).collect();
                            if let Some(j) = eravon_ui::menu::menu_key(ui, &format!("complaint-kind-{i}"), &kind_word(*k), false, eravon_ui::house::DD_MENU_MIN_W, &rows) {
                                kind_at = Some((i, Kind::ALL[j]));
                            }
                        });
                    }
                    add = f.add_row(ui, &t(K::CpAddFile)).clicked();
                });
            });
            if let Some(Err(s)) = &c.written {
                said_hint(ui, "complaint-write-said", lang, &s.name, &s.detail, true);
            }
            Self::sheet_keys_left(keys, &t(K::CfBack), Some((&t(K::CpWrite), Role::Commit, ready)), Some(("order-complaint", crate::platform::Pick::Folder)));
            keys.picked = form.dir.clone();
        });
        if let Some(p) = path_answer(ctx, egui::Id::new("order-complaint-file"), add, crate::platform::Pick::File) {
            if !form.files.iter().any(|(_, x)| *x == p) {
                form.files.push((Kind::Other, p));
            }
        }
        if let Some(p) = Self::sheet_picked(ctx, "order-complaint") {
            form.dir = p;
        }
        if let Some(i) = drop_at {
            form.files.remove(i);
        } else if let Some((i, k)) = kind_at {
            form.files[i].0 = k;
        }
        if let Some(a) = pick {
            form.guild = a;
        }
        if let (true, Some(g)) = (home, &shown) {
            self.open_foreign_of("guild-home", Some(&g.address));
        }
        if back {
            self.shell.desk.complaint = None;
            return;
        }
        self.shell.desk.complaint = Some(form.clone());
        if go == Some(0) && ready {
            let files = crate::house::complaint::files_arg(&form.files);
            let guild = shown.as_ref().map(|g| g.address.clone()).unwrap_or_default();
            self.step(Step::ComplaintWrite, &[("text", &form.text), ("dir", &form.dir), ("files", &files), ("guild", &guild)]);
        }
    }
}
