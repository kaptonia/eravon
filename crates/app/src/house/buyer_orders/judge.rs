use super::*;

impl BOrders {
    pub fn judge_read(&mut self, data: &str, me: &str, now: u64, sheet: bool) {
        let Some(o) = self.open.as_ref() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenThatFirst)));
            return;
        };
        let read = match self.find(&o.instance) {
            None => Err(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable))),
            Some(r) => same_reading(data, me, r, now),
        };
        if let Some(o) = self.open.as_mut() {
            o.judge = Some(read);
            if sheet {
                o.judge_open = true;
                o.inner = Some(crate::house::orders::Inner::Judge(0));
            }
        }
    }

    pub fn case_judge(&mut self, data: &str, me: &str, now: u64, tab: u8, fresh: bool) {
        let Some(r) = self.open.as_ref().and_then(|o| self.find(&o.instance)).cloned() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenThatFirst)));
            return;
        };
        let Some(o) = self.open.as_mut() else { return };
        if fresh {
            o.bench = None;
        }
        if o.judge.is_none() {
            o.judge = Some(same_reading(data, me, &r, now));
        }
        if o.bench.is_none() {
            let card = o.judge.as_ref().and_then(|j| j.as_ref().ok());
            o.bench = Some(match &r.pack {
                Some(p) => crate::house::case::buyer_bench(data, me, p, &r.row.instance, &r.row.case_id, card, now),
                None => Err(Said::bad("E_P6_NO_PACK", crate::house::i18n::t(crate::house::i18n::Key::BoTermsMissing))),
            });
        }
        o.inner = Some(crate::house::orders::Inner::Judge(tab));
    }

    pub fn case_docs(&mut self, tab: u8) {
        let Some(r) = self.open.as_ref().and_then(|o| self.find(&o.instance)).cloned() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenThatFirst)));
            return;
        };
        let terms = crate::house::case::pack_terms(r.pack.as_deref());
        let papers = crate::house::case::papers(crate::house::case::buyer_sources(r.pack.as_deref().unwrap_or_default(), &terms, &r.row.case_id, &r.row.instance));
        if let Some(o) = self.open.as_mut() {
            o.papers = Some(papers);
            o.inner = Some(crate::house::orders::Inner::Docs(tab));
        }
    }

    pub fn case_lapse(&mut self) -> Result<(), String> {
        let o = self.open.as_mut().ok_or_else(|| "先打开订单那一面".to_string())?;
        match o.bench.as_ref() {
            Some(Ok(b)) if b.quote.is_ok() => {
                o.inner = Some(crate::house::orders::Inner::Lapse);
                Ok(())
            }
            _ => Err("先打开本案判官的报价".to_string()),
        }
    }

    pub fn inner_back(&mut self) -> Option<&'static str> {
        let o = self.open.as_mut()?;
        let was = o.inner?;
        o.inner = was.over();
        if o.inner.is_none() {
            o.judge_open = false;
        }
        Some(was.word())
    }

    pub fn judge_close(&mut self) {
        if let Some(o) = self.open.as_mut() {
            o.judge_open = false;
            o.inner = None;
        }
    }

    pub fn seal_export(&mut self, dir: &str) {
        let Some(o) = self.open.as_mut() else { return };
        let got = match (&o.sealable, &o.judge) {
            (None, _) => Err(Said::bad("E_EVIDENCE_NONE", crate::house::i18n::t(crate::house::i18n::Key::SealNothing))),
            (Some(s), Some(Ok(c))) => crate::house::judge_card::export(c, s, dir),
            (_, Some(Err(e))) => Err(e.clone()),
            (_, None) => Err(Said::bad("E_JUDGE_UNREAD", crate::house::i18n::t(crate::house::i18n::Key::JudgeReadKey))),
        };
        self.said = Some(match &got {
            Ok(x) => Said::ok(crate::house::i18n::filln(crate::house::i18n::Key::SealDone, &[&x.files.len().to_string()])),
            Err(e) => e.clone(),
        });
        o.sealed = Some(got);
    }
}

fn same_reading(data: &str, me: &str, r: &BRow, now: u64) -> Result<crate::house::judge_card::Card, Said> {
    let judge = r.row.chain.as_ref().map(|v| v.judge.clone()).unwrap_or_default();
    frozen_of(r).and_then(|amanah| crate::house::judge_card::read(data, me, &judge, &amanah, now))
}


fn frozen_of(r: &BRow) -> Result<String, Said> {
    let pk = r.pack.as_ref().and_then(|b| eravon_link::pack::pack(b).ok()).ok_or_else(|| Said::bad("E_P6_NO_PACK", crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable)))?;
    let o = eravon_cli::shared::order_of(pk.listing.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev).map_err(|f| Said::bad(&f.name(), f.detail()))?;
    Ok(eravon_standard::order::hex_of(&o.offer.amanah_hash))
}

impl BOrders {
    fn complaint_case(&self) -> Result<(BRow, String, Result<Option<String>, Said>, u64), Said> {
        let r = self.open.as_ref().and_then(|o| self.find(&o.instance)).cloned().ok_or_else(|| Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenThatFirst)))?;
        let a = eravon_cli::deskops::complaint::buyer_about(&r)?;
        Ok((r, a.judge, a.pool, a.chain_id))
    }

    pub fn complaint_open(&mut self, data: &str, me: &str, now: u64) {
        match self.complaint_case() {
            Err(s) => self.said = Some(s),
            Ok((r, judge, guild, _)) => {
                let c = crate::house::complaint::read(data, me, &r.row.instance, &r.row.case_id, &judge, guild, now);
                self.said = None;
                if let Some(o) = self.open.as_mut() {
                    o.complaint = Some(c);
                }
            }
        }
    }

    pub fn complaint_write(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, now: u64, form: &crate::house::complaint::Form) {
        let (r, judge, guild, chain_id) = match self.complaint_case() {
            Ok(x) => x,
            Err(e) => {
                self.said = Some(e);
                return;
            }
        };
        let kept = self.open.as_mut().and_then(|o| o.complaint.take()).filter(|c| c.instance.eq_ignore_ascii_case(&r.row.instance) && c.guild.is_ok());
        let mut c = kept.unwrap_or_else(|| crate::house::complaint::read(data, me, &r.row.instance, &r.row.case_id, &judge, guild, now));
        let got = if !self.ticket.live() {
            Err(crate::house::stale())
        } else {
            match s.key_for(me) {
                Err(e) => Err(Said::bad(e.name(), e.detail())),
                Ok(mut k) => {
                    let case = crate::house::complaint::Case { chain_id, logic: &r.row.logic, me, role: eravon_dispute::complaint::Role::Buyer, now };
                    let got = crate::house::complaint::write(&c, form, &k, &case);
                    crate::house::orders::wipe(&mut k);
                    got
                }
            }
        };
        c.written = Some(got);
        self.said = Some(crate::house::complaint::said_of(&c));
        if let Some(o) = self.open.as_mut() {
            o.complaint = Some(c);
        }
    }
}
