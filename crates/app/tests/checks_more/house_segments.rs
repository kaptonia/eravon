use app::house::i18n::{get, Key, Lang};
use app::house::seller::Page;
use app::house::shell::Seat;
use eravon_ui::egui;

fn seller_shell(tag: &str) -> app::shell::Shell {
    let root = std::env::temp_dir().join(format!("ev-seg-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (data, home) = (root.join("data"), root.join("home"));
    std::fs::create_dir_all(&data).expect("data root");
    std::fs::create_dir_all(&home).expect("home");
    let (pin, now) = ("Qw7Rt2Lp", "1758412800");
    let data_s = data.display().to_string();
    let argv: Vec<String> = ["--data", &data_s, "--writer", "j", "--now", now, "--passcode", pin, "--phrase", "test test test test test test test test test test test junk"].iter().map(|x| x.to_string()).collect();
    let flags = eravon_cli::shared::Flags::read(&argv).expect("the init's flags");
    let made = eravon_cli::seller::run("vault-init", &flags).expect("vault-init runs");
    let made = String::from_utf8_lossy(&eravon_literals::json::write(&made)).into_owned();
    assert!(made.contains("\"ok\":true"), "vault-init: {made}");
    let script = r#"{"steps":[{"do":"set-way","to":"http://127.0.0.1:9"}]}"#;
    let r = app::house::drive::Run { data: &data_s, lang: Lang::Zh, budget_ms: 4000, now: 1_758_412_800, door_tallied: Default::default(), registries: Vec::new(), home: home.display().to_string(), passcode: Some(pin.to_string()) };
    match app::house::drive::run(script.as_bytes(), &r) {
        Ok(d) => d.shell,
        Err(e) => panic!("the drive run: {e}"),
    }
}

#[test]
fn the_order_lists_segment_keeps_the_page() {
    if super::alone_in(module_path!(), "the_order_lists_segment_keeps_the_page") {
        return;
    }
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let s = seller_shell("orders");
    let done = get(Lang::Zh, Key::Done).0;
    let (s, r) = app::window::probe_house_seg_tab(&ctx, s, Seat::Seller, Page::Orders, &done, 40.0, 1180.0, 260.0);
    assert!(r.pressed, "{r:?}");
    assert_eq!(r.entry_after, r.entry_before, "the page does not enter again: {r:?}");
    assert!(r.scroll_before > 0.0 && r.scroll_after == r.scroll_before, "the scroll stays: {r:?}");
    assert_eq!((r.page.as_str(), r.history_empty), ("orders", true), "{r:?}");
    assert!(s.desk.sh.seller.orders.as_ref().is_some_and(|o| o.done_tab), "the list stands on the completed segment: {r:?}");
}

#[test]
fn a_key_file_exported_on_the_windows_turn_is_recorded() {
    if super::alone_in(module_path!(), "a_key_file_exported_on_the_windows_turn_is_recorded") {
        return;
    }
    let mut s = seller_shell("keyfile");
    let dir = std::env::temp_dir().join(format!("ev-kf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let pw = "file-pass-472";
    let _ = app::action::apply(&mut s, app::action::Action::BackupKey { pin: "Qw7Rt2Lp".into(), password: pw.into(), again: pw.into(), dir: dir.display().to_string() });
    let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut t = 0.0;
    loop {
        let _ = s.drain_at(t);
        t += 0.05;
        let busy = [app::task::Kind::Vault, app::task::Kind::Keystore, app::task::Kind::Keep].into_iter().any(|k| s.tasks.in_flight(k));
        if (!busy && s.keeps.is_empty() && std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0) > 0) || std::time::Instant::now() > until {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let files: Vec<_> = std::fs::read_dir(&dir).expect("the folder").flatten().map(|e| e.path().display().to_string()).collect();
    assert_eq!(files.len(), 1, "one key file landed: {files:?}, {:?}", s.faults);
    assert!(s.keeps.is_empty(), "what the landing left to the disk ran on the window's turn");
    let (row, _) = app::register::now_row_listed().expect("the register").expect("an identity");
    assert!(row.backed_file && row.backup_at == files[0], "recorded with its place: {} {}", row.backed_file, row.backup_at);
    let seen = s.backup_seen.clone().expect("read with the register");
    assert!(seen.exists && seen.opens, "the card reads the file there, the key in office's: {seen:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

fn texts(out: &egui::FullOutput) -> Vec<String> {
    fn walk(s: &egui::Shape, into: &mut Vec<String>) {
        match s {
            egui::Shape::Text(t) => into.push(t.galley.text().to_string()),
            egui::Shape::Vec(v) => v.iter().for_each(|x| walk(x, into)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut v));
    v
}

#[test]
fn a_seat_with_no_key_says_so_and_not_that_no_data_folder_is_open() {
    if super::alone_in(module_path!(), "a_seat_with_no_key_says_so_and_not_that_no_data_folder_is_open") {
        return;
    }
    let root = std::env::temp_dir().join(format!("ev-nokey-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (data, home) = (root.join("data"), root.join("home"));
    std::fs::create_dir_all(&data).expect("data root");
    std::fs::create_dir_all(&home).expect("home");
    let (pin, now) = ("Qw7Rt2Lp", "1758412800");
    let data_s = data.display().to_string();
    let verb = |v: &str, extra: &[&str]| {
        let mut argv: Vec<String> = ["--data", &data_s, "--writer", "j", "--now", now, "--passcode", pin].iter().map(|x| x.to_string()).collect();
        argv.extend(extra.iter().map(|x| x.to_string()));
        let flags = eravon_cli::shared::Flags::read(&argv).expect("the flags");
        let made = eravon_cli::seller::run(v, &flags).expect("the verb runs");
        let made = String::from_utf8_lossy(&eravon_literals::json::write(&made)).into_owned();
        assert!(made.contains("\"ok\":true"), "{v}: {made}");
    };
    verb("vault-init", &["--phrase", "test test test test test test test test test test test junk"]);
    verb("vault-import", &["--label", "单钥"]);
    let script = r#"{"steps":[{"do":"set-way","to":"http://127.0.0.1:9"},{"do":"seat","to":"buyer"}]}"#;
    let r = app::house::drive::Run { data: &data_s, lang: Lang::Zh, budget_ms: 4000, now: 1_758_412_800, door_tallied: Default::default(), registries: Vec::new(), home: home.display().to_string(), passcode: Some(pin.to_string()) };
    let mut shell = match app::house::drive::run(script.as_bytes(), &r) {
        Ok(d) => d.shell,
        Err(e) => panic!("the drive run: {e}"),
    };
    assert_eq!(shell.desk.seat(), Seat::Buyer, "on the buyer seat");
    assert!(std::mem::take(&mut shell.desk.wizard_due), "the switch asked for this seat's first run");
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let (_, outs) = app::window::probe_frames(&ctx, shell, app::nav::Place::Home, None, 1180.0, 800.0, 8);
    let drawn = outs.last().map(texts).unwrap_or_default();
    let said = |k: Key| get(Lang::Zh, k).0;
    assert!(drawn.contains(&said(Key::ChipNoKey)), "the seat has no key: {drawn:?}");
    assert!(!drawn.contains(&said(Key::FaultWhatNoHome)) && !drawn.contains(&said(Key::SetChangeHome)), "no word of a data folder: {drawn:?}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_network_step_puts_the_home_on_the_row_and_keeps_the_nodes_on_disk() {
    if super::alone_in(module_path!(), "the_network_step_puts_the_home_on_the_row_and_keeps_the_nodes_on_disk") {
        return;
    }
    use app::house::act::Act;
    use app::house::action::Action as Step;
    use eravon_literals::constants::{default_network, factory_of, NETWORKS};
    let mut s = seller_shell("netstep");
    let (data, me) = (s.desk.data.clone(), s.desk.addr.clone());
    assert!(eravon_vault::store::write_factory_endpoints(&data, &me, default_network().standard.chain_id).expect("born"), "written");
    let file = s.home.as_ref().expect("the seat's home").dir(app::home::Slot::Settings).join(app::settings::FILE);
    for n in NETWORKS {
        let chain = n.standard.chain_id;
        app::house::act::apply(&mut s, Act::with(Step::Network, &[("chain", &chain.to_string())])).expect("the network step");
        for t in 0..4 {
            let _ = s.drain_at(f64::from(t));
        }
        let h = eravon_vault::store::home_settings(&data, &me).expect("reads").expect("the home");
        assert_eq!(h.chain_id, Some(chain), "{}: on the row chosen", n.name);
        for m in NETWORKS {
            let want: Vec<String> = factory_of(&m).iter().map(|u| (*u).to_string()).collect();
            assert_eq!(eravon_vault::store::urls_on(&h.endpoints, m.standard.chain_id), want, "{}: {}'s factory nodes kept", n.name, m.name);
        }
        let before = std::fs::read(&file).expect("the settings file");
        app::house::act::apply(&mut s, Act::with(Step::Network, &[("chain", &chain.to_string())])).expect("the network step again");
        assert_eq!(std::fs::read(&file).expect("the settings file"), before, "{}: nothing written", n.name);
    }
}
