//! `gl1` 夹具二进制(K1;契约见基底 galeed-conformance/HARNESS.md)。
//!
//! stdout 恰一个正典 JSON 值,无尾换行;答出即退出码 0(拒绝也是答);哈尼斯误用(读不到的路径、
//! 坏参数、多出的参数、没产出 JSON 的一切结局)退出码 2 且 stdout 空,诊断走 stderr。
//! `audit` 的输入读者与 NO_LABEL 各形照 HARNESS.md 的 audit 一节。
#![forbid(unsafe_code)]
#![cfg_attr(feature = "ablate-k1-gl1-audit", allow(unused_imports))]

use galeed::cryptox::Privkey;
use galeed::hexform;
use galeed::json::{self, Value};
use galeed::{audit, envelope, kit, sign};
use std::ffi::OsString;
use std::io::Write;

const MISUSE: i32 = 2;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    #[cfg(feature = "ablate-k1-misuse")]
    let args = lenient(args);
    let verb = args.first().and_then(|a| a.to_str());
    match (verb, args.len()) {
        (Some("check"), 2) => check(&args[1]),
        (Some("canon"), 2) => canon(&args[1]),
        (Some("sign"), 3 | 4) => sign_cmd(&args[1], &args[2], args.get(3)),
        (Some("audit"), 2) => audit_cmd(&args[1]),
        (Some("kit"), 2) => kit_cmd(&args[1]),
        (Some("mnemonic"), 2) => mnemonic_cmd(&args[1]),
        (Some("derive"), 2) => derive_cmd(&args[1]),
        (Some("keystore"), 3) => keystore_cmd(&args[1], &args[2]),
        (Some("keystore-roundtrip"), 3) => keystore_roundtrip_cmd(&args[1], &args[2]),
        (Some("vault"), 2) => vault_cmd(&args[1]),
        (Some("answer"), 2) => answer_cmd(&args[1]),
        _ => misuse("用法:gl1 check <path> | canon <path> | sign <privkey-hex> <path> [domain] | audit <path> | kit <path> | mnemonic <script> | derive <script> | keystore <file> <password-file> | keystore-roundtrip <privkey-file> <password-file> | vault <script> | answer <path>"),
    }
}

/// Fault-injection variant for the conformance gauges: arguments past a verb's last are dropped and the verb answers,
/// where the harness says an extra argument is misuse (exit 2, stdout empty).
#[cfg(feature = "ablate-k1-misuse")]
fn lenient(mut args: Vec<OsString>) -> Vec<OsString> {
    let most = match args.first().and_then(|a| a.to_str()) {
        Some("sign") => 4,
        Some("keystore" | "keystore-roundtrip") => 3,
        _ => 2,
    };
    args.truncate(most);
    args
}

fn misuse(why: &str) -> i32 {
    eprintln!("gl1: {why}");
    MISUSE
}

/// 答一个值:正典字节上 stdout,不带换行。
fn answer(v: &Value) -> i32 {
    let out = match json::canonical(v) {
        Ok(b) => b,
        Err(t) => return misuse(t.name()),
    };
    let mut so = std::io::stdout().lock();
    if so.write_all(&out).and_then(|()| so.flush()).is_err() {
        return misuse("stdout 写不出");
    }
    0
}

/// 产品自有门(夹具契约之外的 kit、vault、mnemonic、derive、keystore 两扇)答一行:先过门面那一道
/// (`galeed::answer::stamp`:每行 `ok`,拒行带 `refused`),补不齐即误用。契约四门(check、canon、sign、
/// audit)的字形归 HARNESS.md,直走 [`answer`]。
fn product(v: &Value) -> i32 {
    match galeed::answer::stamp(v.clone()) {
        Ok(v) => answer(&v),
        Err(u) => misuse(u.why()),
    }
}

/// 门面那一道的直通门(`gl1 answer <档>`):档里那一个 JSON 值原样过 [`product`](`galeed::answer::stamp`),
/// 补得齐即印那一行、退出零;补不齐(拒行拼不出名、`ok` 不是布尔、不是对象)即误用退出,stdout 一字不印。
/// 档读不成或不是 JSON 也是误用。
fn answer_cmd(p: &OsString) -> i32 {
    let raw = match read_file(p) {
        Ok(b) => b,
        Err(code) => return code,
    };
    match galeed::json::read_open(&raw) {
        Some(v) => product(&v),
        None => misuse("答话档不是 JSON"),
    }
}

fn obj(members: Vec<(&str, Value)>) -> Value {
    Value::Obj(members.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn rejected(t: galeed::token::Token) -> Value {
    obj(vec![("ok", Value::Bool(false)), ("token", s(t.name()))])
}

/// 路径按操作系统交来的字节原样递给文件系统;读不到即误用。
fn read_file(p: &OsString) -> Result<Vec<u8>, i32> {
    std::fs::read(p).map_err(|e| misuse(&format!("读不到 {}:{}", p.to_string_lossy(), e)))
}

fn check(path: &OsString) -> i32 {
    let b = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    match envelope::accept(&b) {
        Ok(e) => answer(&obj(vec![("entry_id", s(&hexform::encode(&e.entry_id))), ("ok", Value::Bool(true))])),
        Err(t) => answer(&rejected(t)),
    }
}

fn canon(path: &OsString) -> i32 {
    let b = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    match json::read(&b).and_then(|v| json::canonical(&v)) {
        Ok(c) => answer(&obj(vec![("canon", s(&hexform::encode(&c))), ("ok", Value::Bool(true))])),
        Err(t) => answer(&rejected(t)),
    }
}

/// `<privkey-hex>`:64 个十六进制数字,任一大小写,可带 `0x` / `0X`;标量在 [1, n − 1]。
fn privkey(arg: &OsString) -> Option<Privkey> {
    let t = arg.to_str()?;
    let digits = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    if digits.len() != 64 {
        return None;
    }
    let raw = hexform::decode_digits(digits.as_bytes())?;
    let mut k = [0u8; 32];
    k.copy_from_slice(&raw);
    Privkey::from_bytes(&k)
}

/// `[domain]`:缺席即 galeed/1;在场则每字节低于 0x80 且无换行,空串也收。
fn domain(arg: Option<&OsString>) -> Option<Vec<u8>> {
    match arg {
        None => Some(galeed::LAW.as_bytes().to_vec()),
        Some(a) => {
            let t = a.to_str()?;
            let b = t.as_bytes();
            if b.iter().all(|c| *c < 0x80 && *c != 0x0A) {
                Some(b.to_vec())
            } else {
                None
            }
        }
    }
}

/// HARNESS.md `gl1 audit`:输入读者读不成、根非对象、必填缺或形不合、§9.4 良构不过,全答 NO_LABEL(退出码 0);
/// 只有读不到路径与坏参数是误用。
#[cfg(not(feature = "ablate-k1-gl1-audit"))]
fn audit_cmd(path: &OsString) -> i32 {
    let b = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let report = json::read_open(&b).as_ref().and_then(audit::read_input).map(|i| audit::audit(&i)).unwrap_or_else(audit::no_label);
    answer(&report)
}

/// 消融形:audit 不出面,退回批二的空壳(任何调用退出码 2)。
#[cfg(feature = "ablate-k1-gl1-audit")]
fn audit_cmd(path: &OsString) -> i32 {
    let _ = path;
    misuse("audit 出面被消融")
}

/// `gl1 kit <path>`:§9.6 取证包判定(HARNESS.md 契约之外的第五个动词;compare.py 不碰它)。
/// 读不成的包答 `{ok:false, reason:"kit"}`;读不到路径才是误用。
fn kit_cmd(path: &OsString) -> i32 {
    let b = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let j = match json::read_open(&b) {
        Some(v) => kit::judge(&v),
        None => kit::Judgment::Nothing("kit"),
    };
    product(&kit::judgment_value(&j))
}

fn sign_cmd(key_arg: &OsString, path: &OsString, domain_arg: Option<&OsString>) -> i32 {
    let Some(key) = privkey(key_arg) else {
        return misuse("privkey 要是 64 个十六进制数字(可带 0x),标量在 [1, n − 1]");
    };
    let Some(d) = domain(domain_arg) else {
        return misuse("domain 每字节要低于 0x80 且不含换行");
    };
    let b = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let v = match json::read(&b) {
        Ok(v) => v,
        Err(t) => return misuse(&format!("待签档过不了 §3.5 第 1 到 5 道:{}", t.name())),
    };
    if v.as_obj().is_none() {
        return misuse("待签档的值不是对象");
    }
    let bytes = match json::canonical(&v) {
        Ok(b) => b,
        Err(t) => return misuse(t.name()),
    };
    let signed = sign::sign_harness(&key, &bytes, &d);
    answer(&galeed::face::signed_value(&signed))
}

// ---------- 钥的派生与备份(HARNESS.md 之外;compare.py 不碰;K1 行程对公开向量读数) ----------

fn script(path: &OsString) -> Result<Value, i32> {
    let b = read_file(path)?;
    json::read_open(&b).filter(|v| v.as_obj().is_some()).ok_or_else(|| misuse("剧本要是 JSON 对象"))
}

fn hex_member(v: &Value, k: &str) -> Option<Vec<u8>> {
    let s = v.get(k)?.as_str()?;
    hexform::decode_digits(s.strip_prefix("0x").unwrap_or(s).as_bytes())
}

/// `gl1 vault <script>`:K6 金库密码学的驱动口。剧本 `{passcode?, seal?, derive?, scrub?}`,
/// 各段现做一遍,答结构。**秘密不上 stdout**:派出的钥只端 sha256,身份只端收件者那一半
/// (公钥);别处一律答形与布尔。
fn vault_cmd(path: &OsString) -> i32 {
    use galeed::cryptox::vault;
    let v = match script(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let digest = |b: &[u8]| s(&hexform::encode(&galeed::cryptox::sha256(b)));
    let bytes_of = |o: &Value, k: &str| o.get(k).and_then(Value::as_str).and_then(|t| hexform::decode_digits(t.strip_prefix("0x").unwrap_or(t).as_bytes()));
    let int_of = |o: &Value, k: &str| match o.get(k) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    };

    let bounds = obj(vec![("maxLogN", Value::Int(u64::from(vault::max_log_n()))), ("minLogN", Value::Int(u64::from(vault::min_log_n()))), ("p", Value::Int(u64::from(vault::KDF_P))), ("r", Value::Int(u64::from(vault::KDF_R)))]);

    let passcode = match v.get("passcode") {
        None => Value::Null,
        Some(p) => {
            let (Some(pw), Some(salt), Some(log_n)) = (bytes_of(p, "passcode"), bytes_of(p, "salt"), int_of(p, "logN")) else {
                return misuse("passcode 段要 {passcode, salt, logN},前二是十六进制");
            };
            let Ok(log_n) = u8::try_from(log_n) else { return misuse("logN 要放得进一个字节") };
            match vault::passcode_key(&pw, &salt, log_n) {
                None => obj(vec![("derived", Value::Bool(false)), ("logN", Value::Int(u64::from(log_n)))]),
                Some(k) => obj(vec![("derived", Value::Bool(true)), ("keyDigest", digest(&k[..])), ("logN", Value::Int(u64::from(log_n)))]),
            }
        }
    };

    let sealed = match v.get("seal") {
        None => Value::Null,
        Some(p) => {
            let (Some(key), Some(plain)) = (bytes_of(p, "key"), bytes_of(p, "plaintext")) else { return misuse("seal 段要 {key, plaintext},都是十六进制") };
            let Ok(key): Result<[u8; vault::KEY_LEN], _> = key.try_into() else { return misuse("seal.key 要是三十二字节") };
            let Some(sg) = vault::seal(&key, &plain) else { return misuse("随机源取不到") };
            // 一封一解、翻一位、换 nonce、换钥:四问各答一格。
            let opened = vault::open(&key, &sg.nonce, &sg.ciphertext);
            let mut flipped = sg.ciphertext.clone();
            if let Some(b) = flipped.first_mut() {
                *b ^= 1;
            }
            let mut other_nonce = sg.nonce;
            other_nonce[0] ^= 1;
            let mut other_key = key;
            other_key[0] ^= 1;
            obj(vec![
                ("expansion", Value::Int((sg.ciphertext.len() - plain.len()) as u64)),
                ("nonceLen", Value::Int(sg.nonce.len() as u64)),
                ("opened", Value::Bool(opened.map(|p| p.to_vec()) == Some(plain.clone()))),
                ("openedWithAnotherKey", Value::Bool(vault::open(&other_key, &sg.nonce, &sg.ciphertext).is_some())),
                ("openedWithAnotherNonce", Value::Bool(vault::open(&key, &other_nonce, &sg.ciphertext).is_some())),
                ("openedWhenFlipped", Value::Bool(vault::open(&key, &sg.nonce, &flipped).is_some())),
            ])
        }
    };

    let derived = match v.get("derive") {
        None => Value::Null,
        Some(p) => {
            let (Some(ikm), Some(salt)) = (bytes_of(p, "ikm"), bytes_of(p, "salt")) else { return misuse("derive 段要 {ikm, salt},都是十六进制") };
            let (Some(r), Some(a)) = (vault::recovery_key(&ikm, &salt), vault::age_secret_bytes(&ikm)) else {
                return misuse("登记簿里读不出用途串");
            };
            let Some(id) = vault::age_identity(&ikm) else { return misuse("age 身份算不出") };
            let again = vault::age_identity(&ikm).map(|x| x.recipient());
            obj(vec![
                ("ageRecipient", s(&id.recipient())),
                ("ageSecretDigest", digest(&a[..])),
                ("distinct", Value::Bool(r[..] != a[..])),
                ("recomputedEqual", Value::Bool(again.as_deref() == Some(id.recipient().as_str()))),
                ("recoveryDigest", digest(&r[..])),
            ])
        }
    };

    let scrubbed = match v.get("scrub") {
        None => Value::Null,
        Some(p) => {
            let Some(mut secret) = bytes_of(p, "secret").filter(|b| !b.is_empty()) else { return misuse("scrub 段要 {secret},非空十六进制") };
            let before = hexform::encode(&secret);
            vault::scrub(&mut secret[..]);
            obj(vec![("after", s(&hexform::encode(&secret))), ("before", s(&before))])
        }
    };

    // 量具驱动口(只增):场给钥与明文,答 nonce 与密文字节;场给钥、nonce、密文,答解出的明文字节。
    // 尺要以契约点名的算法与产品互解,故这两形端的是**字节**,不是摘要。
    // **只在场问了才答**:既有那六格的答话因此一字不动。
    let seal_bytes = match v.get("sealBytes") {
        None => None,
        Some(p) => {
            let (Some(key), Some(plain)) = (bytes_of(p, "key"), bytes_of(p, "plaintext")) else { return misuse("sealBytes 段要 {key, plaintext},都是十六进制") };
            let Ok(key): Result<[u8; vault::KEY_LEN], _> = key.try_into() else { return misuse("sealBytes.key 要是三十二字节") };
            let Some(sg) = vault::seal(&key, &plain) else { return misuse("随机源取不到") };
            Some(obj(vec![("ciphertext", s(&hexform::encode(&sg.ciphertext))), ("nonce", s(&hexform::encode(&sg.nonce)))]))
        }
    };

    let open_bytes = match v.get("openBytes") {
        None => None,
        Some(p) => {
            let (Some(key), Some(nonce), Some(ct)) = (bytes_of(p, "key"), bytes_of(p, "nonce"), bytes_of(p, "ciphertext")) else {
                return misuse("openBytes 段要 {key, nonce, ciphertext},都是十六进制");
            };
            let Ok(key): Result<[u8; vault::KEY_LEN], _> = key.try_into() else { return misuse("openBytes.key 要是三十二字节") };
            let Ok(nonce): Result<[u8; vault::NONCE_LEN], _> = nonce.try_into() else { return misuse("openBytes.nonce 要是二十四字节") };
            Some(obj(vec![("plaintext", vault::open(&key, &nonce, &ct).map_or(Value::Null, |b| s(&hexform::encode(&b))))]))
        }
    };

    // 标定与时间有关,故只答「答得出且落在两界之内」,不答那个数(行程要逐字节可重放)。
    let calibrated = match v.get("calibrate") {
        Some(Value::Bool(true)) => {
            let got = vault::calibrate_log_n();
            obj(vec![
                ("answered", Value::Bool(got.is_some())),
                ("withinBounds", Value::Bool(got.is_some_and(|l| (vault::min_log_n()..=vault::max_log_n()).contains(&l)))),
            ])
        }
        _ => Value::Null,
    };

    let mut members = vec![("bounds", bounds), ("calibrate", calibrated), ("derive", derived), ("passcode", passcode), ("scrub", scrubbed), ("seal", sealed)];
    if let Some(x) = seal_bytes {
        members.push(("sealBytes", x));
    }
    if let Some(x) = open_bytes {
        members.push(("openBytes", x));
    }
    product(&obj(members))
}

/// `gl1 mnemonic <script>`:剧本 `{entropy?, mnemonic?, passphrase?}`。答熵出的助记词、助记词反读的熵(或拒名)、
/// 种子与主钥 `xprv`(助记词成立时)。
fn mnemonic_cmd(path: &OsString) -> i32 {
    use galeed::cryptox;
    let v = match script(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let from_entropy = match v.get("entropy") {
        None => Value::Null,
        Some(_) => match hex_member(&v, "entropy").and_then(|e| cryptox::mnemonic_from_entropy(&e)) {
            Some(m) => s(&m),
            None => obj(vec![("ok", Value::Bool(false)), ("reason", s("entropy"))]),
        },
    };
    let passphrase = v.get("passphrase").and_then(Value::as_str).unwrap_or("");
    let (check, seed, xprv) = match v.get("mnemonic").and_then(Value::as_str) {
        None => (Value::Null, Value::Null, Value::Null),
        Some(m) => match cryptox::mnemonic_entropy(m) {
            Err(e) => {
                let mut f = vec![("ok", Value::Bool(false)), ("token", s(e.name()))];
                if let cryptox::MnemonicError::UnknownWord(i) = e {
                    f.push(("index", Value::Int(i as u64)));
                }
                (obj(f), Value::Null, Value::Null)
            }
            Ok(ent) => match cryptox::mnemonic_seed(m, passphrase) {
                None => (obj(vec![("entropy", s(&hexform::encode(&ent))), ("ok", Value::Bool(true))]), obj(vec![("ok", Value::Bool(false)), ("reason", s("passphrase"))]), Value::Null),
                Some(seed) => (
                    obj(vec![("entropy", s(&hexform::encode(&ent))), ("ok", Value::Bool(true))]),
                    s(&hexform::encode(&seed)),
                    cryptox::bip32_master(&seed).map_or(Value::Null, |k| s(&k.xprv())),
                ),
            },
        },
    };
    product(&obj(vec![("check", check), ("fromEntropy", from_entropy), ("seed", seed), ("xprv", xprv)]))
}

/// `gl1 derive <script>`:剧本 `{seed, path}`。答该路径的 `xprv`、私钥与地址,或拒名(`path` 形不合、`child` 无此子)。
fn derive_cmd(path: &OsString) -> i32 {
    use galeed::cryptox;
    let v = match script(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (Some(seed), Some(p)) = (hex_member(&v, "seed"), v.get("path").and_then(Value::as_str)) else { return misuse("剧本要有 seed 与 path") };
    if cryptox::parse_path(p).is_none() {
        return product(&obj(vec![("ok", Value::Bool(false)), ("reason", s("path"))]));
    }
    match cryptox::derive_path(&seed, p) {
        None => product(&obj(vec![("ok", Value::Bool(false)), ("reason", s("child"))])),
        Some(k) => product(&obj(vec![
            ("address", k.privkey().map_or(Value::Null, |pk| s(&hexform::encode(&pk.address())))),
            ("ok", Value::Bool(true)),
            ("privateKey", s(&hexform::encode(&k.key))),
            ("xprv", s(&k.xprv())),
        ])),
    }
}

/// `gl1 keystore <file> <password-file>`:keystore v3 解密,答地址或拒名。
fn keystore_cmd(file: &OsString, password: &OsString) -> i32 {
    use galeed::cryptox;
    let (b, pw) = match (read_file(file), read_file(password)) {
        (Ok(b), Ok(pw)) => (b, pw),
        (Err(c), _) | (_, Err(c)) => return c,
    };
    match cryptox::keystore_decrypt(&b, cryptox::file_secret(&pw)) {
        Ok(k) => product(&obj(vec![("address", Privkey::from_bytes(&k).map_or(Value::Null, |pk| s(&hexform::encode(&pk.address())))), ("ok", Value::Bool(true))])),
        Err(e) => product(&obj(vec![("ok", Value::Bool(false)), ("token", s(e.name()))])),
    }
}

/// `gl1 keystore-roundtrip <privkey-file> <password-file>`:加密(随机盐与 IV)再解开,答两头的地址与是否同一把。
fn keystore_roundtrip_cmd(key: &OsString, password: &OsString) -> i32 {
    use galeed::cryptox;
    let (kb, pw) = match (read_file(key), read_file(password)) {
        (Ok(k), Ok(p)) => (k, p),
        (Err(c), _) | (_, Err(c)) => return c,
    };
    let Some(pk) = std::str::from_utf8(&kb).ok().map(|t| t.trim_end_matches('\n')).and_then(|t| hexform::decode_digits(t.strip_prefix("0x").unwrap_or(t).as_bytes())).and_then(|b| <[u8; 32]>::try_from(b).ok()).and_then(|b| Privkey::from_bytes(&b)) else {
        return misuse("钥档要是 64 个十六进制数字");
    };
    let file = match cryptox::keystore_encrypt(&pk, cryptox::file_secret(&pw)) {
        Ok(f) => f,
        Err(e) => return product(&obj(vec![("ok", Value::Bool(false)), ("token", s(e.name()))])),
    };
    let back = cryptox::keystore_decrypt(&file, cryptox::file_secret(&pw)).ok().and_then(|b| Privkey::from_bytes(&b)).map(|k| hexform::encode(&k.address()));
    let addr = hexform::encode(&pk.address());
    product(&obj(vec![("address", s(&addr)), ("decrypted", back.clone().map_or(Value::Null, |a| s(&a))), ("same", Value::Bool(back.as_deref() == Some(addr.as_str())))]))
}

