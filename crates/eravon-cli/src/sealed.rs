use eravon_seal::keybox::{self, Vault};
use eravon_seal::{Code, Trouble};
use std::path::PathBuf;

struct Unlocked {
    account: String,
}

fn unlock(now: u64) -> Result<Unlocked, Trouble> {
    let machine = eravon_os::machine::here().map(|(m, _)| m).map_err(|_| Trouble::new(Code::CredentialMissing, String::new()))?;
    let account = eravon_seal::credential::find(&machine)?;
    let pass = std::env::var_os(eravon_seal::credential::PASS_ENV).map(|v| v.into_string().unwrap_or_else(|_| String::from("\u{0}")));
    let mk = eravon_seal::credential::open(&machine, &account, now, pass.as_deref())?;
    drop(pass);
    let mk: [u8; 32] = mk.bytes().try_into().map_err(|_| Trouble::new(Code::CredentialShape, account.clone()))?;
    keybox::hold_master(&mk);
    let vault = Vault::new(machine.clone(), eravon_seal::places::keybox_file_for(&account), eravon_seal::places::keybox_lock_file_for(&account));
    if !keybox::master_is_this(&vault)? {
        eravon_seal::credential::remove(&machine, &account)?;
        return Err(Trouble::new(Code::CredentialShape, format!("{} master", eravon_seal::credential::path(&machine, &account).display())));
    }
    eravon_chain::action::keep_in(std::sync::Arc::new(eravon_send::checked::Logs(eravon_send::checked::Sealed(machine.clone()))));
    Ok(Unlocked { account })
}

pub fn open(s: &mut eravon_vault::session::Session, now: u64) -> Result<(), crate::fault::Fault> {
    let refused = |t: Trouble| crate::named::credential(&t).unwrap_or_else(|| crate::home::down(eravon_vault::vfault::Fault::from(eravon_vault::fault::Fault::from(t))));
    let u = unlock(now).map_err(refused)?;
    if u.account != eravon_vault::places::key_account() {
        let at: PathBuf = eravon_os::machine::here().map(|(m, _)| m).unwrap_or_default();
        return Err(refused(Trouble::new(Code::CredentialShape, eravon_seal::credential::path(&at, &u.account).display().to_string())));
    }
    s.opened().map_err(crate::home::down)
}

pub fn refused(f: &eravon_store::fault::Fault) -> crate::out::Answer {
    let e = crate::deskops::No::store(f.clone()).fault();
    crate::deskwire::answered(String::from_utf8_lossy(&eravon_literals::json::write(&crate::answer::refuse(&e))).into_owned())
}

pub fn word(c: Code) -> String {
    let mut out = String::new();
    for (i, ch) in format!("{c:?}").chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    out
}

pub(crate) fn this_machines(home: &std::path::Path, machine: &std::path::Path) -> Result<(), eravon_store::fault::Fault> {
    use eravon_store::fault::Fault;
    eravon_store::writer::machine_at(Some(machine.to_path_buf()));
    let data = home.display().to_string();
    if let Some(f) = eravon_store::writer::mark_trouble(&data) {
        return Err(Fault::OtherMachine(format!("写者标读不懂:{};标原样留着,要在这台机器写,先改由本机写", f.detail())));
    }
    match eravon_store::writer::other_machine(&data)? {
        Some(m) => Err(Fault::OtherMachine(format!("这一家由另一台机器在写(那台机器的标 {m});要在这台机器写,先改由本机写"))),
        None => Ok(()),
    }
}
