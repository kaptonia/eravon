//! `literals check <FAMILY-LITERALS.md> <base-docs-dir> <zikaron-laws-dir> <GaleedRegistry.sol> <GaleedSuccessionBook.sol>`:K5 检查器。
//! `literals sign-as <privkey-hex> <path> <seat> <domain>`:按席位的放行门(契约 2、铁律 8),
//! 席位与域**成对**登记过才签;越出即误用形(退出码 2、stdout 空)。
//! `literals sign-registered <privkey-hex> <path> <domain>`:产品签名面的驱动口(K5 放行门),登记域答
//! HARNESS.md `gl1 sign` 的四成员形 `{digest, presig, sig, signer}`(同一签名路、同一出面件 `face::signed_value`)退出码 0;未登记域是「参数超出命令自己的表」(契约 10、HARNESS.md 的误用形):
//! 退出码 2、stdout 空、诊断走 stderr。
//! stdout 恰一个正典 JSON 值(`{ok, rules}`;任一不过 ok 为 false、带拒名 `refused`,退出码仍 0);每行过门面那一道(签名门的四成员形另带 `ok`);读不到路径与坏参数退出码 2。
#![forbid(unsafe_code)]

use galeed::json;
use galeed::literals;
use std::ffi::OsString;
use std::io::Write;

fn main() {
    std::process::exit(run());
}

fn misuse(why: &str) -> i32 {
    eprintln!("literals: {why}");
    2
}

fn read(p: &std::path::Path) -> Result<String, i32> {
    std::fs::read_to_string(p).map_err(|e| misuse(&format!("读不到 {}:{}", p.display(), e)))
}

/// 答一行:过门面那一道(`galeed::answer::stamp`)再上 stdout;补不齐的一行照误用退出。
fn emit(v: &galeed::json::Value) -> i32 {
    let v = match galeed::answer::stamp(v.clone()) {
        Ok(v) => v,
        Err(u) => return misuse(u.why()),
    };
    let out = match json::canonical(&v) {
        Ok(b) => b,
        Err(t) => return misuse(t.name()),
    };
    let mut so = std::io::stdout().lock();
    if so.write_all(&out).and_then(|()| so.flush()).is_err() {
        return misuse("stdout 写不出");
    }
    0
}

/// 两扇签名门共用的前三件:钥、待签档、拿去签的字节。
fn signing_inputs(args: &[OsString]) -> Result<(galeed::cryptox::Privkey, Vec<u8>), i32> {
    let key = args[1].to_str().and_then(|t| {
        let d = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
        let raw = galeed::hexform::decode_digits(d.as_bytes())?;
        let mut k = [0u8; 32];
        if raw.len() != 32 {
            return None;
        }
        k.copy_from_slice(&raw);
        galeed::cryptox::Privkey::from_bytes(&k)
    });
    let Some(key) = key else { return Err(misuse("privkey 要是 64 个十六进制数字,标量在 [1, n − 1]")) };
    let Ok(b) = std::fs::read(&args[2]) else { return Err(misuse("读不到待签档")) };
    let Some(v) = galeed::json::read(&b).ok().filter(|v| v.as_obj().is_some()) else { return Err(misuse("待签档要是过 §3.5 前五道的对象")) };
    Ok((key, json::canonical(&v).map_err(|t| misuse(t.name()))?))
}

fn sign_as(args: &[OsString]) -> i32 {
    let (key, bytes) = match signing_inputs(args) {
        Ok(x) => x,
        Err(c) => return c,
    };
    let (Some(seat), Some(domain)) = (args[3].to_str(), args[4].to_str()) else { return misuse("seat 与 domain") };
    match galeed::sign::sign_as(&key, &bytes, seat, domain) {
        Ok(sg) => emit(&galeed::face::signed_value(&sg)),
        Err(galeed::sign::UnregisteredDomain(pair)) => misuse(&format!("席位与域这一对 {pair:?} 未登记:签名面按 (席位, 登记域) 两项放行")),
    }
}

fn sign_registered(args: &[OsString]) -> i32 {
    // 退役:产品面只剩按席位那一扇。这一支留着只为把「域未登记」那一色答给行程,
    // 席位由登记簿按域现答。
    let key = args[1].to_str().and_then(|t| {
        let d = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
        let raw = galeed::hexform::decode_digits(d.as_bytes())?;
        let mut k = [0u8; 32];
        if raw.len() != 32 {
            return None;
        }
        k.copy_from_slice(&raw);
        galeed::cryptox::Privkey::from_bytes(&k)
    });
    let Some(key) = key else { return misuse("privkey 要是 64 个十六进制数字,标量在 [1, n − 1]") };
    let Ok(b) = std::fs::read(&args[2]) else { return misuse("读不到待签档") };
    let Some(v) = galeed::json::read(&b).ok().filter(|v| v.as_obj().is_some()) else { return misuse("待签档要是过 §3.5 前五道的对象") };
    let Some(domain) = args[3].to_str() else { return misuse("domain") };
    let Some(seat) = galeed::sign::seat_of_domain(domain) else {
        return misuse(&format!("域 {domain:?} 未登记:签字钥只签登记簿 signing-domain 角色的五域"));
    };
    let bytes = match json::canonical(&v) {
        Ok(b) => b,
        Err(t) => return misuse(t.name()),
    };
    match galeed::sign::sign_as(&key, &bytes, &seat, domain) {
        Ok(sg) => emit(&galeed::face::signed_value(&sg)),
        Err(galeed::sign::UnregisteredDomain(d)) => misuse(&format!("席位与域这一对 {d:?} 未登记")),
    }
}

fn run() -> i32 {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() == 4 && args[0].to_str() == Some("sign-registered") {
        return sign_registered(&args);
    }
    if args.len() == 5 && args[0].to_str() == Some("sign-as") {
        return sign_as(&args);
    }
    if args.len() != 6 || args[0].to_str() != Some("check") {
        return misuse("用法:literals check <FAMILY-LITERALS.md> <base-docs-dir> <zikaron-laws-dir> <GaleedRegistry.sol> <GaleedSuccessionBook.sol> | sign-registered <privkey-hex> <path> <domain> | sign-as <privkey-hex> <path> <seat> <domain>");
    }
    let reg_text = match read(std::path::Path::new(&args[1])) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let Some(reg) = literals::parse(&reg_text) else { return misuse("登记簿机读块读不成") };
    let docs = std::path::Path::new(&args[2]);
    let (galeed_law, papers_law, roster_law) = match (read(&docs.join(literals::GALEED_LAW_FILE)), read(&docs.join(literals::PAPERS_LAW_FILE)), read(&docs.join(literals::ROSTER_LAW_FILE))) {
        (Ok(a), Ok(b), Ok(r)) => (a, b, r),
        (Err(c), _, _) | (_, Err(c), _) | (_, _, Err(c)) => return c,
    };
    let zdir = std::path::Path::new(&args[3]);
    let mut zlaws: Vec<String> = Vec::new();
    match std::fs::read_dir(zdir) {
        Err(e) => return misuse(&format!("读不到 {}:{}", zdir.display(), e)),
        Ok(rd) => {
            let mut paths: Vec<std::path::PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "md")).collect();
            paths.sort();
            for p in paths {
                match read(&p) {
                    Ok(t) => zlaws.push(t),
                    Err(c) => return c,
                }
            }
        }
    }
    let (contract, book) = match (read(std::path::Path::new(&args[4])), read(std::path::Path::new(&args[5]))) {
        (Ok(c), Ok(b)) => (c, b),
        (Err(c), _) | (_, Err(c)) => return c,
    };
    let rules = literals::check(&reg, &galeed_law, &papers_law, &roster_law, &zlaws, &contract, &book, &galeed::sign::registered_domains(), &galeed::sign::registered_domains_by_seat());
    emit(&literals::report_value(&rules))
}

