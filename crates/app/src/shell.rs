use crate::fault::{Fault, Known};
use crate::feature::Feature;
use crate::home::Home;
use crate::key::Address;
use crate::lock::Lock;
use crate::probe::Report;
use crate::task::Done;
use crate::settings::Settings;
use crate::task::{Outcome, Tasks};
use crate::trace;
use eravon_ui::fonts;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CliAloneNow {
    Off,
    Keyless,
    Keyed(usize),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Banner {
    None,
    ReadOnly(String),
    OtherMachine,
    MarkUnread,
}

impl Banner {
    pub fn as_str(&self) -> &'static str {
        match self {
            Banner::None => "none",
            Banner::ReadOnly(_) => "read_only",
            Banner::OtherMachine => "other_machine",
            Banner::MarkUnread => "mark_unread",
        }
    }
}

pub use crate::nav::Page;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Vault {
    Read(crate::keybox::State),
    Damaged(Fault),
}

impl Vault {
    pub fn of(read: Result<crate::keybox::State, Fault>) -> Vault {
        match read {
            Ok(s) => Vault::Read(s),
            Err(f) => Vault::Damaged(f),
        }
    }

    pub fn gate_up(&self) -> bool {
        match self {
            Vault::Read(s) => s.gate_up(),
            Vault::Damaged(_) => true,
        }
    }

    pub fn keys_ready(&self) -> bool {
        match self {
            Vault::Read(s) => s.keys_ready(),
            Vault::Damaged(_) => false,
        }
    }

    pub fn absent(&self) -> bool {
        matches!(self, Vault::Read(crate::keybox::State::Absent))
    }

    pub fn is(&self, s: crate::keybox::State) -> bool {
        matches!(self, Vault::Read(x) if *x == s)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Vault::Read(s) => s.as_str(),
            Vault::Damaged(_) => "damaged",
        }
    }
}

pub struct Hydrated {
    pub settings: Settings,
}

impl Hydrated {
    pub fn read(home: &crate::home::Home) -> Result<Hydrated, Fault> {
        let settings = Settings::read(home)?;
        Ok(Hydrated { settings })
    }
}

pub fn build_kind() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Ledger,
    Home,
}

#[derive(Clone, Debug)]
pub struct ArchiveRead {
    pub bytes: u64,
    pub records: usize,
}

pub struct Shell {
    pub page: Page,
    pub tasks: Tasks,
    pub fonts: fonts::Found,
    pub font_missing: Vec<fonts::Role>,
    pub last: Option<Report>,
    pub faults: Vec<Fault>,
    pub skinned: bool,
    pub anchor: Option<Address>,
    pub identities: Option<crate::identity::Registry>,
    pub new_words: Option<crate::identity::Fresh>,
    pub new_label: Option<String>,
    pub words: Option<Vec<crate::secret::Secret>>,
    pub home: Option<Home>,
    pub lock: Option<Lock>,
    pub settings: Settings,
    pub vault: Vault,
    pub vault_recoverable: bool,
    pub vault_said: Option<crate::action::Applied>,
    pub said: std::collections::BTreeMap<crate::task::Kind, crate::action::Applied>,
    pub machine: crate::machine::Machine,
    held_back: Vec<Outcome>,
    pub backup_seen: Option<crate::identity::BackupSeen>,
    pub status: Option<(crate::lang::Key, String)>,
    pub failed: std::collections::BTreeMap<crate::task::Kind, Fault>,
    pub primary: Option<(String, crate::keybox::PrimaryKind)>,
    pub documents: String,
    pub rekeying: bool,
    pub seat_wish: Option<crate::roles::Role>,
    pub identity_wish: Option<String>,
    pub lock_pending: bool,
    pub backup_made: Option<(String, crate::backup::Summary)>,
    pub backup_peek: Option<(String, crate::backup::Summary)>,
    pub items_now: Option<u64>,
    pub archive: Option<ArchiveRead>,
    pub desk: crate::house::desk::Desk,
    pub refresh_every: u64,
    pub door: Option<crate::door::Door>,
    pub door_doing: Option<crate::door::Doing>,
    pub door_told: Vec<crate::action::Applied>,
    pub door_waker: Option<crate::door::Waker>,
    pub door_shut: Option<(std::path::PathBuf, Fault)>,
    pub door_too_long: bool,
    pub door_place: Option<(std::path::PathBuf, std::path::PathBuf)>,
    pub door_off: bool,
    pub door_answered: usize,
    pub cli_path: Option<eravon_os::cli_path::State>,
    pub ai_config: Option<String>,
    pub credential: Option<eravon_seal::credential::Term>,
    pub passes: Vec<eravon_seal::credential::Row>,
    pub keeps: Vec<Keep>,
    pub keep_inline: bool,
    pub copies: crate::reads::Held,
    pub archive_asked: bool,
}

fn documents_folder() -> String {
    let home = crate::platform::home_dir();
    let docs = home.as_ref().map(|h| h.join("Documents")).filter(|d| d.is_dir());
    docs.or(home).map(|p| p.display().to_string()).unwrap_or_default()
}

impl Shell {
    pub fn boot(dressed: eravon_ui::skin::Dressed) -> Shell {
        let found = dressed.found;
        let missing = dressed.missing;

        trace::open();
        crate::chainx::speak();
        let _ = crate::chainx::keys_file();
        let mut faults: Vec<Fault> = Vec::new();
        if let Some(f) = trace::trouble() {
            faults.push(f);
        }
        let machine = match crate::machine::read_face() {
            Ok(m) => m,
            Err(f) => {
                faults.push(f);
                crate::machine::Machine::default()
            }
        };
        trace::mark(Feature::H0);
        for r in &missing {
            faults.push(Fault::known(Known::FontMissing, format!("role {}", r.as_str())));
        }
        trace::mark(Feature::H1);

        Shell {
            documents: documents_folder(),
            page: Default::default(),
            tasks: Tasks::new(),
            fonts: found,
            font_missing: missing,
            last: None,
            faults,
            skinned: true,
            anchor: None,
            identities: None,
            new_words: None,
            new_label: None,
            words: None,
            home: None,
            lock: None,
            settings: Settings::default(),
            vault: Vault::of(crate::keybox::state()),
            vault_recoverable: crate::keybox::recoverable().unwrap_or(true),
            vault_said: None,
            said: std::collections::BTreeMap::new(),
            machine,
            held_back: Vec::new(),
            backup_seen: None,
            status: None,
            failed: std::collections::BTreeMap::new(),
            primary: primary_reading(),
            rekeying: false,
            seat_wish: None,
            identity_wish: None,
            lock_pending: false,
            backup_made: None,
            backup_peek: None,
            items_now: None,
            archive: None,
            refresh_every: eravon_literals::constants::READ_REFRESH_SECS,
            desk: crate::house::desk::Desk::default(),
            door: None,
            door_doing: None,
            door_told: Vec::new(),
            door_waker: None,
            door_shut: None,
            door_too_long: false,
            door_place: None,
            door_off: false,
            door_answered: 0,
            cli_path: None,
            ai_config: None,
            credential: None,
            passes: Vec::new(),
            keeps: Vec::new(),
            keep_inline: false,
            copies: crate::reads::Held::default(),
            archive_asked: false,
        }
    }

    pub fn refresh_anchor(&mut self) -> Result<bool, Fault> {
        if !self.unlocked() {
            self.anchor = None;
            return Ok(false);
        }
        match crate::key::load(crate::register::account_now()?.as_deref())? {
            Some(s) => {
                self.anchor = s.address();
                Ok(true)
            }
            None => {
                self.anchor = None;
                    Ok(false)
            }
        }
    }

    pub fn after_unlock(&mut self) {
        self.lock_pending = false;
        self.reread_vault();
        if let Some(o) = crate::local::take_opened() {
            self.faults.extend(o.troubles);
        }
        let opened_now = self.home.is_none() && self.unlocked();
        if self.unlocked() {
            match crate::machine::read() {
                Ok(m) => {
                    if m.backup != self.machine.backup {
                        self.items_now = None;
                    }
                    self.machine = m;
                }
                Err(f) => self.faults.push(f),
            }
            self.reread_credential();
        }
        if opened_now {
            let _ = crate::action::boot_home(self);
            if let Some(l) = self.settings.lang {
                crate::lang::set(l);
            }
            crate::when::set(crate::when::chosen(self.settings.zone));
        }
        if let Err(f) = self.refresh_anchor() {
            self.faults.push(f);
        }
        match crate::register::view() {
            Ok(v) => self.seat_identities(Some(v)),
            Err(f) => self.faults.push(f),
        }
        self.refresh_every = crate::house::general::read();
        self.unlock_house();
    }

    pub fn begin_lock(&mut self) {
        self.lock_pending = true;
        crate::keybox::begin_lock();
        self.reread_vault();
        self.lock_house();
        self.anchor = None;
        self.words = None;
        self.new_words = None;
    }

    pub fn peeked(&self, path: &str) -> Option<&crate::backup::Summary> {
        self.backup_peek.as_ref().filter(|(p, _)| p.trim() == path.trim()).map(|(_, s)| s)
    }

    pub fn after_lock(&mut self) {
        self.reread_vault();
        self.lock_house();
        self.backup_peek = None;
        self.anchor = None;
        self.words = None;
        self.new_words = None;
        self.unload();
    }

    pub fn unload(&mut self) {
        self.close_home();
        self.seat_identities(None);
        self.settings = Settings { role: self.settings.role, ..Settings::default() };
        self.new_label = None;
    }

    pub fn reread_vault(&mut self) {
        let v = Vault::of(crate::keybox::state());
        if let Vault::Damaged(f) = &v {
            self.faults.push(f.clone());
        }
        self.vault = v;
        self.vault_recoverable = crate::keybox::recoverable().unwrap_or(true);
        self.primary = primary_reading();
    }

    pub fn seat_identities(&mut self, reg: Option<crate::identity::Registry>) {
        self.backup_seen = reg
            .as_ref()
            .and_then(|r| r.now())
            .map(|(row, _)| crate::identity::backup_seen(row));
        self.identities = reg;
        self.primary = primary_reading();
    }

    pub fn take_measure_due(&mut self) -> bool {
        let due = self.home.is_some() && self.archive.is_none() && !self.archive_asked && !self.tasks.in_flight(crate::task::Kind::Archive);
        if due {
            self.archive_asked = true;
        }
        due
    }

    pub fn backup_carries(&self) -> Option<(usize, usize, usize)> {
        let ids = self.identities.as_ref().map(|r| r.rows.len()).unwrap_or(0);
        self.archive.as_ref().map(|a| (ids, 0, a.records))
    }

    pub fn identity_listed(&self) -> bool {
        self.identities.as_ref().and_then(|r| r.now()).is_some()
    }

    pub fn seat_unseated(&self) -> bool {
        self.identities
            .as_ref()
            .and_then(|r| r.now())
            .map(|(row, _)| row.address(self.settings.role).is_none())
            .unwrap_or(false)
    }

    pub fn close_home(&mut self) {
        self.source_changed(Source::Home);
        self.home = None;
        self.lock = None;
        self.door_sync();
    }

    pub fn speaks(&self) -> Option<crate::lang::Lang> {
        self.settings.lang.or(self.machine.lang)
    }

    pub fn rest_key_tail(&self, base: &str) -> Option<String> {
        self.machine.rest_keys.tail(&eravon_chain::rest::service_key(base)?)
    }

    pub fn rest_key_said(&self, base: &str) -> String {
        use crate::lang::{fill1, t, Key};
        match self.rest_key_tail(base) {
            Some(k) => fill1(Key::NetRestKeyTail, &k),
            None if eravon_chain::rest::service_key(base).is_some_and(|s| eravon_chain::rest::file_gives(&s)) => t(Key::NetRestKeyFromFile).to_string(),
            None => t(Key::NetRestNoKey).to_string(),
        }
    }

    pub fn unlocked(&self) -> bool {
        self.vault.keys_ready()
    }

    pub fn may_save_settings(&self) -> Result<(), Fault> {
        if self.home.is_none() {
            return Err(Fault::known(Known::NoHome, crate::lang::both_of(|| crate::lang::t(crate::lang::Key::Tail211).to_string())));
        }
        if !self.settings_writable() {
            return Err(Fault::known(
                Known::ReadOnly,
                crate::lang::both_of(|| match self.lock.as_ref() {
                    Some(l) => crate::lang::filln(crate::lang::Key::Tail006, &[&(l.holder()).to_string()]),
                    None => crate::lang::t(crate::lang::Key::Tail212).to_string(),
                }),
            ));
        }
        Ok(())
    }

    pub fn save_settings(&self) -> Result<(), Fault> {
        self.may_save_settings()?;
        let h = self.home.as_ref().expect("上一句已经问过家在不在");
        self.settings.write(h)
    }

    pub fn commit_settings(&mut self, f: impl FnOnce(&mut Settings)) -> Result<(), Fault> {
        self.may_save_settings()?;
        let mut next = self.settings.clone();
        f(&mut next);
        let h = self.home.as_ref().expect("上一句已经问过家在不在");
        next.write(h)?;
        let nodes_moved = crate::chainx::nodes_of(&self.settings) != crate::chainx::nodes_of(&next);
        self.settings = next;
        if nodes_moved {
            crate::chainx::forget_serving();
        }
        Ok(())
    }

    pub fn banner(&self) -> Banner {
        if self.home.is_some() && self.lock.as_ref().map(|l| l.mode() == crate::lock::Mode::OtherMachine).unwrap_or(false) {
            return match self.lock.as_ref().and_then(|l| l.mark_trouble()) {
                Some(_) => Banner::MarkUnread,
                None => Banner::OtherMachine,
            };
        }
        if self.home.is_some() && !self.writable() {
            return Banner::ReadOnly(match self.lock.as_ref() {
                Some(l) => l.holder().to_string(),
                None => String::new(),
            });
        }
        Banner::None
    }

    pub fn leave(&mut self, k: Keep) {
        self.keeps.push(k);
    }

    pub fn keep_turn(&mut self) {
        if self.keep_inline || self.keeps.is_empty() || self.tasks.in_flight(crate::task::Kind::Keep) {
            return;
        }
        let all = std::mem::take(&mut self.keeps);
        let _ = crate::action::behind(self, crate::task::Kind::Keep, move || Ok(Done::Kept(keep(all))));
    }

    pub fn kept(&mut self, all: &[Kept]) {
        use crate::reads::Copies;
        for k in all {
            match k {
                Kept::Machine(Ok(m), at) => {
                    if self.copies.takes(Copies::Machine, *at) {
                        self.machine = m.clone();
                    } else {
                        self.leave(Keep::Machine);
                    }
                }
                Kept::Machine(Err(f), _) | Kept::Wrote(Err(f)) => self.faults.push(f.clone()),
                Kept::Wrote(Ok(())) => {}
                Kept::Register(Ok(v), at) => {
                    if self.copies.takes(Copies::Register, *at) {
                        self.seat_identities(Some(v.clone()));
                    }
                }
                Kept::Register(Err(f), _) => self.faults.push(f.clone()),
            }
        }
    }

    pub fn writable(&self) -> bool {
        self.lock.as_ref().map(|l| l.mode().writable()).unwrap_or(false)
    }

    pub fn settings_writable(&self) -> bool {
        self.lock.as_ref().map(|l| l.mode().writable()).unwrap_or(false)
    }

    pub fn hydrate(&mut self, home: &crate::home::Home) -> Result<(), Fault> {
        let read = Hydrated::read(home)?;
        self.install(read);
        Ok(())
    }

    pub fn install(&mut self, h: Hydrated) {
        let Hydrated { settings } = h;
        self.settings = settings;
    }

    pub fn source_changed(&mut self, scope: Source) {
        self.tasks.new_epoch();
        self.drop_parked();
        let home = scope == Source::Home;
        let Shell {
            page: _,
            documents: _,
            tasks: _,
            fonts: _,
            font_missing: _,
            last: _,
            faults: _,
            skinned: _,
            anchor: _,
            identities: _,
            backup_seen: _,
            new_words: _,
            new_label: _,
            words: _,
            vault: _,
            vault_recoverable: _,
            primary: _,
            rekeying: _,
            seat_wish: _,
            identity_wish: _,
            lock_pending: _,
            backup_made: _,
            backup_peek: _,
            items_now: _,
            archive,
            refresh_every: _,
            desk: _,
            door: _,
            door_doing: _,
            door_told: _,
            door_waker: _,
            door_shut: _,
            door_too_long: _,
            door_place: _,
            door_off: _,
            door_answered: _,
            cli_path: _,
            ai_config: _,
            credential: _,
            passes: _,
            keeps: _,
            keep_inline: _,
            copies: _,
            archive_asked,
            vault_said: _,
            said: _,
            machine: _,
            held_back: _,
            home: _,
            lock: _,
            settings: _,
            failed,
            status,
        } = self;
        failed.clear();
        *archive = None;
        *archive_asked = false;
        if !home {
            return;
        }
        *status = None;
    }

    pub fn trouble(&mut self, f: Fault) -> crate::action::Applied {
        self.faults.push(f.clone());
        crate::action::Applied::Trouble(f)
    }

    pub fn drain(&mut self) -> Vec<Outcome> {
        self.keep_inline = true;
        let mut got = self.drain_at(0.0);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while self.tasks.in_flight(crate::task::Kind::Keep) && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(1));
            got.extend(self.drain_at(0.0));
        }
        self.keep_inline = false;
        self.keep_now();
        got
    }

    pub fn reread_credential(&mut self) {
        use crate::keybox::Credential;
        match crate::keybox::credential_now(self.desk.pace_now()) {
            Ok(Credential::Live(st)) => {
                self.credential = Some(st.term);
                self.passes = st.rows;
            }
            Ok(Credential::Expired) => crate::action::cli_alone_ended(self, Known::CredentialExpired),
            Ok(Credential::Unformed) => crate::action::cli_alone_ended(self, Known::CredentialReclaimed),
            Ok(Credential::None) => {
                self.credential = None;
                self.passes.clear();
                if crate::keybox::take_credential_dropped() {
                    crate::action::alone_ended(self, Known::CliAloneDropped, true);
                } else if self.machine.cli_alone.is_some() {
                    crate::action::cli_alone_ended(self, Known::CredentialMissing);
                }
            }
            Err(f) => self.faults.push(f),
        }
    }

    pub fn cli_alone_now(&self) -> CliAloneNow {
        match (self.machine.cli_alone.is_some() && self.credential.is_some(), self.passes.len()) {
            (false, _) => CliAloneNow::Off,
            (true, 0) => CliAloneNow::Keyless,
            (true, n) => CliAloneNow::Keyed(n),
        }
    }

    pub fn credential_tick(&mut self) {
        if self.unlocked() && self.credential.is_some_and(|t| t.expired(self.desk.pace_now())) {
            crate::action::cli_alone_ended(self, Known::CredentialExpired);
        }
    }

    pub fn keep_now(&mut self) {
        if self.keeps.is_empty() {
            return;
        }
        let all = std::mem::take(&mut self.keeps);
        let out = keep(all);
        self.kept(&out);
    }

    pub fn drain_at(&mut self, now: f64) -> Vec<Outcome> {
        if let Some(f) = trace::take_full() {
            self.faults.push(f);
        }
        self.credential_tick();
        let mut got = self.tasks.drain_at(now);
        let unkept = eravon_chain::settled::unkept();
        if !unkept.is_empty() {
            self.faults.push(crate::fault::Fault::known(crate::fault::Known::ReadsUnkept, unkept.join("; ")));
        }
        for o in got.iter_mut() {
            if o.renoded {
                continue;
            }
            if crate::landing::goes_back(o.kind, crate::landing::Back::Vault) {
                let got = o.result.clone().map(|d| match d {
                    Done::Vault(v) => v,
                    _ => crate::task::Vault::Opened,
                });
                let said = crate::action::vault_landed(self, got);
                self.vault_said = Some(said);
                continue;
            }
            if crate::action::lands_said(o.kind) {
                if o.stale {
                    if let Err(f) = &o.result {
                        self.faults.push(f.clone());
                    }
                } else {
                    let said = crate::action::landed(self, o.kind, o.result.clone());
                    self.said.insert(o.kind, said);
                }
                continue;
            }
            if let Ok(Done::House(l)) = &o.result {
                self.house_landed(o.kind, (**l).clone(), o.stale, None);
                continue;
            }
            if crate::house::is_house(o.kind) {
                if let Err(f) = &o.result {
                    self.house_failed(o.kind, f);
                }
                continue;
            }
            if o.stale {
                if let Err(f) = &o.result {
                    self.faults.push(f.clone());
                }
                continue;
            }
            match &o.result {
                Ok(_) => {
                    self.failed.remove(&o.kind);
                }
                Err(f) => {
                    self.failed.insert(o.kind, f.clone());
                    if o.kind == crate::task::Kind::Backup {
                        self.leave(Keep::Machine);
                    }
                }
            }
            match &o.result {
                Ok(Done::Vault(_)) | Ok(Done::House(_)) => {}
                Ok(Done::Path(_)) => {}
                Ok(Done::Kept(all)) => self.kept(all),
                Ok(Done::CliPath(s)) => self.cli_path = Some(s.clone()),
                Ok(Done::Seated(_)) => {}
                Ok(Done::Check(r)) => self.last = Some(r.clone()),
                Ok(Done::Archive { bytes, records, machine_items }) => {
                    self.archive = Some(ArchiveRead { bytes: *bytes, records: *records });
                    match machine_items {
                        Ok(n) => self.items_now = Some(*n),
                        Err(f) => {
                            self.items_now = None;
                            if !self.faults.iter().any(|x| x.said() == f.said()) {
                                self.faults.push(f.clone());
                            }
                        }
                    }
                }
                Ok(Done::Keystore(crate::task::Keystore::BackedUp { id, path, .. })) => {
                    self.leave(Keep::BackedUp { id: id.clone(), path: path.clone() });
                }
                Ok(Done::BackupMade { path, summary }) => {
                    self.backup_made = Some((path.clone(), summary.clone()));
                    self.leave(Keep::Machine);
                }
                Ok(Done::BackupSeen { path, summary }) => self.backup_peek = Some((path.clone(), summary.clone())),
                Ok(Done::Door(_)) => {}
                Err(f) => self.faults.push(f.clone()),
            }
        }
        if self.lock_pending && !self.tasks.flying().into_iter().any(|k| k.writes_local()) {
            self.lock_pending = false;
            crate::keybox::lock();
            self.after_lock();
        }
        self.keep_turn();
        self.door_turn(&got);
        if !self.held_back.is_empty() {
            let mut all = std::mem::take(&mut self.held_back);
            all.extend(got);
            return all;
        }
        got
    }

    pub fn drain_hold(&mut self) {
        let got = self.drain_at(0.0);
        self.held_back = got;
    }

    pub fn idle_tick(&mut self, last: f64, now: f64) -> bool {
        if self.rekeying || self.lock_pending || !self.unlocked() || !crate::machine::idle_locks(&self.machine, last, now) {
            return false;
        }
        matches!(crate::action::apply(self, crate::action::Action::Lock), crate::action::Applied::LockedUp)
    }

    pub fn note_parts(&self) -> (&'static str, crate::lang::Key, usize, usize, usize) {
        (
            build_kind(),
            match self.lock.as_ref() {
                None => crate::lang::Key::NoteNoHome,
                Some(l) => {
                    if l.mode().writable() {
                        crate::lang::Key::NoteWriter
                    } else {
                        crate::lang::Key::NoteReader
                    }
                }
            },
            trace::dropped(),
            self.fonts.faces.len(),
            fonts::Role::ALL.len(),
        )
    }
}

fn primary_reading() -> Option<(String, crate::keybox::PrimaryKind)> {
    match crate::keybox::primary() {
        Ok(p) => p,
        Err(_) => crate::keybox::primary_kind().ok().flatten().map(|k| (String::new(), k)),
    }
}

#[derive(Clone, Debug)]
pub enum Keep {
    Machine,
    BackedUp { id: Option<String>, path: String },
}

#[derive(Clone, Debug)]
pub enum Kept {
    Machine(Result<crate::machine::Machine, Fault>, u64),
    Wrote(Result<(), Fault>),
    Register(Result<crate::identity::Registry, Fault>, u64),
}

pub fn keep(all: Vec<Keep>) -> Vec<Kept> {
    use crate::reads::{ticket, Copies};
    let mut out = Vec::new();
    for k in all {
        let mut one: Vec<Kept> = Vec::new();
        match k {
            Keep::Machine => {
                let at = ticket(Copies::Machine);
                one.push(Kept::Machine(crate::machine::read(), at));
            }
            Keep::BackedUp { id, path } => {
                if let Some(id) = id {
                    if let Err(f) = crate::register::change(|reg| crate::identity::mark(reg, &id, false, true, Some(&path))) {
                        one.push(Kept::Wrote(Err(f)));
                    }
                }
                let at = ticket(Copies::Register);
                one.push(Kept::Register(crate::register::view(), at));
            }
        }
        if !one.is_empty() && !crate::task::land_part(Done::Kept(one.clone())) {
            out.extend(one);
        }
    }
    out
}
