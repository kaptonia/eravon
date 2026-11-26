use eravon_after::act::{self, Access};
use eravon_literals::json::Out;
use eravon_orders::table::Row;
use eravon_store::seam::Box_;

use crate::house::seller::Said;


mod acts;
pub use acts::{offer_text};
mod judge;
mod answer;
pub use answer::{acts, shown_acts};
use answer::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Overview,
    Delivery,
    Dispute,
    Record,
    Fee,
}

impl Tab {
    pub const fn word(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Delivery => "delivery",
            Tab::Dispute => "dispute",
            Tab::Record => "record",
            Tab::Fee => "fee",
        }
    }

    pub fn of(s: &str) -> Option<Tab> {
        [Tab::Overview, Tab::Delivery, Tab::Dispute, Tab::Record, Tab::Fee].into_iter().find(|t| t.word() == s)
    }
}

pub use eravon_cli::deskops::buyerpage::{receipt_of, BRow, Budget};
pub(crate) use eravon_cli::deskops::buyerpage::deposit_short;
use eravon_cli::deskops::buyerpage::budget_of;

#[derive(Clone, Debug, Default)]
pub struct Open {
    pub instance: String,
    pub tab: Tab,
    pub detail: Option<Out>,
    pub card: Option<Result<act::ConfirmCard, Said>>,
    pub refund: Option<Result<act::Refund, Said>>,
    pub owed: Option<Result<act::Owed, Said>>,
    pub offer: Option<Result<act::Offer, Said>>,
    pub kits: Option<Result<Vec<(String, String, u64)>, Said>>,
    pub first_kits: Option<Result<Vec<(String, String, u64)>, Said>>,
    pub handed: Option<eravon_after::act::Handed>,
    pub judge: Option<Result<crate::house::judge_card::Card, Said>>,
    pub judge_open: bool,
    pub history: crate::house::history::Hist,
    pub sealable: Option<crate::house::judge_card::Sealable>,
    pub sealed: Option<Result<eravon_dispute::seal::Sealed, Said>>,
    pub inner: Option<crate::house::orders::Inner>,
    pub bench: Option<Result<crate::house::case::Bench, Said>>,
    pub papers: Option<crate::house::case::Papers>,
    pub advance: Option<Result<act::AdvanceCard, Said>>,
    pub complaint: Option<crate::house::complaint::Complaint>,
}

#[derive(Clone, Debug, Default)]
pub struct BOrders {
    pub co_other: Option<String>,
    pub me: String,
    pub ticket: crate::house::Ticket,
    pub rows: Option<Result<Vec<BRow>, Said>>,
    pub unscanned: Option<Said>,
    pub follow: crate::house::refresh::Follow,
    pub done_tab: bool,
    pub open: Option<Open>,
    pub said: Option<Said>,
    pub said_on: Option<String>,
    pub drew: Option<String>,
    pub q: String,
    pub from: String,
    pub to: String,
    pub owners: Vec<String>,
    pub export_dir: String,
    pub exported: Option<Result<String, Said>>,
    pub ask: Option<crate::house::ask::TimeAsk>,
    pub seal_next: Option<crate::house::judge_card::Sealable>,
    pub export_why: Option<Said>,
}

pub(crate) use eravon_cli::deskops::buyerpage::key_set;

fn company_row(company: Option<&str>, buyer: &str, acting: impl FnOnce() -> Option<String>) -> bool {
    eravon_cli::deskops::buyerpage::company_row(company, buyer, acting)
}


fn column_holder(co: &eravon_cli::company::Company) -> &str {
    eravon_cli::deskops::buyerpage::column_holder(co)
}


pub(crate) fn product_and_seller(data: &str, owners: &[String], instance: &str) -> (String, String) {
    eravon_cli::deskops::buyerpage::product_and_seller(data, owners, instance, &crate::house::i18n::t(crate::house::i18n::Key::PlGoodsPrivate))
}

fn company_of(w: &eravon_cli::shared::Wire, data: &str, me: &str) -> Option<Result<eravon_cli::company::Company, Said>> {
    eravon_cli::deskops::buyerpage::company_of(w, data, me, &company_shut)
}

fn company_shut(at: String, kind: String, why: String) -> String {
    crate::fill!(crate::house::i18n::Key::OrdersCompanyShut; at, kind, why)
}

fn page_words(f: impl FnOnce(&eravon_cli::deskops::buyerpage::Page) -> Result<eravon_cli::deskops::buyerpage::Table, Said>) -> Result<eravon_cli::deskops::buyerpage::Table, Said> {
    let tokens = crate::house::coin::rows();
    let private = crate::house::i18n::t(crate::house::i18n::Key::PlGoodsPrivate);
    f(&eravon_cli::deskops::buyerpage::Page {
        tokens: &tokens,
        private_goods: &private,
        company_shut: &company_shut,
        company_row: |c, b, a| company_row(c, b, a),
        column_holder,
    })
}

impl BOrders {
    pub fn load(s: &eravon_vault::session::Session, data: &str, me: &str) -> BOrders {
        crate::house::mark::mark(crate::house::mark::FEATURE_P5);
        let mut b = BOrders::default();
        b.reload(s, data, me);
        b
    }

    pub fn reload(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str) {
        self.me = me.to_ascii_lowercase();
        self.unscanned = None;
        let mut owners = Vec::new();
        let got = (|| {
            eravon_vault::session::need_open(s).map_err(|e| crate::house::said_of(&e))?;
            let w = crate::house::orders::wire(data, me)?;
            page_words(|p| eravon_cli::deskops::buyerpage::rows_of(s, data, me, &w, p, &mut owners))
        })();
        if !owners.is_empty() {
            self.owners = owners;
        }
        self.rows = Some(got.map(|t| {
            self.unscanned = t.unscanned;
            t.rows
        }));
        if let Some(o) = self.open.clone() {
            self.open_row(data, me, &o.instance, o.tab);
        }
    }

    pub(crate) fn draw_due(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str) {
        let Some(Ok(rows)) = self.rows.as_ref() else { return };
        let due: Vec<BRow> = rows.iter().filter(|r| r.row.status.as_ref().is_ok_and(|x| x.word == eravon_orders::status::Word::AwaitRedraw)).cloned().collect();
        if due.is_empty() {
            return;
        }
        let Ok(w) = crate::house::orders::wire(data, me) else { return };
        let t = self.ticket.clone();
        let still = move || t.live();
        let b = eravon_after::act::By { s, me, writer: crate::house::seam::WRITER, still: &still };
        for r in &due {
            match eravon_after::act::redraw(&w, &b, &r.case()) {
                Ok(Some(tx)) => {
                    self.said = Some(Said::ok(format!("{} · {tx}", crate::house::i18n::t(crate::house::i18n::Key::OrdersSentRedraw))));
                    self.drew = Some(r.row.instance.clone());
                }
                Ok(None) => {}
                Err(e) => self.said = Some(bad(e)),
            }
        }
    }

    pub fn tell(&mut self, s: Said) {
        self.said_on = self.open.as_ref().map(|o| o.instance.to_ascii_lowercase());
        self.said = Some(s);
    }

    pub fn said_here(&self) -> Option<&Said> {
        let here = self.open.as_ref().map(|o| o.instance.to_ascii_lowercase());
        self.said.as_ref().filter(|_| self.said_on == here)
    }

    pub fn leave(&mut self) {
        if self.said_on.is_some() {
            self.said = None;
            self.said_on = None;
        }
    }

    pub fn row_of(&self, instance: &str) -> Option<&BRow> {
        self.rows.as_ref()?.as_ref().ok()?.iter().find(|r| r.row.instance.eq_ignore_ascii_case(instance))
    }

    pub fn shown(&self) -> Vec<&BRow> {
        let Some(Ok(r)) = &self.rows else { return Vec::new() };
        eravon_cli::deskops::buyerpage::shown_of(r, Some(self.done_tab), &self.q, &self.from, &self.to).0
    }

    fn hits(&self) -> Result<(), Said> {
        let Some(Ok(r)) = &self.rows else { return Ok(()) };
        eravon_cli::deskops::buyerpage::shown_of(r, Some(self.done_tab), &self.q, &self.from, &self.to).1.map_or(Ok(()), Err)
    }

    pub fn search_said(&self) -> Option<Said> {
        self.hits().err()
    }

    fn find(&self, instance: &str) -> Option<&BRow> {
        match &self.rows {
            Some(Ok(r)) => r.iter().find(|x| x.row.instance.eq_ignore_ascii_case(instance)),
            _ => None,
        }
    }

    pub fn readvance(&mut self, data: &str, me: &str) {
        let Some(i) = self.open.as_ref().map(|o| o.instance.clone()) else { return };
        let Some(r) = self.find(&i).cloned().filter(|r| r.row.chain.as_ref().is_some_and(|v| v.phase != 0)) else { return };
        let Ok(w) = crate::house::orders::wire(data, me) else { return };
        let due = act::advance_card(&w, &r.case()).map_err(bad);
        if let Some(o) = self.open.as_mut() {
            o.advance = Some(due);
        }
    }

    pub fn open_row(&mut self, data: &str, me: &str, instance: &str, tab: Tab) {
        if self.said_on.as_deref().is_some_and(|s| !s.eq_ignore_ascii_case(instance)) {
            self.leave();
        }
        let Some(r) = self.find(instance).cloned() else {
            self.tell(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable)));
            return;
        };
        let keep = self.open.take().filter(|o| o.instance.eq_ignore_ascii_case(instance));
        let w = crate::house::orders::wire(data, me).ok();
        let detail = w.as_ref().map(|w| detail_rows(data, &r, w, &self.owners));
        let opened = r.row.chain.as_ref().is_some_and(|v| v.phase != 0);
        let advance = w.as_ref().filter(|_| opened).map(|w| act::advance_card(w, &r.case()).map_err(bad));
        let mut open = Open { instance: r.row.instance.clone(), tab, detail, advance, ..Open::default() };
        if let Some(k) = keep {
            (open.offer, open.judge, open.judge_open, open.sealable, open.sealed) = (k.offer, k.judge, k.judge_open, k.sealable, k.sealed);
            (open.inner, open.bench, open.papers) = (k.inner, k.bench, k.papers);
        }
        self.open = Some(open);
    }

    fn with_open<T>(&mut self, data: &str, me: &str, f: impl FnOnce(&eravon_cli::shared::Wire, &BRow) -> T) -> Option<T> {
        let Some(o) = self.open.as_ref() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::BoOpenThatFirst)));
            return None;
        };
        let Some(r) = self.find(&o.instance).cloned() else {
            self.said = Some(Said::bad("E_ORDERS_NONE", crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable)));
            return None;
        };
        let mut r = r;
        let w = match crate::house::orders::wire(data, me) {
            Ok(w) => w,
            Err(s) => {
                self.said = Some(s);
                return None;
            }
        };
        match live(data, &w, &r) {
            Ok(row) => r.row = row,
            Err((n, d)) => {
                self.said = Some(Said::bad(&n, d));
                return None;
            }
        }
        Some(f(&w, &r))
    }

}

fn live(data: &str, w: &eravon_cli::shared::Wire, r: &BRow) -> Result<eravon_orders::table::Row, eravon_watch::Why> {
    let mut row = r.row.clone();
    eravon_orders::table::reread(data, &r.owner, eravon_scan::scan::Side::Buyer, &crate::house::orders::sw(w), &mut row)?;
    Ok(row)
}


pub const OFFER_PART: &str = "settle-offer.txt";

pub fn rung_shown(index: u64) -> String {
    index.saturating_add(1).to_string()
}

#[cfg(test)]
mod company_rows {
    #[test]
    fn a_row_is_the_companys_when_the_company_acts_for_its_buyer_key() {
        let (old, new, other) = ("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000b1", "0x00000000000000000000000000000000000000c1");
        let asked = std::cell::Cell::new(false);
        assert!(super::company_row(Some(old), &old.to_ascii_uppercase().replacen("0X", "0x", 1), || { asked.set(true); None }), "簿读不成照原钥:原钥即公司");
        assert!(asked.get(), "原钥等于公司也问簿");
        assert!(!super::company_row(Some(old), old, || Some(other.to_string())), "公司记录已交给别家:不是本公司的单");
        assert!(super::company_row(Some(old), old, || Some(old.to_string())));
        assert!(super::company_row(Some(new), old, || Some(new.to_string())));
        assert!(!super::company_row(Some(new), old, || Some(other.to_string())));
        assert!(!super::company_row(Some(new), old, || None), "簿读不成照原钥");
        assert!(!super::company_row(None, old, || Some(old.to_string())), "公司模式关着");
    }
}

#[cfg(test)]
mod rung_numbers {
    #[test]
    fn the_rung_shown_counts_from_one() {
        assert_eq!(super::rung_shown(0), "1");
        assert_eq!(super::rung_shown(2), "3");
    }
}

crate::house::merge::lands!(BOrders {
    whole: [me],
    data: [co_other, ticket, rows, unscanned, follow, drew, owners, export_dir, exported, ask, seal_next, export_why],
    page: [done_tab, q, from, to],
    keyed(|_: &BOrders| ()): [],
    nested: [open],
    said: [said, said_on],
});

crate::house::merge::lands!(Open {
    whole: [],
    data: [],
    page: [instance, tab, card, refund, judge_open, inner],
    keyed(|m: &Open| m.instance.clone()): [detail, owed, offer, kits, first_kits, handed, judge, history, sealable, sealed, bench, papers, advance, complaint],
    nested: [],
    said: [],
});
