use super::*;

pub(super) fn face_view(f: &seat::Face, redraw: Option<Said>, sweep: Option<Said>) -> FaceView {
    let (word, phase) = match f {
        seat::Face::Open(n) => ("open", *n),
        seat::Face::Ruling(_) => ("ruling", eravon_standard::precheck::PHASE_DISPUTED),
        seat::Face::PendingRedraw(_) => ("pendingRedraw", 1),
        seat::Face::Resigned(_) => ("resigned", 8),
        seat::Face::Closed(n) => ("closed", *n),
    };
    FaceView { word, say: f.say(), phase, refundable: f.refundable(), disputed: f.disputed(), redraw, sweep }
}

pub(super) fn said_or(r: &Result<String, Said>) -> Out {
    match r {
        Ok(v) => Out::S(v.clone()),
        Err(s) => s.out(),
    }
}

pub(super) fn sheet_rows(x: &mut Vec<(&'static str, Out)>, sheet: &Option<Result<(String, Vec<(String, u64)>), Said>>) {
    match sheet {
        Some(Ok((h, files))) => {
            x.push(("manifest", Out::S(h.clone())));
            x.push((
                "files",
                Out::A(files.iter().map(|(n, b)| Out::O(vec![("bytes", Out::N(*b)), ("name", Out::S(n.clone()))])).collect()),
            ));
        }
        Some(Err(s)) => x.push(("sheet", s.out())),
        None => {}
    }
}

impl Orders {
    pub(super) fn need_order(&mut self) -> Option<Order> {
        if self.order.is_none() {
            self.said = Some(self.no_order());
        }
        self.order.clone()
    }

    pub fn refund_open(&mut self, data: &str, addr: &str) {
        let Some(o) = self.need_order() else { return };
        let r = wire(data, addr).and_then(|w| refund::card(data, addr, &sw(&w), &o).map_err(said));
        match r {
            Ok(c) => {
                self.card = Some(CardView {
                    amount: c.amount.to_dec(),
                    accepted: c.accepted,
                    judge_fee: c.judge_fee.to_dec(),
                    to_buyer: c.to_buyer.to_dec(),
                    bond_back: c.bond_back.to_dec(),
                    lf_back: c.lf_back.to_dec(),
                })
            }
            Err(s) => self.said = Some(s),
        }
    }

    pub fn refund_send(&mut self, data: &str, addr: &str, key: &Key) {
        if !self.planned(data, addr, key, Plan::Refund) {
            return;
        }
        let Some(o) = self.need_order() else { return };
        if self.card.is_none() {
            self.said = Some(Said::bad("E_ORDERS_CARD", crate::house::i18n::t(crate::house::i18n::Key::OrdersCardFirst)));
            return;
        }
        let co = match order_company(data, addr, &o) {
            Ok(c) => c,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        if co.is_some() {
            let plan = wire(data, addr).map_err(|s| (s.name, s.detail)).and_then(|w| eravon_dispute::company::refund(&sw(&w), &o));
            if self.company_export(data, addr, 0, &o.id, plan, crate::house::ask::Post::None) {
                self.card = None;
            }
            return;
        }
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let Some(mut p) = post_of(key, addr, &w) else {
            self.said = Some(locked());
            return;
        };
        let r = refund::refund(&sw(&w), &o, &mut p);
        self.said = Some(why_said(&r, "refund"));
        if r.is_ok() {
            self.card = None;
            self.read_detail(data, addr);
        }
    }

    pub fn refund_close(&mut self) {
        self.card = None;
    }

    pub fn evidence_open(&mut self, data: &str, addr: &str, home: &str) {
        if self.need_order().is_none() {
            return;
        }
        let (kits, manual) = match eravon_cli::deskops::network::zikaron_of(data, addr) {
            Ok((on, manual)) => (crate::house::settings::zk_kits(home, &manual, on), manual),
            Err(n) => (Err(Said::bad(&n.name, n.detail())), String::new()),
        };
        self.evidence = Some(EvidenceModal {
            home: home.to_string(),
            manual,
            kits,
            kit: None,
            path: String::new(),
            note: String::new(),
            items: None,
            reading: false,
            sheet: None,
        });
    }

    pub fn evidence_kit_begin(&mut self, kit: Option<String>) {
        let Some(m) = self.evidence.as_mut() else { return };
        let listed = |id: &str| m.kits.as_ref().is_ok_and(|ks| ks.iter().any(|(k, _, _)| k == id));
        if kit.as_deref().is_some_and(|k| !listed(k)) {
            return;
        }
        m.kit = kit;
        m.items = None;
        m.sheet = None;
        m.reading = m.kit.is_some() || !m.path.is_empty();
    }

    pub fn evidence_pick_begin(&mut self, path: &str) {
        let Some(m) = self.evidence.as_mut() else { return };
        m.path = path.to_string();
        m.items = None;
        m.sheet = None;
        m.reading = m.kit.is_some() || !m.path.is_empty();
    }

    pub fn evidence_read(&mut self, addr: &str) {
        if !self.evidence.as_ref().is_some_and(|m| m.reading) {
            return;
        }
        self.evidence_reread();
        if let Some(m) = self.evidence.as_mut() {
            m.reading = false;
        }
        self.evidence_resheet(addr);
    }

    pub fn evidence_note(&mut self, addr: &str, note: &str) {
        let Some(m) = self.evidence.as_mut() else { return };
        m.note = note.to_string();
        self.evidence_resheet(addr);
    }

    pub(super) fn evidence_sheet(&self, addr: &str) -> Option<Result<evidence::Evidence, Said>> {
        let o = self.order.as_ref()?;
        let m = self.evidence.as_ref()?;
        if m.kit.is_none() && m.path.is_empty() {
            return None;
        }
        let (chain_id, logic) = eravon_watch::standard();
        let note = (!m.note.trim().is_empty()).then(|| m.note.clone());
        let drop = (!m.path.is_empty()).then_some(m.path.as_str());
        Some(evidence::sheet(&m.home, Some(&m.manual), m.kit.as_deref(), drop, addr, o, chain_id, logic, note).map_err(said))
    }

    pub(super) fn evidence_reread(&mut self) {
        let Some(m) = self.evidence.as_mut() else { return };
        let drop = (!m.path.is_empty()).then_some(m.path.as_str());
        m.items = (m.kit.is_some() || drop.is_some()).then(|| evidence::items(&m.home, Some(&m.manual), m.kit.as_deref(), drop).map_err(said));
    }

    pub(super) fn evidence_resheet(&mut self, addr: &str) {
        let Some(o) = self.order.clone() else { return };
        let Some(m) = self.evidence.as_mut() else { return };
        let Some(items) = m.items.clone() else {
            m.sheet = None;
            return;
        };
        let (chain_id, logic) = eravon_watch::standard();
        let note = (!m.note.trim().is_empty()).then(|| m.note.clone());
        m.sheet = Some(
            items
                .and_then(|(v, source)| evidence::sheet_of(v, source, self.co.as_deref().unwrap_or(addr), &o, chain_id, logic, note).map_err(said))
                .map(|e| (eravon_standard::order::hex_of(&e.manifest), e.files)),
        );
    }

    pub fn evidence_send(&mut self, data: &str, addr: &str, now: u64, key: &Key) {
        if !self.planned(data, addr, key, Plan::Evidence) {
            return;
        }
        let Some(o) = self.need_order() else { return };
        let co = match order_company(data, addr, &o) {
            Ok(c) => c,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let ev = match self.evidence_sheet(co.as_deref().unwrap_or(addr)) {
            Some(Ok(e)) => e,
            Some(Err(s)) => {
                self.said = Some(s);
                return;
            }
            None => {
                self.said = Some(Said::bad("E_DISPUTE_EMPTY", crate::house::i18n::t(crate::house::i18n::Key::OrdersNoEvidenceYet)));
                return;
            }
        };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        if co.is_some() {
            self.seal_next = self.evidence_sealable(&ev.bytes);
            if self.company_export(data, addr, now, &o.id, eravon_dispute::company::commit(&sw(&w), &o, &ev), crate::house::ask::Post::None) {
                self.evidence = None;
            }
            return;
        }
        let Some(mut p) = post_of(key, addr, &w) else {
            self.said = Some(locked());
            return;
        };
        let r = evidence::commit(data, seam::WRITER, now, addr, &sw(&w), &o, &ev, &mut p);
        self.said = Some(why_said(&r, "commit"));
        if r.is_ok() {
            self.sealable = self.evidence_sealable(&ev.bytes);
            self.sealed = None;
            self.evidence = None;
            self.open_one(data, addr, now, &o.id);
        }
    }

    fn evidence_sealable(&self, manifest: &[u8]) -> Option<crate::house::judge_card::Sealable> {
        self.evidence.as_ref().and_then(|m| {
            let drop = (!m.path.is_empty()).then_some(m.path.as_str());
            eravon_dispute::seal::origins(&m.home, Some(&m.manual), m.kit.as_deref(), drop).ok()
        }).map(|origins| crate::house::judge_card::Sealable { origins, manifest: manifest.to_vec() })
    }

    pub fn evidence_close(&mut self) {
        self.evidence = None;
    }

    pub fn settle_open(&mut self) {
        if self.need_order().is_none() {
            return;
        }
        self.settle = Some(SettleModal::default());
    }

    pub fn settle_make(&mut self, data: &str, addr: &str, now: u64, key: &Key, bps: &str, days: &str) {
        if !self.planned(data, addr, key, Plan::Settle(bps.to_string(), days.to_string())) {
            return;
        }
        let Some(o) = self.need_order() else { return };
        let Some(m) = self.settle.as_mut() else { return };
        m.bps = bps.to_string();
        m.days = days.to_string();
        let co = match order_company(data, addr, &o) {
            Ok(c) => c,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        if co.is_some() {
            let (chain_id, _) = eravon_watch::standard();
            let parsed = bps
                .trim()
                .parse::<u64>()
                .map_err(|_| ("E_DISPUTE_BPS".to_string(), crate::fill!(crate::house::i18n::Key::OrdersBpsBad; bps.to_string())))
                .and_then(|b| settle::expiry_in(now, days).map(|e| (b, e)));
            let logic = self.row().map(|r| r.logic.clone()).unwrap_or_else(|| {
                eravon_watch::poll::listed_logic(data, addr, &o)
                    .filter(|l| eravon_literals::constants::generation_of_logic(l).is_some_and(|g| g.is_past()))
                    .unwrap_or_else(|| eravon_watch::standard().1.to_string())
            });
            let made = parsed.and_then(|(b, e)| eravon_dispute::company::settle(chain_id, &logic, co.as_deref().unwrap_or_default(), &o.instance, &o.case_id, b, e));
            let (plan, of) = match made {
                Ok((p, of)) => (Ok(p), Some(of)),
                Err(w) => (Err(w), None),
            };
            let post = of.map_or(crate::house::ask::Post::None, crate::house::ask::Post::Settle);
            self.company_export(data, addr, now, &o.id, plan, post);
            return;
        }
        let Some(k) = key.get() else {
            self.said = Some(locked());
            return;
        };
        let parsed = bps
            .trim()
            .parse::<u64>()
            .map_err(|_| Said::bad("E_DISPUTE_BPS", crate::fill!(crate::house::i18n::Key::OrdersBpsBad; bps.to_string())))
            .and_then(|b| settle::expiry_in(now, days).map(|e| (b, e)).map_err(said));
        let (chain_id, _) = eravon_watch::standard();
        let sign = |d: &[u8; 32]| eravon_cli::shared::sign(&k, d).map_err(|e| (e.name().to_string(), e.detail()));
        let made = parsed.and_then(|(b, expiry)| {
            let w = wire(data, addr)?;
            settle::full_is_refund(&sw(&w), &o, b).map_err(said)?;
            let of = settle::offer(&sign, chain_id, &o.instance, b, expiry).map_err(said)?;
            settle::remember(data, seam::WRITER, now, addr, &o, &of).map_err(said)?;
            let case_id = eravon_standard::order::b32_of("案号", &o.case_id).map_err(|e| Said::bad(&e.name(), e.detail()))?;
            let call = settle::post_call(&case_id, &of.bytes).map_err(said)?;
            let tx = eravon_cli::send::deliver_key(&|| Ok(k), &eravon_cli::send::Opts::default(), &w, addr, &call, "0")
                .map_err(|e| Said::bad(e.name(), e.detail()))?
                .outcome()
                .map_err(|(n, d)| Said::bad(&n, d))?;
            settle::posted(&sw(&w), &tx, &case_id, &of.bytes).map_err(said)?;
            let link = eravon_goods::link::link_of(chain_id, &tx, &of.bytes).map_err(|(n, d)| Said::bad(&n, d))?;
            Ok((of, link))
        });
        let view = made.map(|(of, link)| OfferView { preview: Ok(link), digest: eravon_standard::order::hex_of(&of.digest) });
        if let Some(m) = self.settle.as_mut() {
            m.offer = Some(view);
        }
        self.open_one(data, addr, now, &o.id);
    }

    pub fn settle_preview(&mut self) -> Option<String> {
        let got = self.settle.as_ref().and_then(|m| m.offer.as_ref()).map(|r| r.as_ref().map(|v| v.preview.clone()));
        match got {
            Some(Ok(Ok(u))) => {
                self.opened = Some(u.clone());
                Some(u)
            }
            Some(Ok(Err(s))) => {
                self.said = Some(s);
                None
            }
            _ => {
                self.said = Some(Said::bad("E_DISPUTE_NO_OFFER", crate::house::i18n::t(crate::house::i18n::Key::OrdersOfferFirst)));
                None
            }
        }
    }

    pub fn settle_close(&mut self) {
        self.settle = None;
    }

    pub fn paste_settle(&mut self, data: &str, addr: &str, now: u64, key: &Key, sig: &str) {
        self.sig_settle = sig.to_string();
        self.paste_sig(data, addr, now, key, sig, true);
    }

    pub fn paste_confirm(&mut self, data: &str, addr: &str, now: u64, key: &Key, sig: &str) {
        self.sig_confirm = sig.to_string();
        self.paste_sig(data, addr, now, key, sig, false);
    }

    pub(super) fn paste_sig(&mut self, data: &str, addr: &str, now: u64, key: &Key, sig: &str, settle_one: bool) {
        if !self.planned(data, addr, key, Plan::Sig) {
            return;
        }
        let Some(o) = self.need_order() else { return };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let Some(mut p) = post_of(key, addr, &w) else {
            self.said = Some(locked());
            return;
        };
        let sig = sig.trim();
        let (r, what) = if settle_one {
            (settle::send_settle(data, addr, &sw(&w), &o, sig, &mut p), "settle")
        } else {
            (settle::send_confirm(data, addr, &sw(&w), &o, sig, &mut p), "confirmBySig")
        };
        self.said = Some(why_said(&r, what));
        if r.is_ok() {
            if settle_one {
                self.sig_settle.clear();
            } else {
                self.sig_confirm.clear();
            }
            self.open_one(data, addr, now, &o.id);
        }
    }

    pub(super) fn dispute_tick(&mut self, data: &str, addr: &str, now: u64, key: &Key, w: &eravon_cli::shared::Wire, o: &Order) {
        let mut p = post_of(key, addr, w);
        let company = crate::house::orders::company_mode(data, addr).is_some();
        let got = tick::tick(data, seam::WRITER, now, addr, &sw(w), o, p.as_mut().map(|x| x as &mut dyn Post), company);
        match got {
            Ok(mut t) => {
                let redraw = t.redraw.as_ref().map(|r| why_said(r, "redraw"));
                let sweep = match t.proposal.take() {
                    Some(plan) => {
                        let dir = self.export_dir.clone();
                        if self.export_now(data, addr, now, &o.id, &dir, plan, crate::house::ask::Post::None, false) {
                            if let (Some(Ok(path)), Ok(mut o2)) = (self.exported.clone(), eravon_watch::order::load(data, addr, &o.id)) {
                                o2.set_extra("sweepProposal", &path);
                                if let Err(e) = eravon_watch::order::save(data, seam::WRITER, now, addr, &o2, &[]) {
                                    self.said = Some(said(e));
                                }
                            }
                        }
                        self.said.clone()
                    }
                    None => t.sweep.map(|r| match r {
                        Ok(sweep::Swept::Sent(tx)) => Said::ok(format!("{} · {tx}", sent_say("sweep"))),
                        Ok(sweep::Swept::Skipped(n)) | Ok(sweep::Swept::Nothing(n)) => Said::ok(n),
                        Err(e) => said(e),
                    }),
                };
                self.face = Some(Ok(face_view(&t.face, redraw, sweep)));
                self.entries = Some(t.entries.map_err(said));
            }
            Err(e) => self.face = Some(Err(said(e))),
        }
    }

    pub(super) fn dispute_rows(&self, m: &mut Vec<(&'static str, Out)>) {
        if let Some(c) = &self.card {
            m.push((
                "card",
                Out::O(vec![
                    ("accepted", Out::B(c.accepted)),
                    ("amount", Out::S(c.amount.clone())),
                    ("amountWord", Out::S(money(crate::house::i18n::Lang::Zh, self.order.as_ref(), &c.amount))),
                    ("bondBack", Out::S(c.bond_back.clone())),
                    ("bondBackWord", Out::S(money(crate::house::i18n::Lang::Zh, self.order.as_ref(), &c.bond_back))),
                    ("judgeFee", Out::S(c.judge_fee.clone())),
                    ("judgeFeeWord", Out::S(money(crate::house::i18n::Lang::Zh, self.order.as_ref(), &c.judge_fee))),
                    ("lfBack", Out::S(c.lf_back.clone())),
                    ("lfBackWord", Out::S(money(crate::house::i18n::Lang::Zh, self.order.as_ref(), &c.lf_back))),
                    ("toBuyer", Out::S(c.to_buyer.clone())),
                    ("toBuyerWord", Out::S(money(crate::house::i18n::Lang::Zh, self.order.as_ref(), &c.to_buyer))),
                ]),
            ));
        }
        if let Some(e) = &self.evidence {
            let kits = match &e.kits {
                Ok(ks) => Out::A(
                    ks.iter()
                        .map(|(id, note, n)| Out::O(vec![("contents", Out::N(*n)), ("id", Out::S(id.clone())), ("note", Out::S(note.clone()))]))
                        .collect(),
                ),
                Err(s) => s.out(),
            };
            let mut x = vec![
                ("kit", Out::S(e.kit.clone().unwrap_or_default())),
                ("kits", kits),
                ("note", Out::S(e.note.clone())),
                ("path", Out::S(e.path.clone())),
                ("reading", Out::B(e.reading)),
            ];
            sheet_rows(&mut x, &e.sheet);
            m.push(("evidence", Out::O(x)));
        }
        if let Some(st) = &self.settle {
            let mut x = vec![("bps", Out::S(st.bps.clone())), ("days", Out::S(st.days.clone()))];
            match &st.offer {
                Some(Ok(v)) => x.push((
                    "offer",
                    Out::O(vec![("digest", Out::S(v.digest.clone())), ("preview", said_or(&v.preview))]),
                )),
                Some(Err(s)) => x.push(("offer", s.out())),
                None => {}
            }
            m.push(("settle", Out::O(x)));
        }
        if let Some((b, e)) = self.offer {
            m.push(("proposal", Out::O(vec![("buyerBps", Out::N(b)), ("expiry", Out::N(e))])));
        }
        m.push(("sigConfirm", Out::S(self.sig_confirm.clone())));
        m.push(("sigSettle", Out::S(self.sig_settle.clone())));
        match &self.face {
            Some(Ok(f)) => {
                let mut x = vec![
                    ("disputed", Out::B(f.disputed)),
                    ("phase", Out::N(f.phase)),
                    ("refundable", Out::B(f.refundable)),
                    ("say", Out::S(f.say.clone())),
                    ("word", Out::S(f.word.to_string())),
                ];
                if let Some(r) = &f.redraw {
                    x.push(("redraw", r.out()));
                }
                if let Some(r) = &f.sweep {
                    x.push(("sweep", r.out()));
                }
                m.push(("face", Out::O(x)));
            }
            Some(Err(s)) => m.push(("face", s.out())),
            None => {}
        }
        match &self.entries {
            Some(Ok(es)) => m.push((
                "entries",
                Out::A(
                    es.iter()
                        .map(|e| {
                            Out::O(vec![
                                ("block", Out::N(e.block)),
                                ("kind", Out::S(e.kind.clone())),
                                ("red", Out::S(e.red.clone().unwrap_or_default())),
                                ("signer", Out::S(e.signer.clone())),
                                ("word", Out::S(e.word.clone())),
                            ])
                        })
                        .collect(),
                ),
            )),
            Some(Err(s)) => m.push(("entries", s.out())),
            None => {}
        }
        if let Some(u) = &self.opened {
            m.push(("opened", Out::S(u.clone())));
        }
    }
}
