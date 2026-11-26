use eravon_store::vault::{IdentityKind, Seat};

use crate::keybox;
use crate::records;
use crate::roles::Role;
use crate::vfault::{Fault, R};
use crate::{rules, seam};

pub fn seat_of(r: Role) -> Seat {
    match r {
        Role::Author => Seat::Seller,
        Role::Grantee => Seat::Buyer,
    }
}

pub fn role_of(s: Seat) -> Role {
    match s {
        Seat::Seller => Role::Author,
        Seat::Buyer => Role::Grantee,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub address: String,
    pub kind: IdentityKind,
    pub seat: Seat,
    pub label: String,
    pub primary: bool,
    pub id: String,
    pub created: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vault {
    pub identities: Vec<Identity>,
}

impl Vault {
    pub fn primary(&self) -> R<&Identity> {
        self.identities.iter().find(|i| i.primary).ok_or_else(|| Fault::Vault("密钥库里没有主身份".into()))
    }

    pub fn find(&self, address: &str) -> R<&Identity> {
        self.identities
            .iter()
            .find(|i| i.address.eq_ignore_ascii_case(address))
            .ok_or_else(|| Fault::NoIdentity(address.to_string()))
    }
}

pub fn read(data: &str) -> R<Vault> {
    records::bind(data)?;
    let reg = crate::register::read()?.unwrap_or_default();
    let primary = keybox::primary()?.map(|(id, _)| id);
    let mut out = Vault::default();
    for row in &reg.rows {
        let kind = match row.kind() {
            crate::identity::Kind::Words => IdentityKind::Words,
            crate::identity::Kind::Existing => IdentityKind::Existing,
        };
        for r in row.seats() {
            if let Some(a) = row.current(r) {
                out.identities.push(Identity {
                    address: a.hex(),
                    kind,
                    seat: seat_of(r),
                    label: row.label.clone(),
                    primary: primary.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(&row.id)),
                    id: row.id.clone(),
                    created: row.created.clone(),
                });
            }
        }
    }
    Ok(out)
}

pub struct Born {
    pub address: String,
    pub phrase: String,
    pub sealed_at: (crate::kdf::Params, u64),
    label: String,
    passcode: String,
    fresh: crate::identity::Fresh,
}

impl Drop for Born {
    fn drop(&mut self) {
        let mut p = std::mem::take(&mut self.passcode).into_bytes();
        eravon_os::wipe(&mut p);
        let mut w = std::mem::take(&mut self.phrase).into_bytes();
        eravon_os::wipe(&mut w);
    }
}

pub fn create(passcode: &str, seat: Seat, label: &str, phrase: Option<&str>, _now: u64) -> R<Born> {
    let fresh = match phrase {
        Some(p) => crate::identity::from_words(p)?,
        None => crate::identity::fresh()?,
    };
    let phrase = fresh.words().iter().map(|w| w.expose().to_string()).collect::<Vec<_>>().join(" ");
    let address = fresh.address(role_of(seat)).map(|a| a.hex()).ok_or_else(|| Fault::Vault("这一段助记词派生不出这一席的钥".into()))?;
    Ok(Born { address, phrase, sealed_at: (crate::kdf::params(), 0), label: label.to_string(), passcode: passcode.to_string(), fresh })
}

impl Born {
    pub fn write(&mut self, data: &str) -> R<Option<Fault>> {
        eravon_store::atomic::mkdirs(data)?;
        records::bind(data)?;
        let mut aside = None;
        if keybox::pin_trouble(&self.passcode).is_none() && matches!(keybox::state()?, keybox::State::Absent) {
            let (n, at) = crate::local::set_aside_sealed()?;
            if n > 0 {
                aside = Some(Fault::from(crate::fault::Fault::known(
                    crate::fault::Known::LocalSetAside,
                    format!("{n} · {}", at.map(|p| p.display().to_string()).unwrap_or_default()),
                )));
            }
        }
        let t = std::time::Instant::now();
        keybox::set_pin(&self.passcode)?;
        let ms = u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX);
        let fresh = self.fresh.twin();
        let done = crate::accounts::file(&self.label, false, |reg, checked| crate::identity::add_words(reg, &fresh, true, checked)).map(|_| ());
        keybox::lock();
        done?;
        self.sealed_at = (crate::kdf::params(), ms);
        Ok(aside)
    }
}

pub fn unlock(data: &str, passcode: &str) -> R<()> {
    records::bind(data)?;
    keybox::unlock(passcode)?;
    crate::local::after_open();
    Ok(())
}

pub fn recover(data: &str, phrase: &str, new_passcode: &str, again: &str) -> R<()> {
    records::bind(data)?;
    if keybox::state()? == keybox::State::Absent {
        return Err(Fault::from(crate::fault::Fault::known(crate::fault::Known::KeyboxMissing, String::new())));
    }
    rules::twice(new_passcode, again).map_err(Fault::Rule)?;
    let fresh = crate::identity::from_words(phrase)?;
    crate::identity::recover_with(&fresh, new_passcode)?;
    crate::local::after_open();
    Ok(())
}

pub fn passcode(data: &str, current: &str, next: &str, again: &str) -> R<()> {
    rules::twice(next, again).map_err(Fault::Rule)?;
    records::bind(data)?;
    keybox::change_pin(current, next)?;
    Ok(())
}

pub fn lock_now() {
    keybox::lock();
}

pub fn open_key(data: &str, address: &str) -> R<[u8; 32]> {
    records::bind(data)?;
    Ok(crate::sign::key_of(address)?)
}

pub fn open_phrase(data: &str, address: &str) -> R<String> {
    records::bind(data)?;
    let (row, _) = records::seat_of(address)?;
    let reg = crate::register::read()?.unwrap_or_default();
    Ok(crate::identity::words_of(&reg, &row.id)?.iter().map(|w| w.expose().to_string()).collect::<Vec<_>>().join(" "))
}

pub fn exists(data: &str) -> bool {
    records::bind(data).is_ok_and(|m| m.join(crate::places::keybox_file()).exists())
}

pub fn readable(data: &str) -> R<()> {
    records::bind(data)?;
    keybox::state().map(|_| ()).map_err(Fault::from)
}

pub fn count(data: &str) -> R<usize> {
    records::bind(data)?;
    match read(data) {
        Ok(v) => Ok(v.identities.len()),
        Err(Fault::Locked) => Ok(keybox::slot_count()?),
        Err(e) => Err(e),
    }
}

pub fn wrong_count(data: &str) -> R<u32> {
    records::bind(data)?;
    Ok(u32::try_from(keybox::WRONG_LIMIT.saturating_sub(keybox::tries_left()?)).unwrap_or(u32::MAX))
}

pub fn gate(data: &str) -> R<(&'static str, u32)> {
    records::bind(data)?;
    let max = eravon_literals::constants::PASSCODE_MAX_ATTEMPTS;
    match keybox::state()? {
        keybox::State::Absent => Err(Fault::from(crate::fault::Fault::known(crate::fault::Known::KeyboxMissing, String::new()))),
        keybox::State::LockedOut => Ok((seam::GATE_DEAD, max)),
        keybox::State::Open => Ok((seam::GATE_OPEN, 0)),
        keybox::State::Locked { .. } => {
            let wrong = wrong_count(data)?;
            Ok((if wrong >= max { seam::GATE_DEAD } else { seam::GATE_LOCKED }, wrong))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "48261537";

    #[test]
    fn a_made_vault_opens_lists_both_seats_and_recovers_with_its_words() {
        let data = crate::testing::fresh();
        let mut b = create(PIN, Seat::Seller, "主钥", None, 900).expect("建得出库");
        b.write(&data).expect("落得下盘");
        assert!(exists(&data));
        assert!(matches!(read(&data), Err(Fault::Locked)), "锁着读不到身份表");
        assert!(unlock(&data, "48261538").is_err());
        assert_eq!(wrong_count(&data).unwrap(), 1, "错一次即落盘");
        unlock(&data, PIN).expect("对的口令开得了");
        assert_eq!(wrong_count(&data).unwrap(), 0);
        let v = read(&data).expect("开着读得到");
        assert_eq!(v.identities.len(), 2, "助记词身份占两席");
        let seller = v.find(&b.address).expect("建库那一席在表上");
        assert_eq!(seller.seat, Seat::Seller);
        assert!(seller.primary);
        let key = open_key(&data, &b.address).expect("取得到钥");
        assert_eq!(crate::key::Secret::take(key).and_then(|s| s.address()).map(|a| a.hex()), Some(b.address.clone()));
        assert_eq!(open_phrase(&data, &b.address).expect("取得到词"), b.phrase);
        lock_now();
        let phrase = b.phrase.clone();
        recover(&data, &phrase, "73915264", "73915264").expect("主身份的词恢复得了");
        unlock(&data, "73915264").expect("新口令开得了");
        lock_now();
    }
}
