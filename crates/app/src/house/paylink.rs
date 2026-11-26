use eravon_link as k3;
use eravon_literals::json::Out;
use eravon_terms as k2;

use crate::house::seller::Said;


mod import;
mod company;
mod verify;
pub use verify::{deposit_gap, land_link, open_plan, verify, verify_listing, NOT_LISTING};
use verify::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Pay,
    Import,
}

impl Tab {
    pub const fn name(self) -> &'static str {
        match self {
            Tab::Pay => crate::house::seam::PL_TAB_PAY,
            Tab::Import => crate::house::seam::PL_TAB_IMPORT,
        }
    }

    pub fn of(s: &str) -> Option<Tab> {
        match s {
            crate::house::seam::PL_TAB_PAY => Some(Tab::Pay),
            crate::house::seam::PL_TAB_IMPORT => Some(Tab::Import),
            _ => None,
        }
    }
}

pub use eravon_cli::deskops::paypage::{Cand, Opened, State};

pub fn cand_why(c: &Cand, lang: crate::house::i18n::Lang) -> String {
    if c.refusal.is_empty() {
        crate::house::i18n::library(lang, &c.note)
    } else {
        crate::house::refusal::line(lang, &c.refusal, &c.note)
    }
}

pub trait Why {
    fn why(&self, lang: crate::house::i18n::Lang) -> String;
}

impl Why for Cand {
    fn why(&self, lang: crate::house::i18n::Lang) -> String {
        cand_why(self, lang)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Import {
    pub file: Option<(String, Vec<u8>)>,
    pub said: Option<Said>,
    pub done: Option<eravon_import::import::Done>,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaidView {
    pub buyer_key: String,
    pub bumped: bool,
    pub instance: String,
    pub case_id: String,
    pub id: String,
    pub stop: &'static str,
    pub pay_tx: String,
    pub open_tx: String,
    pub short_address: String,
    pub gap: String,
    pub decimals: Option<u64>,
    pub stale: Option<Said>,
}

#[derive(Clone, Debug, Default)]
pub struct PayLink {
    pub ticket: crate::house::Ticket,
    pub owner: String,
    pub history: crate::house::history::Hist,
    pub tab: Tab,
    pub link: String,
    pub wants_open: bool,
    pub opened: Option<Result<Opened, Said>>,
    pub opened_for: String,
    pub pick: Option<usize>,
    pub payout: String,
    pub payout_said: Option<Said>,
    pub payout_tried: String,
    pub import: Import,
    pub key_for: Option<Result<eravon_cli::buyer::Pick, Said>>,
    pub paid: Option<Result<PaidView, Said>>,
    pub form: Option<eravon_pay::company::Form>,
    pub export_dir: String,
    pub company: Option<Result<CoView, Said>>,
    pub company_on: bool,
    pub co_open: Option<Result<String, Said>>,
    pub ask: Option<crate::house::ask::TimeAsk>,
    pub export_why: Option<Said>,
    pub held: Vec<(&'static str, String)>,
    pub inner: Option<PlInner>,
    pub checked: Option<Result<String, Said>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlInner {
    Doc(bool),
    Lapse(String),
    History(String),
}

impl PlInner {
    pub fn word(&self) -> &'static str {
        match self {
            PlInner::Doc(true) => "goodsText",
            PlInner::Doc(false) => "deliveryText",
            PlInner::Lapse(_) => "lapse",
            PlInner::History(_) => "history",
        }
    }
}

impl PayLink {
    pub fn segment(&self, goods: bool) -> Option<crate::house::case::Segment> {
        let o = self.verified().as_ref()?.as_ref().ok()?;
        let doc = eravon_terms::doc::terms(o.terms_text.as_ref()?.as_bytes()).ok()?;
        let name = if goods { "goods" } else { "delivery" };
        crate::house::case::segment_of(&doc, goods, self.held.iter().find(|(s, _)| *s == name).map(|(_, t)| t))
    }

    pub fn cand_label(&self, addr: &str) -> String {
        match &self.verified() {
            Some(Ok(o)) => o.cands.iter().find(|c| c.address.eq_ignore_ascii_case(addr)).map(Cand::label).unwrap_or_else(|| eravon_orders::table::short(addr)),
            _ => eravon_orders::table::short(addr),
        }
    }

    pub fn lapses(&self) -> Vec<(String, bool, Result<eravon_cands::lapse::Clocks, String>)> {
        let Some(Ok(o)) = &self.verified() else { return Vec::new() };
        o.cands
            .iter()
            .map(|c| {
                let ck = match o.quotes.get(c.n) {
                    Some(Some(q)) => eravon_cands::lapse::clocks(q),
                    _ => Err(crate::house::i18n::t(crate::house::i18n::Key::SetNotRead)),
                };
                (c.address.clone(), c.kind == "guild", ck)
            })
            .collect()
    }

    pub fn takes_drop(&self) -> bool {
        takes_drop(self.inner.as_ref(), self.segment(false).map(|s| s.text.is_some()))
    }

    pub fn lapse_open(&mut self, who: &str) -> Result<(), String> {
        match self.lapses().into_iter().find(|(a, _, _)| a.eq_ignore_ascii_case(who)) {
            Some((a, _, Ok(_))) => {
                self.inner = Some(PlInner::Lapse(a));
                Ok(())
            }
            Some((_, _, Err(e))) => Err(e),
            None => Err(format!("这条链接的候选里没有 {who}")),
        }
    }

    pub fn check_image(&mut self, bytes: &[u8]) {
        let text = self.segment(true).and_then(|s| s.text);
        self.checked = Some(crate::house::seller::goods::image_check(text.as_deref(), bytes));
    }
}

fn same_hash(a: &str, b: &str) -> bool {
    a.trim_start_matches("0x").eq_ignore_ascii_case(b.trim_start_matches("0x"))
}

fn hold_text(held: &mut Vec<(&'static str, String)>, segments: &[(&'static str, String)], path: &str) -> Result<&'static str, Said> {
    let text = std::fs::read_to_string(path).map_err(|e| Said::bad("E_PL_SEGMENT_READ", crate::fill!(crate::house::i18n::Key::PlSegUnread; e.to_string())))?;
    let h = eravon_goods::terms::segment_hash(&text);
    let seg = segments.iter().find(|(_, want)| same_hash(want, &h)).map(|(s, _)| *s).ok_or_else(|| Said::bad("E_PL_SEGMENT_HASH", crate::house::i18n::t(crate::house::i18n::Key::PlSegBad)))?;
    held.retain(|(s, _)| *s != seg);
    held.push((seg, text));
    Ok(seg)
}

fn takes_drop(inner: Option<&PlInner>, delivery_held: Option<bool>) -> bool {
    match inner {
        Some(PlInner::Doc(true)) => true,
        Some(PlInner::Doc(false)) => delivery_held == Some(false),
        _ => false,
    }
}

#[cfg(test)]
mod drops {
    use super::{takes_drop, PlInner};

    #[test]
    fn the_pay_tab_takes_a_drop_only_on_a_text_page_that_checks_it() {
        assert!(!takes_drop(None, Some(false)), "the pay tab itself: the whole window takes the order file");
        assert!(takes_drop(Some(&PlInner::Doc(true)), None));
        assert!(takes_drop(Some(&PlInner::Doc(false)), Some(false)), "delivery text not held: dropped in to check it");
        assert!(!takes_drop(Some(&PlInner::Doc(false)), Some(true)), "delivery text held: nothing to check");
        assert!(!takes_drop(Some(&PlInner::Doc(false)), None), "no delivery text at all");
        assert!(!takes_drop(Some(&PlInner::Lapse("0x01".into())), Some(false)), "a resignation page");
    }
}

#[cfg(test)]
mod cand_why {
    use super::{Cand, Why};
    use crate::house::i18n::Lang;

    fn hidden(refusal: &str, note: &str) -> Cand {
        Cand {
            n: 0,
            kind: "judge",
            address: String::new(),
            amanah: String::new(),
            ruling_window: 0,
            premium: String::new(),
            lf: String::new(),
            deposit: String::new(),
            priced_at_open: false,
            priced_why: (String::new(), String::new()),
            by_call: false,
            choosable: false,
            refusal: refusal.into(),
            note: note.into(),
            joined: false,
            covenant: None,
            name: String::new(),
        }
    }

    #[test]
    fn a_hidden_candidates_reason_is_said_in_the_pages_language() {
        let made = "判官资料现在供的是 0x01,不是本单的 0x02";
        let mut c = hidden("E_ACT_JUDGE_AMANAH", made);
        for lang in [Lang::En, Lang::Zh] {
            assert_eq!(c.why(lang), crate::house::refusal::what(lang, "E_ACT_JUDGE_AMANAH").expect("in the table"), "{lang:?}");
        }
        assert!(!c.why(Lang::En).chars().any(|x| ('\u{4e00}'..='\u{9fff}').contains(&x)), "{}", c.why(Lang::En));
        assert!(!c.why(Lang::Zh).contains("0x") && !c.why(Lang::Zh).contains("E_"), "{}", c.why(Lang::Zh));
        assert_eq!(c.note_said(), made, "a gate's own note is written alone");
        c.joined = true;
        assert_eq!(c.note_said(), format!("E_ACT_JUDGE_AMANAH · {made}"), "a chain check's note is written after its name");
        let party = hidden("E_P3_JUDGE_PARTY", crate::house::seam::PL_PARTY);
        assert_eq!(party.why(Lang::En), crate::house::i18n::library(Lang::En, crate::house::seam::PL_PARTY));
        let unnamed = hidden("", "以调用同意而未放行");
        assert_eq!(unnamed.why(Lang::En), crate::house::i18n::library(Lang::En, "以调用同意而未放行"));
    }
}

#[cfg(test)]
mod held_texts {
    #[test]
    fn a_dropped_text_is_held_only_for_the_segment_its_hash_names() {
        let dir = std::env::temp_dir().join(format!("eravon-seg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("临时目录");
        let put = |n: &str, b: &[u8]| {
            let p = dir.join(n);
            std::fs::write(&p, b).expect("写");
            p.display().to_string()
        };
        let goods = "一张 A2 海报,私密说明";
        let delivery = "PNG 原图一份";
        let segs = [("goods", format!("0x{}", eravon_goods::terms::segment_hash(goods).trim_start_matches("0x").to_ascii_uppercase())), ("delivery", eravon_goods::terms::segment_hash(delivery))];
        let mut held = Vec::new();
        assert_eq!(super::hold_text(&mut held, &segs, &put("g.md", goods.as_bytes())), Ok("goods"));
        assert_eq!(super::hold_text(&mut held, &segs, &put("d.md", delivery.as_bytes())), Ok("delivery"));
        assert_eq!(super::hold_text(&mut held, &segs, &put("g2.md", goods.as_bytes())), Ok("goods"));
        assert_eq!(held.len(), 2, "one text per segment");
        assert_eq!(super::hold_text(&mut held, &segs, &put("x.md", "别的正文".as_bytes())).map_err(|e| e.name), Err("E_PL_SEGMENT_HASH".to_string()));
        assert_eq!(super::hold_text(&mut held, &segs, &put("bin", &[0xff, 0xfe, 0x00])).map_err(|e| e.name), Err("E_PL_SEGMENT_READ".to_string()));
        assert_eq!(super::hold_text(&mut held, &segs, &dir.join("missing").display().to_string()).map_err(|e| e.name), Err("E_PL_SEGMENT_READ".to_string()));
        assert_eq!(super::hold_text(&mut held, &segs, &put("empty", b"")).map_err(|e| e.name), Err("E_PL_SEGMENT_HASH".to_string()));
        assert_eq!(held.len(), 2, "refusals hold nothing");
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoView {
    pub path: String,
    pub order_file: String,
    pub form: &'static str,
    pub instance: String,
    pub case_id: String,
    pub id: String,
    pub receipt: String,
    pub face: Vec<String>,
    pub time_short: bool,
}

impl PayLink {
    pub fn open(home: &str, data: &str, owner: &str) -> PayLink {
        let mut p = PayLink { owner: owner.to_string(), ..PayLink::default() };
        match crate::house::seam::downloads(home, data) {
            Ok(d) => p.import.at = d,
            Err(s) => p.export_why = Some(s),
        }
        p
    }

    pub fn verified(&self) -> &Option<Result<Opened, Said>> {
        static NONE: Option<Result<Opened, Said>> = None;
        if self.opened_for == self.link.trim() { &self.opened } else { &NONE }
    }

    pub fn open_link(&mut self, text: &str, buyer: &str, data: &str, registries: &[String], now: u64) {
        self.history = None;
        self.link = text.trim().to_string();
        self.wants_open = false;
        self.pick = None;
        self.paid = None;
        self.inner = None;
        self.checked = None;
        self.key_for = None;
        self.form = None;
        self.company = None;
        self.co_open = None;
        self.payout = buyer.to_ascii_lowercase();
        self.payout_said = None;
        self.payout_tried.clear();
        let chain = if buyer.is_empty() {
            Err(eravon_pay::fault::Fault::new("E_P3_NO_KEY", crate::house::i18n::t(crate::house::i18n::Key::PlNoKey)))
        } else {
            eravon_pay::check::Chain::of(data, buyer, registries, now)
        };
        self.opened = Some(verify(&self.link, buyer, chain.as_ref()));
        self.opened_for = self.link.clone();
        let segs = match &self.verified() {
            Some(Ok(o)) => o.segments.clone(),
            _ => Vec::new(),
        };
        self.held.retain(|(s, t)| segs.iter().any(|(x, h)| x == s && same_hash(h, &eravon_goods::terms::segment_hash(t))));
    }

    pub fn attach(&mut self, path: &str) -> Result<&'static str, Said> {
        let Some(Ok(o)) = &self.verified() else { return Err(Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_UNOPENED)) };
        let segments = o.segments.clone();
        hold_text(&mut self.held, &segments, path)
    }

    pub fn choose(&mut self, n: usize) -> Result<(), String> {
        let Some(Ok(o)) = &self.verified() else { return Err(crate::house::seam::PL_KEY_UNOPENED.into()) };
        match o.cands.iter().find(|c| c.n == n) {
            Some(c) if c.choosable => {
                self.pick = Some(n);
                Ok(())
            }
            Some(c) => Err(crate::fill!(crate::house::i18n::Key::PlCandBlocked; c.note.to_string())),
            None => Err(crate::fill!(crate::house::i18n::Key::PlNoCand; n.to_string())),
        }
    }

    pub fn set_payout(&mut self, v: &str) {
        match eravon_vault::payout::accept(v.trim()) {
            Ok(a) => {
                self.payout = a;
                self.payout_said = None;
                self.payout_tried.clear();
            }
            Err(e) => {
                self.payout_said = Some(crate::house::said_of(&e));
                self.payout_tried = v.to_string();
            }
        }
    }

    pub fn pay_key(&self) -> Result<(), &'static str> {
        let Some(Ok(o)) = &self.verified() else { return Err(crate::house::seam::PL_KEY_UNOPENED) };
        if o.steps.iter().any(|(_, s)| matches!(s, State::Fail(..))) {
            return Err(crate::house::seam::PL_KEY_FAILED);
        }
        let Some(n) = self.pick else { return Err(crate::house::seam::PL_KEY_NO_PICK) };
        if !o.cands.iter().any(|c| c.n == n && c.choosable) {
            return Err(crate::house::seam::PL_KEY_NO_PICK);
        }
        if self.payout.is_empty() || self.payout_said.is_some() {
            return Err(crate::house::seam::PL_KEY_NO_PAYOUT);
        }
        if matches!(&self.paid, Some(Ok(p)) if p.stop == "opened") {
            return Err(crate::house::seam::PL_KEY_PAID);
        }
        Ok(())
    }

    pub fn pay(&mut self, s: &eravon_vault::session::Session, buyer: &str, registries: &[String], now: u64, stop: &dyn Fn() -> bool) {
        if crate::house::orders::company_mode(s.data(), buyer).is_some() != self.company_on {
            let changed = Said::bad("E_STORE_STALE", crate::house::i18n::t(crate::house::i18n::Key::ErrESTORESTALE));
            self.paid = None;
            self.company = None;
            self.co_open = None;
            if self.company_on {
                self.company = Some(Err(changed));
            } else {
                self.paid = Some(Err(changed));
            }
            return;
        }
        if self.company_on {
            self.paid = None;
            self.co_open = None;
            self.company_export(s.data(), buyer, registries, now);
            return;
        }
        self.company = None;
        self.co_open = None;
        self.paid = Some((|| {
            self.pay_key().map_err(|w| Said::bad("E_APP_SCRIPT", w))?;
            let Some(Ok(o)) = &self.verified() else { return Err(Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_UNOPENED)) };
            let n = self.pick.ok_or_else(|| Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_NO_PICK))?;
            let p = k3::listing::listing(&o.listing).map_err(|e| Said::bad(e.token.name(), crate::house::i18n::t(crate::house::i18n::Key::PlLinkBroken)))?;
            let c = eravon_pay::check::Chain::of(s.data(), buyer, registries, now).map_err(|f| Said::bad(&f.token, f.detail))?;
            let attachments: Vec<String> = self.held.iter().map(|(_, t)| t.clone()).collect();
            let r = eravon_pay::pay::pay(&eravon_pay::pay::PayIn {
                c: &c,
                s,
                link: &o.link,
                listing: &o.listing,
                p: &p,
                candidate: n,
                payout: &self.payout,
                writer: crate::house::seam::WRITER,
                attachments: &attachments,
                stop,
            })
            .map_err(|f| Said::bad(&f.token, f.detail))?;
            Ok(paid_view(r))
        })());
    }

    pub fn form_of(&self) -> Option<eravon_pay::company::Form> {
        if self.form.is_some() {
            return self.form;
        }
        let Some(Ok(o)) = &self.verified() else { return None };
        let n = self.pick?;
        let q = o.quotes.get(n).cloned().flatten()?;
        let p = eravon_link::listing::listing(&o.listing).ok()?;
        Some(eravon_pay::company::default_form(&q, &p, n))
    }

    pub fn pay_plan(&self) -> Result<(), Said> {
        self.pay_key().map_err(|w| Said::bad("E_APP_SCRIPT", w))?;
        if self.company_on {
            self.form_of().ok_or_else(|| Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_NO_PICK))?;
            if self.export_dir.is_empty() {
                return Err(self.export_why.clone().unwrap_or_else(|| Said::bad("E_P3_EXPORT_DIR", crate::house::i18n::t(crate::house::i18n::Key::OrdersExportDirPick))));
            }
        }
        Ok(())
    }

    pub fn rows(&self) -> Out {
        let mut m: Vec<(&'static str, Out)> = vec![("link", Out::S(self.link.clone())), ("tab", Out::S(self.tab.name().into()))];
        match &self.verified() {
            Some(Ok(o)) => m.push(("opened", opened_rows(o))),
            Some(Err(s)) => m.push(("opened", s.out())),
            None => {}
        }
        if let Some(n) = self.pick {
            m.push(("pick", Out::N(n as u64)));
        }
        m.push(("payout", Out::S(self.payout.clone())));
        if let Some(s) = &self.payout_said {
            m.push(("payoutSaid", s.out()));
        }
        match &self.key_for {
            Some(Ok(p)) => m.push((
                "keyFor",
                Out::O(vec![
                    ("address", Out::S(p.address.clone())),
                    ("bumped", Out::B(p.bumped)),
                    ("instance", Out::S(p.instance.clone())),
                    ("openBy", Out::N(p.open_by)),
                    ("stuck", Out::B(p.stuck)),
                ]),
            )),
            Some(Err(s)) => m.push(("keyFor", s.out())),
            None => {}
        }
        let key = self.pay_key();
        m.push(("payKey", Out::O(vec![("enabled", Out::B(key.is_ok())), ("why", Out::S(key.err().unwrap_or_default().into()))])));
        if let Some(h) = crate::house::history::out(&self.history) {
            m.push(("history", h));
        }
        match &self.paid {
            Some(Ok(p)) => m.push(("paid", paid_rows(p))),
            Some(Err(s)) => m.push(("paid", s.out())),
            None => {}
        }
        if let Some(f) = self.form_of() {
            m.push(("form", Out::S(f.name().into())));
        }
        match &self.company {
            Some(Ok(v)) => m.push((
                "company",
                Out::O(vec![
                    ("caseId", Out::S(v.case_id.clone())),
                    ("face", Out::A(v.face.iter().map(|x| Out::S(x.clone())).collect())),
                    ("form", Out::S(v.form.into())),
                    ("id", Out::S(v.id.clone())),
                    ("instance", Out::S(v.instance.clone())),
                    ("orderFile", Out::S(v.order_file.clone())),
                    ("path", Out::S(v.path.clone())),
                    ("receipt", Out::S(v.receipt.clone())),
                ]),
            )),
            Some(Err(s)) => m.push(("company", s.out())),
            None => {}
        }
        if let Some(x) = crate::house::ask::out(&self.ask) {
            m.push(("timeAsk", x));
        }
        match &self.co_open {
            Some(Ok(tx)) => m.push(("companyOpen", Out::S(tx.clone()))),
            Some(Err(s)) => m.push(("companyOpen", s.out())),
            None => {}
        }
        if self.verified().as_ref().is_some_and(|o| o.is_ok()) {
            let seg = |goods: bool| match self.segment(goods) {
                None => Out::S(String::new()),
                Some(s) => {
                    let mut v = vec![("hash", Out::S(s.hash.clone())), ("held", Out::B(s.text.is_some())), ("onBoard", Out::B(s.on_board))];
                    if let (true, Some(t)) = (goods, &s.text) {
                        v.push(("goodsRefs", crate::house::case::refs_out(&eravon_goods::refs::read(t))));
                    }
                    Out::O(v)
                }
            };
            m.push(("described", Out::O(vec![("delivery", seg(false)), ("goods", seg(true))])));
            let lapses = self
                .lapses()
                .into_iter()
                .map(|(a, guild, ck)| {
                    let mut v = vec![("a", Out::S(a)), ("guild", Out::B(guild))];
                    match ck {
                        Ok(c) => v.push(("lapse", crate::house::case::lapse_out(&c, guild))),
                        Err(e) => v.push(("refused", Out::S(e))),
                    }
                    Out::O(v)
                })
                .collect();
            m.push(("lapses", Out::A(lapses)));
        }
        if let Some(x) = &self.inner {
            m.push(("inner", Out::S(x.word().into())));
            if let PlInner::Lapse(a) | PlInner::History(a) = x {
                m.push(("innerWho", Out::S(a.clone())));
            }
        }
        match &self.checked {
            Some(Ok(n)) => m.push(("checked", Out::S(n.clone()))),
            Some(Err(s)) => m.push(("checked", s.out())),
            None => {}
        }
        let i = &self.import;
        let mut im: Vec<(&'static str, Out)> = vec![
            ("file", Out::S(i.file.as_ref().map(|x| x.0.clone()).unwrap_or_default())),
            ("pickFrom", Out::S(i.at.clone())),
        ];
        if let Some(d) = &i.done {
            im.push(("done", done_rows(d)));
        }
        if let Some(s) = &i.said {
            im.push(("said", s.out()));
        }
        im.sort_by(|a, b| a.0.cmp(b.0));
        m.push(("import", Out::O(im)));
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}

pub fn checks_pass(steps: &[(&'static str, State)]) -> bool {
    steps.iter().all(|(_, s)| matches!(s, State::Pass))
}

#[cfg(test)]
mod checks_tests {
    use super::{checks_pass, State};

    #[test]
    fn the_checks_read_as_all_passed_or_not_passed() {
        assert!(checks_pass(&[("a", State::Pass), ("b", State::Pass)]));
        assert!(!checks_pass(&[("a", State::Pass), ("b", State::Fail("E_X".into(), "x".into()))]));
        assert!(checks_pass(&[]));
    }
}

#[cfg(test)]
mod refused_refund {
    #[test]
    fn what_was_refused_is_kept_as_typed_until_an_address_is_taken() {
        let mut pl = super::PayLink::default();
        let good = "0x00000000000000000000000000000000000000aa";
        pl.set_payout(good);
        assert!(pl.payout_said.is_none() && pl.payout_tried.is_empty());
        pl.set_payout(" 0x12 ");
        assert!(pl.payout_said.is_some());
        assert_eq!(pl.payout_tried, " 0x12 ", "as typed, so the cell that holds it is known");
        assert_eq!(pl.payout, good, "the accepted address stays");
        pl.set_payout("");
        assert!(pl.payout_said.is_some());
        assert_eq!(pl.payout_tried, "", "an empty try is remembered as empty");
        pl.set_payout(good);
        assert!(pl.payout_said.is_none() && pl.payout_tried.is_empty(), "taking an address clears the refusal");
    }
}

crate::house::merge::lands!(PayLink {
    whole: [owner],
    data: [ticket, opened, opened_for, key_for, paid, form, company, company_on, co_open, ask, export_why, held, checked, payout_said, import],
    page: [tab, link, wants_open, pick, payout, payout_tried, export_dir, inner, history],
    keyed(|_: &PayLink| ()): [],
    nested: [],
    said: [],
});

#[cfg(test)]
mod verified_tests {
    #[test]
    fn a_check_stands_only_for_the_link_it_checked() {
        let mut p = super::PayLink { link: "eravon://a".into(), opened: Some(Err(super::Said::bad("E_X", "x"))), opened_for: "eravon://a".into(), ..Default::default() };
        assert!(p.verified().is_some());
        p.link = "eravon://b".into();
        assert!(p.verified().is_none(), "another link in the cell: no check result");
        assert_eq!(p.pay_key(), Err(crate::house::seam::PL_KEY_UNOPENED), "and nothing to pay");
        p.link = " eravon://a ".into();
        assert!(p.verified().is_some(), "the same link typed with spaces");
    }
}
