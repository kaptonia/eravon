//! The harness command surface (HARNESS-ROSTER.md): argument spelling, five file forms, the 16 MiB bound. Anything malformed
//! is a misuse (exit 2, empty stdout); every path is read and every form checked before the law predicate runs.

use crate::{answer, Accession, Declaration, Logs, Publication, Status};
use galeed::hexform;
use galeed::json::{self, Value};
use std::collections::BTreeSet;
use std::ffi::OsString;

pub const CMD_DECODE: &str = "decode";
pub const CMD_PUBLICATION: &str = "publication";
pub const CMD_RESOLVE_IDENTITY: &str = "resolve-identity";
pub const CMD_CURRENT_AT: &str = "current-at";
pub const CMD_GUILD_CARD: &str = "guild-card";
pub const CMD_RESOLVE_GUILD: &str = "resolve-guild";
pub const CMD_BENCH: &str = "bench";
pub const CMD_CANDIDATES: &str = "candidates";
pub const CMD_JUDGE: &str = "judge";
pub const CMD_RESOLVE_JUDGE: &str = "resolve-judge";
pub const CMD_COVENANT: &str = "covenant";
pub const CMD_QUOTE: &str = "quote";

pub const F_PUBLICATIONS: &str = "publications";
pub const F_STATUSES: &str = "statuses";
pub const F_DECLARATIONS: &str = "declarations";
pub const F_ACCESSIONS: &str = "accessions";
pub const F_TO: &str = "to";
pub const F_HOLDER: &str = "holder";
pub const F_ROOT: &str = "root";
pub const F_EMITTER: &str = "emitter";
pub const F_BLOCK_NUMBER: &str = "blockNumber";
pub const F_LOG_INDEX: &str = "logIndex";
pub const F_DOCUMENT: &str = "document";
pub const F_WORD: &str = "word";
pub const F_ACTIVE: &str = "active";
pub const F_POINTERS: &str = "pointers";
pub const F_ALLOW: &str = "allow";
pub const F_DENY: &str = "deny";

pub const FILE_BOUND: u64 = 16 * 1024 * 1024;

/// A misuse: only the exit code and the empty stdout are contractual; the diagnostic is this implementation's own.
#[derive(Debug)]
pub struct Misuse(pub String);

fn misuse<T>(why: impl Into<String>) -> Result<T, Misuse> {
    Err(Misuse(why.into()))
}

fn arg_str(a: &OsString) -> Result<&str, Misuse> {
    a.to_str().ok_or_else(|| Misuse("参数不是 UTF-8".into()))
}

/// `<chainId>`, `<blockNumber>` and `<logIndex>`: the integer spelling of `galeed/1` §3.2.
fn int_arg(a: &OsString) -> Result<u64, Misuse> {
    crate::int_spelling(arg_str(a)?).ok_or_else(|| Misuse(format!("整数参数的拼法不合:{}", a.to_string_lossy())))
}

fn address_arg(a: &OsString) -> Result<String, Misuse> {
    let s = arg_str(a)?;
    if hexform::is_hex20(s) {
        Ok(s.to_string())
    } else {
        misuse(format!("地址参数不是 hex20:{s}"))
    }
}

/// Read one path: unreadable, over the bound or refused by the §1 reader are each a misuse.
fn read_value(a: &OsString) -> Result<Value, Misuse> {
    let b = std::fs::read(a).map_err(|e| Misuse(format!("读不到 {}:{e}", a.to_string_lossy())))?;
    if b.len() as u64 > FILE_BOUND {
        return misuse("档过十六 MiB");
    }
    json::read_open(&b).ok_or_else(|| Misuse(format!("{} 读者不收", a.to_string_lossy())))
}

fn int_at(v: &Value, k: &str) -> Option<u64> {
    match v.get(k) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    }
}

fn hex_at(v: &Value, k: &str, form: fn(&str) -> bool) -> Option<String> {
    v.get(k).and_then(Value::as_str).filter(|s| form(s)).map(str::to_string)
}

/// Transport hex: lowercase `0x` plus an even number of hex digits of either case; answers the bytes.
fn transport_hex(v: &Value, k: &str) -> Option<Vec<u8>> {
    hexform::decode_digits(v.get(k)?.as_str()?.strip_prefix("0x")?.as_bytes())
}

/// Form of a publications element: `document` is the transport hex of the log's document octets, `word` hex32.
pub fn publication_of(v: &Value) -> Option<Publication> {
    v.as_obj()?;
    Some(Publication {
        emitter: hex_at(v, F_EMITTER, hexform::is_hex20)?,
        block_number: int_at(v, F_BLOCK_NUMBER)?,
        log_index: int_at(v, F_LOG_INDEX)?,
        document: transport_hex(v, F_DOCUMENT)?,
        word: hex_at(v, F_WORD, hexform::is_hex32)?,
    })
}

pub fn status_of(v: &Value) -> Option<Status> {
    v.as_obj()?;
    Some(Status {
        emitter: hex_at(v, F_EMITTER, hexform::is_hex20)?,
        block_number: int_at(v, F_BLOCK_NUMBER)?,
        log_index: int_at(v, F_LOG_INDEX)?,
        active: match v.get(F_ACTIVE)? {
            Value::Bool(b) => *b,
            _ => return None,
        },
    })
}

/// Form of a `declarations` element (one `Declared` log, law §4.1).
pub fn declaration_of(v: &Value) -> Option<Declaration> {
    v.as_obj()?;
    Some(Declaration {
        emitter: hex_at(v, F_EMITTER, hexform::is_hex20)?,
        block_number: int_at(v, F_BLOCK_NUMBER)?,
        log_index: int_at(v, F_LOG_INDEX)?,
        to: hex_at(v, F_TO, hexform::is_hex20)?,
    })
}

/// Form of an `accessions` element (one `Acceded` log, law §4.1).
pub fn accession_of(v: &Value) -> Option<Accession> {
    v.as_obj()?;
    Some(Accession {
        emitter: hex_at(v, F_EMITTER, hexform::is_hex20)?,
        block_number: int_at(v, F_BLOCK_NUMBER)?,
        log_index: int_at(v, F_LOG_INDEX)?,
        holder: hex_at(v, F_HOLDER, hexform::is_hex20)?,
        root: hex_at(v, F_ROOT, hexform::is_hex20)?,
    })
}

fn arr_at<'a>(v: &'a Value, k: &str) -> Option<&'a [Value]> {
    match v.get(k)? {
        Value::Arr(a) => Some(a),
        _ => None,
    }
}

/// Form of `<logs.json>`: the four arrays present, every element well formed, and no `(blockNumber, logIndex)`
/// repeated across the four arrays together. Any violation is `None`. **The one reader of a logs-set file**: `gro` here and
/// `gpp service-of`/`serving` (HARNESS-PAPERS reads the file "the same way") both read through it.
#[cfg(not(feature = "ablate-k4-gro"))]
pub fn logs_of(v: &Value) -> Option<Logs> {
    let logs = logs_members(v)?;
    let mut seen = BTreeSet::new();
    let pairs = logs
        .publications
        .iter()
        .map(|p| (p.block_number, p.log_index))
        .chain(logs.statuses.iter().map(|s| (s.block_number, s.log_index)))
        .chain(logs.declarations.iter().map(|d| (d.block_number, d.log_index)))
        .chain(logs.accessions.iter().map(|x| (x.block_number, x.log_index)));
    for pair in pairs {
        if !seen.insert(pair) {
            return None;
        }
    }
    Some(logs)
}

/// Fault-injection variant for the conformance gauges: repeated `(blockNumber, logIndex)` pairs are not checked.
#[cfg(feature = "ablate-k4-gro")]
pub fn logs_of(v: &Value) -> Option<Logs> {
    let _ = BTreeSet::<()>::new();
    logs_members(v)
}

fn logs_file(a: &OsString) -> Result<Logs, Misuse> {
    logs_of(&read_value(a)?).ok_or_else(|| Misuse("日志档的形不合(或两条记录同一对 (blockNumber, logIndex))".into()))
}

fn logs_members(v: &Value) -> Option<Logs> {
    v.as_obj()?;
    let publications = arr_at(v, F_PUBLICATIONS)?.iter().map(publication_of).collect::<Option<Vec<_>>>()?;
    let statuses = arr_at(v, F_STATUSES)?.iter().map(status_of).collect::<Option<Vec<_>>>()?;
    let declarations = arr_at(v, F_DECLARATIONS)?.iter().map(declaration_of).collect::<Option<Vec<_>>>()?;
    let accessions = arr_at(v, F_ACCESSIONS)?.iter().map(accession_of).collect::<Option<Vec<_>>>()?;
    Some(Logs { publications, statuses, declarations, accessions })
}

/// Form of `<query.json>`: `pointers` present and a list of strings; `allow` and `deny` empty when absent,
/// and when present lists of hex20 (`null` does not qualify).
fn query_file(a: &OsString) -> Result<(Vec<String>, Vec<String>, Vec<String>), Misuse> {
    let v = read_value(a)?;
    let strings = |k: &str, form: fn(&str) -> bool| -> Option<Vec<String>> { arr_at(&v, k)?.iter().map(|x| x.as_str().filter(|s| form(s)).map(str::to_string)).collect() };
    let listed = |k: &str| -> Option<Vec<String>> { if v.get(k).is_none() { Some(Vec::new()) } else { strings(k, hexform::is_hex20) } };
    match (v.as_obj(), strings(F_POINTERS, |_| true), listed(F_ALLOW), listed(F_DENY)) {
        (Some(_), Some(p), Some(al), Some(de)) => Ok((p, al, de)),
        _ => misuse("查询档的形不合"),
    }
}

/// `<kind>`: exactly one of the five words naming a reserved prefix (law §1).
fn kind_arg(a: &OsString) -> Result<crate::Kind, Misuse> {
    let w = arg_str(a)?;
    crate::Kind::of_word(w).ok_or_else(|| Misuse(format!("kind 要是 roster、guildcard、judge、guildamanah、verifier 之一:{w}")))
}

/// `<codeHash>`: hex32.
fn code_hash_arg(a: &OsString) -> Result<String, Misuse> {
    let s = arg_str(a)?;
    if hexform::is_hex32(s) {
        Ok(s.to_string())
    } else {
        misuse(format!("codeHash 参数不是 hex32:{s}"))
    }
}

/// `<quotes.json>`: the root is an object; every member is decided by §5.10, none being misuse.
fn quotes_file(a: &OsString) -> Result<Value, Misuse> {
    let v = read_value(a)?;
    if v.as_obj().is_some() {
        Ok(v)
    } else {
        misuse("报价档的根不是对象")
    }
}

pub fn run(args: &[OsString]) -> Result<Value, Misuse> {
    let cmd = args.first().map(arg_str).transpose()?;
    let rest = args.get(1..).unwrap_or_default();
    match (cmd, rest) {
        (Some(CMD_DECODE), [file]) => {
            let v = read_value(file)?;
            let p = publication_of(&v).ok_or_else(|| Misuse("发布档的形不合".into()))?;
            Ok(answer::decode(&crate::decode(&p)))
        }
        (Some(CMD_PUBLICATION), [logs, addr, kind]) => {
            let (logs, addr, kind) = (logs_file(logs)?, address_arg(addr)?, kind_arg(kind)?);
            Ok(answer::publication(&logs, &addr, kind))
        }
        // `<chainId>` is read for its spelling; §5.3 reads no chain.
        (Some(CMD_RESOLVE_IDENTITY), [logs, chain, addr]) => {
            let (logs, _, addr) = (logs_file(logs)?, int_arg(chain)?, address_arg(addr)?);
            Ok(answer::identity(&crate::resolve_identity(&logs, &addr)))
        }
        (Some(CMD_CURRENT_AT), [logs, addr, bn, li]) => {
            let (logs, addr, bn, li) = (logs_file(logs)?, address_arg(addr)?, int_arg(bn)?, int_arg(li)?);
            Ok(answer::current(&crate::current_at(&logs, &addr, (bn, li))))
        }
        (Some(CMD_RESOLVE_GUILD), [logs, chain, addr]) => {
            let (logs, chain, addr) = (logs_file(logs)?, int_arg(chain)?, address_arg(addr)?);
            Ok(answer::guild(&crate::guild_of(&logs, chain, &addr)))
        }
        (Some(CMD_GUILD_CARD), [logs, addr]) => {
            let (logs, addr) = (logs_file(logs)?, address_arg(addr)?);
            Ok(answer::card(&crate::guild_card_of(&logs, &addr)))
        }
        (Some(CMD_JUDGE), [logs, addr]) => {
            let (logs, addr) = (logs_file(logs)?, address_arg(addr)?);
            Ok(answer::judge(&crate::judge_of(&logs, &addr)))
        }
        // `<chainId>` is read for its spelling; §5.8 reads no chain.
        (Some(CMD_RESOLVE_JUDGE), [logs, chain, addr]) => {
            let (logs, _, addr) = (logs_file(logs)?, int_arg(chain)?, address_arg(addr)?);
            Ok(answer::resolved_judge(&crate::resolve_judge(&logs, &addr)))
        }
        (Some(CMD_COVENANT), [logs, chain, addr]) => {
            let (logs, chain, addr) = (logs_file(logs)?, int_arg(chain)?, address_arg(addr)?);
            Ok(answer::covenant(&crate::covenant_of(&logs, chain, &addr)))
        }
        (Some(CMD_QUOTE), [quotes, chain, contract, code_hash, token @ ..]) if token.len() <= 1 => {
            let (q, chain, contract, code_hash) = (quotes_file(quotes)?, int_arg(chain)?, address_arg(contract)?, code_hash_arg(code_hash)?);
            let token = token.first().map(address_arg).transpose()?;
            Ok(answer::quote(&crate::quote_of(&q, chain, &contract, &code_hash, token.as_deref())))
        }
        (Some(CMD_BENCH), [logs, chain, bench]) => {
            let (logs, chain, b) = (logs_file(logs)?, int_arg(chain)?, read_value(bench)?);
            Ok(answer::bench(&crate::bench_value(&logs, chain, &b)))
        }
        (Some(CMD_CANDIDATES), [logs, chain, query]) => {
            let (logs, chain, (pointers, allow, deny)) = (logs_file(logs)?, int_arg(chain)?, query_file(query)?);
            Ok(answer::candidates(&crate::candidates(&logs, chain, &pointers, &allow, &deny)))
        }
        _ => misuse("用法:gro decode|publication|resolve-identity|current-at|resolve-guild|judge|guild-card|resolve-judge|covenant|quote|bench|candidates …(参数照 HARNESS-ROSTER.md)"),
    }
}
