#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use eravon_galeed as k1;
use eravon_galeed::Value;
use eravon_link as k3;
use eravon_terms as k2;
use k3::identity::{self, Records};
use k3::token::{Fault, Verdict};

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;
const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn argv() -> Option<Vec<String>> {
    std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect()
}

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erl", eravon_literals::trace::Feature::K3, args.first().map(String::as_str).unwrap_or(""));
    if !eravon_terms::form::registry_ok() {
        return misuse(&Misuse::args(eravon_literals::misuse::said("galeed 家族的登记簿给不出恰一条名录指针前缀(E_REGISTRY_UNUSABLE)", "the galeed family registry does not yield exactly one roster pointer prefix (E_REGISTRY_UNUSABLE)")));
    }
    match run(&args) {
        Ok(v) => {
            print(&v);
            ExitCode::from(eravon_galeed::schema::exit_of(&v))
        }
        Err(m) => misuse(&m),
    }
}

fn print(v: &Value) {
    write_out(&eravon_galeed::schema::answer_bytes(v));
}


fn misuse(m: &Misuse) -> ExitCode {
    m.say("erl");
    ExitCode::from(EXIT_MISUSE)
}


fn write_out(bytes: &[u8]) {
    use std::io::Write;
    let out = std::io::stdout();
    let mut h = out.lock();
    if h.write_all(bytes).is_err() || h.flush().is_err() {
        std::process::exit(EXIT_MISUSE as i32);
    }
}

fn ok(mut m: Vec<(String, Value)>) -> Value {
    m.push(("ok".into(), Value::Bool(true)));
    Value::Obj(m)
}
fn s(k: &str, v: String) -> (String, Value) {
    (k.into(), Value::Str(v))
}
fn i(k: &str, v: u64) -> (String, Value) {
    (k.into(), Value::Int(v))
}

fn read_file(path: &str) -> Result<Vec<u8>, Misuse> {
    let meta = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    if !meta.is_file() || meta.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
    }
    let b = std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    on_its_chain(&b);
    Ok(b)
}

fn on_its_chain(b: &[u8]) {
    use eravon_literals::json::In;
    let Ok(v) = eravon_literals::json::read(b) else { return };
    let id = |v: &In| match v.get("chainId") {
        Some(In::Int(n)) if *n > 0 => Some(*n as u64),
        _ => None,
    };
    if let Some(c) = id(&v).or_else(|| v.get("listing").and_then(id)) {
        if eravon_literals::constants::network_of(c).is_some() {
            eravon_literals::constants::select_chain(c);
        }
    }
}

fn read_records(path: &str) -> Result<Records, Misuse> {
    let b = read_file(path)?;
    let v = k1::canon::read_open(&b).ok_or_else(|| Misuse::unreadable(path, eravon_literals::misuse::NOT_JSON))?;
    Records::read(&v).ok_or_else(|| {
        Misuse::bad("records", path, eravon_literals::misuse::said("要是恰 chainId 与八张记录表的对象,各成员合形、位置不重", "an object of exactly chainId and the eight record lists, each member of its form, no position repeated"))
    })
}

fn arg_hex(name: &str, a: &str, n32: bool) -> Result<String, Misuse> {
    let ok = if n32 { k1::hexform::is_hex32(a) } else { k1::hexform::is_hex20(a) };
    if ok { Ok(a.to_string()) } else { Err(Misuse::bad(name, a, if n32 { eravon_literals::misuse::HEX32 } else { eravon_literals::misuse::HEX20 })) }
}

fn fault(f: &Fault) -> Value {
    f.to_value(true)
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    use eravon_literals::schema::{T_PATH, T_STR};

    const C_LISTING: &str = "eravon.link/1 · 清单判定闭集(E_LISTING_* 与它转上来的 E_RUNG_* · E_LADDER_* · E_JUDGE_* · E_GUILD_*)";
    const C_LINK: &str = "eravon.link/1 · 链接判定闭集(E_LINK_*)";
    const C_RECEIPT: &str = "eravon.link/1 · 回执判定闭集(E_RECEIPT_*)";
    const C_PACK: &str = "eravon.link/1 · 案件包判定闭集(E_PACK_*)";
    const C_SETTLE: &str = "eravon.link/1 · 和解判定闭集(E_SETTLE_*)";
    const C_TERMS: &str = "eravon.terms/1 · 条款判定闭集";
    const C_VERDICT: &str = "eravon.link/1 · 裁断名闭集(LISTING · SETTLE · LINK_* · BOUND · BIND_* · RECEIPT_* · SETTLE_* · LISTING_INVALID · TERMS_INVALID · PACK_INVALID)";
    const C_PAY: &str = "eravon.link/1 · 付款前核验裁断名闭集(PAYABLE · SELLER_KEY_RETIRED · UNPLACED · LISTING_INVALID)";
    const C_TRANSPORT: &str = "eravon.link/1 · 运输闭集(E_*_PREFIX · E_*_B64)";
    const C_CANON: &str = "galeed/1 §10 · 正典形与条目信封闭集";
    pub const ALL: &[Verb] = &[
        Verb::carry("check", &[Arg::need("listing-file", T_PATH)], &[C_LISTING, C_TERMS, C_CANON]),
        Verb::carry("bind", &[Arg::need("listing-file", T_PATH), Arg::need("terms-file", T_PATH)], &[C_VERDICT, C_LISTING, C_TERMS, C_CANON]),
        Verb::carry("link-check", &[Arg::need("link-file", T_PATH)], &[C_LINK, C_CANON]),
        Verb::carry("encode", &[Arg::need("link-file", T_PATH)], &[C_LINK, C_CANON]),
        Verb::new("decode", &[Arg::need("link-text-file", T_PATH)], &["E_LINK_DOC"], &[C_TRANSPORT, C_LINK, C_CANON]),
        Verb::carry("resolve-link", &[Arg::need("link-file", T_PATH), Arg::need("document-file", T_PATH)], &[C_VERDICT, C_LINK, C_LISTING, C_SETTLE, C_CANON]),
        Verb::carry("receipt-check", &[Arg::need("receipt-file", T_PATH)], &[C_RECEIPT, C_CANON]),
        Verb::carry("receipt-encode", &[Arg::need("receipt-file", T_PATH)], &[C_RECEIPT, C_CANON]),
        Verb::new("receipt-decode", &[Arg::need("receipt-text-file", T_PATH)], &["E_RECEIPT_DOC"], &[C_TRANSPORT, C_RECEIPT, C_CANON]),
        Verb::carry("bind-receipt", &[Arg::need("listing-file", T_PATH), Arg::need("receipt-file", T_PATH)], &[C_VERDICT, C_LISTING, C_RECEIPT, C_CANON]),
        Verb::carry("pack-check", &[Arg::need("pack-file", T_PATH)], &[C_PACK, C_CANON]),
        Verb::carry("settle-check", &[Arg::need("settle-file", T_PATH)], &[C_SETTLE, C_CANON]),
        Verb::carry("bind-settle", &[Arg::need("pack-file", T_PATH), Arg::need("settle-file", T_PATH)], &[C_VERDICT, C_PACK, C_SETTLE, C_CANON]),
        Verb::plain("place", &[Arg::need("records-file", T_PATH), Arg::need("seller-key", T_STR), Arg::need("terms-hash", T_STR)]),
        Verb::carry("pay-check", &[Arg::need("listing-file", T_PATH), Arg::need("records-file", T_PATH)], &[C_PAY, C_LISTING, C_TERMS, C_CANON]),
        Verb::plain("history", &[Arg::need("records-file", T_PATH), Arg::need("address", T_STR)]),
        Verb::plain("schema", &[]),
    ];
}

fn listing_answer(p: &k3::listing::Listing) -> Value {
    ok(vec![
        (
            "byCall".into(),
            Value::Arr(
                p.by_call
                    .iter()
                    .map(|(c, d)| Value::Obj(vec![i("candidate", *c as u64), s("digest", d.clone())]))
                    .collect(),
            ),
        ),
        i("candidates", p.candidate_count() as u64),
        s("termsHash", p.terms_hash.clone()),
    ])
}

fn settle_answer(x: &k3::settle::Settle) -> Value {
    ok(vec![i("buyerBps", x.buyer_bps), i("expiry", x.expiry), s("instance", x.instance.clone())])
}

fn run(args: &[String]) -> Result<Value, Misuse> {
    let cmd = args.first().ok_or_else(|| Misuse::verb(None))?.as_str();
    let a = &args[1..];
    match (cmd, a.len()) {
        ("schema", 0) => Ok(ok(k1::schema::value_rows(eravon_literals::schema::register(
            &eravon_literals::schema::version("erl"),
            register::ALL,
            &[],
        )))),
        ("check", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::listing::listing(&b) {
                Ok(p) => listing_answer(&p),
                Err(f) => fault(&f),
            })
        }
        ("bind", 2) => {
            let sb = read_file(&a[0])?;
            let tb = read_file(&a[1])?;
            Ok(match k3::bind::bind(&sb, &tb) {
                Err(v) => v.to_answer(),
                Ok(_) => Verdict::new("BOUND").to_answer(),
            })
        }
        ("link-check", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::link::link(&b) {
                Ok(l) => ok(vec![i("chainId", l.chain_id), s("hash", l.hash.clone()), s("tx", l.tx.clone())]),
                Err(f) => fault(&f),
            })
        }
        ("encode", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::link::link(&b) {
                Ok(_) => ok(vec![s("link", k3::transport::encode_link(&b))]),
                Err(f) => fault(&f),
            })
        }
        ("decode", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::transport::decode_link(&b) {
                Ok((raw, _)) => ok(vec![s("link", k1::hash::hex_encode(&raw))]),
                Err(f) => fault(&f),
            })
        }
        ("resolve-link", 2) => {
            let lb = read_file(&a[0])?;
            let db = read_file(&a[1])?;
            Ok(match k3::link::resolve(&lb, &db) {
                Err(v) => v.to_answer(),
                Ok(k3::link::Resolved::Listing(p)) => {
                    Value::Obj(vec![("document".into(), listing_answer(&p)), s("verdict", "LISTING".into())])
                }
                Ok(k3::link::Resolved::Settle(x)) => {
                    Value::Obj(vec![("document".into(), settle_answer(&x)), s("verdict", "SETTLE".into())])
                }
            })
        }
        ("receipt-check", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::receipt::receipt(&b) {
                Ok(r) => ok(vec![
                    i("candidate", r.candidate),
                    s("form", sig_form_name(&r.sig).into()),
                    i("openBy", r.open_by),
                    s("prev", r.prev.clone()),
                ]),
                Err(f) => fault(&f),
            })
        }
        ("receipt-encode", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::receipt::receipt(&b) {
                Ok(r) => ok(vec![s("text", k3::transport::encode_receipt(&r.bytes))]),
                Err(f) => fault(&f),
            })
        }
        ("receipt-decode", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::transport::decode_receipt(&b) {
                Ok((raw, _)) => ok(vec![s("receipt", k1::hash::hex_encode(&raw))]),
                Err(f) => fault(&f),
            })
        }
        ("bind-receipt", 2) => {
            let sb = read_file(&a[0])?;
            let rb = read_file(&a[1])?;
            Ok(match k3::receipt::bind_receipt_bytes(&sb, &rb) {
                Err(v) => v.to_answer(),
                Ok((_, _, b)) => Value::Obj(vec![
                    s("caseId", k1::hash::hex_encode(&b.salt)),
                    s("digest", k1::hash::hex_encode(&b.order_digest)),
                    s("form", b.form.name().into()),
                    s("instance", b.instance.clone()),
                    s("salt", k1::hash::hex_encode(&b.salt)),
                    s("verdict", b.verdict().into()),
                ]),
            })
        }
        ("pack-check", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::pack::pack(&b) {
                Ok(p) => ok(vec![
                    i("attachments", p.attachments.len() as u64),
                    s("buyerKey", p.receipt.buyer_key.clone()),
                    s("caseId", k1::hash::hex_encode(&p.bound.salt)),
                    s("form", p.bound.form.name().into()),
                    s("instance", p.bound.instance.clone()),
                ]),
                Err(f) => fault(&f),
            })
        }
        ("settle-check", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k3::settle::settle(&b) {
                Ok(x) => settle_answer(&x),
                Err(f) => fault(&f),
            })
        }
        ("bind-settle", 2) => {
            let kb = read_file(&a[0])?;
            let sb = read_file(&a[1])?;
            Ok(match k3::settle::bind_settle_bytes(&kb, &sb) {
                Err(v) => v.to_answer(),
                Ok(d) => Value::Obj(vec![s("digest", k1::hash::hex_encode(&d.digest)), s("verdict", d.verdict().into())]),
            })
        }
        ("place", 3) => {
            let rs = read_records(&a[0])?;
            let seller = arg_hex("seller-key", &a[1], false)?;
            let th = arg_hex("terms-hash", &a[2], true)?;
            let position = match identity::place(&rs, &seller, &th) {
                None => Value::Null,
                Some((p, src)) => identity::position_value(p, Some(src)),
            };
            Ok(ok(vec![("position".into(), position)]))
        }
        ("pay-check", 2) => {
            let sb = read_file(&a[0])?;
            let rs = read_records(&a[1])?;
            Ok(match identity::pay_check(&rs, &sb) {
                Err(v) => v.to_answer(),
                Ok(p) => Value::Obj(vec![s("verdict", p.word().into())]),
            })
        }
        ("history", 2) => {
            let rs = read_records(&a[0])?;
            let addr = arg_hex("address", &a[1], false)?;
            let h = identity::history(&rs, &addr);
            Ok(ok(identity::history_members(&rs, &h)))
        }
        _ => Err(eravon_literals::schema::positional_misuse(register::ALL, args)),
    }
}

fn sig_form_name(v: &Value) -> &'static str {
    match v {
        Value::Str(x) if k2::sig::is_empty_slot(x) => k2::sig::Form::Call.name(),
        _ => k2::sig::Form::Secp256k1.name(),
    }
}
