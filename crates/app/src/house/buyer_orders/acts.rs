use super::*;

pub(super) fn keep_part(data: &str, r: &BRow, name: &str, bytes: Vec<u8>, now: u64) -> Result<(), eravon_store::fault::Fault> {
    let id = if r.record.is_empty() { eravon_import::import::record_id(&r.row.case_id) } else { r.record.clone() };
    let st = eravon_vault::store::open_write(data, crate::house::seam::WRITER, now)?;
    eravon_cli::buyer::keep_part_in(&st, &r.owner, &id, &r.row.case_id, &r.row.instance, name, bytes, now)
}

pub fn offer_text(data: &str, r: &BRow) -> Option<String> {
    if r.record.is_empty() {
        return None;
    }
    let st = eravon_vault::store::open_read(data).ok()?;
    st.part(&r.owner, Box_::Packs, &r.record, OFFER_PART).ok().and_then(|b| String::from_utf8(b).ok())
}

pub(super) use eravon_cli::deskops::buyerpage::{key_party, E_READONLY};

pub const E_DEPOSIT_SHORT: &str = "E_P6_DEPOSIT_SHORT";

pub(super) fn open_case(w: &eravon_cli::shared::Wire, b: &act::By<'_>, r: &BRow) -> eravon_after::fault::R<String> {
    use eravon_after::fault::Fault;
    key_party(r).map_err(|note| Fault::new(E_READONLY, note))?;
    let pk = eravon_link::pack::pack(r.pack.as_ref().ok_or_else(|| Fault::new("E_P6_NO_PACK", crate::house::i18n::t(crate::house::i18n::Key::BoTermsMissing)))?)?;
    let (p, rc) = (&pk.listing, &pk.receipt);
    eravon_standard::chain::listing_payable(&p.logic)?;
    let o = eravon_cli::shared::order_of(p.terms_text.as_bytes(), rc.candidate, &rc.buyer_key, &rc.buyer_payout, rc.open_by, &rc.prev)?;
    let n = rc.candidate as usize;
    let sig_seller = eravon_standard::order::bytes_of("sigSeller", p.candidate_sig(n)).unwrap_or_default();
    let sig_buyer = eravon_watch::poll::sig_bytes(&rc.sig).map_err(|(n, d)| Fault::new(n, d))?;
    let guild = n.checked_sub(p.judges.len()).and_then(|g| p.guilds.get(g)).map(|g| (g.address.to_ascii_lowercase(), rc.seq.unwrap_or(0)));
    let now = eravon_standard::precheck::chain_now(&w.t, &w.table)?;
    let opened = match eravon_cli::seller::open_calls(&eravon_cli::seller::OpenIn { w, o: &o, sig_seller: &sig_seller, sig_buyer: &sig_buyer, from: b.me, guild: guild.as_ref().map(|(g, s)| (g.as_str(), *s)), now }) {
        Ok(x) => x,
        Err(eravon_cli::fault::Fault::Down { token, .. }) if token == eravon_cli::seller::E_OPEN_OPENED => return Ok(String::new()),
        Err(e) => return Err(match eravon_cli::buyer::deposit_short_of(&e) {
            Some(why) => Fault::new(E_DEPOSIT_SHORT, why),
            None => e.into(),
        }),
    };
    let tag = eravon_wallet::tx::Tag::of(eravon_wallet::tx::Kind::Open, &opened.instance);
    let k = eravon_wallet::tx::Keep { data: b.s.data(), writer: b.writer, owner: b.me };
    let send = |call: &eravon_standard::call::Call| -> eravon_after::fault::R<String> {
        if !(b.still)() {
            let s = crate::house::stale();
            return Err(Fault::new(s.name, s.detail));
        }
        let key = || b.s.own_key(b.me, b.me).map_err(|e| eravon_cli::fault::Fault::Arg(format!("{}:{}", e.name(), e.detail())));
        eravon_wallet::tx::post(&k, w, b.me, &key, &eravon_cli::send::Opts::default(), call, "0", &tag).map_err(|f| Fault::new(f.token, f.detail))
    };
    let mut done: Vec<String> = Vec::new();
    if let Some(dep) = &opened.deploy {
        done.push(send(dep)?);
    }
    match send(&opened.init) {
        Ok(tx) => {
            done.push(tx);
            Ok(done.join(" · "))
        }
        Err(e) if done.is_empty() => Err(e),
        Err(e) => Err(Fault::new(e.token, format!("{} · {}", e.detail, done.join(" · ")))),
    }
}

pub(super) fn kits_of(data: &str, me: &str, home: &str) -> Result<Vec<(String, String, u64)>, Said> {
    zk_of(data, me).and_then(|(linked, manual)| crate::house::settings::zk_kits(home, &manual, linked))
}

pub(super) fn zk_of(data: &str, me: &str) -> Result<(bool, String), Said> {
    eravon_cli::deskops::network::zikaron_of(data, me).map_err(|n| Said::bad(&n.name, n.detail()))
}

impl BOrders {
    pub fn confirm_open(&mut self, data: &str, me: &str) {
        let c = self.with_open(data, me, |w, r| act::confirm_card(w, &r.case()).map_err(bad));
        if let (Some(o), Some(c)) = (self.open.as_mut(), c) {
            o.card = Some(c);
        }
    }

    pub fn refund_open(&mut self, data: &str, me: &str, home: &str) {
        let c = self.with_open(data, me, |w, r| act::refund_view(w, &r.case()).map_err(bad));
        let first = self.open_company().then(|| kits_of(data, me, home));
        if let (Some(o), Some(c)) = (self.open.as_mut(), c) {
            o.refund = Some(c);
            o.first_kits = first;
        }
    }

    pub fn owed_open(&mut self, data: &str, me: &str, token: Option<&str>) {
        let c = self.with_open(data, me, |w, r| act::owed(w, &r.case(), token).map_err(bad));
        if let (Some(o), Some(c)) = (self.open.as_mut(), c) {
            o.owed = Some(c);
        }
    }

    pub fn offer_paste(&mut self, data: &str, me: &str, text: &str) {
        let c = self.with_open(data, me, |w, r| (act::offer(w, &r.case(), text).map_err(bad), r.clone(), eravon_standard::precheck::chain_now(&w.t, &w.table)));
        let Some((c, r, now)) = c else { return };
        if c.is_ok() {
            let kept = now.map_err(|e| (e.name(), e.detail())).and_then(|at| keep_part(data, &r, OFFER_PART, text.trim().as_bytes().to_vec(), at).map_err(|e| (e.name().to_string(), e.detail())));
            if let Err((n, d)) = kept {
                self.said = Some(Said::bad(&n, crate::fill!(crate::house::i18n::Key::BoOfferNotKept; d.to_string())));
            }
        }
        if let Some(o) = self.open.as_mut() {
            o.tab = Tab::Dispute;
            o.offer = Some(c);
        }
    }

    pub fn offers_scan(&mut self, data: &str, me: &str) {
        let c = self.with_open(data, me, |w, r| {
            let case = r.case();
            let list = act::offers(w, &case).map_err(bad)?;
            let mut last: Option<Said> = None;
            for (_, st, _) in list.into_iter().rev() {
                match act::offer_of(w, &case, st) {
                    Ok(o) => return Ok(o),
                    Err(f) => last = last.or(Some(bad(f))),
                }
            }
            Err(last.unwrap_or_else(|| Said::bad("E_APP_SETTLE_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoSettleNone))))
        });
        if let (Some(o), Some(c)) = (self.open.as_mut(), c) {
            o.tab = Tab::Dispute;
            o.offer = Some(c);
        }
    }

    pub fn settle_plan(text: &str) -> Result<(), Said> {
        eravon_link::transport::decode_link(text.trim().as_bytes()).map(|_| ()).map_err(|_| Said::bad("E_APP_SETTLE_LINK", crate::house::i18n::t(crate::house::i18n::Key::BoSettleLinkBad)))
    }

    fn settle_instance(data: &str, me: &str, text: &str) -> Result<String, Said> {
        crate::house::asked(Self::settle_plan(text))?;
        let (link, _) = eravon_link::transport::decode_link(text.trim().as_bytes()).map_err(|_| Said::bad("E_APP_SETTLE_LINK", crate::house::i18n::t(crate::house::i18n::Key::BoSettleLinkBad)))?;
        let w = crate::house::orders::wire(data, me)?;
        match eravon_cli::deskops::settle::offer_by_link(&w, &link) {
            Ok(st) => Ok(st.instance.to_ascii_lowercase()),
            Err(eravon_cli::fault::Fault::Down { layer: "K3" | "P6", .. }) => Err(Said::bad("E_APP_SETTLE_LINK", crate::house::i18n::t(crate::house::i18n::Key::BoSettleLinkBad))),
            Err(f) => Err(Said::bad(f.name(), f.detail())),
        }
    }

    pub fn land_settle(&mut self, data: &str, me: &str, text: &str) {
        let inst = match Self::settle_instance(data, me, text) {
            Ok(i) => Some(i),
            Err(e) => {
                self.said = Some(e);
                return;
            }
        };
        match inst.filter(|i| self.find(i).is_some()) {
            Some(i) => {
                self.open_row(data, me, &i, Tab::Dispute);
                self.offer_paste(data, me, text);
            }
            None => self.said = Some(Said::bad("E_APP_SETTLE_ORDER", crate::house::i18n::t(crate::house::i18n::Key::BoSettleNoOrder))),
        }
    }

    pub fn plan(&self, data: &str, a: &str, text: &str) -> Result<(), Said> {
        let o = self.open.as_ref().ok_or_else(|| Said::bad("E_P5_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenFirst)))?;
        if let Some(r) = self.find(&o.instance) {
            eravon_standard::chain::past_gate(r.row.past).map_err(|f| Said::bad(&f.name(), f.detail()))?;
            if r.row.past && eravon_after::act::PAST_LACKS_ACTS.contains(&act_of(a)) {
                return Err(Said::bad(eravon_after::act::PAST_LACKS, crate::house::i18n::t(crate::house::i18n::Key::ErrEP6PASTLACKS)));
            }
        }
        let _ = (o, text);
        if self.open_company() && matches!(a, "confirmSend" | "disputeSend" | "disputePull" | "commitSend" | "settleSign" | "settleCall" | "openSend") {
            if self.export_dir.is_empty() {
                return Err(self.export_why.clone().unwrap_or_else(|| Said::bad("E_ORDERS_EXPORT_DIR", crate::house::i18n::t(crate::house::i18n::Key::OrdersExportDirPick))));
            }
            crate::house::orders::company_mode(data, &self.me).ok_or_else(|| Said::bad("E_ORDERS_COMPANY_OFF", crate::house::i18n::t(crate::house::i18n::Key::OrdersCompanyOff)))?;
        }
        Ok(())
    }

    pub fn fresh_for(&self, me: &str) -> bool {
        self.me.eq_ignore_ascii_case(me)
    }

    pub fn take_readings(&mut self, fresh: BOrders) {
        self.me = fresh.me;
        self.rows = fresh.rows;
        self.unscanned = fresh.unscanned;
        self.owners = fresh.owners;
    }

    pub fn reread_open(&mut self, data: &str, me: &str) {
        if let Some(o) = self.open.clone() {
            if let (Ok(w), Some(Ok(rows))) = (crate::house::orders::wire(data, me), self.rows.as_mut())
                && let Some(r) = rows.iter_mut().find(|r| r.row.instance.eq_ignore_ascii_case(&o.instance))
                && let owner = r.owner.clone()
                && eravon_orders::table::reread(data, &owner, eravon_scan::scan::Side::Buyer, &crate::house::orders::sw(&w), &mut r.row).is_ok()
            {
                let at = eravon_standard::precheck::chain_now(&w.t, &w.table).ok();
                r.budget = budget_of(&w, r, at);
            }
        }
        self.reopen(data, me);
    }

    pub fn reopen(&mut self, data: &str, me: &str) {
        if let Some(o) = self.open.clone() {
            let keep = (o.offer.clone(), o.card.clone(), o.refund.clone(), o.owed.clone(), o.kits.clone(), o.first_kits.clone());
            let sealing = (o.judge.clone(), o.judge_open, o.sealable.clone(), o.sealed.clone());
            self.open_row(data, me, &o.instance, o.tab);
            if let Some(x) = self.open.as_mut() {
                (x.offer, x.card, x.refund, x.owed, x.kits, x.first_kits) = keep;
                (x.judge, x.judge_open, x.sealable, x.sealed) = sealing;
                x.complaint = o.complaint;
            }
        }
    }

    pub fn open_row_shape(&self) -> Option<String> {
        let o = self.open.as_ref()?;
        self.row_of(&o.instance).map(|r| format!("{:?}{:?}", r.row, r.budget))
    }

    pub fn open_follow(&self) -> Option<(String, Option<String>)> {
        Some((self.open.as_ref()?.instance.clone(), self.open_row_shape()))
    }

    pub fn read_whole(&self) -> bool {
        matches!(self.rows, Some(Ok(_))) && self.unscanned.is_none()
    }

    pub fn act(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, what: &str, run: impl FnOnce(&eravon_cli::shared::Wire, &BRow, &act::By<'_>) -> eravon_after::fault::R<String>) {
        if !self.ticket.live() {
            self.said = Some(crate::house::stale());
            return;
        }
        let t = self.ticket.clone();
        let still = move || t.live();
        let b = act::By { s, me, writer: crate::house::seam::WRITER, still: &still };
        let r = self.with_open(data, me, |w, r| run(w, r, &b));
        match r {
            Some(Ok(tx)) => {
                let done = Said::ok(format!("{what} · {tx}"));
                self.sent_close();
                self.reread_open(data, me);
                self.said = Some(done);
            }
            Some(Err(f)) => self.said = Some(bad(f)),
            None => {}
        }
    }

    pub fn cancel(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, now: u64) {
        let mut kept = Ok(());
        self.act(s, data, me, &crate::house::i18n::t(crate::house::i18n::Key::BoCancelled), |w, r, b| {
            act::cancel(w, b, &r.case()).map(|c| {
                kept = keep_end(data, r, eravon_watch::order::State::Cancelled, now);
                let deployed = if c.deploy.is_empty() { String::new() } else { format!(" · {}", c.deploy) };
                format!("{}{deployed}", c.cancel)
            })
        });
        if let (Err(e), Some(done)) = (kept, self.said.clone().filter(|x| x.tone == crate::house::seller::Tone::Ok)) {
            self.said = Some(Said::bad(e.name(), crate::fill!(crate::house::i18n::Key::BoCancelledNotKept; done.detail, e.detail())));
        }
    }

    pub fn sweep(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, token: Option<&str>) {
        let mut got = None;
        self.act(s, data, me, "", |w, r, b| {
            let (tx, o) = act::sweep(w, b, &r.case(), token)?;
            got = Some((tx.clone(), o, r.row.decimals));
            Ok(tx)
        });
        if let (Some((tx, o, decimals)), true) = (got, self.said.as_ref().is_some_and(|x| x.tone == crate::house::seller::Tone::Ok)) {
            let amount = crate::house::coin::amount(crate::lang::lang(), &o.amount, decimals, &o.token, crate::house::coin::Form::Short);
            self.said = Some(Said::ok(crate::fill!(crate::house::i18n::Key::BoSweepSaid; amount, eravon_orders::table::short(&o.payout), tx)));
        }
    }

    pub fn confirm_hand(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, now: u64) {
        if !self.ticket.live() {
            self.said = Some(crate::house::stale());
            return;
        }
        let t = self.ticket.clone();
        let still = move || t.live();
        let b = act::By { s, me, writer: crate::house::seam::WRITER, still: &still };
        match self.with_open(data, me, |w, r| act::confirm_hand_out(w, &b, &r.case(), now).map_err(bad)) {
            Some(Ok(h)) => {
                self.reread_open(data, me);
                if let Some(o) = self.open.as_mut() {
                    o.card = None;
                    o.handed = Some(h);
                }
                self.said = Some(Said::ok(crate::house::i18n::t(crate::house::i18n::Key::OrdersWordConfirmHanded)));
            }
            Some(Err(e)) => self.said = Some(e),
            None => {}
        }
    }

    pub fn open_case(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str) {
        if let Err(a) = eravon_cands::wire::registries(&crate::house::orders::injected()) {
            self.said = Some(Said::bad(&a.name(), a.detail()));
            return;
        }
        let mut already = false;
        self.act(s, data, me, &crate::house::i18n::t(crate::house::i18n::Key::OrdersSentDeploy), |w, r, b| {
            let tx = open_case(w, b, r)?;
            already = tx.is_empty();
            Ok(tx)
        });
        if already && self.said.as_ref().is_some_and(|x| x.tone == crate::house::seller::Tone::Ok) {
            self.said = Some(Said { tone: crate::house::seller::Tone::Note, name: eravon_cli::seller::E_OPEN_OPENED.into(), detail: crate::house::i18n::t(crate::house::i18n::Key::ErrEOPENOPENED) });
        }
    }

    pub fn expire(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, now: u64) {
        let t = self.ticket.clone();
        let still = move || t.live();
        let got = self.with_open(data, me, |w, r| act::expire(w, &act::By { s, me, writer: crate::house::seam::WRITER, still: &still }, &r.case()).map(|x| (x, r.clone())));
        match got {
            Some(Ok((x, r))) => {
                let tx = [x.deploy.as_str(), x.init.as_str()].into_iter().filter(|t| !t.is_empty()).collect::<Vec<_>>().join(" · ");
                let kept = keep_end(data, &r, eravon_watch::order::State::Expired, now);
                let amount = crate::house::coin::amount(crate::lang::lang(), &x.amount, r.row.decimals, &x.token, crate::house::coin::Form::Short);
                self.said = Some(match kept {
                    Ok(_) => Said::ok(crate::fill!(crate::house::i18n::Key::BoExpired; amount, eravon_orders::table::short(&x.payout), tx)),
                    Err(e) => Said::bad(e.name(), crate::fill!(crate::house::i18n::Key::BoExpiredNotKept; tx, e.detail())),
                });
                self.reread_open(data, me);
            }
            Some(Err(f)) => self.said = Some(bad(f)),
            None => {}
        }
    }

    pub fn evidence_open(&mut self, data: &str, me: &str, home: &str) {
        let kits = kits_of(data, me, home);
        if let Some(o) = self.open.as_mut() {
            o.tab = Tab::Dispute;
            o.kits = Some(kits);
        }
    }

    pub fn commit(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, home: &str, drop: Option<&str>, kit: Option<&str>, note: &str, now: u64) {
        let (linked, manual) = match zk_of(data, me) {
            Ok(x) => x,
            Err(e) => {
                self.said = Some(e);
                return;
            }
        };
        let kit = if linked { kit } else { None };
        let t = self.ticket.clone();
        let got = self.with_open(data, me, |w, r| -> eravon_after::fault::R<(String, Vec<u8>, BRow)> {
            let (items, source) = eravon_dispute::evidence::items(home, Some(&manual), kit, drop)?;
            let still = || t.live();
            let (tx, ev) = act::commit(w, &act::By { s, me, writer: crate::house::seam::WRITER, still: &still }, &r.case(), items, source, (!note.trim().is_empty()).then(|| note.to_string()))?;
            Ok((tx, ev.bytes, r.clone()))
        });
        match got {
            Some(Ok((tx, bytes, r))) => {
                let sealable = sealable_of(home, &manual, kit, drop, &bytes);
                let kept = keep_part(data, &r, &format!("evidence-{}.json", &tx.trim_start_matches("0x")[..8.min(tx.len().saturating_sub(2))]), bytes, now);
                self.said = Some(match kept {
                    Ok(_) => Said::ok(crate::house::i18n::t(crate::house::i18n::Key::OrdersSentCommit)),
                    Err(e) => Said::bad(e.name(), crate::fill!(crate::house::i18n::Key::BoEvidenceNotKept; tx, e.detail())),
                });
                self.sent_close();
                self.reread_open(data, me);
                if let Some(o) = self.open.as_mut() {
                    o.sealable = sealable;
                    o.sealed = None;
                }
            }
            Some(Err(f)) => self.said = Some(bad(f)),
            None => {}
        }
    }

    fn sent_close(&mut self) {
        if let Some(o) = self.open.as_mut() {
            o.card = None;
            o.refund = None;
            o.owed = None;
            o.kits = None;
            o.first_kits = None;
        }
    }

    pub fn open_company(&self) -> bool {
        self.open.as_ref().and_then(|o| self.find(&o.instance)).is_some_and(|r| r.access == Access::Company)
    }

    pub fn co_act(&mut self, data: &str, me: &str, now: u64, run: impl FnOnce(&eravon_cli::shared::Wire, &BRow) -> eravon_after::fault::R<eravon_standard::proposal::Plan>) {
        self.co_other = None;
        let got = self.with_open(data, me, |w, r| (run(w, r).map_err(bad), eravon_import::import::record_id(&r.row.case_id)));
        let Some((plan, id)) = got else {
            self.seal_next = None;
            return;
        };
        match plan {
            Ok(p) => {
                let dir = self.export_dir.clone();
                self.export_now(data, me, now, &id, &dir, p, false);
            }
            Err(e) => {
                self.said = Some(e.clone());
                self.exported = Some(Err(e));
                self.seal_next = None;
            }
        }
    }

    pub fn co_open(&mut self, data: &str, me: &str, now: u64) {
        self.co_other = None;
        let got = self.with_open(data, me, |w, r| (eravon_after::company::open(w, &r.case(), data, me).map_err(bad), eravon_import::import::record_id(&r.row.case_id)));
        let Some((plan, id)) = got else {
            self.seal_next = None;
            return;
        };
        match plan {
            Ok((p, other)) => {
                self.co_other = Some(other);
                let dir = self.export_dir.clone();
                self.export_now(data, me, now, &id, &dir, p, false);
            }
            Err(e) => {
                self.said = Some(e.clone());
                self.exported = Some(Err(e));
                self.seal_next = None;
            }
        }
    }

    pub fn co_expire(&mut self, data: &str, me: &str, now: u64) {
        self.co_other = None;
        let got = self.with_open(data, me, |w, r| (eravon_after::company::expire(w, &r.case()).map_err(bad), eravon_import::import::record_id(&r.row.case_id)));
        let Some((plan, id)) = got else {
            self.seal_next = None;
            return;
        };
        match plan {
            Ok((p, other)) => {
                self.co_other = Some(other);
                let dir = self.export_dir.clone();
                self.export_now(data, me, now, &id, &dir, p, false);
            }
            Err(e) => {
                self.said = Some(e.clone());
                self.exported = Some(Err(e));
                self.seal_next = None;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn export_now(&mut self, data: &str, me: &str, now: u64, id: &str, dir: &str, plan: eravon_standard::proposal::Plan, confirmed: bool) {
        if let (true, Some(w)) = (dir.is_empty(), &self.export_why) {
            self.said = Some(w.clone());
            self.exported = Some(Err(w.clone()));
            self.seal_next = None;
            return;
        }
        match crate::house::orders::export_plan(data, me, dir, now, id, &plan, self.co_other.as_deref(), confirmed, &self.ticket) {
            Ok((p, face)) => {
                self.ask = None;
                self.said = Some(Said::ok(crate::house::orders::exported_say(&p, &face)));
                self.exported = Some(Ok(p));
                let next = if matches!(plan.verb, "commit" | "dispute") { self.seal_next.take() } else { None };
                if let Some(o) = self.open.as_mut() {
                    if let Some(s) = next {
                        o.sealable = Some(s);
                        o.sealed = None;
                    }
                    o.card = None;
                    o.refund = None;
                    o.first_kits = None;
                    o.kits = None;
                    o.offer = None;
                }
            }
            Err(e) if e.name == eravon_cli::company::TIME_ASK => {
                self.ask = Some(crate::house::ask::TimeAsk { say: e.detail.clone(), replay: crate::house::ask::Replay::Plan { id: id.to_string(), dir: dir.to_string(), plan, post: crate::house::ask::Post::None } });
                self.said = Some(e);
            }
            Err(e) => {
                self.said = Some(e.clone());
                self.exported = Some(Err(e));
                self.seal_next = None;
            }
        }
    }

    pub fn ask_answer(&mut self, data: &str, me: &str, now: u64, go: bool) {
        let Some(a) = self.ask.take() else { return };
        if !go {
            self.seal_next = None;
            self.said = Some(Said::ok(crate::house::i18n::t(crate::house::i18n::Key::SaidCanceled)));
            return;
        }
        if let crate::house::ask::Replay::Plan { id, dir, plan, .. } = a.replay {
            self.export_now(data, me, now, &id, &dir, plan, true);
        }
    }

    pub fn co_dispute(&mut self, data: &str, me: &str, now: u64, form: eravon_after::company::Form, first: Option<&eravon_dispute::evidence::Evidence>) {
        let wallet = crate::house::orders::wire(data, me).and_then(|w| match company_of(&w, data, me) {
            Some(Ok(c)) => Ok(c.views.wallet),
            Some(Err(s)) => Err(s),
            None => Err(Said::bad("E_ORDERS_COMPANY_OFF", crate::house::i18n::t(crate::house::i18n::Key::OrdersCompanyOff))),
        });
        match wallet {
            Ok(wallet) => self.co_act(data, me, now, |w, r| eravon_after::company::dispute(w, &r.case(), form, &wallet, first)),
            Err(s) => {
                self.said = Some(s.clone());
                self.exported = Some(Err(s));
                self.seal_next = None;
            }
        }
    }

    pub fn co_evidence(&self, data: &str, me: &str, home: &str, drop: Option<&str>, kit: Option<&str>, note: &str) -> Result<(eravon_dispute::evidence::Evidence, Option<crate::house::judge_card::Sealable>), Said> {
        let (linked, manual) = zk_of(data, me)?;
        let kit = if linked { kit } else { None };
        let o = self.open.as_ref().ok_or_else(|| Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenThatFirst)))?;
        let r = self.find(&o.instance).ok_or_else(|| Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable)))?;
        let w = crate::house::orders::wire(data, me)?;
        let (items, source) = eravon_dispute::evidence::items(home, Some(&manual), kit, drop).map_err(|(n, d)| Said::bad(&n, d))?;
        let ev = eravon_dispute::evidence::sheet_for(items, source, &r.row.buyer, &r.row.instance, &r.row.case_id, w.chain_id, &w.logic, (!note.trim().is_empty()).then(|| note.to_string())).map_err(|(n, d)| Said::bad(&n, d))?;
        let sealable = sealable_of(home, &manual, kit, drop, &ev.bytes);
        Ok((ev, sealable))
    }
}

fn sealable_of(home: &str, manual: &str, kit: Option<&str>, drop: Option<&str>, manifest: &[u8]) -> Option<crate::house::judge_card::Sealable> {
    eravon_dispute::seal::origins(home, Some(manual), kit, drop).ok().map(|origins| crate::house::judge_card::Sealable { origins, manifest: manifest.to_vec() })
}

fn act_of(a: &str) -> &str {
    match a {
        "openSend" => "openCase",
        "expireSend" => "expire",
        "cancelSend" => "cancel",
        "advanceSend" => "advance",
        "sweepSend" => "sweep",
        other => other,
    }
}

fn keep_end(data: &str, r: &BRow, state: eravon_watch::order::State, now: u64) -> Result<(), eravon_store::fault::Fault> {
    let id = if r.record.is_empty() { eravon_import::import::record_id(&r.row.case_id) } else { r.record.clone() };
    eravon_orders::table::keep_ended(data, crate::house::seam::WRITER, &r.owner, &id, state, now)
}
