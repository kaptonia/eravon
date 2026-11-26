use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompanyCard {
    pub on: bool,
    pub address: String,
    pub time_check: bool,
    pub gate: Option<Result<CompanyOpen, Said>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanyOpen {
    pub bound: u64,
    pub wallet: String,
    pub ledger: String,
    pub agreement: String,
    pub endpoints: usize,
}

fn wired(data: &str, me: &str) -> Result<(Auto, Table), eravon_cli::deskops::No> {
    wire(data, me).map_err(|e| eravon_cli::deskops::No::new(&e.name, vec![e.detail]))
}

fn said_no(n: eravon_cli::deskops::No) -> Said {
    match n.name.as_str() {
        eravon_cli::deskops::company::R_FORM => bad(&n.name, crate::fill!(crate::house::i18n::Key::SetCoAddrForm; n.detail())),
        _ => bad(&n.name, n.detail()),
    }
}

pub fn company_gate(data: &str, me: &str, address: &str) -> Result<CompanyOpen, Said> {
    eravon_cli::deskops::company::gate(address, || wired(data, me))
        .map(|o| CompanyOpen { bound: o.bound, wallet: o.wallet, ledger: o.ledger, agreement: o.agreement, endpoints: o.endpoints })
        .map_err(said_no)
}

pub fn company_plan(data: &str, me: &str, address: &str) -> Result<(), Said> {
    eravon_cli::deskops::company::plan(address, || wired(data, me)).map_err(said_no)
}

pub fn company_label(c: &CompanyCard) -> Option<&'static str> {
    (c.on && !c.address.is_empty()).then_some("set.companyMode")
}


pub fn kind_key(kind: &str) -> &'static str {
    if kind == "existing" { "set.kindExisting" } else { "set.kindWords" }
}

impl Settings {
    pub fn set_company(&mut self, data: &str, env: &Env, on: bool, address: &str, now: u64, word: &str) {
        let address = address.trim().to_string();
        if on {
            let g = company_gate(data, &self.me, &address);
            if let Err(e) = &g {
                self.said = Some(e.clone());
                self.company.gate = Some(g);
                return;
            }
            if !self.ticket.live() {
                self.said = Some(crate::house::stale());
                return;
            }
            self.said = Some(match eravon_cli::deskops::company::set(data, seam::WRITER, now, &self.me, true, &address) {
                Ok(()) => ok(word),
                Err(n) => said_no(n),
            });
            self.me.clear();
            self.reload(data, env);
            self.company.gate = Some(g);
            return;
        }
        self.said = Some(match eravon_cli::deskops::company::set(data, seam::WRITER, now, &self.me, false, &address) {
            Ok(()) => ok(word),
            Err(n) => said_no(n),
        });
        self.me.clear();
        self.reload(data, env);
        self.company.gate = None;
    }

    pub fn set_time_check(&mut self, data: &str, env: &Env, on: bool, now: u64, word: &str) {
        let gate = self.company.gate.clone();
        self.said = Some(match eravon_cli::deskops::company::time_check(data, seam::WRITER, now, &self.me, on) {
            Ok(()) => ok(word),
            Err(n) => said_no(n),
        });
        self.me.clear();
        self.reload(data, env);
        self.company.gate = gate;
    }

    pub fn company_check(&mut self, data: &str) {
        if self.company.address.is_empty() {
            self.said = Some(bad("E_S9_COMPANY_FORM", crate::house::i18n::t(crate::house::i18n::Key::SetCoAddrEmpty)));
            return;
        }
        let g = company_gate(data, &self.me, &self.company.address);
        self.said = Some(match &g {
            Err(e) => e.clone(),
            Ok(_) => ok(&crate::house::i18n::t(crate::house::i18n::Key::CompanyCheckedSaid)),
        });
        self.company.gate = Some(g);
    }

    pub(super) fn company_out(&self) -> Out {
        let c = &self.company;
        let mut m = vec![
            ("address", Out::S(c.address.clone())),
            ("on", Out::B(c.on)),
            ("timeCheck", Out::B(c.time_check)),
        ];
        match &c.gate {
            None => {}
            Some(Ok(o)) => m.push((
                "gate",
                Out::O(vec![
                    ("agreement", Out::S(o.agreement.clone())),
                    ("calendarBound", Out::N(o.bound)),
                    ("endpoints", Out::N(o.endpoints as u64)),
                    ("ledger", Out::S(o.ledger.clone())),
                    ("open", Out::B(true)),
                    ("wallet", Out::S(o.wallet.clone())),
                ]),
            )),
            Some(Err(e)) => m.push(("gate", Out::O(vec![("detail", Out::S(e.detail.clone())), ("name", Out::S(e.name.clone())), ("open", Out::B(false))]))),
        }
        Out::O(m)
    }
}
