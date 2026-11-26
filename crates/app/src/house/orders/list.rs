use super::*;

impl Orders {
    pub fn open(data: &str, addr: &str, now: u64, key: &Key) -> Orders {
        let mut o = Orders { me: addr.to_string(), ..Orders::default() };
        o.scan(data, addr, now, key);
        o.list(data, addr);
        o
    }

    pub fn table_only(data: &str, addr: &str) -> Orders {
        let mut o = Orders { me: addr.to_string(), ..Orders::default() };
        o.list(data, addr);
        o
    }

    pub fn fresh_for(&self, addr: &str) -> bool {
        self.me.eq_ignore_ascii_case(addr)
    }


    pub fn backstop(data: &str, addr: &str, now: u64, key: &Key, shown: &std::sync::Arc<eravon_chain::recall::Recall>) -> Orders {
        let mut o = Orders { me: addr.to_string(), ..Orders::default() };
        o.scan(data, addr, now, key);
        crate::house::refresh::draw(shown, || o.list_table(data, addr));
        o
    }

    pub fn list(&mut self, data: &str, addr: &str) {
        self.list_table(data, addr);
        if self.sel.is_some() {
            self.read_detail(data, addr);
        }
    }

    pub fn list_table(&mut self, data: &str, addr: &str) {
        let got = wire(data, addr).and_then(|w| eravon_orders::table::table(data, addr, eravon_scan::scan::Side::Seller, &sw(&w)).map_err(said));
        match got {
            Ok(t) => {
                self.foreign = t.foreign;
                self.unscanned = t.unscanned.as_ref().map(|(n, d)| Said::bad(n, d.clone()));
                self.table = Some(Ok(t.rows));
            }
            Err(s) => {
                self.unscanned = None;
                self.table = Some(Err(s));
            }
        }
    }

    pub(super) fn row(&self) -> Option<&Row> {
        let sel = self.sel.as_ref()?;
        self.table.as_ref()?.as_ref().ok()?.iter().find(|r| r.instance.eq_ignore_ascii_case(sel))
    }

    pub fn open_past(&self) -> bool {
        self.row().is_some_and(|r| r.past)
    }

    pub(super) fn read_detail(&mut self, data: &str, addr: &str) {
        let Some(r) = self.row().cloned() else {
            self.detail = None;
            return;
        };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let o = eravon_orders::detail::order_of(data, addr, &r);
        let s = sw(&w);
        let (mut d, refused) = eravon_cli::deskops::orderpage::read_detail(data, addr, &s, &r);
        if let Some(e) = refused {
            self.said = Some(e);
        }
        d.chosen = chosen_of(data, addr, &r, o.as_ref());
        d.advance = advance_of(&s, &r);
        self.detail = Some(d);
    }

    pub(crate) fn open_row_shape(&self) -> Option<String> {
        self.sel.as_ref()?;
        self.row().map(|r| format!("{r:?}"))
    }

    pub(crate) fn open_follow(&self) -> Option<(String, Option<String>)> {
        Some((self.sel.clone()?, self.open_row_shape()))
    }

    pub(crate) fn read_whole(&self) -> bool {
        matches!(self.table, Some(Ok(_))) && self.unscanned.is_none()
    }

    pub(crate) fn readvance(&mut self, data: &str, addr: &str) {
        let (Some(r), true) = (self.row().cloned(), self.detail.is_some()) else { return };
        let Ok(w) = wire(data, addr) else { return };
        let due = advance_of(&sw(&w), &r);
        if let Some(d) = self.detail.as_mut() {
            d.advance = due;
        }
    }

    pub(crate) fn reread_detail(&mut self, data: &str, addr: &str) {
        if let Some(fresh) = self.order.as_ref().and_then(|o| order::load(data, addr, &o.id).ok()) {
            self.order = Some(fresh);
        }
        self.read_detail(data, addr);
    }

    pub fn advance_send(&mut self, data: &str, addr: &str, key: &Key) {
        let Some(r) = self.row().cloned() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable)));
            return;
        };
        let Some(k) = key.get() else {
            self.said = Some(locked());
            return;
        };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let sent = (|| -> Result<String, Said> {
            let not_due = || Said::bad(eravon_watch::pool::ADVANCE_NOT_DUE, eravon_watch::pool::ADVANCE_NOT_DUE_SAYS);
            let v = r.chain.as_ref().filter(|v| v.phase != 0).ok_or_else(not_due)?;
            let case_id = eravon_standard::order::b32_of("案号", &r.case_id).map_err(|f| Said::bad(&f.name(), f.detail()))?;
            let seat = v.seats.saturating_sub(1);
            if !eravon_watch::pool::advance_due(&sw(&w), &r.instance, &case_id, seat).map_err(said)?.moves() {
                return Err(not_due());
            }
            eravon_standard::chain::code_matches(&w.t, &w.table, &r.instance, &w.logic).map_err(|f| Said::bad(&f.name(), f.detail()))?;
            let call = eravon_standard::call::Action::Advance { case_id, seat }.call(&r.instance).map_err(|f| Said::bad(&f.name(), f.detail()))?;
            let mut p = ShellPost { key: k, addr, w: &w, sent: None };
            p.post(&call).map_err(said)
        })();
        self.said = Some(match sent {
            Ok(tx) => Said::ok(format!("{} · {tx}", crate::house::i18n::t(crate::house::i18n::Key::BoAdvanceSaid))),
            Err(s) => s,
        });
        self.list(data, addr);
    }

    pub fn open_buyer(&self) -> Option<String> {
        let sel = self.sel.as_ref()?;
        let rows = self.table.as_ref()?.as_ref().ok()?;
        rows.iter().find(|r| r.instance.eq_ignore_ascii_case(sel)).map(|r| r.buyer.clone()).filter(|b| !b.is_empty())
    }

    pub fn open_row(&mut self, data: &str, addr: &str, now: u64, instance: &str) {
        let Some(r) = self.table.as_ref().and_then(|t| t.as_ref().ok()).and_then(|t| t.iter().find(|r| r.instance.eq_ignore_ascii_case(instance))).cloned() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable)));
            return;
        };
        self.tab = Tab::Overview;
        self.terms_open = false;
        self.inner = None;
        self.history = None;
        self.papers = None;
        self.bench = None;
        self.saved = None;
        self.complaint = None;
        if r.record.is_empty() {
            self.order = None;
            self.face = None;
            self.entries = None;
            self.sel = Some(r.instance.clone());
            self.read_detail(data, addr);
        } else {
            self.open_one(data, addr, now, &r.record);
        }
    }

    pub fn back(&mut self) {
        self.sel = None;
        self.order = None;
        self.detail = None;
        self.card = None;
        self.evidence = None;
        self.settle = None;
        self.deliver = None;
        self.face = None;
        self.entries = None;
        self.terms_open = false;
        self.inner = None;
        self.papers = None;
        self.bench = None;
        self.saved = None;
        self.complaint = None;
    }

    pub fn list_tab(&mut self, done: bool) {
        self.done_tab = done;
    }

    pub fn set_tab(&mut self, t: Tab) {
        self.tab = t;
    }

    pub fn search_set(&mut self, q: &str, from: &str, to: &str) {
        self.search = Search { on: true, q: q.to_string(), from: from.to_string(), to: to.to_string() };
        if let Err(e) = eravon_orders::day::bound(from).and_then(|_| eravon_orders::day::bound(to)) {
            self.said = Some(said(e));
        }
    }

    pub fn search_off(&mut self) {
        self.search = Search::default();
    }

    pub fn shown(&self) -> Vec<&Row> {
        let Some(Ok(rows)) = &self.table else { return Vec::new() };
        let s = &self.search;
        let hits = if s.on { eravon_orders::table::search(rows, &s.q, &s.from, &s.to).unwrap_or_else(|_| rows.iter().collect()) } else { rows.iter().collect() };
        hits.into_iter().filter(|r| eravon_orders::table::done_in(rows, r) == self.done_tab).collect()
    }

    pub fn set_label(&mut self, data: &str, addr: &str, now: u64, buyer: &str, name: &str) {
        match eravon_orders::label::set(data, seam::WRITER, now, addr, buyer, name) {
            Ok(()) => {
                self.said = Some(Said::ok(if name.trim().is_empty() { crate::house::i18n::t(crate::house::i18n::Key::OrdersLabelCleared) } else { crate::fill!(crate::house::i18n::Key::OrdersLabelSaved; name.trim().to_string()) }));
                self.list(data, addr);
            }
            Err(e) => self.said = Some(said(e)),
        }
    }

    pub fn save_originals(&mut self, data: &str, addr: &str, dir: &str) {
        let Some(o) = self.order.clone() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersNoOriginal)));
            return;
        };
        let Some(d) = deliver::delivery(data, addr, &o) else {
            self.said = Some(Said::bad("E_DELIVER_ORIGINAL", crate::house::i18n::t(crate::house::i18n::Key::OrdersNoDelivery)));
            return;
        };
        match deliver::save_originals(&d, dir) {
            Ok(paths) => {
                self.said = Some(Said::ok(crate::fill!(crate::house::i18n::Key::OrdersSavedN; paths.len().to_string())));
                self.saved = Some(paths);
            }
            Err(e) => self.said = Some(said(e)),
        }
    }

    pub fn open_own(&mut self, n: u64) -> Option<String> {
        let d = self.detail.as_ref()?.dispute.as_ref()?;
        match d.evidence.iter().filter(|e| e.ours).nth(n.saturating_sub(1) as usize) {
            Some(e) if !e.source.is_empty() => {
                self.opened = Some(e.source.clone());
                Some(e.source.clone())
            }
            _ => {
                self.said = Some(Said::bad("E_ORDERS_OWN", crate::fill!(crate::house::i18n::Key::OrdersNoEvidenceOriginal; n.to_string())));
                None
            }
        }
    }

    pub fn scan(&mut self, data: &str, addr: &str, now: u64, key: &Key) {
        let opens = opens(data, addr);
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.scan = Some(Err(s));
                return;
            }
        };
        let keys = eravon_vault::keyset::seller(data, addr).unwrap_or_default();
        let book = crate::house::book::load(data, addr).filter(|b| b.holds(addr, w.chain_id, &w.logic) && !keys.is_empty() && b.covers(&keys));
        let from = book.as_ref().map_or(0, |b| crate::house::refresh::scan_from(b.cursor));
        let kept: Vec<eravon_scan::scan::Case> = book
            .map(|b| b.cases.into_iter().map(|c| eravon_scan::scan::Case { case_id: c.case_id, instance: c.instance, seller: c.seller, buyer: c.buyer, block: c.block, tx: c.tx }).collect())
            .unwrap_or_default();
        let got = match key.get() {
            Some(k) => {
                let mut p = ShellPost { key: k, addr, w: &w, sent: None };
                eravon_watch::scan::startup(data, seam::WRITER, now, addr, &sw(&w), from, &kept, opens, Some(&mut p))
            }
            None => eravon_watch::scan::startup(data, seam::WRITER, now, addr, &sw(&w), from, &kept, opens, None),
        };
        self.scan = Some(match got {
            Ok(s) => Ok(crate::fill!(crate::house::i18n::Key::OrdersScanDone; (s.enrolled.len() + s.marked.len()).to_string())),
            Err(e) => Err(said(e)),
        });
        if let Some(Err(s)) = &self.scan {
            self.said = Some(s.clone());
        }
        if self.table.is_some() {
            self.list_table(data, addr);
        }
        self.reopen(data, addr, now);
    }

    pub(crate) fn reopen(&mut self, data: &str, addr: &str, now: u64) {
        if self.sel.is_none() {
            return;
        }
        let record = self.row().map(|r| r.record.clone()).filter(|x| !x.is_empty()).or_else(|| self.order.as_ref().map(|o| o.id.clone()));
        match record {
            Some(id) => self.open_one(data, addr, now, &id),
            None if self.table.is_some() => self.read_detail(data, addr),
            None => {}
        }
    }

    pub fn paste(&mut self, data: &str, addr: &str, now: u64, text: &str) {
        let t = text.trim();
        let bytes = if t.starts_with('{') {
            Ok(t.as_bytes().to_vec())
        } else {
            eravon_cli::seller::decode_receipt_field(t).map_err(|e| Said::bad(e.name(), e.detail()))
        };
        let w = super::wire(data, addr).ok();
        let r = bytes.and_then(|b| eravon_watch::paste::paste_receipt(data, seam::WRITER, now, addr, &b, None, w.as_ref().map(super::sw).as_ref()).map_err(said));
        match r {
            Ok(o) => {
                self.said = Some(Said::ok(crate::fill!(crate::house::i18n::Key::OrdersReceiptIn; crate::house::orders::order_label(&o.instance))));
                self.paste.clear();
                self.paste_open = false;
                self.sel = Some(o.instance.to_ascii_lowercase());
                self.show(data, addr, o);
                self.list(data, addr);
            }
            Err(s) => self.said = Some(s),
        }
    }

    pub(super) fn show(&mut self, data: &str, addr: &str, o: Order) {
        match order_company(data, addr, &o) {
            Ok(c) => {
                self.co = c;
                self.co_err = None;
            }
            Err(s) => {
                self.co = None;
                self.co_err = Some(s);
            }
        }
        if self.order.as_ref().is_some_and(|cur| cur.id != o.id) {
            self.card = None;
            self.evidence = None;
            self.settle = None;
            self.sig_settle.clear();
            self.sig_confirm.clear();
            self.face = None;
            self.entries = None;
            self.opened = None;
            self.judge = None;
            self.judge_open = false;
            self.inner = None;
            self.papers = None;
            self.bench = None;
            self.sealable = None;
            self.seal_next = None;
            self.sealed = None;
            self.complaint = None;
        }
        self.todo = todo::stored(&o);
        self.offer = settle::remembered(&o).map(|(b, e, _)| (b, e));
        self.order = Some(o);
    }

    pub fn open_one(&mut self, data: &str, addr: &str, _now: u64, id: &str) {
        match order::load(data, addr, id) {
            Ok(o) => {
                self.sel = Some(o.instance.to_ascii_lowercase());
                self.show(data, addr, o);
                self.read_face(data, addr);
                if self.table.is_some() {
                    self.read_detail(data, addr);
                }
            }
            Err(e) => self.said = Some(said(e)),
        }
    }

    pub(super) fn read_face(&mut self, data: &str, addr: &str) {
        let Some(o) = self.order.clone() else { return };
        if o.state != State::Deployed {
            self.face = None;
            return;
        }
        let r = wire(data, addr).and_then(|w| seat::face(&sw(&w), &o).map_err(said));
        self.face = Some(r.map(|f| face_view(&f, None, None)));
    }

    pub fn pending_bump(&mut self, data: &str, addr: &str, now: u64, key: &Key) {
        let Some(id) = self.order.as_ref().map(|o| o.id.clone()) else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersPasteFirst)));
            return;
        };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let Some(k) = key.get() else {
            self.said = Some(Said::bad("E_VAULT_LOCKED", crate::house::i18n::t(crate::house::i18n::Key::OrdersBumpLocked)));
            return;
        };
        let mut p = ShellPost { key: k, addr, w: &w, sent: None };
        match eravon_watch::send::bump(data, seam::WRITER, now, addr, &sw(&w), &id, &mut p) {
            Ok(t) => {
                self.said = Some(match &t.sent {
                    Some((what, tx)) => Said::ok(format!("{} · {tx}", sent_say(what))),
                    None => Said::ok(t.order.note.clone()),
                });
                self.show(data, addr, t.order);
            }
            Err(e) => self.said = Some(said(e)),
        }
        self.list(data, addr);
    }

    pub fn pending_void(&mut self, data: &str, addr: &str, now: u64) {
        let Some(id) = self.order.as_ref().map(|o| o.id.clone()) else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersPasteFirst)));
            return;
        };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        match eravon_watch::send::void(data, seam::WRITER, now, addr, addr, &sw(&w), &id) {
            Ok(o) => {
                self.said = Some(Said::ok(o.note.clone()));
                self.show(data, addr, o);
            }
            Err(e) => self.said = Some(said(e)),
        }
        self.list(data, addr);
    }

    pub fn poll(&mut self, data: &str, addr: &str, now: u64, key: &Key) {
        self.round(data, addr, now, key, opens(data, addr));
    }

    pub fn deploy(&mut self, data: &str, addr: &str, now: u64, key: &Key) {
        if !self.planned(data, addr, key, Plan::Open) {
            return;
        }
        self.round(data, addr, now, key, eravon_watch::send::Opens::Yes);
    }

    fn round(&mut self, data: &str, addr: &str, now: u64, key: &Key, opens: eravon_watch::send::Opens) {
        let Some(id) = self.order.as_ref().map(|o| o.id.clone()) else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersPasteFirst)));
            return;
        };
        let w = match wire(data, addr) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return;
            }
        };
        let got = match key.get() {
            Some(k) => {
                let mut p = ShellPost { key: k, addr, w: &w, sent: None };
                eravon_watch::send::tick(data, seam::WRITER, now, addr, &sw(&w), &id, opens, Some(&mut p))
            }
            None => eravon_watch::send::tick(data, seam::WRITER, now, addr, &sw(&w), &id, opens, None),
        };
        match got {
            Ok(t) => {
                self.said = Some(match (&t.sent, &t.refused) {
                    (Some((what, tx)), _) => Said::ok(format!("{} · {tx}", sent_say(what))),
                    (None, Some((name, detail))) => Said::bad(name, detail.clone()),
                    (None, None) => Said::ok(t.order.state.say()),
                });
                let o = t.order;
                if o.state == State::Deployed {
                    if let Err(e) = todo::check(data, seam::WRITER, now, addr, &sw(&w), &o, 0) {
                        self.said = Some(said(e));
                    }
                    self.dispute_tick(data, addr, now, key, &w, &o);
                }
                let fresh = order::load(data, addr, &o.id).unwrap_or(o);
                self.show(data, addr, fresh);
                self.list(data, addr);
            }
            Err(e) => self.said = Some(said(e)),
        }
    }

    pub fn todo_item(&self, which: &str) -> Option<String> {
        let t = self.todo.as_ref()?;
        match which {
            "grantee" => Some(t.grantee.clone()),
            "work" => Some(t.work.clone()),
            "terms" => Some(t.terms_hash.clone()),
            _ => None,
        }
    }

    pub fn rows(&self) -> Out {
        let mut m: Vec<(&'static str, Out)> = vec![("paste", Out::S(self.paste.clone()))];
        match &self.exported {
            Some(Ok(p)) => m.push(("exported", Out::S(p.clone()))),
            Some(Err(e)) => m.push(("exported", e.out())),
            None => {}
        }
        if let Some(x) = crate::house::ask::out(&self.ask) {
            m.push(("timeAsk", x));
        }
        if let Some(o) = &self.order {
            let (say, gap_row) = line(o);
            m.push((
                "order",
                Out::O(vec![
                    ("balance", Out::S(o.balance.clone())),
                    ("caseId", Out::S(o.case_id.clone())),
                    ("deployTx", Out::S(o.deploy_tx.clone())),
                    ("expireTx", Out::S(o.expire_tx.clone())),
                    ("gapRow", Out::S(gap_row.unwrap_or_default())),
                    ("id", Out::S(o.id.clone())),
                    ("instance", Out::S(o.instance.clone())),
                    ("line", Out::S(say)),
                    ("need", Out::S(o.need.clone())),
                    ("note", Out::S(o.note.clone())),
                    ("notice", Out::S(o.notice.clone())),
                    ("state", Out::S(o.state.word().to_string())),
                    ("verdict", Out::S(o.verdict.clone())),
                ]),
            ));
            if let (Some((_, tx)), Some(at)) = (o.pending(), o.pending_at()) {
                m.push((
                    "pending",
                    Out::O(vec![
                        ("gasLimit", Out::N(at.gas_limit)),
                        ("keys", Out::A(vec![Out::S(seam::STEP_PENDING_BUMP.into()), Out::S(seam::STEP_PENDING_VOID.into())])),
                        ("maxFee", Out::S(at.max_fee.clone())),
                        ("nonce", Out::N(at.nonce)),
                        ("priorityFee", Out::S(at.priority_fee.clone())),
                        ("tx", Out::S(tx)),
                    ]),
                ));
            }
        }
        if let Some(d) = &self.deliver {
            let mut x = vec![
                ("keyNote", Out::S(d.key_note.clone())),
                ("keyOn", Out::B(d.key_on)),
                ("method", Out::S(d.method.clone())),
                ("note", Out::S(d.note.clone())),
                ("path", Out::S(d.path.clone())),
                ("reading", Out::B(d.reading)),
            ];
            match &d.sheet {
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
            m.push(("deliver", Out::O(x)));
        }
        if let Some(t) = &self.todo {
            m.push((
                "todo",
                Out::O(vec![
                    ("grantee", Out::S(t.grantee.clone())),
                    ("say", Out::S(todo::TODO_SAY.to_string())),
                    ("terminal", Out::S(t.terminal.clone())),
                    ("terms", Out::S(t.terms_hash.clone())),
                    ("work", Out::S(t.work.clone())),
                ]),
            ));
        }
        match &self.scan {
            Some(Ok(s)) => m.push(("scan", Out::S(s.clone()))),
            Some(Err(s)) => m.push(("scan", s.out())),
            None => {}
        }
        if let Some(s) = &self.said {
            m.push(("said", s.out()));
        }
        if let Some(c) = &self.copied {
            m.push(("copied", Out::S(c.clone())));
        }
        self.dispute_rows(&mut m);
        self.s2_rows(&mut m);
        Out::O(m)
    }
}
