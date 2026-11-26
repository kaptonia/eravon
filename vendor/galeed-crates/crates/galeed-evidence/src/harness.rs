//! The harness command surface (`check`, `digest`, `recipient-check`, `pick-recipient`) plus the product's
//! three age self-checks (`age-roundtrip`, `age-decrypt`, `age-encrypt`; §8.1 leaves §3 beyond the
//! recipient form out of the corpus, so interoperability and round trips are the producer's own tests).
//! Anything malformed is a misuse (exit 2, empty stdout); every path is read and every form checked
//! before the law predicate runs.

use crate::answer;
use galeed::cryptox::{self, AgeIdentity};
use galeed::json::{self, Value};
use std::ffi::OsString;

pub const CMD_CHECK: &str = "check";
pub const CMD_DIGEST: &str = "digest";
pub const CMD_RECIPIENT_CHECK: &str = "recipient-check";
pub const CMD_PICK_RECIPIENT: &str = "pick-recipient";
/// The four commands whose answer shapes the harness fixes verbatim: their answers bypass the answer
/// module (the shape belongs to that contract).
pub const CONTRACT: [&str; 4] = [CMD_CHECK, CMD_DIGEST, CMD_RECIPIENT_CHECK, CMD_PICK_RECIPIENT];
pub const CMD_AGE_ROUNDTRIP: &str = "age-roundtrip";
pub const CMD_AGE_DECRYPT: &str = "age-decrypt";
pub const CMD_AGE_ENCRYPT: &str = "age-encrypt";

pub const FILE_BOUND: u64 = 16 * 1024 * 1024;

#[derive(Debug)]
pub struct Misuse(pub String);

fn misuse<T>(why: impl Into<String>) -> Result<T, Misuse> {
    Err(Misuse(why.into()))
}

fn read_bytes(a: &OsString) -> Result<Vec<u8>, Misuse> {
    let b = std::fs::read(a).map_err(|e| Misuse(format!("读不到 {}:{e}", a.to_string_lossy())))?;
    if b.len() as u64 > FILE_BOUND {
        return misuse("档过十六 MiB");
    }
    Ok(b)
}

/// `<audit.json>`: read as a file first (an unreadable path is misuse) and handed to the core whole: bytes the reader
/// refuses, a root that is no object, a member absent or outside its form, or a basis §9.4 refuses are the law's
/// `unread`, never a misuse.
#[cfg(not(feature = "ablate-k3-gev"))]
fn audit_file(a: &OsString) -> Result<Value, Misuse> {
    Ok(json::read_open(&read_bytes(a)?).unwrap_or(Value::Null))
}

/// Fault-injection variant for the conformance gauges: an unreadable path is read as an empty input and answered
/// `unread`, so a producer that named the wrong file is told the judge's ledger did not read.
#[cfg(feature = "ablate-k3-gev")]
fn audit_file(a: &OsString) -> Result<Value, Misuse> {
    Ok(read_bytes(a).ok().and_then(|b| json::read_open(&b)).unwrap_or(Value::Null))
}

fn identity_file(a: &OsString) -> Result<AgeIdentity, Misuse> {
    std::str::from_utf8(&read_bytes(a)?).ok().and_then(AgeIdentity::parse).ok_or_else(|| Misuse("身份档不是一行 AGE-SECRET-KEY-1…".into()))
}

pub fn run(args: &[OsString]) -> Result<Value, Misuse> {
    let cmd = args.first().map(|a| a.to_str().ok_or_else(|| Misuse("命令不是 UTF-8".into()))).transpose()?;
    let rest = args.get(1..).unwrap_or_default();
    match (cmd, rest) {
        (Some(CMD_CHECK), [path]) => Ok(answer::check(&crate::digest(&read_bytes(path)?))),
        (Some(CMD_DIGEST), [path]) => {
            let (n, d) = crate::file_view(&read_bytes(path)?);
            Ok(answer::digest(n, &d))
        }
        // An argument that cannot be UTF-8 must hold a byte above 0x7E, so §3.2 answers "not the form" and the
        // exit is still 0.
        (Some(CMD_RECIPIENT_CHECK), [r]) => Ok(answer::recipient(r.to_str().is_some_and(crate::recipient_form))),
        (Some(CMD_PICK_RECIPIENT), [audit]) => Ok(answer::pick(&crate::pick(&audit_file(audit)?))),
        (Some(CMD_AGE_ROUNDTRIP), [id, plain]) => {
            let (id, pt) = (identity_file(id)?, read_bytes(plain)?);
            let back = crate::encrypt_to(&id.recipient(), &pt).ok().and_then(|ct| crate::decrypt(&id, &ct));
            Ok(answer::roundtrip(back.as_deref() == Some(pt.as_slice()), &id.recipient(), &cryptox::sha256(&pt)))
        }
        (Some(CMD_AGE_DECRYPT), [id, ct]) => {
            let (id, ct) = (identity_file(id)?, read_bytes(ct)?);
            Ok(answer::decrypted(&crate::decrypt(&id, &ct)))
        }
        (Some(CMD_AGE_ENCRYPT), [r, plain, out]) => {
            let r = r.to_str().unwrap_or_default();
            let got = crate::encrypt_to(r, &read_bytes(plain)?);
            if let Ok(ct) = &got {
                std::fs::write(out, ct).map_err(|e| Misuse(format!("写不出 {}:{e}", out.to_string_lossy())))?;
            }
            Ok(answer::encrypted(&got))
        }
        _ => misuse("用法:gev check|digest|recipient-check|pick-recipient …(参数照 HARNESS-EVIDENCE.md);age-roundtrip|age-decrypt|age-encrypt …"),
    }
}
