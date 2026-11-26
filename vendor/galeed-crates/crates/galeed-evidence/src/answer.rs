//! Answer shapes of the four harness commands and the product's two self-check commands.

use crate::{Binding, Fault, Pick};
use galeed::hexform;
use galeed::json::Value;

pub const OK: &str = "ok";
pub const TOKEN: &str = "token";
pub const INDEX: &str = "index";
pub const MANIFEST: &str = "manifest";
pub const PICK: &str = "pick";
pub const BINDING: &str = "binding";
pub const LIST: &str = "list";
pub const REASON: &str = "reason";
pub const RECIPIENT_FORM: &str = "recipientForm";
pub const PLAINTEXT: &str = "plaintext";
pub const REFUSED: &str = galeed::answer::REFUSED;
pub const R_UNDECRYPTABLE: &str = "age-undecryptable";
pub const R_ROUNDTRIP: &str = "age-roundtrip-differs";
/// The §3.2 token.
pub const E_RECIPIENT: &str = "E_RECIPIENT";

fn obj(m: Vec<(&str, Value)>) -> Value {
    Value::Obj(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}
fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

/// `gev check`: the digest when accepted, the token when refused; the eight file-item tokens add `index`.
pub fn check(r: &Result<[u8; 32], Fault>) -> Value {
    match r {
        Ok(d) => obj(vec![(MANIFEST, s(&hexform::encode(d))), (OK, Value::Bool(true))]),
        Err(f) => {
            let mut m = vec![(OK, Value::Bool(false)), (TOKEN, s(f.name()))];
            if let Some(i) = f.index() {
                m.push((INDEX, Value::Int(i as u64)));
            }
            obj(m)
        }
    }
}

pub fn digest(bytes: u64, sha256: &[u8; 32]) -> Value {
    obj(vec![(crate::key::BYTES, Value::Int(bytes)), (crate::key::SHA256, s(&hexform::encode(sha256)))])
}

pub fn recipient(ok: bool) -> Value {
    if ok {
        obj(vec![(OK, Value::Bool(true))])
    } else {
        obj(vec![(OK, Value::Bool(false)), (TOKEN, s(E_RECIPIENT))])
    }
}

fn binding(b: &Binding) -> Value {
    obj(vec![
        (crate::key::EFFECTIVE, Value::Int(b.effective())),
        (crate::key::ENTRY_ID, s(b.entry_id())),
        (crate::key::RECIPIENT, s(b.recipient())),
        (RECIPIENT_FORM, Value::Bool(b.recipient_form())),
        (crate::key::SEQ, Value::Int(b.seq())),
    ])
}

pub fn pick(p: &Pick) -> Value {
    match p {
        Pick::None => obj(vec![(PICK, s(Pick::NONE))]),
        Pick::Unread => obj(vec![(PICK, s(Pick::UNREAD))]),
        Pick::One(b) => obj(vec![(BINDING, binding(b)), (PICK, s(Pick::ONE))]),
        // The harness spells the list and the reason; the holes a gap names are the product's face (§4.2), not the
        // contract's.
        Pick::Candidates { reason, list, .. } => obj(vec![(LIST, Value::Arr(list.iter().map(binding).collect())), (PICK, s(Pick::CANDIDATES)), (REASON, s(reason.name()))]),
    }
}

/// `gev age-roundtrip`: encrypt to the identity's recipient and decrypt again; true when the plaintext
/// comes back unchanged. Carries the recipient and the plaintext digest (deterministic).
pub fn roundtrip(ok: bool, recipient: &str, sha256: &[u8; 32]) -> Value {
    let mut m = vec![(OK, Value::Bool(ok)), (crate::key::RECIPIENT, s(recipient)), (crate::key::SHA256, s(&hexform::encode(sha256)))];
    if !ok {
        m.push((REFUSED, s(R_ROUNDTRIP)));
    }
    obj(m)
}

pub fn decrypted(p: &Option<Vec<u8>>) -> Value {
    match p {
        Some(b) => obj(vec![(OK, Value::Bool(true)), (PLAINTEXT, s(&hexform::encode(b)))]),
        None => obj(vec![(OK, Value::Bool(false)), (REFUSED, s(R_UNDECRYPTABLE))]),
    }
}

/// `gev age-encrypt`: the ciphertext length when written; the reason when refused, with nothing written.
pub fn encrypted(r: &Result<Vec<u8>, crate::Refused>) -> Value {
    match r {
        Ok(ct) => obj(vec![(crate::key::BYTES, Value::Int(ct.len() as u64)), (OK, Value::Bool(true))]),
        Err(e) => obj(vec![(OK, Value::Bool(false)), (REFUSED, s(e.name()))]),
    }
}
