use eravon_store::record::Candidate;

use crate::fault::{Fault, R};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Judge,
    Guild,
}

impl Side {
    pub const fn word(self) -> &'static str {
        match self {
            Side::Judge => "judge",
            Side::Guild => "guild",
        }
    }
    pub fn of(s: &str) -> Option<Side> {
        match s {
            "judge" => Some(Side::Judge),
            "guild" => Some(Side::Guild),
            _ => None,
        }
    }
}

pub fn target(side: Side, input: &str) -> R<String> {
    match side {
        Side::Judge => judge_address(input),
        Side::Guild => guild_address(input),
    }
}

fn judge_address(a: &str) -> R<String> {
    let ok = a.len() == 42 && a.starts_with("0x") && a[2..].bytes().all(|c| c.is_ascii_hexdigit());
    if !ok {
        return Err(Fault::Address(a.to_string()));
    }
    Ok(a.to_ascii_lowercase())
}

fn guild_address(p: &str) -> R<String> {
    let (chain, addr) = eravon_roster::adapter::pointer(p).ok_or_else(|| Fault::Pointer(p.to_string()))?;
    let want = eravon_roster::pool::standard_chain_id();
    if chain != want {
        return Err(Fault::Chain { want, got: chain });
    }
    Ok(addr.to_string())
}


pub fn pointer_of(addr: &str) -> String {
    eravon_roster::adapter::pointer_string(eravon_roster::pool::standard_chain_id(), addr)
}

pub fn pointer_hint() -> String {
    format!("{}…", pointer_of("0x"))
}

pub fn need_label(l: &str) -> R<String> {
    let t = l.trim();
    if t.is_empty() {
        return Err(Fault::Label);
    }
    Ok(t.to_string())
}


#[derive(Clone, Debug)]
pub struct Row {
    pub row: Candidate,
    pub flaw: Option<String>,
}

pub fn read(data: &str, me: &str) -> R<Vec<Row>> {
    let s = eravon_vault::store::open_read(data)?;
    let rows = match s.candidates_seen(me) {
        Ok(rows) => rows,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(rows.iter().map(|r| Row { row: r.clone(), flaw: flaw(r, &rows) }).collect())
}

pub fn list(data: &str, me: &str) -> R<Vec<Candidate>> {
    let rows: Vec<Candidate> = read(data, me)?.into_iter().map(|r| r.row).collect();
    match rows.iter().find(|r| Side::of(&r.kind).is_none()) {
        Some(bad) => Err(Fault::Kind(bad.kind.clone())),
        None => Ok(rows),
    }
}

fn flaw(r: &Candidate, all: &[Candidate]) -> Option<String> {
    let mut why = Vec::new();
    if Side::of(&r.kind).is_none() {
        why.push(format!("类是 {:?},不在 judge 与 guild 之内", r.kind));
    }
    if need_label(&r.label).is_err() {
        why.push("备注名为空".to_string());
    }
    if all.iter().filter(|o| o.address == r.address).count() > 1 {
        why.push("地址已存在".to_string());
    }
    (!why.is_empty()).then(|| why.join(";"))
}


pub fn add(data: &str, writer: &str, now: u64, me: &str, side: Side, input: &str, label: &str, seen: &str) -> R<Candidate> {
    crate::mark::mark(crate::mark::FEATURE);
    let address = target(side, input)?;
    let label = need_label(label)?;
    let s = eravon_vault::store::open_write(data, writer, now)?;
    let mut rows = match s.candidates_seen(me) {
        Ok(r) => r,
        Err(eravon_store::fault::Fault::Missing(_)) => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    if let Some(bad) = rows.iter().find(|r| Side::of(&r.kind).is_none()) {
        return Err(Fault::Kind(bad.kind.clone()));
    }
    if rows.iter().any(|r| r.address == address) {
        return Err(Fault::Exists(address));
    }
    let row = Candidate { kind: side.word().to_string(), address, label, added_at: now, seen: seen.to_string() };
    rows.push(row.clone());
    s.put_candidates_seen(me, &rows, now)?;
    Ok(row)
}

pub fn rename(data: &str, writer: &str, now: u64, me: &str, address: &str, label: &str) -> R<()> {
    let label = need_label(label)?;
    edit(data, writer, now, me, address, |r| r.label = label.clone())
}

pub fn remember(data: &str, writer: &str, now: u64, me: &str, address: &str, seen: &str) -> R<()> {
    edit(data, writer, now, me, address, |r| {
        if r.seen.is_empty() {
            r.seen = seen.to_string();
        }
    })
}

pub fn remove(data: &str, writer: &str, now: u64, me: &str, address: &str) -> R<()> {
    let s = eravon_vault::store::open_write(data, writer, now)?;
    let mut rows = s.candidates_seen(me)?;
    blocked_by_flaw(&rows)?;
    let before = rows.len();
    rows.retain(|r| r.address != address);
    if rows.len() == before {
        return Err(Fault::Missing(address.to_string()));
    }
    s.put_candidates_seen(me, &rows, now)?;
    Ok(())
}

fn blocked_by_flaw(_rows: &[Candidate]) -> R<()> {
    Ok(())
}


pub fn carry(data: &str, writer: &str, now: u64, from: &str, to: &str) -> R<bool> {
    let s = eravon_vault::store::open_write(data, writer, now)?;
    match s.candidates_seen(to) {
        Ok(rows) if !rows.is_empty() => return Ok(false),
        Ok(_) | Err(eravon_store::fault::Fault::Missing(_)) => {}
        Err(e) => return Err(e.into()),
    }
    let rows = match s.candidates_seen(from) {
        Ok(rows) if !rows.is_empty() => rows,
        Ok(_) | Err(eravon_store::fault::Fault::Missing(_)) => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    s.put_candidates_seen(to, &rows, now)?;
    Ok(true)
}

fn edit(data: &str, writer: &str, now: u64, me: &str, address: &str, f: impl Fn(&mut Candidate)) -> R<()> {
    let s = eravon_vault::store::open_write(data, writer, now)?;
    let mut rows = s.candidates_seen(me)?;
    let r = rows.iter_mut().find(|r| r.address == address).ok_or_else(|| Fault::Missing(address.to_string()))?;
    f(r);
    s.put_candidates_seen(me, &rows, now)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judge_by_address_guild_by_pointer_and_label_required() {
        assert_eq!(target(Side::Judge, "0xABCDEF0123456789abcdef0123456789ABCDEF01").unwrap(), "0xabcdef0123456789abcdef0123456789abcdef01");
        let chain = eravon_literals::constants::current_network().standard.chain_id;
        let p = format!("galeed:guild:{chain}:0x00000000000000000000000000000000000000aa");
        let p = p.as_str();
        assert_eq!(target(Side::Guild, p).unwrap(), "0x00000000000000000000000000000000000000aa");
        assert_eq!(target(Side::Guild, "0x00000000000000000000000000000000000000aa").unwrap_err().name(), "E_CANDS_POINTER");
        assert_eq!(target(Side::Guild, "galeed:guild:5:0x00000000000000000000000000000000000000aa").unwrap_err().name(), "E_CANDS_CHAIN");
        assert_eq!(need_label("  ").unwrap_err().name(), "E_CANDS_LABEL");
        assert_eq!(pointer_of("0x00000000000000000000000000000000000000aa"), p);
    }

    #[test]
    fn the_pointer_hint_follows_the_network() {
        let chain = eravon_literals::constants::active_standard().chain_id;
        assert_eq!(pointer_hint(), format!("galeed:guild:{chain}:0x…"));
    }
}
