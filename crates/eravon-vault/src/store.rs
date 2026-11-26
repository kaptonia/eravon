use std::path::{Path, PathBuf};

use eravon_literals::json::{In, Out};

use eravon_store::atomic;
use eravon_store::doc;
use eravon_store::fault::{Fault, R};
use eravon_store::mark;
use eravon_store::record::{self, Candidate, Kind, Record};
use eravon_store::seam::{self, Box_};
use eravon_store::settings::{Global, Identity};
use eravon_store::writer;

use crate::local::Doc;
use crate::records;
use crate::roles::Role;

fn sf(f: crate::fault::Fault) -> Fault {
    records::store_fault(f)
}

fn shown(p: &Path) -> String {
    p.display().to_string()
}

pub(crate) fn put_file(path: &Path, d: Doc, plain: &[u8]) -> R<()> {
    eravon_store::mirror::gate(&shown(path))?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    crate::local::put(&dir, &name, d, plain).map_err(sf)
}

pub(crate) fn get_file(path: &Path, d: Doc) -> R<Vec<u8>> {
    crate::local::read(path, d).map_err(sf)?.ok_or_else(|| Fault::Missing(shown(path)))
}


fn file_in(dir: &Path, stem: &str) -> R<Option<(PathBuf, Doc)>> {
    let listing = match std::fs::read_dir(dir) {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Fault::Io(format!("{}: {e}", shown(dir)))),
    };
    for e in listing.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        if let Some((s, w)) = name.rsplit_once('.') {
            if s == stem {
                if let Some((_, d)) = records::KINDS.iter().find(|(k, _)| *k == w) {
                    return Ok(Some((e.path(), *d)));
                }
            }
        }
    }
    Ok(None)
}

#[derive(Debug)]
pub struct Store {
    data: String,
    writer: String,
    writable: bool,
}

pub fn open_read(data: &str) -> R<Store> {
    mark::mark(mark::FEATURE);
    need_dir(data)?;
    records::bind(data).map_err(sf)?;
    Ok(Store { data: data.to_string(), writer: String::new(), writable: false })
}

pub fn open_write(data: &str, writer: &str, now: u64) -> R<Store> {
    mark::mark(mark::FEATURE);
    doc::need_time(now)?;
    if writer.trim().is_empty() {
        return Err(Fault::Name("写者标记不许空:单写者纪律要认得出是谁写的".into()));
    }
    atomic::mkdirs(data)?;
    records::bind(data).map_err(sf)?;
    writer::need_writer(data, writer, now)?;
    sweep_put_leftovers(data)?;
    Ok(Store { data: data.to_string(), writer: writer.to_string(), writable: true })
}

fn sweep_put_leftovers(data: &str) -> R<()> {
    static SWEPT: std::sync::Mutex<std::collections::BTreeSet<String>> = std::sync::Mutex::new(std::collections::BTreeSet::new());
    if !SWEPT.lock().unwrap_or_else(|e| e.into_inner()).insert(data.to_string()) {
        return Ok(());
    }
    fn walk(dir: &std::path::Path) -> R<()> {
        let Ok(rd) = std::fs::read_dir(dir) else { return Ok(()) };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(kind) = e.file_type() else { continue };
            if kind.is_dir() {
                walk(&p)?;
            } else if kind.is_file() && crate::home::is_temp_name(&e.file_name().to_string_lossy()) && left_over(&e.file_name().to_string_lossy(), &p) {
                std::fs::remove_file(&p).map_err(|x| Fault::Io(format!("remove {}: {x}", p.display())))?;
            }
        }
        Ok(())
    }
    walk(std::path::Path::new(data))
}

const LEFTOVER_AGE: std::time::Duration = std::time::Duration::from_secs(600);

fn left_over(_name: &str, at: &std::path::Path) -> bool {
    std::fs::metadata(at).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age >= LEFTOVER_AGE)
}

fn need_dir(data: &str) -> R<()> {
    match std::fs::metadata(data) {
        Ok(m) if m.is_dir() => Ok(()),
        Ok(_) => Err(Fault::Root(format!("{data} 不是目录"))),
        Err(e) if atomic::absent(&e) => {
            Err(Fault::Root(format!("数据根 {data} 不在盘上")))
        }
        Err(e) => Err(Fault::Io(format!("看 {data}:{e}"))),
    }
}

impl Store {
    pub fn data(&self) -> &str {
        &self.data
    }
    pub fn writer(&self) -> &str {
        &self.writer
    }

    pub fn need_writable_handle(&self) -> R<()> {
        self.need_write()
    }

    fn need_write(&self) -> R<()> {
        if self.writable {
            Ok(())
        } else {
            Err(Fault::Locked { holder: "(这个把手是只读开的)".into() })
        }
    }

    pub fn init(&self, g: &Global) -> R<()> {
        self.need_write()?;
        if !records::global_path().map_err(sf)?.exists() {
            let mut g = g.clone();
            g.last_writer = self.writer.clone();
            write_global(&self.data, &g)?;
        }
        Ok(())
    }

    pub fn init_identity(&self, addr: &str, now: u64) -> R<()> {
        self.need_write()?;
        seam::need_address(addr)?;
        doc::need_time(now)?;
        for b in Box_::ALL {
            atomic::mkdirs(&shown(&records::box_dir(addr, b).map_err(sf)?))?;
        }
        if !records::single_path(addr, Doc::IdentitySettings).map_err(sf)?.exists() {
            write_identity(&self.data, addr, &Identity::default())?;
        }
        if !records::single_path(addr, Doc::Candidates).map_err(sf)?.exists() {
            self.put_candidates(addr, &[], now)?;
        }
        Ok(())
    }

    pub fn identities(&self) -> R<Vec<String>> {
        identity_table(&self.data)
    }

    pub fn put(
        &self,
        addr: &str,
        kind: Kind,
        id: &str,
        fields: &[(String, String)],
        parts: &[(String, Vec<u8>)],
        now: u64,
    ) -> R<Record> {
        self.need_write()?;
        seam::need_address(addr)?;
        seam::need_name("记录号", id)?;
        doc::need_time(now)?;
        let b = kind.need_home()?;
        let old = match self.get(addr, b, id) {
            Ok(old) if old.kind == kind => Some(old),
            Ok(old) => {
                return Err(Fault::Kind(format!(
                    "{id} 已在 {} 里,是 {},不是 {}",
                    b.dir(),
                    old.kind.word(),
                    kind.word()
                )));
            }
            Err(Fault::Missing(_)) => None,
            Err(e) => return Err(e),
        };
        let created = old.as_ref().map_or(now, |o| o.created);
        let mut table = Vec::new();
        for (name, bytes) in parts {
            seam::need_name("零件名", name)?;
            put_file(&records::part_path(addr, b, id, name, kind).map_err(sf)?, records::of_part(kind), bytes)?;
            table.push((name.clone(), bytes.len() as u64));
        }
        if let Some(old) = old {
            for (n, sz) in old.parts {
                if !table.iter().any(|(m, _)| *m == n) {
                    table.push((n, sz));
                }
            }
        }
        let rec = Record {
            id: id.to_string(),
            kind,
            created,
            updated: now,
            writer: self.writer.clone(),
            fields: fields.to_vec(),
            parts: table,
        };
        put_file(&records::record_path(addr, b, id, kind).map_err(sf)?, records::of_record(kind), &rec.seal())?;
        Ok(rec)
    }

    pub fn get(&self, addr: &str, b: Box_, id: &str) -> R<Record> {
        seam::need_address(addr)?;
        seam::need_name("记录号", id)?;
        let dir = records::record_dir(addr, b, id).map_err(sf)?;
        let (path, sealed_as) = file_in(&dir, records::RECORD)?.ok_or_else(|| Fault::Missing(shown(&dir)))?;
        let rec = Record::open(&get_file(&path, sealed_as)?)?;
        let path = shown(&path);
        if records::of_record(rec.kind) != sealed_as {
            return Err(Fault::Kind(format!("{path} 封作 {},信封里是 {}", sealed_as.tag(), rec.kind.word())));
        }
        if rec.id != id {
            return Err(Fault::Form(format!("{path} 上写的号是 {},不是 {id}", rec.id)));
        }
        Ok(rec)
    }

    pub fn part(&self, addr: &str, b: Box_, id: &str, name: &str) -> R<Vec<u8>> {
        seam::need_address(addr)?;
        seam::need_name("记录号", id)?;
        seam::need_name("零件名", name)?;
        let dir = records::record_dir(addr, b, id).map_err(sf)?;
        let (path, d) = file_in(&dir, name)?.ok_or_else(|| Fault::Missing(format!("{}/{name}", shown(&dir))))?;
        get_file(&path, d)
    }

    pub fn list(&self, addr: &str, b: Box_) -> R<Vec<Record>> {
        seam::need_address(addr)?;
        let dir = records::box_dir(addr, b).map_err(sf)?;
        let mut out = Vec::new();
        for tok in atomic::strict_entries(&shown(&dir), seam::token_ok)? {
            let at = dir.join(&tok);
            let Some((path, d)) = file_in(&at, records::RECORD)? else { continue };
            let rec = Record::open(&get_file(&path, d)?)?;
            if records::record_name(b, &rec.id).map_err(sf)? != tok {
                return Err(Fault::Sealed(format!("swapped · {}: 里住的是 {},目录名不是它的名", shown(&path), rec.id)));
            }
            out.push(self.get(addr, b, &rec.id)?);
        }
        record::newest_first(&mut out);
        Ok(out)
    }

    pub fn delete(&self, addr: &str, kind: Kind, id: &str) -> R<()> {
        self.need_write()?;
        seam::need_address(addr)?;
        seam::need_name("记录号", id)?;
        let b = kind.need_home()?;
        let old = self.get(addr, b, id)?;
        if old.kind != kind {
            return Err(Fault::Kind(format!("{id} 是 {},不是 {}", old.kind.word(), kind.word())));
        }
        let dir = records::record_dir(addr, b, id).map_err(sf)?;
        atomic::retire_dir(&shown(&dir), id)
    }

    pub fn put_candidates(
        &self,
        addr: &str,
        rows: &[(String, String, String, u64)],
        now: u64,
    ) -> R<()> {
        let old = to_change(self.candidates_seen(addr))?;
        let full: Vec<Candidate> = rows
            .iter()
            .map(|(k, a, l, t)| {
                let seen = old
                    .iter()
                    .find(|o| &o.kind == k && &o.address == a)
                    .map(|o| o.seen.clone())
                    .unwrap_or_default();
                Candidate { kind: k.clone(), address: a.clone(), label: l.clone(), added_at: *t, seen }
            })
            .collect();
        self.put_candidates_seen(addr, &full, now)
    }

    pub fn put_candidates_seen(&self, addr: &str, rows: &[Candidate], now: u64) -> R<()> {
        self.need_write()?;
        seam::need_address(addr)?;
        doc::need_time(now)?;
        need_cap(rows.len())?;
        let bytes = doc::seal(
            seam::V_CANDIDATES,
            vec![
                (
                    "entries",
                    Out::A(
                        rows.iter()
                            .map(|c| {
                                let mut m = vec![
                                    ("addedAt", Out::N(c.added_at)),
                                    ("address", Out::S(c.address.clone())),
                                    ("kind", Out::S(c.kind.clone())),
                                    ("label", Out::S(c.label.clone())),
                                ];
                                if !c.seen.is_empty() {
                                    m.push(("seen", Out::S(c.seen.clone())));
                                }
                                Out::O(m)
                            })
                            .collect(),
                    ),
                ),
                ("updatedAt", Out::N(now)),
                ("writer", Out::S(self.writer.clone())),
            ],
        );
        put_file(&records::single_path(addr, Doc::Candidates).map_err(sf)?, Doc::Candidates, &bytes)
    }

    pub fn candidates_seen(&self, addr: &str) -> R<Vec<Candidate>> {
        seam::need_address(addr)?;
        let v: In = doc::open(seam::V_CANDIDATES, &get_file(&records::single_path(addr, Doc::Candidates).map_err(sf)?, Doc::Candidates)?)?;
        let mut rows = record::candidate_rows_seen(&v)?;
        record::newest_first_by(&mut rows, |r| r.added_at, |r| r.address.as_bytes());
        Ok(rows)
    }

    pub fn candidates(&self, addr: &str) -> R<Vec<(String, String, String, u64)>> {
        seam::need_address(addr)?;
        let v: In = doc::open(seam::V_CANDIDATES, &get_file(&records::single_path(addr, Doc::Candidates).map_err(sf)?, Doc::Candidates)?)?;
        let mut rows = record::candidate_rows(&v)?;
        record::candidates_newest_first(&mut rows);
        Ok(rows)
    }
}

pub fn identity_table(data: &str) -> R<Vec<String>> {
    records::bind(data).map_err(sf)?;
    let reg = crate::register::read().map_err(sf)?.unwrap_or_default();
    let mut out = Vec::new();
    for row in &reg.rows {
        for seat in Role::ALL {
            if let (Some(a), Some(_)) = (row.current(seat), row.home(seat)) {
                out.push(a.hex());
            }
        }
    }
    out.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    out.dedup();
    Ok(out)
}


pub fn known_names(data: &str) -> R<Vec<(String, String)>> {
    let st = open_read(data)?;
    let mut out = Vec::new();
    let nk = crate::names::key().map_err(sf)?;
    for addr in identity_table(data)? {
        let (row, _) = records::seat_of(&addr).map_err(sf)?;
        out.push((addr.clone(), crate::home::home_dir_name(&nk, &row.id)));
        for b in Box_::ALL {
            for r in st.list(&addr, b)? {
                let t = records::record_name(b, &r.id).map_err(sf)?;
                out.push((r.id, t));
            }
        }
    }
    Ok(out)
}

pub fn parties_of(data: &str, me: &str) -> Result<Vec<String>, Fault> {
    let me = me.to_ascii_lowercase();
    let party = match read_identity(data, &me) {
        Ok(t) => t.party(&me).to_ascii_lowercase(),
        Err(Fault::Missing(_)) => me.clone(),
        Err(e) => return Err(e),
    };
    Ok(if party == me { vec![me] } else { vec![me, party] })
}

pub fn seen_of(data: &str, me: &str) -> Result<Vec<String>, Fault> {
    let me = me.to_ascii_lowercase();
    match read_identity(data, &me) {
        Ok(t) => Ok(t.seen(&me)),
        Err(Fault::Missing(_)) => Ok(vec![me]),
        Err(e) => Err(e),
    }
}

pub fn keys_of(data: &str, me: &str) -> Result<Vec<String>, Fault> {
    let me = me.to_ascii_lowercase();
    let former = match read_identity(data, &me) {
        Ok(t) => t.former,
        Err(Fault::Missing(_)) => Vec::new(),
        Err(e) => return Err(e),
    };
    let mut out = vec![me.clone()];
    let mut add = |k: String| {
        let k = k.to_ascii_lowercase();
        if !out.contains(&k) {
            out.push(k);
        }
    };
    for f in former {
        add(f);
    }
    if let (Some(a), Ok(Some(reg))) = (crate::key::Address::parse(&me), crate::register::read()) {
        if let Some((row, seat)) = records::row_of_key(&reg, a) {
            if let (Some(root), Some(o)) = (row.address(seat), row.office(seat)) {
                add(root.hex());
                for f in o.former {
                    add(crate::key::Address(f).hex());
                }
            }
        }
    }
    Ok(out)
}

pub fn archive_of(data: &str, me: &str) -> Result<Vec<String>, Fault> {
    let keys = keys_of(data, me)?;
    let reg = crate::register::read().ok().flatten().unwrap_or_default();
    let mut homes: Vec<std::path::PathBuf> = Vec::new();
    let mut out = Vec::new();
    for k in keys {
        let home = crate::key::Address::parse(&k).and_then(|a| records::row_of_key(&reg, a)).and_then(|(row, seat)| row.home(seat));
        match home {
            Some(h) if homes.iter().any(|x| crate::home::same_place(x, &h)) => {}
            Some(h) => {
                homes.push(h);
                out.push(k);
            }
            None => out.push(k),
        }
    }
    Ok(out)
}


pub fn write_global(data: &str, g: &Global) -> R<()> {
    write_gate(data)?;
    records::bind(data).map_err(sf)?;
    put_file(&records::global_path().map_err(sf)?, Doc::GlobalSettings, &g.seal())?;
    crate::tokens::wrote(data, g);
    eravon_literals::constants::set_past_switch(g.past_cases);
    Ok(())
}

fn to_change<T: Default>(r: R<T>) -> R<T> {
    match r {
        Ok(x) => Ok(x),
        Err(Fault::Missing(_)) => Ok(T::default()),
        Err(e @ Fault::NoKey(_)) => Err(e),
        Err(e) => refuse_unread(&format!("在盘上却读不成,不覆盖:{} {}", e.name(), e.detail())).map(|()| T::default()),
    }
}

pub fn refuse_unread(why: &str) -> R<()> {
    Err(Fault::Unread(why.to_string()))
}


pub fn global_to_change(data: &str) -> R<Global> {
    to_change(read_global(data))
}

pub fn identity_to_change(data: &str, addr: &str) -> R<Identity> {
    to_change(read_identity(data, addr))
}

pub fn read_global(data: &str) -> R<Global> {
    records::bind(data).map_err(sf)?;
    Global::open(&get_file(&records::global_path().map_err(sf)?, Doc::GlobalSettings)?)
}

pub fn write_identity(data: &str, addr: &str, s: &Identity) -> R<()> {
    seam::need_address(addr)?;
    write_gate(data)?;
    records::bind(data).map_err(sf)?;
    put_file(&records::single_path(addr, Doc::IdentitySettings).map_err(sf)?, Doc::IdentitySettings, &s.seal())
}

pub fn read_identity(data: &str, addr: &str) -> R<Identity> {
    seam::need_address(addr)?;
    records::bind(data).map_err(sf)?;
    Identity::open(&get_file(&records::single_path(addr, Doc::IdentitySettings).map_err(sf)?, Doc::IdentitySettings)?)
}

pub fn endpoints(data: &str, addr: &str) -> R<Vec<String>> {
    let Some(s) = home_settings(data, addr)? else { return Ok(Vec::new()) };
    let chain = run_network(&s);
    eravon_literals::constants::select_chain(chain);
    Ok(nodes_on(&s, chain))
}

pub fn nodes_in(s: &crate::settings::Settings) -> Vec<String> {
    nodes_on(s, run_network(s))
}

fn run_network(s: &crate::settings::Settings) -> u64 {
    eravon_literals::constants::run_chain().unwrap_or_else(|| network_chain(s))
}

pub fn rpc_endpoints(data: &str, addr: &str) -> R<Vec<String>> {
    let Some(s) = home_settings(data, addr)? else { return Ok(Vec::new()) };
    let chain = run_network(&s);
    eravon_literals::constants::select_chain(chain);
    Ok(urls_on(&s.endpoints, chain))
}

pub fn endpoints_on(data: &str, addr: &str, chain: u64) -> R<Vec<String>> {
    let Some(s) = home_settings(data, addr)? else { return Ok(Vec::new()) };
    Ok(nodes_on(&s, chain))
}

fn nodes_on(s: &crate::settings::Settings, chain: u64) -> Vec<String> {
    let on = crate::settings::Settings { chain_id: Some(chain), ..s.clone() };
    crate::chainx::nodes_of(&on).into_iter().filter(|e| e.chain == chain).map(|e| e.url.for_transport().to_string()).collect()
}

pub fn home_settings(data: &str, addr: &str) -> R<Option<crate::settings::Settings>> {
    let Some(at) = home_at(data, addr)? else { return Ok(None) };
    if !at.is_dir() {
        return Ok(None);
    }
    let home = crate::home::Home::open(&at).map_err(sf)?;
    Ok(Some(crate::settings::Settings::read(&home).map_err(sf)?))
}

pub fn network(data: &str, addr: &str) -> R<u64> {
    let Some(at) = home_at(data, addr)? else { return Ok(eravon_literals::constants::default_network().standard.chain_id) };
    if !at.is_dir() {
        return Ok(eravon_literals::constants::default_network().standard.chain_id);
    }
    let home = crate::home::Home::open(&at).map_err(sf)?;
    Ok(network_chain(&crate::settings::Settings::read(&home).map_err(sf)?))
}

pub fn choose_network(data: &str, addr: &str, chain: u64) -> R<()> {
    let chain = known_chain(chain)?;
    change_home(data, addr, |s| s.chain_id = Some(chain))?;
    eravon_literals::constants::select_chain(chain);
    Ok(())
}

fn network_chain(s: &crate::settings::Settings) -> u64 {
    s.chain_id
        .filter(|c| eravon_literals::constants::network_of(*c).is_some())
        .unwrap_or(eravon_literals::constants::default_network().standard.chain_id)
}


pub fn write_endpoints(data: &str, addr: &str, urls: &[String]) -> R<()> {
    write_gate(data)?;
    let (home, mut s) = settings_to_write(data, addr)?;
    let chain = network_chain(&s);
    s.endpoints = replace_on(&s.endpoints, chain, urls);
    s.write(&home).map_err(sf)
}

pub fn write_factory_endpoints(data: &str, addr: &str, chain: u64) -> R<bool> {
    write_gate(data)?;
    let (home, mut s) = settings_to_write(data, addr)?;
    if own_table(&s) {
        return Ok(false);
    }
    let mut specs = Vec::new();
    for n in eravon_literals::constants::NETWORKS {
        specs.extend(eravon_literals::constants::factory_of(&n).iter().map(|u| format!("{}={u}", n.standard.chain_id)));
    }
    s.endpoints = specs;
    s.chain_id = Some(chain);
    s.write(&home).map_err(sf)?;
    Ok(true)
}

pub fn own_table(s: &crate::settings::Settings) -> bool {
    !s.endpoints.is_empty() || !s.rest.is_empty()
}

pub fn tx_hints(data: &str, owner: &str) -> R<Vec<(String, String, String)>> {
    let st = open_read(data)?;
    let rows = match st.list(owner, Box_::Orders) {
        Ok(r) => r,
        Err(Fault::Missing(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    Ok(rows
        .into_iter()
        .filter(|r| r.kind == Kind::TxReceipt)
        .map(|r| {
            let g = |k: &str| r.field(k).unwrap_or_default().to_string();
            (g("kind"), g("hash"), g("order"))
        })
        .collect())
}

pub fn change_identity(data: &str, addr: &str, f: impl FnOnce(&mut Identity)) -> R<()> {
    let mut t = identity_to_change(data, addr)?;
    f(&mut t);
    write_identity(data, addr, &t)
}

pub fn change_home(data: &str, addr: &str, f: impl FnOnce(&mut crate::settings::Settings)) -> R<()> {
    write_gate(data)?;
    let (home, mut s) = settings_to_write(data, addr)?;
    f(&mut s);
    s.write(&home).map_err(sf)
}

fn settings_to_write(data: &str, addr: &str) -> R<(crate::home::Home, crate::settings::Settings)> {
    let at = home_at(data, addr)?.ok_or_else(|| sf(crate::fault::Fault::known(crate::fault::Known::NoHome, addr.to_string())))?;
    let home = crate::home::Home::open_or_create(&at).map_err(sf)?;
    let s = crate::settings::Settings::read(&home).map_err(sf)?;
    Ok((home, s))
}

pub fn birth_chain(asked: Option<u64>) -> R<u64> {
    asked.map_or(Ok(eravon_literals::constants::default_network().standard.chain_id), known_chain)
}

fn known_chain(c: u64) -> R<u64> {
    if eravon_literals::constants::network_of(c).is_some() {
        Ok(c)
    } else {
        Err(Fault::Form(format!("链 {c} 不是已知的网")))
    }
}

fn write_gate(data: &str) -> R<()> {
    eravon_store::mirror::gate(data)?;
    match writer::take(data)? {
        eravon_store::lock::Mode::Writer => Ok(()),
        _ => writer::need_writer(data, "", 0),
    }
}

fn home_at(data: &str, addr: &str) -> R<Option<PathBuf>> {
    seam::need_address(addr)?;
    records::bind(data).map_err(sf)?;
    let (row, seat) = records::seat_of(addr).map_err(sf)?;
    Ok(row.home(seat))
}

pub fn nodes_chain(s: &crate::settings::Settings, asked: Option<u64>) -> R<u64> {
    asked.map_or(Ok(chain_of(s)), known_chain)
}

pub fn chain_of(s: &crate::settings::Settings) -> u64 {
    network_chain(s)
}

pub fn specs_on(chain: u64, urls: &[String]) -> Vec<String> {
    urls.iter().map(|u| format!("{chain}={u}")).collect()
}

pub fn urls_of(specs: &[String]) -> Vec<String> {
    urls_on(specs, eravon_literals::constants::current_network().standard.chain_id)
}

pub fn urls_on(specs: &[String], chain: u64) -> Vec<String> {
    specs.iter().filter_map(|s| eravon_chain::rpc::endpoint_spec(s).ok()).filter(|(c, _)| *c == chain).map(|(_, u)| u).collect()
}

pub fn typed_on(old: &[String], chain: u64, typed: Vec<String>) -> Vec<String> {
    let chain_of = |s: &String| eravon_chain::rpc::endpoint_spec(s).ok().map(|(c, _)| c);
    let mut named: Vec<u64> = typed.iter().filter_map(chain_of).collect();
    named.push(chain);
    let on = |s: &String| chain_of(s).is_some_and(|c| named.contains(&c));
    let mut out: Vec<String> = old.iter().filter(|s| !on(s)).cloned().collect();
    out.extend(typed);
    out
}

#[cfg(test)]
mod birth_network {
    const PIN: &str = "48261537";
    const PHRASE: &str = "test test test test test test test test test test test junk";
    const SELLER: &str = "0x98e503f35d0a019cb0a251ad243a4ccfcf371f46";

    #[test]
    fn a_home_is_made_on_the_row_chosen_with_that_rows_factory_nodes() {
        assert_eq!(super::birth_chain(None).unwrap(), eravon_literals::constants::NETWORKS[0].standard.chain_id);
        assert!(super::birth_chain(Some(5)).is_err(), "no network lives on chain 5");
        for n in eravon_literals::constants::NETWORKS {
            let chain = n.standard.chain_id;
            assert_eq!(super::birth_chain(Some(chain)).unwrap(), chain);
            let d = crate::testing::fresh();
            crate::onboard::land(&d, crate::onboard::Mode::First, eravon_store::vault::Seat::Seller, "主钥", false, crate::onboard::Source::Words(PHRASE), PIN, None, None, 900).unwrap();
            let mut open = crate::session::Session::locked(&d);
            open.unlock(PIN).unwrap();
            let at = super::home_at(&d, SELLER).unwrap().expect("the seat's home");
            let file = crate::home::Home::open(&at).unwrap().dir(crate::home::Slot::Settings).join(crate::settings::FILE);
            std::fs::remove_file(&file).unwrap();
            assert!(super::write_factory_endpoints(&d, SELLER, chain).unwrap(), "{}: written", n.name);
            assert_eq!(super::network(&d, SELLER).unwrap(), chain, "{}: on the row chosen", n.name);
            let s = super::home_settings(&d, SELLER).unwrap().expect("settings");
            for m in eravon_literals::constants::NETWORKS {
                let want: Vec<String> = eravon_literals::constants::factory_of(&m).iter().map(|u| (*u).to_string()).collect();
                assert_eq!(super::urls_on(&s.endpoints, m.standard.chain_id), want, "{}: {}'s factory nodes", n.name, m.name);
            }
            let _ = std::fs::remove_dir_all(&d);
        }
    }
}

#[cfg(test)]
mod network_nodes {
    #[test]
    fn one_networks_nodes_never_touch_anothers() {
        let specs = vec!["1=https://a.example".to_string(), "11155111=https://s.example".to_string(), "1=https://b.example".to_string()];
        assert_eq!(super::urls_on(&specs, 1), vec!["https://a.example".to_string(), "https://b.example".to_string()]);
        assert_eq!(super::urls_on(&specs, 11155111), vec!["https://s.example".to_string()]);
        let got = super::replace_on(&specs, 11155111, &["http://127.0.0.1:9".to_string()]);
        assert_eq!(super::urls_on(&got, 1), super::urls_on(&specs, 1));
        assert_eq!(super::urls_on(&got, 11155111), vec!["http://127.0.0.1:9".to_string()]);
        let none = super::replace_on(&specs, 1, &[]);
        assert!(super::urls_on(&none, 1).is_empty() && super::urls_on(&none, 11155111).len() == 1);
    }

    #[test]
    fn a_typed_line_for_any_chain_replaces_that_chains_nodes() {
        let here = eravon_literals::constants::current_network().standard.chain_id;
        let other = if here == 1 { 11155111 } else { 1 };
        let line = format!("{other}=https://n.example");
        let kept = format!("{other}0=https://k.example");
        let once = super::typed_on(&[kept.clone(), format!("{other}=https://old.example")], here, vec![line.clone()]);
        assert_eq!(super::typed_on(&once, here, vec![line.clone()]), vec![kept, line]);
    }

    #[test]
    fn the_nodes_chain_is_the_one_named_else_the_homes_own() {
        let on_main = crate::settings::Settings { chain_id: Some(1), ..Default::default() };
        let on_none = crate::settings::Settings::default();
        let default = eravon_literals::constants::default_network().standard.chain_id;
        assert_eq!(super::nodes_chain(&on_main, None).expect("the home's"), 1);
        assert_eq!(super::nodes_chain(&on_none, None).expect("the default"), default);
        assert_eq!(super::nodes_chain(&on_none, Some(1)).expect("named"), 1);
        assert!(super::nodes_chain(&on_main, Some(424242)).is_err(), "an unknown chain is refused");
        let sepolia = vec!["11155111=https://s1.example".to_string(), "11155111=https://s2.example".to_string()];
        let typed = super::specs_on(1, &["https://m.example".to_string()]);
        let got = super::typed_on(&sepolia, super::nodes_chain(&on_none, Some(1)).expect("named"), typed);
        assert_eq!(super::urls_on(&got, 11155111).len(), 2, "the home's own network's nodes stay");
        assert_eq!(super::urls_on(&got, 1), vec!["https://m.example".to_string()]);
    }
}

pub fn replace_on(specs: &[String], chain: u64, urls: &[String]) -> Vec<String> {
    let on = |s: &String| eravon_chain::rpc::endpoint_spec(s).ok().is_some_and(|(c, _)| c == chain);
    let at = specs.iter().position(on).unwrap_or(specs.len());
    let mut out: Vec<String> = specs[..at].iter().filter(|s| !on(s)).cloned().collect();
    out.extend(urls.iter().map(|u| format!("{chain}={u}")));
    out.extend(specs[at..].iter().filter(|s| !on(s)).cloned());
    out
}

pub fn candidates_fit(n: usize) -> R<()> {
    need_cap(n)
}

fn need_cap(n: usize) -> R<()> {
    if n > eravon_literals::constants::CANDIDATE_CAP {
        return Err(Fault::Cap {
            what: seam::N_CANDIDATES,
            cap: eravon_literals::constants::CANDIDATE_CAP,
        });
    }
    Ok(())
}


#[cfg(test)]
mod leftovers {
    #[test]
    fn a_put_left_over_is_swept_and_nothing_else() {
        let d = std::env::temp_dir().join(format!("eravon-vault-leftovers-{}", std::process::id()));
        let deep = d.join("abcd/seller/store");
        std::fs::create_dir_all(&deep).unwrap();
        let tmp = |n: &str| deep.join(format!("{}{n}", eravon_glue::layout::TMP_PREFIX));
        let left = tmp("0123456789abcdef");
        let fresh = tmp("fedcba9876543210");
        let kept = deep.join("identity.json");
        let not_shaped = deep.join(".identity.json.4242.1700000000000000000.tmp");
        for (p, b) in [(&left, &b"half"[..]), (&fresh, b"half"), (&kept, b"whole"), (&not_shaped, b"half")] {
            std::fs::write(p, b).unwrap();
        }
        let long_ago = std::time::SystemTime::now() - 2 * super::LEFTOVER_AGE;
        for p in [&left, &not_shaped] {
            std::fs::File::options().write(true).open(p).unwrap().set_modified(long_ago).unwrap();
        }
        super::sweep_put_leftovers(&d.to_string_lossy()).unwrap();
        assert!(!left.exists(), "别的进程早先断下的残留该清掉");
        assert!(fresh.exists(), "新落下的(可能正在写)不动");
        assert!(kept.exists(), "正经的档不动");
        assert!(not_shaped.exists(), "不是那一种临时形的不动");
        let _ = std::fs::remove_dir_all(&d);
    }
}
