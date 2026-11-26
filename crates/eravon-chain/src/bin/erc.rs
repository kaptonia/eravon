#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;
use std::time::Duration;

use eravon_chain as a2;
use a2::endpoints::Table;
use a2::fault::Fault;
use a2::task::Outcome;
use a2::transport::Auto;
use eravon_literals::json::{Out, write};

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;
const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erc", eravon_chain::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
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
    m.say("erc");
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
    bare: Vec<String>,
}

impl Flags {
    fn one(&self, k: &str) -> Option<&str> {
        self.pairs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    fn many(&self, k: &str) -> Vec<&str> {
        self.pairs.iter().filter(|(a, _)| a == k).map(|(_, v)| v.as_str()).collect()
    }
    fn has(&self, k: &str) -> bool {
        self.bare.iter().any(|a| a == k)
    }
    fn num_need(&self, k: &str) -> Result<u64, Misuse> {
        let s = self.one(k).ok_or_else(|| Misuse::missing(k))?;
        s.parse().map_err(|_| Misuse::not_int(k, s))
    }
    fn num(&self, k: &str, dflt: u64) -> Result<u64, Misuse> {
        match self.one(k) {
            None => Ok(dflt),
            Some(s) => s.parse().map_err(|_| Misuse::not_int(k, s)),
        }
    }
}

const BARE: &[&str] = &["--logs", "--once"];

fn flags(args: &[String]) -> Result<Flags, Misuse> {
    let (pairs, bare) = eravon_literals::schema::read_flags(args.get(1..).unwrap_or_default(), BARE)?;
    Ok(Flags { pairs, bare })
}

fn need(f: &Flags, k: &str) -> Result<String, Misuse> {
    f.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k))
}

fn table(f: &Flags) -> Result<Result<Table, Fault>, Misuse> {
    let inline = f.many("--endpoint");
    match (inline.is_empty(), f.one("--table")) {
        (false, None) => Ok(Table::new(inline.iter().map(|x| x.to_string()).collect())),
        (true, Some(path)) => {
            let m = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
            if !m.is_file() || m.len() > FILE_BOUND {
                return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
            }
            let bytes = std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
            Ok(Table::open(&bytes))
        }
        (false, Some(_)) => Err(Misuse::both(&["--endpoint", "--table"], eravon_literals::misuse::ONE_FORM)),
        (true, None) => Err(Misuse::args(eravon_literals::misuse::NEED_ENDPOINTS)),
    }
}

fn ok(mut m: Vec<(&'static str, Out)>) -> Out {
    m.push(("ok", Out::B(true)));
    Out::O(m)
}

fn refused(f: &Fault) -> Out {
    let mut m = vec![
        ("detail", Out::S(f.detail())),
        ("ok", Out::B(false)),
        ("token", Out::S(f.name().to_string())),
    ];
    if let Some(c) = f.chain_class() {
        m.push(("class", Out::S(c.to_string())));
    }
    Out::O(m)
}

fn ans(r: a2::fault::R<Out>) -> Result<Out, Misuse> {
    Ok(match r {
        Ok(v) => v,
        Err(f) => refused(&f),
    })
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_CHAIN: &str = "eravon.chain/1 · 端点与三分法闭集(E_CHAIN_USAGE · E_CHAIN_TRANSPORT · E_CHAIN_ENDPOINT · E_CHAIN_EXECUTION · E_CHAIN_DISAGREE · E_CHAIN_WRONG)";
    const C_ONE: &str = "eravon.chain/1 · 单端点闭集(E_CHAIN_USAGE · E_CHAIN_TRANSPORT · E_CHAIN_ENDPOINT · E_CHAIN_EXECUTION)";
    pub const ALL: &[Verb] = &[
        Verb::plain("window", &[Arg::need("--end", T_INT), Arg::need("--now", T_INT)]),
        Verb::carry("endpoints-check", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR)], &[C_CHAIN]),
        Verb::carry("chain-id", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT)], &[C_CHAIN]),
        Verb::carry("balance", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--address", T_STR)], &[C_CHAIN]),
        Verb::carry("call", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--to", T_STR), Arg::need("--data", T_STR)], &[C_CHAIN]),
        Verb::carry("trace", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--tx", T_STR), Arg::need("--to", T_STR), Arg::need("--selector", T_STR)], &[C_CHAIN]),
        Verb::carry("logs", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--address", T_STR), Arg::opt("--from", T_INT), Arg::opt("--to", T_INT), Arg::opt("--blocks", T_INT)], &[C_CHAIN]),
        Verb::carry("clock", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT)], &[C_CHAIN]),
        Verb::carry("poll", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--address", T_STR), Arg::opt("--blocks", T_INT), Arg::opt("--interval-ms", T_INT), Arg::opt("--budget-ms", T_INT), Arg::opt("--logs", T_BOOL), Arg::opt("--once", T_BOOL), Arg::opt("--state", T_PATH)], &[C_CHAIN]),
        Verb::carry("gas", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::opt("--chain", T_INT), Arg::need("--to", T_STR), Arg::opt("--from", T_STR), Arg::opt("--value", T_STR), Arg::opt("--value-text", T_STR), Arg::opt("--data", T_STR), Arg::opt("--blocks", T_INT)], &[C_CHAIN]),
        Verb::carry("nonce", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::need("--address", T_STR), Arg::opt("--at", T_STR)], &[C_CHAIN]),
        Verb::carry("send", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::need("--raw", T_STR)], &[C_ONE]),
        Verb::carry("receipt", &[Arg::opt("--endpoint", T_ARR), Arg::opt("--table", T_PATH), Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR), Arg::need("--tx", T_STR), Arg::opt("--wait", T_INT)], &[C_ONE]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args.first().map(String::as_str).ok_or_else(|| Misuse::verb(None))?;
    let f = flags(args)?;
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    match verb {
        "window" => window(&f),
        "endpoints-check" => endpoints_check(&f),
        "chain-id" => chain_id(&f),
        "balance" => balance(&f),
        "call" => call(&f),
        "trace" => trace(&f),
        "logs" => logs(&f),
        "clock" => clock(&f),
        "poll" => poll(&f),
        "gas" => gas(&f),
        "nonce" => nonce(&f),
        "send" => send(&f),
        "receipt" => receipt(&f),
        "schema" => Ok(ok(eravon_literals::schema::out_rows(eravon_literals::schema::register(
            &eravon_literals::schema::version("erc"),
            register::ALL,
            &[],
        )))),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn nonce(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let who = need(f, "--address")?;
    let at = f.one("--at").unwrap_or("latest").to_string();
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        let mut m: Vec<(&'static str, Out)> = Vec::new();
        let n = if at == "pending" {
            let e = a2::sending::endpoint_for_send(&t)?;
            m.push(("endpoint", Out::S(e.said())));
            m.push(("agreement", Out::S("single".into())));
            a2::sending::next_nonce_at(&w, &e, &who, &at)?
        } else {
            let (n, r) = a2::sending::settled_nonce(&w, &t, &who, &at)?;
            m.extend(r.rows());
            n
        };
        m.push(("address", Out::S(who.clone())));
        m.push(("at", Out::S(at.clone())));
        m.push(("nonce", Out::N(n)));
        Ok(ok(m))
    })())
}

fn send(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let raw = need(f, "--raw")?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        let bytes = a2::seam::bytes_of("--raw", &raw)?;
        let sent = a2::sending::send_raw(&w, &t, &bytes)?;
        Ok(ok(vec![
            ("endpoint", Out::S(sent.endpoint.said())),
            ("txHash", Out::S(sent.tx_hash)),
        ]))
    })())
}

fn receipt(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let hash = need(f, "--tx")?;
    let wait = f.num("--wait", 30)?;
    let endpoint = f.one("--endpoint").map(str::to_string);
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        let e = a2::seam::NodeAt::new(endpoint.clone().unwrap_or_else(|| t.list[0].clone()));
        let r = match wait {
            0 => a2::sending::receipt_once(&w, &e, &hash)?,
            n => Some(a2::sending::await_receipt(
                &w,
                &t,
                &e,
                &hash,
                std::time::Duration::from_secs(n),
                std::time::Duration::from_millis(200),
            )?),
        };
        Ok(ok(match r {
            None => vec![("found", Out::B(false)), ("txHash", Out::S(hash.clone()))],
            Some(r) => vec![
                ("block", Out::N(r.block)),
                ("contract", Out::S(r.contract.clone())),
                ("found", Out::B(true)),
                ("gasUsed", Out::N(r.gas_used)),
                ("status", Out::N(r.status)),
                ("txHash", Out::S(r.tx_hash.clone())),
            ],
        }))
    })())
}

fn expect_chain(f: &Flags) -> Result<u64, Misuse> {
    match f.one("--chain") {
        None => Ok(a2::seam::home_chain_id()),
        Some(s) => s.parse().map_err(|_| Misuse::not_int("--chain", s)),
    }
}

fn wire(f: &Flags) -> a2::fault::R<Auto> {
    Auto::for_flags(f.one("--ca-file"), f.one("--proxy"))
}

fn window(f: &Flags) -> Result<Out, Misuse> {
    let end: u64 = f.num_need("--end")?;
    let now: u64 = f.num_need("--now")?;
    let w = a2::clock::window(end, now);
    Ok(ok(vec![("action", Out::B(w.action)), ("stamp", Out::B(w.stamp))]))
}

fn endpoints_check(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    ans(t.map(|t| {
        ok(vec![
            ("count", Out::N(t.len() as u64)),
            ("endpoints", Out::A(t.list.iter().map(|e| Out::S(e.clone())).collect())),
        ])
    }))
}

fn chain_id(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        let (id, r) = a2::read::chain_guard(&w, &t, expect)?;
        let mut m = r.rows();
        m.push(("chainId", Out::N(id)));
        Ok(ok(m))
    })())
}

fn balance(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    let addr = need(f, "--address")?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        let (dec, r) = a2::read::balance(&w, &t, &addr, a2::seam::TAG_LATEST)?;
        let mut m = r.rows();
        m.push(("balance", Out::S(dec)));
        Ok(ok(m))
    })())
}

fn call(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    let to = need(f, "--to")?;
    let data = need(f, "--data")?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        let (out, r) = a2::read::call(&w, &t, &to, &data, a2::seam::TAG_LATEST)?;
        let mut m = r.rows();
        m.push(("data", Out::S(out)));
        Ok(ok(m))
    })())
}

fn trace(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    let tx = need(f, "--tx")?;
    let to = need(f, "--to")?;
    let sel = need(f, "--selector")?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        let (got, rows) = a2::trace::find(&w, &t, &tx, &to, &sel)?;
        let support = Out::A(
            rows.iter()
                .map(|r| {
                    Out::O(vec![
                        ("endpoint", Out::S(r.endpoint.clone())),
                        ("source", Out::S(r.source.to_string())),
                        ("support", Out::S(r.word.clone())),
                    ])
                })
                .collect(),
        );
        let mut m = match got {
            a2::trace::Traced::Found { call, agreement, endpoints } => vec![
                ("agreement", Out::S(agreement.word().to_string())),
                ("endpoints", Out::N(endpoints as u64)),
                ("from", Out::S(call.from)),
                ("input", Out::S(call.input)),
                ("state", Out::S("found".into())),
            ],
            a2::trace::Traced::NotFound { agreement, endpoints } => vec![
                ("agreement", Out::S(agreement.word().to_string())),
                ("endpoints", Out::N(endpoints as u64)),
                ("state", Out::S("not-found".into())),
            ],
            a2::trace::Traced::Untraced => vec![
                ("state", Out::S("untraced".into())),
                ("why", Out::S("表上无一端点能追踪这一笔:依赖追踪的读取未重算".into())),
            ],
        };
        m.push(("support", support));
        Ok(ok(m))
    })())
}

fn logs(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    let addr = need(f, "--address")?;
    let from = f.num("--from", 0)?;
    let to_n = match f.one("--to") {
        None => None,
        Some(_) => Some(f.num("--to", 0)?),
    };
    let blocks = match f.one("--blocks") {
        None => None,
        Some(_) => Some(f.num("--blocks", 0)?),
    };
    let to = a2::seam::Upto::of_range(from, to_n, blocks).map_err(|_| Misuse::both(&["--from", "--to", "--blocks"], eravon_literals::misuse::said("区间不成立:--to 与 --blocks 只给一种,--blocks 至少一块,且不越块号的界", "the range does not hold: give --to or --blocks, at least one block, within the block numbers")))?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        let (rows, r) = a2::read::logs(&w, &t, &addr, from, to)?;
        let mut m = r.rows();
        m.push(("count", Out::N(rows.len() as u64)));
        Ok(ok(m))
    })())
}

fn clock(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        let (n, r) = a2::clock::now(&w, &t)?;
        let mut m = r.rows();
        m.push(("block", Out::N(n.block)));
        m.push(("timestamp", Out::N(n.timestamp)));
        Ok(ok(m))
    })())
}

fn poll(f: &Flags) -> Result<Out, Misuse> {
    if f.has("--once") {
        return poll_once(f);
    }
    let t = table(f)?;
    let expect = expect_chain(f)?;
    let addr = need(f, "--address")?;
    let blocks = f.num("--blocks", 8)?;
    let interval = f.num("--interval-ms", 100)?;
    let budget = f.num("--budget-ms", 8000)?;
    let with_logs = f.has("--logs");
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        a2::seam::need_address(&addr)?;
        let (tb, ad, ca) = (t.clone(), addr.clone(), f.one("--ca-file").map(str::to_string));
        let mut task = a2::action::within(None, || a2::task::spawn("watch", move |tx| {
            let inner = Auto::for_ca_file(ca.as_deref());
            let inner = match inner {
                Ok(x) => x,
                Err(e) => {
                    let _ = tx.send(Outcome::Failed(e));
                    return;
                }
            };
            a2::poll::watch_balance(
                &inner,
                &tb,
                &ad,
                blocks,
                Duration::from_millis(interval),
                with_logs,
                tx,
            );
        }));
        let got = taken(&mut task, Duration::from_millis(budget));
        let mut m: Vec<(&'static str, Out)> = vec![
            ("blocks", Out::N(count(&got, |x| matches!(x, a2::poll::Tick::Block(_))))),
            ("idleTicks", Out::N(task.idle_ticks)),
            ("logs", Out::N(count(&got, |x| matches!(x, a2::poll::Tick::Logs { .. })))),
            ("outcomes", Out::N(task.taken)),
        ];
        for o in &got {
            match o {
                Outcome::Item(a2::poll::Tick::Changed { block, from, to }) => {
                    m.push(("changedAtBlock", Out::N(*block)));
                    m.push(("changedFrom", Out::S(from.clone())));
                    m.push(("changedTo", Out::S(to.clone())));
                }
                Outcome::Failed(fault) => return Err(fault.clone()),
                _ => {}
            }
        }
        m.push(("changed", Out::B(m.iter().any(|(k, _)| *k == "changedAtBlock"))));
        Ok(ok(m))
    })())
}

fn taken<T>(task: &mut a2::task::Task<T>, budget: Duration) -> Vec<Outcome<T>> {
    let deadline = std::time::Instant::now() + budget;
    let mut got = Vec::new();
    while std::time::Instant::now() < deadline {
        let now = task.take();
        let end = now.iter().any(|o| matches!(o, Outcome::Done | Outcome::Failed(_)));
        got.extend(now);
        if end {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    got
}

fn poll_once(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    let addr = need(f, "--address")?;
    let state = need(f, "--state")?;
    let budget = f.num("--budget-ms", 8000)?;
    let with_logs = f.has("--logs");
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        a2::seam::need_address(&addr)?;
        let prev = read_state(&state)?;
        let (was, since) = match &prev {
            Some((b, h)) => (Some(b.clone()), Some(*h)),
            None => (None, None),
        };
        let (tb, ad, ca) = (t.clone(), addr.clone(), f.one("--ca-file").map(str::to_string));
        let mut task = a2::task::spawn("poll", move |tx| {
            let inner = Auto::for_ca_file(ca.as_deref());
            let inner = match inner {
                Ok(x) => x,
                Err(e) => {
                    let _ = tx.send(Outcome::Failed(e));
                    return;
                }
            };
            a2::poll::tick_once(&inner, &tb, &ad, was.as_deref(), since, with_logs, tx);
        });
        let got = taken(&mut task, Duration::from_millis(budget));
        let mut height = prev.as_ref().map(|(_, h)| *h).unwrap_or(0);
        let mut now_value: Option<String> = None;
        let mut m: Vec<(&'static str, Out)> = vec![
            ("blocks", Out::N(count(&got, |x| matches!(x, a2::poll::Tick::Block(_))))),
            ("idleTicks", Out::N(task.idle_ticks)),
            ("logs", Out::N(count(&got, |x| matches!(x, a2::poll::Tick::Logs { .. })))),
            ("outcomes", Out::N(task.taken)),
        ];
        for o in &got {
            match o {
                Outcome::Item(a2::poll::Tick::Block(h)) => height = *h,
                Outcome::Item(a2::poll::Tick::Changed { block, from, to }) => {
                    m.push(("changedAtBlock", Out::N(*block)));
                    m.push(("changedFrom", Out::S(from.clone())));
                    m.push(("changedTo", Out::S(to.clone())));
                    now_value = Some(to.clone());
                }
                Outcome::Failed(fault) => return Err(fault.clone()),
                _ => {}
            }
        }
        if !got.iter().any(|o| matches!(o, Outcome::Done)) {
            return Err(Fault::Transport(format!("late 这一趟没在 {budget} 毫秒内走完(--budget-ms),状态档没写")));
        }
        m.push(("changed", Out::B(m.iter().any(|(k, _)| *k == "changedAtBlock"))));
        let keep = match (now_value, &prev) {
            (Some(v), _) => v,
            (None, Some((b, _))) => b.clone(),
            (None, None) => a2::read::balance(&w, &t, &addr, a2::seam::TAG_LATEST)?.0,
        };
        write_state(&state, &keep, height)?;
        m.push(("state", Out::S(state.clone())));
        m.push(("stateBlock", Out::N(height)));
        Ok(ok(m))
    })())
}

fn read_state(path: &str) -> a2::fault::R<Option<(String, u64)>> {
    let raw = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Fault::Usage(format!("读不到状态档 {path}:{e}"))),
    };
    let doc = eravon_literals::json::read(&raw)
        .map_err(|e| Fault::Usage(format!("状态档 {path} 不是一份读得动的 JSON:{e}")))?;
    let bal = doc
        .get("balance")
        .and_then(|x| x.as_str())
        .ok_or_else(|| Fault::Usage(format!("状态档 {path} 上没有 balance")))?
        .to_string();
    let blk = match doc.get("block") {
        Some(eravon_literals::json::In::Int(n)) if *n >= 0 => *n as u64,
        _ => return Err(Fault::Usage(format!("状态档 {path} 上没有 block(要一枚非负整数)"))),
    };
    Ok(Some((bal, blk)))
}

fn write_state(path: &str, balance: &str, block: u64) -> a2::fault::R<()> {
    let body = write(&Out::O(vec![
        ("balance", Out::S(balance.to_string())),
        ("block", Out::N(block)),
        ("v", Out::S("eravon.poll.state/1".to_string())),
    ]));
    std::fs::write(path, body).map_err(|e| Fault::Usage(format!("写不进状态档 {path}:{e}")))
}

fn count<F: Fn(&a2::poll::Tick) -> bool>(got: &[Outcome<a2::poll::Tick>], f: F) -> u64 {
    got.iter()
        .filter(|o| match o {
            Outcome::Item(t) => f(t),
            _ => false,
        })
        .count() as u64
}

fn quantity_of(dec: &str) -> String {
    let mut digits: Vec<u32> = dec.bytes().map(|b| u32::from(b - b'0')).collect();
    let mut hex = Vec::new();
    while digits.iter().any(|d| *d != 0) {
        let mut rest = 0u32;
        for d in digits.iter_mut() {
            let cur = rest * 10 + *d;
            *d = cur / 16;
            rest = cur % 16;
        }
        hex.push(char::from_digit(rest, 16).unwrap_or('0'));
    }
    if hex.is_empty() {
        return "0x0".to_string();
    }
    format!("0x{}", hex.iter().rev().collect::<String>())
}

fn gas(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let expect = expect_chain(f)?;
    let to = need(f, "--to")?;
    let from = f.one("--from").map(str::to_string);
    let value = match (f.one("--value"), f.one("--value-text")) {
        (Some(v), _) => Some(v.to_string()),
        (None, Some(t)) => match eravon_literals::units::units_of(t, eravon_literals::constants::NATIVE_DECIMALS) {
            Ok(wei) => Some(quantity_of(&wei)),
            Err(n) => return ans(Err(a2::fault::Fault::Usage(format!("{n}:--value-text {t:?} 折不成 wei")))),
        },
        (None, None) => None,
    };
    let data = f.one("--data").map(str::to_string);
    let blocks = f.num("--blocks", 3)?;
    ans((|| {
        let (t, w) = (t?, wire(f)?);
        a2::read::chain_guard(&w, &t, expect)?;
        let (estimated, r) = a2::gas::estimate(
            &w,
            &t,
            from.as_deref(),
            &to,
            value.as_deref(),
            data.as_deref(),
        )?;
        let (price, _) = a2::gas::price(&w, &t)?;
        let base_fees = a2::gas::recent_base_fees(&w, &t, blocks)?;
        let facts = a2::gas::Facts { estimated, gas_price: price.clone(), base_fees };
        let mut m = r.rows();
        m.push((
            "baseFees",
            Out::A(facts.base_fees.iter().map(|x| Out::S(x.clone())).collect()),
        ));
        m.push(("estimatedGas", Out::N(facts.estimated)));
        m.push(("gasPrice", Out::S(facts.gas_price.clone())));
        if let Some(v) = a2::gas::verdict(&facts) {
            m.push(("verdict", Out::B(v)));
        }
        Ok(ok(m))
    })())
}
