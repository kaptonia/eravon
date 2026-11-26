use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn export_plan(data: &str, addr: &str, export_dir: &str, now: u64, id: &str, plan: &eravon_standard::proposal::Plan, unopened: Option<&str>, confirmed: bool, ticket: &crate::house::Ticket) -> Result<(String, Vec<String>), Said> {
    if export_dir.is_empty() {
        return Err(Said::bad("E_ORDERS_EXPORT_DIR", crate::house::i18n::t(crate::house::i18n::Key::OrdersExportDirPick)));
    }
    let (company, time_on) = company_mode(data, addr).ok_or_else(|| Said::bad("E_ORDERS_COMPANY_OFF", crate::house::i18n::t(crate::house::i18n::Key::OrdersCompanyOff)))?;
    let w = wire(data, addr)?;
    let co = open_company(&w, &company)?;
    if !ticket.live() {
        return Err(crate::house::stale());
    }
    let dir = seam::order_proposal_dir(export_dir, id, plan.verb);
    let path = std::path::Path::new(&dir).join("proposal.json").display().to_string();
    let mut e = eravon_cli::company::Export::of(plan.clone());
    if let Some(k) = unopened {
        e.deal = eravon_cli::company::Deal::Unopened(k.to_string());
    }
    let rows = eravon_cli::company::write(now, &w, &co, &path, time_on, confirmed, e).map_err(|f| match crate::house::ask::of_fault(&f) {
        Some(say) => crate::house::ask::said(&say),
        None => Said::bad(f.name(), f.detail()),
    })?;
    Ok((path, eravon_cli::company::face_of(&rows)))
}

pub(crate) fn open_company(w: &eravon_cli::shared::Wire, company: &str) -> Result<eravon_cli::company::Company, Said> {
    match eravon_standard::company::open(&w.t, &w.table, company) {
        Err(f) => Err(Said::bad(&f.name(), f.detail())),
        Ok(Err(sh)) => Err(Said::bad("E_ORDERS_COMPANY_SHUT", crate::fill!(crate::house::i18n::Key::OrdersCompanyShut; sh.at.to_string(), sh.kind.word().to_string(), sh.why.to_string()))),
        Ok(Ok((views, _))) => Ok(eravon_cli::company::Company { address: company.to_ascii_lowercase(), views }),
    }
}

pub(crate) fn set_export_dir(dir: &mut String, why: &mut Option<Said>, home: &str, data: &str) {
    if !dir.is_empty() {
        return;
    }
    match seam::downloads(home, data) {
        Ok(d) => {
            *dir = d;
            *why = None;
        }
        Err(s) => *why = Some(s),
    }
}

pub(crate) fn exported_say(path: &str, face: &[String]) -> String {
    let _ = path;
    if face.is_empty() { crate::house::i18n::t(crate::house::i18n::Key::OrdersProposalAt) } else { crate::fill!(crate::house::i18n::Key::OrdersProposalAtFace; face.join(" ")) }
}

impl Orders {
    pub(super) fn company_export(&mut self, data: &str, addr: &str, now: u64, id: &str, plan: Result<eravon_standard::proposal::Plan, eravon_watch::Why>, post: crate::house::ask::Post) -> bool {
        match plan {
            Ok(p) => {
                let dir = self.export_dir.clone();
                self.export_now(data, addr, now, id, &dir, p, post, false)
            }
            Err(w) => {
                let s = said(w);
                self.said = Some(s.clone());
                self.exported = Some(Err(s));
                self.seal_next = None;
                false
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn export_now(&mut self, data: &str, addr: &str, now: u64, id: &str, dir: &str, plan: eravon_standard::proposal::Plan, post: crate::house::ask::Post, confirmed: bool) -> bool {
        if let (true, Some(w)) = (dir.is_empty(), &self.export_why) {
            self.said = Some(w.clone());
            self.exported = Some(Err(w.clone()));
            self.seal_next = None;
            return false;
        }
        let unopened = match &post {
            crate::house::ask::Post::Open { buyer } => Some(buyer.clone()),
            _ => None,
        };
        match export_plan(data, addr, dir, now, id, &plan, unopened.as_deref(), confirmed, &self.ticket) {
            Ok((p, face)) => {
                self.ask = None;
                self.said = Some(Said::ok(exported_say(&p, &face)));
                self.exported = Some(Ok(p));
                self.originals = plan.originals.iter().map(|o| o.name.clone()).collect();
                if let (true, Some(s)) = (matches!(plan.verb, "commit" | "claimDelivery"), self.seal_next.take()) {
                    self.sealable = Some(s);
                    self.sealed = None;
                }
                if let crate::house::ask::Post::Settle(of) = post {
                    self.settled(data, addr, now, of);
                }
                true
            }
            Err(e) if e.name == eravon_cli::company::TIME_ASK => {
                self.ask = Some(crate::house::ask::TimeAsk { say: e.detail.clone(), replay: crate::house::ask::Replay::Plan { id: id.to_string(), dir: dir.to_string(), plan, post } });
                self.said = Some(e);
                false
            }
            Err(e) => {
                self.said = Some(e.clone());
                self.exported = Some(Err(e));
                self.seal_next = None;
                false
            }
        }
    }

    pub(super) fn settled(&mut self, data: &str, addr: &str, now: u64, of: eravon_dispute::settle::Offer) {
        let Some(o) = self.order.clone() else { return };
        if let Err(e) = settle::remember(data, seam::WRITER, now, addr, &o, &of) {
            self.said = Some(Said::bad(&e.0, crate::fill!(crate::house::i18n::Key::OrdersOfferNotLogged; e.1.to_string())));
            return;
        }
        let view = OfferView { preview: Err(Said::bad("E_DISPUTE_UNPOSTED", crate::house::i18n::t(crate::house::i18n::Key::OrdersOfferInProposal))), digest: eravon_standard::order::hex_of(&of.digest) };
        if let Some(m) = self.settle.as_mut() {
            m.offer = Some(Ok(view));
        }
    }

    pub fn ask_answer(&mut self, data: &str, addr: &str, now: u64, go: bool) {
        let Some(a) = self.ask.take() else { return };
        if !go {
            self.seal_next = None;
            self.said = Some(Said::ok(crate::house::i18n::t(crate::house::i18n::Key::SaidCanceled)));
            return;
        }
        if let crate::house::ask::Replay::Plan { id, dir, plan, post } = a.replay {
            if self.export_now(data, addr, now, &id, &dir, plan, post, true) {
                self.deliver = None;
                self.card = None;
                self.evidence = None;
            }
        }
    }
}
