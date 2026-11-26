use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;

use crate::house::shell::{Shell, Sync, Toast};

#[derive(Clone, Debug)]
pub struct Tick {
    pub block: u64,
    pub chain_time: u64,
    pub agreement: &'static str,
    pub endpoints: usize,
    pub at: u64,
    pub pass: Option<Box<crate::house::refresh::Pass>>,
}

pub enum Endpoints {
    Unset,
    Unreadable(String, String),
    Bad(a2::fault::Fault),
    Ready(Table),
}

pub fn table(data: &str, addr: &str) -> Endpoints {
    let nodes = match eravon_vault::store::endpoints(data, addr) {
        Ok(n) => n,
        Err(e) => return Endpoints::Unreadable(e.name().to_string(), e.detail()),
    };
    if nodes.is_empty() {
        return Endpoints::Unset;
    }
    match Table::new(nodes) {
        Ok(t) => Endpoints::Ready(t),
        Err(e) => Endpoints::Bad(e),
    }
}


pub fn mark_configured(sh: &mut Shell, n: usize) {
    sh.sync.configured = true;
    sh.sync.endpoints = n;
    if sh.sync.block.is_none() {
        sh.sync.table = crate::house::shell::Table::Unread;
    }
}

pub fn forget(sh: &mut Shell, data: &str, addr: &str, reads: bool) {
    sh.sync.block = None;
    sh.sync.chain_time = None;
    sh.sync.agreement = None;
    sh.sync.read_at = None;
    if !reads {
        sh.sync.table = crate::house::shell::Table::Unread;
        return;
    }
    match table(data, addr) {
        Endpoints::Ready(t) => mark_configured(sh, t.len()),
        Endpoints::Bad(_) => mark_bad(sh),
        Endpoints::Unreadable(..) => {
            sh.sync.configured = false;
            sh.sync.endpoints = 0;
            sh.sync.table = crate::house::shell::Table::Unreadable;
        }
        Endpoints::Unset => {
            sh.sync.configured = false;
            sh.sync.endpoints = 0;
            sh.sync.table = crate::house::shell::Table::Unset;
        }
    }
}

pub fn mark_bad(sh: &mut Shell) {
    sh.sync.configured = false;
    sh.sync.endpoints = 0;
    sh.sync.table = crate::house::shell::Table::Bad;
}

pub type Refresh = Option<(crate::house::refresh::Plan, Option<crate::house::book::Book>)>;

pub fn read(table: Table, refresh: Refresh) -> (Option<Tick>, Option<a2::fault::Fault>) {
    if let Some((plan, prior)) = refresh {
        let pass = crate::house::refresh::run(&table, plan, prior);
        return match (pass.tick.clone(), pass.failed.clone()) {
            (Some(mut tick), failed) => {
                tick.pass = Some(Box::new(pass));
                (Some(tick), failed)
            }
            (None, failed) => (None, failed),
        };
    }
    let t = match Auto::web_roots() {
        Ok(t) => t,
        Err(e) => return (None, Some(e)),
    };
    match a2::clock::now(&t, &table) {
        Ok((now, r)) => (
            Some(Tick {
                block: now.block,
                chain_time: now.timestamp,
                agreement: r.agreement.word(),
                endpoints: r.endpoints,
                at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                pass: None,
            }),
            None,
        ),
        Err(e) => (None, Some(e)),
    }
}

pub fn land(sh: &mut Shell, got: Option<Tick>, failed: Option<&a2::fault::Fault>) -> bool {
    let read = got.is_some();
    if let Some(t) = got {
        head(sh, t);
    }
    if let Some(e) = failed {
        self::failed(sh, e);
        return false;
    }
    read
}

fn head(sh: &mut Shell, t: Tick) {
    sh.sync = Sync {
        table: crate::house::shell::Table::Ready,
        configured: true,
        endpoints: t.endpoints,
        block: Some(t.block),
        chain_time: Some(t.chain_time),
        agreement: Some(t.agreement),
        read_at: Some(t.at),
    };
    if let Some(p) = t.pass {
        crate::house::refresh::land(sh, *p);
    }
}

fn failed(sh: &mut Shell, e: &a2::fault::Fault) {
    if matches!(e, a2::fault::Fault::WrongChain(_)) {
        sh.sync = Sync { table: crate::house::shell::Table::WrongChain, configured: true, endpoints: sh.sync.endpoints, ..Sync::default() };
    }
    sh.toast(fault_toast(e));
}

pub fn fault_toast(e: &a2::fault::Fault) -> Toast {
    Toast {
        tone: crate::house::seller::Tone::Bad,
        head: crate::house::i18n::t(crate::house::i18n::Key::SyncReadFail),
        next: next_step(e),
        detail: Some(format!("{} · {}", e.name(), e.detail())),
    }
}


pub fn said_toast(s: &crate::house::seller::Said) -> Toast {
    match unfold(&s.name, &s.detail) {
        Some(f) => {
            let mut t = fault_toast(&f);
            if t.detail.is_some() {
                t.detail = Some(format!("{} · {}", s.name, s.detail));
            }
            t
        }
        None => Toast { tone: crate::house::seller::Tone::Bad, head: s.detail.clone(), next: String::new(), detail: Some(s.name.clone()) },
    }
}

pub fn chain_class(name: &str) -> Option<a2::fault::Fault> {
    let every = [
        a2::fault::Fault::Transport(String::new()),
        a2::fault::Fault::Endpoint(String::new()),
        a2::fault::Fault::Execution(String::new()),
        a2::fault::Fault::Disagree(String::new()),
        a2::fault::Fault::WrongChain(String::new()),
        a2::fault::Fault::Usage(String::new()),
        a2::fault::Fault::Unread(String::new()),
        a2::fault::Fault::Echo(String::new()),
        a2::fault::Fault::OverAsks(String::new()),
    ];
    every.into_iter().find(|f| f.name() == name)
}

pub fn unfold(name: &str, detail: &str) -> Option<a2::fault::Fault> {
    let (name, detail) = match eravon_cli::fault::down_of(detail) {
        Some((_, token, why)) if chain_class(name).is_none() => (token, why),
        _ => (name, detail),
    };
    chain_class(name).map(|f| match f {
        a2::fault::Fault::Transport(_) => a2::fault::Fault::Transport(detail.to_string()),
        a2::fault::Fault::Endpoint(_) => a2::fault::Fault::Endpoint(detail.to_string()),
        a2::fault::Fault::Execution(_) => a2::fault::Fault::Execution(detail.to_string()),
        a2::fault::Fault::Disagree(_) => a2::fault::Fault::Disagree(detail.to_string()),
        a2::fault::Fault::WrongChain(_) => a2::fault::Fault::WrongChain(detail.to_string()),
        a2::fault::Fault::Usage(_) => a2::fault::Fault::Usage(detail.to_string()),
        a2::fault::Fault::Unread(_) => a2::fault::Fault::Unread(detail.to_string()),
        a2::fault::Fault::Echo(_) => a2::fault::Fault::Echo(detail.to_string()),
        a2::fault::Fault::OverAsks(_) => a2::fault::Fault::OverAsks(detail.to_string()),
    })
}

pub fn said_next(s: &crate::house::seller::Said) -> Option<String> {
    unfold(&s.name, &s.detail).map(|f| next_step(&f))
}

pub fn next_of(s: &crate::house::seller::Said) -> String {
    said_next(s).unwrap_or_default()
}

fn next_step(e: &a2::fault::Fault) -> String {
    crate::house::i18n::t(match e {
        a2::fault::Fault::Transport(d) if d.starts_with("thread ") => crate::house::i18n::Key::SyncNextThread,
        a2::fault::Fault::Transport(_) => crate::house::i18n::Key::SyncNextTransport,
        a2::fault::Fault::Endpoint(_) => crate::house::i18n::Key::SyncNextEndpoint,
        a2::fault::Fault::Execution(_) => execution_step(e),
        a2::fault::Fault::Disagree(_) => crate::house::i18n::Key::SyncNextDisagree,
        a2::fault::Fault::WrongChain(_) => {
            let net = crate::house::i18n::t_named(eravon_literals::constants::current_network().word);
            return crate::fill!(crate::house::i18n::Key::SyncNextWrongChain; net);
        }
        a2::fault::Fault::Usage(_) => crate::house::i18n::Key::SyncNextUsage,
        a2::fault::Fault::Unread(_) => crate::house::i18n::Key::SyncNextUnread,
        a2::fault::Fault::Echo(_) => crate::house::i18n::Key::SyncNextEcho,
        a2::fault::Fault::OverAsks(_) => crate::house::i18n::Key::FaultNextOverAsks,
    })
}

fn execution_step(e: &a2::fault::Fault) -> crate::house::i18n::Key {
    if a2::fault::reverted(e) { crate::house::i18n::Key::SyncNextExecution } else { crate::house::i18n::Key::SyncNextRefused }
}


pub fn unreadable(sh: &mut Shell, name: &str, detail: &str) {
    sh.sync = Sync { configured: false, table: crate::house::shell::Table::Unreadable, ..Sync::default() };
    sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: crate::house::i18n::t(crate::house::i18n::Key::SyncSettingsUnread), next: String::new(), detail: Some(format!("{name} · {detail}")) });
}

pub fn unconfigured(sh: &mut Shell) {
    sh.sync = Sync { configured: false, table: crate::house::shell::Table::Unset, ..Sync::default() };
    sh.toast(Toast {
        tone: crate::house::seller::Tone::Bad,
        head: crate::house::i18n::t(crate::house::i18n::Key::SyncNoEndpoints),
        next: crate::house::i18n::t(crate::house::i18n::Key::SyncAddNodes),
        detail: None,
    });
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Say {
    Doing { kind: crate::task::Kind, others: usize },
    Syncing,
    Unconfigured,
    Unreadable,
    BadTable,
    WrongChain,
    Failed,
    ReadAt { at: u64, now: u64, stale: bool },
    Never,
}

pub fn say(table: crate::house::shell::Table, doing: &[crate::task::Kind], syncing: bool, failed: bool, read_at: Option<u64>, now: u64) -> Say {
    use crate::house::shell::Table as T;
    if let Some(k) = doing.last() {
        return Say::Doing { kind: *k, others: doing.len() - 1 };
    }
    if syncing {
        return Say::Syncing;
    }
    match table {
        T::Unset => return Say::Unconfigured,
        T::Unreadable => return Say::Unreadable,
        T::Bad => return Say::BadTable,
        T::WrongChain => return Say::WrongChain,
        T::Unread | T::Ready => {}
    }
    if failed {
        return Say::Failed;
    }
    last(read_at, now)
}

fn last(read_at: Option<u64>, now: u64) -> Say {
    match read_at {
        Some(at) => Say::ReadAt { at, now, stale: crate::house::book::stale(at, now) },
        None => Say::Never,
    }
}


pub const fn voice_word(v: eravon_ui::rail::Voice) -> &'static str {
    match v {
        eravon_ui::rail::Voice::Quiet => "quiet",
        eravon_ui::rail::Voice::Task => "task",
        eravon_ui::rail::Voice::Bad => "bad",
    }
}

pub fn say_words(lang: crate::house::i18n::Lang, s: &Say) -> (String, eravon_ui::rail::Voice) {
    use eravon_ui::rail::Voice;
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    match s {
        Say::Doing { kind, others } => {
            let name = crate::house::i18n::get_named(lang, &format!("task.{}", kind.as_str())).0;
            let words = if *others > 0 { crate::fill_in!(lang, crate::house::i18n::Key::SyncDoingMore; name, (others + 1).to_string()) } else { crate::fill_in!(lang, crate::house::i18n::Key::SyncDoing; name) };
            (words, Voice::Task)
        }
        Say::Syncing => (t(crate::house::i18n::Key::SyncSyncing), Voice::Quiet),
        Say::Unconfigured => (t(crate::house::i18n::Key::ErrEP4NOENDPOINT), Voice::Bad),
        Say::Unreadable => (t(crate::house::i18n::Key::SyncSettingsUnread), Voice::Bad),
        Say::BadTable => (t(crate::house::i18n::Key::SyncBad), Voice::Bad),
        Say::WrongChain => (t(crate::house::i18n::Key::SyncWrongChain), Voice::Bad),
        Say::Failed => (t(crate::house::i18n::Key::SyncFailed), Voice::Bad),
        Say::ReadAt { at, now, stale } => {
            let words = crate::fill_in!(lang, crate::house::i18n::Key::SyncLastAt; eravon_orders::when::clock_or_day(*at, *now));
            let _ = stale;
            (words, Voice::Quiet)
        }
        Say::Never => (t(crate::house::i18n::Key::SyncNotYet), Voice::Quiet),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_estimate_is_worded_by_whether_it_reverted() {
        let fail = crate::house::i18n::t(crate::house::i18n::Key::SyncNextExecution);
        let refused = crate::house::i18n::t(crate::house::i18n::Key::SyncNextRefused);
        let endpoint = crate::house::i18n::t(crate::house::i18n::Key::SyncNextEndpoint);
        let rows = [
            (r#"{"code":3,"message":"execution reverted: no"}"#, &fail),
            (r#"{"code":-32000,"message":"execution reverted"}"#, &fail),
            (r#"{"code":-32005,"message":"rate limit exceeded"}"#, &endpoint),
            (r#"{"code":-32000,"message":"header not found"}"#, &refused),
        ];
        for (error, want) in rows {
            let e = a2::fault::node_refusal(error);
            let detail = e.detail();
            assert_eq!(&fault_toast(&e).next, want, "{detail}");
            let folded = crate::house::seller::Said::bad(e.name(), &detail);
            assert_eq!(&next_of(&folded), want, "{detail}");
            let down = crate::house::seller::Said::bad("E_V_DOWN", &format!("A2 说 {}:{detail}", e.name()));
            assert_eq!(&next_of(&down), want, "{detail}");
        }
        assert_eq!(next_of(&crate::house::seller::Said::bad("E_V_ARG", "金额不对")), "");
    }

    #[test]
    fn every_chain_class_comes_back_from_its_name() {
        let every = [
            a2::fault::Fault::Transport("x".into()),
            a2::fault::Fault::Endpoint("x".into()),
            a2::fault::Fault::Execution("x".into()),
            a2::fault::Fault::Disagree("x".into()),
            a2::fault::Fault::WrongChain("x".into()),
            a2::fault::Fault::Usage("x".into()),
            a2::fault::Fault::Unread("x".into()),
        ];
        let mut names = std::collections::BTreeSet::new();
        for f in &every {
            assert!(names.insert(f.name()), "{} 两类同名", f.name());
            let back = chain_class(f.name()).expect("链层的名该映得回");
            assert_eq!(std::mem::discriminant(&back), std::mem::discriminant(f));
        }
        let wrong = crate::house::seller::Said::bad("E_CHAIN_WRONG", "节点在链 1,本构建钉在链 11155111");
        let net = crate::house::i18n::t_named(eravon_literals::constants::current_network().word);
        assert!(said_toast(&wrong).next.contains(&net), "异链折成 Said 之后,第二行仍写明换回钉住网络");
        assert_eq!(said_next(&wrong).as_deref(), Some(said_toast(&wrong).next.as_str()));
    }

    #[test]
    fn no_endpoints_is_an_error_toast_in_human_words() {
        let mut sh = Shell::default();
        unconfigured(&mut sh);
        let t = sh.toasts.first().expect("该有一条");
        assert_eq!(t.tone, crate::house::seller::Tone::Bad);
        assert!(t.next.contains("设置 › 网络") && !t.next.contains("settings"), "{}", t.next);
    }
}

#[cfg(test)]
mod say_tests {
    use super::{say, Say};
    use crate::task::Kind;
    use crate::house::shell::Table;

    const NOW: u64 = 1_800_000_000;

    #[test]
    fn each_state_is_the_first_that_holds() {
        let at = Some(NOW - 30);
        assert_eq!(say(Table::Ready, &[Kind::Vault, Kind::Export], true, true, at, NOW), Say::Doing { kind: Kind::Export, others: 1 });
        assert_eq!(say(Table::Ready, &[], true, true, at, NOW), Say::Syncing);
        assert_eq!(say(Table::Unset, &[], false, true, at, NOW), Say::Unconfigured);
        assert_eq!(say(Table::Bad, &[], false, true, at, NOW), Say::BadTable);
        assert_eq!(say(Table::WrongChain, &[], false, true, at, NOW), Say::WrongChain);
        assert_eq!(say(Table::Ready, &[], false, true, at, NOW), Say::Failed);
        assert_eq!(say(Table::Ready, &[], false, false, at, NOW), Say::ReadAt { at: NOW - 30, now: NOW, stale: false });
        assert_eq!(say(Table::Unread, &[], false, false, None, NOW), Say::Never);
    }

    #[test]
    fn the_last_sync_is_stale_past_ten_minutes() {
        let line = eravon_literals::constants::READ_STALE_SECS;
        assert_eq!(say(Table::Ready, &[], false, false, Some(NOW - line), NOW), Say::ReadAt { at: NOW - line, now: NOW, stale: false });
        assert_eq!(say(Table::Ready, &[], false, false, Some(NOW - line - 1), NOW), Say::ReadAt { at: NOW - line - 1, now: NOW, stale: true });
    }
}
