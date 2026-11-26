use std::collections::BTreeSet;

use eravon_galeed as k1;
use eravon_galeed::Value;

pub const FILE: &str = "standard-emitters.json";

pub const FORM: &str = "eravon.standard-emitters/1";

pub type Record = BTreeSet<(u64, String)>;

pub fn known() -> Record {
    crate::judged::kept(FILE).map(|b| read(&b)).unwrap_or_default()
}

pub fn keep(chain: u64, addrs: &[String]) {
    if addrs.is_empty() {
        return;
    }
    crate::judged::keep(FILE, |now| {
        let mut all = now.map(|b| read(&b)).unwrap_or_default();
        let before = all.len();
        all.extend(addrs.iter().map(|a| (chain, a.to_ascii_lowercase())));
        if all.len() == before {
            return None;
        }
        write(&all)
    });
}

pub fn write(all: &Record) -> Option<Vec<u8>> {
    let rows = all.iter().map(|(chain, a)| Value::Obj(vec![("address".to_string(), Value::Str(a.clone())), ("chainId".to_string(), Value::Int(*chain))])).collect();
    k1::canon::canonical(&Value::Obj(vec![("emitters".to_string(), Value::Arr(rows)), ("form".to_string(), Value::Str(FORM.to_string()))])).ok()
}

pub fn read(bytes: &[u8]) -> Record {
    let mut out = Record::new();
    let Ok(v) = k1::canon::read(bytes) else { return out };
    if v.get("form").and_then(Value::as_str) != Some(FORM) {
        return out;
    }
    for row in v.get("emitters").and_then(Value::as_arr).cloned().unwrap_or_default() {
        if let (Some(Value::Int(c)), Some(a)) = (row.get("chainId"), row.get("address").and_then(Value::as_str)) {
            out.insert((*c, a.to_ascii_lowercase()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_emitter_record_reads_back_what_it_wrote() {
        let all: Record = [(1, "0xab".to_string()), (11155111, "0xcd".to_string())].into_iter().collect();
        assert_eq!(read(&write(&all).expect("写得成")), all);
        assert!(read(b"{\"emitters\":[],\"form\":\"other/1\"}").is_empty());
    }
}
