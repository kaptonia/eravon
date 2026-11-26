use eravon_literals::json::{In, Out, read, write};

use crate::fault::{Fault, R};

pub const INT_MAX: u64 = (1u64 << 53) - 1;

pub const K_VERSION: &str = "v";

pub fn seal(version: &'static str, mut members: Vec<(&'static str, Out)>) -> Vec<u8> {
    members.push((K_VERSION, Out::S(version.to_string())));
    write(&Out::O(members))
}

pub fn open(version: &'static str, bytes: &[u8]) -> R<In> {
    let v = parse(bytes)?;
    match v.get(K_VERSION).and_then(|x| x.as_str()) {
        Some(got) if got == version => Ok(v),
        Some(got) => Err(Fault::Version { want: version, got: got.to_string() }),
        None => Err(Fault::Version { want: version, got: "(档上没有这一栏)".into() }),
    }
}

pub fn parse_any(bytes: &[u8]) -> R<In> {
    parse(bytes)
}

fn parse(bytes: &[u8]) -> R<In> {
    let v = read(bytes).map_err(|e| Fault::Form(format!("JSON 读不成:{e}")))?;
    match v {
        In::Obj(_) => Ok(v),
        _ => Err(Fault::Form("档的顶层要是一个对象".into())),
    }
}

pub fn need_str(v: &In, key: &str) -> R<String> {
    match v.get(key) {
        Some(In::Str(s)) => Ok(s.clone()),
        Some(other) => Err(Fault::Form(format!("{key} 要是字符串,档上是 {other:?}"))),
        None => Err(Fault::Form(format!("档上缺 {key}"))),
    }
}

pub fn need_u64(v: &In, key: &str) -> R<u64> {
    match v.get(key) {
        Some(In::Int(n)) if *n >= 0 => Ok(*n as u64),
        Some(In::Int(n)) => Err(Fault::Form(format!("{key} 要非负,档上是 {n}"))),
        Some(other) => Err(Fault::Form(format!("{key} 要是整数,档上是 {other:?}"))),
        None => Err(Fault::Form(format!("档上缺 {key}"))),
    }
}

pub fn need_arr<'a>(v: &'a In, key: &str) -> R<&'a [In]> {
    match v.get(key) {
        Some(In::Arr(a)) => Ok(a.as_slice()),
        Some(other) => Err(Fault::Form(format!("{key} 要是列,档上是 {other:?}"))),
        None => Err(Fault::Form(format!("档上缺 {key}"))),
    }
}

pub fn need_obj<'a>(v: &'a In, key: &str) -> R<&'a In> {
    match v.get(key) {
        Some(x @ In::Obj(_)) => Ok(x),
        Some(other) => Err(Fault::Form(format!("{key} 要是对象,档上是 {other:?}"))),
        None => Err(Fault::Form(format!("档上缺 {key}"))),
    }
}

pub fn need_time(t: u64) -> R<u64> {
    if t > INT_MAX { Err(Fault::Time(t)) } else { Ok(t) }
}
