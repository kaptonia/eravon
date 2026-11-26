//! §6 body tables: presence and form of every member of the eight types; one token, `E_BODY_FIELD`,
//! covers every body error. Unknown types (§6.8) are not refused beyond "body is an object" (step 9);
//! members outside a table (§6.9) are allowed and not read.
//!
//! Forms follow the §6 preamble: `prose string` is a string; `skeleton string` is a string, non-empty
//! where required; `token` is a non-empty string without U+0020 (the character set is settled by §3.5
//! check 1); `int` is an integer; `bool` is one of the two literals; hex20 / hex32 as in §1.

#![cfg_attr(feature = "ablate-k1-body-table", allow(dead_code))]

use crate::hexform;
use crate::json::Value;

/// The closed set of §6.4 `category`.
pub const CATEGORY_UNLAWFUL: &str = "unlawful-subject";
pub const CATEGORY_OTHER: &str = "other";
/// The two known `subjectRef.kind` values of §6.3.
pub const KIND_EVM: &str = "evm";
pub const KIND_HASH: &str = "hash";

/// The eight entry types of §6, matched byte for byte.
pub const GENESIS: &str = "genesis";
pub const AMANAH: &str = "amanah";
pub const JUDGMENT: &str = "judgment";
pub const RESIGNATION: &str = "resignation";
pub const ANNOTATION: &str = "annotation";
pub const SUCCESSION: &str = "succession";
pub const MIGRATION: &str = "migration";
pub const KEYBINDING: &str = "keybinding";
/// The eight types of §6; §8.7 item 8 tells unknown types by this list.
pub const KNOWN_TYPES: [&str; 8] = [GENESIS, AMANAH, JUDGMENT, RESIGNATION, ANNOTATION, SUCCESSION, MIGRATION, KEYBINDING];

pub fn is_prose(v: &Value) -> bool {
    matches!(v, Value::Str(_))
}
pub fn is_skeleton_nonempty(v: &Value) -> bool {
    v.as_str().is_some_and(|s| !s.is_empty())
}
pub fn is_token(v: &Value) -> bool {
    v.as_str().is_some_and(|s| !s.is_empty() && !s.contains(' '))
}
pub fn is_int(v: &Value) -> bool {
    matches!(v, Value::Int(_))
}
fn is_bool(v: &Value) -> bool {
    matches!(v, Value::Bool(_))
}
pub fn is_hex20(v: &Value) -> bool {
    v.as_str().is_some_and(hexform::is_hex20)
}
pub fn is_hex32(v: &Value) -> bool {
    v.as_str().is_some_and(hexform::is_hex32)
}
fn is_obj(v: &Value) -> bool {
    matches!(v, Value::Obj(_))
}

fn req(o: &Value, k: &str, form: fn(&Value) -> bool) -> bool {
    o.get(k).is_some_and(form)
}
fn opt(o: &Value, k: &str, form: fn(&Value) -> bool) -> bool {
    o.get(k).is_none_or(form)
}
fn arr_of(v: &Value, each: fn(&Value) -> bool) -> bool {
    match v {
        Value::Arr(a) => a.iter().all(each),
        _ => false,
    }
}

/// §6.2 `venues` element: an object with chainId int, contract hex20, codeHash hex32; other members allowed.
fn venue_elem(v: &Value) -> bool {
    is_obj(v) && req(v, "chainId", is_int) && req(v, "contract", is_hex20) && req(v, "codeHash", is_hex32)
}

/// §6.3 `evidence` element: an object with manifest hex32, produced bool; other members allowed.
fn evidence_elem(v: &Value) -> bool {
    is_obj(v) && req(v, "manifest", is_hex32) && req(v, "produced", is_bool)
}

/// §6.3 / §6.4 `subjectRef`: an object with a required token `kind`; a known kind enforces its table,
/// an unknown kind is tested for these two conditions only.
pub fn subject_ref(v: &Value) -> bool {
    if !is_obj(v) || !req(v, "kind", is_token) {
        return false;
    }
    match v.get("kind").and_then(Value::as_str) {
        Some(KIND_EVM) => {
            req(v, "chainId", is_int) && req(v, "contract", is_hex20) && req(v, "codeHash", is_hex32) && req(v, "id", is_hex32)
        }
        Some(KIND_HASH) => req(v, "sha256", is_hex32) && opt(v, "note_md", is_prose),
        _ => true,
    }
}

/// §4.3 step 12: check `body` against the §6 table for `entryType`. `body` is already an object (step 9).
#[cfg(not(feature = "ablate-k1-body-table"))]
pub fn check(entry_type: &str, body: &Value) -> bool {
    first_bad(entry_type, body).is_none()
}

/// **The §6 table itself**, read member by member: the first member of `body` that fails the table for `entryType`
/// (`None` when the body passes). [`check`] is this answering nothing; a refusal that wants to say which member is
/// wrong asks here (the command line names it and the option that gives it).
pub fn first_bad(entry_type: &str, body: &Value) -> Option<&'static str> {
    let need = |name: &'static str, ok: bool| (!ok).then_some(name);
    match entry_type {
        GENESIS => need("statement_md", req(body, "statement_md", is_prose)),
        AMANAH => need("version", req(body, "version", is_int))
            .or_else(|| need("text_md", req(body, "text_md", is_prose)))
            .or_else(|| need("venues", opt(body, "venues", |v| arr_of(v, venue_elem))))
            // §6.2 `quotes`: an object; what it carries is the registry law's to read, so nothing of its members is
            // tested here.
            .or_else(|| need("quotes", opt(body, "quotes", is_obj))),
        JUDGMENT => need("subjectRef", req(body, "subjectRef", subject_ref))
            .or_else(|| need("amanah", req(body, "amanah", is_hex32)))
            .or_else(|| need("verdict_md", req(body, "verdict_md", is_prose)))
            .or_else(|| need("decision", opt(body, "decision", is_obj)))
            .or_else(|| need("evidence", opt(body, "evidence", |v| arr_of(v, evidence_elem))))
            .or_else(|| need("txRefs", opt(body, "txRefs", |v| arr_of(v, is_hex32))))
            .or_else(|| need("terms", opt(body, "terms", is_hex32))),
        RESIGNATION => {
            let category = body.get("category").and_then(Value::as_str);
            let category_ok = matches!(category, Some(CATEGORY_UNLAWFUL) | Some(CATEGORY_OTHER));
            // The one relation in §6.4: clause is present iff category is unlawful-subject.
            let clause_ok = match (category == Some(CATEGORY_UNLAWFUL), body.get("clause")) {
                (true, Some(c)) => is_skeleton_nonempty(c),
                (false, None) => true,
                _ => false,
            };
            need("subjectRef", req(body, "subjectRef", subject_ref))
                .or_else(|| need("amanah", req(body, "amanah", is_hex32)))
                .or_else(|| need("category", category_ok))
                .or_else(|| need("clause", clause_ok))
                .or_else(|| need("statement_md", req(body, "statement_md", is_prose)))
        }
        ANNOTATION => need("subject", opt(body, "subject", is_hex32)).or_else(|| need("note_md", req(body, "note_md", is_prose))),
        SUCCESSION => need("to", req(body, "to", is_hex20))
            .or_else(|| need("kind", req(body, "kind", is_token)))
            .or_else(|| need("effective", req(body, "effective", is_int)))
            .or_else(|| need("statement_md", req(body, "statement_md", is_prose))),
        // §6.6a: the law the ledger moves to, and the seat's own words.
        MIGRATION => need("spec", req(body, "spec", is_token)).or_else(|| need("statement_md", req(body, "statement_md", is_prose))),
        KEYBINDING => need("recipient", req(body, "recipient", is_token)).or_else(|| need("effective", req(body, "effective", is_int))),
        // §6.8: an unknown type's body is tested for nothing beyond being an object.
        _ => None,
    }
}

/// Fault-injection variant for the conformance gauges: no table is checked.
#[cfg(feature = "ablate-k1-body-table")]
pub fn check(entry_type: &str, body: &Value) -> bool {
    let _ = (entry_type, body);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::read;

    fn body(s: &str) -> Value {
        read(s.as_bytes()).unwrap()
    }

    #[test]
    fn tables() {
        assert!(check(GENESIS, &body(r#"{"statement_md":""}"#)));
        assert!(!check(GENESIS, &body(r#"{}"#)));
        assert!(!check(GENESIS, &body(r#"{"statement_md":null}"#)));
        assert!(check("galeed.dispatch/1", &body(r#"{"anything":1}"#)));
        assert!(check(AMANAH, &body(r#"{"version":0,"text_md":"t","venues":[]}"#)));
        assert!(!check(AMANAH, &body(r#"{"version":1,"text_md":"t","venues":[{"chainId":1}]}"#)));
        let h32 = "0x".to_string() + &"0".repeat(64);
        let h20 = "0x".to_string() + &"0".repeat(40);
        assert!(check(JUDGMENT, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"{h32}","verdict_md":"v","decision":{{}}}}"#))));
        assert!(!check(JUDGMENT, &body(&format!(r#"{{"subjectRef":{{"kind":"evm","chainId":1}},"amanah":"{h32}","verdict_md":"v"}}"#))));
        assert!(check(JUDGMENT, &body(&format!(r#"{{"subjectRef":{{"kind":"other-kind"}},"amanah":"{h32}","verdict_md":"v"}}"#))));
        assert!(!check(JUDGMENT, &body(&format!(r#"{{"subjectRef":{{"kind":"x y"}},"amanah":"{h32}","verdict_md":"v"}}"#))));
        assert!(!check(JUDGMENT, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"{h32}","verdict_md":"v","decision":3}}"#))));
        assert!(check(RESIGNATION, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"{h32}","category":"unlawful-subject","clause":"c","statement_md":""}}"#))));
        assert!(!check(RESIGNATION, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"{h32}","category":"other","clause":"c","statement_md":""}}"#))));
        assert!(!check(RESIGNATION, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"{h32}","category":"unlawful-subject","statement_md":""}}"#))));
        assert!(!check(RESIGNATION, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"{h32}","category":"Other","statement_md":""}}"#))));
        assert!(check(ANNOTATION, &body(r#"{"note_md":"n"}"#)));
        assert!(check(SUCCESSION, &body(&format!(r#"{{"to":"{h20}","kind":"anything","effective":0,"statement_md":""}}"#))));
        assert!(check(KEYBINDING, &body(r#"{"recipient":"age1x","effective":1}"#)));
        assert!(!check(KEYBINDING, &body(r#"{"recipient":"age 1","effective":1}"#)));
        assert_eq!(first_bad(JUDGMENT, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"0xabc","verdict_md":"v"}}"#))), Some("amanah"));
        assert_eq!(first_bad(RESIGNATION, &body(&format!(r#"{{"subjectRef":{{"kind":"hash","sha256":"{h32}"}},"amanah":"{h32}","category":"unlawful-subject","statement_md":""}}"#))), Some("clause"));
        assert_eq!(first_bad(GENESIS, &body(r#"{"statement_md":""}"#)), None);
    }

    /// §6.2 `quotes` (optional): an object of any members passes, whatever they hold; any other value is the
    /// member the table names, so a seat whose tooling wrote quotes as an array or a string is refused at `quotes`
    /// before signing (the command line names it), and a reader of another tool's amanah with quotes the registry
    /// law does not know still accepts the entry.
    #[test]
    fn amanah_quotes_is_an_object_of_anything() {
        assert!(check(AMANAH, &body(r#"{"version":1,"text_md":"t"}"#)), "缺席即过");
        assert!(check(AMANAH, &body(r#"{"version":1,"text_md":"t","quotes":{}}"#)));
        assert!(check(AMANAH, &body(r#"{"version":1,"text_md":"t","quotes":{"0x01":"anything","x":[1,{"y":null}]}}"#)), "成员一概不判");
        for bad in [r#"[]"#, r#""q""#, r#"0"#, r#"null"#, r#"true"#] {
            let b = body(&format!(r#"{{"version":1,"text_md":"t","quotes":{bad}}}"#));
            assert_eq!(first_bad(AMANAH, &b), Some("quotes"), "{bad}");
        }
        // The order of §6.2's table: an earlier member's fault is named first.
        assert_eq!(first_bad(AMANAH, &body(r#"{"version":1,"text_md":"t","venues":3,"quotes":3}"#)), Some("venues"));
    }
}
