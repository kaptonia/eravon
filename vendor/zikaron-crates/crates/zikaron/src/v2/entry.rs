//! Envelope (law §4), signature (law §5) and the eight body tables (law §6) of `zikaron/2`. The canonical form
//! (§3), the preimage, message and digest (§5.1 to §5.3) and the signature tests (§5.4, §5.5) are the
//! `zikaron/1` core's own functions, called here, the two laws stating them word for word; what this file
//! states is what `zikaron/2` adds or changes: the spec literal, the one domain, and the eight bodies.

use super::tokens::{self as t2, Domain, EntryType};
use crate::entry as v1;
use crate::hexfmt;
use crate::json::{self, Value};
use crate::tokens::Token;
use crate::trace;

/// A byte string that passes the thirteen steps of §4.3.
#[derive(Clone, Debug)]
pub struct Entry {
    pub bytes: Vec<u8>,
    pub id: [u8; 32],
    /// `hex32(entry_id)`, the spelling every comparison and report row uses (§1).
    pub id_hex: String,
    pub value: Value,
    /// The entryType bytes as they came (§6.9: an unlisted type is still an entry).
    pub entry_type: String,
    /// The same value read through the §6 table; compare this, not the literal.
    pub kind: EntryType,
    pub author: String,
    pub seq: u64,
    /// `None` at seq 0 (§4.3 step 8).
    pub prev: Option<String>,
    pub body: Value,
}

/// §1 `token`: a non-empty string every scalar value of which lies in U+0021 through U+007E.
fn is_token(v: &Value) -> bool {
    matches!(v, Value::Str(s) if !s.is_empty() && s.bytes().all(|b| (0x21..=0x7e).contains(&b)))
}

fn is_hex20(v: &Value) -> bool {
    matches!(v, Value::Str(s) if hexfmt::is_hex20(s))
}

fn is_hex32(v: &Value) -> bool {
    matches!(v, Value::Str(s) if hexfmt::is_hex32(s))
}

fn is_int(v: &Value) -> bool {
    matches!(v, Value::Int(_))
}

/// A `prose string` row tests that the value is a string and nothing else (§6).
fn is_prose(v: &Value) -> bool {
    matches!(v, Value::Str(_))
}

type Test = fn(&Value) -> bool;

/// A required member: present and of its row's form.
fn req(body: &Value, key: &'static str, test: Test) -> Result<(), &'static str> {
    match body.member(key) {
        Some(v) if test(v) => Ok(()),
        _ => Err(key),
    }
}

/// An optional member: absent, or present and of its row's form (`null` is present, §6).
fn opt(body: &Value, key: &'static str, test: Test) -> Result<(), &'static str> {
    match body.member(key) {
        Some(v) if !test(v) => Err(key),
        _ => Ok(()),
    }
}

/// An object row ("members beyond these permitted", §6.10): the value is an object, its named members carry
/// their forms, and nothing else is tested.
fn object_rows(
    v: &Value,
    key: &'static str,
    required: &[(&'static str, Test)],
    optional: &[(&'static str, Test)],
) -> Result<(), &'static str> {
    if !v.is_obj() {
        return Err(key);
    }
    for (k, test) in required {
        req(v, k, *test).map_err(|_| key)?;
    }
    for (k, test) in optional {
        opt(v, k, *test).map_err(|_| key)?;
    }
    Ok(())
}

/// The body table of law §6 for the entry's type: `Err` names the first member the table refuses. One token
/// covers every body fault (§6), so no order of tests within a body is part of the law; the name is for the
/// people who wrote it.
pub fn body_fault(kind: EntryType, body: &Value) -> Option<&'static str> {
    body_check(kind, body).err()
}

fn body_check(kind: EntryType, body: &Value) -> Result<(), &'static str> {
    match kind {
        // §6.1
        EntryType::Genesis => {
            req(body, "statement_md", is_prose)?;
            if let Some(c) = body.member("continues") {
                object_rows(c, "continues", &[("root", is_hex20), ("entry_id", is_hex32)], &[])?;
            }
            Ok(())
        }
        // §6.2
        EntryType::History => {
            req(body, "content", is_hex32)?;
            match body.member("mode") {
                Some(m) => object_rows(m, "mode", &[("mark", is_token), ("toolchain", is_hex32)], &[])?,
                None => return Err("mode"),
            }
            opt(body, "note_md", is_prose)?;
            if let Some(f) = body.member("for") {
                object_rows(
                    f,
                    "for",
                    &[("app", is_token), ("identity", is_hex20), ("ref", is_token)],
                    &[("seat", is_token)],
                )?;
            }
            Ok(())
        }
        // §6.3
        EntryType::Grant => {
            req(body, "grantee", is_hex20)?;
            req(body, "work", is_hex32)?;
            req(body, "terms", is_hex32)?;
            opt(body, "history", is_hex32)?;
            if let Some(w) = body.member("window") {
                object_rows(w, "window", &[("from", is_int), ("to", is_int)], &[])?;
                // The one relation a row of §6 states.
                if w.member("from").and_then(Value::as_int) > w.member("to").and_then(Value::as_int) {
                    return Err("window");
                }
            }
            opt(body, "scope_md", is_prose)?;
            opt(body, "upstream", is_hex32)?;
            opt(body, "ledger", is_hex20)?;
            Ok(())
        }
        // §6.4
        EntryType::Revocation => {
            req(body, "grant", is_hex32)?;
            opt(body, "case", is_hex32)?;
            Ok(())
        }
        // §6.5
        EntryType::Succession => {
            req(body, "to", is_hex20)?;
            req(body, "kind", is_token)?;
            req(body, "effective", is_int)?;
            req(body, "statement_md", is_prose)?;
            Ok(())
        }
        // §6.6
        EntryType::Migration => {
            req(body, "spec", is_token)?;
            req(body, "statement_md", is_prose)?;
            Ok(())
        }
        // §6.7
        EntryType::Annotation => {
            opt(body, "subject", is_hex32)?;
            req(body, "note_md", is_prose)?;
            Ok(())
        }
        // §6.8
        EntryType::Retraction => {
            req(body, "subject", is_hex32)?;
            opt(body, "note_md", is_prose)?;
            Ok(())
        }
        // §6.9: an unknown type has only the object test of step 9, already met.
        EntryType::Other => Ok(()),
    }
}

const ENVELOPE_KEYS: [&str; 7] = ["spec", "entryType", "author", "seq", "prev", "body", "sig"];

/// §4.3 step 3: none of the seven keys absent (`E_ENVELOPE_MISSING`), then no eighth key
/// (`E_ENVELOPE_CLOSED`).
fn envelope_keys_ok(members: &[(String, Value)]) -> Result<(), Token> {
    for k in ENVELOPE_KEYS {
        if !members.iter().any(|(mk, _)| mk == k) {
            return Err(Token::EnvelopeMissing);
        }
    }
    if members.iter().any(|(mk, _)| !ENVELOPE_KEYS.contains(&mk.as_str())) {
        return Err(Token::EnvelopeClosed);
    }
    Ok(())
}

/// The §5.3 digest of an entry under this law's one domain, from the §5.1 preimage of its parsed value.
pub fn entry_digest(v: &Value) -> [u8; 32] {
    v1::presig_and_digest(&v1::b6_bytes(v), Domain::Entry.as_str()).1
}

/// Law §4.3: the thirteen steps in order, the first failure's token; what passes is an entry, nothing else is.
pub fn check(b: &[u8]) -> Result<Entry, Token> {
    trace::mark(t2::K3);
    // Step 1: §3.5 acceptance.
    let v = json::accept(b)?;
    // Step 2.
    let members = match &v {
        Value::Obj(ms) => ms,
        _ => return Err(Token::Envelope),
    };
    // Step 3.
    envelope_keys_ok(members)?;
    // Step 4: a string byte-equal to `zikaron/2`; one token covers every other form.
    match v.member("spec") {
        Some(Value::Str(s)) if s == t2::SPEC => {}
        _ => return Err(Token::Spec),
    }
    // Step 5.
    let entry_type = match v.member("entryType") {
        Some(x) if is_token(x) => x.as_str().unwrap_or_default().to_string(),
        _ => return Err(Token::EntryType),
    };
    // Step 6.
    let author = match v.member("author") {
        Some(Value::Str(s)) if hexfmt::is_hex20(s) => s.clone(),
        _ => return Err(Token::Author),
    };
    // Step 7.
    let seq = match v.member("seq") {
        Some(Value::Int(i)) => *i,
        _ => return Err(Token::Seq),
    };
    // Step 8.
    let prev = match v.member("prev") {
        Some(Value::Null) => None,
        Some(Value::Str(s)) if hexfmt::is_hex32(s) => Some(s.clone()),
        _ => return Err(Token::Prev),
    };
    if prev.is_none() != (seq == 0) {
        return Err(Token::PrevSeq);
    }
    // Step 9.
    let body = match v.member("body") {
        Some(x) if x.is_obj() => x.clone(),
        _ => return Err(Token::Body),
    };
    // Step 10.
    let sig = match v.member("sig") {
        Some(Value::Str(s)) if hexfmt::is_hex65(s) => s.clone(),
        _ => return Err(Token::SigForm),
    };
    let kind = EntryType::of(&entry_type);
    // Step 11: §4.2.
    if (kind == EntryType::Genesis) != (seq == 0) {
        return Err(Token::GenesisPlace);
    }
    // Step 12: §6.
    if body_check(kind, &body).is_err() {
        return Err(Token::BodyField);
    }
    // Step 13: §5.4 then §5.5.
    v1::verify_signature(&sig, &entry_digest(&v), &author)?;
    let id = v1::entry_id(b);
    Ok(Entry {
        bytes: b.to_vec(),
        id,
        id_hex: hexfmt::encode(&id),
        value: v,
        entry_type,
        kind,
        author,
        seq,
        prev,
        body,
    })
}
