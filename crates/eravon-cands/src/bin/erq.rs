#![forbid(unsafe_code)]

use std::process::ExitCode;

use eravon_cands::act::{self, Act, Seat};
use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;
use eravon_literals::json::{Out, write};
use eravon_literals::misuse::Misuse;

const EXIT_MISUSE: u8 = 2;

fn main() -> ExitCode {
    let Some(args) = std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect::<Option<Vec<String>>>() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erq", eravon_cands::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
    match eravon_chain::task::entry(|| {
        eravon_scan::bare::keep_reads_in(eravon_scan::bare::machine_if_there());
        eravon_scan::evidence::keep_blocks_on_machine();
        run(&args)
    }) {
        Ok(v) => {
            write_out(&write(&v));
            ExitCode::from(eravon_literals::schema::exit_of(&v))
        }
        Err(m) => misuse(&m),
    }
}

fn misuse(m: &Misuse) -> ExitCode {
    m.say("erq");
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
    fn many(&self, k: &str) -> Vec<String> {
        self.pairs.iter().filter(|(a, _)| a == k).map(|(_, v)| v.clone()).collect()
    }
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    use eravon_literals::schema::{T_ARR, T_INT, T_PATH, T_STR};

    const C_CHAIN: &str = "eravon.chain/1 · 端点与三分法闭集(E_CHAIN_USAGE · E_CHAIN_TRANSPORT · E_CHAIN_ENDPOINT · E_CHAIN_EXECUTION · E_CHAIN_DISAGREE · E_CHAIN_WRONG)";
    const C_ACT: &str = "eravon-cands act · 行为时核闭集(E_ACT_* · E_POOL_* · E_QUOTE_FORM · E_A1_PRECHECK 各门)";
    const C_POOL: &str = "eravon-roster pool · 池读法闭集(E_POOL_*)";
    pub const ALL: &[Verb] = &[
        Verb::new("act", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::need("--chain", T_INT), Arg::need("--logic", T_STR), Arg::need("--order", T_PATH), Arg::need("--registry", T_ARR)], &["E_ACT_FORM"], &[C_CHAIN, C_ACT]),
        Verb::new("pool", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::need("--chain", T_INT), Arg::need("--logic", T_STR), Arg::need("--registry", T_ARR), Arg::need("--pointer", T_STR), Arg::need("--guild-terms-file", T_PATH), Arg::need("--token", T_STR), Arg::opt("--party", T_ARR)], &[], &[C_CHAIN, C_POOL]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args.first().map(String::as_str).ok_or_else(|| Misuse::verb(None))?;
    let f = Flags { pairs: eravon_literals::schema::read_pairs(args.get(1..).unwrap_or_default())? };
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    match verb {
        "act" => act_verb(&f),
        "pool" => pool_verb(&f),
        "schema" => Ok(ok(eravon_literals::schema::out_rows(eravon_literals::schema::register(&eravon_literals::schema::version("erq"), register::ALL, &["E_ACT_FORM"])))),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn ok(mut m: Vec<(&'static str, Out)>) -> Out {
    m.push(("ok", Out::B(true)));
    Out::O(m)
}

fn refused(gate: &str, token: &str, detail: &str) -> Out {
    Out::O(vec![("detail", Out::S(detail.to_string())), ("gate", Out::S(gate.to_string())), ("ok", Out::B(false)), ("token", Out::S(token.to_string()))])
}

fn pool_verb(f: &Flags) -> Result<Out, Misuse> {
    let need = |k: &str| f.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k));
    let chain: u64 = need("--chain")?.parse().map_err(|_| Misuse::not_int("--chain", f.one("--chain").unwrap_or_default()))?;
    let logic = need("--logic")?.to_ascii_lowercase();
    let pointer = need("--pointer")?;
    let token = need("--token")?.to_ascii_lowercase();
    if !eravon_galeed::hexform::is_hex20(&token) {
        return Err(Misuse::bad("--token", &token, eravon_literals::misuse::HEX20));
    }
    let terms_path = need("--guild-terms-file")?;
    let terms = std::fs::read(&terms_path).map_err(|_| Misuse::unreadable(&terms_path, eravon_literals::misuse::UNREADABLE))?;
    let terms = String::from_utf8(terms).map_err(|_| Misuse::unreadable(&terms_path, eravon_literals::misuse::said("不是 UTF-8 文本", "not UTF-8 text")))?;
    let parties: Vec<String> = f.many("--party").into_iter().map(|p| p.to_ascii_lowercase()).collect();
    let nodes = f.many("--endpoint");
    if nodes.is_empty() {
        return Err(Misuse::args(eravon_literals::misuse::NEED_ENDPOINTS));
    }
    let registries: Vec<String> = f.many("--registry").into_iter().map(|r| r.to_ascii_lowercase()).collect();
    let chain_fault = |e: a2::fault::Fault| refused("chain", &e.name(), &e.detail());
    let table = match Table::new(nodes) {
        Ok(t) => t,
        Err(e) => return Ok(chain_fault(e)),
    };
    let t = match Auto::for_flags(f.one("--ca-file"), f.one("--proxy")) {
        Ok(t) => t,
        Err(e) => return Ok(chain_fault(e)),
    };
    if let Err(e) = a2::read::chain_guard(&t, &table, chain) {
        return Ok(chain_fault(e));
    }
    let (head, _) = match a2::read::height(&t, &table) {
        Ok(h) => h,
        Err(e) => return Ok(chain_fault(e)),
    };
    let now = match eravon_scan::scan::block_stamp(&t, &table, head) {
        Ok(n) => n,
        Err(e) => return Ok(refused("chain", &e.name(), &e.detail())),
    };
    let Some((_, guild)) = eravon_roster::adapter::pointer(&pointer) else {
        return Ok(refused("pool", eravon_roster::pool::PoolRefusal::Pointer.name(), &pointer));
    };
    let w = eravon_cands::wire::Wire { t, table, registries, now };
    let logs = match eravon_cands::registry::logs_of(&w, &[guild.to_string()]) {
        Ok(l) => l,
        Err(a) => return Ok(refused("registry", a.name(), &format!("{a:?}"))),
    };
    let book = eravon_cands::book::ChainBook { t: &w.t, table: &w.table, logic: &logic };
    Ok(match eravon_roster::pool::pool_candidate(&logs, &book, chain, &pointer, &terms, &token, &parties, now, head.saturating_add(1)) {
        Err(r) => refused("pool", r.name(), &pointer),
        Ok(p) => ok(vec![
            ("amanahHash", Out::S(p.amanah_hash)),
            ("expires", Out::N(p.expires)),
            ("guild", Out::S(p.guild)),
            ("head", Out::N(head)),
            ("members", Out::N(p.members as u64)),
            ("panelHash", Out::S(p.panel_hash)),
            ("quote", Out::S(p.quote)),
            ("root", Out::S(p.root)),
            ("router", Out::S(p.router)),
            ("seq", Out::N(p.seq)),
        ]),
    })
}

fn act_verb(f: &Flags) -> Result<Out, Misuse> {
    let need = |k: &str| f.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k));
    let chain: u64 = need("--chain")?.parse().map_err(|_| Misuse::not_int("--chain", f.one("--chain").unwrap_or_default()))?;
    let logic = need("--logic")?.to_ascii_lowercase();
    let order_path = need("--order")?;
    let nodes = f.many("--endpoint");
    if nodes.is_empty() {
        return Err(Misuse::args(eravon_literals::misuse::NEED_ENDPOINTS));
    }
    let registries: Vec<String> = f.many("--registry").into_iter().map(|r| r.to_ascii_lowercase()).collect();
    let bytes = std::fs::read(&order_path).map_err(|_| Misuse::unreadable(&order_path, eravon_literals::misuse::UNREADABLE))?;
    let chain_fault = |e: a2::fault::Fault| refused("chain", &e.name(), &e.detail());
    let o = match eravon_standard::order::read_json(&bytes) {
        Ok(o) => o,
        Err(e) => return Ok(refused("form", "E_ACT_FORM", &e.detail())),
    };
    let table = match Table::new(nodes) {
        Ok(t) => t,
        Err(e) => return Ok(chain_fault(e)),
    };
    let t = match Auto::for_flags(f.one("--ca-file"), f.one("--proxy")) {
        Ok(t) => t,
        Err(e) => return Ok(chain_fault(e)),
    };
    if let Err(e) = a2::read::chain_guard(&t, &table, chain) {
        return Ok(chain_fault(e));
    }
    let (head, _) = match a2::read::height(&t, &table) {
        Ok(h) => h,
        Err(e) => return Ok(chain_fault(e)),
    };
    let now = match eravon_scan::scan::block_stamp(&t, &table, head) {
        Ok(n) => n,
        Err(e) => return Ok(refused("chain", &e.name(), &e.detail())),
    };
    let w = eravon_cands::wire::Wire { t, table, registries, now };
    let asked: Vec<String> = [o.offer.judge.clone(), o.offer.guild.clone()].into_iter().filter(|a| !a.trim_start_matches("0x").bytes().all(|b| b == b'0')).collect();
    let logs = match eravon_cands::registry::logs_of(&w, &asked) {
        Ok(l) => l,
        Err(a) => return Ok(refused("registry", a.name(), &format!("{a:?}"))),
    };
    let ev = eravon_scan::evidence::OnChain { t: &w.t, table: &w.table };
    let book = eravon_cands::book::ChainBook { t: &w.t, table: &w.table, logic: &logic };
    let a = Act { t: &w.t, table: &w.table, logs: &logs, book: &book, ev: &ev, chain_id: chain, head, now };
    Ok(match act::check(&a, &o, &logic) {
        Err(r) => refused(r.gate, &r.name, &r.why),
        Ok(p) => {
            let seat = match p.seat {
                Seat::Named { judge, label, narrow } => Out::O(vec![
                    ("judge", Out::S(judge)),
                    ("label", Out::S(label)),
                    ("narrow", Out::A(narrow.into_iter().map(Out::S).collect())),
                    ("track", Out::S("named".into())),
                ]),
                Seat::Guild { guild, seq, panel_hash, pool, .. } => Out::O(vec![
                    ("guild", Out::S(guild)),
                    ("panelHash", Out::S(panel_hash)),
                    ("pool", Out::N(pool as u64)),
                    ("seq", Out::N(seq)),
                    ("track", Out::S("guild".into())),
                ]),
            };
            ok(vec![
                ("deposit", Out::S(p.priced.deposit.to_dec())),
                ("head", Out::N(head)),
                ("lf", Out::S(p.priced.lf.to_dec())),
                ("premium", Out::S(p.priced.premium.to_dec())),
                ("quote", Out::S(eravon_standard::order::hex_of(&p.quote))),
                ("seat", seat),
            ])
        }
    })
}
