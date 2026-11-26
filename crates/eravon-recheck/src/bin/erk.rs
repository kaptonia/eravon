#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::path::Path;
use std::process::ExitCode;

use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;
use eravon_literals::json::{Out, write};
use eravon_recheck::check::{self, Opts};
use eravon_recheck::dossier::{self, Opened};
use eravon_recheck::fault::Fault;
use eravon_recheck::report;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erk", eravon_recheck::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
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
    m.say("erk");
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

fn ok(mut m: Vec<(&'static str, Out)>) -> Out {
    m.push(("ok", Out::B(true)));
    Out::O(m)
}

fn refused(f: &Fault) -> Out {
    Out::O(vec![("detail", Out::S(f.detail())), ("ok", Out::B(false)), ("token", Out::S(f.name()))])
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_CHAIN: &str = "eravon.chain/1 · 端点与三分法闭集(E_CHAIN_USAGE · E_CHAIN_TRANSPORT · E_CHAIN_ENDPOINT · E_CHAIN_EXECUTION · E_CHAIN_DISAGREE · E_CHAIN_WRONG)";
    const C_READ: &str = "eravon.standard/1 与 A3 · ABI、形闸与日志形闭集(E_A1_FORM · E_A1_ABI · E_A3_FORM)";
    const C_DOSSIER: &str = "eravon.dossier/1 · 卷宗判定闭集(E_DOSSIER_* · E_TRANSACTION),装在 E_R1_DOSSIER 里";
    pub const ALL: &[Verb] = &[
        Verb::new(
            "check",
            &[
                Arg::need("--dossier", T_PATH),
                Arg::opt("--table", T_PATH),
                Arg::opt("--endpoint", T_ARR),
                Arg::opt("--ca-file", T_PATH), Arg::opt("--proxy", T_STR),
                Arg::opt("--registry", T_ARR),
            ],
            &["E_R1_DOSSIER", "E_R1_FORM"],
            &[C_CHAIN, C_READ, C_DOSSIER],
        ),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args.first().map(String::as_str).ok_or_else(|| Misuse::verb(None))?;
    let f = flags(args)?;
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    match verb {
        "check" => check_cmd(&f),
        "schema" => {
            let mut rows = eravon_literals::schema::out_rows(eravon_literals::schema::register(
                &eravon_literals::schema::version("erk"),
                register::ALL,
                &["E_R1_DOSSIER", "E_R1_FORM", "E_CHAIN_USAGE", "E_CHAIN_TRANSPORT", "E_CHAIN_ENDPOINT", "E_CHAIN_EXECUTION", "E_CHAIN_DISAGREE", "E_CHAIN_WRONG"],
            ));
            rows.push(("closed", eravon_recheck::report::closed()));
            Ok(ok(rows))
        }
        _ => Err(Misuse::verb(Some(verb))),
    }
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

fn check_cmd(f: &Flags) -> Result<Out, Misuse> {
    let dir = f.one("--dossier").ok_or_else(|| Misuse::missing("--dossier"))?;
    let meta = std::fs::metadata(dir).map_err(|_| Misuse::unreadable(dir, eravon_literals::misuse::NOT_FOLDER))?;
    if !meta.is_dir() {
        return Err(Misuse::unreadable(dir, eravon_literals::misuse::NOT_FOLDER));
    }
    std::fs::read_dir(dir).map_err(|_| Misuse::unreadable(dir, eravon_literals::misuse::NOT_FOLDER))?;
    let t = table(f)?;
    eravon_scan::bare::keep_reads_in(eravon_scan::bare::machine_if_there());
    eravon_scan::evidence::keep_blocks_on_machine();
    let opts = Opts { registries: f.many("--registry").iter().map(|x| x.to_ascii_lowercase()).collect(), events_from: 0 };
    let case = match dossier::open(Path::new(dir)) {
        Ok(Opened::Refused(k4)) => {
            let said = String::from_utf8_lossy(&k4).into_owned();
            return Ok(refused(&Fault::Dossier(format!("卷宗过不了 K4 的 §2 至 §4:{said}"))));
        }
        Ok(Opened::Whole(c)) => c,
        Err(e) => return Ok(refused(&e)),
    };
    let r = (|| -> Result<Vec<report::Item>, Fault> {
        let t = t?;
        let w = wire(f)?;
        check::run(&w, &t, &case, &opts)
    })();
    Ok(match r {
        Err(e) => refused(&e),
        Ok(items) => ok(vec![
            ("caseId", Out::S(case.case_id.clone())),
            ("counts", report::counts(&items)),
            ("instance", Out::S(case.instance.clone())),
            ("items", Out::A(items.iter().map(report::Item::out).collect())),
            ("registries", Out::A(opts.registries.iter().map(|r| Out::S(r.clone())).collect())),
        ]),
    })
}
