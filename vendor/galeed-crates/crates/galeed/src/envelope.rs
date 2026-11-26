//! §4 envelope and the thirteen acceptance steps of §4.3: a verifier decides in this order and reports
//! the token of the first failure; only bytes that pass all thirteen are an entry.

#[cfg(not(feature = "ablate-k1-accept-thirteen"))]
use crate::body;
use crate::cryptox;
#[cfg(not(feature = "ablate-k1-accept-thirteen"))]
use crate::hexform;
use crate::json::{self, Value};
#[cfg(not(feature = "ablate-k1-accept-thirteen"))]
use crate::sign;
use crate::token::Token;

/// The seven members of §4.1, a closed set.
pub const KEYS: [&str; 7] = ["spec", "entryType", "judge", "seq", "prev", "body", "sig"];
pub const KEY_ENTRY_TYPE: &str = KEYS[1];
pub const KEY_JUDGE: &str = KEYS[2];
pub const KEY_SEQ: &str = KEYS[3];
pub const KEY_BODY: &str = KEYS[5];

/// **The one assembly of an entry's bytes** (§4.1, §5; B029: the command line's mint and the desk's seal both build
/// through here, never each its own seven members): `spec` the law, `entryType`, `judge` the author, `seq`, `prev`
/// (`null` at the genesis position), `body`; their canonical bytes are handed to `sign` (which returns the signature as
/// `galeed/1` §5.4 spells it, `0x` and 130 hex digits) and the signature joins them as `sig`. A value the canonical
/// writer refuses is that token (`Ok(Err)`); a signer that refuses is its own error. Whether the bytes are an entry is
/// §4.3's question, asked by the caller ([`accept`]) where it lands them.
pub fn assemble<E>(author: &str, entry_type: &str, seq: u64, prev: Option<&str>, body: Value, sign: impl FnOnce(&[u8]) -> Result<String, E>) -> Result<Result<Vec<u8>, Token>, E> {
    let [k_spec, k_type, k_judge, k_seq, k_prev, k_body, k_sig] = KEYS;
    let mut members = vec![
        (k_spec.to_string(), Value::Str(crate::LAW.to_string())),
        (k_type.to_string(), Value::Str(entry_type.to_string())),
        (k_judge.to_string(), Value::Str(author.to_string())),
        (k_seq.to_string(), Value::Int(seq)),
        (k_prev.to_string(), prev.map_or(Value::Null, |p| Value::Str(p.to_string()))),
        (k_body.to_string(), body),
    ];
    let unsigned = match json::canonical(&Value::Obj(members.clone())) {
        Ok(b) => b,
        Err(t) => return Ok(Err(t)),
    };
    members.push((k_sig.to_string(), Value::Str(sign(&unsigned)?)));
    Ok(json::canonical(&Value::Obj(members)))
}

pub struct Entry {
    pub value: Value,
    pub entry_id: [u8; 32],
}

impl Entry {
    /// The `body` when this entry's `entryType` equals `entry_type` byte for byte (step 12 already checked it
    /// against that type's table), else `None`. Callers compare types here so the comparison stays in the core.
    pub fn body_of_type(&self, entry_type: &str) -> Option<&Value> {
        (self.value.get(KEYS[1]).and_then(Value::as_str) == Some(entry_type)).then(|| self.value.get(KEYS[5])).flatten()
    }
}

#[cfg(not(feature = "ablate-k1-accept-thirteen"))]
pub fn accept(b: &[u8]) -> Result<Entry, Token> {
    // 1. §3.5.
    let value = json::accept(b)?;
    // 2. The root is an object.
    let obj = value.as_obj().ok_or(Token::Envelope)?;
    // 3. Exactly the seven keys: missing first, then extra.
    if KEYS.iter().any(|k| value.get(k).is_none()) {
        return Err(Token::EnvelopeMissing);
    }
    if obj.iter().any(|(k, _)| !KEYS.contains(&k.as_str())) {
        return Err(Token::EnvelopeClosed);
    }
    // 4. spec is a string equal to galeed/1 byte for byte.
    if value.get("spec").and_then(Value::as_str) != Some(crate::LAW) {
        return Err(Token::Spec);
    }
    // 5. entryType is a token.
    let entry_type = value.get("entryType").filter(|v| body::is_token(v)).and_then(Value::as_str).ok_or(Token::EntryType)?;
    // 6. judge is hex20.
    let judge = value.get("judge").and_then(Value::as_str).filter(|s| hexform::is_hex20(s)).ok_or(Token::Judge)?;
    // 7. seq is an int.
    let seq = match value.get("seq") {
        Some(Value::Int(n)) => *n,
        _ => return Err(Token::Seq),
    };
    // 8. prev is null or hex32, and null iff seq is 0.
    let prev_null = match value.get("prev") {
        Some(Value::Null) => true,
        Some(Value::Str(s)) if hexform::is_hex32(s) => false,
        _ => return Err(Token::Prev),
    };
    if prev_null != (seq == 0) {
        return Err(Token::PrevSeq);
    }
    // 9. body is an object.
    let body_v = value.get("body").filter(|v| v.as_obj().is_some()).ok_or(Token::Body)?;
    // 10. sig is hex65.
    let sig = value.get("sig").and_then(Value::as_str).filter(|s| hexform::is_hex65(s)).ok_or(Token::SigForm)?;
    // 11. §4.2: entryType is genesis iff seq is 0.
    if (entry_type == body::GENESIS) != (seq == 0) {
        return Err(Token::GenesisPlace);
    }
    // 12. The §6 body table for the type.
    if !body::check(entry_type, body_v) {
        return Err(Token::BodyField);
    }
    // 13. §5 signature.
    sign::verify(obj, judge, sig)?;
    Ok(Entry { entry_id: cryptox::sha256(b), value })
}

/// Fault-injection variant for the conformance gauges: only the first two steps run.
#[cfg(feature = "ablate-k1-accept-thirteen")]
pub fn accept(b: &[u8]) -> Result<Entry, Token> {
    let value = json::accept(b)?;
    value.as_obj().ok_or(Token::Envelope)?;
    Ok(Entry { entry_id: cryptox::sha256(b), value })
}

#[cfg(test)]
mod tests {
    use super::*;

    const H20: &str = "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf";

    fn entry_bytes(extra: &str, seq: u64, prev: &str, entry_type: &str, body: &str, sig_len: usize) -> String {
        let sig = format!("0x{}", "0".repeat(sig_len));
        let mut members = vec![
            ("body".to_string(), body.to_string()),
            ("entryType".to_string(), format!("\"{entry_type}\"")),
            ("judge".to_string(), format!("\"{H20}\"")),
            ("prev".to_string(), prev.to_string()),
            ("seq".to_string(), seq.to_string()),
            ("sig".to_string(), format!("\"{sig}\"")),
            ("spec".to_string(), "\"galeed/1\"".to_string()),
        ];
        if !extra.is_empty() {
            members.push(("extra".to_string(), extra.to_string()));
            members.sort();
        }
        let s: Vec<String> = members.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
        format!("{{{}}}", s.join(","))
    }

    fn tok(b: impl AsRef<[u8]>) -> Token {
        accept(b.as_ref()).err().unwrap()
    }

    #[test]
    fn steps_two_to_twelve() {
        assert_eq!(tok(b"[]"), Token::Envelope);
        assert_eq!(tok(b"{}"), Token::EnvelopeMissing);
        assert_eq!(tok(&entry_bytes("1", 0, "null", "genesis", "{\"statement_md\":\"\"}", 130)), Token::EnvelopeClosed);
        assert_eq!(tok(entry_bytes("", 0, "null", "genesis", "{\"statement_md\":\"\"}", 130).replace("galeed/1", "galeed/2").as_bytes()), Token::Spec);
        assert_eq!(tok(&entry_bytes("", 0, "null", "a b", "{}", 130)), Token::EntryType);
        assert_eq!(tok(entry_bytes("", 0, "null", "genesis", "{}", 130).replace(H20, "0xABC").as_bytes()), Token::Judge);
        assert_eq!(tok(entry_bytes("", 0, "null", "genesis", "{}", 130).replace("\"seq\":0", "\"seq\":\"0\"").as_bytes()), Token::Seq);
        assert_eq!(tok(&entry_bytes("", 0, "\"0x1\"", "genesis", "{}", 130)), Token::Prev);
        assert_eq!(tok(&entry_bytes("", 1, "null", "amanah", "{}", 130)), Token::PrevSeq);
        assert_eq!(tok(&entry_bytes("", 0, "null", "genesis", "[]", 130)), Token::Body);
        assert_eq!(tok(&entry_bytes("", 0, "null", "genesis", "{}", 128)), Token::SigForm);
        assert_eq!(tok(&entry_bytes("", 0, "null", "amanah", "{}", 130)), Token::GenesisPlace);
        assert_eq!(tok(&entry_bytes("", 0, "null", "genesis", "{}", 130)), Token::BodyField);
        // An all-zero sig has v = 0: E_SIG_V, the first check of step 13.
        assert_eq!(tok(&entry_bytes("", 0, "null", "genesis", "{\"statement_md\":\"\"}", 130)), Token::SigV);
    }
}
