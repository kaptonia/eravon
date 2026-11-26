use eravon_cands::fault::Absent;
use eravon_cands::table::Side;
use eravon_cands::view::{Brief, Detail};
use eravon_literals::json::Out;
use eravon_payways::ways::{self, Ways};
use eravon_store::record::Candidate;

use crate::house::seam;


mod pay;
pub use pay::{Pay};
mod cands;
pub use cands::{pill_of, Cands, Field, Pill};
pub(crate) mod goods;
pub use goods::{Goods, GoodsEdit, GoodsInner, Leave, Qr, Shown};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Payways,
    Cands,
    Goods,
    Settings,
    Orders,
    Alerts,
    PayLink,
    Wallet,
    Export,
    Queue,
}

impl Page {
    pub const fn name(self) -> &'static str {
        match self {
            Page::Payways => seam::PAGE_PAYWAYS,
            Page::Cands => seam::PAGE_CANDS,
            Page::Goods => seam::PAGE_GOODS,
            Page::Settings => seam::PAGE_SETTINGS,
            Page::Orders => seam::PAGE_ORDERS,
            Page::Alerts => seam::PAGE_ALERTS,
            Page::PayLink => seam::PAGE_PAYLINK,
            Page::Wallet => seam::PAGE_WALLET,
            Page::Export => seam::PAGE_EXPORT,
            Page::Queue => seam::PAGE_QUEUE,
        }
    }
    pub fn of(s: &str) -> Option<Option<Page>> {
        match s {
            seam::PAGE_PAYWAYS => Some(Some(Page::Payways)),
            seam::PAGE_CANDS => Some(Some(Page::Cands)),
            seam::PAGE_GOODS => Some(Some(Page::Goods)),
            seam::PAGE_SETTINGS => Some(Some(Page::Settings)),
            seam::PAGE_ORDERS => Some(Some(Page::Orders)),
            seam::PAGE_ALERTS => Some(Some(Page::Alerts)),
            seam::PAGE_PAYLINK => Some(Some(Page::PayLink)),
            seam::PAGE_WALLET => Some(Some(Page::Wallet)),
            seam::PAGE_EXPORT => Some(Some(Page::Export)),
            seam::PAGE_QUEUE => Some(Some(Page::Queue)),
            seam::PAGE_HOME => Some(None),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CandsPlan {
    Add(Side, String, String),
    Label(String),
    Remove,
}

pub use eravon_cli::deskops::said::{Said, Tone};

#[derive(Clone, Debug, Default)]
pub struct Seller {
    page: Option<Page>,
    pub pay: Option<Pay>,
    pub cands: Option<Cands>,
    pub field: Field,
    pub goods: Option<Goods>,
    pub settings: Option<crate::house::settings::Settings>,
    pub orders: Option<crate::house::orders::Orders>,
    pub home: Option<crate::house::home::Home>,
}

impl Seller {
    pub fn page(&self) -> Option<Page> {
        self.page
    }

    pub fn set_page(&mut self, _door: &crate::house::shell::Door, p: Option<Page>) {
        self.page = p;
    }

    pub fn rows(&self) -> Vec<(&'static str, Out)> {
        let mut m = Vec::new();
        if let Some(p) = &self.pay {
            m.push(("payways", p.rows()));
        }
        if let Some(c) = &self.cands {
            m.push(("cands", c.rows()));
        }
        if let Some(g) = &self.goods {
            m.push(("goods", g.rows()));
        }
        if let Some(o) = &self.orders {
            m.push(("orders", o.rows()));
        }
        if let Some(h) = &self.home {
            m.push(("home", h.rows()));
        }
        if let Some(x) = &self.settings {
            m.push(("settings", x.rows()));
        }
        if self.pay.is_some() || self.cands.is_some() || self.goods.is_some() || self.settings.is_some() {
            m.push(("page", Out::S(self.page.map_or(seam::PAGE_HOME, Page::name).to_string())));
        }
        m
    }
}

use eravon_goods::draft::Draft;
use eravon_goods::goods::{self as g1, Row, Saved};

