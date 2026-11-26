use crate::args::Args;
use crate::codes::{Door, Key, Reason};
use crate::out::{self, s, Answer, Said};
use crate::verbs;
use crate::door::{Reply, Request};

pub fn ask(a: &Args) -> Answer {
    let home = whole_dir(&a.need(verbs::HOME_FLAG));
    let req = Request { verb: a.verb().to_string(), home: home.display().to_string(), args: read_args(a) };
    if let Some((seat, verb)) = crate::verbwire::split(a.verb()) {
        let mut argv = vec!["--data".to_string(), req.home.clone()];
        argv.extend(req.args.iter().flat_map(|(k, v)| [format!("--{k}"), v.clone()]));
        let as_given: Vec<String> = std::iter::once(a.verb().to_string()).chain(argv.iter().cloned()).collect();
        let f = crate::shared::Flags::read(&argv).unwrap_or_else(|m| crate::verbwire::misused(m, &as_given));
        crate::verbwire::judge(seat, verb, &f).unwrap_or_else(|m| crate::verbwire::misused(m, &as_given));
    }
    let machine = match eravon_os::machine::here() {
        Ok((m, _)) => m,
        Err(_) => return absent(Door::NoMachine, Said::NoMachine),
    };
    let place = crate::door::place(&machine, &home);
    let mut door = match eravon_os::door::connect(&place) {
        Ok(d) => d,
        Err(e) => {
            return match e.kind() {
                std::io::ErrorKind::NotFound if writes_alone(&machine) => absent(Door::Alone, Said::DoorAlone),
                std::io::ErrorKind::NotFound => match crate::sealed::this_machines(&home, &machine) {
                    Err(t) => crate::sealed::refused(&t),
                    Ok(()) => absent(Door::NotOpen, Said::DesktopLocked),
                },
                std::io::ErrorKind::InvalidInput => absent(Door::PathTooLong, Said::DoorPathLong),
                std::io::ErrorKind::PermissionDenied => absent(Door::NotYou, Said::DoorNotYou),
                _ => absent(Door::Broken, Said::DoorBroken),
            };
        }
    };
    if crate::door::put(&mut door, &req.to_bytes()).is_err() {
        return absent(Door::Broken, Said::DoorBroken);
    }
    match crate::door::take(&mut door).ok().and_then(|b| Reply::of_bytes(&b)) {
        Some(r) => answer_of(r),
        None => absent(Door::Broken, Said::DoorBroken),
    }
}

pub(crate) fn writes_alone(machine: &std::path::Path) -> bool {
    !matches!(eravon_seal::credential::find(machine), Err(t) if t.code == eravon_seal::Code::CredentialMissing)
}

fn read_args(a: &Args) -> Vec<(String, String)> {
    use crate::doorverbs::Flag;
    let own = verbs::accepts(a.verb(), true).unwrap_or_default();
    let mut out = Vec::new();
    for flag in own.into_iter().filter(|f| *f != verbs::HOME_FLAG) {
        let values: Vec<String> = match crate::doorverbs::kind(a.verb(), flag).unwrap_or(Flag::Value) {
            _ if crate::verbwire::split(a.verb()).is_some() => crate::deskwire::values(a, flag),
            Flag::File => vec![whole_file(&a.need(flag))],
            Flag::Out => vec![whole_place(&a.need(flag))],
            Flag::Place => a.one(flag).map(|p| whole_place(&p)).into_iter().collect(),
            Flag::Value => match (a.verb(), flag) {
                (_, "window-from" | "window-to" | "effective") => a.within_ceiling(flag).map(|n| n.to_string()).into_iter().collect(),
                ("kit-export", "entry") => a.many(flag).iter().map(|x| verbs::entry_id_form(x)).collect(),
                _ => a.one(flag).into_iter().collect(),
            },
        };
        out.extend(values.into_iter().map(|v| (flag.to_string(), v)));
    }
    out
}

fn whole_dir(path: &str) -> std::path::PathBuf {
    match std::fs::canonicalize(path) {
        Ok(p) if p.is_dir() => p,
        _ => out::misuse(Reason::Unreadable, out::typed(path), Said::Unreadable),
    }
}

fn whole_file(path: &str) -> String {
    match std::fs::canonicalize(path) {
        Ok(p) if p.is_file() && std::fs::File::open(&p).is_ok() => p.display().to_string(),
        _ => out::misuse(Reason::Unreadable, out::typed(path), Said::Unreadable),
    }
}

fn whole_place(path: &str) -> String {
    match std::path::absolute(path) {
        Ok(p) => p.display().to_string(),
        Err(_) => out::misuse(Reason::Unreadable, out::typed(path), Said::Unreadable),
    }
}

fn absent(why: Door, said: Said) -> Answer {
    out::unanswered(Reason::Desktop, vec![(Key::Detail, s(why.as_str()))]).telling(said.text().to_string())
}

fn answer_of(r: Reply) -> Answer {
    match r {
        Reply::Wrote { entry_id, ledger, seq } => verbs::wrote(&entry_id, &ledger, seq, true),
        Reply::Answered { body } => crate::deskwire::answered(body),
        Reply::Anchored { tx, block } => verbs::anchored(&tx, Some(block)),
        Reply::Reverted { tx, status } => verbs::tx_failed(&tx, status),
        Reply::NotYet { tx, waited } => verbs::tx_not_yet(&tx, waited),
        Reply::Unheard { tx, detail } => verbs::tx_unheard(&tx, &detail),
        Reply::Voided { tx } => verbs::tx_void(&tx),
        Reply::Kit { kit_id, path, entries, files, dropped } => verbs::kit_written(&dropped, entries, files, &kit_id, &path),
        Reply::Queued { count } => verbs::queued(count),
        Reply::OnDesktop => out::denied(Reason::OnDesktop, vec![]).telling(Said::OnDesktop.text().to_string()),
        Reply::Refused { code, tail, network, zh, en, .. } => match crate::codes_own::desk_reason(&code) {
            Some((reason, token)) => {
                let mut members = Vec::new();
                if let Some(t) = token.map(str::to_string).or_else(|| (reason.as_str() != code).then(|| code.clone())) {
                    members.push((Key::Token, s(&t)));
                }
                members.push((Key::Detail, s(&tail)));
                let a = if network { out::unanswered(reason, members) } else { out::denied(reason, members) };
                a.telling(out::pick(zh, en))
            }
            None => absent(Door::Broken, Said::DoorBroken),
        },
        Reply::Partial { code, tail, zh, en } => crate::deskwire::partial(answer_of(Reply::Refused { code, tail, network: false, zh, en })),
        Reply::Closed => absent(Door::Closed, Said::DesktopClosed),
    }
}
