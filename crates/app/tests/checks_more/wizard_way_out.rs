use app::action::{apply, apply_settled, Action, Applied};
use eravon_ui::egui;

const W: f32 = 1180.0;
const H: f32 = 760.0;

fn home_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eravon-way-out-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_way_out_appears_once_a_passcode_and_an_identity_are_here() {
    if super::alone_in(module_path!(), "the_way_out_appears_once_a_passcode_and_an_identity_are_here") {
        return;
    }
    super::vault_open();
    let m = app::home::machine_dir().expect("the machine directory");
    let _ = std::fs::remove_file(m.join(app::places::keybox_file()));
    app::keybox::lock();
    let ctx = egui::Context::default();
    let s = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let dir = home_dir("first-run");
    let (s, r) = app::window::probe_wizard_way_out(
        &ctx,
        s,
        |sh, phase| {
            if phase == 1 {
                assert!(matches!(apply_settled(sh, Action::SetPin { pin: "27618394".into(), again: "27618394".into() }), Applied::Unlocked));
            } else {
                super::seated(sh, &dir);
                apply(sh, Action::ReadIdentities);
                assert!(matches!(apply(sh, Action::OpenHome { root: dir.display().to_string() }), Applied::Homed { .. }));
            }
        },
        W,
        H,
    );
    assert_eq!(r.offered, [false, false, true], "no way out before a passcode and an identity, one after: {r:?}");
    assert_eq!(r.exit_drawn, [false, false, true], "\"Exit setup\" drawn only then: {r:?}");
    assert!(r.left_by_click && !r.still_open && r.seat_has_key_after, "\"Exit setup\" clicked leaves, onto the seat with a key: {r:?}");
    assert!(s.home.is_some(), "a home");
    assert_eq!(app::backup::count_now().ok(), Some(0), "leaving writes no record");
    assert!(r.reopened && r.reopened_offered && r.reopened_step == "Network", "after a restart the wizard opens at the network step, with its way out: {r:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

