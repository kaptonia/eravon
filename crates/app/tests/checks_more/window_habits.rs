use app::action::{apply, Action, Applied};
use app::nav::{Place, Step};
use eravon_ui::egui;

const W: f32 = 1180.0;
const H: f32 = 760.0;

fn shell(ctx: &egui::Context) -> app::shell::Shell {
    super::vault_open();
    app::shell::Shell::boot(eravon_ui::skin::dress(ctx))
}

fn shell_with_ledger(ctx: &egui::Context, name: &str) -> app::shell::Shell {
    let mut s = shell(ctx);
    let dir = std::env::temp_dir().join(format!("eravon-habits-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    super::seated(&mut s, &dir);
    answers!(apply(&mut s, Action::OpenHome { root: dir.display().to_string() }), Applied::Homed { .. });
    s
}

#[test]
fn back_from_the_words_check_returns_to_the_same_words() {
    if super::alone_in(module_path!(), "back_from_the_words_check_returns_to_the_same_words") {
        return;
    }
    let ctx = egui::Context::default();
    let mut s = shell(&ctx);
    answers!(apply(&mut s, Action::NewIdentity), Applied::FreshWords);
    let (_, r) = app::window::probe_wizard_back(&ctx, s, Place::Home, W, H);
    assert!(r.pressed_on_check && r.pressed_on_words, "back drawn on both pages: {r:?}");
    assert_eq!((r.step_after_check.as_str(), r.checking_after, r.same_words), ("Key", false, true), "{r:?}");
    assert_eq!(r.step_after_words, "Pin", "{r:?}");
}

#[test]
fn the_language_switches_on_the_wizard_and_the_gate_keeps_it() {
    if super::alone_in(module_path!(), "the_language_switches_on_the_wizard_and_the_gate_keeps_it") {
        return;
    }
    super::vault_open();
    let m = app::home::machine_dir().expect("the machine directory");
    let _ = std::fs::remove_file(m.join(app::places::keybox_file()));
    app::keybox::lock();
    let ctx = egui::Context::default();
    let s = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let (mut s, wizard) = app::window::probe_lang_pick(&ctx, s, Place::Home, false, Step::Pin, W, H);
    assert!(wizard.drawn && wizard.after != wizard.before && wizard.machine_after == wizard.after && wizard.still_up, "{wizard:?}");
    answers!(app::action::apply_settled(&mut s, Action::SetPin { pin: "27618394".into(), again: "27618394".into() }), Applied::Unlocked);
    apply(&mut s, Action::Lock);
    let (_, gate) = app::window::probe_lang_pick(&ctx, s, Place::Home, true, Step::Pin, W, H);
    assert!(!gate.drawn && gate.after == gate.before && gate.before == wizard.after && gate.still_up, "{gate:?}");
}

#[test]
fn typing_under_an_overlay_never_lands_in_the_passcode_row() {
    if super::alone_in(module_path!(), "typing_under_an_overlay_never_lands_in_the_passcode_row") {
        return;
    }
    let ctx = egui::Context::default();
    super::vault_open();
    app::keybox::lock();
    let s = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let (_, r) = app::window::probe_typed_under(&ctx, s, W, H);
    assert_eq!(r.bare, 3, "with nothing over it the row takes what is typed: {r:?}");
    assert!(r.sheet_up && r.pressed, "the sheet was up and pressed: {r:?}");
    assert_eq!(r.under, 0, "under the sheet the row takes nothing: {r:?}");
    assert!(r.locked, "{r:?}");
}

#[test]
fn the_passcode_cells_sit_in_the_middle_of_the_gates_group() {
    if super::alone_in(module_path!(), "the_passcode_cells_sit_in_the_middle_of_the_gates_group") {
        return;
    }
    let ctx = egui::Context::default();
    super::vault_open();
    app::keybox::lock();
    for lang in app::lang::Lang::ALL {
        app::lang::set(lang);
        let (_, g) = app::window::probe_gate_column(&ctx, app::shell::Shell::boot(eravon_ui::skin::dress(&ctx)), None, W, H);
        let above = g.cells.top() - g.title_ink_bottom;
        let below = g.forgot.top() - g.cells.bottom();
        assert!(above > 10.0 && (above - below).abs() < 0.5, "{lang:?}: the key stands as far under the cells as the title's strokes stand over them: {above} and {below} ({g:?})");
        let (top, bottom) = (g.group_top, g.window_h - g.forgot.bottom());
        assert!((top - bottom).abs() < 1.5, "{lang:?}: the group is centred in the window: {top} over, {bottom} under");
        let (_, g) = app::window::probe_gate_column(&ctx, app::shell::Shell::boot(eravon_ui::skin::dress(&ctx)), Some("refused"), W, H);
        let said = g.said.expect("the refusal is said in its box");
        let above = g.cells.top() - g.title_ink_bottom;
        assert!((said.top() - g.cells.bottom() - above).abs() < 0.5 && (g.forgot.top() - said.bottom() - above).abs() < 0.5, "{lang:?}: {above}, then {} and {} ({g:?})", said.top() - g.cells.bottom(), g.forgot.top() - said.bottom());
    }
    app::lang::set(app::lang::Lang::Zh);
}

#[test]
fn the_busy_words_under_the_passcode_cells_are_centred_under_them() {
    if super::alone_in(module_path!(), "the_busy_words_under_the_passcode_cells_are_centred_under_them") {
        return;
    }
    let ctx = egui::Context::default();
    super::vault_open();
    app::keybox::lock();
    let lead = 4.25 * 2.0 + 6.0;
    for lang in app::lang::Lang::ALL {
        app::lang::set(lang);
        for wizard in [false, true] {
            let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
            let (hold, wait) = std::sync::mpsc::channel::<()>();
            let _ = shell.tasks.spawn(app::task::Kind::Vault, move || {
                let _ = wait.recv();
                Err(app::fault::Fault::known(app::fault::Known::FileMissing, ""))
            });
            let (_, cells, busy) = app::window::probe_busy_under_cells(&ctx, shell, wizard, W, H);
            drop(hold);
            let busy = busy.unwrap_or_else(|| panic!("{lang:?}, wizard {wizard}: the busy words are drawn"));
            assert!(cells.is_positive(), "{lang:?}, wizard {wizard}: the passcode cells are drawn");
            let middle = (busy.left() - lead + busy.right()) / 2.0;
            assert!((middle - cells.center().x).abs() < 1.0, "{lang:?}, wizard {wizard}: the busy line's middle {middle} is the cells' middle {}", cells.center().x);
            assert!(busy.top() > cells.bottom(), "{lang:?}, wizard {wizard}: the busy words are under the cells");
        }
    }
    app::lang::set(app::lang::Lang::Zh);
}

#[test]
fn a_page_set_from_outside_the_window_is_shown_next() {
    if super::alone_in(module_path!(), "a_page_set_from_outside_the_window_is_shown_next") {
        return;
    }
    let ctx = egui::Context::default();
    let s = shell_with_ledger(&ctx, "show");
    let (_, before, after, page) = app::window::probe_show_follow(&ctx, s, Place::Home, app::nav::Page::About, W, H);
    assert_ne!(before, after, "the window moved");
    assert_eq!(page, "About", "and left the shell's page where it was set");
}

#[test]
fn the_ui_kit_files_name_no_page_of_this_product() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry = std::fs::read_to_string(root.join("family-components.json")).expect("the component registry");
    let at = registry.find("\"id\": \"ui-kit\"").expect("the ui-kit component");
    let home = &registry[at..];
    let home = &home[home.find("\"home\"").expect("its home")..];
    let home = &home[..home.find(']').expect("its home's end")];
    let files: Vec<&str> = home.split('"').filter(|x| x.starts_with("crates/app/src/") && x.ends_with(".rs")).collect();
    assert!(files.contains(&"crates/app/src/window/chrome.rs") && files.contains(&"crates/app/src/window/mod.rs"), "{files:?}");
    const MEETS: [&str; 2] = ["crates/app/src/window/skeleton.rs", "crates/app/src/window/drive.rs"];
    for m in MEETS {
        assert!(files.contains(&m), "{m} is in the kit's home: {files:?}");
    }
    for f in files.into_iter().filter(|f| !MEETS.contains(f)) {
        let code = super::code_only(&std::fs::read_to_string(root.join(f)).expect(f));
        for name in ["Route::", "Place::", "View::", "Section::", "Stack::", "Page::", "tab::", "Step::", "nav::rail(", "nav::history_of", "nav::stack_of"] {
            let named = code.match_indices(name).any(|(i, _)| !code[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_'));
            assert!(!named, "{f} names `{name}`");
        }
    }
}

