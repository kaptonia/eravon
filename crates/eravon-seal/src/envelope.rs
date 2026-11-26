use crate::keybox::LocalKey;
use crate::names::NameKey;
use crate::trouble::{Code, Coded, Trouble};
use std::path::{Path, PathBuf};

pub const SETTINGS_DIR: &str = "settings";
pub const STORE_DIR: &str = "store";
pub const WRITER_LOCK: &str = ".lock";
pub const WRITER_ROOM: &str = "";
pub const SEATS: [&str; 2] = ["seller", "buyer"];
pub const NEXT: &str = ".ev-next";

pub fn staged_name(name: &str) -> String {
    format!("{name}{NEXT}")
}

pub fn is_staged(p: &str) -> bool {
    p.ends_with(NEXT)
}

pub fn unstaged(p: &str) -> Option<&str> {
    p.strip_suffix(NEXT)
}

pub use eravon_glue::sealed::{MAGIC, MAGIC_V2};
pub const NONCE: usize = 24;
pub const DIGEST: usize = 16;
pub const ALG_KEYED: u8 = 2;
pub const PLACE_KEY_INFO: &str = "eravon-local/place-key";
pub const OWNER_DOMAIN: &str = "eravon-local/owner";
pub const PLACE_DOMAIN: &str = "eravon-local/place";
pub const LABEL_FILE: &str = "home-label.json";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Doc {
    Registry,
    Checked,
    Settings,
    Machine,
    Plan,
    HomeLabel,
    BackupIndex,
    GlobalSettings,
    Product,
    Draft,
    TermsOriginal,
    OfferSignature,
    Payload,
    Receipt,
    OrderArchive,
    Candidates,
    OrderLabel,
    TxReceipt,
    NoticeRead,
    IdentitySettings,
    Reading,
    MachineSealed,
}

impl Doc {
    pub const ALL: [Doc; 22] = [
        Doc::Registry,
        Doc::Checked,
        Doc::Settings,
        Doc::Machine,
        Doc::Plan,
        Doc::HomeLabel,
        Doc::BackupIndex,
        Doc::GlobalSettings,
        Doc::Product,
        Doc::Draft,
        Doc::TermsOriginal,
        Doc::OfferSignature,
        Doc::Payload,
        Doc::Receipt,
        Doc::OrderArchive,
        Doc::Candidates,
        Doc::OrderLabel,
        Doc::TxReceipt,
        Doc::NoticeRead,
        Doc::IdentitySettings,
        Doc::Reading,
        Doc::MachineSealed,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            Doc::Registry => "registry",
            Doc::Checked => "checked",
            Doc::Settings => "settings",
            Doc::Machine => "machine",
            Doc::Plan => "plan",
            Doc::HomeLabel => "home-label",
            Doc::BackupIndex => "backup-index",
            Doc::MachineSealed => "machine-sealed",
            other => house_tag(other),
        }
    }

    pub fn version(self) -> u16 {
        1
    }

    pub fn from_tag(t: &str) -> Option<Doc> {
        Doc::ALL.into_iter().find(|d| d.tag() == t)
    }

    pub fn in_machine(self) -> bool {
        matches!(self, Doc::Registry | Doc::Checked | Doc::Machine | Doc::Plan | Doc::BackupIndex | Doc::MachineSealed | Doc::GlobalSettings)
    }

    pub fn keyed(self) -> bool {
        false
    }

    pub fn canonical_shape(self) -> Option<Code> {
        match self {
            Doc::Registry => Some(Code::IdentitiesShape),
            Doc::Checked | Doc::Settings | Doc::HomeLabel | Doc::BackupIndex => Some(Code::SettingsShape),
            Doc::Machine | Doc::MachineSealed => Some(Code::MachineShape),
            Doc::Plan => None,
            Doc::GlobalSettings | Doc::IdentitySettings | Doc::Product | Doc::Draft | Doc::TermsOriginal | Doc::OfferSignature | Doc::Payload | Doc::Receipt | Doc::OrderArchive | Doc::Candidates | Doc::OrderLabel | Doc::TxReceipt | Doc::NoticeRead | Doc::Reading => None,
        }
    }
}

fn house_tag(d: Doc) -> &'static str {
    use eravon_literals::literals::LOCAL_DATA_KINDS as K;
    match d {
        Doc::GlobalSettings => K[0],
        Doc::Product => K[1],
        Doc::Draft => K[2],
        Doc::TermsOriginal => K[3],
        Doc::OfferSignature => K[4],
        Doc::Payload => K[5],
        Doc::Receipt => K[6],
        Doc::OrderArchive => K[7],
        Doc::Candidates => K[8],
        Doc::OrderLabel => K[9],
        Doc::TxReceipt => K[10],
        Doc::NoticeRead => K[11],
        Doc::IdentitySettings => K[12],
        Doc::Reading => K[13],
        Doc::Registry | Doc::Checked | Doc::Settings | Doc::Machine | Doc::Plan | Doc::HomeLabel | Doc::BackupIndex | Doc::MachineSealed => "",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unread {
    NotSealed,
    Truncated,
    OtherKind,
    NewerVersion,
    Swapped,
    Unopenable,
}

impl Unread {
    pub const ALL: [Unread; 6] = [Unread::NotSealed, Unread::Truncated, Unread::OtherKind, Unread::NewerVersion, Unread::Swapped, Unread::Unopenable];

    pub fn word(self) -> &'static str {
        match self {
            Unread::NotSealed => "not-sealed",
            Unread::Truncated => "truncated",
            Unread::OtherKind => "other-kind",
            Unread::NewerVersion => "newer-version",
            Unread::Swapped => "swapped",
            Unread::Unopenable => "unopenable",
        }
    }

    pub fn of<F: Coded>(f: &F) -> Option<Unread> {
        if f.sealed_code() != Some(Code::LocalSeal) {
            return None;
        }
        let first = f.sealed_tail().split(' ').next().unwrap_or_default();
        Unread::ALL.into_iter().find(|u| u.word() == first)
    }

    pub fn fault(self, doc: Doc, what: &str) -> Trouble {
        Trouble::new(Code::LocalSeal, format!("{} · {} {what}", self.word(), doc.tag()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    Machine,
    Home(String),
    Label,
}

impl Owner {
    pub fn text(&self) -> String {
        match self {
            Owner::Machine => "machine".to_string(),
            Owner::Home(n) => format!("home:{n}"),
            Owner::Label => "home-label".to_string(),
        }
    }

    pub fn parse(t: &str) -> Option<Owner> {
        match t {
            "machine" => Some(Owner::Machine),
            "home-label" => Some(Owner::Label),
            _ => t.strip_prefix("home:").filter(|n| is_home_number(n)).map(|n| Owner::Home(n.to_string())),
        }
    }

    fn half(&self) -> [u8; DIGEST / 2] {
        half_of(OWNER_DOMAIN, &[self.text().as_bytes()])
    }
}

fn half_of(domain: &str, parts: &[&[u8]]) -> [u8; DIGEST / 2] {
    let mut msg = domain.as_bytes().to_vec();
    for p in parts {
        msg.push(0);
        msg.extend_from_slice(p);
    }
    let d = eravon::cryptox::sha256(&msg);
    let mut out = [0u8; DIGEST / 2];
    out.copy_from_slice(&d[..DIGEST / 2]);
    out
}

pub fn is_home_number(n: &str) -> bool {
    n.len() == 32 && n.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub owner: Owner,
    pub doc: Doc,
    pub logical: String,
}

impl Ident {
    pub fn place_half(logical: &str, key: &[u8; 32]) -> [u8; DIGEST / 2] {
        let mut k = [0u8; 32];
        crate::crypto::hkdf_sha256(key, PLACE_KEY_INFO.as_bytes(), &mut k);
        let mut msg = PLACE_DOMAIN.as_bytes().to_vec();
        msg.push(0);
        msg.extend_from_slice(logical.as_bytes());
        let mac = crate::crypto::hmac_sha256(&k, &msg);
        eravon_os::wipe(&mut k);
        let mut out = [0u8; DIGEST / 2];
        out.copy_from_slice(&mac[..DIGEST / 2]);
        out
    }

    pub fn digest_keyed(&self, key: &[u8; 32]) -> [u8; DIGEST] {
        let mut d = [0u8; DIGEST];
        d[..DIGEST / 2].copy_from_slice(&self.owner.half());
        d[DIGEST / 2..].copy_from_slice(&Ident::place_half(&self.logical, key));
        d
    }
}

#[derive(Clone, Debug)]
pub struct Expect {
    pub owner: Option<Owner>,
    pub doc: Doc,
    pub rel: Option<String>,
}

struct Head<'a> {
    tag: &'a [u8],
    version: u16,
    digest: [u8; DIGEST],
    nonce: [u8; NONCE],
    aad: &'a [u8],
    sealed: &'a [u8],
}

fn take<'a>(bytes: &'a [u8], i: &mut usize, n: usize) -> Result<&'a [u8], Unread> {
    let s = bytes.get(*i..*i + n).ok_or(Unread::Truncated)?;
    *i += n;
    Ok(s)
}

fn parse_head(bytes: &[u8]) -> Result<Head<'_>, Unread> {
    if !bytes.starts_with(MAGIC_V2) {
        return Err(Unread::NotSealed);
    }
    let mut i = MAGIC_V2.len();
    let alg = take(bytes, &mut i, 1)?[0];
    if alg < ALG_KEYED {
        return Err(Unread::NotSealed);
    }
    if alg > ALG_KEYED {
        return Err(Unread::NewerVersion);
    }
    let len = take(bytes, &mut i, 1)?[0] as usize;
    let tag = take(bytes, &mut i, len)?;
    let v = take(bytes, &mut i, 2)?;
    let version = u16::from_be_bytes([v[0], v[1]]);
    let mut digest = [0u8; DIGEST];
    digest.copy_from_slice(take(bytes, &mut i, DIGEST)?);
    let mut nonce = [0u8; NONCE];
    nonce.copy_from_slice(take(bytes, &mut i, NONCE)?);
    if bytes.len() < i + 16 {
        return Err(Unread::Truncated);
    }
    Ok(Head { tag, version, digest, nonce, aad: &bytes[..i], sealed: &bytes[i..] })
}

fn frame(id: &Ident, plain: &[u8]) -> Vec<u8> {
    let o = id.owner.text();
    let mut out = Vec::with_capacity(4 + o.len() + id.logical.len() + plain.len());
    for part in [o.as_bytes(), id.logical.as_bytes()] {
        out.extend_from_slice(&(part.len() as u16).to_be_bytes());
        out.extend_from_slice(part);
    }
    out.extend_from_slice(plain);
    out
}

fn unframe(b: &[u8]) -> Option<(String, String, &[u8])> {
    let mut i = 0usize;
    let mut text = || -> Option<String> {
        let n = u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize;
        let s = std::str::from_utf8(b.get(i + 2..i + 2 + n)?).ok()?.to_string();
        i += 2 + n;
        Some(s)
    };
    let owner = text()?;
    let logical = text()?;
    Some((owner, logical, &b[i..]))
}

pub fn envelope(key: &[u8; 32], id: &Ident, nonce: &[u8; NONCE], plain: &[u8]) -> Result<Vec<u8>, Trouble> {
    let tag = id.doc.tag().as_bytes();
    let mut out = Vec::with_capacity(MAGIC_V2.len() + 4 + tag.len() + DIGEST + NONCE + plain.len() + 64);
    out.extend_from_slice(MAGIC_V2);
    out.push(ALG_KEYED);
    out.push(tag.len() as u8);
    out.extend_from_slice(tag);
    out.extend_from_slice(&id.doc.version().to_be_bytes());
    out.extend_from_slice(&id.digest_keyed(key));
    out.extend_from_slice(nonce);
    let mut body = frame(id, plain);
    let ct = crate::crypto::xchacha_seal(key, nonce, &out, &body);
    eravon_os::wipe(&mut body);
    out.extend_from_slice(&ct.ok_or_else(|| Trouble::new(Code::LocalSeal, id.doc.tag().to_string()))?);
    Ok(out)
}

fn head(doc: Doc) -> Vec<u8> {
    let tag = doc.tag().as_bytes();
    let mut h = MAGIC_V2.to_vec();
    h.push(ALG_KEYED);
    h.push(tag.len() as u8);
    h.extend_from_slice(tag);
    h.extend_from_slice(&doc.version().to_be_bytes());
    h
}

const BIND: bool = true;

pub fn unseal(key: &[u8; 32], ex: &Expect, bytes: &[u8], what: &str, nk: Option<&NameKey>) -> Result<Vec<u8>, Trouble> {
    unseal_headed(key, ex, &head(ex.doc), bytes, what, nk)
}

fn unseal_headed(key: &[u8; 32], ex: &Expect, h: &[u8], bytes: &[u8], what: &str, nk: Option<&NameKey>) -> Result<Vec<u8>, Trouble> {
    let no = |u: Unread| -> Trouble { u.fault(ex.doc, what) };
    let p = parse_head(bytes).map_err(no)?;
    if !bytes.starts_with(h) {
        return Err(no(if p.tag != ex.doc.tag().as_bytes() {
            Unread::OtherKind
        } else if p.version > ex.doc.version() {
            Unread::NewerVersion
        } else {
            Unread::Unopenable
        }));
    }
    let h = p;
    let Some(owner) = &ex.owner else { return Err(no(Unread::Swapped)) };
    if BIND && h.digest[..DIGEST / 2] != owner.half() {
        return Err(no(Unread::Swapped));
    }
    let fixed = (!ex.doc.keyed()).then_some(ex.rel.as_deref()).flatten();
    let place_differs = BIND && fixed.is_some_and(|rel| h.digest[DIGEST / 2..] != Ident::place_half(rel, key));
    let mut body = crate::crypto::xchacha_open(key, &h.nonce, h.aad, h.sealed).ok_or_else(|| no(Unread::Unopenable))?;
    if place_differs {
        eravon_os::wipe(&mut body);
        return Err(no(Unread::Swapped));
    }
    let checked = (|| -> Result<Vec<u8>, Unread> {
        let (o, logical, plain) = unframe(&body).ok_or(Unread::Unopenable)?;
        let id = Ident { owner: Owner::parse(&o).ok_or(Unread::Swapped)?, doc: ex.doc, logical };
        if BIND && (&id.owner != owner || id.digest_keyed(key) != h.digest) {
            return Err(Unread::Swapped);
        }
        if BIND && ex.doc.keyed() {
            let from_content = logical_rel(ex.doc, ex.rel.as_deref().unwrap_or(&id.logical), plain).map_err(|_| Unread::Swapped)?;
            if from_content != id.logical {
                return Err(Unread::Swapped);
            }
            if let Some(rel) = &ex.rel {
                let keyed = nk.and_then(|nk| disk_rel(ex.doc, &id.logical, nk).ok());
                if keyed.as_deref() != Some(rel.as_str()) {
                    return Err(Unread::Swapped);
                }
            }
        }
        Ok(plain.to_vec())
    })();
    eravon_os::wipe(&mut body);
    checked.map_err(no)
}

pub fn opens_under_key(key: &[u8; 32], bytes: &[u8]) -> bool {
    match parse_head(bytes) {
        Ok(h) => crate::crypto::xchacha_open(key, &h.nonce, h.aad, h.sealed).map(|mut b| eravon_os::wipe(&mut b)).is_some(),
        Err(_) => false,
    }
}

pub fn open_staged(key: &LocalKey, doc: Doc, bytes: &[u8]) -> Option<Vec<u8>> {
    let h = parse_head(bytes).ok()?;
    if h.tag != doc.tag().as_bytes() {
        return None;
    }
    let mut body = crate::crypto::xchacha_open(key.bytes(), &h.nonce, h.aad, h.sealed)?;
    let out = unframe(&body).map(|(_, _, p)| p.to_vec());
    eravon_os::wipe(&mut body);
    out
}

fn nonce() -> Result<[u8; NONCE], Trouble> {
    let mut n = [0u8; NONCE];
    crate::trouble::fill_random(&mut n)?;
    Ok(n)
}

pub fn is_sealed(bytes: &[u8]) -> bool {
    eravon_glue::sealed::is_sealed(bytes)
}

pub fn seal_with(key: &LocalKey, id: &Ident, plain: &[u8]) -> Result<Vec<u8>, Trouble> {
    envelope(key.bytes(), id, &nonce()?, plain)
}

pub fn open_with(key: &LocalKey, ex: &Expect, bytes: &[u8], what: &str, nk: Option<&NameKey>) -> Result<Vec<u8>, Trouble> {
    let doc = ex.doc;
    let h = head(doc);
    unseal_headed(key.bytes(), ex, &h, bytes, what, nk)
}

pub fn seal_checked(key: &LocalKey, id: &Ident, plain: &[u8]) -> Result<Vec<u8>, Trouble> {
    let doc = id.doc;
    if let Some(shape) = doc.canonical_shape() {
        if let Err(t) = eravon::json::parse(plain) {
            return Err(Trouble::new(shape, format!("{} {t:?}", doc.tag())));
        }
    }
    seal_with(key, id, plain)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub number: String,
    pub whose: Option<(String, String)>,
}

impl Label {
    pub fn new(whose: Option<(String, String)>) -> Result<Label, Trouble> {
        let mut n = [0u8; 16];
        crate::trouble::fill_random(&mut n)?;
        Ok(Label { number: eravon::hexfmt::encode(&n).trim_start_matches("0x").to_ascii_lowercase(), whose })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        use eravon::json::Value;
        let whose = match &self.whose {
            Some((id, seat)) => Value::Obj(vec![("identity".into(), Value::Str(id.clone())), ("seat".into(), Value::Str(seat.clone()))]),
            None => Value::Str("none".into()),
        };
        eravon::json::canon_bytes(&Value::Obj(vec![("home".into(), Value::Str(self.number.clone())), ("owner".into(), whose)]))
    }

    pub fn parse(b: &[u8]) -> Option<Label> {
        use eravon::json::Value;
        let v = eravon::json::parse(b).ok()?;
        let number = v.member("home")?.as_str()?.to_string();
        if !is_home_number(&number) {
            return None;
        }
        let whose = match v.member("owner")? {
            Value::Str(s) if s == "none" => None,
            o @ Value::Obj(_) => {
                let id = o.member("identity")?.as_str()?.to_string();
                let seat = SEATS.iter().find(|r| Some(**r) == o.member("seat").and_then(|s| s.as_str()))?.to_string();
                Some((id, seat))
            }
            _ => return None,
        };
        Some(Label { number, whose })
    }
}

pub fn label_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_DIR).join(LABEL_FILE)
}

pub fn label_rel() -> String {
    format!("{}/{LABEL_FILE}", SETTINGS_DIR)
}

pub fn label_ident() -> Ident {
    Ident { owner: Owner::Label, doc: Doc::HomeLabel, logical: label_rel() }
}

pub fn read_label(root: &Path, key: &LocalKey) -> Result<Option<Label>, Trouble> {
    let at = label_path(root);
    let bytes = match std::fs::read(&at) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Trouble::io(&e, &at.display().to_string())),
    };
    let ex = Expect { owner: Some(Owner::Label), doc: Doc::HomeLabel, rel: Some(label_rel()) };
    let plain = open_with(key, &ex, &bytes, &at.display().to_string(), None)?;
    let l = Label::parse(&plain).ok_or_else(|| Trouble::new(Code::SettingsShape, format!("{} {}", Doc::HomeLabel.tag(), at.display())))?;
    Ok(Some(l))
}

pub fn logical_rel(doc: Doc, rel: &str, plain: &[u8]) -> Result<String, Trouble> {
    let _ = (doc, plain);
    Ok(rel.to_string())
}

pub fn disk_rel(doc: Doc, logical: &str, nk: &NameKey) -> Result<String, Trouble> {
    let bad = || Trouble::new(Code::LocalSeal, format!("{} {logical}", doc.tag()));
    if !doc.keyed() {
        return Ok(logical.to_string());
    }
    let _ = nk;
    Err(bad())
}

