//! Test fixtures shared by the core's own tests: keys by scalar, entries signed through the judge door and written
//! by the core's writer, and audit inputs spelled in the harness transport.

use crate::cryptox::{self, Privkey};
use crate::hexform;
use crate::json::{self, Value};

pub fn key(n: u8) -> Privkey {
    let mut k = [0u8; 32];
    k[31] = n;
    Privkey::from_bytes(&k).unwrap()
}

pub fn addr(k: &Privkey) -> String {
    hexform::encode(&k.address())
}

/// One entry: the six members signed by the judge door (`sign::sign_as`), written by the core's writer. `spec` is
/// the law's literal unless `spec` says otherwise (a migrated ledger's new-spec entry).
pub fn entry_under(k: &Privkey, spec: &str, seq: u64, prev: Option<&str>, entry_type: &str, body: &str) -> Vec<u8> {
    let mut six = vec![
        ("body".to_string(), json::read(body.as_bytes()).unwrap()),
        ("entryType".to_string(), Value::Str(entry_type.into())),
        ("judge".to_string(), Value::Str(addr(k))),
        ("prev".to_string(), prev.map_or(Value::Null, |p| Value::Str(p.into()))),
        ("seq".to_string(), Value::Int(seq)),
        ("spec".to_string(), Value::Str(spec.into())),
    ];
    let signed = crate::sign::sign_as(k, &json::canonical(&Value::Obj(six.clone())).unwrap(), crate::literals::SEAT_JUDGE, crate::LAW).unwrap();
    six.push(("sig".to_string(), Value::Str(hexform::encode(&signed.sig))));
    json::canonical(&Value::Obj(six)).unwrap()
}

pub fn entry(k: &Privkey, seq: u64, prev: Option<&str>, entry_type: &str, body: &str) -> Vec<u8> {
    entry_under(k, crate::LAW, seq, prev, entry_type, body)
}

pub fn id(b: &[u8]) -> String {
    hexform::encode(&cryptox::sha256(b))
}

/// One anchor record on chain 1 in the transport spelling: block `n`, timestamp `n`, transaction word `n`, the given
/// sender, verdict and hash, and the bytes where `carried` names them.
pub fn record(n: u64, sender: &str, verdict: &str, hash: &str, carried: Option<&[u8]>) -> String {
    let entry = carried.map(|b| format!(r#","entry":"{}""#, hexform::encode(b))).unwrap_or_default();
    format!(r#"{{"chainId":1,"blockNumber":{n},"blockTimestamp":{n},"tx":"0x{n:064x}","sender":"{sender}","hash":"{hash}","verdict":"{verdict}"{entry}}}"#)
}

/// The harness input: the root, the records, and a basis of one window on chain 1 over blocks 0 to 100 whose
/// `senders` are the given addresses (ascending, deduplicated).
pub fn input(root: &str, records: &[String], senders: &[String]) -> Value {
    let mut s: Vec<String> = senders.to_vec();
    s.sort();
    s.dedup();
    let senders = s.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(",");
    let text = format!(r#"{{"root":"{root}","anchors":[{}],"basis":{{"chains":[{{"chainId":1,"fromBlock":0,"toBlock":100,"registries":[],"senders":[{senders}]}}]}}}}"#, records.join(","));
    json::read_open(text.as_bytes()).unwrap()
}
