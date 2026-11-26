use super::*;

impl PayLink {
    pub fn company_export(&mut self, data: &str, buyer: &str, registries: &[String], now: u64) {
        self.company_export_as(data, buyer, registries, now, false);
    }

    pub fn ask_answer(&mut self, data: &str, buyer: &str, registries: &[String], now: u64, go: bool) {
        if self.ask.take().is_none() {
            return;
        }
        if go {
            self.company_export_as(data, buyer, registries, now, true);
        } else {
            self.company = None;
        }
    }

    pub(super) fn company_export_as(&mut self, data: &str, buyer: &str, registries: &[String], now: u64, confirmed: bool) {
        self.ask = None;
        self.company = Some((|| {
            self.pay_key().map_err(|w| Said::bad("E_APP_SCRIPT", w))?;
            let Some(Ok(o)) = self.verified() else { return Err(Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_UNOPENED)) };
            let n = self.pick.ok_or_else(|| Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_NO_PICK))?;
            let form = self.form_of().ok_or_else(|| Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_NO_PICK))?;
            if self.export_dir.is_empty() {
                return Err(self.export_why.clone().unwrap_or_else(|| Said::bad("E_P3_EXPORT_DIR", crate::house::i18n::t(crate::house::i18n::Key::OrdersExportDirPick))));
            }
            let (company, time_on) = crate::house::orders::company_mode(data, buyer).ok_or_else(|| Said::bad("E_P3_COMPANY_OFF", crate::house::i18n::t(crate::house::i18n::Key::OrdersCompanyOff)))?;
            let p = k3::listing::listing(&o.listing).map_err(|e| Said::bad(e.token.name(), crate::house::i18n::t(crate::house::i18n::Key::PlLinkBroken)))?;
            let c = eravon_pay::check::Chain::of(data, buyer, registries, now).map_err(|f| Said::bad(&f.token, f.detail))?;
            let views = match eravon_standard::company::open(&c.w.t, &c.w.table, &company) {
                Err(f) => return Err(Said::bad(&f.name(), f.detail())),
                Ok(Err(sh)) => return Err(Said::bad("E_P3_COMPANY_SHUT", crate::fill!(crate::house::i18n::Key::OrdersCompanyShut; sh.at.to_string(), sh.kind.word().to_string(), sh.why.to_string()))),
                Ok(Ok((v, _))) => v,
            };
            if !self.ticket.live() {
                return Err(crate::house::stale());
            }
            let co = eravon_cli::company::Company { address: company, views };
            let dir = crate::house::seam::order_proposal_dir(&self.export_dir, &crate::house::seam::link_tag(&o.listing, n), form.name());
            let path = std::path::Path::new(&dir).join("proposal.json").display().to_string();
            let attachments: Vec<String> = self.held.iter().map(|(_, t)| t.clone()).collect();
            let e = eravon_pay::company::export(
                &eravon_pay::company::CoIn { c: &c, link: &o.link, listing: &o.listing, p: &p, candidate: n, form, co: &co, time_on, path: &path, confirmed, attachments: &attachments },
                crate::house::seam::WRITER,
            )
            .map_err(|f| if f.token == eravon_cli::company::TIME_ASK { crate::house::ask::said(&f.detail) } else { Said::bad(&f.token, f.detail) })?;
            let order_file = crate::house::seam::company_order_file(&dir);
            eravon_store::landing::land_bytes(std::path::Path::new(&order_file), &e.pack).map_err(|t| {
                let said = t.said().map(str::to_string).unwrap_or_else(|| t.code().to_string());
                Said::bad("E_P3_EXPORT_DIR", crate::fill!(crate::house::i18n::Key::PlCantWrite; order_file.to_string(), said))
            })?;
            Ok(CoView {
                path: e.path,
                order_file,
                form: e.form.name(),
                instance: e.instance,
                case_id: e.case_id,
                id: e.id,
                receipt: String::from_utf8_lossy(&e.receipt).into_owned(),
                face: eravon_cli::company::face_of(&e.rows),
                time_short: eravon_cli::company::time_short(&e.rows),
            })
        })());
        if let Some(Err(s)) = &self.company {
            if s.name == eravon_cli::company::TIME_ASK {
                self.ask = Some(crate::house::ask::TimeAsk { say: s.detail.clone(), replay: crate::house::ask::Replay::Paylink });
            }
        }
    }

    pub fn co_open_plan(&self) -> Result<(), Said> {
        match &self.company {
            Some(Ok(_)) => Ok(()),
            _ => Err(Said::bad("E_APP_SCRIPT", crate::house::i18n::t(crate::house::i18n::Key::PlExportFirst))),
        }
    }

    pub fn company_open(&mut self, s: &eravon_vault::session::Session, buyer: &str, registries: &[String], now: u64, id: &str) {
        self.co_open = Some((|| {
            let id = if id.is_empty() {
                match &self.company {
                    Some(Ok(v)) => v.id.clone(),
                    _ => return Err(Said::bad("E_APP_SCRIPT", crate::house::i18n::t(crate::house::i18n::Key::PlExportFirst))),
                }
            } else {
                id.to_string()
            };
            let c = eravon_pay::check::Chain::of(s.data(), buyer, registries, now).map_err(|f| Said::bad(&f.token, f.detail))?;
            let (plan, seller) = eravon_pay::company::open_plan(&c, s, &id).map_err(|f| Said::bad(&f.token, f.detail))?;
            crate::house::orders::export_plan(s.data(), buyer, &self.export_dir, now, &id, &plan, Some(&seller), false, &self.ticket).map(|(path, _)| path)
        })());
    }
}
