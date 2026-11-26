#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::path::Path;
use std::process::ExitCode;

use eravon_dossier as k4;
use eravon_galeed as k1;
use eravon_galeed::Value;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;

fn argv() -> Option<Vec<String>> {
    std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect()
}

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erd", eravon_literals::trace::Feature::K4, args.first().map(String::as_str).unwrap_or(""));
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
    m.say("erd");
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

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_DOSSIER: &str = "eravon.dossier/1 · 卷宗判定闭集(E_DOSSIER_* · E_TRANSACTION)";
    const C_DOCS: &str = "eravon.terms/1 与 eravon.link/1 · 条款、清单、回执与裁断名闭集";
    const C_CANON: &str = "galeed/1 §10 · 正典形与条目信封闭集";
    pub const ALL: &[Verb] = &[
        Verb::carry("check", &[Arg::need("dossier-dir", T_PATH)], &[C_DOSSIER, C_DOCS, C_CANON]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Value, Misuse> {
    match (args.first().map(String::as_str), args.len()) {
        (Some("check"), 2) => check(Path::new(&args[1])),
        (Some("schema"), 1) => {
            let mut m = k1::schema::value_rows(eravon_literals::schema::register(
                &eravon_literals::schema::version("erd"),
                register::ALL,
                &[],
            ));
            m.push(("ok".into(), Value::Bool(true)));
            Ok(Value::Obj(m))
        }
        _ => Err(eravon_literals::schema::positional_misuse(register::ALL, args)),
    }
}

fn check(dir: &Path) -> Result<Value, Misuse> {
    let meta = std::fs::metadata(dir).map_err(|_| Misuse::unreadable(&dir.to_string_lossy(), eravon_literals::misuse::NOT_FOLDER))?;
    if !meta.is_dir() {
        return Err(Misuse::unreadable(&dir.to_string_lossy(), eravon_literals::misuse::NOT_FOLDER));
    }
    std::fs::read_dir(dir).map_err(|_| Misuse::unreadable(&dir.to_string_lossy(), eravon_literals::misuse::NOT_FOLDER))?;

    let idx_bytes = match k4::files::read_index_bytes(dir) {
        Err(k4::files::Stop::Fault(f)) => return Ok(f.to_value(true)),
        Ok(b) => b,
    };
    let idx = match k4::index::index(&idx_bytes) {
        Err(f) => return Ok(f.to_value(true)),
        Ok(i) => i,
    };
    Ok(match k4::files::files(dir, &idx) {
        Err(k4::files::Stop::Fault(f)) => f.to_value(true),
        Ok(w) => Value::Obj(vec![
            ("attachments".into(), Value::Int(w.attachments as u64)),
            s("buyerConsent", w.buyer_consent.name().into()),
            ("candidate".into(), Value::Int(w.candidate)),
            s("caseId", w.case_id.clone()),
            (
                "entryId".into(),
                match &w.entry_id {
                    Some(x) => Value::Str(x.clone()),
                    None => Value::Null,
                },
            ),
            s("instance", w.instance.clone()),
            ("ok".into(), Value::Bool(true)),
            s("sellerConsent", w.seller_consent.name().into()),
            s("termsHash", w.terms_hash.clone()),
        ]),
    })
}
