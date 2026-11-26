use super::*;
use eravon_vault::tokens;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenForm {
    pub address: String,
    pub label: String,
    pub manual_for: String,
}

impl TokenForm {
    pub fn manual_at(&self, address: &str) -> bool {
        !self.manual_for.is_empty() && self.manual_for.eq_ignore_ascii_case(address.trim())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSheet {
    pub address: String,
    pub used: Result<Vec<(String, String)>, Said>,
}

fn here(data: &str) -> Vec<eravon_store::settings::TokenRow> {
    tokens::on_chain(&tokens::list(data), tokens::chain())
}

fn refill<T: eravon_chain::transport::Transport>(data: &str, now: u64, t: &T, table: &Table) -> eravon_standard::chain::Filled {
    eravon_standard::chain::refill_tokens(data, seam::WRITER, now, t, table)
}

fn said_no(n: eravon_cli::deskops::No) -> Said {
    let unnamed = || crate::house::i18n::t(crate::house::i18n::Key::GoodsUnnamed);
    match n.name.as_str() {
        tokens::R_IN_USE => {
            let names: Vec<String> = n.facts.iter().skip(1).map(|x| if x.trim().is_empty() { unnamed() } else { x.clone() }).collect();
            bad(tokens::R_IN_USE, crate::fill!(crate::house::i18n::Key::SetTokenTokenInUse; n.facts.first().cloned().unwrap_or_default(), names.join("、")))
        }
        eravon_cli::deskops::tokens::R_FIXED => bad(&n.name, crate::house::i18n::t(crate::house::i18n::Key::SetTokenFixed)),
        name if name.starts_with("token-") => bad(name, crate::house::i18n::t_named(&format!("set.token.{name}"))),
        name => bad(name, n.detail()),
    }
}

impl Settings {
    pub fn read_of(&self, address: &str) -> String {
        self.token_read.iter().find(|(a, _)| a.eq_ignore_ascii_case(address)).map(|(_, w)| w.clone()).unwrap_or_default()
    }

    pub(super) fn take_read(&mut self, read: Vec<(String, String)>) {
        for (address, words) in read {
            match self.token_read.iter_mut().find(|(a, _)| a.eq_ignore_ascii_case(&address)) {
                Some(x) => x.1 = words,
                None => self.token_read.push((address, words)),
            }
        }
    }

    pub(crate) fn read_tokens(&mut self, data: &str) {
        self.tokens = here(data);
    }

    pub fn fill_pending(&mut self, data: &str, now: u64) {
        self.tokens = here(data);
        if self.me.is_empty() || !self.tokens.iter().any(|r| r.source == tokens::FROM_PENDING || r.source == tokens::FROM_MANUAL) {
            return;
        }
        let Ok((t, table)) = network::wire(data, &self.me) else { return };
        let f = refill(data, now, &t, &table);
        self.take_read(f.read);
        if f.changed > 0 {
            self.tokens = here(data);
        }
    }

    pub fn token_add(&mut self, data: &str, address: &str, label: &str, typed: &str, now: u64) {
        self.token_form.address = address.to_string();
        self.token_form.label = label.to_string();
        let manual = self.token_form.manual_at(address).then_some(typed);
        let me = self.me.clone();
        let probe = |at: &str| match network::wire(data, &me) {
            Ok((t, table)) => eravon_standard::chain::token_probe(&t, &table, at),
            Err(e) => eravon_standard::chain::TokenProbe::Unreachable(format!("{}:{}", e.name, e.detail)),
        };
        self.said = Some(match eravon_cli::deskops::tokens::add(data, seam::WRITER, now, address, label, manual, probe) {
            Ok(eravon_cli::deskops::tokens::Added::Manual(at)) => {
                self.token_form.manual_for = at;
                Said { tone: crate::house::seller::Tone::Note, name: "token-decimals-manual".to_string(), detail: crate::house::i18n::t(crate::house::i18n::Key::SetTokenManual) }
            }
            Ok(eravon_cli::deskops::tokens::Added::Row(made)) => {
                let pending = made.source == tokens::FROM_PENDING;
                self.tokens = here(data);
                self.token_form = TokenForm::default();
                ok(crate::fill!(if pending { crate::house::i18n::Key::SetTokenAddedPending } else { crate::house::i18n::Key::SetTokenAdded }; made.label))
            }
            Err(n) => said_no(n),
        });
    }

    pub fn token_open(&mut self, data: &str, address: &str) {
        if tokens::find(&self.tokens, tokens::chain(), address).is_none() {
            self.said = Some(bad(tokens::R_UNKNOWN, crate::house::i18n::t(crate::house::i18n::Key::SetTokenTokenUnknown)));
            return;
        }
        let used = eravon_goods::goods::using(data, address).map_err(|e| bad(e.name(), e.detail()));
        self.token_sheet = Some(TokenSheet { address: address.to_ascii_lowercase(), used });
    }

    pub fn token_close(&mut self) {
        self.token_sheet = None;
    }

    pub fn token_remove(&mut self, data: &str, address: &str, now: u64) {
        self.said = Some(match eravon_cli::deskops::tokens::remove(data, seam::WRITER, now, address) {
            Ok(row) => {
                self.tokens = here(data);
                self.token_sheet = None;
                ok(crate::fill!(crate::house::i18n::Key::SetTokenRemoved; row.label))
            }
            Err(n) => said_no(n),
        });
    }

    pub fn token_decimals(&mut self, data: &str, address: &str, typed: &str, now: u64) {
        let got = eravon_cli::deskops::tokens::decimals(data, seam::WRITER, now, address, typed);
        self.tokens = here(data);
        self.said = Some(match got {
            Ok(_) => ok(crate::house::i18n::t(crate::house::i18n::Key::SetTokenDecimalsSet)),
            Err(n) => said_no(n),
        });
    }

    pub(super) fn tokens_out(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        let rows = self
            .tokens
            .iter()
            .map(|r| {
                Out::O(vec![
                    ("address", s(&r.address)),
                    ("builtin", Out::B(tokens::builtin(r))),
                    ("chainId", Out::N(r.chain)),
                    ("decimals", r.decimals.map_or(Out::Null, |d| Out::N(u64::from(d)))),
                    ("decimalsSource", s(&r.source)),
                    ("label", s(&r.label)),
                ])
            })
            .collect();
        let mut m = vec![
            ("form", Out::O(vec![("address", s(&self.token_form.address)), ("label", s(&self.token_form.label)), ("manual", Out::B(self.token_form.manual_at(&self.token_form.address)))])),
            ("rows", Out::A(rows)),
        ];
        if let Some(sh) = &self.token_sheet {
            let used = match &sh.used {
                Ok(v) => Out::A(v.iter().map(|(who, name)| Out::O(vec![("identity", s(who)), ("product", s(name))])).collect()),
                Err(e) => e.out(),
            };
            let read = self.read_of(&sh.address);
            let mut sheet = vec![("address", s(&sh.address))];
            if !read.is_empty() {
                sheet.push(("read", s(&read)));
            }
            sheet.push(("used", used));
            m.push(("sheet", Out::O(sheet)));
        }
        Out::O(m)
    }
}
