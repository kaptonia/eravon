use eravon_store::vault::Seat;

use crate::session::Session;
use crate::records;
use crate::vault::{self, role_of, Identity, Vault};
use crate::vfault::{Fault, R};
use crate::seam;

pub fn list(v: &Vault, seat: Seat) -> Vec<&Identity> {
    v.identities.iter().filter(|i| i.seat == seat).collect()
}

pub fn primary_of(v: &Vault, seat: Seat) -> Option<&Identity> {
    v.identities.iter().find(|i| i.seat == seat && i.primary)
}

pub fn primary(v: &Vault) -> Option<&Identity> {
    v.identities.iter().find(|i| i.primary)
}

pub fn active(data: &str, seat: Seat) -> R<Option<String>> {
    records::bind(data)?;
    let Some((row, _)) = crate::register::now_row_listed()? else { return Ok(None) };
    Ok(row.current(role_of(seat)).map(|a| a.hex()))
}

pub fn switch(data: &str, seat: Seat, address: &str) -> R<String> {
    let v = vault::read(data)?;
    let want = find(&v, address)?;
    if want.seat != seat {
        return Err(Fault::NoIdentity(format!(
            "{} 在{}席上,切不到{}席来",
            want.address,
            word(want.seat),
            word(seat)
        )));
    }
    let id = want.id.clone();
    crate::register::change(|reg| crate::identity::switch(reg, &id, role_of(seat)))?;
    Ok(want.address.clone())
}


fn word(s: Seat) -> &'static str {
    match s {
        Seat::Seller => "卖家",
        Seat::Buyer => "买家",
    }
}

pub fn find<'a>(v: &'a Vault, address: &str) -> R<&'a Identity> {
    v.identities
        .iter()
        .find(|i| i.address.eq_ignore_ascii_case(address))
        .ok_or_else(|| Fault::NoIdentity(format!("身份表上没有 {address}")))
}

pub fn find_in(data: &str, address: &str) -> R<Seat> {
    let v = vault::read(data)?;
    Ok(find(&v, address)?.seat)
}

pub fn rename(s: &Session, address: &str, label: &str) -> R<()> {
    let v = s.vault()?;
    let one = find(&v, address)?;
    let id = one.id.clone();
    crate::register::change(|reg| crate::identity::rename(reg, &id, label.trim()))?;
    Ok(())
}

pub fn new_words(s: &Session, seat: Seat, label: &str, now: u64) -> R<(String, String)> {
    new_words_as(s, seat, label, true, now)
}

pub fn new_words_as(s: &Session, seat: Seat, label: &str, named: bool, _now: u64) -> R<(String, String)> {
    let f = crate::identity::fresh()?;
    let phrase = f.words().iter().map(|w| w.expose().to_string()).collect::<Vec<_>>().join(" ");
    let address = add_words(s, seat, label, named, &f)?;
    Ok((address, phrase))
}

pub fn import_words(s: &Session, seat: Seat, label: &str, phrase: &str, now: u64) -> R<String> {
    import_words_as(s, seat, label, true, phrase, now)
}

pub fn import_words_as(s: &Session, seat: Seat, label: &str, named: bool, phrase: &str, _now: u64) -> R<String> {
    let f = crate::identity::from_words(phrase)?;
    add_words(s, seat, label, named, &f)
}

fn add_words(s: &Session, seat: Seat, label: &str, named: bool, f: &crate::identity::Fresh) -> R<String> {
    let v = s.vault()?;
    let address = f.address(role_of(seat)).map(|a| a.hex()).ok_or_else(|| Fault::Vault("这一段助记词派生不出这一席的钥".into()))?;
    if let Some(had) = v.identities.iter().find(|i| i.address.eq_ignore_ascii_case(&address)) {
        return Err(Fault::Exists(here_as(had)));
    }
    if named {
        label_free(&v, seat, label, None)?;
    }
    file(label, named, |reg, checked| crate::identity::add_words(reg, f, true, checked))?;
    adopt(s.data(), seat, &address)?;
    Ok(address)
}

pub(crate) fn file(
    label: &str,
    named: bool,
    add: impl FnOnce(&mut crate::identity::Registry, &str) -> Result<crate::identity::Row, crate::fault::Fault>,
) -> R<crate::identity::Row> {
    let label = label.trim().to_string();
    let row = crate::register::change(|reg| {
        let row = add(reg, if named { &label } else { "" })?;
        if label.is_empty() { Ok(row) } else { crate::identity::rename(reg, &row.id, &label) }
    })?;
    records::lay_seats(&row)?;
    Ok(row)
}

pub fn label_free(v: &Vault, seat: Seat, label: &str, except: Option<&str>) -> R<()> {
    let want = label.trim();
    if want.is_empty() {
        return Ok(());
    }
    let mine = except.and_then(|a| v.identities.iter().find(|i| i.address.eq_ignore_ascii_case(a))).map(|i| i.id.clone());
    let taken = v.identities.iter().find(|i| i.seat == seat && mine.as_deref() != Some(i.id.as_str()) && i.label.trim().eq_ignore_ascii_case(want));
    match taken {
        Some(i) => Err(Fault::from(crate::fault::Fault::known(crate::fault::Known::IdentityNameTaken, format!("{want} · {}", i.address)))),
        None => Ok(()),
    }
}


pub fn here_as(i: &Identity) -> String {
    let label = if i.label.trim().is_empty() { "(未起名)" } else { i.label.trim() };
    format!("{} 已在本机,本机标签「{label}」", i.address)
}

pub fn adopt(data: &str, seat: Seat, address: &str) -> R<()> {
    records::bind(data)?;
    if crate::register::now_row_listed()?.is_some() {
        return Ok(());
    }
    let (row, _) = records::seat_of(address)?;
    crate::register::change(|reg| crate::identity::switch(reg, &row.id, role_of(seat)))?;
    Ok(())
}

pub fn remove(s: &Session, address: &str) -> R<Option<String>> {
    let v = s.vault()?;
    let one = find(&v, address)?;
    let (id, seat) = (one.id.clone(), one.seat);
    crate::register::change(|reg| crate::identity::delete(reg, &id, false))?;
    active(s.data(), seat)
}

pub fn new_primary(s: &mut Session, seat: Seat, label: &str, named: bool, phrase: Option<&str>, passcode: &str, now: u64) -> R<(String, Option<String>, Vec<String>)> {
    let was = crate::register::read()?.and_then(|r| r.current.clone());
    let (address, words) = match phrase {
        Some(p) => (import_words_as(s, seat, label, named, p, now)?, None),
        None => new_words_as(s, seat, label, named, now).map(|(a, w)| (a, Some(w)))?,
    };
    let id = find(&s.vault()?, &address)?.id.clone();
    if let Err(e) = crate::rekey::set_primary(&id, passcode) {
        let undone = crate::register::change(|reg| {
            crate::identity::delete(reg, &id, true)?;
            if let Some(c) = was.clone().filter(|c| reg.find(&c.0).is_some()) {
                reg.current = Some(c);
            }
            Ok(())
        });
        let e: Fault = e.into();
        return Err(match undone {
            Ok(_) => e,
            Err(u) => Fault::Vault(format!("{}:{};新身份没撤回:{}", e.name(), e.detail(), u.raw())),
        });
    }
    let mut trouble: Vec<String> = Vec::new();
    if let Err(e) = s.opened() {
        trouble.push(format!("{}:{}", e.name(), e.detail()));
    }
    Ok((address, words, trouble))
}

pub fn who(data: &str, seat: Seat) -> R<String> {
    if let Some(a) = active(data, seat)? {
        return Ok(a);
    }
    let v = vault::read(data)?;
    primary_of(&v, seat)
        .or_else(|| list(&v, seat).into_iter().next())
        .map(|i| i.address.clone())
        .ok_or_else(|| Fault::NoIdentity(format!("{}席上还没有钥", word(seat))))
}

pub fn rows(v: &Vault, seat: Seat, active: Option<&str>) -> Vec<eravon_literals::json::Out> {
    use eravon_literals::json::Out;
    list(v, seat)
        .into_iter()
        .map(|i| {
            Out::O(vec![
                ("address", Out::S(i.address.clone())),
                ("createdAt", Out::S(i.created.clone())),
                ("current", Out::B(active.is_some_and(|a| a.eq_ignore_ascii_case(&i.address)))),
                ("kind", Out::S(i.kind.word().to_string())),
                ("label", Out::S(i.label.clone())),
                ("primary", Out::B(i.primary)),
            ])
        })
        .collect()
}

fn in_office(key: &str) -> R<(crate::identity::Row, Seat)> {
    let (row, role) = records::seat_of(key)?;
    if !row.current(role).is_some_and(|a| a.hex().eq_ignore_ascii_case(key)) {
        return Err(Fault::NoIdentity(format!("{key} 不是这一席的在任钥")));
    }
    Ok((row, crate::vault::seat_of(role)))
}

pub fn next_words(s: &Session, key: &str) -> R<(String, Option<String>)> {
    crate::session::need_open(s)?;
    records::bind(s.data())?;
    let (row, seat) = in_office(key)?;
    if row.kind() == crate::identity::Kind::Existing {
        return next_key(&row, seat).map(|k| (k, None));
    }
    let id = row.id.clone();
    let pending = crate::places::seed_next_slot(&id);
    if let Some(mut raw) = crate::keybox::get(&pending)? {
        let words = <[u8; crate::family::ENTROPY_BYTES]>::try_from(raw.as_slice()).ok().map(|e| crate::cryptx::phrase_of(&e));
        eravon_os::wipe(&mut raw);
        if let Some(mut phrase) = words {
            let f = crate::identity::from_words(&phrase)?;
            let made = row.seats().into_iter().all(|r| row.office(r).is_some_and(|o| f.address(r).is_some_and(|a| o.next == Some(a.0) || o.current == a.0)));
            if made {
                let next = f.address(role_of(seat)).map(|a| a.hex()).ok_or_else(|| Fault::Vault("这一段助记词派生不出这一席的钥".into()))?;
                return Ok((next, Some(phrase)));
            }
            eravon_os::wipe_string(&mut phrase);
        }
    }
    let f = crate::identity::fresh()?;
    let mut nexts: Vec<(crate::roles::Role, crate::key::Address)> = Vec::new();
    for r in row.seats() {
        let k = crate::key::derived(f.entropy(), r).ok_or_else(|| Fault::Vault("新助记词派生不出这一席的钥".into()))?;
        let a = k.address().ok_or_else(|| Fault::Vault("新助记词派生不出这一席的钥".into()))?;
        crate::key::install_at(&crate::places::key_slot(&a), &k)?;
        nexts.push((r, a));
    }
    crate::keybox::drop_item(&pending)?;
    crate::keybox::put(&pending, f.entropy())?;
    crate::register::change(|reg| {
        for (r, a) in &nexts {
            crate::identity::set_next(reg, &id, *r, Some(*a))?;
        }
        Ok(())
    })?;
    let next = nexts.iter().find(|(r, _)| *r == role_of(seat)).map(|(_, a)| a.hex()).ok_or_else(|| Fault::Vault("新助记词派生不出这一席的钥".into()))?;
    let phrase = f.words().iter().map(|w| w.expose().to_string()).collect::<Vec<_>>().join(" ");
    Ok((next, Some(phrase)))
}

fn next_key(row: &crate::identity::Row, seat: Seat) -> R<String> {
    let role = role_of(seat);
    if let Some(n) = row.office(role).and_then(|o| o.next) {
        let n = crate::key::Address(n);
        if crate::keybox::present(&crate::places::key_slot(&n))? {
            return Ok(n.hex());
        }
    }
    let k = crate::key::generate()?;
    let a = k.address().ok_or_else(|| Fault::Vault("新钥派生不出地址".into()))?;
    crate::key::install_at(&crate::places::key_slot(&a), &k)?;
    let id = row.id.clone();
    crate::register::change(|reg| crate::identity::set_next(reg, &id, role, Some(a)).map(|_| ()))?;
    Ok(a.hex())
}

pub fn next_of(data: &str, key: &str) -> Option<String> {
    records::bind(data).ok()?;
    let (row, seat) = in_office(key).ok()?;
    row.office(role_of(seat))?.next.map(|n| crate::key::Address(n).hex())
}

pub fn rotation_left(data: &str, key: &str) -> R<bool> {
    records::bind(data)?;
    let reg = crate::register::read()?.unwrap_or_default();
    let Some(want) = crate::key::Address::parse(key) else { return Ok(false) };
    let Some((row, _)) = records::row_of_key(&reg, want) else { return Ok(false) };
    if row.seats().into_iter().any(|r| row.office(r).is_some_and(|o| o.next.is_some())) {
        return Ok(true);
    }
    Ok(row.kind() == crate::identity::Kind::Words && crate::keybox::present(&crate::places::seed_next_slot(&row.id))?)
}

pub fn other_seat(data: &str, key: &str) -> Option<String> {
    records::bind(data).ok()?;
    let (row, role) = records::seat_of(key).ok()?;
    row.current(role.other()).map(|a| a.hex())
}

pub fn rotation_pairs(data: &str, old: &str, new: &str) -> R<Vec<(Seat, String, String)>> {
    records::bind(data)?;
    let (ro, _) = records::seat_of(old)?;
    let (rn, sn) = records::seat_of(new)?;
    let hex = |a: [u8; 20]| crate::key::Address(a).hex();
    let mut out = Vec::new();
    if ro.id.eq_ignore_ascii_case(&rn.id) {
        let taking = rn.office(sn).and_then(|o| o.next).is_some_and(|n| hex(n).eq_ignore_ascii_case(new));
        for r in rn.seats() {
            let (Some(o), Some(root)) = (rn.office(r), rn.address(r)) else { continue };
            let pair = match (taking, o.next) {
                (true, Some(n)) => Some((hex(o.current), hex(n))),
                (true, None) => None,
                (false, Some(n)) => Some((hex(o.current), hex(n))),
                (false, _) if o.generation == 0 => None,
                (false, _) => Some((if o.generation == 1 { root.hex() } else { o.former.last().copied().map(hex).unwrap_or_else(|| root.hex()) }, hex(o.current))),
            };
            if let Some((a, b)) = pair {
                out.push((crate::vault::seat_of(r), a, b));
            }
        }
        return Ok(out);
    }
    for r in crate::roles::Role::ALL {
        if let (Some(a), Some(b)) = (ro.current(r), rn.current(r)) {
            out.push((crate::vault::seat_of(r), a.hex(), b.hex()));
        }
    }
    Ok(out)
}

pub fn hand_over(data: &str, key: &str) -> R<()> {
    hand_over_but(data, key, &[])
}

pub fn hand_over_but(data: &str, key: &str, wait: &[Seat]) -> R<()> {
    records::bind(data)?;
    let (row, _) = records::seat_of(key)?;
    let id = row.id.clone();
    let waiting = row.kind() == crate::identity::Kind::Words && row.seats().into_iter().any(|r| row.office(r).is_some_and(|o| o.next.is_some()));
    let words_here = crate::keybox::present(&crate::places::seed_next_slot(&id))?;
    let held: Vec<crate::roles::Role> = wait.iter().map(|s| role_of(*s)).collect();
    crate::register::change(|reg| {
        let Some(now) = reg.find(&id).cloned() else { return Ok(()) };
        for r in now.seats() {
            if now.office(r).and_then(|o| o.next).is_some() && !held.contains(&r) {
                crate::identity::take_office(reg, &id, r)?;
            }
        }
        Ok(())
    })?;
    if waiting && !words_here {
        return Err(Fault::Down {
            token: E_V_ROTATE_WORDS_GONE,
            detail: format!(
                "身份 {id} 的各席已交给新钥,可这一趟换钥造的新助记词不在这台机器上(数据是换钥半路做的整机备份恢复来的):这一身份的助记词与恢复封仍是换钥之前的词,它们派生不出在任钥。在做那份备份的机器上把这一趟换钥做完,再做一次整机备份恢复过来"
            ),
        });
    }
    words_settled(&id).map(|_| ())
}

pub fn backup_again(data: &str, key: &str) -> bool {
    records::bind(data).is_ok()
        && records::seat_of(key).is_ok_and(|(row, role)| row.kind() == crate::identity::Kind::Existing && row.office(role).is_some_and(|o| o.generation > 0) && !row.backed())
}

pub const E_V_ROTATE_WORDS_GONE: &str = "E_V_ROTATE_WORDS_GONE";

pub fn waiting_other(data: &str, key: &str) -> Option<String> {
    records::bind(data).ok()?;
    let (row, role) = records::seat_of(key).ok()?;
    let mine = row.office(role)?;
    if mine.next.is_some() || mine.generation == 0 {
        return None;
    }
    let other = row.seats().into_iter().any(|r| r != role && row.office(r).is_some_and(|o| o.next.is_some()));
    if !other {
        return None;
    }
    let before = if mine.generation == 1 { row.address(role)?.0 } else { *mine.former.last()? };
    Some(crate::key::Address(before).hex())
}

pub fn settle_words(data: &str, key: &str) -> R<bool> {
    records::bind(data)?;
    let (row, _) = records::seat_of(key)?;
    words_settled(&row.id)
}

pub fn same_row(data: &str, a: &str, b: &str) -> bool {
    records::bind(data).is_ok() && matches!((records::seat_of(a), records::seat_of(b)), (Ok((x, _)), Ok((y, _))) if x.id.eq_ignore_ascii_case(&y.id))
}

pub fn words_settled(id: &str) -> R<bool> {
    let pending = crate::places::seed_next_slot(id);
    let Some(mut raw) = crate::keybox::get(&pending)? else { return Ok(false) };
    let got = <[u8; crate::family::ENTROPY_BYTES]>::try_from(raw.as_slice());
    eravon_os::wipe(&mut raw);
    let Ok(mut e) = got else { return Err(Fault::Vault(format!("{pending} 里的助记词读不成"))) };
    let reg = crate::register::read()?.unwrap_or_default();
    let Some(row) = reg.find(id).cloned() else {
        eravon_os::wipe(&mut e);
        return Err(Fault::NoIdentity(id.to_string()));
    };
    let on = row.kind() == crate::identity::Kind::Words && row.seats().into_iter().all(|r| crate::key::derived(&e, r).and_then(|k| k.address()) == row.current(r));
    if !on {
        eravon_os::wipe(&mut e);
        return Ok(false);
    }
    let seed = crate::places::seed_slot(id);
    let mut before = crate::keybox::get(&seed)?;
    let primary = reseal_owed(id)?;
    let put_back = |before: &Option<Vec<u8>>| -> Result<(), crate::fault::Fault> {
        crate::keybox::drop_item(&seed)?;
        if let Some(b) = before {
            crate::keybox::put(&seed, b)?;
        }
        Ok(())
    };
    if let Some(kind) = primary {
        crate::keybox::put(&reseal_slot(id), kind.as_str().as_bytes())?;
    }
    crate::keybox::drop_item(&seed)?;
    crate::keybox::put(&seed, &e)?;
    if let Some(kind) = primary {
        let sealed = crate::keybox::drop_recovery(id).and_then(|()| reseal_hook()).and_then(|()| crate::keybox::add_recovery(id, crate::keybox::PrimaryKind::Words, &e));
        if !matches!(sealed, Ok(true)) {
            let why = match &sealed {
                Ok(_) => "恢复封没有落下".to_string(),
                Err(f) => format!("{}:{}", f.said(), f.tail()),
            };
            let seal_back = match kind {
                crate::keybox::PrimaryKind::Words => before.as_deref().map(|b| crate::keybox::add_recovery(id, kind, b).map(|_| ())).unwrap_or(Ok(())),
                _ => match row.keys.seats().first().and_then(|r| row.address(*r)) {
                    Some(root) => match crate::key::load_at(&crate::places::key_slot(&root)) {
                        Ok(Some(k)) => crate::key::seal_recovery(id, &k).map(|_| ()),
                        Ok(None) => Ok(()),
                        Err(f) => Err(f),
                    },
                    None => Ok(()),
                },
            };
            let back = put_back(&before).and(seal_back);
            if let Some(b) = before.as_mut() {
                eravon_os::wipe(b);
            }
            eravon_os::wipe(&mut e);
            return Err(Fault::Vault(match back {
                Ok(()) => format!("新助记词的恢复封没封成,种子与恢复封已放回原样,换钥没做完、再跑一次即接着做:{why}"),
                Err(b) => format!("新助记词的恢复封没封成({why}),放回原样也没成:{}:{}", b.said(), b.tail()),
            }));
        }
    }
    if let Some(b) = before.as_mut() {
        eravon_os::wipe(b);
    }
    eravon_os::wipe(&mut e);
    crate::register::change(|reg| {
        if let Some(r) = reg.rows.iter_mut().find(|r| r.id.eq_ignore_ascii_case(id)) {
            r.backed_words = false;
        }
        Ok(())
    })?;
    crate::keybox::drop_item(&reseal_slot(id))?;
    crate::keybox::drop_item(&pending)?;
    Ok(true)
}

fn reseal_slot(id: &str) -> String {
    format!("{}-reseal", crate::places::seed_next_slot(id))
}

fn reseal_owed(id: &str) -> R<Option<crate::keybox::PrimaryKind>> {
    let slot = reseal_slot(id);
    match crate::keybox::primary()? {
        Some((p, k)) if p.eq_ignore_ascii_case(id) => Ok(Some(k)),
        Some(_) => {
            crate::keybox::drop_item(&slot)?;
            Ok(None)
        }
        None => Ok(crate::keybox::get(&slot)?.and_then(|b| match b.as_slice() {
            b"words" => Some(crate::keybox::PrimaryKind::Words),
            b"key-file" => Some(crate::keybox::PrimaryKind::KeyFile),
            _ => None,
        })),
    }
}

#[cfg(not(test))]
fn reseal_hook() -> Result<(), crate::fault::Fault> {
    Ok(())
}

#[cfg(test)]
fn reseal_hook() -> Result<(), crate::fault::Fault> {
    if tests::RESEAL_FAILS.with(|f| f.replace(false)) {
        return Err(crate::fault::Fault::known(crate::fault::Known::KeyboxShape, "test: the new seal's write fails"));
    }
    Ok(())
}

pub const VERBS: [&str; 6] =
    [seam::V_NEW_KEY, seam::V_IMPORT, seam::V_SWITCH, seam::V_REMOVE, seam::V_RESET, seam::V_PHRASE];

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "48261537";

    fn family() -> (String, Session, String) {
        let data = crate::testing::fresh();
        let mut b = vault::create(PIN, Seat::Seller, "主钥", None, 900).expect("a vault");
        b.write(&data).expect("written");
        let mut s = Session::locked(&data);
        s.unlock(PIN).expect("opens");
        (data, s, b.address.clone())
    }

    #[test]
    fn a_label_taken_in_the_seat_is_refused_and_nothing_is_written() {
        let (_d, s, _) = family();
        let before = s.vault().unwrap();
        assert_eq!(new_words_as(&s, Seat::Seller, "主钥", true, 901).map(|_| ()).unwrap_err().name(), "IDENTITY_NAME_TAKEN");
        assert_eq!(s.vault().unwrap(), before);
    }

    #[test]
    fn the_primary_cannot_be_deleted_and_a_secondary_can() {
        let (d, s, primary) = family();
        assert_eq!(remove(&s, &primary).unwrap_err().name(), "PRIMARY_DELETE");
        let (second, _) = new_words_as(&s, Seat::Seller, "二号", true, 901).expect("a second identity");
        assert_eq!(switch(&d, Seat::Seller, &second).expect("switched"), second);
        assert_eq!(active(&d, Seat::Seller).unwrap().as_deref(), Some(second.as_str()));
        remove(&s, &second).expect("a secondary identity goes");
        assert!(find(&s.vault().unwrap(), &second).is_err());
    }

    #[test]
    fn a_key_from_the_other_seat_cannot_be_switched_to() {
        let (d, s, _) = family();
        let buyer = list(&s.vault().unwrap(), Seat::Buyer)[0].address.clone();
        assert!(matches!(switch(&d, Seat::Seller, &buyer), Err(Fault::NoIdentity(_))));
    }

    thread_local! {
        pub(super) static RESEAL_FAILS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    const PHRASE: &str = "test test test test test test test test test test test junk";
    const NEW_PIN: &str = "73915264";

    fn taken_office_on_new_words() -> (String, String, String, String, Session) {
        let data = crate::testing::fresh();
        let old = crate::onboard::land(&data, crate::onboard::Mode::First, Seat::Seller, "主钥", false, crate::onboard::Source::Words(PHRASE), PIN, None, None, 900).expect("首启").address;
        let mut s = Session::locked(&data);
        s.unlock(PIN).expect("opens");
        let (new, phrase) = next_words(&s, &old).expect("new words made ready");
        let phrase = phrase.expect("a row of words gets new words");
        let (row, _) = records::seat_of(&new).expect("the row");
        crate::register::change(|reg| {
            for r in row.seats() {
                crate::identity::take_office(reg, &row.id, r)?;
            }
            Ok(())
        })
        .expect("in office");
        (data, row.id, new, phrase, s)
    }

    fn recovers(data: &str, phrase: &str) -> bool {
        crate::vault::recover(data, phrase, NEW_PIN, NEW_PIN).is_ok()
    }

    #[test]
    fn a_run_after_the_old_seal_was_dropped_and_the_new_one_not_landed_seals_anew() {
        let (data, id, new, phrase, _s) = taken_office_on_new_words();
        let pending = crate::places::seed_next_slot(&id);
        let e = crate::keybox::get(&pending).expect("read").expect("pending words");
        assert_eq!(reseal_owed(&id).expect("read"), Some(crate::keybox::PrimaryKind::Words));
        crate::keybox::put(&reseal_slot(&id), b"words").expect("mark");
        crate::keybox::drop_item(&crate::places::seed_slot(&id)).expect("seed out");
        crate::keybox::put(&crate::places::seed_slot(&id), &e).expect("seed in");
        crate::keybox::drop_recovery(&id).expect("old seal dropped");
        assert_eq!(crate::keybox::primary().expect("read"), None, "the vault itself names no primary now");
        assert!(settle_words(&data, &new).expect("the run again"), "the words settle");
        assert_eq!(crate::keybox::primary().expect("read").map(|(p, k)| (p.to_ascii_lowercase(), k)), Some((id.to_ascii_lowercase(), crate::keybox::PrimaryKind::Words)));
        assert!(!crate::keybox::present(&reseal_slot(&id)).expect("read") && !crate::keybox::present(&pending).expect("read"), "the slot and the pending words go");
        assert!(!recovers(&data, PHRASE), "the old words recover no more");
        assert!(recovers(&data, &phrase), "the new words recover");
    }

    #[test]
    fn keys_taking_office_without_their_words_here_are_said_by_name() {
        let data = crate::testing::fresh();
        let old = crate::onboard::land(&data, crate::onboard::Mode::First, Seat::Seller, "主钥", false, crate::onboard::Source::Words(PHRASE), PIN, None, None, 900).expect("首启").address;
        let mut s = Session::locked(&data);
        s.unlock(PIN).expect("opens");
        let (new, _) = next_words(&s, &old).expect("new words made ready");
        let (row, _) = records::seat_of(&new).expect("the row");
        crate::keybox::drop_item(&crate::places::seed_next_slot(&row.id)).expect("words gone");
        let e = hand_over(&data, &new).expect_err("said by name");
        assert_eq!(e.name(), E_V_ROTATE_WORDS_GONE, "{}", e.detail());
        assert_eq!(active(&data, Seat::Seller).expect("read").as_deref(), Some(new.to_ascii_lowercase().as_str()), "the new key took office");
        assert!(recovers(&data, PHRASE), "the seal is the words before");
    }

    #[test]
    fn a_new_seal_that_fails_puts_seed_and_seal_back_and_the_run_again_finishes() {
        let (data, id, new, phrase, _s) = taken_office_on_new_words();
        let seed_before = crate::keybox::get(&crate::places::seed_slot(&id)).expect("read");
        RESEAL_FAILS.with(|f| f.set(true));
        let e = settle_words(&data, &new).expect_err("the seal does not land");
        assert!(e.detail().contains("已放回原样"), "said: {}", e.detail());
        assert_eq!(crate::keybox::get(&crate::places::seed_slot(&id)).expect("read"), seed_before, "the seed as it was");
        assert!(crate::keybox::present(&crate::places::seed_next_slot(&id)).expect("read"), "the pending words stay");
        assert!(crate::keybox::has_recovery(&id).expect("read"), "the seal back");
        assert!(recovers(&data, PHRASE), "the old words still recover");
        assert!(settle_words(&data, &new).expect("the run again"), "the run again finishes");
        assert!(recovers(&data, &phrase), "the new words recover");
        assert!(!recovers(&data, PHRASE), "the old words no more");
    }

    #[test]
    fn a_run_that_stopped_before_the_seed_changed_is_finished_and_the_words_are_unconfirmed() {
        let (data, id, new, phrase, _s) = taken_office_on_new_words();
        crate::register::change(|reg| crate::identity::mark(reg, &id, true, false, None)).expect("the words before confirmed");
        assert!(recovers(&data, PHRASE), "the old words still recover before the run again");
        assert!(settle_words(&data, &new).expect("the run again"));
        let reg = crate::register::read().expect("read").expect("register");
        assert!(!reg.find(&id).expect("row").backed_words, "the new words are nobody's confirmed copy");
        assert!(!recovers(&data, PHRASE));
        assert!(recovers(&data, &phrase));
    }

    #[test]
    fn a_change_left_part_way_reads_from_the_register_and_the_vault() {
        let data = crate::testing::fresh();
        let old = crate::onboard::land(&data, crate::onboard::Mode::First, Seat::Seller, "主钥", false, crate::onboard::Source::Words(PHRASE), PIN, None, None, 900).expect("首启").address;
        let mut s = Session::locked(&data);
        s.unlock(PIN).expect("opens");
        assert!(!rotation_left(&data, &old).expect("read"), "nothing begun");
        let (new, _) = next_words(&s, &old).expect("new words made ready");
        assert!(rotation_left(&data, &old).expect("read") && rotation_left(&data, &new).expect("read"), "a key made ready next, not in office");
        let (row, _) = records::seat_of(&new).expect("the row");
        let seats = row.seats();
        assert_eq!(seats.len(), 2);
        crate::register::change(|reg| crate::identity::take_office(reg, &row.id, seats[0])).expect("one seat in office");
        assert!(rotation_left(&data, &old).expect("read"), "one seat in office, the other not");
        crate::register::change(|reg| crate::identity::take_office(reg, &row.id, seats[1])).expect("the other seat in office");
        assert!(rotation_left(&data, &new).expect("read"), "every seat in office, the words pending");
        assert!(settle_words(&data, &new).expect("the words settle"));
        assert!(!rotation_left(&data, &new).expect("read") && !rotation_left(&data, &old).expect("read"), "done");
        assert!(!rotation_left(&data, "0x00000000000000000000000000000000000000a1").expect("read"), "no row here");
    }

    const FILE_PIN: &str = "file-pass-0451";

    fn key_file_primary_changed() -> (String, String, String, String, Vec<u8>, Session) {
        let (data, s, _) = family();
        let single = crate::add::made(&s, Seat::Seller, "单钥", true).expect("a key made here");
        let path = crate::restore::export_to(&s, PIN, &single, FILE_PIN, FILE_PIN, &format!("{data}/kf")).expect("its key file");
        let file = std::fs::read(&path).expect("the file");
        let (row, _) = records::seat_of(&single).expect("the row");
        crate::rekey::set_primary(&row.id, PIN).expect("made primary");
        assert_eq!(crate::keybox::primary().expect("read").map(|(p, k)| (p.to_ascii_lowercase(), k)), Some((row.id.to_ascii_lowercase(), crate::keybox::PrimaryKind::KeyFile)), "the primary's seal is its key file's");
        let mut s = Session::locked(&data);
        s.unlock(PIN).expect("opens");
        let (new, words) = next_words(&s, &single).expect("a next key made ready");
        assert!(words.is_none(), "an adopted key's change brings no words");
        assert_eq!(next_words(&s, &single).expect("the same run again").0, new, "a run again makes no second key");
        assert!(rotation_left(&data, &single).expect("read"), "a key made ready next, not in office");
        hand_over(&data, &single).expect("handed over");
        (data, row.id, single, new, file, s)
    }

    fn file_recovers(data: &str, file: &[u8]) -> bool {
        crate::restore::rebuild(data, file, FILE_PIN, NEW_PIN, NEW_PIN, Seat::Seller, "单钥", 0).is_ok()
    }

    #[test]
    fn an_adopted_key_changes_to_a_new_single_key_and_its_row_never_holds_words() {
        let (data, id, old, new, _file, s) = key_file_primary_changed();
        let reg = crate::register::read().expect("read").expect("register");
        let row = reg.find(&id).expect("row").clone();
        assert_eq!(row.kind(), crate::identity::Kind::Existing);
        assert_eq!(row.current(crate::roles::Role::Author).map(|a| a.hex()), Some(new.to_ascii_lowercase()));
        assert!(!row.backed(), "the key file exported before backs up the key in office no more");
        for slot in [crate::places::seed_slot(&id), crate::places::seed_next_slot(&id)] {
            assert!(!crate::keybox::present(&slot).expect("read"), "{slot}: no seed in an adopted key's row");
        }
        assert!(!rotation_left(&data, &new).expect("read") && !rotation_left(&data, &old).expect("read"), "done");
        assert!(!settle_words(&data, &new).expect("read"), "no words to settle");
        assert!(s.phrase_for(&new).is_err(), "the key in office has no words to show");
    }

    #[test]
    fn a_key_file_primary_changed_keeps_its_seal() {
        let (data, id, _old, _new, file, _s) = key_file_primary_changed();
        assert_eq!(crate::keybox::primary().expect("read").map(|(p, k)| (p.to_ascii_lowercase(), k)), Some((id.to_ascii_lowercase(), crate::keybox::PrimaryKind::KeyFile)));
        assert!(file_recovers(&data, &file), "the key before's file recovers");
    }
}
