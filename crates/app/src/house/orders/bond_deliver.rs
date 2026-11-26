use super::*;

pub(super) fn post_of<'a>(key: &Key, addr: &'a str, w: &'a eravon_cli::shared::Wire) -> Option<ShellPost<'a>> {
    key.get().map(|k| ShellPost { key: k, addr, w, sent: None })
}

pub(super) fn locked() -> Said {
    Said::bad("E_VAULT_LOCKED", crate::house::i18n::t(crate::house::i18n::Key::OrdersLockedFirst))
}

pub(super) fn why_said(r: &Result<String, eravon_watch::Why>, what: &str) -> Said {
    match r {
        Ok(tx) => Said::ok(format!("{} · {tx}", sent_say(what))),
        Err((n, d)) => Said::bad(n, d.clone()),
    }
}

pub(super) fn sent_say(what: &str) -> String {
    let key = match what {
        "deploy" => "orders.sent.deploy",
        "expire" => "orders.sent.expire",
        "refund" => "orders.sent.refund",
        "commit" => "orders.sent.commit",
        "settle" => "orders.sent.settle",
        "confirmBySig" => "orders.sent.confirmBySig",
        "claimDelivery" => "orders.sent.claimDelivery",
        "redraw" => "orders.sent.redraw",
        "sweep" => "orders.sent.sweep",
        other => return other.to_string(),
    };
    crate::house::i18n::t_named(key)
}

fn key_for(_company: bool) -> bool {
    true
}


impl Orders {
    pub fn deliver_open(&mut self, data: &str, addr: &str) {
        let Some(o) = self.order.clone() else {
            self.said = Some(self.no_order());
            return;
        };
        let mut m = DeliverModal::default();
        let company = matches!(order_company(data, addr, &o), Ok(Some(_)));
        match wire(data, addr).map_err(|s| (s.name, s.detail)).and_then(|w| deliver::phase(&sw(&w), &o.instance)).and_then(deliver::gate) {
            Ok(()) if key_for(company) => m.key_on = true,
            Ok(()) => {
                m.key_note = "E_DELIVER_COMPANY:公司那一单".to_string();
                m.key_off = Some(Said::bad("E_DELIVER_COMPANY", "公司那一单".to_string()));
            }
            Err((n, d)) => {
                m.key_note = format!("{n}:{d}");
                m.key_off = Some(Said::bad(&n, d));
            }
        }
        self.deliver = Some(m);
    }

    pub fn deliver_pick_begin(&mut self, path: &str) {
        let Some(m) = self.deliver.as_mut() else { return };
        m.path = path.to_string();
        m.items = None;
        m.sheet = None;
        m.reading = !path.is_empty();
    }

    pub fn deliver_pick_read(&mut self, addr: &str) {
        let Some(m) = self.deliver.as_mut() else { return };
        if !m.reading {
            return;
        }
        m.items = Some(deliver::items(&m.path).map_err(said));
        m.reading = false;
        self.deliver_resheet(addr);
    }

    pub fn deliver_note(&mut self, addr: &str, note: &str) {
        let Some(m) = self.deliver.as_mut() else { return };
        m.note = note.to_string();
        self.deliver_resheet(addr);
    }

    pub(super) fn deliver_resheet(&mut self, addr: &str) {
        let Some(o) = self.order.clone() else { return };
        let Some(m) = self.deliver.as_mut() else { return };
        let items = match m.items.clone() {
            Some(i) => i,
            None if m.reading || m.note.trim().is_empty() => {
                m.sheet = None;
                return;
            }
            None => Ok(Vec::new()),
        };
        let (chain_id, logic) = eravon_watch::standard();
        let note = (!m.note.trim().is_empty()).then(|| m.note.clone());
        m.sheet = Some(
            items
                .and_then(|v| deliver::sheet_of(v, self.co.as_deref().unwrap_or(addr), &o, chain_id, logic, note).map_err(said))
                .map(|s| (eravon_standard::order::hex_of(&s.manifest), s.files)),
        );
    }

    pub fn company_open(&mut self, data: &str, addr: &str, now: u64, key: &Key) {
        if !self.planned(data, addr, key, Plan::Open) {
            return;
        }
        let Some(o) = self.order.clone() else { return };
        let co = match order_company(data, addr, &o) {
            Ok(Some(co)) => co,
            Ok(None) => {
                self.said = Some(Said::bad("E_ORDERS_COMPANY_OFF", crate::house::i18n::t(crate::house::i18n::Key::OrdersCompanyOff)));
                return;
            }
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let made = wire(data, addr).map_err(|s| (s.name, s.detail)).and_then(|w| eravon_watch::company::open(&sw(&w), data, addr, &o, &co, now));
        match made {
            Ok((plan, buyer)) => {
                self.company_export(data, addr, now, &o.id, Ok(plan), crate::house::ask::Post::Open { buyer });
            }
            Err(w) => {
                self.company_export(data, addr, now, &o.id, Err(w), crate::house::ask::Post::None);
            }
        }
    }

    pub fn deliver_send(&mut self, data: &str, addr: &str, now: u64, key: &Key) {
        if !self.planned(data, addr, key, Plan::Deliver) {
            return;
        }
        let Some(o) = self.order.clone() else { return };
        let Some(m) = self.deliver.clone() else { return };
        if !m.key_on {
            self.said = Some(m.refusal());
            return;
        }
        let co = match order_company(data, addr, &o) {
            Ok(c) => c,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        if let Some(co) = co {
            let note = (!m.note.trim().is_empty()).then(|| m.note.clone());
            let mut claimed: Option<Vec<u8>> = None;
            let plan = wire(data, addr).map_err(|s| (s.name, s.detail)).and_then(|w| {
                eravon_standard::company::open(&w.t, &w.table, &co).map_err(|f| (f.name(), f.detail()))?.map_err(|sh| ("E_ORDERS_COMPANY_SHUT".to_string(), sh.why))?;
                let s = deliver::sheet(&m.path, &co, &o, w.chain_id, &w.logic, note)?;
                claimed = Some(s.bytes.clone());
                eravon_deliver::company::deliver(&sw(&w), &o, &s)
            });
            self.seal_next = claimed.map(|manifest| crate::house::judge_card::Sealable { origins: eravon_dispute::seal::Origins { kit_root: None, drop: Some(m.path.clone()) }, manifest });
            if self.company_export(data, addr, now, &o.id, plan, crate::house::ask::Post::None) {
                self.deliver = None;
            }
            return;
        }
        let Some(k) = key.get() else {
            self.said = Some(Said::bad("E_VAULT_LOCKED", crate::house::i18n::t(crate::house::i18n::Key::OrdersLockedFirst)));
            return;
        };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let note = (!m.note.trim().is_empty()).then(|| m.note.clone());
        let r = deliver::sheet(&m.path, addr, &o, w.chain_id, &w.logic, note).and_then(|s| {
            let mut p = ShellPost { key: k, addr, w: &w, sent: None };
            deliver::claim(data, seam::WRITER, now, addr, &sw(&w), &o, &s, &m.method, &m.note, &m.path, &mut p).map(|tx| (tx, s.bytes.clone()))
        });
        self.said = Some(match r {
            Ok((tx, manifest)) => {
                self.sealable = Some(crate::house::judge_card::Sealable { origins: eravon_dispute::seal::Origins { kit_root: None, drop: Some(m.path.clone()) }, manifest });
                self.sealed = None;
                self.deliver = None;
                Said::ok(format!("{} · {tx}", sent_say("claimDelivery")))
            }
            Err(e) => said(e),
        });
        self.read_detail(data, addr);
    }

    pub fn deliver_close(&mut self) {
        self.deliver = None;
    }
}
