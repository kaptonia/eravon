use crate::vfault::R;
use crate::session::{Session, need_open};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Buyer {
    pub owners: Vec<String>,
    pub mine: Vec<String>,
    pub keys: Vec<String>,
}

pub fn buyer(s: &Session, me: &str) -> R<Buyer> {
    need_open(s)?;
    let owners = owners_of(s, me)?;
    let mut mine = owners.clone();
    mine.extend(former_of(s)?);
    let mine = dedup(mine);
    let mut keys = mine.clone();
    keys.extend(company_key(s.data(), me)?);
    Ok(Buyer { owners, mine, keys: dedup(keys) })
}

fn former_of(s: &Session) -> R<Vec<String>> {
    crate::records::bind(s.data())?;
    let reg = crate::register::read()?.unwrap_or_default();
    let seat = crate::roles::Role::Grantee;
    let mut out = Vec::new();
    for row in &reg.rows {
        let (Some(root), Some(o)) = (row.address(seat), row.office(seat)) else { continue };
        if root.0 != o.current {
            out.push(root.hex());
        }
        out.extend(o.former.iter().map(|f| crate::key::Address(*f).hex()));
    }
    Ok(out)
}

fn owners_of(s: &Session, me: &str) -> R<Vec<String>> {
    let mut owners: Vec<String> = s
        .vault()?
        .identities
        .iter()
        .filter(|i| i.seat == eravon_store::vault::Seat::Buyer)
        .map(|i| i.address.to_ascii_lowercase())
        .collect();
    if !me.is_empty() && !owners.iter().any(|o| o.eq_ignore_ascii_case(me)) {
        owners.insert(0, me.to_ascii_lowercase());
    }
    Ok(owners)
}


fn company_key(data: &str, me: &str) -> R<Option<String>> {
    Ok(crate::store::parties_of(data, me)?.into_iter().nth(1))
}


pub fn seller(data: &str, me: &str) -> R<Vec<String>> {
    Ok(dedup(seen(data, me)?))
}


fn seen(data: &str, me: &str) -> R<Vec<String>> {
    Ok(crate::store::seen_of(data, me)?)
}


fn dedup(keys: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for k in keys {
        let k = k.to_ascii_lowercase();
        if !out.contains(&k) {
            out.push(k);
        }
    }
    out
}
