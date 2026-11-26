#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;
use eravon_literals::json::{Out, write};
use eravon_scan as a3;
use a3::fault::Fault;
use a3::scan::{self, Side};

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erx", eravon_scan::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
    match eravon_chain::task::entry(|| {
        eravon_scan::bare::keep_reads_in(eravon_scan::bare::machine_if_there());
        eravon_scan::evidence::keep_blocks_on_machine();
        run(&args)
    }) {
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
    m.say("erx");
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
    fn num(&self, k: &str, dflt: u64) -> Result<u64, Misuse> {
        match self.one(k) {
            None => Ok(dflt),
            Some(s) => s.parse().map_err(|_| Misuse::not_int(k, s)),
        }
    }
}

fn flags(args: &[String]) -> Result<Flags, Misuse> {
    Ok(Flags { pairs: eravon_literals::schema::read_pairs(args.get(1..).unwrap_or_default())? })
}

fn need(f: &Flags, k: &str) -> Result<String, Misuse> {
    f.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k))
}

fn ok(mut m: Vec<(&'static str, Out)>) -> Out {
    m.push(("ok", Out::B(true)));
    Out::O(m)
}

fn refused(f: &Fault) -> Out {
    Out::O(vec![
        ("detail", Out::S(f.detail())),
        ("ok", Out::B(false)),
        ("token", Out::S(f.name())),
    ])
}

fn ans(r: a3::fault::R<Out>) -> Result<Out, Misuse> {
    Ok(match r {
        Ok(v) => v,
        Err(f) => refused(&f),
    })
}

fn upto(f: &Flags) -> Result<a2::seam::Upto, Misuse> {
    let to = match f.one("--to") {
        None => None,
        Some(_) => Some(f.num("--to", 0)?),
    };
    a2::seam::Upto::of_range(0, to, None).map_err(|_| Misuse::bad("--to", f.one("--to").unwrap_or_default(), eravon_literals::misuse::NOT_INT))
}

fn table(f: &Flags) -> Result<Result<Table, a2::fault::Fault>, Misuse> {
    if let Some(path) = f.one("--table") {
        let bytes = std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
        return Ok(Table::open(&bytes));
    }
    let inline = f.many("--endpoint");
    if inline.is_empty() {
        return Err(Misuse::args(eravon_literals::misuse::NEED_ENDPOINTS));
    }
    Ok(Table::new(inline.iter().map(|x| (*x).to_string()).collect()))
}

fn wire(f: &Flags) -> a2::fault::R<Auto> {
    Auto::for_flags(f.one("--ca-file"), f.one("--proxy"))
}

fn chain_and_logic(f: &Flags) -> Result<(u64, String), Misuse> {
    let (chain, logic, _) = eravon_standard::chain::standard_table();
    let c = match f.one("--chain") {
        None => chain,
        Some(s) => s.parse().map_err(|_| Misuse::not_int("--chain", s))?,
    };
    Ok((c, f.one("--logic").unwrap_or(logic).to_string()))
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_CHAIN: &str = "eravon.chain/1 · 端点与三分法闭集(E_CHAIN_USAGE · E_CHAIN_TRANSPORT · E_CHAIN_ENDPOINT · E_CHAIN_EXECUTION · E_CHAIN_DISAGREE · E_CHAIN_WRONG)";
    const C_A1: &str = "eravon.standard/1 · ABI 与形闸闭集(E_A1_FORM · E_A1_ABI)";
    const C_LAW: &str = "eravon.terms/1 · 订单派生闭集(E_ORDER_*)";
    const C_K1: &str = "galeed/1 · 证据包与审计(§9.6 reason · §8.7 label)";
    pub const ALL: &[Verb] = &[
        Verb::new("opened", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::need("--key", T_STR), Arg::need("--side", T_STR), Arg::opt("--from", T_INT), Arg::opt("--to", T_INT)], &["E_A3_FORM"], &[C_CHAIN]),
        Verb::new("rebuild", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::need("--tx", T_STR), Arg::need("--case-id", T_STR), Arg::opt("--judge", T_ARR), Arg::opt("--guild", T_ARR), Arg::opt("--seq", T_INT)], &["E_A3_FORM", "E_A3_REBUILD"], &[C_CHAIN, C_A1, C_LAW]),
        Verb::new("history", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::need("--instance", T_STR), Arg::opt("--from", T_INT), Arg::opt("--to", T_INT)], &["E_A3_FORM"], &[C_CHAIN, C_A1]),
        Verb::new("ledger", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--root", T_STR), Arg::need("--basis-file", T_PATH)], &["E_A3_FORM"], &[C_CHAIN, C_K1]),
        Verb::new("board-key", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--key", T_STR), Arg::opt("--from", T_INT)], &["E_A3_FORM"], &[C_CHAIN]),
        Verb::new("board-tx", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--tx", T_STR), Arg::need("--hash", T_STR)], &["E_A3_FORM"], &[C_CHAIN]),
        Verb::new("order-no", &[Arg::need("--endpoint", T_ARR), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--logic", T_STR), Arg::need("--key", T_STR), Arg::need("--side", T_STR), Arg::opt("--from", T_INT), Arg::opt("--to", T_INT)], &["E_A3_FORM"], &[C_CHAIN]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args.first().map(String::as_str).ok_or_else(|| Misuse::verb(None))?;
    let f = flags(args)?;
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    match verb {
        "opened" => opened(&f),
        "rebuild" => rebuild(&f),
        "history" => history(&f),
        "ledger" => ledger(&f),
        "board-key" => board_key(&f),
        "board-tx" => board_tx(&f),
        "order-no" => order_no(&f),
        "schema" => Ok(ok(eravon_literals::schema::out_rows(eravon_literals::schema::register(
            &eravon_literals::schema::version("erx"),
            register::ALL,
            &["E_A3_FORM", "E_CHAIN_USAGE", "E_CHAIN_TRANSPORT", "E_CHAIN_ENDPOINT", "E_CHAIN_EXECUTION", "E_CHAIN_DISAGREE", "E_CHAIN_WRONG"],
        )))),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn opened(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let (chain, logic) = chain_and_logic(f)?;
    let key = need(f, "--key")?;
    let side = match need(f, "--side")?.as_str() {
        "seller" => Side::Seller,
        "buyer" => Side::Buyer,
        x => return Err(Misuse::bad("--side", x, eravon_literals::misuse::SEAT)),
    };
    let from = f.num("--from", 0)?;
    let to = upto(f)?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let sw = scan::by_key(&w, &t, &logic, side, &key, from, to)?;
        Ok(ok(vec![
            ("agreement", Out::S(sw.agreement.to_string())),
            (
                "cases",
                Out::A(
                    sw.cases
                        .iter()
                        .map(|c| {
                            Out::O(vec![
                                ("block", Out::N(c.block)),
                                ("buyer", Out::S(c.buyer.clone())),
                                ("caseId", Out::S(c.case_id.clone())),
                                ("instance", Out::S(c.instance.clone())),
                                ("seller", Out::S(c.seller.clone())),
                                ("tx", Out::S(c.tx.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("count", Out::N(sw.cases.len() as u64)),
            ("foreign", Out::N(sw.foreign)),
        ]))
    })())
}

fn rebuild(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let (chain, logic) = chain_and_logic(f)?;
    let tx = need(f, "--tx")?;
    let case_id = need(f, "--case-id")?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let r = match scan::rebuild(&w, &t, &tx, chain, &logic, &case_id)? {
            scan::Rebuild::Done(r) => r,
            scan::Rebuild::Untraced => {
                return Ok(ok(vec![
                    ("state", Out::S("untraced".into())),
                    ("why", Out::S(scan::SAY_UNTRACED.into())),
                ]));
            }
        };
        let judges: Vec<String> = f.many("--judge").iter().map(|x| (*x).to_string()).collect();
        let guilds: Vec<String> = f.many("--guild").iter().map(|x| (*x).to_string()).collect();
        let receipt = if judges.is_empty() && guilds.is_empty() {
            None
        } else {
            let seq = match f.one("--seq") {
                None => None,
                Some(_) => Some(f.num("--seq", 0).map_err(|_| a3::fault::Fault::Form("--seq 不是数".into()))?),
            };
            Some(scan::backfill(&r, chain, &judges, &guilds, seq)?)
        };
        let mut m = vec![
            ("amount", Out::S(r.amount.clone())),
            ("buyer", Out::S(r.buyer.clone())),
            ("caseId", Out::S(r.case_id.clone())),
            ("logic", Out::S(r.logic.to_ascii_lowercase())),
            ("members", Out::N(r.members as u64)),
            ("past", Out::B(r.past)),
            ("seller", Out::S(r.seller.clone())),
            ("termsHash", Out::S(r.terms_hash.clone())),
            ("path", Out::S(r.path.to_string())),
            ("state", Out::S("rebuilt".into())),
            ("token", Out::S(r.token.clone())),
        ];
        if let Some(b) = receipt {
            m.push(("receipt", Out::S(String::from_utf8_lossy(&b).into_owned())));
        }
        Ok(ok(m))
    })())
}

fn history(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let (chain, logic) = chain_and_logic(f)?;
    let instance = need(f, "--instance")?;
    let from = f.num("--from", 0)?;
    let to = upto(f)?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let rows = scan::history(&w, &t, chain, &logic, &instance, from, to)?;
        Ok(ok(vec![
            ("count", Out::N(rows.len() as u64)),
            (
                "moments",
                Out::A(
                    rows.iter()
                        .map(|m| {
                            Out::O(vec![
                                ("block", Out::N(m.block)),
                                ("event", Out::S(m.name.clone())),
                                ("logIndex", Out::N(m.log_index)),
                                ("tx", Out::S(m.tx.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]))
    })())
}

fn ledger(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let (chain, _) = chain_and_logic(f)?;
    let root = need(f, "--root")?;
    let path = need(f, "--basis-file")?;
    let bytes = std::fs::read(&path).map_err(|_| Misuse::unreadable(&path, eravon_literals::misuse::UNREADABLE))?;
    ans((|| {
        let basis = eravon_galeed::canon::read_open(&bytes).ok_or_else(|| Fault::Form("基底档不是 JSON".into()))?;
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let (head, _) = a2::read::height(&w, &t)?;
        let ev = a3::evidence::OnChain { t: &w, table: &t };
        let l = a3::ledger::read(&w, &t, &ev, chain, &root, &basis, head)?;
        let s = |x: &str| Out::S(x.to_string());
        Ok(ok(vec![
            (
                "entries",
                Out::A(
                    l.entries
                        .iter()
                        .map(|e| {
                            Out::O(vec![
                                ("entryId", s(&e.entry_id)),
                                ("entryType", s(e.value.get("entryType").and_then(eravon_galeed::Value::as_str).unwrap_or_default())),
                                ("block", Out::N(e.block)),
                                ("tx", s(&e.tx)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("discarded", Out::A(l.discarded.iter().map(|d| Out::O(vec![("block", Out::N(d.block)), ("entryId", s(&d.entry_id)), ("sender", s(&d.sender)), ("tx", s(&d.tx)), ("why", s(eravon_scan::ledger::DISCARDED_SAYS))])).collect())),
            ("head", Out::N(head)),
            ("label", s(&l.label())),
            ("lineage", Out::A(l.lineage.iter().map(|k| s(k)).collect())),
            ("narrow", Out::A(l.narrow.iter().map(|n| s(n)).collect())),
            ("unchecked", Out::A(l.unchecked.iter().map(|n| Out::O(vec![("logIndex", Out::N(n.log_index)), ("reason", s(&n.reason)), ("tx", s(&n.tx))])).collect())),
            ("notAnchors", Out::A(l.not_anchors.iter().map(|n| Out::O(vec![("logIndex", Out::N(n.log_index)), ("reason", s(&n.reason)), ("tx", s(&n.tx))])).collect())),
            (
                "records",
                Out::A(
                    l.records
                        .iter()
                        .map(|r| Out::O(vec![("block", Out::N(r.block_number)), ("hash", s(&r.hash)), ("sender", s(&r.sender)), ("tx", s(&r.tx)), ("verdict", s(r.verdict.as_str()))]))
                        .collect(),
                ),
            ),
            ("root", s(&l.root)),
        ]))
    })())
}

fn post_out(p: &a3::board::Post) -> Out {
    Out::O(vec![
        ("block", Out::N(p.block)),
        ("document", Out::S(eravon_standard::order::hex_of(&p.document))),
        ("hash", Out::S(eravon_standard::order::hex_of(&p.hash))),
        ("key", Out::S(eravon_standard::order::hex_of(&p.key))),
        ("logIndex", Out::N(p.log_index)),
        ("poster", Out::S(p.poster.clone())),
        ("tx", Out::S(p.tx.clone())),
    ])
}

fn hex32(f: &Flags, k: &str) -> Result<[u8; 32], Misuse> {
    let v = need(f, k)?;
    eravon_standard::order::b32_of(k, &v).map_err(|_| Misuse::bad(k, &v, eravon_literals::misuse::said("不是 0x 加六十四位十六进制", "not 0x and sixty-four hex digits")))
}

fn board_key(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let (chain, _) = chain_and_logic(f)?;
    let key = hex32(f, "--key")?;
    let from = f.num("--from", 0)?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let posts = a3::board::by_key(&w, &t, &key, from, a2::seam::Upto::Latest)?;
        Ok(ok(vec![("count", Out::N(posts.len() as u64)), ("posts", Out::A(posts.iter().map(post_out).collect()))]))
    })())
}

fn board_tx(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let (chain, _) = chain_and_logic(f)?;
    let tx = need(f, "--tx")?;
    let hash = hex32(f, "--hash")?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let posts = a3::board::by_tx(&w, &t, &tx, &hash)?;
        Ok(ok(vec![("count", Out::N(posts.len() as u64)), ("posts", Out::A(posts.iter().map(post_out).collect()))]))
    })())
}

fn order_no(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let (chain, logic) = chain_and_logic(f)?;
    let key = need(f, "--key")?;
    let side = match need(f, "--side")?.as_str() {
        "seller" => Side::Seller,
        "buyer" => Side::Buyer,
        x => return Err(Misuse::bad("--side", x, eravon_literals::misuse::SEAT)),
    };
    let from = f.num("--from", 0)?;
    let to = upto(f)?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let sw = scan::by_key(&w, &t, &logic, side, &key, from, to)?;
        let mut stamps = Vec::with_capacity(sw.cases.len());
        for c in &sw.cases {
            stamps.push(scan::block_stamp(&w, &t, c.block)?);
        }
        let nos = scan::order_numbers(&sw.cases, &stamps)?;
        Ok(ok(vec![
            ("count", Out::N(nos.len() as u64)),
            (
                "orders",
                Out::A(
                    nos.iter()
                        .map(|n| {
                            Out::O(vec![
                                ("caseId", Out::S(n.case_id.clone())),
                                ("month", Out::S(n.month.clone())),
                                ("no", Out::S(format!("{}-{:04}", n.month, n.seq))),
                                ("seq", Out::N(n.seq)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]))
    })())
}
