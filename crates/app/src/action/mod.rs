use crate::feature::Feature;
use crate::key::Address;
use crate::keystore::Params;
use crate::shell::{Page, Shell};
use crate::task::{Done, Kind, Reaped, Spawned};
use crate::trace;

#[derive(Clone, PartialEq, Eq)]
pub enum ImportForm {
    Words(crate::secret::Secret),
    PrivateKey { key: crate::secret::Secret, keyfile: Option<KeyFileOut> },
    Keystore { path: String, password: crate::secret::Secret },
}

#[derive(Clone, PartialEq, Eq)]
pub struct KeyFileOut {
    pub password: crate::secret::Secret,
    pub again: crate::secret::Secret,
    pub dir: String,
}

impl std::fmt::Debug for ImportForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportForm::Words(_) => f.write_str("Words(…)"),
            ImportForm::PrivateKey { keyfile, .. } => write!(f, "PrivateKey {{ key: …, keyfile: {} }}", if keyfile.is_some() { "…" } else { "none" }),
            ImportForm::Keystore { path, .. } => write!(f, "Keystore {{ path: {path:?}, password: … }}"),
        }
    }
}

macro_rules! closed_actions {
    ($(#[$em:meta])* pub enum $closed:ident { $( $(#[$m:meta])* $name:ident $( ( $($tup:tt)* ) )? $( { $($body:tt)* } )? ),* $(,)? }) => {
        $(#[$em])*
        pub enum $closed { $( $(#[$m])* $name $( ( $($tup)* ) )? $( { $($body)* } )? ),* }

        impl $closed {
            pub const NAMES: &'static [&'static str] = &[$(stringify!($name)),*];

            pub fn name(&self) -> &'static str {
                match self {
                    $( $closed::$name { .. } => stringify!($name), )*
                }
            }
        }
    };
}

closed_actions! {
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Action {
    Show(Page),
    SelfCheck,
    Measure,
    Quit,
    SwitchRole,
    ReadIdentities,
    NewIdentity,
    ConfirmIdentity { answers: Vec<(usize, crate::secret::Secret)>, label: String },
    DropFresh,
    ImportIdentity { form: ImportForm, label: String },
    SwitchIdentity { id: String },
    DeleteIdentity { id: String, pin: crate::secret::Secret },
    NameIdentity { id: String, label: String },
    SetPin { pin: crate::secret::Secret, again: crate::secret::Secret },
    Unlock { pin: crate::secret::Secret },
    Reseal { pin: crate::secret::Secret },
    ResetEmptyKeybox,
    SetMachineAside,
    Lock,
    ChangePin { old: crate::secret::Secret, pin: crate::secret::Secret, again: crate::secret::Secret },
    RecoverWords { words: crate::secret::Secret, pin: crate::secret::Secret, again: crate::secret::Secret },
    RecoverKeystore { path: String, password: crate::secret::Secret, pin: crate::secret::Secret, again: crate::secret::Secret },
    RevealWords { pin: crate::secret::Secret },
    HideWords,
    BackupKey { pin: crate::secret::Secret, password: crate::secret::Secret, again: crate::secret::Secret, dir: String },
    SetAutoLock { on: bool, secs: u64 },
    SetPrimary { id: String, pin: crate::secret::Secret },
    ExportBackup { pin: crate::secret::Secret, password: crate::secret::Secret, again: crate::secret::Secret, dir: String },
    PeekBackup { path: String, password: crate::secret::Secret },
    RestoreBackup { path: String, password: crate::secret::Secret, how: RestoreHow },
    OpenHome { root: String },
    ChangeHome { root: String },
    SetEndpoints { specs: String },
    SetLogAsks { n: u64 },
    SetLang { lang: crate::lang::Lang },
    SetZone { zone: crate::when::Zone },
    SetAppearance { appearance: String },
    TakeWriter,
    ReadCliPath,
    SetCliPath { on: bool },
    SetCliAnchor { to: crate::machine::CliAnchor },
    SetRefreshEvery { typed: String },
    SetCliDesktop { on: bool },
    SetCliAlone { lasts: crate::machine::Lasts, pin: crate::secret::Secret },
    CliAloneOff,
    FaceConsent { identity: String, role: String, pin: crate::secret::Secret },
    CliPassAdd { note: String, pin: crate::secret::Secret },
    CliPassDrop { tag: String },
    SetProxy { choice: String },
    SetRestMode { on: bool },
    AddRest { base: String, key: crate::secret::Secret },
    RemoveRest { base: String },
    House(crate::house::act::Act),
}

}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreHow {
    FirstRun,
    Settings { pin: crate::secret::Secret },
    Locked { pin: crate::secret::Secret, again: crate::secret::Secret },
}

impl Action {
    pub fn forget_pin(&mut self) {
        match self {
            Action::BackupKey { pin, .. }
            | Action::DeleteIdentity { pin, .. }
            | Action::RevealWords { pin }
            | Action::ExportBackup { pin, .. }
            | Action::SetCliAlone { pin, .. }
            | Action::FaceConsent { pin, .. }
            | Action::CliPassAdd { pin, .. } => pin.clear(),
            Action::SetPin { pin, again } | Action::RecoverWords { pin, again, .. } | Action::RecoverKeystore { pin, again, .. } => {
                pin.clear();
                again.clear();
            }
            Action::Unlock { pin } | Action::Reseal { pin } | Action::SetPrimary { pin, .. } | Action::RestoreBackup { how: RestoreHow::Settings { pin }, .. } => pin.clear(),
            Action::RestoreBackup { how: RestoreHow::Locked { pin, again }, .. } => {
                pin.clear();
                again.clear();
            }
            Action::ChangePin { old, pin, again } => {
                old.clear();
                pin.clear();
                again.clear();
            }
            _ => {}
        }
    }

    pub fn gate_pin(&self) -> Option<&crate::secret::Secret> {
        match self {
            Action::SetPin { pin, .. }
            | Action::Unlock { pin }
            | Action::Reseal { pin }
            | Action::RecoverWords { pin, .. }
            | Action::RecoverKeystore { pin, .. }
            | Action::SetPrimary { pin, .. }
            | Action::RestoreBackup { how: RestoreHow::Settings { pin } | RestoreHow::Locked { pin, .. }, .. } => Some(pin),
            Action::ChangePin { old, .. } => Some(old),
            _ => None,
        }
    }

    pub fn hand_pin(&mut self) -> Option<crate::secret::Secret> {
        let cell = match self {
            Action::BackupKey { pin, .. }
            | Action::DeleteIdentity { pin, .. }
            | Action::RevealWords { pin }
            | Action::ExportBackup { pin, .. }
            | Action::SetCliAlone { pin, .. }
            | Action::FaceConsent { pin, .. }
            | Action::CliPassAdd { pin, .. }
            | Action::SetPin { pin, .. }
            | Action::Unlock { pin }
            | Action::Reseal { pin }
            | Action::RecoverWords { pin, .. }
            | Action::RecoverKeystore { pin, .. }
            | Action::SetPrimary { pin, .. }
            | Action::RestoreBackup { how: RestoreHow::Settings { pin } | RestoreHow::Locked { pin, .. }, .. } => pin,
            Action::ChangePin { old, .. } => old,
            _ => return None,
        };
        if cell.is_empty() {
            return None;
        }
        Some(std::mem::take(cell))
    }

    pub fn pin_asked(&self) -> Option<&crate::secret::Secret> {
        match self {
            Action::BackupKey { pin, .. } | Action::RevealWords { pin } | Action::ExportBackup { pin, .. } | Action::SetCliAlone { pin, .. } | Action::FaceConsent { pin, .. } | Action::CliPassAdd { pin, .. } => Some(pin),
            Action::DeleteIdentity { pin, .. } => delete_gate(pin),
            Action::SetPrimary { .. } | Action::RestoreBackup { .. } => None,
            Action::SetPin { .. }
            | Action::Unlock { .. }
            | Action::Reseal { .. }
            | Action::ChangePin { .. }
            | Action::RecoverWords { .. }
            | Action::RecoverKeystore { .. } => None,
            _ => None,
        }
    }

    pub fn needs_key(&self) -> bool {
        match self {
            Action::SetPrimary { .. }
            | Action::ExportBackup { .. }
            | Action::RestoreBackup { how: RestoreHow::FirstRun | RestoreHow::Settings { .. }, .. }
            | Action::ConfirmIdentity { .. }
            | Action::ImportIdentity { .. }
            | Action::DeleteIdentity { .. }
            | Action::BackupKey { .. }
            | Action::SetCliDesktop { .. }
            | Action::SetCliAlone { .. }
            | Action::CliAloneOff
            | Action::CliPassAdd { .. }
            | Action::CliPassDrop { .. }
            | Action::FaceConsent { .. }
            | Action::AddRest { .. } => true,
            Action::RevealWords { .. } => false,
            Action::NameIdentity { .. } => false,
            Action::Show(_)
            | Action::SelfCheck
            | Action::Quit
            | Action::Measure
            | Action::ReadIdentities
            | Action::NewIdentity
            | Action::DropFresh
            | Action::SwitchRole
            | Action::SwitchIdentity { .. }
            | Action::SetPin { .. }
            | Action::Unlock { .. }
            | Action::Reseal { .. }
            | Action::ResetEmptyKeybox
            | Action::SetMachineAside
            | Action::Lock
            | Action::SetAutoLock { .. }
            | Action::RestoreBackup { how: RestoreHow::Locked { .. }, .. }
            | Action::PeekBackup { .. }
            | Action::ChangePin { .. }
            | Action::RecoverWords { .. }
            | Action::RecoverKeystore { .. }
            | Action::HideWords
            | Action::OpenHome { .. }
            | Action::ChangeHome { .. }
            | Action::TakeWriter
            | Action::SetLang { .. }
            | Action::SetZone { .. }
            | Action::SetAppearance { .. }
            | Action::SetEndpoints { .. }
            | Action::SetLogAsks { .. }
            | Action::SetRestMode { .. }
            | Action::RemoveRest { .. }
            | Action::SetProxy { .. }
            | Action::SetCliAnchor { .. }
            | Action::SetRefreshEvery { .. }
            | Action::ReadCliPath
            | Action::SetCliPath { .. }
            | Action::House(_) => false,
        }
    }

    pub fn feature(&self) -> Feature {
        match self {
            Action::Show(_) | Action::SelfCheck | Action::Quit => Feature::H1,
            Action::SwitchRole
            | Action::ReadIdentities
            | Action::NewIdentity
            | Action::ConfirmIdentity { .. }
            | Action::DropFresh
            | Action::ImportIdentity { .. }
            | Action::NameIdentity { .. }
            | Action::SwitchIdentity { .. }
            | Action::DeleteIdentity { .. }
            | Action::SetPin { .. }
            | Action::Unlock { .. }
            | Action::Reseal { .. }
            | Action::ResetEmptyKeybox
            | Action::SetMachineAside
            | Action::Lock
            | Action::SetAutoLock { .. }
            | Action::ChangePin { .. }
            | Action::RecoverWords { .. }
            | Action::RecoverKeystore { .. }
            | Action::RevealWords { .. }
            | Action::HideWords
            | Action::SetPrimary { .. }
            | Action::BackupKey { .. }
            | Action::FaceConsent { .. } => Feature::H2,
            Action::ExportBackup { .. } | Action::PeekBackup { .. } | Action::RestoreBackup { .. } => Feature::H2,
            Action::OpenHome { .. } | Action::ChangeHome { .. } | Action::TakeWriter => Feature::H3,
            Action::Measure => Feature::H3,
            Action::SetEndpoints { .. } | Action::SetLogAsks { .. } | Action::SetRestMode { .. } | Action::AddRest { .. } | Action::RemoveRest { .. } | Action::SetProxy { .. } => Feature::H2,
            Action::SetRefreshEvery { .. } => Feature::S9,
            Action::SetLang { .. } | Action::SetZone { .. } => Feature::H5,
            Action::ReadCliPath | Action::SetCliPath { .. } | Action::SetCliDesktop { .. } | Action::SetCliAlone { .. } | Action::CliAloneOff | Action::CliPassAdd { .. } | Action::CliPassDrop { .. } => Feature::H5,
            Action::SetCliAnchor { .. } => Feature::S4,
            Action::SetAppearance { .. } => Feature::H0,
            Action::House(_) => Feature::H1,
        }
    }

    pub fn reads_only(&self) -> bool {
        matches!(
            self,
            Action::Show(_)
                | Action::Quit
                | Action::SelfCheck
                | Action::ReadIdentities
                | Action::Measure
        )
    }

}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Applied {
    Shown(Page),
    Started(Kind),
    Refused(Kind),
    Stopped(Reaped),
    Seated(crate::roles::Role),
    Identities(usize),
    FreshWords,
    FreshDropped,
    IdentityMade { id: String, seats: Vec<(crate::roles::Role, Address)>, restored: bool },
    IdentityNamed { id: String, label: String },
    IdentitySwitched { id: String },
    IdentityKept { id: String },
    IdentityDeleted { id: String },
    Unlocked,
    LockedUp,
    KeyboxReset,
    MachineAside { from: String, to: String },
    PinChanged,
    AutoLockSet { on: bool, secs: u64 },
    PrimarySet { id: String },
    BackupRestored(crate::backup::Summary),
    Recovered,
    WordsShown,
    WordsHidden,
    KeyBackedUp { path: String, address: Address },
    Homed { root: String, mode: crate::lock::Mode },
    Endpoints(usize),
    LogAsks(u64),
    RestMode(bool),
    RestAdded { base: String },
    RestKeyed { base: String },
    RestRemoved { left: usize },
    Spoken(crate::lang::Lang),
    Zoned(crate::when::Zone),
    Appeared(String),
    ProxySet(String),
    RefreshEvery(u64),
    CliAnchorSet(crate::machine::CliAnchor),
    CliDesktopSet(bool),
    CliAloneSet(Option<Option<u64>>),
    CliPassAdded { pass: crate::secret::Secret, row: eravon_seal::credential::Row },
    CliPassDropped { tag: String },
    CliPathRead(eravon_os::cli_path::State),
    Trouble(crate::fault::Fault),
    Consented { text: String },
    House(crate::house::act::Said),
}

fn after_vault_shut(shell: &mut Shell) {
    shell.reread_vault();
    if !shell.unlocked() {
        shell.after_lock();
    }
}

pub fn held_back(shell: &Shell, a: &Action) -> Option<crate::fault::Known> {
    if shell.rekeying && !matches!(a, Action::Show(_) | Action::Quit | Action::SelfCheck) {
        return Some(crate::fault::Known::Rekeying);
    }
    if a.needs_key() && !shell.unlocked() {
        return Some(crate::fault::Known::Locked);
    }
    None
}

pub fn apply(shell: &mut Shell, a: Action) -> Applied {
    trace::mark(a.feature());
    if let Some(k) = held_back(shell, &a) {
        return shell.trouble(crate::fault::Fault::known(k, String::new()));
    }
    if a.pin_asked().is_some() {
        let mut rest = a;
        let pin = rest.hand_pin().unwrap_or_default();
        rest.forget_pin();
        return vault(shell, move || {
            crate::keybox::check_pin(pin.expose())?;
            Ok(crate::task::Vault::Gate(Box::new(rest)))
        });
    }
    body(shell, a)
}

fn delete_gate(pin: &crate::secret::Secret) -> Option<&crate::secret::Secret> {
    Some(pin)
}


fn vault(shell: &mut Shell, work: impl FnOnce() -> Result<crate::task::Vault, crate::fault::Fault> + Send + 'static) -> Applied {
    match shell.tasks.spawn(Kind::Vault, move || work().map(Done::Vault)) {
        Spawned::Started => Applied::Started(Kind::Vault),
        Spawned::InFlight => Applied::Refused(Kind::Vault),
    }
}


pub fn vault_landed(shell: &mut Shell, got: Result<crate::task::Vault, crate::fault::Fault>) -> Applied {
    use crate::task::Vault;
    match got {
        Ok(Vault::Opened) => {
            shell.after_unlock();
            if let Some(f) = crate::keybox::take_reseal_trouble() {
                return shell.trouble(f);
            }
            Applied::Unlocked
        }
        Ok(Vault::Changed) => {
            shell.after_unlock();
            Applied::PinChanged
        }
        Ok(Vault::Recovered) => {
            shell.after_unlock();
            Applied::Recovered
        }
        Ok(Vault::Gate(a)) => {
            shell.reread_vault();
            if shell.lock_pending {
                return shell.trouble(crate::fault::Fault::known(crate::fault::Known::Locked, String::new()));
            }
            if let Some(k) = held_back(shell, &a) {
                return shell.trouble(crate::fault::Fault::known(k, String::new()));
            }
            body(shell, *a)
        }
        Ok(Vault::PrimarySet { id }) => {
            shell.rekeying = false;
            shell.unload();
            shell.after_unlock();
            Applied::PrimarySet { id }
        }
        Ok(Vault::Restored { summary }) => {
            shell.rekeying = false;
            shell.unload();
            shell.after_unlock();
            Applied::BackupRestored(summary)
        }
        Ok(Vault::Identity { row, restored, fresh, was }) => identity_landed(shell, row, restored, fresh, was),
        Err(f) => {
            shell.rekeying = false;
            shell.new_label = None;
            after_vault_shut(shell);
            shell.trouble(f)
        }
    }
}

fn busy_for_rekey(shell: &Shell) -> Option<crate::fault::Fault> {
    let others: Vec<Kind> = shell.tasks.flying().into_iter().filter(|k| k.writes_local()).collect();
    (!others.is_empty()).then(|| {
        crate::fault::Fault::known(crate::fault::Known::BusyForRekey, others.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(" "))
    })
}

pub fn apply_settled(shell: &mut Shell, a: Action) -> Applied {
    let first = apply(shell, a);
    if let Applied::Started(k) = first {
        if lands_said(k) {
            return said_settled(shell, k);
        }
    }
    if first != Applied::Started(Kind::Vault) {
        return first;
    }
    loop {
        if let Some(done) = shell.vault_said.take() {
            return done;
        }
        let finished = shell.tasks.finished_in_flight(Kind::Vault);
        shell.drain_hold();
        if shell.vault_said.is_none() {
            if finished && shell.tasks.in_flight(Kind::Vault) {
                return shell.trouble(crate::fault::Fault::known(crate::fault::Known::OutcomeLost, Kind::Vault.as_str().to_string()));
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}

pub fn spawn_for(shell: &mut Shell, kind: Kind, work: impl FnOnce() -> Result<Done, crate::fault::Fault> + Send + 'static) -> crate::task::Spawned {
    shell.tasks.spawn(kind, work)
}

pub fn wait_path(shell: &mut Shell, wait: crate::platform::Wait) -> crate::task::Spawned {
    shell.tasks.spawn(Kind::Path, move || wait().map(Done::Path))
}

pub fn lands_said(k: Kind) -> bool {
    crate::landing::goes_back(k, crate::landing::Back::Said)
}

pub fn landed(shell: &mut Shell, k: Kind, got: Result<Done, crate::fault::Fault>) -> Applied {
    let lost = |shell: &mut Shell| shell.trouble(crate::fault::Fault::known(crate::fault::Known::OutcomeLost, k.as_str().to_string()));
    match (k, got) {
        (Kind::Seat, got) => seat_landed(shell, got),
        (_, Err(f)) => shell.trouble(f),
        (_, Ok(_)) => lost(shell),
    }
}

fn said_settled(shell: &mut Shell, k: Kind) -> Applied {
    loop {
        if let Some(done) = shell.said.remove(&k) {
            if done != Applied::Started(k) {
                return done;
            }
        }
        let finished = shell.tasks.finished_in_flight(k);
        shell.drain_hold();
        if !shell.said.contains_key(&k) {
            if (finished && shell.tasks.in_flight(k)) || !shell.tasks.in_flight(k) {
                return shell.trouble(crate::fault::Fault::known(crate::fault::Known::OutcomeLost, k.as_str().to_string()));
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}

pub fn behind(shell: &mut Shell, kind: Kind, work: impl FnOnce() -> Result<Done, crate::fault::Fault> + Send + 'static) -> Spawned {
    shell.tasks.spawn(kind, work)
}

fn body(shell: &mut Shell, a: Action) -> Applied {
    match a {
        Action::Show(p) => {
            shell.page = p;
            Applied::Shown(p)
        }
        Action::SelfCheck => {
            match shell.tasks.spawn(Kind::SelfCheck, || crate::probe::run().map(Done::Check)) {
                Spawned::Started => Applied::Started(Kind::SelfCheck),
                Spawned::InFlight => Applied::Refused(Kind::SelfCheck),
            }
        }
        Action::Measure => {
            let Some(root) = shell.home.as_ref().map(|h| h.root().to_path_buf()) else {
                return shell.trouble(crate::fault::Fault::known(
                    crate::fault::Known::NoHome,
                    crate::lang::both_of(|| crate::lang::t(crate::lang::Key::Tail001).to_string()),
                ));
            };
            match shell.tasks.spawn(Kind::Archive, move || crate::house::measure(&root)) {
                Spawned::Started => Applied::Started(Kind::Archive),
                Spawned::InFlight => Applied::Refused(Kind::Archive),
            }
        }
        Action::Quit => {
            for w in shell.words.iter_mut().flatten() {
                w.clear();
            }
            shell.words = None;
            shell.new_words = None;
            shell.door_off = true;
            shell.door_sync();
            Applied::Stopped(shell.tasks.shutdown())
        }

        Action::SwitchRole => match switch_role(shell) {
            Ok(a) => a,
            Err(f) => shell.trouble(f),
        },
        Action::ReadIdentities => match crate::register::view() {
            Ok(r) => {
                let n = r.rows.len();
                shell.seat_identities(Some(r));
                Applied::Identities(n)
            }
            Err(f) => shell.trouble(f),
        },
        Action::NewIdentity => match crate::identity::fresh() {
            Ok(f) => {
                shell.new_words = Some(f);
                Applied::FreshWords
            }
            Err(f) => shell.trouble(f),
        },
        Action::ConfirmIdentity { answers, label } => match confirm_identity(shell, &answers, &crate::house::forms::label_kept(&label)) {
            Ok(started) => started,
            Err(f) => shell.trouble(f),
        },
        Action::DropFresh => {
            if let Some(kept) = crate::house::forms::fresh_kept(shell) {
                return kept;
            }
            shell.new_words = None;
            Applied::FreshDropped
        }
        Action::ImportIdentity { form, label } => match import_identity(shell, form, &crate::house::forms::label_kept(&label)) {
            Ok(started) => started,
            Err(f) => shell.trouble(f),
        },
        Action::SwitchIdentity { id } => switch_identity(shell, &id),
        Action::NameIdentity { id, label } => match name_identity(shell, &id, &label) {
            Ok(row) => Applied::IdentityNamed { id: row.id, label: row.label },
            Err(f) => shell.trouble(f),
        },
        Action::DeleteIdentity { id, pin: _ } => match delete_identity(shell, &id) {
            Ok(id) => {
                shell.seat_follow();
                Applied::IdentityDeleted { id }
            }
            Err(f) => shell.trouble(f),
        },
        Action::SetPin { pin, again } => {
            if let Err(f) = same_twice(&pin, &again) {
                return shell.trouble(f);
            }
            vault(shell, move || {
                if crate::keybox::pin_trouble(pin.expose()).is_none()
                    && matches!(crate::keybox::state()?, crate::keybox::State::Absent)
                {
                    let (n, at) = crate::local::set_aside_sealed()?;
                    if n > 0 {
                        crate::local::note_trouble(crate::fault::Fault::known(
                            crate::fault::Known::LocalSetAside,
                            format!("{n} · {}", at.map(|p| p.display().to_string()).unwrap_or_default()),
                        ));
                    }
                }
                crate::keybox::set_pin(pin.expose()).map(|_| crate::task::Vault::Opened)
            })
        }
        Action::Unlock { pin } => vault(shell, move || {
            crate::keybox::unlock(pin.expose())?;
            crate::local::after_open();
            Ok(crate::task::Vault::Opened)
        }),
        Action::Reseal { pin } => vault(shell, move || {
            crate::keybox::reseal(pin.expose())?;
            crate::local::after_open();
            Ok(crate::task::Vault::Opened)
        }),
        Action::ResetEmptyKeybox => match crate::keybox::reset_empty() {
            Ok(()) => {
                shell.after_lock();
                Applied::KeyboxReset
            }
            Err(f) => shell.trouble(f),
        },
        Action::SetMachineAside => set_machine_aside(shell),
        Action::Lock => {
            if shell.tasks.flying().into_iter().any(|k| k.writes_local()) {
                shell.begin_lock();
            } else {
                crate::keybox::lock();
                shell.after_lock();
            }
            Applied::LockedUp
        }
        Action::ChangePin { old, pin, again } => {
            if let Err(f) = same_twice(&pin, &again) {
                return shell.trouble(f);
            }
            vault(shell, move || crate::keybox::change_pin(old.expose(), pin.expose()).map(|_| crate::task::Vault::Changed))
        }
        Action::RecoverWords { words, pin, again } => {
            if let Err(f) = same_twice(&pin, &again) {
                return shell.trouble(f);
            }
            let fresh = match crate::identity::from_words(words.expose()) {
                Ok(f) => f,
                Err(f) => return shell.trouble(f),
            };
            vault(shell, move || {
                crate::identity::recover_with(&fresh, pin.expose())?;
                crate::local::after_open();
                Ok(crate::task::Vault::Recovered)
            })
        }
        Action::RecoverKeystore { path, password, pin, again } => {
            if let Err(f) = same_twice(&pin, &again) {
                return shell.trouble(f);
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => return shell.trouble(crate::fault::classify(&e, &path)),
            };
            vault(shell, move || {
                let secret = crate::keystore::decrypt_typed(&bytes, &password)?;
                crate::key::recover_with(&secret, pin.expose())?;
                crate::local::after_open();
                Ok(crate::task::Vault::Recovered)
            })
        }
        Action::RevealWords { pin: _ } => match reveal_words(shell) {
            Ok(()) => Applied::WordsShown,
            Err(f) => {
                after_vault_shut(shell);
                shell.trouble(f)
            }
        },
        Action::HideWords => {
            for w in shell.words.iter_mut().flatten() {
                w.clear();
            }
            shell.words = None;
            Applied::WordsHidden
        }
        Action::SetAutoLock { on, secs } => match set_auto_lock(shell, on, secs) {
            Ok((on, secs)) => Applied::AutoLockSet { on, secs },
            Err(f) => shell.trouble(f),
        },
        Action::SetPrimary { id, pin } => {
            if let Some(f) = busy_for_rekey(shell) {
                return shell.trouble(f);
            }
            let started = vault(shell, move || {
                crate::keybox::unlock(pin.expose())?;
                crate::rekey::set_primary(&id, pin.expose())?;
                Ok(crate::task::Vault::PrimarySet { id })
            });
            shell.rekeying = started == Applied::Started(Kind::Vault);
            started
        }
        Action::ExportBackup { pin: _, password, again, dir } => match export_backup(shell, password, &again, &dir) {
            Ok(Spawned::Started) => Applied::Started(Kind::Backup),
            Ok(Spawned::InFlight) => Applied::Refused(Kind::Backup),
            Err(f) => shell.trouble(f),
        },
        Action::PeekBackup { path, password } => {
            match shell.tasks.spawn(Kind::Backup, move || crate::backup::peek(std::path::Path::new(&path), password.expose()).map(|s| Done::BackupSeen { path, summary: s })) {
                Spawned::Started => Applied::Started(Kind::Backup),
                Spawned::InFlight => Applied::Refused(Kind::Backup),
            }
        }
        Action::RestoreBackup { path, password, how } => {
            if let RestoreHow::Locked { pin, again } = &how {
                if let Err(f) = same_twice(pin, again) {
                    return shell.trouble(f);
                }
                if let Some(t) = crate::keybox::pin_trouble(pin.expose()) {
                    return shell.trouble(crate::fault::Fault::known(crate::fault::Known::PinShape, t.as_str().to_string()));
                }
            }
            if let Some(f) = busy_for_rekey(shell) {
                return shell.trouble(f);
            }
            let started = vault(shell, move || {
                let at = std::path::Path::new(&path);
                let done = match &how {
                    RestoreHow::FirstRun => crate::backup::restore(at, password.expose(), crate::backup::From::FirstRun)?,
                    RestoreHow::Settings { pin } => {
                        crate::keybox::unlock(pin.expose())?;
                        crate::backup::restore(at, password.expose(), crate::backup::From::Settings(pin.expose()))?
                    }
                    RestoreHow::Locked { pin, .. } => crate::backup::restore(at, password.expose(), crate::backup::From::Locked(pin.expose()))?,
                };
                Ok(crate::task::Vault::Restored { summary: done.summary })
            });
            shell.rekeying = started == Applied::Started(Kind::Vault);
            started
        }
        Action::BackupKey { pin: _, password, again, dir } => match backup_key(shell, password, &again, &dir) {
            Ok(Spawned::Started) => Applied::Started(Kind::Keystore),
            Ok(Spawned::InFlight) => Applied::Refused(Kind::Keystore),
            Err(f) => shell.trouble(f),
        },
        Action::OpenHome { root } => match open_home(shell, &root) {
            Ok(mode) => Applied::Homed { root, mode },
            Err(f) => shell.trouble(f),
        },
        Action::ChangeHome { root } => match change_home(shell, &root) {
            Ok(mode) => Applied::Homed { root, mode },
            Err(f) => shell.trouble(f),
        },
        Action::TakeWriter => match take_writer(shell) {
            Ok((root, mode)) => Applied::Homed { root, mode },
            Err(f) => shell.trouble(f),
        },
        Action::SetEndpoints { specs } => match set_endpoints(shell, &specs) {
            Ok(n) => Applied::Endpoints(n),
            Err(f) => shell.trouble(f),
        },
        Action::SetLogAsks { n } => match set_log_asks(shell, n) {
            Ok(n) => Applied::LogAsks(n),
            Err(f) => shell.trouble(f),
        },
        Action::SetRestMode { on } => match set_rest_mode(shell, on) {
            Ok(on) => Applied::RestMode(on),
            Err(f) => shell.trouble(f),
        },
        Action::AddRest { base, key } => match add_rest(shell, &base, &key) {
            Ok(rest::Added::Row(base)) => Applied::RestAdded { base },
            Ok(rest::Added::Keyed(base)) => Applied::RestKeyed { base },
            Err(f) => shell.trouble(f),
        },
        Action::RemoveRest { base } => match remove_rest(shell, &base) {
            Ok(left) => Applied::RestRemoved { left },
            Err(f) => shell.trouble(f),
        },

        Action::SetLang { lang } => match set_lang(shell, lang) {
            Ok(l) => Applied::Spoken(l),
            Err(f) => shell.trouble(f),
        },
        Action::SetZone { zone } => match set_zone(shell, zone) {
            Ok(z) => {
                eravon_orders::when::set(crate::house::settings::zone_of(z.as_str()));
                Applied::Zoned(z)
            }
            Err(f) => shell.trouble(f),
        },
        Action::SetAppearance { appearance } => match set_appearance(shell, &appearance) {
            Ok(a) => Applied::Appeared(a),
            Err(f) => shell.trouble(f),
        },
        Action::SetProxy { choice } => match set_proxy(shell, &choice) {
            Ok(c) => Applied::ProxySet(c),
            Err(f) => shell.trouble(f),
        },
        Action::ReadCliPath => {
            let s = cli_path_now();
            shell.cli_path = Some(s.clone());
            shell.ai_config = cli_beside().filter(|_| !matches!(s, eravon_os::cli_path::State::Unsupported(_))).map(|cli| ai_config(&cli));
            Applied::CliPathRead(s)
        }
        Action::SetCliPath { on } => match crate::action::cli_beside() {
            Some(cli) => match shell.tasks.spawn(Kind::CliPath, move || cli_path_set(&cli, on)) {
                Spawned::Started => Applied::Started(Kind::CliPath),
                Spawned::InFlight => Applied::Refused(Kind::CliPath),
            },
            None => shell.trouble(crate::fault::Fault::known(crate::fault::Known::CliPathUnsupported, String::new())),
        },
        Action::SetCliDesktop { on } => {
            if shell.machine.cli_path() == crate::machine::CliPath::Alone {
                return shell.trouble(crate::fault::Fault::known(crate::fault::Known::CliAloneOn, String::new()));
            }
            match crate::machine::update(|m| m.cli_desktop = on) {
                Ok(m) => {
                    shell.machine = m;
                    Applied::CliDesktopSet(on)
                }
                Err(f) => shell.trouble(f),
            }
        }
        Action::SetCliAlone { lasts, .. } => match cli_alone_on(shell, lasts) {
            Ok(expires) => Applied::CliAloneSet(Some(expires)),
            Err(f) => shell.trouble(f),
        },
        Action::CliAloneOff => match cli_alone_off(shell) {
            Ok(()) => Applied::CliAloneSet(None),
            Err(f) => shell.trouble(f),
        },
        Action::FaceConsent { identity, role, .. } => match face::face_consent(shell, &identity, &role) {
            Ok(text) => Applied::Consented { text },
            Err(f) => shell.trouble(f),
        },
        Action::CliPassAdd { note, .. } => match cli_pass_add(shell, &note) {
            Ok((pass, row)) => Applied::CliPassAdded { pass, row },
            Err(f) => shell.trouble(f),
        },
        Action::CliPassDrop { tag } => match cli_pass_drop(shell, &tag) {
            Ok(()) => Applied::CliPassDropped { tag },
            Err(f) => shell.trouble(f),
        },
        Action::SetCliAnchor { to } => {
            if !eravon_cli::home::anchor_live(&shell.machine) {
                return shell.trouble(crate::fault::Fault::known(crate::fault::Known::CliAnchorGrey, String::new()));
            }
            match crate::machine::update(|m| m.cli_anchor = to) {
                Ok(m) => {
                    shell.machine = m;
                    Applied::CliAnchorSet(to)
                }
                Err(f) => shell.trouble(f),
            }
        }
        Action::SetRefreshEvery { typed } => match crate::house::general::set(&typed) {
            Ok(n) => {
                shell.refresh_every = n;
                shell.desk.sh.refresh_every = n;
                Applied::RefreshEvery(n)
            }
            Err(f) => shell.trouble(f),
        },
        Action::House(a) => Applied::House(crate::house::act::apply(shell, a)),
    }
}



static LANDED_TAMPER: std::sync::OnceLock<fn(&std::path::Path)> = std::sync::OnceLock::new();

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Next {
    Send,
    Wait,
    Held,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Enqueued {
    pub queued: usize,
    pub next: Next,
}



mod archive;
mod face;
mod rest;
mod home;
mod identity;
pub use self::identity::seats_with_entries;
mod keys;
mod office;

fn set_zone(shell: &mut Shell, zone: crate::when::Zone) -> Result<crate::when::Zone, crate::fault::Fault> {
    shell.may_save_settings()?;
    shell.commit_settings(|s| s.zone = Some(zone))?;
    crate::when::set(zone);
    Ok(zone)
}


fn set_lang(shell: &mut Shell, lang: crate::lang::Lang) -> Result<crate::lang::Lang, crate::fault::Fault> {
    if shell.home.is_some() {
        shell.may_save_settings()?;
    }
    shell.machine = crate::machine::update(|m| m.lang = Some(lang))?;
    if shell.home.is_some() {
        shell.commit_settings(|s| s.lang = Some(lang))?;
    }
    crate::lang::set(lang);
    Ok(lang)
}

pub use self::home::*;
pub use self::identity::*;
pub use self::keys::switch_to;
use self::archive::*;
use self::rest::*;
pub use self::rest::add_refused;
use self::office::*;
pub use self::office::{follow_register, settle, Back, Followed, Seating};

pub fn cli_beside() -> Option<std::path::PathBuf> {
    eravon_os::cli_path::beside_this_program()
}

pub fn ai_config(cli: &std::path::Path) -> String {
    use eravon::json::Value;
    let o = |k: &str, v: Value| Value::Obj(vec![(k.to_string(), v)]);
    let server = Value::Obj(vec![("command".to_string(), Value::Str(cli.display().to_string())), ("args".to_string(), Value::Arr(vec![Value::Str("mcp".to_string())]))]);
    String::from_utf8_lossy(&eravon::json::canon_bytes(&o("mcpServers", o("eravon", server)))).into_owned()
}

pub fn ai_config_with_pass(config: &str, pass: &str) -> Option<String> {
    use eravon::json::Value;
    let mut v = eravon::json::parse(config.as_bytes()).ok()?;
    let Value::Obj(top) = &mut v else { return None };
    let Value::Obj(servers) = &mut top.iter_mut().find(|(k, _)| k == "mcpServers")?.1 else { return None };
    let Value::Obj(server) = &mut servers.iter_mut().find(|(k, _)| k == "eravon")?.1 else { return None };
    server.retain(|(k, _)| k != "env");
    server.push(("env".to_string(), Value::Obj(vec![(eravon_seal::credential::PASS_ENV.to_string(), Value::Str(pass.to_string()))])));
    Some(String::from_utf8_lossy(&eravon::json::canon_bytes(&v)).into_owned())
}

fn cli_path_now() -> eravon_os::cli_path::State {
    match cli_beside() {
        Some(cli) => eravon_os::cli_path::state(&cli),
        None => eravon_os::cli_path::State::Unsupported(String::new()),
    }
}

fn cli_alone_on(shell: &mut Shell, lasts: crate::machine::Lasts) -> Result<Option<u64>, crate::fault::Fault> {
    if !lasts.ok() {
        return Err(crate::fault::Fault::known(crate::fault::Known::MachineShape, crate::machine::member::CLI_ALONE.to_string()));
    }
    let issued = shell.desk.pace_now();
    let term = eravon_seal::credential::Term { issued, expires: lasts.expires(issued) };
    crate::keybox::issue_credential(term)?;
    match crate::machine::update(|m| m.cli_alone = Some(lasts)) {
        Ok(m) => shell.machine = m,
        Err(f) => {
            let _ = crate::keybox::drop_credential();
            return Err(f);
        }
    }
    shell.credential = Some(term);
    Ok(term.expires)
}

fn cli_pass_add(shell: &mut Shell, note: &str) -> Result<(crate::secret::Secret, eravon_seal::credential::Row), crate::fault::Fault> {
    match crate::keybox::add_pass(note, crate::house::now_secs()) {
        Ok((pass, row)) => {
            shell.passes.push(row.clone());
            Ok((pass, row))
        }
        Err(f) => {
            match f.which() {
                Some(crate::fault::Known::CredentialExpired) => cli_alone_ended(shell, crate::fault::Known::CredentialExpired),
                Some(crate::fault::Known::CredentialShape) => cli_alone_ended(shell, crate::fault::Known::CredentialReclaimed),
                Some(crate::fault::Known::CredentialMissing) if shell.machine.cli_alone.is_some() => cli_alone_ended(shell, crate::fault::Known::CredentialMissing),
                _ => {}
            }
            Err(f)
        }
    }
}

fn cli_pass_drop(shell: &mut Shell, tag: &str) -> Result<(), crate::fault::Fault> {
    let done = crate::keybox::drop_pass(tag);
    shell.reread_credential();
    done
}

fn cli_alone_off(shell: &mut Shell) -> Result<(), crate::fault::Fault> {
    crate::keybox::drop_credential()?;
    shell.credential = None;
    shell.passes.clear();
    shell.machine = crate::machine::update(|m| m.cli_alone = None)?;
    Ok(())
}

pub fn cli_alone_ended(shell: &mut Shell, why: crate::fault::Known) {
    let had = match crate::keybox::drop_credential() {
        Ok(had) => had,
        Err(f) => {
            shell.faults.push(f);
            return;
        }
    };
    alone_ended(shell, why, had);
}

pub fn alone_ended(shell: &mut Shell, why: crate::fault::Known, had: bool) {
    shell.credential = None;
    shell.passes.clear();
    if shell.machine.cli_alone.is_none() && !had {
        return;
    }
    match crate::machine::update(|m| m.cli_alone = None) {
        Ok(m) => shell.machine = m,
        Err(f) => shell.faults.push(f),
    }
    shell.faults.push(crate::fault::Fault::known(why, crate::places::credential_file()));
}

fn cli_path_set(cli: &std::path::Path, on: bool) -> Result<Done, crate::fault::Fault> {
    crate::machine::cli_path(cli, on).map(Done::CliPath)
}

#[cfg(test)]
mod ai_config_tests {
    #[test]
    fn the_config_copied_carries_the_key_just_made() {
        use eravon::json::Value;
        let base = super::ai_config(std::path::Path::new("/opt/eravon/bin/eravon"));
        let with = super::ai_config_with_pass(&base, "zkp_test").expect("in shape");
        let v = eravon::json::parse(with.as_bytes()).expect("json");
        let server = v.member("mcpServers").and_then(|m| m.member("eravon")).expect("the server");
        assert_eq!(server.member("env").and_then(|e| e.member(eravon_seal::credential::PASS_ENV)), Some(&Value::Str("zkp_test".into())));
        assert_eq!(server.member("command"), Some(&Value::Str("/opt/eravon/bin/eravon".into())));
        assert_eq!(server.member("args"), Some(&Value::Arr(vec![Value::Str("mcp".into())])));
        let mut back = eravon::json::parse(with.as_bytes()).expect("json");
        if let Value::Obj(top) = &mut back {
            if let Some((_, Value::Obj(servers))) = top.iter_mut().find(|(k, _)| k == "mcpServers") {
                if let Some((_, Value::Obj(s))) = servers.iter_mut().find(|(k, _)| k == "eravon") {
                    s.retain(|(k, _)| k != "env");
                }
            }
        }
        assert_eq!(String::from_utf8_lossy(&eravon::json::canon_bytes(&back)), base, "nothing else changed");
        assert!(super::ai_config_with_pass("not json", "zkp_test").is_none());
        assert!(super::ai_config_with_pass(r#"{"mcpServers":{}}"#, "zkp_test").is_none());
    }
}
