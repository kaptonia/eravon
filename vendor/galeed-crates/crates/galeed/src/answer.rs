//! The one shape of every answer line: each carries `ok`, and each `ok: false` carries the same
//! refusal-name key, [`REFUSED`].
//!
//! Every door of every product binary passes its answer through [`stamp`] before stdout. A refusal
//! with no name never leaves: [`stamp`] answers `Err` and the binary exits as a misuse (empty stdout,
//! diagnostics on stderr).
//!
//! Fixture doors whose answer shapes are fixed byte for byte by the conformance harness do not pass
//! through here: their shapes belong to that contract.
//!
//! Older refusal keys (`reason`, `token`, `error`, `why`) are kept: when [`REFUSED`] is absent the name
//! is the first non-empty string among them, in [`LEGACY`] order.

use crate::json::Value;

pub const OK: &str = "ok";
/// Refusal-name key: every `ok: false` line carries exactly this one, with a non-empty string.
pub const REFUSED: &str = "refused";
/// Older refusal keys, in the order a name is taken from them.
pub const LEGACY: [&str; 4] = ["reason", "token", "error", "why"];

/// Why an answer line cannot leave (handled as a misuse).
#[derive(Debug, PartialEq, Eq)]
pub enum Unfit {
    NotObject,
    OkNotBool,
    Unnamed,
}

impl Unfit {
    pub fn why(&self) -> &'static str {
        match self {
            Unfit::NotObject => "答话不是一个 JSON 对象",
            Unfit::OkNotBool => "答话的 ok 不是布尔",
            Unfit::Unnamed => "拒行没有拒名",
        }
    }
}

fn named(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)
}

/// The refusal name of an answer: [`REFUSED`] if present, else the first non-empty string in [`LEGACY`]
/// order; failing both, the name of the first `ok: false` object among its members (arrays element by
/// element, recursively). Nested answers (`steps`, `anchors[].judgment`) take the inner name here and
/// nowhere else.
pub fn name_of(v: &Value) -> Option<String> {
    named(v.get(REFUSED)).or_else(|| LEGACY.iter().find_map(|k| named(v.get(k)))).or_else(|| inner_name(v))
}

fn inner_name(v: &Value) -> Option<String> {
    let kids: Vec<&Value> = match v {
        Value::Obj(m) => m.iter().map(|(_, x)| x).collect(),
        Value::Arr(a) => a.iter().collect(),
        _ => return None,
    };
    kids.into_iter().find_map(|k| match k {
        Value::Obj(_) if k.get(OK) == Some(&Value::Bool(false)) => name_of(k),
        Value::Obj(_) | Value::Arr(_) => inner_name(k),
        _ => None,
    })
}

/// Complete the answer: a missing `ok` means success; `ok: false` without a refusal key takes its name
/// from the older keys, or fails with [`Unfit::Unnamed`].
#[cfg(not(feature = "ablate-k1-stamp"))]
pub fn stamp(v: Value) -> Result<Value, Unfit> {
    let Value::Obj(mut m) = v else { return Err(Unfit::NotObject) };
    let ok = match m.iter().find(|(k, _)| k == OK).map(|(_, x)| x) {
        None => {
            m.push((OK.to_string(), Value::Bool(true)));
            true
        }
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(Unfit::OkNotBool),
    };
    if !ok {
        let this = Value::Obj(m.clone());
        if named(this.get(REFUSED)).is_none() {
            let name = name_of(&this).ok_or(Unfit::Unnamed)?;
            m.retain(|(k, _)| k != REFUSED);
            m.push((REFUSED.to_string(), Value::Str(name)));
        }
    }
    Ok(Value::Obj(m))
}

/// The key of a command line's refusal code: an `ok: false` answer of `galeed` carries its code here.
pub const REASON: &str = "reason";

/// The command line's answer shape (`galeed`): a missing `ok` means success, and every `ok: false` carries a
/// non-empty [`REASON`] of its own. Nothing is folded in from other members or from nested rows: an answer that
/// refuses without saying why cannot leave.
pub fn stamp_reason(v: Value) -> Result<Value, Unfit> {
    let Value::Obj(mut m) = v else { return Err(Unfit::NotObject) };
    let ok = match m.iter().find(|(k, _)| k == OK).map(|(_, x)| x) {
        None => {
            m.push((OK.to_string(), Value::Bool(true)));
            true
        }
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(Unfit::OkNotBool),
    };
    if !ok && named(Value::Obj(m.clone()).get(REASON)).is_none() {
        return Err(Unfit::Unnamed);
    }
    Ok(Value::Obj(m))
}

/// Fault-injection variant for the conformance gauges: a refusal row that names nothing passes as it is, so every
/// door prints a row saying "refused" without saying what, and exits as if it had answered.
#[cfg(feature = "ablate-k1-stamp")]
pub fn stamp(v: Value) -> Result<Value, Unfit> {
    let Value::Obj(mut m) = v else { return Err(Unfit::NotObject) };
    match m.iter().find(|(k, _)| k == OK).map(|(_, x)| x) {
        None => m.push((OK.to_string(), Value::Bool(true))),
        Some(Value::Bool(_)) => {}
        Some(_) => return Err(Unfit::OkNotBool),
    }
    Ok(Value::Obj(m))
}

pub fn refused(name: &str, more: Vec<(&str, Value)>) -> Value {
    let mut m: Vec<(String, Value)> = more.into_iter().filter(|(k, _)| *k != OK && *k != REFUSED).map(|(k, v)| (k.to_string(), v)).collect();
    m.push((OK.to_string(), Value::Bool(false)));
    m.push((REFUSED.to_string(), Value::Str(name.to_string())));
    Value::Obj(m)
}

/// Top-level name of a nested refusal (per-cell check tables, rule tables): `<family>-<first failing part>`.
pub fn first_failed(family: &str, part: &str) -> String {
    format!("{family}-{part}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nested_failing_step_names_the_outer_refusal() {
        use crate::json::Value as V;
        let o = |m: Vec<(&str, V)>| V::Obj(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
        // The anchor step succeeded but one judgment inside it failed: the outer name is that judgment's.
        let face = o(vec![("ok", V::Bool(false)), ("steps", V::Arr(vec![o(vec![("ok", V::Bool(true)), ("anchors", V::Arr(vec![o(vec![("judgment", o(vec![("ok", V::Bool(false)), ("token", V::Str("KIT_MISSING".into()))]))])]))])]))]);
        assert_eq!(name_of(&face).as_deref(), Some("KIT_MISSING"));
        assert!(stamp(face).is_ok(), "有名的拒行照出门");
        assert_eq!(stamp(o(vec![("ok", V::Bool(false))])), Err(Unfit::Unnamed), "真没有名的照旧不许出门");
    }

    fn o(m: Vec<(&str, Value)>) -> Value {
        Value::Obj(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    #[test]
    fn every_line_leaves_with_ok_and_every_refusal_with_its_name() {
        let got = stamp(o(vec![("rows", Value::Arr(vec![]))])).unwrap();
        assert_eq!(got.get(OK), Some(&Value::Bool(true)), "缺 ok 即答成");
        let got = stamp(o(vec![(OK, Value::Bool(false)), ("reason", Value::Str("identity".into()))])).unwrap();
        assert_eq!(got.get(REFUSED).and_then(Value::as_str), Some("identity"), "拒名取自旧键");
        assert_eq!(got.get("reason").and_then(Value::as_str), Some("identity"), "旧键加不删");
        let got = stamp(o(vec![(OK, Value::Bool(false)), ("reason", Value::Null), ("token", Value::Str("E_JSON".into()))])).unwrap();
        assert_eq!(got.get(REFUSED).and_then(Value::as_str), Some("E_JSON"), "空的旧键跳过");
        let got = stamp(o(vec![(OK, Value::Bool(false)), (REFUSED, Value::Str("own".into())), ("reason", Value::Str("other".into()))])).unwrap();
        assert_eq!(got.get(REFUSED).and_then(Value::as_str), Some("own"), "自带的拒名为准");
        assert_eq!(stamp(o(vec![(OK, Value::Bool(false))])), Err(Unfit::Unnamed), "无名的拒不出门");
        assert_eq!(stamp(o(vec![(OK, Value::Bool(false)), ("error", o(vec![]))])), Err(Unfit::Unnamed), "对象不是拒名");
        assert_eq!(stamp(o(vec![(OK, Value::Bool(false)), (REFUSED, Value::Str(String::new()))])), Err(Unfit::Unnamed), "空串不是拒名");
        assert_eq!(stamp(o(vec![(OK, Value::Str("yes".into()))])), Err(Unfit::OkNotBool));
        assert_eq!(stamp(Value::Arr(vec![])), Err(Unfit::NotObject));
        let r = refused("shape", vec![(OK, Value::Bool(true)), ("x", Value::Int(1))]);
        assert_eq!(stamp(r.clone()), Ok(r.clone()), "refused 造出的一行已合门面");
        assert_eq!(r.get(OK), Some(&Value::Bool(false)));
    }
}
