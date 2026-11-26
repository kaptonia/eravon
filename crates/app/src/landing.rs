use crate::task::Kind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    Said,
    Vault,
    Gate,
    Path,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnOk {
    Toast,
    Quiet,
    Back(Back),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnErr {
    Said,
    Back(Back),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    Last,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Then {
    Nothing,
}

#[derive(Clone, Copy, Debug)]
pub struct Landing {
    pub kind: Kind,
    pub ok: OnOk,
    pub err: OnErr,
    pub kept: Kept,
    pub then: Then,
}

const fn row(kind: Kind, ok: OnOk, err: OnErr) -> Landing {
    Landing { kind, ok, err, kept: Kept::Last, then: Then::Nothing }
}

const TOAST: OnOk = OnOk::Toast;
const QUIET: OnOk = OnOk::Quiet;
const SAID: OnErr = OnErr::Said;

pub const TABLE: [Landing; 32] = [
    row(Kind::SelfCheck, TOAST, SAID),
    row(Kind::Archive, TOAST, SAID),
    row(Kind::Keystore, TOAST, SAID),
    row(Kind::Vault, OnOk::Back(Back::Vault), OnErr::Back(Back::Vault)),
    row(Kind::Backup, TOAST, SAID),
    row(Kind::Path, OnOk::Back(Back::Path), SAID),
    row(Kind::CliPath, TOAST, SAID),
    row(Kind::Sync, QUIET, SAID),
    row(Kind::Orders, QUIET, SAID),
    row(Kind::Goods, QUIET, SAID),
    row(Kind::Cands, QUIET, SAID),
    row(Kind::Home, QUIET, SAID),
    row(Kind::Remind, QUIET, SAID),
    row(Kind::PayLink, QUIET, SAID),
    row(Kind::Bo, QUIET, SAID),
    row(Kind::History, QUIET, SAID),
    row(Kind::BHome, QUIET, SAID),
    row(Kind::Wallet, QUIET, SAID),
    row(Kind::Settings, QUIET, SAID),
    row(Kind::Export, QUIET, SAID),
    row(Kind::First, QUIET, SAID),
    row(Kind::Door, QUIET, SAID),
    row(Kind::Seat, OnOk::Back(Back::Said), OnErr::Back(Back::Said)),
    row(Kind::Keep, QUIET, SAID),
    row(Kind::OrdersRead, QUIET, SAID),
    row(Kind::CandsRead, QUIET, SAID),
    row(Kind::BoRead, QUIET, SAID),
    row(Kind::WalletRead, QUIET, SAID),
    row(Kind::SettingsRead, QUIET, SAID),
    row(Kind::ExportRead, QUIET, SAID),
    row(Kind::GoodsRead, QUIET, SAID),
    row(Kind::CandsLocal, QUIET, SAID),
];

pub fn of(k: Kind) -> &'static Landing {
    TABLE.iter().find(|r| r.kind == k).unwrap_or(&TABLE[0])
}

pub fn goes_back(k: Kind, back: Back) -> bool {
    of(k).ok == OnOk::Back(back)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_row() {
        assert_eq!(TABLE.len(), Kind::ALL.len());
        for (r, k) in TABLE.iter().zip(Kind::ALL) {
            assert_eq!(r.kind, k, "row out of order or missing");
        }
    }

    #[test]
    fn what_goes_back_goes_back_both_ways() {
        for r in TABLE {
            if let OnOk::Back(b) = r.ok {
                if b != Back::Path {
                    assert_eq!(r.err, OnErr::Back(b), "{:?}", r.kind);
                }
            }
            if let OnErr::Back(b) = r.err {
                assert_eq!(r.ok, OnOk::Back(b), "{:?}", r.kind);
            }
        }
    }
}
