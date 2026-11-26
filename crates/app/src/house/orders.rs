use eravon_deliver::{deliver, todo};
use eravon_dispute::{entries::EntryView, evidence, refund, seat, settle, sweep, tick};
use eravon_literals::json::Out;
use eravon_orders::table::Row;
use eravon_watch::order::{self, Order, State};
use eravon_watch::send::Post;

use crate::house::seam;
use crate::house::seller::Said;


mod list;
mod bond_deliver;
use bond_deliver::*;
mod export;
pub(crate) use export::{export_plan, exported_say, open_company, set_export_dir};
mod dispute;
use dispute::*;
mod judge;
pub(crate) use judge::wipe;
pub(crate) mod answer;
pub use answer::{acts, shown_acts};
pub struct Key {
    key: Option<[u8; 32]>,
    ticket: crate::house::Ticket,
}

impl Key {
    pub fn of(session: &eravon_vault::session::Session, addr: &str) -> Key {
        Key { key: session.key_for(addr).ok(), ticket: crate::house::Ticket::standing() }
    }
    pub fn locked() -> Key {
        Key { key: None, ticket: crate::house::Ticket::standing() }
    }
    pub fn bound(mut self, t: &crate::house::Ticket) -> Key {
        self.ticket = t.clone();
        self
    }
    fn get(&self) -> Option<[u8; 32]> {
        self.key.filter(|_| self.ticket.live())
    }
    pub fn is_open(&self) -> bool {
        self.get().is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    Deliver,
    Refund,
    Evidence,
    Settle(String, String),
    Sig,
    Open,
}

impl Drop for Key {
    fn drop(&mut self) {
        if let Some(k) = self.key.as_mut() {
            for b in k.iter_mut() {
                *b = 0;
            }
            let _ = std::hint::black_box(&k);
        }
    }
}

pub(crate) struct GoodsSend<'a> {
    pub key: &'a Key,
    pub addr: &'a str,
    pub w: &'a eravon_cli::shared::Wire,
    pub live: &'a dyn Fn() -> bool,
}

impl eravon_goods::goods::Sender for GoodsSend<'_> {
    fn send(&mut self, call: &eravon_standard::call::Call) -> Result<String, (String, String)> {
        if !(self.live)() {
            return Err(eravon_goods::sign::stale());
        }
        let Some(k) = self.key.get() else {
            return Err(("E_VAULT_LOCKED".into(), crate::house::i18n::t(crate::house::i18n::Key::OrdersLockedFirst)));
        };
        eravon_cli::send::deliver_key(&|| Ok(k), &eravon_cli::send::Opts::default(), self.w, self.addr, call, "0").map_err(|e| (e.name().to_string(), e.detail()))?.outcome()
    }
}

struct ShellPost<'a> {
    key: [u8; 32],
    addr: &'a str,
    w: &'a eravon_cli::shared::Wire,
    sent: Option<(String, eravon_watch::order::Broadcast)>,
}

impl Post for ShellPost<'_> {
    fn from(&self) -> Option<String> {
        Some(self.addr.to_string())
    }
    fn post(&mut self, call: &eravon_standard::call::Call) -> Result<String, eravon_watch::Why> {
        self.post_landed(call, &mut |_, _| Ok(()))
    }
    fn resend(&mut self, call: &eravon_standard::call::Call, at: &eravon_watch::order::Broadcast) -> Result<String, eravon_watch::Why> {
        self.resend_landed(call, at, &mut |_, _| Ok(()))
    }
    fn post_landed(&mut self, call: &eravon_standard::call::Call, land: eravon_watch::send::Land<'_>) -> Result<String, eravon_watch::Why> {
        self.send(call, &eravon_cli::send::Opts::default(), land)
    }
    fn resend_landed(&mut self, call: &eravon_standard::call::Call, at: &eravon_watch::order::Broadcast, land: eravon_watch::send::Land<'_>) -> Result<String, eravon_watch::Why> {
        let o = eravon_cli::send::replacement(self.w, &eravon_cli::send::pending_of(at)).map_err(|e| (e.name().to_string(), e.detail()))?;
        self.send(call, &o, land)
    }
    fn broadcast(&self) -> Option<(String, eravon_watch::order::Broadcast)> {
        self.sent.clone()
    }
}

pub(crate) fn draw_round(data: &str, addr: &str, now: u64, key: &Key) -> Option<Said> {
    let k = key.get()?;
    let w = wire(data, addr).ok()?;
    let mut p = ShellPost { key: k, addr, w: &w, sent: None };
    let got = eravon_watch::send::draw_round(data, crate::house::seam::WRITER, now, addr, &sw(&w), &mut p);
    match got {
        Err(e) => Some(said(e)),
        Ok(v) => v.into_iter().last().map(|(_, r)| match r {
            Ok(tx) => Said::ok(format!("{} · {tx}", bond_deliver::sent_say("redraw"))),
            Err(e) => said(e),
        }),
    }
}

impl ShellPost<'_> {
    fn send(&mut self, call: &eravon_standard::call::Call, o: &eravon_cli::send::Opts, land: eravon_watch::send::Land<'_>) -> Result<String, eravon_watch::Why> {
        let k = self.key;
        self.sent = None;
        let mut landed = None;
        let got = eravon_cli::send::deliver_key_landed(&|| Ok(k), o, self.w, self.addr, call, "0", &mut |_, tx, b| {
            land(tx, b).map_err(|(token, why)| eravon_cli::fault::Fault::Down { layer: "S4", token, why })?;
            landed = Some((tx.to_string(), b.clone()));
            Ok(())
        });
        self.sent = landed;
        let d = got.map_err(|e| (e.name().to_string(), e.detail()))?;
        d.outcome()
    }
}

pub const ZK_OFF: &str = eravon_cli::deskops::network::ZK_OFF;

pub fn order_label(name: &str) -> String {
    let address = name.len() == 42 && name.starts_with("0x") && name[2..].bytes().all(|c| c.is_ascii_hexdigit());
    if address { eravon_orders::table::short(name) } else { name.to_string() }
}

pub fn row_label(r: &Row) -> String {
    order_label(&eravon_orders::table::name(r))
}

pub fn zk_linked(ident: &eravon_store::settings::Identity) -> bool {
    eravon_cli::deskops::network::zk_linked(ident)
}

fn said((n, d): eravon_watch::Why) -> Said {
    Said::bad(&n, d)
}

static INJECTED: std::sync::RwLock<Vec<String>> = std::sync::RwLock::new(Vec::new());

pub(crate) fn inject(registries: &[String]) {
    if let Ok(mut g) = INJECTED.write() {
        *g = registries.to_vec();
    }
}

pub(crate) fn injected() -> Vec<String> {
    INJECTED.read().map(|g| g.clone()).unwrap_or_default()
}

pub(crate) fn wire(data: &str, addr: &str) -> Result<eravon_cli::shared::Wire, Said> {
    let (t, table) = eravon_watch::wire_parts(data, addr).map_err(said)?;
    let (chain_id, logic) = eravon_watch::standard();
    Ok(eravon_cli::shared::Wire { t, table, source: eravon_cli::home::Source::Identity, chain_id, logic: logic.to_string(), registries: injected() })
}

pub(crate) fn sw(w: &eravon_cli::shared::Wire) -> eravon_watch::Wire<'_> {
    eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries }
}

pub(crate) use eravon_cli::deskops::buyerpage::company_mode;

pub(crate) fn order_company(data: &str, addr: &str, o: &Order) -> Result<Option<String>, Said> {
    eravon_cli::deskops::orderpage::order_company(data, addr, o, &|| wire(data, addr).ok(), route_of)
}

fn listed_seller(data: &str, addr: &str, o: &Order) -> Option<String> {
    eravon_cli::deskops::orderpage::listed_seller(data, addr, o)
}

fn route_of(addr: &str, seller: &str, company: Option<&str>, acting: &mut dyn FnMut() -> Option<String>) -> Result<Option<String>, Said> {
    eravon_cli::deskops::orderpage::route_of(addr, seller, company, acting)
}



pub(crate) fn opens(data: &str, addr: &str) -> eravon_watch::send::Opens {
    eravon_payways::ways::read(data, addr).map_or(eravon_watch::send::Opens::Off, |w| eravon_watch::send::Opens::auto(w.auto_open))
}

fn route_local(data: &str, addr: &str, o: &Order) -> Option<Result<Option<String>, Said>> {
    let seller = listed_seller(data, addr, o)?;
    let on = company_mode(data, addr).map(|(c, _)| c);
    let local = seller.eq_ignore_ascii_case(addr) || on.as_deref().is_some_and(|c| seller.eq_ignore_ascii_case(c));
    local.then(|| route_of(addr, &seller, on.as_deref(), &mut || None))
}

impl Orders {
    pub fn complete_table(&self) -> Option<Result<Vec<Row>, Said>> {
        match (&self.table, &self.unscanned) {
            (Some(Ok(_)), Some(u)) => Some(Err(u.clone())),
            (t, _) => t.clone(),
        }
    }

    pub fn no_order(&self) -> Said {
        Said::bad("E_ORDERS_NONE", crate::house::i18n::t(if self.sel.is_some() { crate::house::i18n::Key::OrdersNoRecord } else { crate::house::i18n::Key::BoOpenFirst }))
    }

    pub fn plan(&self, data: &str, addr: &str, key: &Key, what: &Plan) -> Result<(), Said> {
        let o = self.order.as_ref().ok_or_else(|| self.no_order())?;
        if let Some(r) = self.row() {
            eravon_standard::chain::past_gate(r.past).map_err(|f| Said::bad(&f.name(), f.detail()))?;
        }
        match what {
            Plan::Refund if self.card.is_none() => return Err(Said::bad("E_ORDERS_CARD", crate::house::i18n::t(crate::house::i18n::Key::OrdersCardFirst))),
            Plan::Deliver => match &self.deliver {
                None => return Err(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersDeliverFirst))),
                Some(m) if !m.key_on => return Err(m.refusal()),
                _ => {}
            },
            Plan::Settle(bps, days) => {
                bps.trim().parse::<u64>().map_err(|_| Said::bad("E_DISPUTE_BPS", crate::fill!(crate::house::i18n::Key::OrdersBpsBad; bps.to_string())))?;
                settle::expiry_in(0, days).map_err(said)?;
            }
            _ => {}
        }
        match route_local(data, addr, o) {
            Some(Err(s)) => Err(s),
            Some(Ok(Some(_))) => {
                if self.export_dir.is_empty() {
                    return Err(self.export_why.clone().unwrap_or_else(|| Said::bad("E_ORDERS_EXPORT_DIR", crate::house::i18n::t(crate::house::i18n::Key::OrdersExportDirPick))));
                }
                company_mode(data, addr).map(|_| ()).ok_or_else(|| Said::bad("E_ORDERS_COMPANY_OFF", crate::house::i18n::t(crate::house::i18n::Key::OrdersCompanyOff)))
            }
            _ if key.get().is_none() && route_local(data, addr, o).is_some() => Err(locked()),
            _ => Ok(()),
        }
    }

    fn planned(&mut self, data: &str, addr: &str, key: &Key, what: Plan) -> bool {
        match crate::house::asked(self.plan(data, addr, key, &what)) {
            Ok(()) => true,
            Err(s) => {
                self.said = Some(s);
                false
            }
        }
    }
}

pub fn line(o: &Order) -> (String, Option<String>) {
    let gap = (!o.gap.is_empty()).then(|| crate::fill!(crate::house::i18n::Key::OrdersGapLine; money(crate::house::i18n::current(), Some(o), &o.gap).to_string(), crate::house::i18n::library(crate::house::i18n::current(), order::GAP_NOTE)));
    (o.state.say().to_string(), gap)
}


pub fn money(lang: crate::house::i18n::Lang, o: Option<&Order>, amount: &str) -> String {
    money_as(lang, o, amount, crate::house::coin::Form::Full)
}

pub fn money_as(lang: crate::house::i18n::Lang, o: Option<&Order>, amount: &str, form: crate::house::coin::Form) -> String {
    let decimals = o.and_then(eravon_watch::units::of);
    let token = o.map(|o| o.token.as_str()).unwrap_or_default();
    crate::house::coin::amount(lang, amount, decimals, token, form)
}

pub fn money_pair(lang: crate::house::i18n::Lang, o: Option<&Order>, have: &str, need: &str) -> String {
    crate::house::coin::pair(lang, have, need, o.and_then(eravon_watch::units::of), o.map(|o| o.token.as_str()).unwrap_or_default(), crate::house::coin::Form::Full)
}

#[derive(Clone, Debug, Default)]
pub struct DeliverModal {
    pub path: String,
    pub items: Option<Result<deliver::Items, Said>>,
    pub reading: bool,
    pub note: String,
    pub method: String,
    pub sheet: Option<Result<(String, Vec<(String, u64)>), Said>>,
    pub key_on: bool,
    pub key_note: String,
    pub key_off: Option<Said>,
}

impl DeliverModal {
    pub fn refusal(&self) -> Said {
        self.key_off.clone().unwrap_or_else(|| Said::bad("E_DELIVER_PHASE", self.key_note.clone()))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Orders {
    pub company: bool,
    pub paste: String,
    pub pending_ask: Option<crate::house::action::Action>,
    pub order: Option<Order>,
    pub deliver: Option<DeliverModal>,
    pub todo: Option<todo::Todo>,
    pub scan: Option<Result<String, Said>>,
    pub said: Option<Said>,
    pub copied: Option<String>,
    pub card: Option<CardView>,
    pub evidence: Option<EvidenceModal>,
    pub settle: Option<SettleModal>,
    pub offer: Option<(u64, u64)>,
    pub sig_settle: String,
    pub sig_confirm: String,
    pub face: Option<Result<FaceView, Said>>,
    pub entries: Option<Result<Vec<EntryView>, Said>>,
    pub opened: Option<String>,
    pub me: String,
    pub table: Option<Result<Vec<Row>, Said>>,
    pub unscanned: Option<Said>,
    pub follow: crate::house::refresh::Follow,
    pub foreign: u64,
    pub done_tab: bool,
    pub search: Search,
    pub sel: Option<String>,
    pub tab: Tab,
    pub detail: Option<DetailView>,
    pub judge: Option<Result<crate::house::judge_card::Card, Said>>,
    pub judge_open: bool,
    pub history: crate::house::history::Hist,
    pub inner: Option<Inner>,
    pub papers: Option<crate::house::case::Papers>,
    pub bench: Option<Result<crate::house::case::Bench, Said>>,
    pub complaint: Option<crate::house::complaint::Complaint>,
    pub sealable: Option<crate::house::judge_card::Sealable>,
    pub seal_next: Option<crate::house::judge_card::Sealable>,
    pub sealed: Option<Result<eravon_dispute::seal::Sealed, Said>>,
    pub paste_open: bool,
    pub terms_open: bool,
    pub saved: Option<Vec<String>>,
    pub export_dir: String,
    pub exported: Option<Result<String, Said>>,
    pub originals: Vec<String>,
    pub co: Option<String>,
    pub co_err: Option<Said>,
    pub ask: Option<crate::house::ask::TimeAsk>,
    pub export_why: Option<Said>,
    pub ticket: crate::house::Ticket,
}

#[derive(Clone, Debug)]
pub struct CardView {
    pub amount: String,
    pub accepted: bool,
    pub judge_fee: String,
    pub to_buyer: String,
    pub bond_back: String,
    pub lf_back: String,
}

#[derive(Clone, Debug)]
pub struct EvidenceModal {
    pub home: String,
    pub manual: String,
    pub kits: Result<Vec<(String, String, u64)>, Said>,
    pub kit: Option<String>,
    pub path: String,
    pub items: Option<Result<evidence::Items, Said>>,
    pub reading: bool,
    pub note: String,
    pub sheet: Option<Result<(String, Vec<(String, u64)>), Said>>,
}

#[derive(Clone, Debug, Default)]
pub struct SettleModal {
    pub bps: String,
    pub days: String,
    pub offer: Option<Result<OfferView, Said>>,
}

#[derive(Clone, Debug)]
pub struct OfferView {
    pub digest: String,
    pub preview: Result<String, Said>,
}

#[derive(Clone, Debug)]
pub struct FaceView {
    pub word: &'static str,
    pub say: String,
    pub phase: u64,
    pub refundable: bool,
    pub disputed: bool,
    pub redraw: Option<Said>,
    pub sweep: Option<Said>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inner {
    Judge(u8),
    Docs(u8),
    Lapse,
    History,
}

impl Inner {
    pub fn word(self) -> &'static str {
        match self {
            Inner::Judge(_) => "caseJudge",
            Inner::Docs(_) => "caseDocs",
            Inner::Lapse => "lapse",
            Inner::History => "history",
        }
    }

    pub fn over(self) -> Option<Inner> {
        match self {
            Inner::Lapse => Some(Inner::Judge(2)),
            Inner::Judge(_) | Inner::Docs(_) | Inner::History => None,
        }
    }

    pub fn depth_of(word: &str) -> usize {
        match word {
            w if w == Inner::Lapse.word() => 2,
            _ => 1,
        }
    }

    pub fn tab(self) -> u8 {
        match self {
            Inner::Judge(t) | Inner::Docs(t) => t,
            Inner::Lapse | Inner::History => 0,
        }
    }

    pub fn of(page: &str, tab: &str) -> Result<Inner, String> {
        let t: u8 = if tab.is_empty() { 0 } else { tab.parse().map_err(|_| format!("tab 要一个页签序号,给的是 {tab}"))? };
        match (page, t) {
            ("judge", 0..=2) => Ok(Inner::Judge(t)),
            ("docs", 0..=3) => Ok(Inner::Docs(t)),
            _ => Err(format!("{page} 没有第 {t} 页签")),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
    Delivery,
    Dispute,
    Record,
    Fee,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Overview, Tab::Delivery, Tab::Dispute, Tab::Record, Tab::Fee];
    pub fn word(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Delivery => "delivery",
            Tab::Dispute => "dispute",
            Tab::Record => "record",
            Tab::Fee => "fee",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Tab::Overview => "orders.tabOverview",
            Tab::Delivery => "orders.tabDelivery",
            Tab::Dispute => "orders.tabDispute",
            Tab::Record => "orders.tabRecord",
            Tab::Fee => "orders.tabFee",
        }
    }
    pub fn of(w: &str) -> Option<Tab> {
        Tab::ALL.into_iter().find(|t| t.word() == w)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Search {
    pub on: bool,
    pub q: String,
    pub from: String,
    pub to: String,
}

pub use eravon_cli::deskops::orderpage::Detail as DetailView;

pub(crate) fn advance_of(w: &eravon_watch::Wire<'_>, r: &Row) -> Option<Result<eravon_watch::pool::Due, Said>> {
    let v = r.chain.as_ref().filter(|v| v.phase != 0)?;
    Some(eravon_standard::order::b32_of("案号", &r.case_id).map_err(|f| Said::bad(&f.name(), f.detail())).and_then(|id| eravon_watch::pool::advance_due(w, &r.instance, &id, v.seats.saturating_sub(1)).map_err(said)))
}

pub(crate) fn chosen_of(data: &str, addr: &str, r: &Row, o: Option<&Order>) -> Option<String> {
    if r.chain.as_ref().is_some_and(|v| v.phase != 0) {
        return None;
    }
    let o = o?;
    let listing = order::part(data, addr, &o.id, &order::part_listing()).ok().and_then(|b| eravon_link::listing::listing(&b).ok())?;
    eravon_orders::detail::chosen(&listing, o.candidate)
}




#[cfg(test)]
mod seller_routes {
    #[test]
    fn an_order_takes_the_company_path_when_the_company_acts_for_its_seller_key() {
        let (me, co, old, other) = (
            "0x00000000000000000000000000000000000000d1",
            "0x00000000000000000000000000000000000000c0",
            "0x00000000000000000000000000000000000000a1",
            "0x00000000000000000000000000000000000000e1",
        );
        let asked = std::cell::Cell::new(false);
        assert_eq!(super::route_of(me, me, Some(co), &mut || { asked.set(true); None }).map_err(|s| s.name), Ok(None), "本席钥走持钥路");
        assert!(!asked.get(), "本席钥不问簿");
        assert_eq!(super::route_of(me, old, Some(co), &mut || Some(co.to_string())).map_err(|s| s.name), Ok(Some(co.to_string())), "记录换到本公司走公司路");
        assert_eq!(super::route_of(me, co, Some(co), &mut || None).map_err(|s| s.name), Ok(Some(co.to_string())), "簿读不成照原钥");
        assert_eq!(super::route_of(me, co, Some(co), &mut || Some(other.to_string())).map_err(|s| s.name), Err("E_ORDERS_SELLER_KEY".to_string()), "公司记录已交给别家");
        assert_eq!(super::route_of(me, old, Some(co), &mut || Some(other.to_string())).map_err(|s| s.name), Err("E_ORDERS_SELLER_KEY".to_string()));
        assert_eq!(super::route_of(me, old, None, &mut || Some(co.to_string())).map_err(|s| s.name), Err("E_ORDERS_SELLER_KEY".to_string()), "公司模式关着");
    }

    #[test]
    fn an_order_of_a_former_key_takes_the_key_path_once_this_key_acts_for_it() {
        let (me, co, old) = ("0x00000000000000000000000000000000000000d1", "0x00000000000000000000000000000000000000c0", "0x00000000000000000000000000000000000000a1");
        assert_eq!(super::route_of(me, old, None, &mut || Some(me.to_string())).map_err(|s| s.name), Ok(None), "换钥后旧钥的单走持钥路");
        assert_eq!(super::route_of(me, old, Some(co), &mut || Some(me.to_ascii_uppercase().replace("0X", "0x"))).map_err(|s| s.name), Ok(None), "公司模式开着、大小写不同照认");
        assert_eq!(super::route_of(me, old, None, &mut || None).map_err(|s| s.name), Err("E_ORDERS_SELLER_KEY".to_string()), "簿读不成不猜");
    }
}

#[cfg(test)]
mod label_tests {
    #[test]
    fn an_order_is_named_by_its_number_else_its_short_instance() {
        let inst = "0x8399f4e97c18326d53a499daca31bdc3dc7826bc";
        assert_eq!(super::order_label("202611-003"), "202611-003");
        let l = super::order_label(inst);
        assert!(l.len() < inst.len() && l.starts_with("0x8399") && l.ends_with("26bc"), "{l}");
        assert_eq!(super::order_label("商品甲"), "商品甲");
    }
}

crate::house::merge::lands!(Orders {
    whole: [me],
    data: [company, scan, copied, opened, table, unscanned, follow, foreign, export_dir, ticket],
    page: [paste, pending_ask, deliver, card, evidence, settle, sig_settle, sig_confirm, done_tab, search, sel, tab, judge_open, inner, paste_open, terms_open],
    keyed(|m: &Orders| m.sel.clone()): [order, todo, offer, face, entries, detail, judge, history, papers, bench, complaint, sealable, seal_next, sealed, saved, exported, originals, co, co_err, ask, export_why],
    nested: [],
    said: [said],
});
