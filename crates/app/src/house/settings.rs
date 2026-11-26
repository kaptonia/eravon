use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;
use eravon_literals::json::Out;
use eravon_store::settings;
use eravon_vault as h2;

use crate::house::seam;
use crate::house::seller::Said;
use crate::house::shell::Seat;


mod keys;
mod account;
pub use account::gas_of;
pub(crate) use account::payout_land;
mod network;
pub use network::{asked, nodes_of, verbs, zk_away, zk_kits, zk_state};
use network::*;
mod company;
pub use company::{CompanyCard, CompanyOpen, company_gate, company_label, company_plan, kind_key};
mod general;
mod rotate;
pub use rotate::RotView;
mod cli;
pub use cli::{cli_read, cli_word};
pub use general::{every_of, every_parse, every_words, lang_now, look_now, look_of, reading_now, zone_now, zone_of, EVERY_MAX};
mod rows;
mod tokens;
pub use tokens::{TokenForm, TokenSheet};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRow {
    pub address: String,
    pub label: String,
    pub primary: bool,
    pub current: bool,
    pub kind: String,
    pub created: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZkState {
    Linked { at: String, layer: String, kits: usize, absent: Option<(String, String)> },
    Absent { name: String, detail: String },
    Bad { name: String, detail: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    General,
    Look,
    Identity,
    Company,
    Opening,
    Network,
    Notify,
    Tokens,
    Zikaron,
    Data,
    About,
}

impl Section {
    pub const ALL: [Section; 11] = [
        Section::General,
        Section::Look,
        Section::Identity,
        Section::Opening,
        Section::Company,
        Section::Network,
        Section::Notify,
        Section::Tokens,
        Section::Zikaron,
        Section::Data,
        Section::About,
    ];

    pub const HOME: [Section; 11] = [
        Section::General,
        Section::Look,
        Section::Identity,
        Section::Network,
        Section::Notify,
        Section::Data,
        Section::Opening,
        Section::Company,
        Section::Tokens,
        Section::Zikaron,
        Section::About,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Section::Identity => "identity",
            Section::Company => "company",
            Section::Opening => "opening",
            Section::Data => "data",
            Section::Network => "network",
            Section::Notify => "notify",
            Section::Zikaron => "zikaron",
            Section::Tokens => "tokens",
            Section::General => "general",
            Section::Look => "look",
            Section::About => "about",
        }
    }

    pub fn of(name: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|x| x.name() == name)
    }

    pub fn in_seat(self, seat: Seat) -> bool {
        self != Section::Opening || seat == Seat::Seller
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub ticket: crate::house::Ticket,
    pub section: Option<Section>,
    pub zk_link: bool,
    pub auto_lock: bool,
    pub idle_minutes: u64,
    pub last_backup: Option<(u64, String)>,
    pub changed: Option<usize>,
    pub last_backup_failed: Option<u64>,
    pub seat: Seat,
    pub me: String,
    pub keys: Result<Vec<KeyRow>, Said>,
    pub gas: Option<Result<(String, String, usize), Said>>,
    pub endpoints: Vec<String>,
    pub rest: Vec<(String, bool)>,
    pub rest_on: bool,
    pub zk: (String, ZkState),
    pub language: String,
    pub zone: String,
    pub refresh_every: u64,
    pub way: String,
    pub way_unread: String,
    pub look: String,
    pub company: CompanyCard,
    pub tokens: Vec<eravon_store::settings::TokenRow>,
    pub token_form: TokenForm,
    pub token_sheet: Option<TokenSheet>,
    pub token_read: Vec<(String, String)>,
    pub cli_path: Option<eravon_os::cli_path::State>,
    pub cli_anchor: (String, bool),
    pub gas_book: Option<(String, String, u64)>,
    pub gas_asked: bool,
    pub rotate: Option<Result<RotView, Said>>,
    pub said: Option<Said>,
}

#[derive(Clone, Debug, Default)]
pub struct Env {
    pub home: String,
}

fn bad(name: &str, detail: impl Into<String>) -> Said {
    Said { tone: crate::house::seller::Tone::Bad, name: name.to_string(), detail: detail.into() }
}

fn ok(detail: impl Into<String>) -> Said {
    Said { tone: crate::house::seller::Tone::Ok, name: "ok".into(), detail: detail.into() }
}

fn h2bad(e: h2::vfault::Fault) -> Said {
    crate::house::said_of(&e)
}

#[allow(clippy::type_complexity)]
fn lock_rows(m: &crate::machine::Machine) -> (bool, u64, Option<(u64, String)>, Option<usize>) {
    let now = crate::backup::measured(m.backup.as_ref()).ok();
    (m.auto_lock, m.auto_lock_secs / 60, m.backup.as_ref().map(|b| (b.at, b.path.clone())), crate::machine::backup_behind(m.backup.as_ref(), now).map(|n| n as usize))
}


impl Settings {
    pub fn open(data: &str, seat: Seat, env: &Env) -> Settings {
        let mut s = Settings {
            ticket: crate::house::Ticket::standing(),
            section: None,
            zk_link: eravon_literals::constants::ZIKARON_LINK_DEFAULT_ON,
            auto_lock: eravon_literals::constants::AUTO_LOCK_DEFAULT_ON,
            idle_minutes: u64::from(eravon_literals::constants::AUTO_LOCK_DEFAULT_MINUTES),
            last_backup: None,
            changed: None,
            last_backup_failed: None,
            seat,
            me: String::new(),
            keys: Ok(Vec::new()),
            gas: None,
            endpoints: Vec::new(),
            rest: Vec::new(),
            rest_on: false,
            zk: (String::new(), ZkState::Absent { name: String::new(), detail: String::new() }),
            language: String::new(),
            zone: String::new(),
            refresh_every: 0,
            way: "system".to_string(),
            way_unread: String::new(),
            look: String::new(),
            company: CompanyCard::default(),
            tokens: Vec::new(),
            token_form: TokenForm::default(),
            token_sheet: None,
            token_read: Vec::new(),
            cli_path: None,
            cli_anchor: (eravon_seal::machine::CliAnchor::Send.as_str().to_string(), true),
            gas_book: None,
            gas_asked: false,
            rotate: None,
            said: None,
        };
        s.reload(data, env);
        s
    }

    pub fn reload(&mut self, data: &str, env: &Env) {
        let now_me = crate::house::shell::current_key(data, self.seat).unwrap_or_default();
        self.keys = keys(data, self.seat);
        if let Ok(g) = eravon_vault::store::read_global(data) {
            self.refresh_every = every_of(g.refresh_every);
        }
        if let Ok(m) = crate::machine::read() {
            (self.auto_lock, self.idle_minutes, self.last_backup, self.changed) = lock_rows(&m);
            self.last_backup_failed = m.backup_failed;
            self.language = m.lang.map(|l| l.as_str().to_string()).unwrap_or_default();
            (self.way, self.way_unread) = match m.proxy.as_deref().map(str::trim).unwrap_or_default() {
                "" => ("system".to_string(), String::new()),
                saved => match eravon_chain::transport::choice_of(saved) {
                    Ok(c) => (eravon_chain::transport::choice_word(&c), String::new()),
                    Err(e) => ("system".to_string(), e.detail()),
                },
            };
            self.look = look_of(m.appearance.as_deref().unwrap_or_default()).to_string();
            self.cli_anchor = (m.cli_anchor.as_str().to_string(), eravon_cli::home::anchor_live(&m));
        }
        self.zone = reading_now(data).0.as_str().to_string();
        self.cli_path = Some(cli_read());
        self.read_tokens(data);
        let home = eravon_vault::store::home_settings(data, &now_me).ok().flatten();
        self.rest_on = home.as_ref().is_some_and(|h| h.rest_on);
        let keys = eravon_vault::machine::rest_keys_now();
        let keyed = |b: &str| eravon_chain::rest::service_key(b).is_some_and(|svc| keys.iter().any(|(k, v)| k.eq_ignore_ascii_case(&svc) && !v.is_empty()));
        self.rest = home.map(|h| h.rest).unwrap_or_default().into_iter().map(|b| {
            let k = keyed(&b);
            (b, k)
        }).collect();
        if !reload_scope(&self.me, &now_me) {
            return;
        }
        self.me = now_me;
        self.gas = None;
        if self.me.is_empty() {
            let why = crate::house::shell::current_key(data, self.seat).err().unwrap_or_default();
            self.said = Some(bad("E_S9_NO_KEY", crate::fill!(crate::house::i18n::Key::SetNoKeyYet; why)));
            self.endpoints.clear();
            self.zk = (String::new(), zk_state(&env.home, ""));
            return;
        }
        let ident = match eravon_vault::store::read_identity(data, &self.me) {
            Ok(t) => t,
            Err(eravon_store::fault::Fault::Missing(_)) => settings::Identity::default(),
            Err(e) => {
                self.said = Some(bad(e.name(), e.detail()));
                settings::Identity::default()
            }
        };
        self.endpoints = match eravon_vault::store::rpc_endpoints(data, &self.me) {
            Ok(n) => n,
            Err(e) => {
                self.said = Some(bad(e.name(), e.detail()));
                Vec::new()
            }
        };
        self.zk_link = ident.zikaron_on();
        self.company.on = ident.company_on();
        self.company.address = ident.company.clone();
        self.company.time_check = ident.time_check_on();
        self.zk = if crate::house::orders::zk_linked(&ident) {
            (ident.zikaron_dir.clone(), zk_state(&env.home, &ident.zikaron_dir))
        } else {
            (ident.zikaron_dir.clone(), ZkState::Absent { name: crate::house::orders::ZK_OFF.to_string(), detail: String::new() })
        };
    }
}

fn reload_scope(was: &str, now: &str) -> bool {
    was.is_empty() || !was.eq_ignore_ascii_case(now)
}



pub(crate) fn keys(data: &str, seat: Seat) -> Result<Vec<KeyRow>, Said> {
    let v = h2::vault::read(data).map_err(h2bad)?;
    let active = h2::accounts::active(data, seat.store()).map_err(h2bad)?;
    Ok(h2::accounts::list(&v, seat.store())
        .into_iter()
        .map(|i| KeyRow {
            address: i.address.clone(),
            label: i.label.clone(),
            primary: i.primary,
            current: active.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(&i.address)),
            kind: i.kind.word().to_string(),
            created: i.created.clone(),
        })
        .collect())
}

pub(crate) fn write_global(data: &str, now: u64, f: impl FnOnce(&mut settings::Global)) -> Result<(), Said> {
    eravon_cli::deskops::write_global(data, seam::WRITER, now, f).map_err(|n| bad(&n.name, n.detail()))
}

pub const THIRD_PARTY: &str = crate::about::NOTICES;

pub fn notice_count() -> usize {
    crate::about::count()
}

pub fn about() -> (String, String) {
    const LOCK: &[u8] = include_bytes!("../../../../BASE-LOCK.json");
    let commit = eravon_literals::json::read(LOCK)
        .ok()
        .and_then(|v| v.get("sourceCommit").and_then(|x| x.as_str().map(str::to_string)))
        .unwrap_or_default();
    let digest = eravon_galeed::hash::hex_encode(&eravon_galeed::hash::sha256(LOCK));
    (format!("ERAVON {}", env!("CARGO_PKG_VERSION")), format!("{commit} · {}", &digest[..18.min(digest.len())]))
}

pub const LOOKS: [&str; 3] = ["light", "dark", "system"];

pub(crate) fn took(said: Option<&crate::house::seller::Said>) -> bool {
    said.is_none_or(|x| x.tone != crate::house::seller::Tone::Bad)
}

crate::house::merge::lands!(Settings {
    whole: [me, seat],
    data: [ticket, zk_link, auto_lock, idle_minutes, last_backup, changed, last_backup_failed, keys, gas, endpoints, rest, rest_on, zk, language, zone, refresh_every, way, way_unread, look, company, tokens, token_read, cli_path, cli_anchor, gas_book, gas_asked, rotate],
    page: [section, token_form, token_sheet],
    keyed(|_: &Settings| ()): [],
    nested: [],
    said: [said],
});
