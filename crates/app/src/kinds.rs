#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Kind {
    SelfCheck,
    Archive,
    Keystore,
    Vault,
    Backup,
    Path,
    CliPath,
    Sync,
    Orders,
    Goods,
    Cands,
    Home,
    Remind,
    PayLink,
    Bo,
    History,
    BHome,
    Wallet,
    Settings,
    Export,
    First,
    Door,
    Seat,
    Keep,
    OrdersRead,
    CandsRead,
    BoRead,
    WalletRead,
    SettingsRead,
    ExportRead,
    GoodsRead,
    CandsLocal,
}

impl Kind {
    pub fn waited_at_quit(self) -> bool {
        match self {
            Kind::Keystore | Kind::Vault | Kind::Backup | Kind::Goods | Kind::Cands | Kind::Settings | Kind::Export | Kind::First | Kind::Door | Kind::Seat | Kind::Keep => true,
            Kind::SelfCheck | Kind::Archive | Kind::Sync | Kind::Orders | Kind::Home | Kind::Remind | Kind::PayLink | Kind::Bo | Kind::History | Kind::BHome | Kind::Wallet
            | Kind::OrdersRead | Kind::CandsRead | Kind::BoRead | Kind::WalletRead | Kind::SettingsRead | Kind::ExportRead | Kind::GoodsRead | Kind::CandsLocal
            | Kind::Path
            | Kind::CliPath => false,
        }
    }

    pub fn writes_local(self) -> bool {
        match self {
            Kind::Keystore | Kind::Goods | Kind::Cands | Kind::Orders | Kind::PayLink | Kind::Bo | Kind::Wallet | Kind::Settings | Kind::Door | Kind::Seat | Kind::Keep => true,
            Kind::OrdersRead | Kind::CandsRead | Kind::BoRead | Kind::WalletRead | Kind::SettingsRead => true,
            Kind::SelfCheck | Kind::Archive | Kind::Vault | Kind::Backup | Kind::Sync | Kind::Home | Kind::Remind | Kind::History | Kind::BHome | Kind::Export | Kind::First | Kind::Path | Kind::CliPath => false,
            Kind::ExportRead | Kind::GoodsRead | Kind::CandsLocal => false,
        }
    }

    pub const ALL: [Kind; 32] = [
        Kind::SelfCheck,
        Kind::Archive,
        Kind::Keystore,
        Kind::Vault,
        Kind::Backup,
        Kind::Path,
        Kind::CliPath,
        Kind::Sync,
        Kind::Orders,
        Kind::Goods,
        Kind::Cands,
        Kind::Home,
        Kind::Remind,
        Kind::PayLink,
        Kind::Bo,
        Kind::History,
        Kind::BHome,
        Kind::Wallet,
        Kind::Settings,
        Kind::Export,
        Kind::First,
        Kind::Door,
        Kind::Seat,
        Kind::Keep,
        Kind::OrdersRead,
        Kind::CandsRead,
        Kind::BoRead,
        Kind::WalletRead,
        Kind::SettingsRead,
        Kind::ExportRead,
        Kind::GoodsRead,
        Kind::CandsLocal,
    ];

    pub fn follows_source(self) -> bool {
        match self {
            Kind::SelfCheck | Kind::Keystore | Kind::Vault | Kind::Backup | Kind::Path | Kind::CliPath | Kind::First | Kind::Door | Kind::Seat | Kind::Keep => false,
            Kind::Archive | Kind::Sync | Kind::Orders | Kind::Goods | Kind::Cands | Kind::Home | Kind::Remind | Kind::PayLink | Kind::Bo | Kind::History | Kind::BHome | Kind::Wallet | Kind::Settings | Kind::Export => true,
            Kind::OrdersRead | Kind::CandsRead | Kind::BoRead | Kind::WalletRead | Kind::SettingsRead | Kind::ExportRead | Kind::GoodsRead | Kind::CandsLocal => true,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::SelfCheck => "selfcheck",
            Kind::Archive => "archive",
            Kind::Keystore => "keystore",
            Kind::Vault => "vault",
            Kind::Backup => "backup",
            Kind::Path => "path",
            Kind::CliPath => "clipath",
            Kind::Sync => "sync",
            Kind::Orders => "orders",
            Kind::Goods => "goods",
            Kind::Cands => "cands",
            Kind::Home => "home",
            Kind::Remind => "remind",
            Kind::PayLink => "paylink",
            Kind::Bo => "bo",
            Kind::History => "history",
            Kind::BHome => "bhome",
            Kind::Wallet => "wallet",
            Kind::Settings => "settings",
            Kind::Export => "export",
            Kind::First => "first",
            Kind::Door => "door",
            Kind::Seat => "seat",
            Kind::Keep => "keep",
            Kind::OrdersRead => "orders-read",
            Kind::CandsRead => "cands-read",
            Kind::BoRead => "bo-read",
            Kind::WalletRead => "wallet-read",
            Kind::SettingsRead => "settings-read",
            Kind::ExportRead => "export-read",
            Kind::GoodsRead => "goods-read",
            Kind::CandsLocal => "cands-local",
        }
    }
}

impl Kind {
    pub fn keeps_reads(self) -> bool {
        match self {
            Kind::Orders | Kind::Goods | Kind::Cands | Kind::Home | Kind::PayLink | Kind::Bo | Kind::History | Kind::BHome | Kind::Wallet | Kind::Settings | Kind::Export | Kind::First | Kind::Door => true,
            Kind::OrdersRead | Kind::CandsRead | Kind::BoRead | Kind::WalletRead | Kind::SettingsRead | Kind::ExportRead | Kind::GoodsRead | Kind::CandsLocal => true,
            Kind::Sync | Kind::SelfCheck | Kind::Archive | Kind::Keystore | Kind::Vault | Kind::Backup | Kind::Path | Kind::CliPath | Kind::Remind | Kind::Seat | Kind::Keep => false,
        }
    }

    pub fn reads_nodes(self) -> bool {
        false
    }

    pub fn keeper() -> Option<std::sync::Arc<dyn eravon_chain::settled::Keep>> {
        crate::chainx::settled_keeper()
    }

    pub fn carries(self) -> bool {
        false
    }

    pub fn wakes_when_done(self) -> bool {
        false
    }

    pub fn read_of(self) -> Option<Kind> {
        match self {
            Kind::Orders => Some(Kind::OrdersRead),
            Kind::Cands => Some(Kind::CandsRead),
            Kind::Bo => Some(Kind::BoRead),
            Kind::Wallet => Some(Kind::WalletRead),
            Kind::Settings => Some(Kind::SettingsRead),
            Kind::Export => Some(Kind::ExportRead),
            Kind::Goods => Some(Kind::GoodsRead),
            _ => None,
        }
    }

    pub fn page_of(self) -> Kind {
        match self {
            Kind::OrdersRead => Kind::Orders,
            Kind::CandsRead => Kind::Cands,
            Kind::BoRead => Kind::Bo,
            Kind::WalletRead => Kind::Wallet,
            Kind::SettingsRead => Kind::Settings,
            Kind::ExportRead => Kind::Export,
            Kind::GoodsRead => Kind::Goods,
            Kind::CandsLocal => Kind::Cands,
            k => k,
        }
    }
}
