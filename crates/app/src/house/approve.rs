use crate::door::Asked;
use crate::fault::{Fault, Known};
use crate::shell::Shell;
use eravon_cli::door::{Reply, Request};

pub struct Held {
    pub asked: Asked,
    pub open_until: Option<u64>,
    pub home: Option<std::path::PathBuf>,
    pub approved: bool,
}

pub struct Told {
    pub verb: String,
    pub serial: Option<u64>,
    pub answer: std::sync::mpsc::Receiver<Reply>,
    pub req: Request,
    pub home: Option<std::path::PathBuf>,
}

fn home_of(shell: &Shell) -> Option<std::path::PathBuf> {
    shell.home.as_ref().map(|h| h.root().to_path_buf())
}

fn signs(q: &Request) -> bool {
    eravon_cli::verbwire::split(&q.verb).is_some_and(|(seat, v)| eravon_cli::schema::signs(seat, v, &q.args))
}

fn wait_of(q: &Request) -> u64 {
    q.one("wait").and_then(|w| w.parse().ok()).unwrap_or(0)
}

pub fn gate(shell: &mut Shell, a: Asked) -> Option<Asked> {
    if !signs(&a.req) || std::mem::take(&mut shell.desk.door_cleared) {
        return Some(a);
    }
    match eravon_seal::machine::turn(eravon_cli::home::anchor_now(&shell.machine), 1) {
        eravon_seal::machine::Turn::Send | eravon_seal::machine::Turn::Nothing => Some(a),
        eravon_seal::machine::Turn::Leave => {
            queue(shell, a);
            None
        }
    }
}


fn queue(shell: &mut Shell, a: Asked) {
    let wait = wait_of(&a.req);
    let open_until = (wait > 0).then(|| shell.desk.pace_now().saturating_add(wait));
    let count = shell.desk.door_held.len() as u64 + 1;
    if open_until.is_none() {
        shell.door_send(&a.answer, Reply::Queued { count });
    }
    let home = home_of(shell);
    shell.desk.door_held.push_back(Held { asked: a, open_until, home, approved: false });
}

pub fn holding(shell: &Shell) -> bool {
    shell.desk.door_held.iter().any(|h| h.open_until.is_some())
}

fn front(shell: &Shell) -> Option<usize> {
    if !shell.unlocked() {
        return None;
    }
    shell.desk.door_held.iter().position(|h| !h.approved)
}

pub fn awaiting(shell: &Shell) -> Option<&Request> {
    front(shell).map(|i| &shell.desk.door_held[i].asked.req)
}

pub fn queued(shell: &Shell) -> usize {
    shell.desk.door_held.len()
}

pub fn awaiting_flags(shell: &Shell) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for (k, _) in awaiting(shell).map(|q| q.args.as_slice()).unwrap_or_default() {
        if !names.contains(k) {
            names.push(k.clone());
        }
    }
    names
}

pub fn approve(shell: &mut Shell) {
    let Some(i) = front(shell) else { return };
    shell.desk.door_held[i].approved = true;
    run_approved(shell);
}

fn run_approved(shell: &mut Shell) {
    if !shell.unlocked() || shell.door_doing.is_some() || shell.tasks.in_flight(crate::task::Kind::Door) {
        return;
    }
    let Some(i) = shell.desk.door_held.iter().position(|h| h.approved) else { return };
    let Some(h) = shell.desk.door_held.remove(i) else { return };
    if h.open_until.is_some() {
        shell.desk.door_cleared = true;
        shell.door_do(h.asked);
        shell.desk.door_cleared = false;
        return;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let req = h.asked.req.clone();
    shell.desk.door_cleared = true;
    shell.door_do(Asked { req: h.asked.req, answer: tx });
    shell.desk.door_cleared = false;
    let serial = match &shell.door_doing {
        Some(crate::door::Doing::Verb { serial, .. }) => Some(*serial),
        None => None,
    };
    shell.desk.door_told.push(Told { verb: req.verb.clone(), serial, answer: rx, req, home: h.home });
}

pub fn told(shell: &mut Shell, landed: &[crate::task::Outcome]) {
    let mut kept = Vec::new();
    for t in std::mem::take(&mut shell.desk.door_told) {
        let got = match t.serial {
            Some(serial) => match landed.iter().find(|o| o.serial == serial) {
                Some(o) => Some(match &o.result {
                    Ok(crate::task::Done::Door(r)) => r.clone(),
                    Err(f) if f.which() == Some(Known::VaultLocked) && !shell.unlocked() => {
                        let (answer, _) = std::sync::mpsc::channel();
                        shell.desk.door_held.push_front(Held { asked: Asked { req: t.req, answer }, open_until: None, home: t.home, approved: true });
                        continue;
                    }
                    Err(f) => crate::door::refused(f),
                    Ok(_) => crate::door::refused(&Fault::known(Known::OutcomeLost, crate::task::Kind::Door.as_str().to_string())),
                }),
                None if shell.tasks.serial_of(crate::task::Kind::Door) != Some(serial) => Some(crate::door::refused(&Fault::known(Known::OutcomeLost, crate::task::Kind::Door.as_str().to_string()))),
                None => None,
            },
            None => match t.answer.try_recv() {
                Ok(r) => Some(r),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(crate::door::refused(&Fault::known(Known::OutcomeLost, crate::task::Kind::Door.as_str().to_string()))),
            },
        };
        match got {
            Some(r) => {
                crate::house::remeasure(shell);
                let (tone, head, detail) = outcome(&t.verb, &r);
                shell.desk.door_lines.push_back(crate::house::shell::Toast { tone, head, next: String::new(), detail });
            }
            None => kept.push(t),
        }
    }
    shell.desk.door_told = kept;
    if shell.unlocked() && !shell.covered() && shell.desk.sh.toasts.is_empty() {
        if let Some(line) = shell.desk.door_lines.pop_front() {
            shell.desk.sh.toast(line);
        }
    }
}

fn outcome(verb: &str, r: &Reply) -> (crate::house::seller::Tone, String, Option<String>) {
    use crate::house::seller::Tone;
    let not_done = |detail: String| (Tone::Bad, crate::house::i18n::filln(crate::house::i18n::Key::DoorApprovedFailed, &[verb]), Some(detail));
    match r {
        Reply::Answered { body } => {
            let ok = eravon_literals::json::read_raw(body.as_bytes()).ok().is_some_and(|v| v.get("ok") == Some(&eravon_literals::json::Raw::Bool(true)));
            if ok {
                (Tone::Ok, crate::house::i18n::filln(crate::house::i18n::Key::DoorApprovedDone, &[verb]), None)
            } else {
                not_done(body.clone())
            }
        }
        Reply::Refused { code, tail, .. } | Reply::Partial { code, tail, .. } => not_done(format!("{code} · {tail}")),
        Reply::Reverted { tx, status } => not_done(format!("reverted {tx} status {status}")),
        Reply::Wrote { entry_id, .. } => not_done(format!("wrote {entry_id}")),
        Reply::Anchored { tx, block } => not_done(format!("anchored {tx} block {block}")),
        Reply::NotYet { tx, .. } => not_done(format!("not yet {tx}")),
        Reply::Unheard { tx, detail } => not_done(format!("unheard {tx} · {detail}")),
        Reply::Voided { tx } => not_done(format!("voided {tx}")),
        Reply::Kit { kit_id, .. } => not_done(format!("kit {kit_id}")),
        Reply::Queued { count } => not_done(format!("queued {count}")),
        Reply::OnDesktop => not_done("on desktop".to_string()),
        Reply::Closed => not_done("closed".to_string()),
    }
}

pub fn decline(shell: &mut Shell) {
    let Some(i) = front(shell) else { return };
    let Some(h) = shell.desk.door_held.remove(i) else { return };
    let f = Fault::known(Known::Denied, h.asked.req.verb.clone()).worded(Some(crate::lang::Key::DoorDeclined), Some(crate::lang::Key::DoorDeclinedNext));
    let r = crate::door::refused(&f);
    shell.door_send(&h.asked.answer, r);
}

pub fn expire(shell: &mut Shell, landed: &[crate::task::Outcome]) {
    let here = home_of(shell);
    let locked = !shell.unlocked();
    let mut gone: Vec<Held> = Vec::new();
    let mut kept = std::collections::VecDeque::new();
    for h in std::mem::take(&mut shell.desk.door_held) {
        if locked || (here.is_some() && h.home == here) {
            kept.push_back(h);
        } else {
            gone.push(h);
        }
    }
    shell.desk.door_held = kept;
    for h in &gone {
        if h.open_until.is_some() {
            shell.door_send(&h.asked.answer, Reply::Closed);
        }
    }
    if !gone.is_empty() {
        dropped_said(shell, gone.len());
    }
    let now = shell.desk.pace_now();
    let count = shell.desk.door_held.len() as u64;
    let due: Vec<usize> = shell.desk.door_held.iter().enumerate().filter(|(_, h)| !h.approved && h.open_until.is_some_and(|t| now >= t)).map(|(i, _)| i).collect();
    for i in due {
        let answer = shell.desk.door_held[i].asked.answer.clone();
        shell.desk.door_held[i].open_until = None;
        shell.door_send(&answer, Reply::Queued { count });
    }
    told(shell, landed);
    run_approved(shell);
}

fn dropped_said(shell: &mut Shell, n: usize) {
    let head = crate::house::i18n::filln(crate::house::i18n::Key::DoorQueueDropped, &[&n.to_string()]);
    shell.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Note, head, next: String::new(), detail: None });
}

pub fn next_due(shell: &Shell) -> Option<u64> {
    shell.desk.door_held.iter().filter_map(|h| h.open_until).min()
}

pub fn closed(shell: &mut Shell) {
    let held = std::mem::take(&mut shell.desk.door_held);
    for h in held {
        if h.open_until.is_some() {
            shell.door_send(&h.asked.answer, Reply::Closed);
        } else {
            shell.desk.door_held.push_back(h);
        }
    }
    shell.desk.door_cleared = false;
}

#[cfg(test)]
mod outcome_tests {
    use super::outcome;
    use crate::house::i18n::{filln, Key};
    use crate::house::seller::Tone;
    use eravon_cli::door::Reply;

    #[test]
    fn done_only_when_the_verbs_own_answer_says_ok() {
        let v = "seller-offer-sign";
        let done = filln(Key::DoorApprovedDone, &[v]);
        let failed = filln(Key::DoorApprovedFailed, &[v]);
        assert_eq!(outcome(v, &Reply::Answered { body: r#"{"ok":true,"sig":"0x01"}"#.into() }), (Tone::Ok, done, None));
        let inner = r#"{"ok":false,"reason":"E_X","detail":"\"ok\":true"}"#;
        assert_eq!(outcome(v, &Reply::Answered { body: inner.into() }), (Tone::Bad, failed.clone(), Some(inner.to_string())));
        assert_eq!(outcome(v, &Reply::Answered { body: "not json".into() }).1, failed);
        let refused = Reply::Refused { code: "LOCKED".into(), tail: "t".into(), network: false, zh: String::new(), en: String::new() };
        assert_eq!(outcome(v, &refused), (Tone::Bad, failed.clone(), Some("LOCKED · t".to_string())));
        for r in [Reply::Closed, Reply::OnDesktop, Reply::Queued { count: 1 }, Reply::Anchored { tx: "0x02".into(), block: 7 }, Reply::Reverted { tx: "0x03".into(), status: 0 }] {
            let (tone, head, detail) = outcome(v, &r);
            assert!(tone == Tone::Bad && head == failed && detail.is_some_and(|d| !d.is_empty()), "{r:?}");
        }
    }
}
