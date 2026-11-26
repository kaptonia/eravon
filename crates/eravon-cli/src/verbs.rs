use crate::args::{self, Args};
use crate::verbs_own::{ACCEPTS, ALL_FLAGS, MORE_FLAGS};
pub use crate::verbs_own::{BY_HAND, KEYED_TOO, REPEATED, WRITE};
use crate::codes::{Contract, Exit, Key, Reason};
use crate::out::{self, s, Answer};
use eravon::json::Value;

const KIT_OK: &str = "KIT_OK";

fn flags<'a>(base: &[&'a str], more: &[&'a str]) -> Vec<&'a str> {
    base.iter().chain(more.iter()).copied().collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Need {
    Flag(&'static str),
    OneOf(&'static [&'static str]),
    AnyOf(&'static [&'static str]),
}

impl Need {
    pub fn flags(self) -> Vec<&'static str> {
        match self {
            Need::Flag(f) => vec![f],
            Need::OneOf(fs) | Need::AnyOf(fs) => fs.to_vec(),
        }
    }
}

pub use crate::doorverbs::HOME;

pub const HOME_FLAG: &str = "desktop";

pub fn accepts(verb: &str, homed: bool) -> Option<Vec<&'static str>> {
    if let (true, Some((_, own))) = (homed, HOME.iter().find(|(name, _)| *name == verb)) {
        return Some(std::iter::once(HOME_FLAG).chain(own.iter().map(|(f, _)| *f)).collect());
    }
    ACCEPTS.iter().find(|(name, _, _, _)| *name == verb).map(|(_, shared, own, _)| flags(shared, own))
}

pub fn required(verb: &str, homed: bool) -> Vec<Need> {
    if homed && HOME.iter().any(|(name, _)| *name == verb) {
        return Vec::new();
    }
    let Some((_, shared, _, own)) = ACCEPTS.iter().find(|(name, _, _, _)| *name == verb) else { return Vec::new() };
    let mut out: Vec<Need> = crate::shared::ctx_musts().iter().map(|f| f.trim_start_matches("--")).filter(|f| shared.contains(f)).map(Need::Flag).collect();
    out.extend(own.iter().copied());
    out
}

pub(crate) fn rest_keys_from(a: &Args) {
    let Some(path) = a.one("rest-keys") else { return };
    use eravon_chain::rest::KeysFileBad;
    match eravon_chain::rest::keys_file(std::path::Path::new(path.trim())) {
        Ok(keys) => eravon_chain::rest::hold_file_keys(keys),
        Err(why) => out::misuse(
            Reason::Args,
            out::Subject::flag("rest-keys"),
            match why {
                KeysFileBad::Unreadable => out::Said::Unreadable,
                KeysFileBad::NotUtf8 => out::Said::NotUtf8,
                KeysFileBad::Shape => out::Said::FileShape,
            },
        ),
    }
}

fn known(name: &str) -> bool {
    ALL_FLAGS.contains(&name) || MORE_FLAGS.contains(&name)
}

pub fn run(a: &Args) -> Answer {
    crate::seam();
    let answer = {
        let _action = eravon_chain::action::open(None);
        run_in(a)
    };
    let unkept = eravon_chain::settled::unkept();
    if unkept.is_empty() {
        return answer;
    }
    let line = out::pick(format!("这台机器没留下这一趟读到的:{}", unkept.join(";")), format!("This machine kept none of: {}", unkept.join("; ")));
    let told = match &answer.told {
        Some(t) => format!("{t}\n{line}"),
        None => line,
    };
    answer.with(vec![(Key::Unkept, Value::Arr(unkept.iter().map(|w| s(w)).collect()))]).telling(told)
}

fn run_in(a: &Args) -> Answer {
    if let Some(allowed) = accepts(a.verb(), a.homed()) {
        a.close(&allowed, required(a.verb(), a.homed()));
    }
    if a.homed() {
        return crate::desk::ask(a);
    }
    if a.verb() != "mcp" {
        rest_keys_from(a);
    }
    crate::verbs_own::dispatch(a)
}

pub(crate) fn queued(count: u64) -> Answer {
    out::partial(Reason::Queued, vec![(Key::Count, Value::Int(count))]).telling(out::Said::Queued.text().to_string())
}

pub(crate) fn anchored(tx: &str, block: Option<u64>) -> Answer {
    out::affirmed(vec![(Key::BlockNumber, block.map(Value::Int).unwrap_or(Value::Null)), (Key::Tx, s(tx))])
}

pub(crate) fn tx_failed(tx: &str, status: u64) -> Answer {
    out::denied(Reason::TxStatus, vec![(Key::Tx, s(tx)), (Key::State, Value::Int(status))])
}

pub(crate) fn tx_not_yet(tx: &str, waited: u64) -> Answer {
    out::unanswered(Reason::TxNotYet, vec![(Key::Tx, s(tx)), (Key::Count, Value::Int(waited))])
}

pub(crate) fn tx_void(tx: &str) -> Answer {
    out::denied(Reason::TxVoid, vec![(Key::Tx, s(tx))])
}

pub(crate) fn tx_unheard(tx: &str, detail: &str) -> Answer {
    out::unanswered(Reason::Unreachable, vec![(Key::Detail, s(detail)), (Key::Tx, s(tx))])
}

pub(crate) fn kit_written(dropped: &[String], entries: u64, files: u64, kit_id: &str, path: &str) -> Answer {
    out::affirmed(vec![
        (Key::Dropped, Value::Arr(dropped.iter().map(|x| s(x)).collect())),
        (Key::Entries, Value::Int(entries)),
        (Key::Files, Value::Int(files)),
        (Key::KitId, s(kit_id)),
        (Key::Path, s(path)),
        (Key::State, s(KIT_OK)),
    ])
}

pub(crate) fn entry_id_form(x: &str) -> String {
    let bare = x.strip_prefix("0x").unwrap_or(x);
    match eravon_store::layout::EntryName::parse(bare) {
        Some(n) => format!("0x{}", n.as_str()),
        None => out::misuse(Reason::Args, out::typed(x), out::Said::EntryNotId),
    }
}

fn reasons_listed() -> &'static [Reason] {
    &Reason::ALL
}


pub(crate) fn contract(_a: &Args) -> Answer {
    let m = |k: Contract, v: Value| (k.as_str().to_string(), v);
    let names = |xs: &[&str]| Value::Arr(xs.iter().map(|x| s(x)).collect());
    let exits = Exit::ALL
        .iter()
        .map(|e| Value::Obj(vec![m(Contract::Code, Value::Int(e.code() as u64)), m(Contract::Name, s(e.name()))]))
        .collect();
    let reasons = reasons_listed().iter().map(|r| r.as_str()).chain(crate::codes_own::desk_names()).map(s).collect();
    let stands = args::STANDS_FOR
        .iter()
        .map(|(flag, of)| Value::Obj(vec![m(Contract::Flag, s(flag)), m(Contract::For, s(of))]))
        .collect();
    let verbs = ACCEPTS
        .iter()
        .map(|(name, shared, own, _)| {
            let mut ms = vec![m(Contract::Name, s(name)), m(Contract::Flags, names(&flags(shared, own)))];
            if let Some((_, home)) = HOME.iter().find(|(v, _)| v == name) {
                ms.push(m(Contract::HomeFlags, names(&crate::doorverbs::names(home))));
            }
            ms.push(m(Contract::Required, Value::Arr(required(name, false).iter().map(|n| names(&n.flags())).collect())));
            Value::Obj(ms)
        })
        .collect();
    out::verbatim(
        Exit::Affirmed,
        Value::Obj(vec![
            (Key::Ok.as_str().to_string(), Value::Bool(true)),
            m(Contract::Exits, Value::Arr(exits)),
            m(Contract::Reasons, Value::Arr(reasons)),
            m(Contract::StandsFor, Value::Arr(stands)),
            m(Contract::Verbs, Value::Arr(verbs)),
        ]),
    )
}

pub(crate) fn wrote(entry_id: &str, ledger: &str, seq: u64, written: bool) -> Answer {
    out::affirmed(vec![(Key::EntryId, s(entry_id)), (Key::Ledger, s(ledger)), (Key::Seq, Value::Int(seq)), (Key::Written, Value::Bool(written))])
}

pub fn all_flag_names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = ALL_FLAGS.to_vec();
    v.extend(MORE_FLAGS.iter().copied());
    v
}

pub fn is_known_flag(x: &str) -> bool {
    known(x)
}

pub fn flag_named(x: &str) -> Option<&'static str> {
    ALL_FLAGS.iter().chain(MORE_FLAGS.iter()).find(|f| **f == x).copied()
}

pub const VERBS: [&str; ACCEPTS.len()] = {
    let mut names = [""; ACCEPTS.len()];
    let mut i = 0;
    while i < ACCEPTS.len() {
        names[i] = ACCEPTS[i].0;
        i += 1;
    }
    names
};
