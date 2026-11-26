use crate::args::Args;
use crate::codes::{Exit, Reason};
use crate::out::{self, Answer, Subject};
use crate::seam;
use crate::shared::Flags;
use eravon_literals::json::Out;
use eravon_literals::misuse::Misuse;

fn seats() -> [(&'static str, &'static [&'static str]); 2] {
    [(seam::SEAT_SELLER, seam::SELLER_VERBS), (seam::SEAT_BUYER, seam::BUYER_VERBS)]
}

pub fn split(name: &str) -> Option<(&'static str, &'static str)> {
    seats().into_iter().find_map(|(seat, verbs)| {
        let rest = name.strip_prefix(seat)?.strip_prefix('-')?;
        verbs.iter().find(|v| **v == rest).map(|v| (seat, *v))
    })
}

pub const SHARED: &[&str] = &["data", "now", "writer", "endpoint", "table", "roots", "proxy", "rest-keys", "chain", "logic", "passcode"];
pub const SENDING: &[&str] = &["data", "now", "writer", "endpoint", "table", "roots", "proxy", "rest-keys", "chain", "logic", "passcode", "send", "wait", "gas-limit", "max-fee", "priority-fee"];
pub const LOCAL: &[&str] = &["data", "now", "writer", "endpoint", "table", "roots", "proxy", "rest-keys", "chain", "logic"];

pub fn run(a: &Args) -> Answer {
    let (seat, verb) = split(a.verb()).unwrap_or_else(|| out::misuse(Reason::Args, Subject::Placed { at: 1, bytes: a.verb().len() }, out::Said::NotAVerb));
    let argv: Vec<String> = std::env::args_os().skip(1).map(|x| x.to_string_lossy().into_owned()).collect();
    eravon_literals::misuse::keep_arguments(&argv);
    let argv = keyed(a, argv);
    let feature = match seat {
        seam::SEAT_BUYER => eravon_literals::trace::Feature::V2,
        _ => eravon_literals::trace::Feature::V1,
    };
    eravon_literals::trace::entry("eravon", feature, a.verb());
    eravon_scan::evidence::keep_blocks_on_machine();
    let f = Flags::read(argv.get(1..).unwrap_or_default()).unwrap_or_else(|m| misused(m, &argv));
    unsaid_secrets(&f);
    judge(seat, verb, &f).unwrap_or_else(|m| misused(m, &argv));
    crate::home::run_signs(crate::schema::signs(seat, verb, f.pairs()));
    if let Ok(Some(c)) = f.opt_int("--chain") {
        if eravon_literals::constants::network_of(c).is_some() {
            if !crate::schema::chain_is_own(verb) {
                eravon_literals::constants::pin_run_chain(c);
            }
            eravon_literals::constants::select_chain(c);
        }
    } else if !crate::schema::chain_is_own(verb) {
        crate::home::keep_to_home_network();
    }
    let v = match seat {
        seam::SEAT_SELLER => crate::seller::run(verb, &f),
        _ => crate::buyer::run(verb, &f),
    };
    match v {
        Ok(v) => match crate::home::desktop_holds(&v) {
            Some(f) => answer_of(&crate::answer::refuse(&f)),
            None => answer_of(&v),
        },
        Err(m) => misused(m, &argv),
    }
}

fn keyed(a: &Args, argv: Vec<String>) -> Vec<String> {
    if a.one("key").is_none() && a.one(crate::args::KEY_FILE).is_none() {
        return argv;
    }
    let k = a.key("key");
    let file = format!("--{}", crate::args::KEY_FILE);
    let mut out = Vec::with_capacity(argv.len());
    let mut it = argv.into_iter();
    while let Some(x) = it.next() {
        if x == "--key" || x == file {
            it.next();
            out.extend(["--key".to_string(), eravon::hexfmt::encode(&k)]);
            continue;
        }
        out.push(x);
    }
    out
}

const SECRET_SAID_FROM: usize = 8;

fn unsaid_secrets(f: &Flags) {
    let in_text = |q: String| q.replace('\\', "\\\\").replace('"', "\\\"");
    for name in eravon_literals::schema::SECRET_FLAGS {
        for v in f.many(&format!("--{name}")) {
            let said = format!("({} bytes)", v.len());
            if !v.is_empty() {
                eravon_literals::echo::hide(&in_text(format!("{v:?}")), &in_text(format!("{said:?}")));
            }
            if v.trim().len() >= SECRET_SAID_FROM {
                eravon_literals::echo::hide(&v, &said);
            }
        }
    }
}

const INPUT_FILES: [&str; 6] = ["--block", "--draft-file", "--file", "--listing", "--manifest-file", "--terms"];

fn readable_file(path: &str) -> bool {
    std::fs::File::open(path).and_then(|x| x.metadata()).is_ok_and(|m| m.is_file())
}

pub fn judge(seat: &str, verb: &str, f: &Flags) -> Result<(), Misuse> {
    crate::schema::path_gate(seat, verb, f.pairs())?;
    for (k, v) in f.pairs() {
        if INPUT_FILES.contains(&k.as_str()) && !v.is_empty() && !readable_file(v) {
            return Err(Misuse::unreadable(v, eravon_literals::misuse::UNREADABLE));
        }
    }
    eravon_literals::schema::one_form(f.pairs())?;
    if let Some(p) = f.one("--proxy") {
        if eravon_chain::transport::choice_of(p).is_err() {
            use eravon_literals::misuse::{Reason, Subject, PROXY};
            return Err(Misuse::new(Reason::Args, Subject::flag("--proxy"), PROXY));
        }
    }
    crate::shared::ctx_shape(f)?;
    Ok(())
}

pub fn answer_of(v: &Out) -> Answer {
    answer_bytes(&eravon_literals::json::write(v), eravon_literals::schema::exit_of(v))
}

pub fn answer_bytes(bytes: &[u8], code: u8) -> Answer {
    let exit = match code {
        0 => Exit::Affirmed,
        3 => Exit::Partial,
        4 => Exit::Unanswered,
        _ => Exit::Denied,
    };
    match eravon::json::parse(bytes) {
        Ok(value) if exit == Exit::Unanswered => out::unanswered_as_is(family_form(value)),
        Ok(value) => out::verbatim(exit, family_form(value)),
        Err(_) => out::unanswered(Reason::Desktop, vec![(crate::codes::Key::Detail, out::s(crate::codes::Door::Broken.as_str()))]),
    }
}

fn family_form(v: eravon::json::Value) -> eravon::json::Value {
    use eravon::json::Value;
    let Value::Obj(mut ms) = v else { return v };
    let refused = ms.iter().any(|(k, x)| k == "ok" && matches!(x, Value::Bool(false)));
    if refused && !ms.iter().any(|(k, _)| k == "reason") {
        if let Some(m) = ms.iter_mut().find(|(k, _)| k == "token") {
            m.0 = crate::codes::Key::Reason.as_str().to_string();
        }
    }
    Value::Obj(ms)
}

pub(crate) fn misused(m: Misuse, argv: &[String]) -> ! {
    let (reason, subject) = misuse_parts(&m, argv);
    let said = m.said();
    out::misuse(reason, subject, out::Said::Words(said.zh, said.en))
}

pub(crate) fn misuse_parts(m: &Misuse, argv: &[String]) -> (Reason, Subject) {
    use eravon_literals::misuse::{Reason as R, Subject as S};
    let reason = match m.reason() {
        R::Args => Reason::Args,
        R::Unreadable => Reason::Unreadable,
        R::Key => Reason::Key,
    };
    let subject = match m.subject() {
        S::Program => Subject::Program,
        S::Flags(names) => Subject::Flags(names.iter().map(|n| n.trim_start_matches("--")).collect()),
        S::Placed { at, bytes } => match argv.get(at.wrapping_sub(1)).and_then(|x| x.strip_prefix("--")).and_then(crate::verbs::flag_named) {
            Some(name) => Subject::flag(name),
            None => Subject::Placed { at: *at, bytes: *bytes },
        },
        S::Unplaced { bytes } => Subject::Unplaced { bytes: *bytes },
    };
    (reason, subject)
}

pub fn scrub(bytes: Vec<u8>) -> Vec<u8> {
    eravon_literals::echo::scrub(&bytes)
}



#[cfg(test)]
mod tests {
    #[test]
    fn a_read_refused_by_the_limit_on_questions_leaves_with_four_and_says_the_limit_on_stderr() {
        let over = eravon_chain::fault::Fault::OverAsks(String::new()).name();
        let line = crate::out::over_asks_line();
        let (what, next) = (eravon_glue::book::OVER_ASKS_WORDS, eravon_glue::book::OVER_ASKS_NEXT_WORDS);
        assert!(line.contains(crate::out::say(what.0, what.1)) && line.contains(crate::out::say(next.0, next.1)), "{line}");
        let a = super::answer_bytes(format!(r#"{{"ok":false,"token":"{over}","detail":"eth_getLogs OVER_ASKS 256"}}"#).as_bytes(), 4);
        assert_eq!((a.exit.code(), a.told.as_deref()), (4, Some(line.as_str())));
        let other = super::answer_bytes(br#"{"ok":false,"token":"E_CHAIN_TRANSPORT","detail":"connect refused"}"#, 4);
        assert_eq!(other.exit.code(), 4);
        assert_ne!(other.told.as_deref(), Some(line.as_str()), "another refusal does not say the limit");
    }
}
