use super::*;

impl Orders {
    pub fn judge_read(&mut self, data: &str, addr: &str, now: u64, open: bool) {
        let Some(o) = self.need_order() else { return };
        let judge = self.row().and_then(|r| r.chain.as_ref()).map(|v| v.judge.clone()).unwrap_or_default();
        let read = wire(data, addr)
            .and_then(|w| eravon_watch::poll::deal(data, addr, &o, &sw(&w)).map_err(|(n, d)| Said::bad(&n, d)))
            .and_then(|d| crate::house::judge_card::read(data, addr, &judge, &eravon_standard::order::hex_of(&d.o.offer.amanah_hash), now));
        self.judge = Some(read);
        if open {
            self.judge_open = true;
            self.case_judge(data, addr, now, 0);
        }
    }

    pub fn judge_close(&mut self) {
        self.judge_open = false;
        if matches!(self.inner, Some(Inner::Judge(_)) | Some(Inner::Lapse)) {
            self.inner = None;
        }
    }

    pub fn case_judge(&mut self, data: &str, addr: &str, now: u64, tab: u8) {
        let Some(o) = self.need_order() else { return };
        if self.judge.is_none() {
            let judge = self.row().and_then(|r| r.chain.as_ref()).map(|v| v.judge.clone()).unwrap_or_default();
            let read = wire(data, addr)
                .and_then(|w| eravon_watch::poll::deal(data, addr, &o, &sw(&w)).map_err(|(n, d)| Said::bad(&n, d)))
                .and_then(|d| crate::house::judge_card::read(data, addr, &judge, &eravon_standard::order::hex_of(&d.o.offer.amanah_hash), now));
            self.judge = Some(read);
        }
        if self.bench.is_none() {
            let card = self.judge.as_ref().and_then(|j| j.as_ref().ok()).cloned();
            self.bench = Some(crate::house::case::bench(data, addr, &o, card.as_ref(), now));
        }
        self.inner = Some(Inner::Judge(tab));
    }

    pub fn case_docs(&mut self, data: &str, addr: &str, tab: u8) {
        let (Some(o), Some(d)) = (self.order.clone(), self.detail.as_ref()) else {
            self.said = Some(Said::bad("E_ORDERS_NO_RECORD", crate::house::i18n::t(crate::house::i18n::Key::OrdersNoRecord)));
            return;
        };
        self.papers = Some(crate::house::case::papers(crate::house::case::seller_sources(data, addr, &o, &d.overview.terms)));
        self.inner = Some(Inner::Docs(tab));
    }

    pub fn case_lapse(&mut self) -> Result<(), String> {
        match self.bench.as_ref() {
            Some(Ok(b)) if b.quote.is_ok() => {
                self.inner = Some(Inner::Lapse);
                Ok(())
            }
            _ => Err("先打开本案判官的报价".to_string()),
        }
    }

    pub fn inner_back(&mut self) {
        self.inner = self.inner.and_then(Inner::over);
        if self.inner.is_none() {
            self.judge_open = false;
            self.terms_open = false;
        }
    }

    pub fn seal_export(&mut self, dir: &str) {
        let Some(s) = self.sealable.clone() else {
            self.said = Some(Said::bad("E_EVIDENCE_NONE", crate::house::i18n::t(crate::house::i18n::Key::SealNothing)));
            return;
        };
        let got = match &self.judge {
            Some(Ok(c)) => crate::house::judge_card::export(c, &s, dir),
            Some(Err(e)) => Err(e.clone()),
            None => Err(Said::bad("E_JUDGE_UNREAD", crate::house::i18n::t(crate::house::i18n::Key::JudgeReadKey))),
        };
        self.said = Some(match &got {
            Ok(x) => Said::ok(crate::house::i18n::filln(crate::house::i18n::Key::SealDone, &[&x.files.len().to_string()])),
            Err(e) => e.clone(),
        });
        self.sealed = Some(got);
    }
}

pub(crate) fn wipe(k: &mut [u8; 32]) {
    for b in k.iter_mut() {
        *b = 0;
    }
    let _ = std::hint::black_box(&k);
}

impl Orders {
    fn complaint_case(&self, data: &str, addr: &str) -> Result<(Row, String, Result<Option<String>, Said>, u64), Said> {
        let r = self.row().cloned().ok_or_else(|| Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenFirst)))?;
        let a = eravon_cli::deskops::complaint::seller_about(data, addr, &r, self.order.as_ref(), |o| crate::house::orders::order_company(data, addr, o))?;
        Ok((r, a.judge, a.pool, a.chain_id))
    }

    pub fn complaint_open(&mut self, data: &str, addr: &str, now: u64) {
        match self.complaint_case(data, addr) {
            Err(s) => self.said = Some(s),
            Ok((r, judge, guild, _)) => {
                let c = crate::house::complaint::read(data, addr, &r.instance, &r.case_id, &judge, guild, now);
                self.said = None;
                self.complaint = Some(c);
            }
        }
    }

    pub fn complaint_write(&mut self, data: &str, addr: &str, now: u64, key: &Key, form: &crate::house::complaint::Form) {
        let (r, judge, guild, chain_id) = match self.complaint_case(data, addr) {
            Ok(x) => x,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let mut c = match self.complaint.take().filter(|c| c.instance.eq_ignore_ascii_case(&r.instance) && c.guild.is_ok()) {
            Some(c) => c,
            None => crate::house::complaint::read(data, addr, &r.instance, &r.case_id, &judge, guild, now),
        };
        let got = match key.get() {
            None => Err(Said::bad("E_VAULT_LOCKED", crate::house::i18n::t(crate::house::i18n::Key::OrdersLockedFirst))),
            Some(_) if !self.ticket.live() => Err(crate::house::stale()),
            Some(mut k) => {
                let case = crate::house::complaint::Case { chain_id, logic: &r.logic, me: addr, role: eravon_dispute::complaint::Role::Seller, now };
                let got = crate::house::complaint::write(&c, form, &k, &case);
                wipe(&mut k);
                got
            }
        };
        c.written = Some(got);
        self.said = Some(crate::house::complaint::said_of(&c));
        self.complaint = Some(c);
    }
}


