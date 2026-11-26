use super::*;

pub(super) fn confirm_identity(shell: &mut Shell, answers: &[(usize, crate::secret::Secret)], label: &str) -> Result<Applied, crate::fault::Fault> {
    let Some(f) = shell.new_words.as_ref() else {
        return Err(crate::fault::Fault::known(crate::fault::Known::PhraseConfirm, String::new()));
    };
    crate::identity::confirm(f, answers)?;
    shell.new_label = Some(label.trim().to_string());
    let before = registered()?;
    let was = current_seat()?;
    let f = f.twin();
    let label = label.trim().to_string();
    Ok(vault(shell, move || {
        let row = crate::register::change(|reg| {
            crate::identity::add_words(reg, &f, true, &label)
        })?;
        let restored = before.contains(&row.id);
        Ok(crate::task::Vault::Identity { row, restored, fresh: true, was })
    }))
}

pub(super) fn identity_landed(shell: &mut Shell, row: crate::identity::Row, restored: bool, fresh: bool, was: Option<(String, crate::roles::Role)>) -> Applied {
    if fresh {
        shell.new_words = None;
    }
    let mut row = row;
    if let Some(label) = shell.new_label.take().filter(|l| !l.trim().is_empty()) {
        match crate::register::change(|reg| crate::identity::rename(reg, &row.id, &label)) {
            Ok(fresh_row) => row = fresh_row,
            Err(f) => shell.faults.push(f),
        }
    }
    let landed = row.seat_here_or_first(shell.settings.role);
    match enter(shell, &row, landed, Back::To(was)) {
        Ok(()) => {
            shell.key_born(&row);
            shell.seat_follow();
            shell.reread_vault();
            Applied::IdentityMade { id: row.id.clone(), seats: seats_of(&row), restored }
        }
        Err(f) => shell.trouble(f),
    }
}

pub(super) fn seats_of(row: &crate::identity::Row) -> Vec<(crate::roles::Role, Address)> {
    row.seats().into_iter().filter_map(|s| row.address(s).map(|a| (s, a))).collect()
}

fn current_seat() -> Result<Option<(String, crate::roles::Role)>, crate::fault::Fault> {
    Ok(crate::register::read()?.and_then(|r| r.current))
}

pub(super) fn registered() -> Result<Vec<String>, crate::fault::Fault> {
    Ok(crate::register::read()?.map(|r| r.rows.into_iter().map(|x| x.id).collect()).unwrap_or_default())
}

pub(super) fn import_identity(shell: &mut Shell, form: ImportForm, label: &str) -> Result<Applied, crate::fault::Fault> {
    if let ImportForm::PrivateKey { keyfile: Some(out), .. } = &form {
        key_file_checks(&out.password, &out.again, &out.dir)?;
    }
    shell.new_label = Some(label.trim().to_string());
    let before = registered()?;
    let was = current_seat()?;
    let label = label.trim().to_string();
    Ok(vault(shell, move || {
        let row = imported(form, &label)?;
        let restored = before.contains(&row.id);
        Ok(crate::task::Vault::Identity { row, restored, fresh: false, was })
    }))
}

pub(super) fn imported(form: ImportForm, label: &str) -> Result<crate::identity::Row, crate::fault::Fault> {
    use crate::fault::{Fault, Known};
    let seat = crate::house::seam::import_seat(&form)?;
    let row = match form {
        ImportForm::Words(text) => {
            let f = crate::identity::from_words(text.expose())?;
            crate::register::change(|reg| {
                let row = crate::identity::add_words(reg, &f, true, label)?;
                Ok(row)
            })?
        }
        ImportForm::PrivateKey { key, keyfile } => {
            let s = crate::key::from_hex(key.expose()).ok_or_else(|| Fault::known(Known::KeyMalformed, String::new()))?;
            let becomes_primary = crate::keybox::primary()?.is_none();
            if let (Some(_), Some(addr)) = (&keyfile, s.address()) {
                let reg = crate::register::view()?;
                crate::identity::meets(&reg, &crate::identity::Keys::One { seat, addr }, label)?;
            }
            let landed = match keyfile {
                Some(out) => {
                    let dir = key_file_checks(&out.password, &out.again, &out.dir)?;
                    Some(write_key_file(&s, &out.password, &dir)?.0)
                }
                None if becomes_primary => return Err(Fault::known(Known::PrimaryNoKeyFile, String::new())),
                None => None,
            };
            crate::register::change(|reg| {
                let row = crate::identity::add_existing(reg, &s, seat, landed.is_some(), label)?;
                if let Some(at) = landed.as_deref() {
                    crate::identity::mark(reg, &row.id, false, true, Some(at))?;
                }
                Ok(row)
            })?
        }
        ImportForm::Keystore { path, password } => {
            let p = path.trim();
            let bytes = std::fs::read(p).map_err(|e| crate::fault::classify(&e, p))?;
            let s = crate::keystore::decrypt_typed(&bytes, &password)?;
            crate::register::change(|reg| {
                let row = crate::identity::add_existing(reg, &s, seat, true, label)?;
                Ok(row)
            })?
        }
    };
    Ok(row)
}

pub(super) fn delete_identity(shell: &mut Shell, id: &str) -> Result<String, crate::fault::Fault> {
    let stood = shell.identities.as_ref().and_then(crate::identity::now_row).is_some_and(|(r, _)| r.id.eq_ignore_ascii_case(id));
    let (gone, next) = crate::register::change(|reg| crate::identity::delete(reg, id, false))?;
    match next {
        Some((r, s)) if stood => {
            if let Err(f) = enter(shell, &r, s, Back::Gone) {
                shell.faults.push(f);
            }
        }
        Some((r, s)) => enter(shell, &r, s, Back::Was)?,
        None => {
            shell.refresh_anchor()?;
            shell.seat_identities(Some(crate::register::view()?));
        }
    }
    Ok(gone.id)
}

pub fn seats_with_entries(row: &crate::identity::Row) -> (Vec<crate::roles::Role>, Vec<crate::fault::Fault>) {
    let mut seats = Vec::new();
    let mut troubles = Vec::new();
    for s in row.seats() {
        let Some(home) = row.home(s) else { continue };
        let dir = home.join(crate::home::Slot::Store.as_str());
        if !dir.is_dir() {
            continue;
        }
        match std::fs::read_dir(&dir) {
            Ok(mut l) => {
                if l.next().is_some() {
                    seats.push(s);
                }
            }
            Err(e) => {
                seats.push(s);
                troubles.push(crate::fault::classify(&e, &dir.display().to_string()));
            }
        }
    }
    (seats, troubles)
}

pub(super) fn same_twice(pin: &crate::secret::Secret, again: &crate::secret::Secret) -> Result<(), crate::fault::Fault> {
    if pin != again {
        return Err(crate::fault::Fault::known(crate::fault::Known::PasswordsDiffer, String::new()));
    }
    Ok(())
}

pub(super) fn set_auto_lock(shell: &mut Shell, on: bool, secs: u64) -> Result<(bool, u64), crate::fault::Fault> {
    shell.machine = crate::machine::update(|m| {
        m.auto_lock = on;
        m.auto_lock_secs = secs;
    })?;
    Ok((shell.machine.auto_lock, shell.machine.auto_lock_secs))
}

pub(super) fn export_backup(shell: &mut Shell, password: crate::secret::Secret, again: &crate::secret::Secret, dir: &str) -> Result<Spawned, crate::fault::Fault> {
    crate::backup::password_ok(&password, again)?;
    let dir = crate::home::landing(dir)?;
    let now = super::home::now_secs();
    Ok(shell.tasks.spawn(Kind::Backup, move || {
        let made = (|| {
            std::fs::create_dir_all(&dir).map_err(|e| crate::fault::classify(&e, &dir.display().to_string()))?;
            crate::backup::export_staged(&dir, password.expose(), now, |at| crate::task::stage_at(Kind::Backup, at))
        })();
        match made {
            Ok(done) => Ok(Done::BackupMade { path: done.path.display().to_string(), summary: done.summary }),
            Err(f) => {
                let _ = crate::machine::update(|m| m.backup_failed = Some(now));
                Err(f)
            }
        }
    }))
}

pub(super) fn set_appearance(shell: &mut Shell, appearance: &str) -> Result<String, crate::fault::Fault> {
    shell.machine = crate::machine::update(|m| m.appearance = Some(appearance.to_string()))?;
    Ok(shell.machine.appearance.clone().unwrap_or_default())
}


pub(super) fn set_proxy(shell: &mut Shell, choice: &str) -> Result<String, crate::fault::Fault> {
    let written = crate::machine::proxy::choose(choice, shell.machine.proxy.as_deref())?;
    shell.machine = crate::machine::update(|m| m.proxy = Some(written))?;
    Ok(shell.machine.proxy.as_deref().map(crate::machine::proxy::shown).unwrap_or_default())
}

pub(super) fn reveal_words(shell: &mut Shell) -> Result<(), crate::fault::Fault> {
    use crate::fault::{Fault, Known};
    let view = crate::register::view()?;
    let id = crate::identity::now_row(&view)
        .map(|(r, _)| r.id)
        .ok_or_else(|| Fault::known(Known::NoIdentity, String::new()))?;
    shell.words = Some(crate::identity::words_of(&view, &id)?);
    Ok(())
}

pub(super) fn name_identity(shell: &mut Shell, id: &str, label: &str) -> Result<crate::identity::Row, crate::fault::Fault> {
    let row = crate::register::change(|reg| crate::identity::rename(reg, id, label))?;
    shell.seat_identities(Some(crate::register::view()?));
    Ok(row)
}

pub(super) fn key_file_checks(password: &crate::secret::Secret, again: &crate::secret::Secret, dir: &str) -> Result<String, crate::fault::Fault> {
    crate::backup::password_ok(password, again)?;
    Ok(crate::home::landing(dir)?.display().to_string())
}

pub(super) fn write_key_file(secret: &crate::key::Secret, password: &crate::secret::Secret, dir: &str) -> Result<(String, Address), crate::fault::Fault> {
    let ks = crate::keystore::encrypt(secret, password.expose(), Params::standard(), now_secs())?;
    std::fs::create_dir_all(dir).map_err(|e| crate::fault::classify(&e, dir))?;
    let path = std::path::Path::new(dir).join(&ks.file_name);
    eravon_store::landing::land_bytes_for(eravon_store::landing::Readers::Owner, &path, &ks.json)
        .map_err(|t| crate::fault::Fault::of_landing(t))?;
    if let Some(tamper) = LANDED_TAMPER.get() {
        tamper(&path);
    }
    landed_check(&path, ks.address)?;
    Ok((path.display().to_string(), ks.address))
}

pub(super) fn backup_key(shell: &mut Shell, password: crate::secret::Secret, again: &crate::secret::Secret, dir: &str) -> Result<Spawned, crate::fault::Fault> {
    use crate::fault::{Fault, Known};
    let dir = key_file_checks(&password, again, dir)?;
    let seat = shell.settings.role;
    let view = crate::register::view()?;
    let acct = crate::identity::account_now(&view)
        .ok_or_else(|| Fault::known(Known::SeatUnseated, crate::lang::both_of(|| crate::lang::t(crate::lang::Key::IdSeatEmpty).to_string())))?;
    let secret = crate::key::load(Some(&acct))?.ok_or_else(|| Fault::known(Known::KeychainMissing, acct.clone()))?;
    let id = crate::identity::now_row(&view).map(|(r, _)| r.id);
    Ok(shell.tasks.spawn(Kind::Keystore, move || {
        let (path, address) = write_key_file(&secret, &password, &dir)?;
        Ok(Done::Keystore(crate::task::Keystore::BackedUp { path, address, id, seat }))
    }))
}

pub fn set_landed_tamper(f: fn(&std::path::Path)) -> bool {
    LANDED_TAMPER.set(f).is_ok()
}

pub(super) fn landed_check(path: &std::path::Path, want: Address) -> Result<(), crate::fault::Fault> {
    use crate::fault::{Fault, Known};
    let at = path.display().to_string();
    let back = std::fs::read(path).map_err(|e| crate::fault::classify(&e, &at))?;
    let shape = crate::keystore::shape(&back)
        .map_err(|f| Fault::known(Known::BackupNotLanded, crate::lang::both_of(|| format!("{at}: {}", f.said()))))?;
    let bad = shape.compliant();
    if !bad.is_empty() {
        return Err(Fault::known(Known::BackupNotLanded, format!("{at}: {}", bad.join(" "))));
    }
    if shape.address != Some(want) {
        return Err(Fault::known(Known::BackupNotLanded, format!("{at}: {} ≠ {}", shape.address_text, want.hex())));
    }
    Ok(())
}
