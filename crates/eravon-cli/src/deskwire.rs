use std::cell::RefCell;


use crate::out::Answer;

pub fn is_place(verb: &str, flag: &str) -> bool {
    crate::verbwire::split(verb).is_some_and(|(seat, v)| crate::schema::place_flags(seat, v).iter().any(|p| p.trim_start_matches("--") == flag))
}

pub fn values(a: &crate::args::Args, flag: &'static str) -> Vec<String> {
    let given = a.many(flag);
    if crate::doorverbs::kind(a.verb(), flag).is_some_and(crate::doorverbs::Flag::is_path) {
        given.iter().map(|p| whole(p)).collect()
    } else {
        given
    }
}

pub fn secret(flag: &str) -> bool {
    let bare = flag.trim_start_matches('-');
    let bare = bare.strip_suffix(eravon_literals::schema::SECRET_FILE_SUFFIX).unwrap_or(bare);
    eravon_literals::schema::SECRET_FLAGS.contains(&bare)
}

pub fn answered(body: String) -> Answer {
    match eravon_literals::json::read(body.as_bytes()) {
        Ok(v) => match v.get(MISUSED) {
            Some(m) => misused_here(m),
            None => crate::verbwire::answer_bytes(body.as_bytes(), eravon_literals::schema::exit_of_read(&v)),
        },
        Err(_) => crate::verbwire::answer_bytes(b"", 4),
    }
}

const MISUSED: &str = "misused";

fn misuse_body(m: &eravon_literals::misuse::Misuse, argv: &[String]) -> String {
    use crate::out::Subject;
    use eravon_literals::json::Out;
    let (reason, subject) = crate::verbwire::misuse_parts(m, argv);
    let subject = match subject {
        Subject::Program => Out::O(vec![("program", Out::B(true))]),
        Subject::Flags(names) => Out::O(vec![("flags", Out::A(names.into_iter().map(|n| Out::S(n.to_string())).collect()))]),
        Subject::Placed { bytes, .. } | Subject::Endpoint { bytes, .. } | Subject::Unplaced { bytes } => Out::O(vec![("bytes", Out::N(bytes as u64))]),
    };
    let said = m.said();
    let v = Out::O(vec![(MISUSED, Out::O(vec![("reason", Out::S(reason.as_str().to_string())), ("subject", subject), ("zh", Out::S(said.zh.to_string())), ("en", Out::S(said.en.to_string()))]))]);
    String::from_utf8_lossy(&eravon_literals::json::write(&v)).into_owned()
}

fn misused_here(m: &eravon_literals::json::In) -> Answer {
    use crate::codes::Reason;
    use crate::out::Subject;
    let text = |k: &str| m.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let reason = match text("reason").as_deref() {
        Some(r) if r == Reason::Args.as_str() => Reason::Args,
        Some(r) if r == Reason::Unreadable.as_str() => Reason::Unreadable,
        Some(r) if r == Reason::Key.as_str() => Reason::Key,
        _ => return crate::verbwire::answer_bytes(b"", 4),
    };
    let subject = match m.get("subject") {
        Some(s) if s.get("program").is_some() => Subject::Program,
        Some(s) if s.get("flags").is_some() => {
            let names: Option<Vec<&'static str>> = s.get("flags").and_then(|f| f.as_arr()).map(|a| a.iter().map(|n| n.as_str().and_then(crate::verbs::flag_named)).collect()).flatten();
            match names {
                Some(n) if !n.is_empty() => Subject::Flags(n),
                _ => return crate::verbwire::answer_bytes(b"", 4),
            }
        }
        Some(s) => match s.get("bytes") {
            Some(eravon_literals::json::In::Int(n)) if *n >= 0 => Subject::Unplaced { bytes: *n as usize },
            _ => return crate::verbwire::answer_bytes(b"", 4),
        },
        None => return crate::verbwire::answer_bytes(b"", 4),
    };
    let (Some(zh), Some(en)) = (text("zh"), text("en")) else { return crate::verbwire::answer_bytes(b"", 4) };
    crate::out::misuse(reason, subject, crate::out::Said::Words(Box::leak(zh.into_boxed_str()), Box::leak(en.into_boxed_str())))
}

fn whole(p: &str) -> String {
    match std::fs::canonicalize(p) {
        Ok(x) => x.display().to_string(),
        Err(_) => std::path::absolute(p).map(|x| x.display().to_string()).unwrap_or_else(|_| p.to_string()),
    }
}

pub fn door_exit(code: &str) -> eravon_literals::schema::Exit {
    match crate::codes_own::desk_reason(code) {
        Some((reason, _)) => eravon_literals::schema::class(reason.as_str()),
        None => eravon_literals::schema::Exit::Unanswered,
    }
}

pub fn refusal(code: String, tail: String, zh: String, en: String) -> crate::door::Reply {
    use eravon_literals::schema::Exit;
    match door_exit(&code) {
        Exit::Partial => crate::door::Reply::Partial { code, tail, zh, en },
        exit => crate::door::Reply::Refused { code, tail, network: exit == Exit::Unanswered, zh, en },
    }
}

pub fn partial(a: Answer) -> Answer {
    match a.exit {
        crate::codes::Exit::Denied => Answer { exit: crate::codes::Exit::Partial, ..a },
        _ => a,
    }
}

pub struct Lent {
    pub session: eravon_vault::session::Session,
}

std::thread_local! {
    static LENT: RefCell<Option<Lent>> = const { RefCell::new(None) };
}

pub fn lend<T>(l: Lent, f: impl FnOnce() -> T) -> T {
    LENT.with(|x| *x.borrow_mut() = Some(l));
    let out = f();
    LENT.with(|x| *x.borrow_mut() = None);
    out
}

pub const WORDS: &str = "phrase";

fn unsaid(v: eravon_literals::json::Out) -> eravon_literals::json::Out {
    use eravon_literals::json::Out;
    match v {
        Out::O(ms) => Out::O(ms.into_iter().filter(|(k, _)| *k != WORDS).map(|(k, x)| (k, unsaid(x))).collect()),
        Out::A(xs) => Out::A(xs.into_iter().map(unsaid).collect()),
        x => x,
    }
}

pub fn run_for_desktop(req: &crate::door::Request, l: Lent) -> Option<String> {
    let (seat, verb) = crate::verbwire::split(&req.verb)?;
    let mut argv = vec!["--data".to_string(), req.home.clone()];
    for (k, v) in &req.args {
        argv.push(format!("--{k}"));
        argv.push(v.clone());
    }
    let f = crate::shared::Flags::read(&argv).ok()?;
    if let Some(e) = other_chain(seat, verb, &f) {
        return Some(String::from_utf8_lossy(&eravon_literals::json::write(&crate::answer::refuse(&e))).into_owned());
    }
    let ran = lend(l, || match seat {
        crate::seam::SEAT_BUYER => crate::buyer::run(verb, &f),
        _ => crate::seller::run(verb, &f),
    });
    Some(match ran {
        Ok(v) => String::from_utf8_lossy(&eravon_literals::json::write(&unsaid(v))).into_owned(),
        Err(m) => misuse_body(&m, &argv),
    })
}

pub const E_OTHER_CHAIN: &str = "E_DOOR_OTHER_CHAIN";

fn other_chain(seat: &str, verb: &str, f: &crate::shared::Flags) -> Option<crate::fault::Fault> {
    other_chain_of(seat, verb, f.opt_int("--chain").ok().flatten(), eravon_literals::constants::current_network().standard.chain_id)
}


fn other_chain_of(seat: &str, verb: &str, asked: Option<u64>, here: u64) -> Option<crate::fault::Fault> {
    let asked = asked?;
    if crate::schema::chain_is_own(verb) || eravon_literals::constants::network_of(asked).is_none() || asked == here {
        return None;
    }
    let said = match crate::out::lang() {
        crate::out::Lang::Zh => format!("--chain 与这台桌面所在的网不同:经桌面的命令照桌面的网跑;请在桌面的设置 › 网络里换网,或经桌面跑 {seat}-settings-network --chain {asked},或不带 --desktop 直跑"),
        crate::out::Lang::En => format!("--chain names another network than this desktop's: a command through the desktop runs on the desktop's network; switch it in the desktop's Settings › Network or through the desktop with {seat}-settings-network --chain {asked}, or run without --desktop"),
    };
    Some(crate::fault::Fault::Down { layer: "desk", token: E_OTHER_CHAIN.to_string(), why: format!("{said} ({asked} · {here})") })
}

pub(crate) fn lent_session(data: &str) -> Option<eravon_vault::session::Session> {
    LENT.with(|x| {
        let mut l = x.borrow_mut();
        let l = l.as_mut()?;
        (whole(l.session.data()) == whole(data)).then(|| l.session.fork().ok()).flatten()
    })
}

pub(crate) fn lent() -> bool {
    LENT.with(|x| x.borrow().is_some())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_and_places_are_told_by_the_books() {
        let file_form = format!("--{}{}", eravon_literals::schema::SECRET_FLAGS[0], eravon_literals::schema::SECRET_FILE_SUFFIX);
        let other = format!("--{}", eravon_literals::schema::SECRET_FLAGS[1]);
        assert!(secret(&file_form) && secret(&other) && !secret("--id") && !secret("id"));
        assert!(is_place("seller-watch", "data"), "the data root is a place");
        assert!(!is_place("seller-watch", "id"));
        assert!(!is_place("nosuch-verb", "data"));
        for (verb, row) in crate::verbs::HOME {
            let row = crate::doorverbs::names(row);
            assert!(row.iter().all(|f| !secret(f) && *f != "data" && *f != crate::verbs::HOME_FLAG), "{verb}: {row:?}");
        }
    }

    #[test]
    fn a_keys_words_never_go_back_through_the_door() {
        use eravon_literals::json::Out;
        let words = || Out::S("test test test test test test test test test test test junk".into());
        let v = Out::O(vec![("ok", Out::B(true)), (WORDS, words()), ("new", Out::O(vec![("address", Out::S("0x1".into())), (WORDS, words())])), ("each", Out::A(vec![Out::O(vec![(WORDS, words()), ("n", Out::N(1))])]))]);
        let back = String::from_utf8(eravon_literals::json::write(&unsaid(v))).expect("utf-8");
        assert!(!back.contains("phrase") && !back.contains("junk"), "{back}");
        assert!(back.contains("\"address\":\"0x1\"") && back.contains("\"ok\":true") && back.contains("\"n\":1"), "{back}");
    }

    #[test]
    fn a_chain_the_desktop_is_not_on_is_refused_by_name() {
        let mainnet = eravon_literals::constants::NETWORK_MAINNET.standard.chain_id;
        let sepolia = eravon_literals::constants::NETWORK_SEPOLIA.standard.chain_id;
        for seat in [crate::seam::SEAT_SELLER, crate::seam::SEAT_BUYER] {
            for (asked, here) in [(sepolia, mainnet), (mainnet, sepolia)] {
                match super::other_chain_of(seat, "history", Some(asked), here) {
                    Some(crate::fault::Fault::Down { layer, token, why }) => {
                        assert_eq!((layer, token.as_str()), ("desk", super::E_OTHER_CHAIN));
                        let switch = format!("{seat}-settings-network --chain {asked}");
                        assert!(why.contains("--desktop") && why.contains(&switch) && why.contains(&format!("({asked} · {here})")), "{why}");
                    }
                    other => panic!("{seat} {asked} on a desktop on {here}: {other:?}"),
                }
            }
            assert!(super::other_chain_of(seat, "history", Some(mainnet), mainnet).is_none(), "the desktop's own network runs on");
            assert!(super::other_chain_of(seat, "history", Some(31_337), mainnet).is_none(), "a chain no known network lives on runs on, as a direct run leaves it");
            assert!(super::other_chain_of(seat, "history", None, mainnet).is_none(), "no --chain runs on");
        }
    }

    #[test]
    fn every_door_row_with_another_networks_chain_runs_only_where_the_chain_is_its_own() {
        let mainnet = eravon_literals::constants::NETWORK_MAINNET.standard.chain_id;
        let sepolia = eravon_literals::constants::NETWORK_SEPOLIA.standard.chain_id;
        let own: [&str; 6] = [
            "seller-settings-network", "seller-settings-endpoints", "seller-vault-rotate",
            "buyer-settings-network", "buyer-settings-endpoints", "buyer-vault-rotate",
        ];
        let mut ran: Vec<&str> = Vec::new();
        for (name, row) in crate::verbs::HOME {
            let (seat, verb) = crate::verbwire::split(name).expect("a door row is a seat's verb");
            assert!(crate::doorverbs::names(row).contains(&"chain"), "{name}: every door row takes --chain");
            for (asked, here) in [(sepolia, mainnet), (mainnet, sepolia)] {
                match super::other_chain_of(seat, verb, Some(asked), here) {
                    None => ran.push(name),
                    Some(crate::fault::Fault::Down { token, .. }) => assert_eq!(token, super::E_OTHER_CHAIN, "{name}"),
                    Some(other) => panic!("{name}: {other:?}"),
                }
            }
        }
        ran.dedup();
        assert_eq!(ran.len(), own.len(), "{ran:?}");
        for name in own {
            assert!(ran.contains(&name), "{name} runs on another network's --chain: {ran:?}");
        }
    }
}
