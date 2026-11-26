use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Leave {
    List,
    Page(Option<Page>),
    Seat(crate::house::shell::Seat),
    Close,
}

impl Leave {
    pub fn name(&self) -> String {
        match self {
            Leave::List => "list".into(),
            Leave::Page(p) => format!("page:{}", p.map_or(seam::PAGE_HOME, Page::name)),
            Leave::Seat(s) => format!("seat:{}", s.name()),
            Leave::Close => "close".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GoodsEdit {
    pub id: Option<String>,
    pub draft: Draft,
    pub tab: usize,
    pub last: Option<Saved>,
    pub ask: Option<Leave>,
    pub dirty: bool,
    pub stored: Option<Draft>,
}

impl GoodsEdit {
    pub fn fees_shown(&self) -> Option<&[eravon_goods::resolve::Fee]> {
        let sv = self.last.as_ref()?;
        (!self.dirty && !sv.resolved.fees.is_empty()).then_some(sv.resolved.fees.as_slice())
    }

    pub fn left_out(&self, lang: crate::house::i18n::Lang) -> Vec<(String, String)> {
        match &self.last {
            Some(sv) if !self.dirty => left_out(lang, &sv.resolved.excluded),
            _ => Vec::new(),
        }
    }
}

pub fn left_out(lang: crate::house::i18n::Lang, excluded: &[eravon_goods::resolve::Excluded]) -> Vec<(String, String)> {
    excluded.iter().map(|x| (x.address.clone(), crate::house::refusal::line(lang, &x.name, &x.detail))).collect()
}

#[derive(Clone, Debug)]
pub struct Qr {
    pub link: String,
    pub fits: bool,
    pub modules: Option<(usize, Vec<bool>)>,
    pub saved: Option<Result<(String, usize), Said>>,
    pub image: Option<Result<(usize, Vec<u8>), Said>>,
}

#[derive(Clone, Debug)]
pub struct Shown {
    pub id: String,
    pub view: Result<g1::View, Said>,
    pub qr: Option<Qr>,
    pub opened: Option<String>,
    pub copied: Option<String>,
    pub hashes: Vec<String>,
    pub taken: Option<Result<Vec<String>, Said>>,
    pub regs: Option<Result<Vec<eravon_goods::company::Reg>, Said>>,
    pub exported: Option<Result<String, Said>>,
    pub inner: Option<GoodsInner>,
    pub checked: Option<Result<String, Said>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoodsInner {
    Doc(bool),
    Lapse(String),
}

impl GoodsInner {
    pub fn word(&self) -> &'static str {
        match self {
            GoodsInner::Doc(true) => "goodsText",
            GoodsInner::Doc(false) => "deliveryText",
            GoodsInner::Lapse(_) => "lapse",
        }
    }
}

impl Shown {
    pub fn texts(&self) -> Option<(String, String)> {
        let v = self.view.as_ref().ok()?;
        let d = &v.draft;
        let secs = |c: &str, a: &str, b: &str, m: &str| eravon_goods::goods::secs_of(c, a, b, m, &mut eravon_goods::verdict::Verdict::default()).unwrap_or(0);
        Some((eravon_goods::terms::goods_text(&d.name, &d.description), eravon_goods::terms::delivery_text(secs("t1", &d.t1, &d.t1h, &d.t1m), secs("t2", &d.t2, &d.t2h, &d.t2m), &d.delivery_note)))
    }

    pub fn lapses(&self) -> Vec<(String, Result<eravon_cands::lapse::Clocks, String>)> {
        let Ok(v) = self.view.as_ref() else { return Vec::new() };
        let fees = if v.fees_signed.is_empty() { &v.fees_new } else { &v.fees_signed };
        fees.iter().map(|f| (f.candidate.clone(), eravon_cands::lapse::of_spelling(&f.quote))).collect()
    }

    pub fn check_image(&mut self, bytes: &[u8]) {
        self.checked = Some(image_check(self.texts().map(|(g, _)| g).as_deref(), bytes));
    }
}

impl Shown {
    pub fn wants_taken(&self) -> bool {
        self.view.as_ref().is_ok_and(|v| !v.row.draft && !v.row.listed)
    }

    pub fn fill_taken(&mut self, rows: Option<&Result<Vec<eravon_orders::table::Row>, Said>>) {
        if !self.wants_taken() {
            self.taken = None;
            return;
        }
        let Some(rows) = rows else { return };
        self.taken = Some(match rows {
            Ok(rows) => Ok(rows
                .iter()
                .filter(|r| self.hashes.iter().any(|h| h.eq_ignore_ascii_case(&r.terms_hash)) && eravon_orders::table::taken(r))
                .map(eravon_orders::table::name)
                .collect()),
            Err(e) => Err(e.clone()),
        });
    }
}

impl Shown {
    pub fn web_link(&self) -> Result<String, Said> {
        self.link()
    }

    pub fn link(&self) -> Result<String, Said> {
        let v = self.view.as_ref().map_err(Clone::clone)?;
        if let Some(l) = v.current_link() {
            return Ok(l.link.clone());
        }
        if v.row.draft {
            return Err(Said::bad("E_GOODS_DRAFT", crate::house::i18n::t(crate::house::i18n::Key::GoodsDraftNoLink)));
        }
        if !v.row.listed {
            return Err(Said::bad("E_GOODS_UNLISTED", crate::house::i18n::t(crate::house::i18n::Key::GoodsLinkOff)));
        }
        if !v.link_state.is_empty() {
            return Err(Said::bad(&v.link_state, crate::fill!(crate::house::i18n::Key::GoodsNoLinkTodo; v.pending.join(&crate::house::i18n::t(crate::house::i18n::Key::ListJoin)))));
        }
        if !v.pending.is_empty() {
            return Err(Said::bad("E_VAULT_LOCKED", crate::fill!(crate::house::i18n::Key::GoodsTodoUnlock; v.pending.join(&crate::house::i18n::t(crate::house::i18n::Key::ListJoin)))));
        }
        Err(Said::bad("E_GOODS_NO_LINK", crate::house::i18n::t(crate::house::i18n::Key::GoodsNoLink)))
    }
}

#[derive(Clone, Debug)]
pub struct Goods {
    pub rows: Result<Vec<Row>, Said>,
    pub names: Vec<(String, String)>,
    pub query: String,
    pub stages: Vec<eravon_goods::series::Written>,
    pub shown: Option<Shown>,
    pub edit: Option<GoodsEdit>,
    pub said: Option<Said>,
    pub swept: Vec<(String, Vec<String>, bool)>,
    pub ask: Option<crate::house::ask::TimeAsk>,
    pub me: String,
}

impl Goods {
    pub fn open(data: &str, addr: &str, f: &Field, now: u64, signer: &eravon_goods::sign::Signer) -> Goods {
        let mut g = Goods { rows: Ok(Vec::new()), names: Vec::new(), query: String::new(), stages: Vec::new(), shown: None, edit: None, said: None, swept: Vec::new(), ask: None, me: addr.to_string() };
        g.sweep(data, addr, f, now, signer);
        g
    }

    pub fn entered(slot: &mut Option<Goods>, addr: &str) {
        match slot.as_mut() {
            Some(g) if g.me.eq_ignore_ascii_case(addr) => {
                g.edit = None;
                g.shown = None;
            }
            _ => *slot = None,
        }
    }

    pub fn local(data: &str, addr: &str) -> Goods {
        let mut g = Goods { rows: Ok(Vec::new()), names: Vec::new(), query: String::new(), stages: Vec::new(), shown: None, edit: None, said: None, swept: Vec::new(), ask: None, me: addr.to_string() };
        g.reload(data, addr);
        g
    }

    pub fn sweep(&mut self, data: &str, addr: &str, f: &Field, now: u64, signer: &eravon_goods::sign::Signer) {
        let c = ctx(data, addr, f, now, signer);
        match g1::sweep(&c) {
            Ok(x) => self.swept = x,
            Err(e) => self.said = Some(Said::bad(e.name(), e.detail())),
        }
        self.reload(data, addr);
        self.refresh(data, addr, now);
    }

    fn reload(&mut self, data: &str, addr: &str) {
        self.rows = g1::list(data, addr, &self.query).map_err(|e| Said::bad(e.name(), e.detail()));
        self.names = g1::names(data, addr).unwrap_or_default();
        self.stages = eravon_goods::series::stages(data, addr);
    }

    pub fn series_known(&self) -> Vec<eravon_goods::series::Known> {
        let own = self.edit.as_ref().and_then(|e| e.id.clone());
        eravon_goods::series::known(&self.stages, own.as_deref())
    }

    fn refresh(&mut self, data: &str, addr: &str, now: u64) {
        if let Some(s) = self.shown.as_mut() {
            s.view = g1::view(data, addr, &s.id, now).map_err(|e| Said::bad(e.name(), e.detail()));
            s.hashes = g1::terms_hashes(data, addr, &s.id).unwrap_or_default();
            s.taken = None;
        }
    }

    pub fn search(&mut self, data: &str, addr: &str, q: &str) {
        self.query = q.to_string();
        self.reload(data, addr);
    }

    pub fn show(&mut self, data: &str, addr: &str, id: &str, now: u64) {
        self.edit = None;
        self.shown = Some(Shown {
            id: id.to_string(),
            view: g1::view(data, addr, id, now).map_err(|e| Said::bad(e.name(), e.detail())),
            qr: None,
            opened: None,
            copied: None,
            hashes: g1::terms_hashes(data, addr, id).unwrap_or_default(),
            taken: None,
            regs: None,
            exported: None,
            inner: None,
            checked: None,
        });
    }

    pub fn company_check(&mut self, data: &str, addr: &str, now: u64, done_word: &str) {
        let Some(id) = self.shown.as_ref().map(|s| s.id.clone()) else { return };
        let k = eravon_goods::sign::Signer::locked();
        let c = g1::Ctx { data, writer: seam::WRITER, now, me: addr, registries: &[], signer: &k, route: eravon_watch::wire_parts(data, addr) };
        let r = g1::company_check(&c, &id).map_err(|e| Said::bad(e.name(), e.detail()));
        self.said = Some(match &r {
            Ok((_, true)) => Said::ok(done_word),
            Ok((rows, false)) => Said::bad("E_GOODS_AWAITING", crate::fill!(crate::house::i18n::Key::GoodsAwaitCompany; rows.iter().map(|x| format!("{} {}", &x.digest[..10.min(x.digest.len())], x.state)).collect::<Vec<_>>().join(&crate::house::i18n::t(crate::house::i18n::Key::ListJoin)))),
            Err(e) => e.clone(),
        });
        self.refresh(data, addr, now);
        if let Some(s) = self.shown.as_mut() {
            s.regs = Some(r.map(|(rows, _)| rows));
        }
    }

    pub fn export_proposal(&mut self, data: &str, addr: &str, dir: &str, ok_word: &str) {
        self.export_proposal_as(data, addr, dir, ok_word, false);
    }

    pub fn ask_answer(&mut self, data: &str, addr: &str, go: bool, ok_word: &str) {
        let Some(a) = self.ask.take() else { return };
        if !go {
            self.said = Some(Said::ok(crate::house::i18n::t(crate::house::i18n::Key::SaidCanceled)));
            return;
        }
        if let crate::house::ask::Replay::Goods { dir, .. } = a.replay {
            self.export_proposal_as(data, addr, &dir, ok_word, true);
        }
    }

    fn export_proposal_as(&mut self, data: &str, addr: &str, dir: &str, ok_word: &str, confirmed: bool) {
        let Some(s) = self.shown.as_mut() else { return };
        match g1::proposal_time(data, addr, &s.id) {
            Ok(Some((bound, short))) if eravon_cli::company::asks(confirmed) => {
                let say = eravon_cli::company::ask_say(bound, &short);
                self.said = Some(crate::house::ask::said(&say));
                self.ask = Some(crate::house::ask::TimeAsk { say, replay: crate::house::ask::Replay::Goods { id: s.id.clone(), dir: dir.to_string() } });
                return;
            }
            Err(e) => {
                self.said = Some(Said::bad(e.name(), e.detail()));
                return;
            }
            _ => {}
        }
        self.ask = None;
        let r = g1::export_proposal(data, addr, &s.id, dir).map_err(|e| Said::bad(e.name(), e.detail()));
        self.said = Some(match &r {
            Ok(p) => Said::ok(format!("{ok_word} · {p}")),
            Err(e) => e.clone(),
        });
        s.exported = Some(r);
    }

    pub fn back(&mut self) {
        self.shown = None;
    }

    pub fn copy(&mut self, ok_word: &str) -> Option<String> {
        let s = self.shown.as_mut()?;
        match s.link() {
            Ok(l) => {
                s.copied = Some(l.clone());
                self.said = Some(Said::ok(ok_word));
                Some(l)
            }
            Err(x) => {
                self.said = Some(x);
                None
            }
        }
    }

    pub fn preview(&mut self) -> Option<String> {
        let s = self.shown.as_mut()?;
        match s.web_link() {
            Ok(l) => {
                s.opened = Some(l.clone());
                Some(l)
            }
            Err(x) => {
                self.said = Some(x);
                None
            }
        }
    }

    pub fn qr(&mut self) {
        let Some(s) = self.shown.as_mut() else { return };
        match s.web_link() {
            Ok(l) => {
                let fits = eravon_goods::link::qr_fits(&l);
                let modules = if fits {
                    match eravon_goods::link::qr_made(&l) {
                        Ok(m) => Some(m),
                        Err((n, d)) => {
                            self.said = Some(Said::bad(&n, d));
                            None
                        }
                    }
                } else {
                    None
                };
                s.qr = Some(Qr { link: l, fits, modules, saved: None, image: None });
            }
            Err(x) => self.said = Some(x),
        }
    }

    pub fn qr_close(&mut self) {
        if let Some(s) = self.shown.as_mut() {
            s.qr = None;
        }
    }

    pub fn qr_save(&mut self, path: &str, ok_word: &str) {
        let Some(q) = self.shown.as_mut().and_then(|s| s.qr.as_mut()) else { return };
        let r = if std::fs::symlink_metadata(path).is_ok() {
            Err(Said::bad("E_OCCUPIED", path.to_string()))
        } else {
            eravon_goods::link::qr_png(&q.link, path).map(|n| (path.to_string(), n)).map_err(|(n, d)| Said::bad(&n, d))
        };
        self.said = Some(match &r {
            Ok(_) => Said::ok(ok_word),
            Err(x) => x.clone(),
        });
        q.saved = Some(r);
    }

    pub fn qr_copy(&mut self) {
        let Some(q) = self.shown.as_mut().and_then(|s| s.qr.as_mut()) else { return };
        let r = eravon_goods::link::qr_pixels(&q.link).map_err(|(n, d)| Said::bad(&n, d));
        if let Err(x) = &r {
            self.said = Some(x.clone());
        }
        q.image = Some(r);
    }

    pub fn set_listed(&mut self, data: &str, addr: &str, now: u64, on: bool, word: &str) {
        let Some(id) = self.shown.as_ref().map(|s| s.id.clone()) else { return };
        let k = eravon_goods::sign::Signer::locked();
        let c = g1::Ctx { data, writer: seam::WRITER, now, me: addr, registries: &[], signer: &k, route: eravon_watch::wire_parts(data, addr) };
        self.said = Some(match g1::set_listed(&c, &id, on) {
            Ok(()) => Said::ok(word),
            Err(e) => Said::bad(e.name(), e.detail()),
        });
        self.reload(data, addr);
        self.refresh(data, addr, now);
    }

    pub fn delete(&mut self, data: &str, addr: &str, now: u64, word: &str) {
        let Some(id) = self.shown.as_ref().map(|s| s.id.clone()) else { return };
        let k = eravon_goods::sign::Signer::locked();
        let c = g1::Ctx { data, writer: seam::WRITER, now, me: addr, registries: &[], signer: &k, route: eravon_watch::wire_parts(data, addr) };
        match g1::delete(&c, &id) {
            Ok(()) => {
                self.said = Some(Said::ok(word));
                self.shown = None;
            }
            Err(e) => self.said = Some(Said::bad(e.name(), e.detail())),
        }
        self.reload(data, addr);
    }

    pub fn new_one(&mut self) {
        self.edit = Some(GoodsEdit { id: None, draft: Draft::default(), tab: 0, last: None, ask: None, dirty: false, stored: Draft::open(&Draft::default().seal()) });
    }

    pub fn open_one(&mut self, data: &str, addr: &str, id: &str) {
        match g1::load_edit(data, addr, id) {
            Ok(d) => self.edit = Some(GoodsEdit { id: Some(id.to_string()), stored: Some(d.clone()), draft: d, tab: 0, last: None, ask: None, dirty: false }),
            Err(e) => self.said = Some(Said::bad(e.name(), e.detail())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save(
        &mut self,
        data: &str,
        addr: &str,
        f: &Field,
        now: u64,
        signer: &eravon_goods::sign::Signer,
        ok_word: &str,
        draft_word: &str,
    ) {
        self.store(data, addr, f, now, signer, ok_word, draft_word, false);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn publish(
        &mut self,
        data: &str,
        addr: &str,
        f: &Field,
        now: u64,
        signer: &eravon_goods::sign::Signer,
        key: &crate::house::orders::Key,
        board: bool,
        ok_word: &str,
        draft_word: &str,
    ) {
        self.store(data, addr, f, now, signer, ok_word, draft_word, true);
        if !board {
            return;
        }
        let published = self.said.as_ref().is_some_and(|s| s.tone == crate::house::seller::Tone::Ok);
        let id = self.edit.as_ref().and_then(|e| e.last.as_ref()).filter(|sv| published && !sv.draft && sv.company_refused.is_none() && !g1::company_version(data, addr, &sv.id)).map(|sv| sv.id.clone());
        if let (Some(id), true) = (id, key.is_open()) {
            self.post(data, addr, f, now, signer, key, &id, ok_word);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn post(&mut self, data: &str, addr: &str, f: &Field, now: u64, signer: &eravon_goods::sign::Signer, key: &crate::house::orders::Key, id: &str, ok_word: &str) {
        let c = ctx(data, addr, f, now, signer);
        let got = crate::house::orders::wire(data, addr).and_then(|w| {
            let live = || signer.live();
            let mut send = crate::house::orders::GoodsSend { key, addr, w: &w, live: &live };
            g1::post(&c, id, &mut send).map_err(|e| { let (n, d) = e.said(); Said::bad(&n, d) })
        });
        self.said = Some(match got {
            Ok(p) => Said::ok(&format!("{ok_word} · {}", p.tx)),
            Err(s) => s,
        });
        self.reload(data, addr);
        self.refresh(data, addr, now);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn confirm_fees(&mut self, data: &str, addr: &str, f: &Field, now: u64, signer: &eravon_goods::sign::Signer, id: &str, ok_word: &str) {
        let c = ctx(data, addr, f, now, signer);
        self.said = Some(match g1::confirm_fees(&c, id) {
            Ok(sv) if sv.pending.iter().all(|p| p == g1::POST) => Said::ok(ok_word),
            Ok(sv) => {
                let (n, d) = sv.link_state.unwrap_or_else(|| ("E_GOODS_PENDING".into(), sv.pending.join(",")));
                Said::bad(&n, d)
            }
            Err(e) => Said::bad(e.name(), e.detail()),
        });
        self.reload(data, addr);
        self.refresh(data, addr, now);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn revoke(&mut self, data: &str, addr: &str, now: u64, key: &crate::house::orders::Key, live: &dyn Fn() -> bool, word: &str, dir: Result<String, Said>, at_word: &str) {
        let Some(id) = self.shown.as_ref().map(|s| s.id.clone()) else { return };
        let k = eravon_goods::sign::Signer::locked();
        let c = g1::Ctx { data, writer: seam::WRITER, now, me: addr, registries: &[], signer: &k, route: eravon_watch::wire_parts(data, addr) };
        if g1::company_version(data, addr, &id) {
            let got = dir.and_then(|d| g1::revoke_proposal(&c, &id, &d).map_err(|e| Said::bad(e.name(), e.detail())));
            self.said = Some(match &got {
                Ok(p) => Said::ok(format!("{at_word} · {p}")),
                Err(e) => e.clone(),
            });
            if let (Ok(p), Some(s)) = (got, self.shown.as_mut()) {
                s.exported = Some(Ok(p));
            }
            self.reload(data, addr);
            return;
        }
        let got = crate::house::orders::wire(data, addr).and_then(|w| {
            let mut send = crate::house::orders::GoodsSend { key, addr, w: &w, live };
            g1::revoke(&c, &id, &mut send).map_err(|e| { let (n, d) = e.said(); Said::bad(&n, d) })
        });
        self.said = Some(match got {
            Ok(tx) => Said::ok(&format!("{word} · {tx}")),
            Err(s) => s,
        });
        self.reload(data, addr);
        self.refresh(data, addr, now);
    }

    #[allow(clippy::too_many_arguments)]
    fn store(&mut self, data: &str, addr: &str, f: &Field, now: u64, signer: &eravon_goods::sign::Signer, ok_word: &str, draft_word: &str, publish: bool) {
        let Some(e) = self.edit.as_mut() else { return };
        let c = ctx(data, addr, f, now, signer);
        let got = if publish {
            e.dirty = g1::dirty(e.stored.as_ref(), &e.draft);
            g1::publish_recorded(&c, e.id.as_deref(), &e.draft)
        } else {
            g1::save(&c, e.id.as_deref(), &e.draft)
        };
        match got {
            Ok(sv) => {
                self.said = Some(if sv.draft { Said { tone: crate::house::seller::Tone::Note, name: "draft".into(), detail: draft_word.into() } } else { Said::ok(ok_word) });
                e.id = Some(sv.id.clone());
                e.last = Some(sv);
                e.stored = g1::stored(data, addr, e.id.as_deref());
            }
            Err(x) => self.said = Some(Said::bad(x.name(), x.detail())),
        }
        self.reload(data, addr);
        self.refresh(data, addr, now);
    }

    pub fn dirty(&self) -> bool {
        self.edit.as_ref().is_some_and(|e| g1::dirty(e.stored.as_ref(), &e.draft))
    }

    pub fn mark_dirty(&mut self) {
        let d = self.dirty();
        if let Some(e) = self.edit.as_mut() {
            e.dirty = d;
        }
    }

    pub(crate) fn request_leave(&mut self, to: Leave) -> bool {
        if self.edit.is_none() {
            return true;
        }
        if self.dirty() {
            if let Some(e) = self.edit.as_mut() {
                e.ask = Some(to);
            }
            return false;
        }
        self.edit = None;
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn answer(
        &mut self,
        data: &str,
        addr: &str,
        f: &Field,
        now: u64,
        signer: &eravon_goods::sign::Signer,
        a: &str,
        ok_word: &str,
        draft_word: &str,
    ) -> Option<Leave> {
        let to = self.edit.as_ref().and_then(|e| e.ask.clone())?;
        match a {
            "save" => {
                self.save(data, addr, f, now, signer, ok_word, draft_word);
                if self.dirty() {
                    if let Some(e) = self.edit.as_mut() {
                        e.ask = None;
                    }
                    return None;
                }
                self.edit = None;
                Some(to)
            }
            "discard" => {
                self.edit = None;
                Some(to)
            }
            _ => {
                if let Some(e) = self.edit.as_mut() {
                    e.ask = None;
                }
                None
            }
        }
    }

    pub fn rows(&self) -> Out {
        let mut m = vec![(
            "rows",
            match &self.rows {
                Ok(list) => Out::A(
                    list.iter()
                        .map(|r| {
                            Out::O(vec![
                                ("candNames", Out::A(r.cands.iter().map(|n| Out::S(n.clone())).collect())),
                                ("cands", Out::N(r.cands.len() as u64)),
                                ("description", Out::S(r.description.clone())),
                                ("draft", Out::B(r.draft)),
                                ("id", Out::S(r.id.clone())),
                                ("listed", Out::B(r.listed)),
                                ("name", Out::S(r.name.clone())),
                                ("pending", Out::A(r.pending.iter().map(|p| Out::S(p.clone())).collect())),
                                ("price", Out::S(r.price.clone())),
                                ("token", Out::S(r.token.clone())),
                                ("updatedAt", Out::N(r.updated)),
                                ("version", Out::N(r.version)),
                            ])
                        })
                        .collect(),
                ),
                Err(s) => s.out(),
            },
        )];
        if let Some(e) = &self.edit {
            let mut x = vec![
                ("dirty", Out::B(e.dirty)),
                ("draft", draft_out(&e.draft)),
                ("id", Out::S(e.id.clone().unwrap_or_default())),
                ("tab", Out::N(e.tab as u64)),
            ];
            if let Some(a) = &e.ask {
                x.push(("ask", Out::S(a.name())));
            }
            if let Some(sv) = &e.last {
                let mut l = vec![
                    ("cells", sv.verdict.rows()),
                    ("draft", Out::B(sv.draft)),
                    (
                        "excluded",
                        Out::A(
                            sv.resolved
                                .excluded
                                .iter()
                                .map(|o| Out::O(vec![("address", Out::S(o.address.clone())), ("detail", Out::S(o.detail.clone())), ("name", Out::S(o.name.clone()))]))
                                .collect(),
                        ),
                    ),
                    (
                        "guilds",
                        Out::A(
                            sv.resolved
                                .guilds
                                .iter()
                                .map(|g| {
                                    Out::O(vec![
                                        ("amanahHash", Out::S(g.amanah_hash.clone())),
                                        ("panelHash", Out::S(g.panel_hash.clone())),
                                        ("pointer", Out::S(g.pointer.clone())),
                                        ("seq", Out::N(g.seq)),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    (
                        "judges",
                        Out::A(
                            sv.resolved
                                .judges
                                .iter()
                                .map(|j| {
                                    let mut r = vec![("amanahHash", Out::S(j.amanah_hash.clone())), ("judge", Out::S(j.judge.clone()))];
                                    if let Some(src) = &j.source {
                                        r.push(("source", Out::S(src.clone())));
                                    }
                                    Out::O(r)
                                })
                                .collect(),
                        ),
                    ),
                    ("fees", Out::A(sv.resolved.fees.iter().map(fee_out).collect())),
                    ("version", Out::N(sv.version)),
                ];
                if let Some(h) = &sv.terms_hash {
                    l.push(("termsHash", Out::S(h.clone())));
                }
                if let Some(k) = &sv.link {
                    l.push(("link", Out::S(k.clone())));
                }
                if let Some((n, d)) = &sv.company_refused {
                    l.push(("companyRefused", Out::O(vec![("detail", Out::S(d.clone())), ("name", Out::S(n.clone()))])));
                }
                if let Some((n, d)) = &sv.link_state {
                    l.push(("linkState", Out::O(vec![("detail", Out::S(d.clone())), ("name", Out::S(n.clone()))])));
                }
                l.push(("pending", Out::A(sv.pending.iter().map(|p| Out::S(p.clone())).collect())));
                x.push(("last", Out::O(l)));
            }
            m.push(("edit", Out::O(x)));
        }
        m.push(("query", Out::S(self.query.clone())));
        if let Some(s) = &self.shown {
            m.push(("shown", shown_out(s)));
        }
        m.push((
            "swept",
            Out::A(
                self.swept
                    .iter()
                    .map(|(id, why, done)| {
                        Out::O(vec![
                            ("done", Out::B(*done)),
                            ("id", Out::S(id.clone())),
                            ("why", Out::A(why.iter().map(|w| Out::S(w.clone())).collect())),
                        ])
                    })
                    .collect(),
            ),
        ));
        if let Some(x) = crate::house::ask::out(&self.ask) {
            m.push(("timeAsk", x));
        }
        if let Some(s) = &self.said {
            m.push(("said", s.out()));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}

pub(super) fn ctx<'a>(data: &'a str, addr: &'a str, f: &'a Field, now: u64, signer: &'a eravon_goods::sign::Signer) -> g1::Ctx<'a> {
    g1::Ctx { data, writer: seam::WRITER, now, me: addr, registries: &f.registries, signer, route: eravon_watch::wire_parts(data, addr) }
}

pub(super) fn shown_out(s: &Shown) -> Out {
    let mut m = vec![("id", Out::S(s.id.clone()))];
    if let Some(i) = &s.inner {
        m.push(("inner", Out::S(i.word().into())));
        if let crate::house::seller::GoodsInner::Lapse(c) = i {
            m.push(("lapseOf", Out::S(c.clone())));
        }
    }
    if let Some(c) = &s.checked {
        m.push(("checked", match c {
            Ok(n) => Out::O(vec![("name", Out::S(n.clone()))]),
            Err(x) => Out::O(vec![("refused", x.out())]),
        }));
    }
    match &s.view {
        Err(x) => m.push(("view", x.out())),
        Ok(v) => {
            m.push(("draft", Out::B(v.row.draft)));
            m.push(("editing", Out::B(v.editing)));
            m.push(("listed", Out::B(v.row.listed)));
            m.push(("name", Out::S(v.row.name.clone())));
            m.push(("price", Out::S(v.row.price.clone())));
            m.push(("revoked", Out::S(v.revoked.clone().unwrap_or_default())));
            m.push(("linkState", Out::S(v.link_state.clone())));
            m.push(("pending", Out::A(v.pending.iter().map(|p| Out::S(p.clone())).collect())));
            m.push(("feesSigned", Out::A(v.fees_signed.iter().map(fee_out).collect())));
            if !v.fees_new.is_empty() {
                m.push(("feesNew", Out::A(v.fees_new.iter().map(fee_out).collect())));
            }
            m.push((
                "links",
                Out::A(
                    v.links
                        .iter()
                        .map(|l| {
                            Out::O(vec![
                                ("bytes", Out::N(l.link.len() as u64)),
                                ("expiry", Out::N(l.expiry)),
                                ("link", Out::S(l.link.clone())),
                                ("postedTx", Out::S(l.posted_tx.clone())),
                                ("qrFits", Out::B(eravon_goods::link::qr_fits(&l.link))),
                                ("version", Out::N(l.version)),
                            ])
                        })
                        .collect(),
                ),
            ));
            m.push(("version", Out::N(v.row.version)));
            if let Some((g, _)) = s.texts() {
                m.push(("goodsRefs", crate::house::case::refs_out(&eravon_goods::refs::read(&g))));
            }
            m.push((
                "lapses",
                Out::A(
                    s.lapses()
                        .iter()
                        .map(|(c, r)| match r {
                            Ok(x) => Out::O(vec![("candidate", Out::S(c.clone())), ("terms", crate::house::case::lapse_out(x, c.starts_with("galeed:")))]),
                            Err(e) => Out::O(vec![("candidate", Out::S(c.clone())), ("refused", Out::S(e.clone()))]),
                        })
                        .collect(),
                ),
            ));
            if let Some((state, address, digests)) = &v.company {
                m.push((
                    "company",
                    Out::O(vec![
                        ("address", Out::S(address.clone())),
                        ("awaiting", Out::B(v.awaiting_company())),
                        ("digests", Out::A(digests.iter().map(|d| Out::S(d.clone())).collect())),
                        ("state", Out::S(state.clone())),
                    ]),
                ));
            }
        }
    }
    match &s.regs {
        Some(Ok(rows)) => m.push((
            "regs",
            Out::A(rows.iter().map(|x| Out::O(vec![("digest", Out::S(x.digest.clone())), ("state", Out::S(x.state.clone())), ("why", Out::S(x.why.clone()))])).collect()),
        )),
        Some(Err(e)) => m.push(("regs", e.out())),
        None => {}
    }
    match &s.exported {
        Some(Ok(p)) => m.push(("exported", Out::S(p.clone()))),
        Some(Err(e)) => m.push(("exported", e.out())),
        None => {}
    }
    if let Some(q) = &s.qr {
        let mut x = vec![("fits", Out::B(q.fits)), ("side", Out::N(q.modules.as_ref().map_or(0, |(n, _)| *n as u64)))];
        match &q.saved {
            Some(Ok((p, n))) => x.push(("saved", Out::O(vec![("path", Out::S(p.clone())), ("px", Out::N(*n as u64))]))),
            Some(Err(e)) => x.push(("saved", e.out())),
            None => {}
        }
        match &q.image {
            Some(Ok((side, px))) => x.push(("image", Out::O(vec![("bytes", Out::N(px.len() as u64)), ("px", Out::N(*side as u64))]))),
            Some(Err(e)) => x.push(("image", e.out())),
            None => {}
        }
        m.push(("qr", Out::O(x)));
    }
    if let Some(o) = &s.opened {
        m.push(("opened", Out::S(o.clone())));
    }
    if let Some(c) = &s.copied {
        m.push(("copied", Out::S(c.clone())));
    }
    match &s.taken {
        Some(Ok(v)) => m.push(("taken", Out::A(v.iter().map(|x| Out::S(x.clone())).collect()))),
        Some(Err(e)) => m.push(("taken", e.out())),
        None => {}
    }
    m.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(m)
}

pub(crate) fn draft_out(d: &Draft) -> Out {
    let st = |x: &str| Out::S(x.to_string());
    Out::O(vec![
        ("cands", Out::A(d.cands.iter().map(|c| st(c)).collect())),
        ("deliveryNote", st(&d.delivery_note)),
        ("deliveryPrivate", Out::B(d.delivery_private)),
        ("description", st(&d.description)),
        ("goodsPrivate", Out::B(d.goods_private)),
        ("goodsRefs", crate::house::case::refs_out(&d.refs())),
        ("ladder", Out::A(d.ladder.iter().map(|t| Out::O(vec![("name", st(&t.name)), ("pct", st(&t.pct))])).collect())),
        ("name", st(&d.name)),
        ("price", st(&d.price)),
        ("t1", st(&d.t1)),
        ("t1h", st(&d.t1h)),
        ("t1m", st(&d.t1m)),
        ("t2", st(&d.t2)),
        ("t2h", st(&d.t2h)),
        ("t2m", st(&d.t2m)),
        ("token", st(&d.token)),
    ])
}

fn fee_out(f: &eravon_goods::resolve::Fee) -> Out {
    let mut r = vec![("amount", Out::S(f.amount.to_dec())), ("candidate", Out::S(f.candidate.clone())), ("token", Out::S(f.token.clone()))];
    match &f.priced {
        Ok(q) => {
            r.push(("deposit", Out::S(q.deposit.to_dec())));
            r.push(("lf", Out::S(q.lf.to_dec())));
            r.push(("premium", Out::S(q.premium.to_dec())));
            r.push(("pricedAtOpen", Out::B(q.rate.is_some())));
            if let Some(x) = &q.rate {
                r.push(("rateAt", Out::N(x.rate_at)));
            }
        }
        Err((n, d)) => r.push(("why", Out::O(vec![("detail", Out::S(d.clone())), ("name", Out::S(n.clone()))]))),
    }
    r.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(r)
}

pub fn list_line(body: &str) -> String {
    use crate::md::Block;
    let blocks = crate::md::blocks(body);
    let para = blocks.iter().find_map(|b| match b {
        Block::Para(s) => Some(s.clone()),
        _ => None,
    });
    let first = para.or_else(|| {
        blocks.into_iter().find_map(|b| match b {
            Block::Heading(_, s) | Block::Item(s) | Block::Quote(s) | Block::Para(s) => Some(s),
            Block::Row(cells) => Some(cells.join(" ")),
            Block::Blank => None,
        })
    });
    first.map(|s| s.replace("**", "").replace('`', "").trim().to_string()).unwrap_or_default()
}

#[cfg(test)]
mod left_out_says_why {
    use crate::house::i18n::Lang;

    #[test]
    fn a_candidate_kept_out_of_the_terms_says_why_in_the_pages_language() {
        let x = |name: &str| eravon_goods::resolve::Excluded { address: "0xab".into(), name: name.into(), detail: "0xab 暂停接单".into() };
        let rows = super::left_out(Lang::En, &[x("E_GOODS_NOT_ACCEPTING")]);
        assert_eq!(rows, vec![("0xab".to_string(), crate::house::refusal::what(Lang::En, "E_GOODS_NOT_ACCEPTING").expect("in the table"))]);
        assert!(!rows[0].1.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{rows:?}");
        assert_eq!(super::left_out(Lang::Zh, &[x("E_GOODS_NOT_ACCEPTING")])[0].1, crate::house::refusal::what(Lang::Zh, "E_GOODS_NOT_ACCEPTING").expect("in the table"), "a Chinese page too says the table's words, not the sentence with the address");
        for n in ["E_GOODS_NOT_ACCEPTING", "E_GOODS_LEDGER_UNREAD", "E_GOODS_NOT_IN_TABLE", "E_GOODS_NO_QUOTE", "E_GOODS_QUOTE_UNREAD"] {
            for lang in [Lang::Zh, Lang::En] {
                assert!(crate::house::refusal::what(lang, n).is_some_and(|w| !w.is_empty()) && crate::house::refusal::next(lang, n).is_some_and(|w| !w.is_empty()), "{n} {lang:?}");
            }
        }
    }
}

pub(crate) fn image_check(goods: Option<&str>, bytes: &[u8]) -> Result<String, Said> {
    match goods.map(|g| eravon_goods::refs::read(g).refs) {
        Some(refs) if !refs.images.is_empty() => match eravon_goods::refs::check(&refs, bytes) {
            Some(i) => Ok(i.name.clone()),
            None => Err(Said::bad("E_GOODS_IMAGE_DIFFERS", crate::house::i18n::t(crate::house::i18n::Key::ErrEGOODSIMAGEDIFFERS))),
        },
        _ => Err(Said::bad("E_GOODS_NO_IMAGES", crate::house::i18n::t(crate::house::i18n::Key::ErrEGOODSNOIMAGES))),
    }
}

#[cfg(test)]
mod qr_way_out {
    use super::*;

    #[test]
    fn the_list_takes_one_plain_line_of_a_description_written_in_headings_and_items() {
        let said = list_line("### 规格\n\n- **重量** 3 克\n- 颜色:黑\n\n正文一段。");
        assert_eq!(said, "正文一段。");
        assert!(!said.contains('\n') && !said.contains('#') && !said.contains("- "), "{said}");
        assert_eq!(list_line("\n\n- **重量** 3 克\n- 颜色"), "重量 3 克");
        assert_eq!(list_line("  \n"), "");
    }

    fn with_qr(link: &str) -> Goods {
        let shown = Shown {
            id: "g1".into(),
            view: Err(Said::bad("E_TEST", "not read")),
            qr: Some(Qr { link: link.into(), fits: true, modules: None, saved: None, image: None }),
            opened: None,
            copied: None,
            hashes: Vec::new(),
            taken: None,
            regs: None,
            exported: None,
            inner: None,
            checked: None,
        };
        Goods { rows: Ok(Vec::new()), names: Vec::new(), query: String::new(), stages: Vec::new(), shown: Some(shown), edit: None, said: None, swept: Vec::new(), ask: None, me: String::new() }
    }

    #[test]
    fn a_save_where_that_name_is_taken_is_refused_and_overwrites_nothing() {
        let dir = std::env::temp_dir().join(format!("eravon-qr-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp folder");
        let at = crate::house::seam::qr_file(&dir.display().to_string(), "g1", 3);
        assert_eq!(std::path::Path::new(&at).file_name().and_then(|n| n.to_str()), Some("g1-v3.png"));
        let mut g = with_qr("eravon://open?e=abc");
        g.qr_save(&at, "saved");
        assert!(matches!(g.shown.as_ref().and_then(|s| s.qr.as_ref()).and_then(|q| q.saved.as_ref()), Some(Ok(_))), "{:?}", g.said);
        let first = std::fs::read(&at).expect("the image is there");
        std::fs::write(&at, b"an earlier picture").expect("stand-in");
        g.qr_save(&at, "saved");
        assert_eq!(g.said.as_ref().map(|s| s.name.as_str()), Some("E_OCCUPIED"));
        assert_eq!(std::fs::read(&at).expect("still there"), b"an earlier picture");
        assert!(first.starts_with(b"\x89PNG"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_copied_image_is_the_saved_one_and_the_step_claims_nothing() {
        let link = "eravon://open?e=abc";
        let mut g = with_qr(link);
        g.qr_copy();
        assert!(g.said.is_none());
        let image = g.shown.as_ref().and_then(|s| s.qr.as_ref()).and_then(|q| q.image.clone());
        let want = eravon_goods::link::qr_pixels(link).expect("fits");
        assert_eq!(image, Some(Ok(want.clone())));
        assert_eq!(want.1.len(), want.0 * want.0);
    }
}

crate::house::merge::lands!(Goods {
    whole: [me],
    data: [rows, names, stages, swept, ask],
    page: [query, edit, shown],
    keyed(|_: &Goods| ()): [],
    nested: [],
    said: [said],
});
