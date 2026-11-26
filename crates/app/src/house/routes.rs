use super::{it, Group, Item, Place, Role, Section, Step, ALERTS, HOME};
use crate::lang::Key;
use eravon_ui::icons::Glyph;

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum View {
    Orders,
    Queue,
    Goods,
    Cands,
    Export,
    Payways,
    Alerts,
    PayLink,
    Wallet,
}

impl View {
    pub const ALL: [View; 9] = [View::Orders, View::Queue, View::Goods, View::Cands, View::Export, View::Payways, View::Alerts, View::PayLink, View::Wallet];

    pub fn key(self) -> Key {
        match self {
            View::Orders => Key::NavOrders,
            View::Queue => Key::NavQueue,
            View::Goods => Key::NavGoods,
            View::Cands => Key::NavCands,
            View::Export => Key::IdExportGroup,
            View::Payways => Key::NavPayways,
            View::Alerts => Key::NavAlerts,
            View::PayLink => Key::TaskPaylink,
            View::Wallet => Key::TaskWallet,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            View::Orders => crate::house::seam::PAGE_ORDERS,
            View::Queue => crate::house::seam::PAGE_QUEUE,
            View::Goods => crate::house::seam::PAGE_GOODS,
            View::Cands => crate::house::seam::PAGE_CANDS,
            View::Export => crate::house::seam::PAGE_EXPORT,
            View::Payways => crate::house::seam::PAGE_PAYWAYS,
            View::Alerts => crate::house::seam::PAGE_ALERTS,
            View::PayLink => crate::house::seam::PAGE_PAYLINK,
            View::Wallet => crate::house::seam::PAGE_WALLET,
        }
    }
}

const ORDERS: Item = it(Place::View(View::Orders, 0), Key::NavOrders, Glyph::Receipt);
const QUEUE: Item = it(Place::View(View::Queue, 0), Key::NavQueue, Glyph::Ledger);
const EXPORT: Item = it(Place::View(View::Export, 0), Key::IdExportGroup, Glyph::House(eravon_ui::page_marks::PageMark::Upload));

pub const AUTHOR: [Group; 1] = [Group {
    title: None,
    items: &[
        HOME,
        ORDERS,
        QUEUE,
        it(Place::View(View::Goods, 0), Key::NavGoods, Glyph::Kit),
        it(Place::View(View::Cands, 0), Key::NavCands, Glyph::House(eravon_ui::page_marks::PageMark::Scale)),
        EXPORT,
        it(Place::View(View::Wallet, 0), Key::TaskWallet, Glyph::Vault),
        it(Place::View(View::Payways, 0), Key::NavPayways, Glyph::House(eravon_ui::page_marks::PageMark::Card)),
        ALERTS,
    ],
}];

pub const GRANTEE: [Group; 1] = [Group {
    title: None,
    items: &[HOME, ORDERS, QUEUE, it(Place::View(View::PayLink, 0), Key::TaskPaylink, Glyph::Inbox), it(Place::View(View::Wallet, 0), Key::TaskWallet, Glyph::Vault), ALERTS, EXPORT],
}];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Own {
    Opening,
    Company,
    Tokens,
    Zikaron,
}

impl Own {
    pub const ALL: [Own; 4] = [Own::Opening, Own::Company, Own::Tokens, Own::Zikaron];

    pub const SHOWN: [Own; 3] = [Own::Opening, Own::Tokens, Own::Zikaron];

    pub fn key(self) -> Key {
        match self {
            Own::Opening => Key::SetSecOpening,
            Own::Company => Key::SetCompanyMode,
            Own::Tokens => Key::SetSecTokens,
            Own::Zikaron => Key::SetSecZikaron,
        }
    }

    pub fn glyph(self) -> eravon_ui::icons::Glyph {
        use eravon_ui::icons::Glyph;
        match self {
            Own::Opening => Glyph::House(eravon_ui::page_marks::PageMark::Bolt),
            Own::Company => Glyph::House(eravon_ui::page_marks::PageMark::Building),
            Own::Tokens => Glyph::House(eravon_ui::page_marks::PageMark::Coin),
            Own::Zikaron => Glyph::House(eravon_ui::page_marks::PageMark::Stones),
        }
    }

    pub fn note(self) -> Key {
        match self {
            Own::Opening => Key::SetNoteOpening,
            Own::Company => Key::SetNoteCompany,
            Own::Tokens => Key::SetNoteTokens,
            Own::Zikaron => Key::SetNoteZikaron,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Own::Opening => "opening",
            Own::Company => "company",
            Own::Tokens => "tokens",
            Own::Zikaron => "zikaron",
        }
    }
}

pub fn sections(family: &[Section], role: Role) -> Vec<Section> {
    let mut v: Vec<Section> = family.iter().copied().filter(|s| *s != Section::About).collect();
    v.extend(Own::SHOWN.into_iter().filter(|o| *o != Own::Opening || role == Role::Author).map(Section::House));
    v.extend(family.iter().copied().filter(|s| *s == Section::About));
    v
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SeatStep {
    Network,
    Role,
    Tail,
}

impl SeatStep {
    pub fn title(self) -> Key {
        match self {
            SeatStep::Network => Key::SetNetwork,
            SeatStep::Role => Key::FirstRole,
            SeatStep::Tail => Key::SetPayout,
        }
    }

    pub fn on(self, role: Role) -> bool {
        match self {
            SeatStep::Network | SeatStep::Role => true,
            SeatStep::Tail => role == Role::Author,
        }
    }


    pub fn deferrable(self, _role: Role) -> bool {
        match self {
            SeatStep::Network | SeatStep::Role => false,
            SeatStep::Tail => true,
        }
    }

    pub fn opens(self, _role: Role) -> bool {
        matches!(self, SeatStep::Role)
    }
}

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Pin => "pin",
            Step::Key => "key",
            Step::Own(SeatStep::Network) => "network",
            Step::Own(SeatStep::Role) => "role",
            Step::Own(SeatStep::Tail) => "tail",
            Step::Backup => "backup",
        }
    }

    pub fn title_for(self, _role: Role) -> Key {
        self.title()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SeatDone {
    pub network: bool,
    pub role: bool,
    pub tail: bool,
    pub on: Role,
}

impl SeatDone {
    pub fn of(shell: &crate::shell::Shell) -> SeatDone {
        SeatDone { network: shell.desk.ready.seat || shell.desk.wiz_chain_done, role: shell.desk.ready.seat, tail: shell.desk.ready.tail, on: shell.settings.role }
    }

    pub fn done(&self, s: SeatStep) -> bool {
        match s {
            SeatStep::Network => self.network,
            SeatStep::Role => self.role,
            SeatStep::Tail => self.tail,
        }
    }

    pub fn gate(&self, want: Step) -> Option<Step> {
        match want {
            Step::Own(s) if !s.on(self.on) => Some(Step::Backup),
            Step::Own(SeatStep::Tail) if !self.role => Some(Step::Own(SeatStep::Role)),
            _ => None,
        }
    }

    pub fn wants_wizard(&self, _role: Role) -> bool {
        !self.role
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Detail {
    Order(String),
    BuyerOrder(String),
    Goods(String),
    Cand(String),
    PayLink,
    Inner(Box<Detail>, String),
}

impl Detail {
    pub fn name(&self) -> String {
        match self {
            Detail::Order(_) => "Order".into(),
            Detail::BuyerOrder(_) => "BuyerOrder".into(),
            Detail::Goods(_) => "Goods".into(),
            Detail::Cand(_) => "Cand".into(),
            Detail::PayLink => "PayLink".into(),
            Detail::Inner(_, w) => format!("Inner.{w}"),
        }
    }
}

#[cfg(test)]
mod shown_tests {
    use super::*;

    #[test]
    fn the_settings_home_lists_no_company_mode_and_the_section_stays() {
        for role in [Role::Author, Role::Grantee] {
            assert!(!sections(&[], role).contains(&Section::House(Own::Company)), "{role:?}");
        }
        assert!(Own::ALL.contains(&Own::Company) && Own::SHOWN.iter().all(|o| Own::ALL.contains(o)));
        assert_eq!(crate::house::settings::Section::of(Own::Company.as_str()).map(|s| s.name()), Some("company"), "set-section company 照旧");
    }
}
