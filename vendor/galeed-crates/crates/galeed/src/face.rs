//! Shared answer shape of a signature: `{digest, presig, sig, signer}` (the `gl1 sign` output contract of
//! the conformance harness). The harness door and the product's registered-signing door share one path
//! and one shape.

use crate::hexform;
use crate::json::Value;
use crate::sign::Signed;

pub fn signed_value(s: &Signed) -> Value {
    let hex = |b: &[u8]| Value::Str(hexform::encode(b));
    Value::Obj(vec![
        ("digest".into(), hex(&s.digest)),
        ("presig".into(), hex(&s.presig)),
        ("sig".into(), hex(&s.sig)),
        ("signer".into(), hex(&s.signer)),
    ])
}
