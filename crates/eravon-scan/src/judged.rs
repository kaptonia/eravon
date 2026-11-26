use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use eravon_galeed as k1;
use eravon_galeed::Value;

pub const FILE: &str = "anchors.json";

pub const FORM: &str = "eravon.judged-anchors/1";

pub type Room = dyn eravon_send::checked::Room<Fail = eravon_seal::Trouble>;

static ROOM: Mutex<Option<Arc<dyn Fn() -> Option<Arc<Room>> + Send + Sync>>> = Mutex::new(None);

pub fn keep_in(room: Option<Arc<Room>>) {
    *ROOM.lock().unwrap_or_else(|e| e.into_inner()) = room.map(|r| Arc::new(move || Some(r.clone())) as Arc<dyn Fn() -> Option<Arc<Room>> + Send + Sync>);
}

pub fn keep_by(find: fn() -> Option<Arc<Room>>) {
    *ROOM.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(find));
}

fn room() -> Option<Arc<Room>> {
    if eravon_chain::action::keeper().is_none() || eravon_chain::recall::current().is_some() {
        return None;
    }
    let find = ROOM.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
    find()
}

pub fn kept(file: &str) -> Option<Vec<u8>> {
    room()?.read(file)
}

pub fn keep(file: &str, merge: impl FnOnce(Option<Vec<u8>>) -> Option<Vec<u8>>) {
    let Some(r) = room() else { return };
    let _held = r.hold(file);
    if let Some(bytes) = merge(r.read(file)) {
        let _ = r.put(file, &bytes);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub chain: u64,
    pub block_hash: String,
    pub tx: String,
    pub log_index: u64,
}

impl Key {
    pub fn of(chain: u64, block_hash: &str, tx: &str, log_index: u64) -> Key {
        Key { chain, block_hash: block_hash.to_ascii_lowercase(), tx: tx.to_ascii_lowercase(), log_index }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Judged {
    Shows(k1::audit::Record),
    Nothing(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub anchors: BTreeMap<Key, Judged>,
    pub codes: BTreeMap<(u64, String, u64), String>,
}

pub fn known() -> Record {
    room().and_then(|r| r.read(FILE)).map(|b| read(&b)).unwrap_or_default()
}

pub fn add(new: &Record) {
    if new.anchors.is_empty() && new.codes.is_empty() {
        return;
    }
    let Some(r) = room() else { return };
    let _held = r.hold(FILE);
    let mut all = r.read(FILE).map(|b| read(&b)).unwrap_or_default();
    let before = (all.anchors.len(), all.codes.len());
    for (k, j) in &new.anchors {
        all.anchors.entry(k.clone()).or_insert_with(|| j.clone());
    }
    for (k, c) in &new.codes {
        all.codes.entry(k.clone()).or_insert_with(|| c.clone());
    }
    if (all.anchors.len(), all.codes.len()) == before {
        return;
    }
    if let Some(bytes) = write(&all) {
        let _ = r.put(FILE, &bytes);
    }
}

fn write(all: &Record) -> Option<Vec<u8>> {
    let anchors = all
        .anchors
        .iter()
        .map(|(k, j)| {
            let mut m = vec![
                ("blockHash".to_string(), Value::Str(k.block_hash.clone())),
                ("chainId".to_string(), Value::Int(k.chain)),
                ("logIndex".to_string(), Value::Int(k.log_index)),
                ("tx".to_string(), Value::Str(k.tx.clone())),
            ];
            match j {
                Judged::Shows(r) => m.push(("shows".to_string(), crate::ledger::record_value(r))),
                Judged::Nothing(why) => m.push(("nothing".to_string(), Value::Str(why.clone()))),
            }
            Value::Obj(m)
        })
        .collect();
    let codes = all
        .codes
        .iter()
        .map(|((chain, addr, block), hash)| {
            Value::Obj(vec![
                ("address".to_string(), Value::Str(addr.clone())),
                ("block".to_string(), Value::Int(*block)),
                ("chainId".to_string(), Value::Int(*chain)),
                ("codeHash".to_string(), Value::Str(hash.clone())),
            ])
        })
        .collect();
    k1::canon::canonical(&Value::Obj(vec![
        ("anchors".to_string(), Value::Arr(anchors)),
        ("codes".to_string(), Value::Arr(codes)),
        ("form".to_string(), Value::Str(FORM.to_string())),
    ]))
    .ok()
}

fn read(bytes: &[u8]) -> Record {
    let mut out = Record::default();
    let Ok(v) = k1::canon::read(bytes) else { return out };
    if v.get("form").and_then(Value::as_str) != Some(FORM) {
        return out;
    }
    let int = |x: Option<&Value>| match x {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    };
    let rows = |k: &str| v.get(k).and_then(Value::as_arr).map(Vec::as_slice).unwrap_or(&[]).to_vec();
    for row in rows("anchors") {
        let key = (|| Some(Key::of(int(row.get("chainId"))?, row.get("blockHash")?.as_str()?, row.get("tx")?.as_str()?, int(row.get("logIndex"))?)))();
        let judged = match (row.get("shows"), row.get("nothing").and_then(Value::as_str)) {
            (Some(r), None) => crate::ledger::record_read(r).map(Judged::Shows),
            (None, Some(why)) => Some(Judged::Nothing(why.to_string())),
            _ => None,
        };
        if let (Some(k), Some(j)) = (key, judged) {
            out.anchors.insert(k, j);
        }
    }
    for row in rows("codes") {
        let got = (|| Some(((int(row.get("chainId"))?, row.get("address")?.as_str()?.to_ascii_lowercase(), int(row.get("block"))?), row.get("codeHash")?.as_str()?.to_ascii_lowercase())))();
        if let Some((k, h)) = got {
            out.codes.insert(k, h);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Mem(Mutex<BTreeMap<String, Vec<u8>>>);

    impl eravon_send::checked::Room for Mem {
        type Fail = eravon_seal::Trouble;
        fn read(&self, file: &str) -> Option<Vec<u8>> {
            self.0.lock().unwrap().get(file).cloned()
        }
        fn put(&self, file: &str, plain: &[u8]) -> Result<(), eravon_seal::Trouble> {
            self.0.lock().unwrap().insert(file.to_string(), plain.to_vec());
            Ok(())
        }
    }

    struct Nowhere;

    impl eravon_chain::settled::Keep for Nowhere {
        fn load(&self, _part: &str) -> Option<Vec<u8>> {
            None
        }
        fn save(&self, _part: &str, _plain: &[u8]) -> Result<(), eravon_chain::settled::Unkept> { Ok(()) }
    }

    #[test]
    fn a_judged_anchor_is_kept_and_read_back_and_only_where_the_action_keeps_reads() {
        let room: Arc<Mem> = Default::default();
        keep_in(Some(room.clone()));
        let a = Key::of(1, "0xAA", "0xBB", 3);
        let b = Key::of(1, "0xaa", "0xcc", 0);
        let rec = k1::audit::Record { chain_id: 1, block_number: 9, block_timestamp: 99, tx: "0xbb".into(), sender: "0x11".into(), hash: "0x22".into(), verdict: k1::audit::Verdict::Counted, entry: Some(vec![1, 2]) };
        let one = |k: &Key, j: Judged| Record { anchors: BTreeMap::from([(k.clone(), j)]), codes: BTreeMap::new() };
        {
            let _none = eravon_chain::action::open(None);
            add(&one(&a, Judged::Shows(rec.clone())));
            assert!(known() == Record::default() && room.0.lock().unwrap().is_empty(), "no keeper: nothing read or written");
        }
        let _kept = eravon_chain::action::open(Some(Arc::new(Nowhere)));
        add(&one(&a, Judged::Shows(rec.clone())));
        add(&one(&b, Judged::Nothing("not carried".into())));
        add(&Record { anchors: BTreeMap::new(), codes: BTreeMap::from([((1, "0xee".to_string(), 9), "0xcc".to_string())]) });
        let got = known();
        assert_eq!(got.anchors.get(&a), Some(&Judged::Shows(rec)));
        assert_eq!(got.anchors.get(&b), Some(&Judged::Nothing("not carried".into())));
        assert_eq!(got.codes.get(&(1, "0xee".to_string(), 9)).map(String::as_str), Some("0xcc"));
        keep("other.json", |now| now.is_none().then(|| b"one".to_vec()));
        keep("other.json", |now| now.map(|b| [b, b"+two".to_vec()].concat()));
        assert_eq!(kept("other.json").as_deref(), Some(&b"one+two"[..]));
        keep_in(None);
        assert_eq!(kept("other.json"), None, "no room: nothing");
    }
}
