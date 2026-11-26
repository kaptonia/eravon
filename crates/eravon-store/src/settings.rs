use eravon_literals::json::{In, Out};

use crate::doc;
use crate::fault::R;
use crate::seam;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Global {
    pub seller_ready: bool,
    pub buyer_ready: bool,
    pub last_writer: String,
    pub export_dir: String,
    pub tokens: Option<Vec<TokenRow>>,
    pub refresh_every: Option<u64>,
    pub past_cases: bool,
    pub extra: Vec<(String, In)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenRow {
    pub chain: u64,
    pub address: String,
    pub label: String,
    pub decimals: Option<u8>,
    pub source: String,
}

impl TokenRow {
    fn face(&self) -> Out {
        Out::O(vec![
            ("address", Out::S(self.address.clone())),
            ("chainId", Out::N(self.chain)),
            ("decimals", self.decimals.map_or(Out::Null, |d| Out::N(u64::from(d)))),
            ("decimalsSource", Out::S(self.source.clone())),
            ("label", Out::S(self.label.clone())),
        ])
    }

    fn of(v: &In) -> R<TokenRow> {
        let decimals = match v.get("decimals") {
            None | Some(In::Null) => None,
            Some(In::Int(d)) if *d >= 0 => Some(u8::try_from(*d).unwrap_or(u8::MAX)),
            Some(other) => return Err(crate::fault::Fault::Form(format!("tokens 一行的 decimals 要是非负整数或 null,档上是 {other:?}"))),
        };
        Ok(TokenRow {
            chain: doc::need_u64(v, "chainId")?,
            address: doc::need_str(v, "address")?,
            label: doc::need_str(v, "label")?,
            decimals,
            source: doc::need_str(v, "decimalsSource")?,
        })
    }
}

impl Default for Global {
    fn default() -> Global {
        Global {
            seller_ready: false,
            buyer_ready: false,
            last_writer: String::new(),
            export_dir: String::new(),
            tokens: None,
            refresh_every: None,
            past_cases: false,
            extra: Vec::new(),
        }
    }
}

const GLOBAL_MEMBERS: [&str; 8] = ["buyerReady", "exportDir", "lastWriter", "pastCases", "refreshEvery", "sellerReady", "tokens", "v"];

const IDENTITY_MEMBERS: [&str; 11] = ["company", "companyMode", "companyPast", "former", "handOn", "payout", "swept", "timeCheck", "v", "zikaronDir", "zikaronLink"];

fn unknown_members(v: &In, known: &[&str]) -> Vec<(String, In)> {
    match v {
        In::Obj(m) => m.iter().filter(|(k, _)| !known.contains(&k.as_str())).cloned().collect(),
        _ => Vec::new(),
    }
}

fn with_extra(bytes: Vec<u8>, extra: &[(String, In)]) -> Vec<u8> {
    if extra.is_empty() {
        return bytes;
    }
    let Ok(In::Obj(mut m)) = eravon_literals::json::read(&bytes) else { return bytes };
    for (k, v) in extra {
        if !m.iter().any(|(n, _)| n == k) {
            m.push((k.clone(), v.clone()));
        }
    }
    eravon_literals::json::write_in(&In::Obj(m))
}

impl Global {
    pub fn seal(&self) -> Vec<u8> {
        let mut m = vec![
            ("buyerReady", Out::B(self.buyer_ready)),
            ("lastWriter", Out::S(self.last_writer.clone())),
            ("sellerReady", Out::B(self.seller_ready)),
        ];
        if !self.export_dir.is_empty() {
            m.push(("exportDir", Out::S(self.export_dir.clone())));
        }
        if let Some(rows) = &self.tokens {
            m.push(("tokens", Out::A(rows.iter().map(TokenRow::face).collect())));
        }
        if let Some(n) = self.refresh_every {
            m.push(("refreshEvery", Out::N(n)));
        }
        if self.past_cases {
            m.push(("pastCases", Out::B(true)));
        }
        with_extra(doc::seal(seam::V_SETTINGS, m), &self.extra)
    }

    pub fn open(bytes: &[u8]) -> R<Global> {
        Global::fields(&doc::open(seam::V_SETTINGS, bytes)?)
    }

    fn fields(v: &In) -> R<Global> {
        Ok(Global {
            seller_ready: need_bool(v, "sellerReady")?,
            buyer_ready: need_bool(v, "buyerReady")?,
            last_writer: doc::need_str(v, "lastWriter")?,
            export_dir: match v.get("exportDir") {
                None => String::new(),
                Some(_) => doc::need_str(v, "exportDir")?,
            },
            tokens: match v.get("tokens") {
                None | Some(In::Null) => None,
                Some(_) => Some(doc::need_arr(v, "tokens")?.iter().map(TokenRow::of).collect::<R<Vec<_>>>()?),
            },
            refresh_every: match v.get("refreshEvery") {
                None => None,
                Some(_) => Some(doc::need_u64(v, "refreshEvery")?),
            },
            past_cases: match v.get("pastCases") {
                None => false,
                Some(_) => need_bool(v, "pastCases")?,
            },
            extra: unknown_members(v, &GLOBAL_MEMBERS),
        })
    }
}

fn need_bool(v: &In, key: &str) -> R<bool> {
    match v.get(key) {
        Some(In::Bool(b)) => Ok(*b),
        Some(other) => {
            Err(crate::fault::Fault::Form(format!("{key} 要是真假,档上是 {other:?}")))
        }
        None => Err(crate::fault::Fault::Form(format!("档上缺 {key}"))),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Identity {
    pub payout: String,
    pub zikaron_dir: String,
    pub zikaron_link: Option<bool>,
    pub company_mode: Option<bool>,
    pub company: String,
    pub time_check: Option<bool>,
    pub company_past: Vec<String>,
    pub former: Vec<String>,
    pub swept: Vec<String>,
    pub hand_on: Vec<HandOn>,
    pub extra: Vec<(String, In)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HandOn {
    pub chain: u64,
    pub from: String,
    pub kind: String,
    pub id: String,
    pub digest: String,
    pub logic: String,
    pub how: String,
    pub landed: bool,
}

pub mod hand_on {
    pub const ORDER: &str = "order";
    pub const OFFER: &str = "offer";
    pub const TERMS: &str = "terms";
    pub const REGISTER: &str = "register";
    pub const EXPIRE: &str = "expire";
    pub const REVOKE: &str = "revoke";
}

impl HandOn {
    fn out(&self) -> Out {
        let mut m = vec![
            ("chain", Out::N(self.chain)),
            ("digest", Out::S(self.digest.clone())),
            ("from", Out::S(self.from.clone())),
            ("how", Out::S(self.how.clone())),
            ("id", Out::S(self.id.clone())),
            ("kind", Out::S(self.kind.clone())),
            ("landed", Out::B(self.landed)),
        ];
        if !self.logic.is_empty() {
            m.push(("logic", Out::S(self.logic.clone())));
        }
        Out::O(m)
    }

    fn open(x: &In) -> R<HandOn> {
        let bad = |why: String| crate::fault::Fault::Form(format!("handOn 的一项{why}"));
        let In::Obj(m) = x else { return Err(bad(format!("要是对象,档上是 {x:?}"))) };
        let get = |k: &str| m.iter().find(|(n, _)| n == k).map(|(_, v)| v);
        let text = |k: &str, need: bool| -> R<String> {
            match get(k) {
                Some(In::Str(s)) => Ok(s.clone()),
                None if !need => Ok(String::new()),
                other => Err(bad(format!("的 {k} 要是字符串,档上是 {other:?}"))),
            }
        };
        let chain = match get("chain") {
            Some(In::Int(n)) if *n > 0 => *n as u64,
            other => return Err(bad(format!("的 chain 要是正整数,档上是 {other:?}"))),
        };
        let landed = match get("landed") {
            Some(In::Bool(b)) => *b,
            other => return Err(bad(format!("的 landed 要是真假,档上是 {other:?}"))),
        };
        let h = HandOn { chain, from: text("from", true)?, kind: text("kind", true)?, id: text("id", true)?, digest: text("digest", true)?, logic: text("logic", false)?, how: text("how", true)?, landed };
        if ![hand_on::ORDER, hand_on::OFFER, hand_on::TERMS].contains(&h.kind.as_str()) || ![hand_on::REGISTER, hand_on::EXPIRE, hand_on::REVOKE].contains(&h.how.as_str()) {
            return Err(bad(format!("的 kind 或 how 不认得:{} {}", h.kind, h.how)));
        }
        Ok(h)
    }
}

impl Identity {
    pub fn zikaron_on(&self) -> bool {
        self.zikaron_link.unwrap_or(eravon_literals::constants::ZIKARON_LINK_DEFAULT_ON)
    }

    pub fn company_on(&self) -> bool {
        self.company_mode.unwrap_or(eravon_literals::constants::COMPANY_MODE_DEFAULT_ON)
    }

    pub fn time_check_on(&self) -> bool {
        self.time_check.unwrap_or(eravon_literals::constants::TIME_CHECK_DEFAULT_ON)
    }

    pub fn party(&self, me: &str) -> String {
        if self.company_on() && !self.company.is_empty() {
            self.company.to_ascii_lowercase()
        } else {
            me.to_ascii_lowercase()
        }
    }

    pub fn move_company(&mut self, to: &str) {
        let to = to.to_ascii_lowercase();
        let old = self.company.to_ascii_lowercase();
        if !old.is_empty() && old != to && !self.company_past.contains(&old) {
            self.company_past.push(old);
        }
        self.company = to;
    }

    pub fn seen(&self, me: &str) -> Vec<String> {
        let me = me.to_ascii_lowercase();
        let mut out = vec![me.clone()];
        let more = std::iter::once(self.party(&me)).chain(self.company_past.iter().map(|c| c.to_ascii_lowercase())).chain(self.former.iter().map(|c| c.to_ascii_lowercase()));
        for k in more {
            if !k.is_empty() && !out.contains(&k) {
                out.push(k);
            }
        }
        out
    }

    pub fn take_over(&mut self, old_key: &str, old: &Identity) {
        self.former = former_after(old_key, old, &self.former);
        if self.payout.is_empty() {
            self.payout = old.payout.clone();
        }
        if self.zikaron_dir.is_empty() {
            self.zikaron_dir = old.zikaron_dir.clone();
        }
        self.zikaron_link = self.zikaron_link.or(old.zikaron_link);
        self.time_check = self.time_check.or(old.time_check);
        self.company_mode = self.company_mode.or(old.company_mode);
        if self.company.is_empty() {
            self.company = old.company.clone();
        }
        for c in &old.company_past {
            let c = c.to_ascii_lowercase();
            if !self.company_past.contains(&c) && !c.eq_ignore_ascii_case(&self.company) {
                self.company_past.push(c);
            }
        }
        for (k, v) in &old.extra {
            if !self.extra.iter().any(|(n, _)| n == k) {
                self.extra.push((k.clone(), v.clone()));
            }
        }
    }
}

fn keys_before(old_key: &str, old: &Identity) -> Vec<String> {
    old.former.iter().map(|k| k.to_ascii_lowercase()).chain(std::iter::once(old_key.to_ascii_lowercase())).collect()
}

fn former_after(old_key: &str, old: &Identity, mine: &[String]) -> Vec<String> {
    let mut former: Vec<String> = Vec::new();
    for k in keys_before(old_key, old).into_iter().chain(mine.iter().map(|k| k.to_ascii_lowercase())) {
        if !k.is_empty() && !former.contains(&k) {
            former.push(k);
        }
    }
    former
}


fn strs(v: &[String]) -> Out {
    Out::A(v.iter().map(|s| Out::S(s.clone())).collect())
}

fn need_strs(v: &In, key: &str) -> R<Vec<String>> {
    let mut out = Vec::new();
    for x in doc::need_arr(v, key)? {
        match x {
            In::Str(s) => out.push(s.clone()),
            other => {
                return Err(crate::fault::Fault::Form(format!(
                    "{key} 的每一项要是字符串,档上有 {other:?}"
                )));
            }
        }
    }
    Ok(out)
}

impl Identity {
    pub fn seal(&self) -> Vec<u8> {
        let mut members = vec![("payout", Out::S(self.payout.clone()))];
        if !self.zikaron_dir.is_empty() {
            members.push(("zikaronDir", Out::S(self.zikaron_dir.clone())));
        }
        if let Some(z) = self.zikaron_link {
            members.push(("zikaronLink", Out::B(z)));
        }
        if let Some(m) = self.company_mode {
            members.push(("companyMode", Out::B(m)));
        }
        if !self.company.is_empty() {
            members.push(("company", Out::S(self.company.clone())));
        }
        if let Some(t) = self.time_check {
            members.push(("timeCheck", Out::B(t)));
        }
        if !self.company_past.is_empty() {
            members.push(("companyPast", strs(&self.company_past)));
        }
        if !self.former.is_empty() {
            members.push(("former", strs(&self.former)));
        }
        if !self.swept.is_empty() {
            members.push(("swept", strs(&self.swept)));
        }
        if !self.hand_on.is_empty() {
            members.push(("handOn", Out::A(self.hand_on.iter().map(HandOn::out).collect())));
        }
        with_extra(doc::seal(seam::V_IDENTITY_SETTINGS, members), &self.extra)
    }

    pub fn open(bytes: &[u8]) -> R<Identity> {
        let v = doc::open(seam::V_IDENTITY_SETTINGS, bytes)?;
        Ok(Identity {
            payout: doc::need_str(&v, "payout")?,
            zikaron_dir: match v.get("zikaronDir") {
                None => String::new(),
                Some(In::Str(s)) => s.clone(),
                Some(other) => {
                    return Err(crate::fault::Fault::Form(format!("zikaronDir 要是字符串,档上是 {other:?}")));
                }
            },
            zikaron_link: match v.get("zikaronLink") {
                None => None,
                Some(In::Bool(b)) => Some(*b),
                Some(other) => {
                    return Err(crate::fault::Fault::Form(format!("zikaronLink 要是真假,档上是 {other:?}")));
                }
            },
            company_mode: opt_bool(&v, "companyMode")?,
            company: match v.get("company") {
                None => String::new(),
                Some(In::Str(s)) => s.clone(),
                Some(other) => {
                    return Err(crate::fault::Fault::Form(format!("company 要是字符串,档上是 {other:?}")));
                }
            },
            time_check: opt_bool(&v, "timeCheck")?,
            company_past: match v.get("companyPast") {
                None => Vec::new(),
                Some(_) => need_strs(&v, "companyPast")?,
            },
            former: match v.get("former") {
                None => Vec::new(),
                Some(_) => need_strs(&v, "former")?,
            },
            swept: match v.get("swept") {
                None => Vec::new(),
                Some(_) => need_strs(&v, "swept")?,
            },
            hand_on: match v.get("handOn") {
                None => Vec::new(),
                Some(_) => doc::need_arr(&v, "handOn")?.iter().map(HandOn::open).collect::<R<Vec<_>>>()?,
            },
            extra: unknown_members(&v, &IDENTITY_MEMBERS),
        })
    }
}

fn opt_bool(v: &In, key: &str) -> R<Option<bool>> {
    match v.get(key) {
        None => Ok(None),
        Some(In::Bool(b)) => Ok(Some(*b)),
        Some(other) => Err(crate::fault::Fault::Form(format!("{key} 要是真假,档上是 {other:?}"))),
    }
}

#[cfg(test)]
mod zikaron_link_tests {
    use super::*;

    #[test]
    fn the_company_keys_are_written_only_once_touched_and_read_back() {
        let plain = Identity::default();
        let b = plain.seal();
        let t = String::from_utf8(b.clone()).unwrap();
        assert!(!t.contains("companyMode") && !t.contains("timeCheck") && !t.contains("\"company\""));
        assert!(!plain.company_on() && plain.time_check_on());
        let set = Identity { company_mode: Some(true), company: "0x".to_string() + &"ab".repeat(20), time_check: Some(false), ..Identity::default() };
        let back = Identity::open(&set.seal()).unwrap();
        assert_eq!(back, set);
        assert!(back.company_on() && !back.time_check_on());
    }

    #[test]
    fn a_record_moved_twice_reads_every_key_and_keeps_its_company() {
        let k = |b: &str| "0x".to_string() + &b.repeat(20);
        let (a, b, c, co, co0) = (k("aa"), k("bb"), k("cc"), k("c0"), k("c9"));
        let mut tb = Identity::default();
        tb.take_over(&a, &Identity { payout: k("01"), company_mode: Some(true), company: co.clone(), company_past: vec![co0.clone()], ..Identity::default() });
        assert_eq!(tb.former, vec![a.clone()]);
        let mut tc = Identity { payout: k("02"), ..Identity::default() };
        tc.take_over(&b.to_ascii_uppercase().replace("0X", "0x"), &tb);
        assert_eq!(tc.former, vec![a.clone(), b.clone()], "root first, then each key in the order it acceded");
        assert_eq!(tc.payout, k("02"), "a member of its own stays");
        assert!(tc.company_on() && tc.company == co && tc.company_past == vec![co0.clone()]);
        assert_eq!(tc.seen(&c), vec![c.clone(), co, co0, a, b]);
        let back = Identity::open(&tc.seal()).unwrap();
        assert_eq!(back, tc);
    }

    #[test]
    fn moving_the_company_keeps_the_old_one_and_the_scan_sees_both() {
        let me = "0x".to_string() + &"11".repeat(20);
        let (a, b) = ("0x".to_string() + &"aa".repeat(20), "0x".to_string() + &"bb".repeat(20));
        let mut t = Identity { company_mode: Some(true), ..Identity::default() };
        t.move_company(&a);
        assert!(t.company_past.is_empty() && !String::from_utf8(t.seal()).unwrap().contains("companyPast"));
        assert_eq!(t.seen(&me), vec![me.clone(), a.clone()]);
        t.move_company(&a.to_ascii_uppercase().replace("0X", "0x"));
        assert!(t.company_past.is_empty());
        t.move_company(&b);
        assert_eq!(t.company_past, vec![a.clone()]);
        assert_eq!(t.seen(&me), vec![me.clone(), b.clone(), a.clone()]);
        let back = Identity::open(&t.seal()).unwrap();
        assert_eq!(back, t);
        t.move_company(&a);
        assert_eq!(t.company_past, vec![a.clone(), b.clone()]);
        assert_eq!(t.seen(&me), vec![me, a, b]);
    }

    #[test]
    fn the_link_key_is_written_only_once_touched_and_reads_back() {
        let plain = Identity::default();
        let bytes = plain.seal();
        assert!(!String::from_utf8_lossy(&bytes).contains("zikaronLink"));
        assert_eq!(Identity::open(&bytes).unwrap().zikaron_on(), eravon_literals::constants::ZIKARON_LINK_DEFAULT_ON);
        let off = Identity { zikaron_link: Some(false), ..Identity::default() };
        let back = Identity::open(&off.seal()).unwrap();
        assert_eq!(back.zikaron_link, Some(false));
        assert!(!back.zikaron_on());
    }
}

#[cfg(test)]
mod generation_tests {
    use super::*;

    const OLD: &str = r#"{"activeBuyer":"","activeSeller":"0x00000000000000000000000000000000000000a1","buyerReady":false,"contractVersion":"0x0000000000000000000000000000000000000000","displayCurrency":"USD","language":"zh","lastWriter":"old","sellerReady":true,"v":"eravon.store.settings/1"}"#;

    #[test]
    fn a_version_1_file_read_directly_is_refused_whole() {
        assert!(matches!(Global::open(OLD.as_bytes()), Err(crate::fault::Fault::Version { .. })));
        let two = OLD.replace("settings/1", "settings/2");
        assert!(matches!(Global::open(two.as_bytes()), Err(crate::fault::Fault::Version { .. })), "the second version, whose members went to the machine settings, too");
    }

    #[test]
    fn a_new_home_is_born_without_the_dropped_column() {
        let text = String::from_utf8(Global::default().seal()).unwrap();
        assert!(!text.contains("Currency") && !text.contains("language") && text.contains(r#""v":"eravon.store.settings/3""#), "{text}");
    }

    #[test]
    fn members_this_version_does_not_know_are_written_back() {
        let newer = r#"{"buyerReady":false,"contractVersion":"x","future":{"a":[1,-2,null]},"lastWriter":"w","sellerReady":false,"v":"eravon.store.settings/3"}"#;
        let mut g = Global::open(newer.as_bytes()).unwrap();
        assert_eq!(g.extra.len(), 2);
        g.last_writer = "v".into();
        let back = String::from_utf8(g.seal()).unwrap();
        assert_eq!(back, newer.replace(r#""lastWriter":"w""#, r#""lastWriter":"v""#), "the unknown member back as it came, canonical");
        let id = r#"{"later":"kept","payout":"","v":"eravon.store.identity-settings/2"}"#;
        let t = Identity::open(id.as_bytes()).unwrap();
        assert_eq!(String::from_utf8(t.seal()).unwrap(), id);
        let born = String::from_utf8(Identity::default().seal()).unwrap();
        assert_eq!(born, r#"{"payout":"","v":"eravon.store.identity-settings/2"}"#);
    }
}

