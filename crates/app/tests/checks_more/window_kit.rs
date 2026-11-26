use app::lang::{t, Key};
use std::path::PathBuf;

fn window_shell(ctx: &eravon_ui::egui::Context) -> app::shell::Shell {
    super::vault_open();
    app::shell::Shell::boot(eravon_ui::skin::dress(ctx))
}

fn seller_with_a_draft(tag: &str) -> app::shell::Shell {
    use app::house::i18n::Lang;
    let root = std::env::temp_dir().join(format!("ev-kit-{tag}-{}", std::process::id()));
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
    let script = r#"{"steps":[{"do":"set-way","to":"http://127.0.0.1:9"},{"do":"page","to":"goods"},{"do":"pe-new"},{"do":"pe-set","k":"name","v":"rail"},{"do":"pe-save"},{"do":"pe-leave"}]}"#;
    let r = app::house::drive::Run { data: &data_s, lang: Lang::Zh, budget_ms: 4000, now: 1_758_412_800, door_tallied: Default::default(), registries: Vec::new(), home: home.display().to_string(), passcode: Some(pin.to_string()) };
    match app::house::drive::run(script.as_bytes(), &r) {
        Ok(d) => d.shell,
        Err(e) => panic!("the drive run: {e}"),
    }
}

#[test]
fn a_rail_item_shows_its_views_own_page() {
    if super::alone_in(module_path!(), "a_rail_item_shows_its_views_own_page") {
        return;
    }
    let ctx = eravon_ui::egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let shell = seller_with_a_draft("rail");
    let (_, walk) = app::window::probe_rail_walk(&ctx, shell, 1180.0, 760.0);
    let mut seen = 0;
    for w in &walk {
        if let (Some(detail), Some(back)) = (&w.detail, &w.from_elsewhere) {
            seen += 1;
            assert_eq!(back, &w.entered, "{}: from another view, its own page", w.item);
            assert_eq!(w.forward.as_ref(), Some(detail), "{}: the page left there is one step forward", w.item);
        }
    }
    assert!(seen > 0, "a view with a detail page was walked: {walk:?}");
}

#[test]
fn a_window_not_shown_still_does_its_own_turn() {
    if super::alone_in(module_path!(), "a_window_not_shown_still_does_its_own_turn") {
        return;
    }
    let ctx = eravon_ui::egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let shell = seller_with_a_draft("own-turn");
    assert!(!shell.desk.data.is_empty(), "the desk stands on the home opened");
    let before = shell.desk.sh.links.len();
    let mut w = app::window::UnseenProbe::new(shell);
    app::house::openurl::push("eravon://not-a-link-kind".to_string());
    for _ in 0..3 {
        w.pass(&ctx, 0.01);
    }
    assert_eq!(w.shell().desk.sh.links.len(), before + 1, "the link delivered was taken in passes not drawn");
}

#[test]
fn a_desktop_task_keeps_its_reads_in_the_sealed_room() {
    if super::alone_in(module_path!(), "a_desktop_task_keeps_its_reads_in_the_sealed_room") {
        return;
    }
    let shell = seller_with_a_draft("keeper");
    assert!(shell.unlocked());
    assert!(app::kinds::Kind::Orders.keeps_reads() && app::kinds::Kind::Door.keeps_reads() && !app::kinds::Kind::Sync.keeps_reads());
    let k = app::kinds::Kind::keeper().expect("a keeper on this machine");
    let part = eravon_chain::settled::NOTES;
    k.save(part, b"{\"form\":\"probe\"}").expect("the probe part is kept");
    assert_eq!(k.load(part).as_deref(), Some(&b"{\"form\":\"probe\"}"[..]), "sealed with the master key the desktop holds, and read back");
    let sealed = std::fs::read(app::home::machine_dir().expect("machine").join(eravon_send::checked::DIR).join(eravon_send::checked::logs_file(part))).expect("written");
    assert!(!String::from_utf8_lossy(&sealed).contains("probe"), "sealed, not in the clear");
}

#[test]
fn a_window_left_alone_wakes_for_the_refresh_period() {
    if super::alone_in(module_path!(), "a_window_left_alone_wakes_for_the_refresh_period") {
        return;
    }
    let ctx = eravon_ui::egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let shell = seller_with_a_draft("wake");
    let mut w = app::window::UnseenProbe::new(shell);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while w.shell().house_busy() && std::time::Instant::now() < until {
        w.pass(&ctx, 0.01);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    w.shell().desk.clock = Some(1_000);
    w.shell().desk.sh.refresh_at = Some(1_000);
    w.shell().desk.sh.refresh_every = 60;
    let mut woke = w.pass_wakes(&ctx, 0.01);
    for _ in 0..20 {
        if woke >= std::time::Duration::from_secs(1) {
            break;
        }
        woke = w.pass_wakes(&ctx, 0.01);
    }
    assert!(woke <= std::time::Duration::from_secs(60) && woke >= std::time::Duration::from_secs(59), "the next pass at the period: {woke:?}");
    w.shell().desk.clock = Some(1_060);
    let _ = w.pass_wakes(&ctx, 0.01);
    assert_eq!(w.shell().desk.sh.refresh_at, Some(1_060), "the pass that woke started the period's refresh");
}

#[test]
fn a_drop_no_zone_took_is_said() {
    if super::alone_in(module_path!(), "a_drop_no_zone_took_is_said") {
        return;
    }
    let ctx = eravon_ui::egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let shell = seller_with_a_draft("drop");
    let (shell, toast) = app::window::probe_drop_untaken(&ctx, shell, false, &["/x/a.txt".to_string()], 1180.0, 760.0);
    assert_eq!(toast.as_deref(), Some(t(Key::DropNotHere)), "on a page that takes no files");
    let (_, toast) = app::window::probe_drop_untaken(&ctx, shell, true, &["/x/a.txt".to_string()], 1180.0, 760.0);
    assert_eq!(toast.as_deref(), Some(t(Key::DropCloseSheet)), "over a sheet");
}

#[test]
fn a_dropped_file_list_reads_into_local_paths() {
    use app::platform::drops::paths_of_uri_list;
    let list = b"# a comment\r\nfile:///srv/u/%E5%8E%9F%E4%BB%B6%20one.txt\r\nfile://localhost/srv/two\r\nfile:///tmp/three?x=1\r\n\r\n";
    assert_eq!(paths_of_uri_list(list), vec![PathBuf::from("/srv/u/原件 one.txt"), PathBuf::from("/srv/two"), PathBuf::from("/tmp/three")]);
    let bad = b"file://elsewhere/srv/u/a\nhttps://example.com/b\nfile:///srv/u/c%4\nfile:///srv/u/d%zz\nfile:///srv/u/e%00f\nfile:relative\nfile:///srv/u/kept\n";
    assert_eq!(paths_of_uri_list(bad), vec![PathBuf::from("/srv/u/kept")], "the bad lines left out, the good one kept");
    assert!(paths_of_uri_list(b"").is_empty());
}

#[test]
fn a_drag_hovers_only_over_this_window_and_a_drop_lands_once() {
    use app::platform::drops::{Drops, Watcher};
    let shared = std::sync::Arc::new(std::sync::Mutex::new(Drops::default()));
    let w = Watcher::over(shared.clone());
    let feed = |f: &dyn Fn(&mut Drops)| f(&mut shared.lock().expect("the state"));
    feed(&|d| d.entered(false, true));
    assert!(!w.hovering(), "another surface");
    feed(&|d| d.entered(true, false));
    assert!(!w.hovering(), "not a list of files");
    feed(&|d| d.entered(true, true));
    assert!(w.hovering());
    feed(&|d| d.left());
    assert!(!w.hovering(), "dragged away");
    feed(&|d| d.entered(true, true));
    feed(&|d| d.delivered(b"file:///srv/u/a.txt\r\nfile:///srv/u/b\r\n"));
    assert!(!w.hovering(), "dropped: no longer hovering");
    assert_eq!(w.take(), vec![PathBuf::from("/srv/u/a.txt"), PathBuf::from("/srv/u/b")]);
    assert!(w.take().is_empty(), "once");
}

#[test]
fn the_window_feeds_the_pieces_drag_into_the_frame() {
    let a = PathBuf::from("/x/a");
    let b = PathBuf::from("/x/b");
    assert_eq!(app::window::probe_feed_drops(false, Vec::new(), vec![a.clone()]), (0, vec![a.clone()]), "nothing from the piece: as given");
    assert_eq!(app::window::probe_feed_drops(true, Vec::new(), Vec::new()), (1, Vec::new()));
    assert_eq!(app::window::probe_feed_drops(false, vec![b.clone()], vec![a.clone()]), (0, vec![a, b]));
}

#[test]
fn a_folder_has_the_dialog_too() {
    use app::platform::Pick;
    let ctx = eravon_ui::egui::Context::default();
    let shell = window_shell(&ctx);
    let (shell, r) = app::window::probe_pick_which(&ctx, shell, false, false, 1180.0, 760.0);
    assert!(!r.asked && r.opened == vec![Pick::FileOrFolder], "where nothing is asked first: the dialog at once: {r:?}");
    let (shell, r) = app::window::probe_pick_which(&ctx, shell, true, true, 1180.0, 760.0);
    assert!(r.asked && r.texts.iter().any(|x| x == t(Key::PickWhich)), "{r:?}");
    assert_eq!(r.opened, vec![Pick::Folder], "the folder key: a folder dialog");
    let (_, r) = app::window::probe_pick_which(&ctx, shell, true, false, 1180.0, 760.0);
    assert_eq!(r.opened, vec![Pick::File], "the file key: a file dialog");
}

#[test]
fn only_linux_asks_file_or_folder_first() {
    assert_eq!(app::platform::asks_file_or_folder(), cfg!(target_os = "linux"));
}

#[test]
fn the_piece_receives_drops_only_where_the_windowing_layer_does_not() {
    use app::platform::{backend_of, drops, Backend};
    use drops::Devices::{Every, NewestOnly};
    let plain = ["wl_compositor", "wl_seat", "wl_data_device_manager", "xdg_wm_base"];
    assert_eq!(drops::devices_of_globals(plain), Every, "no gtk_shell1: every device has the drags");
    assert_eq!(drops::devices_of_globals(plain.into_iter().chain(["gtk_shell1"])), NewestOnly, "gtk_shell1: the newest alone");
    assert!(drops::binds_beside_clipboard(Every));
    assert!(!drops::binds_beside_clipboard(NewestOnly), "beside the clipboard's, one of the two would go without");
    let (x, empty) = (Some(std::ffi::OsStr::new(":0")), Some(std::ffi::OsStr::new("")));
    for display in [x, empty, None] {
        assert_eq!(backend_of(Some(Every), display), Backend::Default, "every device: a Wayland window, whatever X display ({display:?})");
        assert_eq!(backend_of(None, display), Backend::Default, "no compositor read: as the windowing layer picks ({display:?})");
    }
    assert_eq!(backend_of(Some(NewestOnly), x), Backend::X11, "the newest alone with an X display: X11");
    assert_eq!(backend_of(Some(NewestOnly), empty), Backend::Default, "an empty one names none");
    assert_eq!(backend_of(Some(NewestOnly), None), Backend::Default, "no X display: a Wayland window, the piece binding none");
    let mut cell = 0u8;
    let p = (&mut cell as *mut u8).cast::<std::ffi::c_void>();
    assert!(drops::taken_here(drops::Window::Wayland { display: p, surface: p }), "a Wayland window: the piece");
    assert!(!drops::taken_here(drops::Window::Other), "an X11 window (or any other): the windowing layer's own");
    assert!(!drops::taken_here(drops::Window::Wayland { display: std::ptr::null_mut(), surface: p }), "no display given: nothing");
    let w = drops::watch(drops::Window::Other, std::sync::Arc::new(|| ()));
    assert!(w.is_none(), "nothing started for an X11 window");
    drops::Watcher::over(std::sync::Arc::new(std::sync::Mutex::new(drops::Drops::default()))).stop();
    #[cfg(not(target_os = "linux"))]
    assert_eq!(app::platform::window_backend(), Backend::Default, "macOS and Windows: as the windowing layer picks");
}

#[test]
fn the_window_stops_the_piece_once_as_it_closes() {
    use app::platform::drops;
    let n = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let m = n.clone();
    let ctx = eravon_ui::egui::Context::default();
    let shell = window_shell(&ctx);
    let w = drops::Watcher::stopped_by(std::sync::Arc::new(std::sync::Mutex::new(drops::Drops::default())), std::sync::Arc::new(move || {
        m.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }));
    let (_, gone) = app::window::probe_close_with_drops(shell, w);
    assert!(gone, "the window no longer holds the piece");
    assert_eq!(n.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn every_sheet_the_window_draws_is_counted_as_up() {
    let ctx = eravon_ui::egui::Context::default();
    super::vault_open();
    let shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let (_, each) = app::window::probe_each_sheet_is_one(shell);
    assert_eq!(each.len(), 4);
    for (name, up) in each {
        assert!(up, "{name} is a sheet up");
    }
}
