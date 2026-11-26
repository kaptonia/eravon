use app::door::Doing;
use app::task::{Done, Kind, Outcome};
use eravon_cli::door::Reply;
use std::sync::mpsc::{channel, Receiver};

fn shell() -> app::shell::Shell {
    let ctx = eravon_ui::egui::Context::default();
    super::vault_open();
    app::shell::Shell::boot(eravon_ui::skin::dress(&ctx))
}

fn landed(kind: Kind, serial: u64, result: Result<Done, app::fault::Fault>) -> Outcome {
    Outcome { kind, serial, result, epoch: 0, stale: false, earlier: false, nodes: 0, renoded: false, part: false }
}

fn answer(body: &str) -> Done {
    Done::Door(Reply::Answered { body: body.into() })
}

fn replied(rx: &Receiver<Reply>) -> Option<Reply> {
    rx.try_recv().ok()
}

#[test]
fn a_send_is_answered_by_its_own_landing_and_a_broadcast_never_reads_as_lost() {
    let mut s = shell();
    let (tx, rx) = channel();
    let (go, wait) = channel::<()>();
    let started = s.tasks.spawn(Kind::Door, move || {
        let _ = wait.recv();
        Ok(Done::Door(Reply::Closed))
    });
    assert!(matches!(started, app::task::Spawned::Started));
    let mine = s.tasks.serial_of(Kind::Door).expect("its serial");
    s.door_doing = Some(Doing::Verb { answer: tx, serial: mine });
    s.door_turn(&[landed(Kind::Door, mine + 100, Ok(answer("{\"other\":true}")))]);
    assert!(replied(&rx).is_none() && s.door_doing.is_some(), "another serial's landing does not answer it");
    s.door_turn(&[]);
    assert!(replied(&rx).is_none() && s.door_doing.is_some(), "no landing, no answer");
    s.door_turn(&[landed(Kind::Door, mine, Ok(answer("{\"mine\":true}")))]);
    assert_eq!(replied(&rx), Some(Reply::Answered { body: "{\"mine\":true}".into() }));
    assert!(s.door_doing.is_none(), "answered once");
    let _ = go.send(());
}

#[test]
fn each_landing_of_a_request_has_its_own_reply() {
    let fault = || app::fault::Fault::known(app::fault::Known::Unreachable, "node".to_string());
    let mut s = shell();
    let (tx, rx) = channel();
    s.door_doing = Some(Doing::Verb { answer: tx, serial: 3 });
    s.door_turn(&[landed(Kind::Door, 3, Err(fault()))]);
    assert!(matches!(replied(&rx), Some(Reply::Refused { code, network: true, .. }) if code == "UNREACHABLE"));
    let (tx, rx) = channel();
    s.door_doing = Some(Doing::Verb { answer: tx, serial: 5 });
    s.door_turn(&[landed(Kind::Door, 5, Ok(Done::Path(None)))]);
    assert!(matches!(replied(&rx), Some(Reply::Refused { code, .. }) if code == app::fault::Known::OutcomeLost.as_str()));
}
