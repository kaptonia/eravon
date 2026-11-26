use super::*;

fn root_of(typed: &str) -> Option<crate::key::Address> {
    let t = typed.trim();
    let hex = t.strip_prefix(eravon_glue::face::MARKER_PREFIX).unwrap_or(t);
    crate::key::Address::parse(hex).filter(|_| hex.len() == 42)
}

fn role_word(role: &str) -> Key {
    match role {
        "eravon-seller" => Key::IdSeatAuthor,
        "eravon-buyer" => Key::IdSeatGrantee,
        _ => Key::SetPayout,
    }
}

impl Win {
    pub(in crate::window) fn house_consent_open(&mut self) {
        self.shell.desk.consent = Some(crate::house::desk::Consent::default());
    }

    pub(in crate::window) fn house_consent(&mut self, ctx: &egui::Context, now: f64) {
        let Some(c) = self.shell.desk.consent.as_ref() else { return };
        let busy = self.shell.tasks.in_flight(crate::task::Kind::Vault);
        let len = crate::keybox::PIN_LEN;
        let shake = self.ux.pin_shake;
        let mine = self.shell.anchor.map(|a| a.hex()).unwrap_or_default();
        let out_text = c.text.clone();
        let trouble = c.trouble.clone();
        let roles = eravon_vault::sign::CONSENT_ROLES;
        let words: Vec<&str> = roles.iter().map(|r| t(role_word(r))).collect();
        let (mut go, mut close) = (false, false);
        let out = sheet::show(
            ctx,
            sheet::Spec::new("set-consent", tk::SHEET_NARROW).shake(shake),
            self,
            |ui, me| {
                let Some(c) = me.shell.desk.consent.as_mut() else { return };
                match &out_text {
                    Some(text) => {
                        sheet::title(ui, t(Key::ConsentDone), "");
                        let mut copied = None;
                        fields::stack(ui, |ui, st| {
                            st.field(ui, t(Key::ConsentText), None, None, |ui, w| copied = card::whole(ui, text, w));
                        });
                        tell_copied(ui.ctx(), copied);
                    }
                    None => {
                        sheet::title(ui, t(Key::SetConsentRow), "");
                        let id_bad = !c.identity.trim().is_empty() && root_of(&c.identity).is_none();
                        let mut pick = None;
                        fields::stack(ui, |ui, st| {
                            st.field(ui, t(Key::ConsentIdentity), None, id_bad.then(|| (t(Key::ConsentIdentityWhy), true)), |ui, w| fields::line(ui, &mut c.identity, t(Key::ConsentIdentityHint), w, fields::Look { mono: true, bad: id_bad, ..Default::default() }));
                            st.field(ui, t(Key::ConsentRole), None, None, |ui, _| {
                                let items: Vec<menu::Item> = words.iter().enumerate().map(|(i, w)| menu::Item::Row(menu::Row { label: w, check: Some(c.role == Some(i)), ..Default::default() })).collect();
                                let label = c.role.and_then(|i| words.get(i).copied()).unwrap_or(t(Key::ExPickKey));
                                pick = menu::menu_key(ui, "set-consent-role", label, true, eravon_ui::house::ROLE_MENU_W, &items);
                            });
                        });
                        if let Some(i) = pick {
                            c.role = Some(i);
                        }
                        card::form(ui, |ui, f| {
                            let _ = f.info(ui, t(Key::IdAddress), card::Show::Mono(&mine));
                        });
                        fields::stack(ui, |ui, st| {
                            let _ = pin_field(st, ui, t(Key::IdPinGate), None, "set-consent-pin", &mut c.pin, shake, !busy);
                        });
                        sheet::foot_note(ui, t(Key::ConsentSaid));
                    }
                }
                if let Some(f) = &trouble {
                    states::err_box(ui, "set-consent-trouble", f.human(), &f.next(), t(Key::U3RawError), &f.shown());
                }
            },
            |ui, me| {
                if out_text.is_some() {
                    close = page::Page::new().primary(ui, t(Key::WizFinish)).1.clicked();
                    return;
                }
                let Some(c) = me.shell.desk.consent.as_ref() else { return };
                let id_bad = root_of(&c.identity).is_none();
                let still = if id_bad && !c.identity.trim().is_empty() { t(Key::ConsentIdentityWhy).to_string() } else { still_needs(&[(id_bad, Key::ConsentIdentity), (c.role.is_none(), Key::ConsentRole), (c.pin.chars() != len, Key::IdPinGate)]) };
                let ready = still.is_empty() && !busy;
                go = page::Pen::new().press_why(ui, t(Key::ConsentGo), ready, &still).clicked() && ready;
                close = key::key(ui, t(Key::CfBack), Role::Secondary, !busy).clicked();
            },
        );
        if close || (out.esc && !busy) {
            self.shell.desk.consent = None;
            return;
        }
        if go {
            let Some(c) = self.shell.desk.consent.as_mut() else { return };
            let Some(role) = c.role.and_then(|i| roles.get(i).copied()) else { return };
            c.trouble = None;
            let a = Action::FaceConsent { identity: c.identity.trim().to_string(), role: role.to_string(), pin: std::mem::take(&mut c.pin) };
            let r = self.act(a, now);
            self.vault_or(VaultSite::Consent, r, now);
        }
    }

    pub(in crate::window) fn house_consent_back(&mut self, r: Applied, now: f64) {
        let Some(c) = self.shell.desk.consent.as_mut() else { return };
        match r {
            Applied::Consented { text } => {
                self.ux.pin_shake = None;
                c.text = Some(text);
            }
            Applied::Trouble(f) => {
                self.ux.pin_shake = Some(now);
                c.pin.clear();
                c.trouble = Some(f);
            }
            _ => {}
        }
    }
}
