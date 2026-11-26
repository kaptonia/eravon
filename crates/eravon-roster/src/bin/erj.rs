#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use eravon_galeed as k1;
use eravon_galeed::Value;
use eravon_roster as k5;
use k5::adapter::Logs;

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
    eravon_literals::trace::entry("erj", eravon_roster::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
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
    m.say("erj");
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

fn s(k: &str, v: String) -> (String, Value) {
    (k.into(), Value::Str(v))
}

fn read_file(path: &str) -> Result<Vec<u8>, Misuse> {
    let meta = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    if !meta.is_file() || meta.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
    }
    std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))
}

fn read_logs(b: &[u8]) -> Option<Logs> {
    let logs = k5::adapter::logs_of(&k1::canon::read_open(b)?)?;
    let mut seen: std::collections::BTreeSet<(u64, u64)> = std::collections::BTreeSet::new();
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

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_ROSTER: &str = "galeed.roster/1 · 名录接入闭集(galeed_roster::Refusal)";
    pub const ALL: &[Verb] = &[
        Verb::carry("guild", &[Arg::need("logs-file", T_PATH), Arg::need("guild-addr", T_STR)], &[C_ROSTER]),
        Verb::plain("candidate-set", &[Arg::need("logs-file", T_PATH), Arg::need("pointers", T_STR), Arg::need("allow", T_STR), Arg::need("parties", T_STR)]),
        Verb::new("pool", &[Arg::need("logs-file", T_PATH), Arg::need("book-file", T_PATH), Arg::need("pointer", T_STR), Arg::need("guild-terms-file", T_PATH), Arg::need("token", T_STR), Arg::need("parties", T_STR), Arg::need("now", T_INT)], &["E_POOL_POINTER", "E_POOL_ROSTER", "E_POOL_ROSTER_EMPTY", "E_POOL_NO_GUILDTERMS", "E_POOL_TERMS_DIGEST", "E_POOL_NO_QUOTE", "E_POOL_QUOTE_FORM", "E_POOL_NO_PANEL", "E_POOL_PANEL_HASH", "E_POOL_PANEL_EXPIRED", "E_POOL_PANEL_WORD", "E_POOL_PANEL_EMPTY"], &[]),
        Verb::plain("panel-hash", &[Arg::need("members", T_STR), Arg::need("hashes", T_STR)]),
        Verb::plain("gate", &[Arg::need("gate-file", T_PATH)]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Value, Misuse> {
    match (args.first().map(String::as_str), args.len()) {
        (Some("schema"), 1) => {
            let mut m = k1::schema::value_rows(eravon_literals::schema::register(
                &eravon_literals::schema::version("erj"),
                register::ALL,
                &[],
            ));
            m.push(("ok".into(), Value::Bool(true)));
            Ok(Value::Obj(m))
        }
        (Some("guild"), 3) => {
            let logs = logs_of(&args[1])?;
            let addr = &args[2];
            if !k1::hexform::is_hex20(addr) {
                return Err(Misuse::bad("guild-addr", addr, eravon_literals::misuse::HEX20));
            }
            Ok(match k5::adapter::guild_of(&logs, 1, addr) {
                Err(r) => Value::Obj(vec![
                    ("ok".into(), Value::Bool(false)),
                    s("token", r.name().into()),
                ]),
                Ok(g) => Value::Obj(vec![
                    ("card".into(), k5::adapter::card_answer(&g.card)),
                    ("issued".into(), Value::Int(g.issued)),
                    (
                        "judges".into(),
                        Value::Arr(g.judges.iter().map(|r| Value::Str(r.walk.final_addr.clone())).collect()),
                    ),
                    ("ok".into(), Value::Bool(true)),
                    s("guild", g.guild.clone()),
                ]),
            })
        }
        (Some("candidate-set"), 5) => {
            let logs = logs_of(&args[1])?;
            let pointers = split_list(&args[2]);
            let allow = split_list(&args[3]);
            let parties = split_list(&args[4]);
            let c = k5::pool::candidate_set(&logs, 1, &pointers, &allow, &parties);
            Ok(Value::Obj(vec![
                (
                    "candidates".into(),
                    Value::Arr(c.list.iter().map(|x| Value::Str(x.clone())).collect()),
                ),
                ("count".into(), Value::Int(c.list.len() as u64)),
                ("ok".into(), Value::Bool(true)),
            ]))
        }
        (Some("pool"), 8) => {
            let logs = logs_of(&args[1])?;
            let book = k5::book::FixtureBook::from_bytes(&read_file(&args[2])?).ok_or_else(|| Misuse::unreadable(&args[2], eravon_literals::misuse::said("读不成登记簿", "does not read as a registry book")))?;
            let pointer = &args[3];
            let terms_text = String::from_utf8(read_file(&args[4])?).map_err(|_| Misuse::unreadable(&args[4], eravon_literals::misuse::said("不是 UTF-8 文本", "not UTF-8 text")))?;
            let token = &args[5];
            let parties = split_list(&args[6]);
            let now = k5_now(&args[7])?;
            if !k1::hexform::is_hex20(token) {
                return Err(Misuse::bad("token", token, eravon_literals::misuse::HEX20));
            }
            let chain = k5::pool::standard_chain_id();
            Ok(
                match k5::pool::pool_candidate(
                    &logs, &book, chain, pointer, &terms_text, token, &parties, now, u64::MAX,
                ) {
                    Err(r) => Value::Obj(vec![
                        ("ok".into(), Value::Bool(false)),
                        s("token", r.name().into()),
                    ]),
                    Ok(p) => Value::Obj(vec![
                        s("amanahHash", p.amanah_hash.clone()),
                        ("expires".into(), Value::Int(p.expires)),
                        s("guild", p.guild.clone()),
                        ("members".into(), Value::Int(p.members as u64)),
                        ("ok".into(), Value::Bool(true)),
                        s("panelHash", p.panel_hash.clone()),
                        s("quote", p.quote.clone()),
                        s("root", p.root.clone()),
                        s("router", p.router.clone()),
                        ("seq".into(), Value::Int(p.seq)),
                    ]),
                },
            )
        }
        (Some("gate"), 2) => {
            let b = read_file(&args[1])?;
            let v = k1::canon::read_open(&b).ok_or_else(|| Misuse::unreadable(&args[1], eravon_literals::misuse::NOT_JSON))?;
            let count = match v.get("candidates") {
                Some(Value::Int(n)) => *n as usize,
                _ => return Err(Misuse::unreadable(&args[1], eravon_literals::misuse::said("gate-file 里 candidates 要是整数", "the gate file's candidates must be an integer"))),
            };
            let mut by_call = Vec::new();
            if let Some(Value::Arr(rows)) = v.get("byCall") {
                for r in rows {
                    let c = match r.get("candidate") {
                        Some(Value::Int(n)) => *n as usize,
                        _ => return Err(Misuse::unreadable(&args[1], eravon_literals::misuse::said("gate-file 里 byCall 每行的 candidate 要是整数", "every byCall row of the gate file needs an integer candidate"))),
                    };
                    let d = r.get("digest").and_then(Value::as_str).ok_or_else(|| Misuse::unreadable(&args[1], eravon_literals::misuse::said("gate-file 里 byCall 每行要有文本 digest", "every byCall row of the gate file needs a text digest")))?.to_string();
                    by_call.push((c, d));
                }
            }
            let reg = v.get("registry");
            let mut read = |_c: usize, d: &str| match reg.and_then(|m| m.get(d)).and_then(Value::as_str) {
                Some("held") => k5::gate::Registration::Held,
                Some("not-held") => k5::gate::Registration::NotHeld,
                _ => k5::gate::Registration::Unverified,
            };
            let got = k5::gate::gate(count, &by_call, &mut read);
            let rows: Vec<Value> = got
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    Value::Obj(vec![
                        ("candidate".into(), Value::Int(i as u64)),
                        s("note", a.note().to_string()),
                        s("state", a.word().to_string()),
                    ])
                })
                .collect();
            let choosable: Vec<Value> = got
                .iter()
                .enumerate()
                .filter(|(_, a)| a.choosable())
                .map(|(i, _)| Value::Int(i as u64))
                .collect();
            Ok(Value::Obj(vec![
                ("candidates".into(), Value::Arr(rows)),
                ("choosable".into(), Value::Arr(choosable)),
                ("ok".into(), Value::Bool(true)),
            ]))
        }
        (Some("panel-hash"), 3) => {
            let members = split_list(&args[1]);
            let hashes = split_list(&args[2]);
            if members.len() != hashes.len()
                || !members.iter().all(|m| k1::hexform::is_hex20(m))
                || !hashes.iter().all(|h| k1::hexform::is_hex32(h))
            {
                return Err(Misuse::args(eravon_literals::misuse::said("members 与 hashes 要逗号分开、一样多:members 每枚 0x 加四十位十六进制,hashes 每枚 0x 加六十四位十六进制", "members and hashes are comma-separated and as many: each member 0x and 40 hex digits, each hash 0x and 64")));
            }
            let h = k5::pool::panel_hash(&members, &hashes).ok_or_else(|| Misuse::args(eravon_literals::misuse::said("members 与 hashes 编不成成员表", "members and hashes do not make a member table")))?;
            Ok(Value::Obj(vec![("ok".into(), Value::Bool(true)), s("panelHash", h)]))
        }
        _ => Err(eravon_literals::schema::positional_misuse(register::ALL, args)),
    }
}

fn k5_now(a: &str) -> Result<u64, Misuse> {
    k5::adapter::int_spelling(a).ok_or_else(|| Misuse::bad("now", a, eravon_literals::misuse::INT_SPELLING))
}

fn logs_of(path: &str) -> Result<Logs, Misuse> {
    read_logs(&read_file(path)?).ok_or_else(|| Misuse::unreadable(path, eravon_literals::misuse::said("读不成名册日志", "does not read as roster logs")))
}

fn split_list(a: &str) -> Vec<String> {
    if a.is_empty() { Vec::new() } else { a.split(',').map(str::to_string).collect() }
}
