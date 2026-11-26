pub mod general;
pub mod act;
pub mod approve;
pub mod forms;
pub mod drive;
pub mod cards;
pub mod desk;
pub mod exec;
pub mod flow;
pub mod landed;
pub mod merge;
pub mod run;
pub mod action;
pub mod ask;
pub mod book;
pub mod bundle;
pub mod buyer_home;
pub mod buyer_orders;
pub mod coin;
pub mod export;
pub mod first;
pub mod gatecard;
pub mod history;
pub mod home;
pub mod i18n;
pub mod case;
pub mod complaint;
pub mod judge_card;
pub mod link;
pub mod links;
pub mod openurl;
pub use crate::platform::house as platform;
pub mod orders;
pub mod paylink;
pub mod queue;
pub mod refresh;
pub mod refusal;
pub mod seam;
pub mod seller;
pub mod settings;
pub mod shell;
pub mod steps;
pub mod sync;
pub mod text;
pub mod vocab;
pub mod wallet;

#[macro_export]
macro_rules! fill {
    ($key:expr; $($a:expr),* $(,)?) => {
        $crate::house::i18n::filln($key, &[$($crate::house::i18n::arg(&$a)),*])
    };
}

#[macro_export]
macro_rules! fill_in {
    ($lang:expr, $key:expr; $($a:expr),* $(,)?) => {
        $crate::house::i18n::filln_in($lang, $key, &[$($crate::house::i18n::arg(&$a)),*])
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ticket;

impl Ticket {
    pub fn standing() -> Ticket {
        Ticket
    }

    pub fn live(&self) -> bool {
        !crate::task::ticket_void()
    }
}

pub fn asked(r: Result<(), seller::Said>) -> Result<(), seller::Said> {
    r
}


pub fn stale() -> seller::Said {
    seller::Said::bad(STALE, i18n::t(i18n::Key::ErrESTORESTALE))
}

pub const STALE: &str = "E_APP_STALE";

pub fn said_of(e: &eravon_vault::vfault::Fault) -> seller::Said {
    match e.family() {
        Some(f) => seller::Said::bad(f.which().map_or("", |k| k.as_str()), f.human()),
        None => seller::Said::bad(e.name(), e.detail()),
    }
}

pub fn said_fault(f: &crate::fault::Fault) -> seller::Said {
    seller::Said::bad(f.which().map_or("", |k| k.as_str()), f.said())
}

pub fn check_file(backup: bool, file: &str, pass: &eravon_ui::secret::Secret) -> Result<(), seller::Said> {
    if pass.overflowed() {
        return Err(said_fault(&crate::fault::Fault::known(crate::fault::Known::PasswordLong, eravon_ui::secret::CAP.to_string())));
    }
    if backup {
        return crate::backup::peek(std::path::Path::new(file), pass.expose()).map(|_| ()).map_err(|f| said_fault(&f));
    }
    let bytes = std::fs::read(file).map_err(|e| said_of(&eravon_vault::vfault::Fault::from(crate::fault::Fault::known(crate::fault::Known::KeystoreShape, crate::fill!(i18n::Key::GateFileUnreadable; e.to_string())))))?;
    eravon_vault::keyfile::accept(&bytes, pass.expose()).map(|_| ()).map_err(|e| said_of(&e))
}

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::H1;

    pub const FEATURE_P1: eravon_literals::trace::Feature = eravon_literals::trace::Feature::P1;

    pub const FEATURE_P5: eravon_literals::trace::Feature = eravon_literals::trace::Feature::P5;

    pub const FEATURE_P8: eravon_literals::trace::Feature = eravon_literals::trace::Feature::P8;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

pub fn is_house(k: crate::task::Kind) -> bool {
    use crate::task::Kind;
    k.page_of() != k || matches!(k, Kind::Sync | Kind::Orders | Kind::Goods | Kind::Cands | Kind::Home | Kind::Remind | Kind::PayLink | Kind::Bo | Kind::BHome | Kind::Wallet | Kind::Settings | Kind::Export | Kind::First)
}

pub const WRITER: &str = "eravon-app";

pub fn data() -> Result<String, crate::fault::Fault> {
    Ok(crate::home::machine_dir()?.display().to_string())
}

pub fn store_fault(e: eravon_store::fault::Fault) -> crate::fault::Fault {
    crate::fault::Fault::from(e)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn write_global(f: impl FnOnce(&mut eravon_store::settings::Global)) -> Result<(), crate::fault::Fault> {
    let data = data()?;
    let s = eravon_vault::store::open_write(&data, WRITER, now_secs().max(1)).map_err(store_fault)?;
    let mut g = eravon_vault::store::global_to_change(s.data()).map_err(store_fault)?;
    let was = g.clone();
    f(&mut g);
    if g == was {
        return Ok(());
    }
    eravon_vault::store::write_global(s.data(), &g).map_err(store_fault)
}

pub fn remeasure(shell: &mut crate::shell::Shell) {
    if let Some(root) = shell.home.as_ref().map(|h| h.root().to_path_buf()) {
        crate::action::spawn_for(shell, crate::task::Kind::Archive, move || measure(&root));
    }
}

pub fn measure(root: &std::path::Path) -> Result<crate::task::Done, crate::fault::Fault> {
    crate::task::stage_at(crate::task::Kind::Archive, 0);
    let home = crate::home::Home::open(root)?;
    let bytes = home.usage()?;
    let records = crate::backup::count_now()? as usize;
    let machine_items = crate::machine::read().and_then(|m| crate::backup::measured(m.backup.as_ref()));
    Ok(crate::task::Done::Archive { bytes, records, machine_items })
}

#[cfg(test)]
mod said_tests {
    #[test]
    fn a_passcode_rule_is_said_in_the_family_words() {
        use eravon_vault::vfault::Fault;
        let differ = super::said_of(&Fault::Rule(eravon_vault::seam::RULE_MATCH));
        assert_eq!(differ.name, "PASSWORDS_DIFFER");
        assert!(!differ.detail.contains(eravon_vault::seam::RULE_MATCH));
        let first_run = eravon_vault::fault::Fault::known(eravon_vault::fault::Known::PasswordsDiffer, String::new());
        assert_eq!(differ.detail, first_run.human());
        for r in eravon_vault::seam::RULES.into_iter().filter(|r| *r != eravon_vault::seam::RULE_MATCH) {
            let s = super::said_of(&Fault::Rule(r));
            assert_eq!(s.name, "PIN_SHAPE");
            assert!(!s.detail.contains(r), "{r}");
        }
        assert_eq!(super::said_of(&Fault::Rule(eravon_vault::seam::RULE_FILE_LEN)).name, "PASSWORD_SHORT");
        assert_eq!(super::said_of(&Fault::Locked).name, Fault::Locked.name());
    }
}
