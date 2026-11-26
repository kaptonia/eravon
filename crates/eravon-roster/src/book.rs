use eravon_galeed as k1;
use eravon_terms_free::as_int;

use crate::mark;

mod eravon_terms_free {
    use eravon_galeed::Value;

    pub fn as_int(v: &Value) -> Option<u64> {
        match v {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelRecord {
    pub index: u64,
    pub panel_hash: String,
    pub roster_hash: String,
    pub expires: u64,
    pub recorded: u64,
    pub router: String,
    pub router_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Engaged {
    pub index: u64,
    pub members: Vec<String>,
    pub amanah_hashes: Vec<String>,
    pub roster_hash: String,
    pub expires: u64,
    pub router: String,
}

pub trait PanelBook {
    fn root_of(&self, guild: &str) -> Result<String, String>;
    fn count(&self, root: &str, terms_hash: &str) -> Result<u64, String>;
    fn panel_at(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<PanelRecord>, String>;
    fn engaged(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<Engaged>, String>;
}

pub struct FixtureBook {
    records: Vec<(String, String, Engaged, PanelRecord)>,
}

const ZERO20: &str = "0x0000000000000000000000000000000000000000";
const ZERO32: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

impl FixtureBook {
    pub fn from_bytes(b: &[u8]) -> Option<FixtureBook> {
        mark::mark(mark::FEATURE);
        let v = k1::canon::read_open(b)?;
        let rows = match v.get("records")? {
            k1::Value::Arr(a) => a,
            _ => return None,
        };
        let mut records = Vec::with_capacity(rows.len());
        for r in rows {
            let root = r.get("root")?.as_str()?.to_ascii_lowercase();
            let terms_hash = r.get("termsHash")?.as_str()?.to_string();
            let members = strs(r.get("members")?)?;
            let amanah_hashes = strs(r.get("amanahHashes")?)?;
            if !members.iter().all(|m| k1::hexform::is_hex20(m))
                || !amanah_hashes.iter().all(|h| k1::hexform::is_hex32(h))
            {
                return None;
            }
            let roster_hash = r.get("rosterHash")?.as_str()?.to_string();
            let expires = as_int(r.get("expires")?)?;
            let index = as_int(r.get("index")?)?;
            let recorded = match r.get("recorded") {
                Some(x) => as_int(x)?,
                None => 0,
            };
            let opt = |k: &str, d: &str| -> Option<String> {
                match r.get(k) {
                    Some(x) => Some(x.as_str()?.to_ascii_lowercase()),
                    None => Some(d.to_string()),
                }
            };
            let router = opt("router", ZERO20)?;
            let router_hash = opt("routerHash", ZERO32)?;
            if !k1::hexform::is_hex20(&router) || !k1::hexform::is_hex32(&router_hash) {
                return None;
            }
            let panel_hash = match r.get("panelHash") {
                Some(x) => x.as_str()?.to_string(),
                None => crate::pool::panel_hash(&members, &amanah_hashes)?,
            };
            if records.iter().any(|(g, t, e, _): &(String, String, Engaged, PanelRecord)| *g == root && *t == terms_hash && e.index == index) {
                return None;
            }
            let rec = PanelRecord { index, panel_hash, roster_hash: roster_hash.clone(), expires, recorded, router: router.clone(), router_hash };
            records.push((root, terms_hash, Engaged { index, members, amanah_hashes, roster_hash, expires, router }, rec));
        }
        Some(FixtureBook { records })
    }

    fn rows_of(&self, root: &str, terms_hash: &str) -> Vec<&(String, String, Engaged, PanelRecord)> {
        self.records.iter().filter(|(g, t, _, _)| g.eq_ignore_ascii_case(root) && t == terms_hash).collect()
    }
}

fn strs(v: &k1::Value) -> Option<Vec<String>> {
    match v {
        k1::Value::Arr(a) => a.iter().map(|x| x.as_str().map(str::to_string)).collect(),
        _ => None,
    }
}

impl PanelBook for FixtureBook {
    fn root_of(&self, guild: &str) -> Result<String, String> {
        Ok(guild.to_ascii_lowercase())
    }

    fn count(&self, root: &str, terms_hash: &str) -> Result<u64, String> {
        Ok(self.rows_of(root, terms_hash).len() as u64)
    }

    fn panel_at(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<PanelRecord>, String> {
        Ok(self.rows_of(root, terms_hash).iter().find(|(_, _, e, _)| e.index == index).map(|(_, _, _, r)| r.clone()))
    }

    fn engaged(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<Engaged>, String> {
        Ok(self.rows_of(root, terms_hash).iter().find(|(_, _, e, _)| e.index == index).map(|(_, _, e, _)| e.clone()))
    }
}
