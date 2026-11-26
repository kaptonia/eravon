use std::collections::BTreeMap;

use eravon_galeed as k1;
use eravon_galeed::Value;

pub const FILE: &str = "block-times.json";

pub const FORM: &str = "eravon.block-times/1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub hash: String,
    pub time: u64,
}

pub type Record = BTreeMap<(u64, u64), Kept>;

pub fn known() -> Record {
    crate::judged::kept(FILE).map(|b| read(&b)).unwrap_or_default()
}

pub fn time_in(all: &Record, chain: u64, number: u64, hash: Option<&str>) -> Option<u64> {
    let k = all.get(&(chain, number))?;
    match hash {
        Some(h) if !k.hash.eq_ignore_ascii_case(h) => None,
        _ => Some(k.time),
    }
}

pub fn time_of(chain: u64, number: u64, hash: Option<&str>) -> Option<u64> {
    time_in(&known(), chain, number, hash)
}

pub fn keep(chain: u64, number: u64, hash: &str, time: u64) {
    let k = Kept { hash: hash.to_ascii_lowercase(), time };
    crate::judged::keep(FILE, |now| {
        let mut all = now.map(|b| read(&b)).unwrap_or_default();
        if all.get(&(chain, number)) == Some(&k) {
            return None;
        }
        all.insert((chain, number), k.clone());
        write(&all)
    });
}

pub fn write(all: &Record) -> Option<Vec<u8>> {
    let rows = all
        .iter()
        .map(|((chain, number), k)| {
            Value::Obj(vec![
                ("block".to_string(), Value::Int(*number)),
                ("chainId".to_string(), Value::Int(*chain)),
                ("hash".to_string(), Value::Str(k.hash.clone())),
                ("time".to_string(), Value::Int(k.time)),
            ])
        })
        .collect();
    k1::canon::canonical(&Value::Obj(vec![("blocks".to_string(), Value::Arr(rows)), ("form".to_string(), Value::Str(FORM.to_string()))])).ok()
}

pub fn read(bytes: &[u8]) -> Record {
    let mut out = Record::new();
    let Ok(v) = k1::canon::read(bytes) else { return out };
    if v.get("form").and_then(Value::as_str) != Some(FORM) {
        return out;
    }
    for row in v.get("blocks").and_then(Value::as_arr).cloned().unwrap_or_default() {
        let int = |k: &str| match row.get(k) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        };
        if let (Some(chain), Some(number), Some(hash), Some(time)) = (int("chainId"), int("block"), row.get("hash").and_then(Value::as_str), int("time")) {
            out.insert((chain, number), Kept { hash: hash.to_ascii_lowercase(), time });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_reads_back_what_it_wrote() {
        let mut all = Record::new();
        all.insert((1, 7), Kept { hash: "0xab".into(), time: 1_700_000_084 });
        all.insert((11155111, 9), Kept { hash: "0xcd".into(), time: 1_700_000_108 });
        let bytes = write(&all).expect("写得成");
        assert_eq!(read(&bytes), all);
        assert!(read(b"{\"blocks\":[],\"form\":\"other/1\"}").is_empty());
    }
}
