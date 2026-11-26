#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;
use eravon_literals::constants as k7;
use eravon_literals::json::{Out, write};
use eravon_standard as a1;
use a1::call::{Action, BySig};
use a1::fault::Fault;
use a1::order::{self, Order};
use eravon_terms as k2;

#[path = "era/build.rs"]
mod build;
use build::*;
#[path = "era/read.rs"]
mod read;
use read::*;
#[path = "era/company.rs"]
mod company;
use company::*;
#[path = "era/precheck.rs"]
mod precheck;
use precheck::*;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;
const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("era", eravon_standard::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
    match eravon_chain::task::entry(|| run(&args)) {
        Ok(v) => {
            print(&v);
            ExitCode::from(eravon_literals::schema::exit_of(&v))
        }
        Err(m) => misuse(&m),
    }
}

fn argv() -> Option<Vec<String>> {
    std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect()
}

fn print(v: &Out) {
    write_out(&write(v));
}


fn misuse(m: &Misuse) -> ExitCode {
    m.say("era");
    ExitCode::from(EXIT_MISUSE)
}


fn write_out(bytes: &[u8]) {
    use std::io::Write;
    let out = std::io::stdout();
    let mut h = out.lock();
    if h.write_all(&eravon_literals::echo::scrub(bytes)).is_err() || h.flush().is_err() {
        std::process::exit(EXIT_MISUSE as i32);
    }
}

struct Flags {
    pairs: Vec<(String, String)>,
}

impl Flags {
    fn one(&self, k: &str) -> Option<&str> {
        self.pairs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    fn many(&self, k: &str) -> Vec<&str> {
        self.pairs.iter().filter(|(a, _)| a == k).map(|(_, v)| v.as_str()).collect()
    }
}

fn flags(args: &[String]) -> Result<Flags, Misuse> {
    Ok(Flags { pairs: eravon_literals::schema::read_pairs(args.get(1..).unwrap_or_default())? })
}

fn need(f: &Flags, k: &str) -> Result<String, Misuse> {
    f.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k))
}

fn num(f: &Flags, k: &str) -> Result<u64, Misuse> {
    let v = need(f, k)?;
    v.parse().map_err(|_| Misuse::not_int(k, &v))
}

fn read_bounded(path: &str) -> Result<Vec<u8>, Misuse> {
    let m = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    if !m.is_file() || m.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
    }
    std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))
}

fn ok(mut m: Vec<(&'static str, Out)>) -> Out {
    m.push(("ok", Out::B(true)));
    Out::O(m)
}

fn refused(f: &Fault) -> Out {
    let mut m = vec![
        ("detail", Out::S(f.detail())),
        ("ok", Out::B(false)),
        ("token", Out::S(f.name())),
    ];
    if let Some(g) = f.precheck_gate() {
        m.push(("gate", Out::S(g.to_string())));
    }
    Out::O(m)
}

fn ans(r: a1::fault::R<Out>) -> Result<Out, Misuse> {
    Ok(match r {
        Ok(v) => v,
        Err(f) => refused(&f),
    })
}

fn tx(f: &Flags) -> Result<Out, Misuse> {
    let value = match (f.one("--value"), f.one("--value-text")) {
        (Some(v), _) => v.to_string(),
        (None, Some(t)) => match eravon_literals::units::units_of(t, eravon_literals::constants::NATIVE_DECIMALS) {
            Ok(v) => v,
            Err(n) => return ans(Err(a1::fault::Fault::Law { token: n.into(), detail: format!("--value-text {t:?} 折不成 wei") })),
        },
        (None, None) => "0".to_string(),
    };
    let (priority, cap, value) = match (word("--priority-fee", &need(f, "--priority-fee")?)?, word("--max-fee", &need(f, "--max-fee")?)?, word("--value", &value)?) {
        (Ok(p), Ok(c), Ok(v)) => (p, c, v),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return ans(Err(e)),
    };
    let t = a1::tx::Tx {
        chain_id: num(f, "--chain")?,
        nonce: num(f, "--nonce")?,
        max_priority_fee: priority,
        max_fee: cap,
        gas_limit: num(f, "--gas-limit")?,
        to: Some(a1::order::addr_of("--to", &need(f, "--to")?).map_err(|_| Misuse::bad("--to", f.one("--to").unwrap_or_default(), eravon_literals::misuse::HEX20))?),
        value,
        data: a1::order::bytes_of("--data", f.one("--data").unwrap_or("0x")).map_err(|_| Misuse::bad("--data", f.one("--data").unwrap_or_default(), eravon_literals::misuse::HEX_EVEN))?,
    };
    let mut m = vec![
        ("digest", Out::S(a1::order::hex_of(&t.digest()))),
        ("payload", Out::S(a1::order::hex_of(&t.signing_payload()))),
        ("type", Out::N(u64::from(a1::tx::TX_TYPE_1559))),
    ];
    if let Some(sig) = f.one("--sig") {
        let raw = match t.raw(&a1::order::bytes_of("--sig", sig).map_err(|_| Misuse::bad("--sig", sig, eravon_literals::misuse::HEX_EVEN))?) {
            Ok(r) => r,
            Err(e) => return ans(Err(e)),
        };
        let h = a1::tx::Tx::hash(&t, &a1::order::bytes_of("--sig", sig).map_err(|_| Misuse::bad("--sig", sig, eravon_literals::misuse::HEX_EVEN))?)
            .map_err(|_| Misuse::bad("--sig", sig, eravon_literals::misuse::said("不是一枚 65 字节的签名", "not a 65-byte signature")))?;
        m.push(("raw", Out::S(a1::order::hex_of(&raw))));
        m.push(("txHash", Out::S(a1::order::hex_of(&h))));
    }
    Ok(ok(m))
}

fn call_out(c: &a1::call::Call) -> Out {
    Out::O(vec![("data", Out::S(c.hex())), ("to", Out::S(c.to.clone()))])
}

fn word(name: &str, s: &str) -> Result<Result<[u8; 32], Fault>, Misuse> {
    let wide = || Fault::Form(format!("{name} {s:?} 超出 256 位"));
    match s.strip_prefix("0x") {
        Some(h) => {
            let mut w = [0u8; 32];
            let b = a1::order::bytes_of(name, &format!("0x{h}")).map_err(|_| Misuse::bad(name, s, eravon_literals::misuse::INT_OR_HEX))?;
            if b.len() > 32 {
                return Ok(Err(wide()));
            }
            w[32 - b.len()..].copy_from_slice(&b);
            Ok(Ok(w))
        }
        None => {
            if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
                return Err(Misuse::bad(name, s, eravon_literals::misuse::INT_OR_HEX));
            }
            let digits = s.trim_start_matches('0');
            match k2::u256::U256::from_dec(if digits.is_empty() { "0" } else { digits }) {
                Some(n) => Ok(Ok(n.to_be())),
                None => Ok(Err(wide())),
            }
        }
    }
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_LAW: &str = "eravon.terms/1 · 订单派生与报价解码闭集(E_ORDER_* · E_QUOTE_*)";
    const C_CHAIN: &str = "eravon.chain/1 · 端点与三分法闭集(E_CHAIN_USAGE · E_CHAIN_TRANSPORT · E_CHAIN_ENDPOINT · E_CHAIN_EXECUTION · E_CHAIN_DISAGREE · E_CHAIN_WRONG)";
    pub const ALL: &[Verb] = &[
        Verb::plain("sol", &[]),
        Verb::new("address", &[Arg::need("--order", T_PATH), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR)], &["E_A1_FORM"], &[C_LAW]),
        Verb::new("deploy-logic", &[Arg::need("--initcode", T_PATH)], &["E_A1_FORM"], &[]),
        Verb::new("deploy-verifier", &[Arg::need("--initcode", T_PATH)], &["E_A1_FORM"], &[]),
        Verb::new("deploy-board", &[Arg::need("--initcode", T_PATH)], &["E_A1_FORM"], &[]),
        Verb::new("deploy-instance", &[Arg::need("--order", T_PATH), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR)], &["E_A1_FORM"], &[C_LAW]),
        Verb::new("init", &[Arg::need("--order", T_PATH), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::opt("--quote", T_STR), Arg::opt("--sig-buyer", T_STR), Arg::opt("--sig-seller", T_STR), Arg::opt("--member", T_ARR), Arg::opt("--amanah", T_ARR)], &["E_A1_FORM", "E_A1_ABI"], &[C_LAW]),
        Verb::new("expire", &[Arg::need("--now", T_INT), Arg::need("--order", T_PATH), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR)], &["E_A1_FORM", "E_A1_PRECHECK"], &[C_LAW]),
        Verb::new("merged", &[Arg::opt("--order", T_PATH), Arg::opt("--terms", T_PATH), Arg::opt("--candidate", T_INT), Arg::opt("--buyer-key", T_STR), Arg::opt("--buyer-payout", T_STR), Arg::opt("--open-by", T_INT), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::opt("--quote", T_STR), Arg::opt("--sig-buyer", T_STR), Arg::opt("--sig-seller", T_STR), Arg::opt("--member", T_ARR), Arg::opt("--amanah", T_ARR), Arg::opt("--verifier-initcode", T_PATH), Arg::opt("--board-initcode", T_PATH)], &["E_A1_FORM", "E_A1_ABI", "E_A1_PRECHECK"], &[C_LAW]),
        Verb::new("cancel", &[Arg::opt("--order", T_PATH), Arg::opt("--terms", T_PATH), Arg::opt("--candidate", T_INT), Arg::opt("--buyer-key", T_STR), Arg::opt("--buyer-payout", T_STR), Arg::opt("--open-by", T_INT), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::opt("--sig-buyer", T_STR)], &["E_A1_FORM", "E_A1_ABI"], &[C_LAW]),
        Verb::new("revoke", &[Arg::need("--seller", T_STR), Arg::need("--terms-hash", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR)], &["E_A1_FORM"], &[]),
        Verb::new("accept-funding", &[Arg::need("--seller", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR)], &["E_A1_FORM"], &[]),
        Verb::new("post", &[Arg::need("--key", T_STR), Arg::need("--document", T_PATH), Arg::opt("--board", T_STR)], &["E_A1_FORM"], &[]),
        Verb::new("action", &[Arg::need("--instance", T_STR), Arg::need("--do", T_STR), Arg::opt("--manifest", T_PATH), Arg::opt("--buyer-bps", T_INT), Arg::opt("--settle-expiry", T_INT), Arg::opt("--sig-buyer", T_STR), Arg::opt("--sig-seller", T_STR), Arg::opt("--case-id", T_STR), Arg::opt("--seat", T_INT), Arg::opt("--member", T_ARR), Arg::opt("--amanah", T_ARR), Arg::opt("--who", T_STR), Arg::opt("--token", T_STR), Arg::opt("--window-end", T_INT), Arg::opt("--now", T_INT)], &["E_A1_FORM", "E_A1_PRECHECK"], &[]),
        Verb::new("bysig", &[Arg::need("--instance", T_STR), Arg::need("--do", T_STR), Arg::need("--sig", T_STR), Arg::opt("--manifest", T_PATH)], &["E_A1_FORM"], &[]),
        Verb::new("read", &[Arg::need("--endpoint", T_ARR), Arg::need("--what", T_STR), Arg::opt("--feed", T_STR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::opt("--instance", T_STR), Arg::opt("--case-id", T_STR), Arg::opt("--seat", T_INT), Arg::opt("--who", T_STR), Arg::opt("--token", T_STR), Arg::opt("--order", T_PATH), Arg::opt("--guild", T_STR), Arg::opt("--book", T_STR), Arg::opt("--key", T_STR), Arg::opt("--digest", T_STR), Arg::opt("--buyer-bps", T_INT), Arg::opt("--settle-expiry", T_INT), Arg::opt("--fund", T_STR), Arg::opt("--quote", T_STR)], &["E_A1_FORM", "E_A1_ABI", "E_A1_CODEHASH"], &[C_LAW, C_CHAIN]),
        Verb::new("precheck", &[Arg::need("--what", T_STR), Arg::opt("--seat-state", T_INT), Arg::opt("--feed", T_STR), Arg::opt("--book", T_STR), Arg::opt("--guild", T_STR), Arg::opt("--order", T_PATH), Arg::opt("--sig-seller", T_STR), Arg::opt("--sig-buyer", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::opt("--window-end", T_INT), Arg::opt("--now", T_INT), Arg::opt("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--need", T_INT), Arg::opt("--need-text", T_STR), Arg::opt("--units-text", T_BOOL), Arg::opt("--token", T_STR), Arg::opt("--owner", T_STR), Arg::opt("--instance", T_STR), Arg::opt("--max-stale", T_INT), Arg::opt("--sender", T_STR), Arg::opt("--via", T_STR), Arg::opt("--accepted", T_INT), Arg::opt("--phase", T_INT), Arg::opt("--quote", T_STR), Arg::opt("--to", T_STR), Arg::opt("--data", T_STR)], &["E_A1_FORM", "E_A1_PRECHECK"], &[C_LAW, C_CHAIN]),
        Verb::new("digest", &[Arg::opt("--instance", T_STR), Arg::need("--do", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--manifest", T_PATH), Arg::opt("--buyer-bps", T_INT), Arg::opt("--settle-expiry", T_INT), Arg::opt("--terms", T_PATH), Arg::opt("--candidate", T_INT), Arg::opt("--expiry", T_INT)], &["E_A1_FORM"], &[C_LAW]),
        Verb::new("decode-log", &[Arg::need("--data", T_STR), Arg::opt("--topic", T_ARR)], &["E_A1_FORM", "E_A1_ABI"], &[]),
        Verb::new("decode-init", &[Arg::need("--data", T_STR)], &["E_A1_FORM", "E_A1_ABI"], &[]),
        Verb::new("tx", &[Arg::need("--chain", T_INT), Arg::need("--nonce", T_INT), Arg::need("--priority-fee", T_STR), Arg::need("--max-fee", T_STR), Arg::need("--gas-limit", T_INT), Arg::need("--to", T_STR), Arg::opt("--value", T_STR), Arg::opt("--value-text", T_STR), Arg::opt("--data", T_STR), Arg::opt("--sig", T_STR)], &["E_A1_FORM"], &[]),
        Verb::new("approve", &[Arg::need("--token", T_STR), Arg::need("--spender", T_STR), Arg::opt("--amount", T_STR), Arg::opt("--amount-text", T_STR), Arg::opt("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR)], &["E_A1_FORM"], &[]),
        Verb::new("permit", &[Arg::need("--token", T_STR), Arg::need("--owner", T_STR), Arg::need("--spender", T_STR), Arg::opt("--amount", T_STR), Arg::opt("--amount-text", T_STR), Arg::opt("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::need("--deadline", T_INT), Arg::need("--domain", T_STR), Arg::need("--nonce", T_STR), Arg::opt("--sig", T_STR)], &["E_A1_FORM"], &[]),
        Verb::new("company", &[Arg::need("--endpoint", T_ARR), Arg::need("--company", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR)], &["E_A1_FORM", "E_A1_ABI"], &[C_CHAIN]),
        Verb::new("time-check", &[Arg::need("--endpoint", T_ARR), Arg::need("--company", T_STR), Arg::need("--now", T_INT), Arg::opt("--chain", T_INT), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--t1", T_INT), Arg::opt("--resign-lead", T_INT), Arg::opt("--t2", T_INT), Arg::opt("--t3", T_ARR), Arg::opt("--offer-expiry", T_INT), Arg::opt("--settle-expiry", T_INT), Arg::opt("--evidence-t3", T_INT)], &["E_A1_FORM", "E_A1_ABI"], &[C_CHAIN]),
        Verb::new("proposal-read", &[Arg::need("--file", T_PATH)], &["E_A1_FORM"], &[]),
        Verb::new("proposal-pay", &[Arg::need("--wallet", T_STR), Arg::need("--token", T_STR), Arg::need("--to", T_STR), Arg::opt("--amount", T_STR), Arg::opt("--amount-text", T_STR), Arg::opt("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR)], &["E_A1_FORM", "E_A1_ABI"], &[]),
        Verb::new("proposal-consent", &[Arg::need("--logic", T_STR), Arg::need("--party", T_STR), Arg::need("--digest", T_STR), Arg::need("--original", T_PATH), Arg::need("--kind", T_STR), Arg::opt("--name", T_STR)], &["E_A1_FORM", "E_A1_ABI"], &[]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args.first().map(String::as_str).ok_or_else(|| Misuse::verb(None))?;
    let f = flags(args)?;
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    match verb {
        "sol" => sol_table(),
        "address" => address(&f),
        "deploy-logic" => deploy_logic(&f),
        "deploy-verifier" => deploy_verifier(&f),
        "deploy-board" => deploy_board(&f),
        "cancel" => cancel(&f),
        "revoke" => revoke(&f),
        "accept-funding" => accept_funding(&f),
        "post" => post(&f),
        "deploy-instance" => deploy_instance(&f),
        "init" => init(&f),
        "expire" => expire(&f),
        "merged" => merged(&f),
        "action" => action(&f),
        "bysig" => bysig(&f),
        "read" => read(&f),
        "precheck" => precheck(&f),
        "digest" => digest(&f),
        "decode-log" => decode_log(&f),
        "decode-init" => decode_init(&f),
        "tx" => tx(&f),
        "approve" => approve(&f),
        "permit" => permit(&f),
        "company" => company(&f),
        "time-check" => time_check(&f),
        "proposal-read" => proposal_read(&f),
        "proposal-pay" => proposal_pay(&f),
        "proposal-consent" => proposal_consent(&f),
        "schema" => Ok(ok(eravon_literals::schema::out_rows(eravon_literals::schema::register(
            &eravon_literals::schema::version("era"),
            register::ALL,
            &[],
        )))),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn sol_table() -> Result<Out, Misuse> {
    let fun = |f: &a1::sol::Fun| {
        Out::O(vec![
            ("name", Out::S(f.name.to_string())),
            ("selector", Out::S(order::hex_of(&f.selector()))),
            ("sig", Out::S(f.sig.to_string())),
        ])
    };
    let ev = |e: &a1::sol::Ev| {
        Out::O(vec![
            (
                "indexed",
                Out::A(e.indexed.iter().map(|x| Out::B(*x)).collect()),
            ),
            ("name", Out::S(e.name.to_string())),
            ("sig", Out::S(e.sig.to_string())),
            ("topic0", Out::S(order::hex_of(&e.topic0()))),
        ])
    };
    Ok(ok(vec![
        ("actions", Out::A(a1::sol::ACTIONS.iter().map(fun).collect())),
        ("aggregate3", fun(&a1::sol::AGGREGATE3)),
        ("bookEvents", Out::A(vec![ev(&a1::sol::BOOK_ENGAGED)])),
        ("events", Out::A(a1::sol::all_events().iter().map(ev).collect())),
        ("init", fun(&a1::sol::INIT)),
        ("exported", Out::A(a1::sol::EXPORTED.iter().map(fun).collect())),
        (
            "guildOnly",
            Out::A(a1::sol::GUILD_ONLY.iter().map(|n| Out::S((*n).to_string())).collect()),
        ),
        ("judgeFace", Out::A(a1::sol::JUDGE_FACE.iter().map(fun).collect())),
        (
            "judgeOnly",
            Out::A(a1::sol::JUDGE_ONLY.iter().map(|n| Out::S((*n).to_string())).collect()),
        ),
        ("views", Out::A(a1::sol::VIEWS.iter().map(fun).collect())),
    ]))
}
