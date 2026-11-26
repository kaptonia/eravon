use crate::fault::{Fault, Known};
use crate::local::Doc;
use crate::roles::Role;
use eravon_store::record::Kind;
use eravon_store::seam::Box_;
use std::path::{Path, PathBuf};

pub const GLOBAL_FILE: &str = "settings.json";

pub const BOXES: [&str; 4] = ["products", "orders", "cases", "packs"];

pub const RECORD: &str = "record";

pub const KINDS: [(&str, Doc); 10] = [
    ("product", Doc::Product),
    ("draft", Doc::Draft),
    ("terms", Doc::TermsOriginal),
    ("offer", Doc::OfferSignature),
    ("payload", Doc::Payload),
    ("receipt", Doc::Receipt),
    ("order", Doc::OrderArchive),
    ("label", Doc::OrderLabel),
    ("tx", Doc::TxReceipt),
    ("read", Doc::NoticeRead),
];

pub const SINGLES: [(&str, Doc); 3] = [("identity.json", Doc::IdentitySettings), ("candidates.json", Doc::Candidates), ("reading.json", Doc::Reading)];

pub const RETIRED: [&str; 1] = [eravon_literals::literals::LOCAL_DATA_KINDS[14]];

pub fn retired(tag: &str) -> bool {
    RETIRED.contains(&tag)
}

pub fn word_of(d: Doc) -> Option<&'static str> {
    KINDS.iter().find(|(_, k)| *k == d).map(|(w, _)| *w)
}

pub fn single_of(d: Doc) -> Option<&'static str> {
    SINGLES.iter().find(|(_, k)| *k == d).map(|(n, _)| *n)
}

pub fn counted(d: Doc) -> bool {
    word_of(d).is_some() || matches!(d, Doc::Candidates)
}

pub fn machine_doc(rel: &str) -> Option<Doc> {
    (rel == GLOBAL_FILE).then_some(Doc::GlobalSettings)
}

pub fn home_doc(rel: &str) -> Option<Doc> {
    let kind = |file: &str| file.rsplit_once('.').and_then(|(_, w)| KINDS.iter().find(|(k, _)| *k == w)).map(|(_, d)| *d);
    match rel.split('/').collect::<Vec<_>>().as_slice() {
        [file] => SINGLES.iter().find(|(n, _)| n == file).map(|(_, d)| *d),
        [b, _, file] if BOXES.contains(b) => kind(file),
        _ => None,
    }
}

pub fn of_record(k: Kind) -> Doc {
    match k {
        Kind::Product => Doc::Product,
        Kind::TermsDocument => Doc::TermsOriginal,
        Kind::OfferSignature => Doc::OfferSignature,
        Kind::Payload => Doc::Payload,
        Kind::Receipt => Doc::Receipt,
        Kind::OrderFile | Kind::Dossier | Kind::DeliveryListing => Doc::OrderArchive,
        Kind::Candidates => Doc::Candidates,
        Kind::OrderLabel => Doc::OrderLabel,
        Kind::TxReceipt => Doc::TxReceipt,
        Kind::NotificationRead => Doc::NoticeRead,
    }
}

pub fn of_part(k: Kind) -> Doc {
    match k {
        Kind::Product => Doc::Draft,
        other => of_record(other),
    }
}

pub fn bind(data: &str) -> Result<PathBuf, Fault> {
    let at = PathBuf::from(data);
    crate::places::set(crate::places::Places { key_account: crate::places::ACCOUNT.to_string(), machine_dir: Some(at.clone()), user_home: None });
    let now = crate::home::machine_dir()?;
    if !eravon_store::writer::same_place(&now, &at) {
        return Err(Fault::known(Known::HomeUnreachable, format!("{data} · {}", now.display())));
    }
    Ok(now)
}

pub fn seat_of(addr: &str) -> Result<(crate::identity::Row, Role), Fault> {
    let want = crate::key::Address::parse(addr).ok_or_else(|| Fault::known(Known::NoIdentity, addr.to_string()))?;
    let reg = crate::register::read()?.unwrap_or_default();
    row_of_key(&reg, want).ok_or_else(|| Fault::known(Known::NoIdentity, addr.to_string()))
}

pub fn row_of_key(reg: &crate::identity::Registry, want: crate::key::Address) -> Option<(crate::identity::Row, Role)> {
    let find = |names: &dyn Fn(&crate::identity::Row, Role) -> bool| reg.rows.iter().find_map(|row| Role::ALL.into_iter().find(|s| names(row, *s)).map(|s| (row.clone(), s)));
    find(&|row, s| row.address(s) == Some(want) || row.current(s) == Some(want))
        .or_else(|| find(&|row, s| row.office(s).is_some_and(|o| o.former.contains(&want.0) || o.next == Some(want.0))))
}


pub fn lay_seats(row: &crate::identity::Row) -> Result<(), Fault> {
    for seat in row.seats() {
        if let Some(h) = row.home(seat) {
            crate::home::Home::open_or_create(h)?;
        }
    }
    Ok(())
}

pub fn store_dir(addr: &str) -> Result<PathBuf, Fault> {
    let (row, seat) = seat_of(addr)?;
    let home = row.home(seat).ok_or_else(|| Fault::known(Known::SeatUnseated, addr.to_string()))?;
    Ok(home.join(crate::home::Slot::Store.as_str()))
}

pub fn box_dir(addr: &str, b: Box_) -> Result<PathBuf, Fault> {
    Ok(store_dir(addr)?.join(b.dir()))
}

pub fn record_name(b: Box_, id: &str) -> Result<String, Fault> {
    Ok(name_under(&crate::names::key()?, b, id))
}

fn name_under(nk: &crate::names::NameKey, b: Box_, id: &str) -> String {
    nk.name(match b {
        Box_::Products => crate::names::Logical::Products(id),
        Box_::Orders => crate::names::Logical::Orders(id),
        Box_::Cases => crate::names::Logical::Cases(id),
        Box_::Packs => crate::names::Logical::Packs(id),
    })
}

pub fn rel_under(nk: &crate::names::NameKey, rel: &str, envelope: impl FnOnce(&str) -> Result<Option<Vec<u8>>, Fault>) -> Result<Option<String>, Fault> {
    Ok(in_box(rel, envelope)?.map(|(b, id, file)| format!("{}/{}/{}/{file}", crate::home::Slot::Store.as_str(), b.dir(), name_under(nk, b, &id))))
}

pub fn logical_under(rel: &str, envelope: impl FnOnce(&str) -> Result<Option<Vec<u8>>, Fault>) -> Result<Option<String>, Fault> {
    Ok(in_box(rel, envelope)?.map(|(b, id, file)| format!("{}/{}/{id}/{file}", crate::home::Slot::Store.as_str(), b.dir())))
}

fn in_box(rel: &str, envelope: impl FnOnce(&str) -> Result<Option<Vec<u8>>, Fault>) -> Result<Option<(Box_, String, String)>, Fault> {
    let store = crate::home::Slot::Store.as_str();
    let Some(inner) = rel.strip_prefix(store).and_then(|r| r.strip_prefix('/')) else { return Ok(None) };
    let parts: Vec<&str> = inner.split('/').collect();
    let [b, _, file] = parts.as_slice() else { return Ok(None) };
    let Some(b) = Box_::from_dir(b) else { return Ok(None) };
    if home_doc(inner).is_none() {
        return Ok(None);
    }
    let shape = || Fault::known(Known::LocalSeal, rel.to_string());
    let plain = envelope(&envelope_rel(rel).ok_or_else(shape)?)?.ok_or_else(shape)?;
    let id = eravon_store::record::Record::open(&plain).map_err(|e| Fault::known(Known::LocalSeal, format!("{rel} · {}", e.detail())))?.id;
    Ok(Some((b, id, file.to_string())))
}

pub fn envelope_rel(rel: &str) -> Option<String> {
    let (dir, _) = rel.rsplit_once('/')?;
    Some(format!("{dir}/{RECORD}."))
}

pub fn envelope_on_disk<'a>(at: &'a Path, key: &'a crate::keybox::LocalKey) -> impl FnOnce(&str) -> Result<Option<Vec<u8>>, Fault> + 'a {
    move |prefix: &str| {
        let Some(dir) = at.parent() else { return Ok(None) };
        let Ok(list) = std::fs::read_dir(dir) else { return Ok(None) };
        for e in list.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Some(word) = name.strip_prefix(&format!("{RECORD}.")) else { continue };
            let env_rel = format!("{prefix}{word}");
            let Some(doc) = crate::local::home_doc(&env_rel).filter(|_| !crate::local::is_staged(&name)) else { continue };
            let raw = std::fs::read(e.path()).map_err(|x| Fault::known(Known::LocalSeal, format!("{env_rel} · {x}")))?;
            return crate::local::open_with(key, &crate::local::expect_at(&e.path(), doc)?, &raw, &env_rel).map(Some);
        }
        Ok(None)
    }
}

pub fn record_dir(addr: &str, b: Box_, id: &str) -> Result<PathBuf, Fault> {
    Ok(box_dir(addr, b)?.join(record_name(b, id)?))
}

pub fn record_path(addr: &str, b: Box_, id: &str, kind: Kind) -> Result<PathBuf, Fault> {
    Ok(record_dir(addr, b, id)?.join(format!("{RECORD}.{}", word_of(of_record(kind)).unwrap_or_default())))
}

pub fn part_path(addr: &str, b: Box_, id: &str, part: &str, kind: Kind) -> Result<PathBuf, Fault> {
    Ok(record_dir(addr, b, id)?.join(format!("{part}.{}", word_of(of_part(kind)).unwrap_or_default())))
}

pub fn single_path(addr: &str, d: Doc) -> Result<PathBuf, Fault> {
    Ok(store_dir(addr)?.join(single_of(d).unwrap_or_default()))
}

pub fn global_path() -> Result<PathBuf, Fault> {
    Ok(crate::home::machine_dir()?.join(GLOBAL_FILE))
}

pub fn store_fault(f: Fault) -> eravon_store::fault::Fault {
    use eravon_store::fault::Fault as S;
    let said = format!("{}: {}", f.said(), f.tail());
    match f.which() {
        Some(Known::Locked) => S::NoKey(said),
        Some(Known::LocalSeal) => S::Sealed(said),
        Some(Known::NoIdentity | Known::SeatUnseated) => S::Missing(said),
        Some(Known::HomeUnreachable) => S::Root(said),
        _ => S::Io(said),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_known_by_any_key_of_its_seats() {
        use crate::identity::{Keys, Registry, Row, NO_BACKUP_AT, NO_CREATED, NO_LABEL};
        use eravon_seal::identities::Office;
        let a = |b: u8| crate::key::Address([b; 20]);
        let office = |current: u8, next: Option<u8>, former: &[u8]| Some(Office { generation: 1 + former.len() as u64, current: [current; 20], next: next.map(|n| [n; 20]), former: former.iter().map(|f| [*f; 20]).collect() });
        let row = |root: u8, grantee: u8, author_office: Option<Office>, grantee_office: Option<Office>| Row {
            id: a(root).hex(),
            keys: Keys::Both { author: a(root), grantee: a(grantee) },
            author_office,
            grantee_office,
            author_home: format!("/h/{root}/author"),
            grantee_home: format!("/h/{root}/grantee"),
            backed_words: true,
            backed_file: false,
            backup_at: NO_BACKUP_AT.to_string(),
            label: NO_LABEL.to_string(),
            created: NO_CREATED.to_string(),
            network: None,
            unread_network: None,
        };
        let reg = Registry { current: None, rows: vec![row(1, 5, office(3, Some(4), &[2]), office(6, None, &[])), row(9, 10, None, None)], left: Vec::new() };
        let seat = |b: u8| row_of_key(&reg, a(b)).map(|(r, s)| (r.id, s));
        for (key, want) in [(1, Role::Author), (2, Role::Author), (3, Role::Author), (4, Role::Author), (5, Role::Grantee), (6, Role::Grantee)] {
            assert_eq!(seat(key), Some((a(1).hex(), want)), "key {key}");
        }
        assert_eq!(seat(9), Some((a(9).hex(), Role::Author)), "a row of its own by its root");
        assert_eq!(seat(10), Some((a(9).hex(), Role::Grantee)));
        assert_eq!(seat(7), None, "a key of no row");
    }

    #[test]
    fn a_records_place_says_its_kind() {
        assert_eq!(home_doc("products/abc/record.product"), Some(Doc::Product));
        assert_eq!(home_doc("products/abc/body.draft"), Some(Doc::Draft));
        assert_eq!(home_doc("orders/abc/record.order"), Some(Doc::OrderArchive));
        assert_eq!(home_doc("queue.json"), None, "the retired send queue is no record");
        assert_eq!(home_doc("identity.json"), Some(Doc::IdentitySettings));
        assert_eq!(home_doc("shelf/abc/record.product"), None);
        assert_eq!(home_doc("products/abc/record.tmp"), None);
        assert_eq!(machine_doc(GLOBAL_FILE), Some(Doc::GlobalSettings));
        for (w, d) in KINDS {
            assert_eq!(word_of(d), Some(w));
        }
        for (n, d) in SINGLES {
            assert_eq!(single_of(d), Some(n));
        }
    }
}
