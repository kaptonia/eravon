use super::*;

#[derive(Clone, Debug)]
pub struct Pay {
    pub ways: Result<Ways, Said>,
    pub fund: String,
    pub open_window: String,
    pub auto_open: String,
    pub buyer_may_open: String,
    pub said: Option<Said>,
    pub payout: String,
    pub fresh: bool,
}

impl Pay {
    pub fn open(data: &str, addr: &str, _now: u64) -> Pay {
        let ways = ways::read(data, addr).map_err(|e| Said::bad(e.name(), e.detail()));
        let (fund, open_window, auto_open, buyer_may_open) = match &ways {
            Ok(w) => (w.kept.clone(), w.open_window.to_string(), w.auto_open.to_string(), w.buyer_may_open.to_string()),
            Err(_) => Default::default(),
        };
        let payout = eravon_vault::payout::get(data, addr).unwrap_or_default();
        Pay { ways, fund, open_window, auto_open, buyer_may_open, said: None, payout, fresh: true }
    }

    pub fn reread_payout(&mut self, data: &str, addr: &str) {
        self.payout = eravon_vault::payout::get(data, addr).unwrap_or_default();
    }

    pub fn save_opening(&mut self, data: &str, addr: &str, now: u64, ok_word: &str, fund_given: bool) {
        let kept = self.ways.as_ref().map(|w| w.kept.clone()).unwrap_or_default();
        let switch = |v: &str, k: &str| -> Result<Option<bool>, Said> {
            match v.trim() {
                "" => Ok(None),
                "true" => Ok(Some(true)),
                "false" => Ok(Some(false)),
                other => Err(Said::bad("E_APP_SWITCH", format!("{k} 只收 true 或 false,给的是 {other:?}"))),
            }
        };
        let edit = (|| -> Result<eravon_payways::ways::Edit, Said> {
            let open_window = match self.open_window.trim() {
                "" => None,
                n => Some(n.parse::<u64>().map_err(|_| Said::bad("E_PAYWAYS_OPEN_WINDOW", format!("开案期限要是秒数,给的是 {n:?}")))?),
            };
            Ok(eravon_payways::ways::Edit {
                auto_open: switch(&self.auto_open, "open-auto")?,
                open_window,
                buyer_may_open: switch(&self.buyer_may_open, "open-buyer")?,
                fund: (!self.fund.trim().is_empty() && (fund_given || !self.fund.trim().eq_ignore_ascii_case(&kept))).then(|| self.fund.trim().to_string()),
                own: None,
            })
        })();
        let edit = match edit {
            Ok(e) => e,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        match eravon_cli::deskops::payways::save(data, seam::WRITER, now, addr, &edit) {
            Ok(saved) => {
                self.ways = Ok(saved.ways);
                self.said = Some(Said::ok(ok_word));
            }
            Err(n) => self.said = Some(Said::bad(&n.name, n.detail())),
        }
    }

    pub fn save_own(&mut self, data: &str, addr: &str, now: u64, on: bool) {
        let edit = eravon_payways::ways::Edit { own: Some(on), ..Default::default() };
        match eravon_cli::deskops::payways::save(data, seam::WRITER, now, addr, &edit) {
            Ok(saved) => {
                self.ways = Ok(saved.ways);
                self.said = Some(Said::ok("ok"));
            }
            Err(n) => self.said = Some(Said::bad(&n.name, n.detail())),
        }
    }

    pub fn rows(&self) -> Out {
        let mut m = vec![
            ("autoOpen", Out::S(self.auto_open.clone())),
            ("buyerMayOpen", Out::S(self.buyer_may_open.clone())),
            ("fund", Out::S(self.fund.clone())),
            ("openWindow", Out::S(self.open_window.clone())),
            ("payout", Out::S(self.payout.clone())),
        ];
        match &self.ways {
            Ok(w) => m.push(("saved", Out::O(ways::rows(w)))),
            Err(s) => m.push(("saved", s.out())),
        }
        if let Some(s) = &self.said {
            m.push(("said", s.out()));
        }
        Out::O(m)
    }
}
