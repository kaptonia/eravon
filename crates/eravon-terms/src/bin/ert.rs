#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use eravon_galeed as k1;
use eravon_galeed::Value;
use eravon_terms as k2;
use k2::derive::{Action, Party};
use k2::form;
use k2::token::Fault;
use k2::u256::U256;

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
    eravon_literals::trace::entry("ert", eravon_literals::trace::Feature::K2, args.first().map(String::as_str).unwrap_or(""));
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
    m.say("ert");
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

fn fault(f: &Fault) -> Value {
    f.to_value(true)
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

fn arg_int(name: &str, a: &str) -> Result<u64, Misuse> {
    form::int_spelling(a).ok_or_else(|| Misuse::bad(name, a, eravon_literals::misuse::INT_SPELLING))
}

fn arg_dec256(a: &str) -> Result<U256, Misuse> {
    U256::from_dec(a).ok_or_else(|| Misuse::bad("amount", a, eravon_literals::misuse::said("要是十进制、无前导零、不过 2^256 − 1", "a decimal without leading zeros, at most 2^256 - 1")))
}

fn arg_hex20(name: &str, a: &str) -> Result<String, Misuse> {
    if k1::hexform::is_hex20(a) { Ok(a.to_string()) } else { Err(Misuse::bad(name, a, eravon_literals::misuse::HEX20)) }
}

fn arg_privkey(a: &str) -> Result<[u8; 32], Misuse> {
    let bad = || Misuse::key(a);
    let body = a.strip_prefix("0x").or_else(|| a.strip_prefix("0X")).unwrap_or(a);
    if body.len() != 64 {
        return Err(bad());
    }
    let bytes = k1::hexform::decode_digits(body.as_bytes()).ok_or_else(bad)?;
    let mut w = [0u8; 32];
    w.copy_from_slice(&bytes);
    if !k1::sign::scalar_in_range(&w) {
        return Err(bad());
    }
    Ok(w)
}

fn arg_transport_hex(name: &str, a: &str) -> Result<Vec<u8>, Misuse> {
    let bad = || Misuse::bad(name, a, eravon_literals::misuse::HEX_EVEN);
    let body = a.strip_prefix("0x").ok_or_else(bad)?;
    if body.len() % 2 != 0 {
        return Err(bad());
    }
    k1::hexform::decode_digits(body.as_bytes()).ok_or_else(bad)
}

fn arg_party(a: &str) -> Result<Party, Misuse> {
    match a {
        "buyer" => Ok(Party::Buyer),
        "seller" => Ok(Party::Seller),
        _ => Err(Misuse::bad("party", a, eravon_literals::misuse::SEAT)),
    }
}

fn read_file(path: &str) -> Result<Vec<u8>, Misuse> {
    let meta = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    if !meta.is_file() || meta.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
    }
    std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))
}

fn read_sig_json(path: &str) -> Result<Value, Misuse> {
    let b = read_file(path)?;
    k1::canon::read_open(&b).ok_or_else(|| Misuse::unreadable(path, eravon_literals::misuse::NOT_JSON))
}

fn arg_action(rest: &[String]) -> Result<(Action, usize), Misuse> {
    let name = rest.first().ok_or_else(|| Misuse::missing("action-name"))?.as_str();
    let need = match name {
        "ClaimDelivery" => 1,
        "Confirm" | "Dispute" | "Refund" | "Cancel" => 0,
        "Settle" => 2,
        _ => return Err(Misuse::bad("action-name", name, eravon_literals::misuse::said("要是 ClaimDelivery、Confirm、Dispute、Refund、Settle、Cancel 之一", "one of ClaimDelivery, Confirm, Dispute, Refund, Settle, Cancel"))),
    };
    if rest.len() < 1 + need {
        return Err(Misuse::bad("action-name", name, eravon_literals::misuse::said("这个动作后面的参数数目不对", "this action is followed by the wrong number of arguments")));
    }
    let a = match name {
        "ClaimDelivery" => Action::ClaimDelivery { manifest: rest[1].clone() },
        "Confirm" => Action::Confirm,
        "Refund" => Action::Refund,
        "Dispute" => Action::Dispute,
        "Cancel" => Action::Cancel,
        "Settle" => Action::Settle { buyer_bps: arg_int("buyerBps", &rest[1])?, expiry: U256::from_u64(arg_int("expiry", &rest[2])?) },
        _ => unreachable!("上面的表已闭"),
    };
    Ok((a, 1 + need))
}

struct Derived {
    terms: k2::doc::Terms,
    offer: k2::derive::Offer,
}

fn derive_offer(path: &str, c: &str) -> Result<Result<Derived, Fault>, Misuse> {
    let b = read_file(path)?;
    let c = arg_int("candidate", c)?;
    Ok((|| {
        let terms = k2::doc::terms(&b)?;
        let offer = k2::derive::offer(&terms, c)?;
        Ok(Derived { terms, offer })
    })())
}

fn offer_value(o: &k2::derive::Offer) -> Value {
    let ladder = o
        .ladder
        .iter()
        .map(|r| {
            Value::Obj(vec![
                i("buyerBps", r.buyer_bps),
                i("feeBearer", r.fee_bearer),
                i("sellerBps", r.seller_bps),
            ])
        })
        .collect();
    Value::Obj(vec![
        s("amanahHash", k1::hash::hex_encode(&o.amanah_hash)),
        ("amount".into(), Value::Str(o.amount.to_dec())),
        ("buyerMayOpen".into(), Value::Bool(o.buyer_may_open)),
        i("confirmWindow", o.confirm_window),
        i("deliveryWindow", o.delivery_window),
        i("expiry", o.expiry),
        s("guild", o.guild.clone()),
        s("judge", o.judge.clone()),
        ("ladder".into(), Value::Arr(ladder)),
        i("nonce", o.nonce),
        i("openWindow", o.open_window),
        s("quoteHash", k1::hash::hex_encode(&o.quote_hash)),
        s("sellerFund", o.seller_fund.clone()),
        s("sellerKey", o.seller_key.clone()),
        s("sellerPayout", o.seller_payout.clone()),
        s("termsHash", k1::hash::hex_encode(&o.terms_hash)),
        s("token", o.token.clone()),
    ])
}

fn quote_value(q: &k2::quote::Quote) -> Value {
    Value::Obj(vec![
        ("amountCap".into(), Value::Str(q.amount_cap.to_dec())),
        ("autoAccept".into(), Value::Bool(q.auto_accept)),
        i("feeBps", q.fee_bps),
        ("feeFixed".into(), Value::Str(q.fee_fixed.to_dec())),
        ("feeMax".into(), Value::Str(q.fee_max.to_dec())),
        ("feeMin".into(), Value::Str(q.fee_min.to_dec())),
        s("feeUnit", q.fee_unit.clone()),
        s("feedToken", q.feed_token.clone()),
        s("feedUnit", q.feed_unit.clone()),
        i("lapseDefault", q.lapse_default),
        i("maxStale", q.max_stale),
        i("offerWindow", q.offer_window),
        i("premiumBps", q.premium_bps),
        i("rulingWindow", q.ruling_window),
        s("token", q.token.clone()),
    ])
}

fn derive_order(
    d: &Derived,
    buyer_key: &str,
    buyer_payout: &str,
    open_by: u64,
    prev: &str,
) -> Result<k2::derive::Order, Fault> {
    k2::derive::order(&d.offer, d.terms.chain_id, &d.terms.logic, buyer_key, buyer_payout, open_by, prev)
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    use eravon_literals::schema::{T_INT, T_PATH, T_STR};

    const C_TERMS: &str = "eravon.terms/1 · 条款判定闭集(eravon_terms::token::Fault)";
    const C_DERIVE: &str = "eravon.terms/1 · 要约与订单派生闭集(E_OFFER_* · E_ORDER_*)";
    const C_ACTION: &str = "eravon.terms/1 · 动作闸闭集(E_ACTION_*)";
    const C_SIG: &str = "eravon.terms/1 · 验签闭集(E_SIG_*)";
    const C_CANON: &str = "galeed/1 §10 · 正典形与条目信封闭集";

    const TERMS: Arg = Arg::need("terms-file", T_PATH);
    const CANDIDATE: Arg = Arg::need("candidate", T_INT);
    const BUYER_KEY: Arg = Arg::need("buyer-key", T_STR);
    const BUYER_PAYOUT: Arg = Arg::need("buyer-payout", T_STR);
    const OPEN_BY: Arg = Arg::need("open-by", T_INT);
    const PREV: Arg = Arg::need("prev", T_STR);
    const PRIVKEY: Arg = Arg::need("privkey", T_STR);
    const ACTION_NAME: Arg = Arg::need("action-name", T_STR);
    const ACTION_ARGS: Arg = Arg::opt("action-args", eravon_literals::schema::T_ARR);

    pub const ALL: &[Verb] = &[
        Verb::carry("check", &[TERMS], &[C_TERMS, C_CANON]),
        Verb::new("quote", &[Arg::need("quote-hex", T_STR)], &["E_QUOTE_LEN", "E_QUOTE_FIELD", "E_QUOTE_FEED"], &[]),
        Verb::carry("offer", &[TERMS, CANDIDATE], &[C_TERMS, C_CANON, C_DERIVE]),
        Verb::carry("order", &[TERMS, CANDIDATE, BUYER_KEY, BUYER_PAYOUT, OPEN_BY, PREV], &[C_TERMS, C_CANON, C_DERIVE]),
        Verb::carry(
            "action",
            &[TERMS, CANDIDATE, BUYER_KEY, BUYER_PAYOUT, OPEN_BY, PREV, ACTION_NAME, ACTION_ARGS],
            &[C_TERMS, C_CANON, C_DERIVE, C_ACTION],
        ),
        Verb::carry("sign-offer", &[PRIVKEY, TERMS, CANDIDATE], &[C_TERMS, C_CANON, C_DERIVE]),
        Verb::carry("sign-order", &[PRIVKEY, TERMS, CANDIDATE, BUYER_PAYOUT, OPEN_BY, PREV], &[C_TERMS, C_CANON, C_DERIVE]),
        Verb::carry(
            "sign-action",
            &[PRIVKEY, TERMS, CANDIDATE, BUYER_KEY, BUYER_PAYOUT, OPEN_BY, PREV, ACTION_NAME, ACTION_ARGS],
            &[C_TERMS, C_CANON, C_DERIVE, C_ACTION],
        ),
        Verb::carry("verify-offer", &[TERMS, CANDIDATE, Arg::need("seller-sig", T_STR)], &[C_TERMS, C_CANON, C_DERIVE, C_SIG]),
        Verb::carry(
            "verify-order",
            &[TERMS, CANDIDATE, BUYER_KEY, BUYER_PAYOUT, OPEN_BY, PREV, Arg::need("sig-file", T_PATH)],
            &[C_TERMS, C_CANON, C_DERIVE, C_SIG],
        ),
        Verb::new(
            "verify-action",
            &[
                TERMS,
                CANDIDATE,
                BUYER_KEY,
                BUYER_PAYOUT,
                OPEN_BY,
                PREV,
                ACTION_NAME,
                ACTION_ARGS,
                Arg::need("party", T_STR),
                Arg::need("sig-file", T_PATH),
            ],
            &["E_ACTION_PARTY", "E_SIG_FORM"],
            &[C_TERMS, C_CANON, C_DERIVE, C_ACTION, C_SIG],
        ),
        Verb::new(
            "fee",
            &[Arg::need("quote-hex", T_STR), Arg::need("amount", T_STR)],
            &["E_QUOTE_LEN", "E_QUOTE_FIELD", "E_QUOTE_FEED", "E_FEE_OVERFLOW"],
            &[],
        ),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Value, Misuse> {
    let cmd = args.first().ok_or_else(|| Misuse::verb(None))?.as_str();
    let a = &args[1..];
    match (cmd, a.len()) {
        ("schema", 0) => Ok(ok(k1::schema::value_rows(eravon_literals::schema::register(
            &eravon_literals::schema::version("ert"),
            register::ALL,
            &[],
        )))),
        ("check", 1) => {
            let b = read_file(&a[0])?;
            Ok(match k2::doc::terms(&b) {
                Ok(t) => ok(vec![s("termsHash", k1::hash::hex_encode(&t.terms_hash))]),
                Err(f) => fault(&f),
            })
        }
        ("quote", 1) => {
            let bytes = arg_transport_hex("quote-hex", &a[0])?;
            Ok(match k2::quote::decode(&bytes) {
                Ok(q) => ok(vec![
                    ("quote".into(), quote_value(&q)),
                    s("quoteHash", k1::hash::hex_encode(&q.hash())),
                ]),
                Err(f) => fault(&f),
            })
        }
        ("offer", 2) => Ok(match derive_offer(&a[0], &a[1])? {
            Err(f) => fault(&f),
            Ok(d) => {
                let sh = d.offer.struct_hash();
                ok(vec![
                    s("digest", k1::hash::hex_encode(&d.offer.offer_digest(d.terms.chain_id, &d.terms.logic))),
                    ("offer".into(), offer_value(&d.offer)),
                    s("structHash", k1::hash::hex_encode(&sh)),
                ])
            }
        }),
        ("order", 6) => {
            let (bk, bp, ob, pv) = order_args(&a[2..6])?;
            Ok(match derive_offer(&a[0], &a[1])? {
                Err(f) => fault(&f),
                Ok(d) => match derive_order(&d, &bk, &bp, ob, &pv) {
                    Err(f) => fault(&f),
                    Ok(o) => ok(vec![
                        s("caseId", k1::hash::hex_encode(&o.salt)),
                        s("digest", k1::hash::hex_encode(&o.order_digest)),
                        s("instance", o.instance.clone()),
                        s("offerDigest", k1::hash::hex_encode(&d.offer.offer_digest(d.terms.chain_id, &d.terms.logic))),
                        s("salt", k1::hash::hex_encode(&o.salt)),
                    ]),
                },
            })
        }
        ("action", n) if n >= 7 => {
            let (bk, bp, ob, pv) = order_args(&a[2..6])?;
            let (act, eaten) = arg_action(&a[6..])?;
            if 6 + eaten != n {
                return Err(Misuse::args(eravon_literals::misuse::POSITIONAL));
            }
            Ok(match derive_action(&a[0], &a[1], &bk, &bp, ob, &pv, &act)? {
                Err(f) => fault(&f),
                Ok((d, o, sh)) => ok(vec![
                    s("digest", k1::hash::hex_encode(&k2::derive::digest(d.terms.chain_id, &o.instance, &sh))),
                    s("instance", o.instance.clone()),
                    s("structHash", k1::hash::hex_encode(&sh)),
                ]),
            })
        }
        ("sign-offer", 3) => {
            let key = arg_privkey(&a[0])?;
            Ok(match derive_offer(&a[1], &a[2])? {
                Err(f) => fault(&f),
                Ok(d) => {
                    let dg = d.offer.offer_digest(d.terms.chain_id, &d.terms.logic);
                    let (sig, signer) = k2::sig::sign_secp256k1(&key, &dg).ok_or_else(|| Misuse::key_cannot())?;
                    ok(vec![s("digest", k1::hash::hex_encode(&dg)), s("sig", sig), s("signer", signer)])
                }
            })
        }
        ("sign-order", 6) => {
            let key = arg_privkey(&a[0])?;
            let bp = arg_hex20("buyer-payout", &a[3])?;
            let ob = arg_int("openBy", &a[4])?;
            let pv = arg_hex20("prev", &a[5])?;
            let addr = address_of_secp(&key).ok_or_else(|| Misuse::key_cannot())?;
            Ok(match derive_offer(&a[1], &a[2])? {
                Err(f) => fault(&f),
                Ok(d) => match derive_order(&d, &addr, &bp, ob, &pv) {
                    Err(f) => fault(&f),
                    Ok(o) => {
                        let (sig, _) = k2::sig::sign_secp256k1(&key, &o.order_digest).ok_or_else(|| Misuse::key_cannot())?;
                        ok(vec![
                            s("buyerKey", addr.clone()),
                            s("digest", k1::hash::hex_encode(&o.order_digest)),
                            s("instance", o.instance.clone()),
                            s("sig", sig),
                        ])
                    }
                },
            })
        }
        ("sign-action", n) if n >= 8 => {
            let key = arg_privkey(&a[0])?;
            let (bk, bp, ob, pv) = order_args(&a[3..7])?;
            let (act, eaten) = arg_action(&a[7..])?;
            if 7 + eaten != n {
                return Err(Misuse::args(eravon_literals::misuse::POSITIONAL));
            }
            Ok(match derive_action(&a[1], &a[2], &bk, &bp, ob, &pv, &act)? {
                Err(f) => fault(&f),
                Ok((d, o, sh)) => {
                    let dg = k2::derive::digest(d.terms.chain_id, &o.instance, &sh);
                    let (sig, signer) = k2::sig::sign_secp256k1(&key, &dg).ok_or_else(|| Misuse::key_cannot())?;
                    ok(vec![
                        s("digest", k1::hash::hex_encode(&dg)),
                        s("instance", o.instance.clone()),
                        s("sig", sig),
                        s("signer", signer),
                    ])
                }
            })
        }
        ("verify-offer", 3) => Ok(match derive_offer(&a[0], &a[1])? {
            Err(f) => fault(&f),
            Ok(d) => {
                let dg = d.offer.offer_digest(d.terms.chain_id, &d.terms.logic);
                match k2::sig::verify_seller(&dg, &a[2], &d.terms.seller_key) {
                    Ok((form, signer)) => {
                        let mut m = vec![s("form", form.name().into())];
                        m.extend(signer.map(|x| s("signer", x)));
                        ok(m)
                    }
                    Err(f) => fault(&f),
                }
            }
        }),
        ("verify-order", 7) => {
            let (bk, bp, ob, pv) = order_args(&a[2..6])?;
            let sigv = read_sig_json(&a[6])?;
            Ok(match derive_offer(&a[0], &a[1])? {
                Err(f) => fault(&f),
                Ok(d) => match derive_order(&d, &bk, &bp, ob, &pv) {
                    Err(f) => fault(&f),
                    Ok(o) => match k2::sig::verify_buyer(&o.order_digest, &sigv, &bk) {
                        Ok(form) => ok(vec![s("form", form.name().into()), s("instance", o.instance.clone())]),
                        Err(f) => fault(&f),
                    },
                },
            })
        }
        ("verify-action", n) if n >= 9 => {
            let (bk, bp, ob, pv) = order_args(&a[2..6])?;
            let (act, eaten) = arg_action(&a[6..])?;
            if 6 + eaten + 2 != n {
                return Err(Misuse::args(eravon_literals::misuse::POSITIONAL));
            }
            let party = arg_party(&a[6 + eaten])?;
            let sigv = read_sig_json(&a[6 + eaten + 1])?;
            Ok(match derive_action(&a[0], &a[1], &bk, &bp, ob, &pv, &act)? {
                Err(f) => fault(&f),
                Ok((d, o, sh)) => {
                    if !act.admits(party) {
                        return Ok(fault(&Fault::new(k2::token::Token::ActionParty)));
                    }
                    let dg = k2::derive::digest(d.terms.chain_id, &o.instance, &sh);
                    match k2::sig::verify_action(&act, party, &dg, &sigv, &d.terms.seller_key, &bk) {
                        Ok(form) => ok(vec![s("form", form.name().into())]),
                        Err(f) => fault(&f),
                    }
                }
            })
        }
        ("fee", 2) => {
            let bytes = arg_transport_hex("quote-hex", &a[0])?;
            let amount = arg_dec256(&a[1])?;
            Ok(match k2::quote::decode(&bytes).and_then(|q| k2::fee::fee(&q, &amount)) {
                Err(f) => fault(&f),
                Ok(f) => ok(vec![
                    ("lf".into(), match f.lf {
                        Some(x) => Value::Str(x.to_dec()),
                        None => Value::Null,
                    }),
                    ("premium".into(), Value::Str(f.premium.to_dec())),
                    ("priced".into(), Value::Bool(f.lf.is_some())),
                ]),
            })
        }
        _ => Err(eravon_literals::schema::positional_misuse(register::ALL, args)),
    }
}

fn order_args(a: &[String]) -> Result<(String, String, u64, String), Misuse> {
    Ok((arg_hex20("buyer-key", &a[0])?, arg_hex20("buyer-payout", &a[1])?, arg_int("openBy", &a[2])?, arg_hex20("prev", &a[3])?))
}

fn derive_action(
    path: &str,
    c: &str,
    bk: &str,
    bp: &str,
    ob: u64,
    pv: &str,
    act: &Action,
) -> Result<Result<(Derived, k2::derive::Order, [u8; 32]), Fault>, Misuse> {
    Ok(match derive_offer(path, c)? {
        Err(f) => Err(f),
        Ok(d) => derive_order(&d, bk, bp, ob, pv).and_then(|o| {
            k2::derive::check_action(act)?;
            let sh = act.struct_hash();
            Ok((d, o, sh))
        }),
    })
}

fn address_of_secp(key: &[u8; 32]) -> Option<String> {
    k1::sign::Privkey::from_bytes(key).map(|k| k1::hash::hex_encode(&k.address()))
}
