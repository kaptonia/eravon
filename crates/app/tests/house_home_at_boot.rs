#[test]
fn the_window_start_takes_the_platform_home() {
    let base = std::env::temp_dir().join(format!("evhb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let machine = base.join("m");
    std::fs::create_dir_all(&machine).expect("the machine directory");
    app::places::set(app::places::Places { key_account: app::places::ACCOUNT.to_string(), machine_dir: Some(machine), user_home: None });
    app::keybox::set_pin("27618394").expect("the vault opens");
    let ctx = eravon_ui::egui::Context::default();
    let shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let home = shell.desk.sh.seller.field.home.clone();
    drop(shell);
    let _ = std::fs::remove_dir_all(&base);
    assert!(!home.is_empty(), "the started shell holds a home directory");
    assert_eq!(home, app::house::seam::user_home(""), "the platform's home");
    let src = |rel: &str| std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).expect("source");
    let window = src("src/window/drive.rs");
    let boot = window.find("Shell::boot(dressed)").expect("the window starts through Shell::boot");
    assert!(boot < window.find("action::start(&mut shell)").expect("then the family's start"), "Shell::boot comes first");
    let drive = src("src/house/drive.rs");
    let at = drive.find("fn boot(r: &Run<'_>)").expect("the drive door's start");
    let body = &drive[at..at + drive[at..].find("\n}").expect("end")];
    assert!(body.contains("boot_bare()") && body.contains("if !r.home.is_empty()"), "the drive door starts the same shell and overrides the home only when injected: {body}");
}
