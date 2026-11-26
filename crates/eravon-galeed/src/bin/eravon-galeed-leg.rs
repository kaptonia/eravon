#![forbid(unsafe_code)]

use std::process::ExitCode;

use eravon_literals::misuse::{self, Misuse};

use eravon_galeed as k1;
use k1::Value;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;

mod expected {
    pub const KECCAK_EMPTY: &str =
        "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";
    pub const SHA256_EMPTY: &str =
        "0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    pub const ADDRESS_OF_KEY_ONE: &str = "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf";
    pub const SIG_R: &str = "0xe7c93726a865578504442b1a6827f676e0ed74bdff2be3960d1e253bbcfc4462";
    pub const SIG_S: &str = "0x6aa772b878bc912bdbb33a0014ec507c4b3896ea85aa914b74dee9b7ac3e56da";
    pub const SIG_V: u8 = 28;
}

fn argv() -> Option<Vec<String>> {
    std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect()
}

fn main() -> ExitCode {
    if let Some(a) = argv() {
        misuse::keep_arguments(&a);
    }
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(m) => {
            m.say("eravon-galeed-leg");
            ExitCode::from(EXIT_MISUSE)
        }
    }
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_CANON: &str = "galeed/1 §10 · 正典形与条目信封闭集";
    pub const ALL: &[Verb] = &[
        Verb::plain("selftest", &[]),
        Verb::carry("canon", &[Arg::need("--case", T_PATH)], &[C_CANON]),
        Verb::carry("entry", &[Arg::need("--case", T_PATH)], &[C_CANON]),
        Verb::plain("schema", &[]),
    ];
}

fn run() -> Result<u8, Misuse> {
    let args: Vec<String> = argv().ok_or_else(Misuse::not_text)?;
    eravon_literals::trace::entry("eravon-galeed-leg", k1::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
    match args.first().map(String::as_str) {
        Some("selftest") if args.len() == 1 => Ok(selftest()),
        Some("schema") if args.len() == 1 => {
            let mut m = k1::schema::value_rows(eravon_literals::schema::register(
                &eravon_literals::schema::version("eravon-galeed-leg"),
                register::ALL,
                &[],
            ));
            m.push(("ok".into(), Value::Bool(true)));
            print(&Value::Obj(m));
            Ok(EXIT_OK)
        }
        Some("canon") => case_op("canon", &args[1..]),
        Some("entry") => case_op("entry", &args[1..]),
        Some(other) => Err(Misuse::verb(Some(other))),
        None => Err(Misuse::verb(None)),
    }
}

fn case_op(op: &'static str, rest: &[String]) -> Result<u8, Misuse> {
    let pairs = eravon_literals::schema::read_pairs(rest)?;
    eravon_literals::schema::form_gate(register::ALL, op, &pairs)?;
    let path = pairs.iter().find(|(k, _)| k == "--case").map(|(_, v)| v.clone()).ok_or_else(|| Misuse::missing("--case"))?;
    let path = &path;
    let bytes = read_case(path)?;

    let mut members: Vec<(String, Value)> = vec![
        ("v".into(), Value::Str("eravon-galeed-leg/1".into())),
        ("op".into(), Value::Str(op.into())),
        ("bytes".into(), Value::Int(bytes.len() as u64)),
    ];

    if op == "canon" {
        match k1::canon::read(&bytes).and_then(|v| k1::canon::canonical(&v)) {
            Err(t) => members.push(("token".into(), Value::Str(t.name().into()))),
            Ok(canon) => {
                members.push(("token".into(), Value::Str("ok".into())));
                members.push((
                    "canonSha256".into(),
                    Value::Str(k1::hash::hex_encode(&k1::hash::sha256(&canon))),
                ));
            }
        }
    } else {
        match k1::entry::accept(&bytes) {
            Err(t) => members.push(("token".into(), Value::Str(t.name().into()))),
            Ok(e) => {
                members.push(("token".into(), Value::Str("ok".into())));
                members.push((
                    "entryId".into(),
                    Value::Str(k1::hash::hex_encode(&e.entry_id)),
                ));
            }
        }
    }

    print(&Value::Obj(members));
    Ok(EXIT_OK)
}

fn read_case(path: &str) -> Result<Vec<u8>, Misuse> {
    std::fs::read(path).map_err(|_| Misuse::unreadable(path, misuse::UNREADABLE))
}


fn selftest() -> u8 {
    match legs() {
        Ok(()) => {
            print(&Value::Obj(vec![
                ("v".into(), Value::Str("eravon-galeed-leg/1".into())),
                ("op".into(), Value::Str("selftest".into())),
                ("ok".into(), Value::Bool(true)),
                ("checked".into(), Value::Int(5)),
            ]));
            EXIT_OK
        }
        Err((token, detail)) => {
            print(&Value::Obj(vec![
                ("v".into(), Value::Str("eravon-galeed-leg/1".into())),
                ("op".into(), Value::Str("selftest".into())),
                ("ok".into(), Value::Bool(false)),
                ("token".into(), Value::Str(token.into())),
                ("detail".into(), Value::Str(detail)),
            ]));
            EXIT_OK
        }
    }
}

fn legs() -> Result<(), (&'static str, String)> {
    let canonical_bytes = br#"{"a":1,"b":"x"}"#;
    let accepted = k1::canon::accept(canonical_bytes)
        .map_err(|t| ("E_LEG_CANON", format!("正典字节被拒:{}", t.name())))?;
    if k1::canon::canonical(&accepted).ok().as_deref() != Some(&canonical_bytes[..]) {
        return Err(("E_LEG_CANON", "正典字节转一圈变了样".into()));
    }
    match k1::canon::accept(br#"{"b":"x","a":1}"#) {
        Err(t) if t.name() == "E_NOT_CANONICAL" => {}
        Err(t) => return Err(("E_LEG_CANON", format!("乱序的字节该报 E_NOT_CANONICAL,报的是 {}", t.name()))),
        Ok(_) => return Err(("E_LEG_CANON", "乱序的字节被当成正典收下了".into())),
    }

    if k1::hash::hex_encode(&k1::hash::keccak256(b"")) != expected::KECCAK_EMPTY {
        return Err(("E_LEG_KECCAK", "keccak256(空串)对不上 cast keccak 的答".into()));
    }
    if k1::hash::hex_encode(&k1::hash::sha256(b"")) != expected::SHA256_EMPTY {
        return Err(("E_LEG_SHA256", "sha256(空串)对不上 shasum 的答".into()));
    }

    let mut key_bytes = [0u8; 32];
    key_bytes[31] = 1;
    let key = k1::sign::Privkey::from_bytes(&key_bytes)
        .ok_or(("E_LEG_SIGN", "私钥 0x…01 立不起来".to_string()))?;
    if k1::hash::hex_encode(&key.address()) != expected::ADDRESS_OF_KEY_ONE {
        return Err(("E_LEG_SIGN", "地址对不上 cast wallet address 的答".into()));
    }
    let digest = [0x11u8; 32];
    let sig = k1::sign::sign_digest(&key, &digest);
    if k1::hash::hex_encode(&sig.r) != expected::SIG_R
        || k1::hash::hex_encode(&sig.s) != expected::SIG_S
        || sig.v != expected::SIG_V
    {
        return Err(("E_LEG_SIGN", "签名三段对不上 cast wallet sign 的答".into()));
    }
    if !k1::sign::scalar_in_range(&sig.s) || k1::sign::is_high_s(&sig.s) {
        return Err(("E_LEG_SIGN", "这枚签名的 s 不在低半区".into()));
    }
    match k1::sign::recover(&digest, &sig.r, &sig.s, sig.v - 27) {
        Some(a) if k1::hash::hex_encode(&a) == expected::ADDRESS_OF_KEY_ONE => {}
        Some(a) => {
            return Err((
                "E_LEG_SIGN",
                format!("恢复出来的是 {},不是签的那一把", k1::hash::hex_encode(&a)),
            ));
        }
        None => return Err(("E_LEG_SIGN", "恢复不出地址".into())),
    }

    match k1::entry::accept(b"{}") {
        Err(t) if t.name() == "E_ENVELOPE_MISSING" => {}
        Err(t) => {
            return Err((
                "E_LEG_ENVELOPE",
                format!("空对象该报 E_ENVELOPE_MISSING,报的是 {}", t.name()),
            ));
        }
        Ok(_) => return Err(("E_LEG_ENVELOPE", "空对象被当成条目收下了".into())),
    }

    let body = b"hello carried";
    let frame = abi_bytes_frame(body);
    let h = k1::hash::sha256(body);
    match k1::carried::anchor_bytes(&frame, &h) {
        Some(got) if got == body => {}
        Some(_) => return Err(("E_LEG_CARRIED", "携带形取出来的不是那份字节".into())),
        None => return Err(("E_LEG_CARRIED", "框对、摘要对的携带形没被认出来".into())),
    }
    let mut wrong = h;
    wrong[0] ^= 0xff;
    if k1::carried::anchor_bytes(&frame, &wrong).is_some() {
        return Err(("E_LEG_CARRIED", "摘要不符的携带形照样被收下了".into()));
    }

    Ok(())
}

fn abi_bytes_frame(body: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8; 64];
    v[31] = 32;
    let len = body.len() as u64;
    v[56..64].copy_from_slice(&len.to_be_bytes());
    v.extend_from_slice(body);
    while v.len() % 32 != 0 {
        v.push(0);
    }
    v
}

fn print(v: &Value) {
    use std::io::Write;
    let bytes = eravon_galeed::schema::answer_bytes(v);
    let out = std::io::stdout();
    let mut h = out.lock();
    if h.write_all(&bytes).is_err() || h.flush().is_err() {
        std::process::exit(EXIT_MISUSE as i32);
    }
}
