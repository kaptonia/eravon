use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! answers {
    ($got:expr, $pat:pat) => {{
        let got = $got;
        assert!(matches!(got, $pat), "expected {}, answered {:?}", stringify!($pat), got);
    }};
    ($got:expr, $pat:pat, $($said:tt)+) => {{
        let got = $got;
        assert!(matches!(got, $pat), "{}: expected {}, answered {:?}", format!($($said)+), stringify!($pat), got);
    }};
}

fn output_of(cmd: &mut Command) -> std::io::Result<std::process::Output> {
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    eravon_os::spawn(cmd)?.wait_with_output()
}

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn moved() -> Vec<(String, String)> {
    let table = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/family/family-rename.json")).expect("the rename table reads");
    let mut out = Vec::new();
    for row in table.split('{').skip(1) {
        let field = |k: &str| row.split(&format!("\"{k}\": \"")).nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
        let (home, from) = (field("home"), field("from"));
        if let (Some(h), Some(f)) = (home.strip_prefix("crates/"), from.strip_prefix("crates/app/src/")) {
            if !h.starts_with("app/src/") && !f.contains('/') {
                out.push((f.to_string(), h.to_string()));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn read_crate_file(krate: &str, name: &str) -> std::io::Result<String> {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(krate).join("src").join(name))
}

fn place_of(name: &str) -> PathBuf {
    let name = if name == "bin/app.rs" { "bin/eravon-app.rs" } else { name };
    let here = src().join(name);
    if here.is_file() || here.with_extension("").is_dir() {
        return here;
    }
    match moved().into_iter().find(|(f, _)| f == name) {
        Some((_, h)) => Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(h),
        None => {
            let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-vault/src").join(name);
            if vault.is_file() || vault.with_extension("").is_dir() { vault } else { here }
        }
    }
}

fn read_src_file(name: &str) -> std::io::Result<String> {
    let p = place_of(name);
    if p.is_file() {
        return std::fs::read_to_string(&p);
    }
    let dir = p.with_extension("");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|x| x.extension().and_then(|e| e.to_str()) == Some("rs"))
        .collect();
    files.sort_by_key(|f| (f.file_name().and_then(|n| n.to_str()) != Some("mod.rs"), f.clone()));
    let mut out = String::new();
    for f in files {
        out.push_str(&std::fs::read_to_string(&f)?);
        out.push('\n');
    }
    Ok(out)
}

fn house_pages() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).expect("a page folder reads").flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                let name = p.strip_prefix(src()).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                out.push((name, std::fs::read_to_string(&p).expect("a page reads")));
            }
        }
    }
    let mut out = Vec::new();
    walk(&src().join("window/house"), &mut out);
    out.sort();
    out
}

fn shipped() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(src()).expect("src/ reads") {
        let p = e.expect("a directory entry").path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if p.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.push((name, std::fs::read_to_string(&p).expect("a source file reads")));
        } else if p.is_dir() && p.join("mod.rs").is_file() {
            let file = format!("{name}.rs");
            let text = read_src_file(&file).expect("a module folder reads");
            out.push((file, text));
        }
    }
    for (name, _) in moved() {
        if !out.iter().any(|(n, _)| *n == name) {
            let text = read_src_file(&name).expect("a module kept elsewhere reads");
            out.push((name, text));
        }
    }
    out.push(("bin/app.rs".into(), read_src_file("bin/app.rs").expect("bin/app.rs reads")));
    out.sort();
    out
}

fn vault_open() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let machine = checks_machine();
        let _ = std::fs::remove_dir_all(&machine);
        extern "C" fn tidy() {
            let _ = std::fs::remove_dir_all(checks_machine());
        }
        unsafe extern "C" {
            fn atexit(f: extern "C" fn()) -> i32;
        }
        unsafe {
            atexit(tidy);
        }
        app::places::set(app::places::Places { key_account: app::places::ACCOUNT.to_string(), machine_dir: Some(machine), user_home: None });
        app::keybox::set_pin("27618394").expect("开得了这一趟的库");
    });
}

fn seated(shell: &mut app::shell::Shell, home: &Path) -> app::key::Address {
    if app::register::now_row_listed().expect("the register").is_none() {
        let s = app::key::generate().expect("a fresh key");
        app::register::change(|reg| app::identity::add_existing(reg, &s, app::roles::Role::Author, false, "")).expect("an identity");
    }
    seat_home(home);
    assert!(shell.refresh_anchor().expect("the key reads back"), "the key is in the vault");
    let (row, seat) = app::register::now_row_listed().expect("the register").expect("an identity");
    row.address(seat).expect("a seated key")
}

fn seated_anew(shell: &mut app::shell::Shell, home: &Path) -> app::key::Address {
    let s = app::key::generate().expect("a fresh key");
    app::register::change(|reg| app::identity::add_existing(reg, &s, app::roles::Role::Author, false, "")).expect("an identity");
    seated(shell, home)
}

fn seat_home(home: &Path) {
    assert!(std::env::var(ALONE_CHILD).is_ok(), "this test moves the register's home: run it alone (`alone_in`)");
    let (row, seat) = app::register::now_row_listed().expect("the register").expect("an identity");
    let old = row.home(seat).expect("a seated home");
    app::register::change_listed(|reg| Ok(app::identity::rehome(reg, &old, home))).expect("the register follows");
}

fn checks_machine() -> PathBuf {
    let t = std::env::temp_dir();
    let base = if t.as_os_str().len() <= 40 { t } else { PathBuf::from("/tmp") };
    base.join(format!("zk-checks-machine-{}", std::process::id()))
}

fn code_only(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn no_button_word_ends_in_an_ellipsis() {
    const CLICKABLE: [&str; 17] = [
        "key::key(",
        "key::Key::new(",
        ".press_long_saying(ui,",
        "page::Guide::key(",
        "page::Guide::last(",
        ".press(ui,",
        ".primary_with(ui,",
        "menu::row(",
        "f.row(ui,",
        "f.go(ui,",
        "f.go_show(ui,",
        "f.danger(ui,",
        "f.nav_big(ui,",
        "drop::file_row(",
        "self.long_key(ui,",
        "pick::key(",
        "path_row(ui,",
    ];
    let words: std::collections::BTreeMap<String, (&str, &str)> = app::lang::TABLE.iter().map(|(k, zh, en)| (format!("{k:?}"), (*zh, *en))).collect();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (name, text) in shipped().into_iter().chain(house_pages()) {
        let code = code_only(&text);
        for pat in CLICKABLE {
            let mut from = 0;
            while let Some(at) = code[from..].find(pat) {
                let start = from + at + pat.len();
                let mut depth = 1usize;
                let mut end = start;
                for (i, ch) in code[start..].char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = start + i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let args = &code[start..end.max(start)];
                let args = if pat == ".press_long_saying(ui," { args.split(',').next().unwrap_or(args) } else { args };
                let mut rest = args;
                while let Some(k) = rest.find("Key::") {
                    let tail = &rest[k + 5..];
                    let id: String = tail.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                    if let Some((zh, en)) = words.get(&id) {
                        assert!(!zh.ends_with('…') && !en.ends_with('…') && !en.ends_with("..."), "{name}: {pat} draws Key::{id} ({zh} / {en})");
                        seen.insert(id.clone());
                    }
                    rest = &tail[id.len()..];
                }
                from = start;
            }
        }
    }
    for k in ["SetTokensDecimalsEdit", "ChangePinTitle", "BackupRestoreTitle", "OrdersRefundKey", "SetTokensAddNav", "IdRotate", "WlVaultSetKey"] {
        assert!(seen.contains(k), "the scan did not reach Key::{k}");
    }
}

fn enclosing_call(code: &str, at: usize) -> String {
    let b = code.as_bytes();
    let mut depth = 0i32;
    let mut i = at;
    while i > 0 {
        i -= 1;
        match b[i] {
            b')' => depth += 1,
            b'(' if depth == 0 => {
                let head = &code[..i];
                let start = head.rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':')).map(|k| k + 1).unwrap_or(0);
                return head[start..].to_string();
            }
            b'(' => depth -= 1,
            _ => {}
        }
    }
    String::new()
}

#[test]
fn every_scroll_area_goes_in_through_the_librarys_scroll() {
    let mut seen = 0;
    for (name, text) in shipped().into_iter().chain(house_pages()) {
        let code = code_only(&text);
        for (at, _) in code.match_indices("ScrollArea::") {
            let call = enclosing_call(&code, at);
            assert!(["scroll::page", "scroll::nested", "scroll::nested_rows"].iter().any(|w| call.ends_with(w)), "{name}: a scroll area shown by `{call}`, not through `eravon_ui::scroll`");
            seen += 1;
        }
    }
    assert_eq!(seen, 3, "the scan reaches the app's three scroll areas");
}

#[test]
fn nothing_in_a_row_of_then_opens_a_layout_of_its_own() {
    let mut rows = 0;
    for (name, text) in shipped().into_iter().chain(house_pages()) {
        let code = code_only(&text);
        for call in ["width::then(", "width::then_at(", "width::then_line("] {
            for (at, _) in code.match_indices(call) {
                let open = at + call.len() - 1;
                let b = code.as_bytes();
                let (mut depth, mut end) = (0i32, open);
                for (k, ch) in b.iter().enumerate().skip(open) {
                    match ch {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = k;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let args = &code[open..=end];
                for opener in [".horizontal(", ".horizontal_top(", ".horizontal_wrapped(", ".horizontal_centered(", ".vertical(", ".vertical_centered(", ".vertical_centered_justified("] {
                    assert!(!args.contains(opener), "{name}: a row of `{call}` opens a layout ({opener}) inside it");
                }
                rows += 1;
            }
        }
    }
    assert!(rows > 0, "the scan reaches the app's rows of `then`");
}

#[test]
fn the_app_keeps_no_private_copy_of_a_settled_constant() {
    for (name, text) in shipped() {
        let code = code_only(&text);
        for needle in ["Color32::from_rgb", "Color32::from_rgba", "CornerRadius::", "FontId::new"] {
            assert!(
                !code.contains(needle),
                "{name} contains {needle}: design values live in the kit crate and the app keeps no private copy"
            );
        }
    }
}

#[test]
fn the_egui_family_touches_the_shipped_app_in_one_module_only() {
    for (name, text) in shipped() {
        let code = code_only(&text);
        let touches = code.contains("eframe") || code.contains("egui::") || code.contains("use eravon_ui::egui");
        if name == "window.rs" {
            assert!(touches, "window.rs 本该是那一处;它不碰 egui 就说明窗子搬家了");
        } else {
            assert!(!touches, "{name} 碰了 egui 一族:出货的这一份里只许 window.rs 碰");
        }
    }
}

#[test]
fn no_test_driver_code_ships_in_the_app() {
    let toml = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("the manifest reads");
    assert!(!toml.contains("app-drive") && !toml.contains("doorword"), "the app manifest names no test-driver binary and no test-driver dependency");
    for (name, text) in shipped() {
        for needle in ["H0_PALETTE", "H1_BOOT", "emit-plan"] {
            assert!(!text.contains(needle), "{name} carries the test-driver literal {needle}");
        }
    }
}

#[test]
fn the_frame_path_takes_what_is_there_and_never_waits() {
    let text = read_src_file("task.rs").expect("读不出 task.rs");
    let code = code_only(&text);
    for needle in [".recv()", "recv_timeout", "recv_deadline"] {
        assert!(!code.contains(needle), "task.rs 里有 {needle}:收信只许 try_recv");
    }
    let shutdown = code.split("pub fn shutdown").nth(1).expect("没有 shutdown");
    let joins_total = code.matches(".join()").count();
    let joins_in_shutdown = shutdown.matches(".join()").count();
    assert_eq!(joins_total, joins_in_shutdown, "join 只许住 shutdown");
    assert_eq!(joins_in_shutdown, 1, "关门恰一处 join");
}

#[test]
fn exactly_one_place_starts_a_thread() {
    let (mut built, mut bare) = (0, 0);
    for (_, text) in shipped() {
        built += code_only(&text).matches("thread::Builder").count();
        bare += code_only(&text).matches("thread::spawn").count();
    }
    assert_eq!((built, bare), (1, 0), "起线程只许一处(它在单飞那一问之后),且不用会崩的那一口");
}

#[test]
fn nothing_reaches_the_task_pool_except_through_apply() {
    for (name, text) in shipped() {
        if name == "action.rs" || name == "task.rs" {
            continue;
        }
        assert!(
            !code_only(&text).contains("tasks.spawn("),
            "{name} 绕过了 action::apply:动作只有一处正主"
        );
    }
}

#[test]
fn the_trace_channel_is_never_compiled_out() {
    let text = read_src_file("trace.rs").expect("读不出 trace.rs");
    assert!(
        !code_only(&text).contains("#[cfg("),
        "痕迹通道里有 cfg:发布构建照开,没有哪个开关能把它编掉"
    );
}

#[test]
fn a_mark_carries_a_feature_id_and_nothing_else() {
    let text = read_src_file("trace.rs").expect("读不出 trace.rs");
    assert!(
        code_only(&text).contains("pub fn mark(f: Feature)"),
        "记号只收闭型的功能号,收不下一句自由文本"
    );
    for f in app::feature::Feature::ALL {
        let id = f.id();
        assert!(!id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()), "功能号 {id:?} 不成形");
    }
}

#[test]
fn every_fault_carries_its_evidence_tail() {
    use app::fault::{classify, Class, Fault, Known};
    let a = Fault::known(Known::FileMissing, "原话");
    assert_eq!(a.tail(), "原话");
    let b = Fault::unknown("原话");
    assert_eq!(b.tail(), "原话");
    assert!(b.passthrough(), "未知那一支照原样透传,一个字不加");
    let e = std::io::Error::other("说不清");
    let c = classify(&e, "主语");
    assert_eq!(c.class(), Class::Unknown);
    assert_eq!(c.said(), c.tail(), "不猜:脸上那句就是底下那句");
}

#[test]
fn the_known_table_is_closed_and_each_member_has_a_sentence() {
    use app::fault::{translate, Known};
    let mut said: Vec<&str> = Vec::new();
    for k in Known::ALL {
        let s = translate(k);
        assert!(!s.is_empty(), "{} 没有人话", k.as_str());
        assert!(!said.contains(&s), "两枚已知错误共用一句人话:{s}");
        said.push(s);
    }
}

#[test]
fn the_frame_never_touches_the_disk() {
    let text = read_src_file("window.rs").expect("读不出 window.rs");
    let code = code_only(&text);
    for needle in ["usage()", ".survey()", "read_dir", "std::fs::", "metadata("] {
        assert!(!code.contains(needle), "window.rs 里有 {needle}:读盘要走后台那一趟");
    }
    assert!(app::task::Kind::ALL.contains(&app::task::Kind::Archive));
}

#[test]
fn there_is_no_system_keychain_interface_anywhere() {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.file_name().and_then(|x| x.to_str()) == Some("target") {
                continue;
            }
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") && p.components().any(|c| c.as_os_str() == "src") {
                out.push(p);
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("crates 目录").to_path_buf();
    let mut files = Vec::new();
    walk(&crates, &mut files);
    let symbols = ["SecItem", "kSecClass", "kSecAttr", "SecKeychain", "CoreFoundation", "name = \"Security\"", "keychain::"];
    for p in &files {
        let code = code_only(&std::fs::read_to_string(p).expect("读得出源"));
        for sym in symbols {
            assert!(!code.contains(sym), "{} 里还有钥匙串接口那一族:{sym}", p.display());
        }
    }
    assert!(!src().join("keychain.rs").exists(), "钥匙串那一档该整个没了");
    let hits = |t: &str| t.contains("钥匙串") || t.to_lowercase().contains("keychain");
    for (k, zh, en) in app::lang::TABLE.iter() {
        assert!(!hits(zh) && !hits(en), "话册 {k:?} 的人话还说钥匙串");
    }
    for k in app::fault::Known::ALL {
        assert!(!hits(app::fault::translate(k)), "拒因 {} 的人话还说钥匙串", k.as_str());
    }
    let platform = code_only(&read_src_file("platform.rs").expect("平台接口那一处"));
    assert_eq!(platform.matches("pub fn ask_path(").count(), 1, "选档框只此一处");
    for (name, text) in shipped() {
        if name == "platform.rs" {
            continue;
        }
        assert!(!code_only(&text).contains("extern \"C\""), "{name} 里有 FFI");
    }
}

#[test]
fn platform_capabilities_go_through_one_interface() {
    let platform = code_only(&read_src_file("platform.rs").expect("平台接口"));
    for f in ["pub fn ask_path(", "pub fn lock_now(", "pub fn lock_wait(", "pub fn home_dir(", "pub fn zone_rules(", "pub fn user_temp_dir(", "pub fn app_data_dir(", "pub fn say_without_window("] {
        assert_eq!(platform.matches(f).count(), 1, "接口里 {f} 恰一处");
    }
    assert!(platform.contains("compile_error!"), "没有实现的系统在编译时停下");
    let faces = code_only(&std::fs::read_to_string(src().join("window").join("faces.rs")).expect("窗那一处"));
    assert_eq!(faces.matches("crate::platform::say_without_window(").count(), 1, "没有窗时说话只经平台接口一处");
    assert!(!faces.contains("eprintln!"), "窗那一处不自写标准错误");
    let bin = code_only(&std::fs::read_to_string(place_of("bin/app.rs")).expect("出货那一枚"));
    assert!(!bin.contains("eprintln!"), "出货那一枚不自写标准错误");
    for (name, text) in shipped() {
        if name == "platform.rs" {
            continue;
        }
        let code = code_only(&text);
        for w in ["rfd::", "objc_msgSend", "var_os(\"HOME\")", "\"/etc/localtime\"", "/System/Library", "confstr"] {
            assert!(!code.contains(w), "{name} 里有 {w}:平台能力只经 platform 那一处");
        }
    }
    let fonts = code_only(&std::fs::read_to_string(src().join("../../eravon-ui/src/fonts.rs")).expect("字体那一处"));
    assert_eq!(fonts.matches("pub const ROLES:").count(), 3, "字体表三份(macOS、Windows 各一份,其余系统一份)");
    assert!(fonts.contains("#[cfg(target_os = \"macos\")]\npub const ROLES:") && fonts.contains("#[cfg(target_os = \"windows\")]\npub const ROLES:"), "macOS 与 Windows 两份各点名");
    assert!(!fonts.contains("not(target_os = \"linux\")"), "不以「不是 Linux」当 macOS");
}

#[test]
fn settings_hold_no_key_material_and_words_stay_with_identity() {
    let text = read_src_file("settings.rs").expect("读不出 settings.rs");
    let code = code_only(&text);
    for field in ["privkey", "private_key", "secret", "Secret", "mnemonic"] {
        assert!(!code.contains(field), "设置那一档里出现了 {field}:设置不装密钥材料");
    }
    for w in ["cryptx::", "family::", "identity::", "phrase_of(", ".words()", "Secret::take("] {
        assert!(!code.contains(w), "设置那一档里出现了 {w}:设置不碰派生与种子");
    }
    for (name, t) in shipped() {
        let c = code_only(&t);
        if name != "cryptx.rs" {
            for w in ["bip39", "BIP39"] {
                assert!(!c.contains(w), "{name} 里出现了 {w}:词表件只许住 cryptx");
            }
        }
        if c.contains("phrase_of(") {
            assert!(["cryptx.rs", "identity.rs"].contains(&name.as_str()), "{name} 把熵读成了词:只许身份那一层");
        }
        if c.contains(".words()") {
            assert!(["identity.rs", "window.rs"].contains(&name.as_str()), "{name} 拿了明文词:只许身份那一层与显示");
        }
    }
}

#[test]
fn the_bare_key_can_be_revealed_only_once() {
    let text = read_src_file("key.rs").expect("读不出 key.rs");
    assert!(
        code_only(&text).contains("pub fn reveal_once(slot: &mut Option<Secret>)"),
        "签名一改,「一次性」就不再成立"
    );
    let s = app::key::Secret::take([9u8; 32]).expect("在阶内");
    let mut slot = Some(s);
    assert!(app::key::reveal_once(&mut slot).is_some(), "第一次给得出");
    assert!(slot.is_none(), "给过之后那一格空了");
    assert!(app::key::reveal_once(&mut slot).is_none(), "没有第二次");
}

#[test]
fn no_path_writes_a_plaintext_key_to_disk() {
    let text = read_src_file("key.rs").expect("读不出 key.rs");
    let code = code_only(&text);
    for w in ["fs::write", "File::create", "land_bytes"] {
        assert!(!code.contains(w), "key.rs 里有落盘的路 {w}:明文只许住内存与系统钥匙串");
    }
    assert!(code.contains("\n    fn bytes(&self)"), "bytes() 要住 key.rs 自己一处");
    assert!(!code.contains("pub(crate) fn bytes"), "bytes() 不许对 crate 公开");
    assert!(!code.contains("pub fn bytes"), "bytes() 不许对外公开");
    let at = code.find("impl Secret {").expect("impl Secret");
    let block = &code[at..at + code[at..].find("\n}\n").expect("impl Secret 收尾")];
    let mut fns: Vec<String> = block.lines().map(str::trim).filter(|l| (l.starts_with("pub") || l.starts_with("fn ")) && l.contains("fn ")).map(|l| l.trim_end_matches('{').trim_end().to_string()).collect();
    fns.sort();
    let mut want: Vec<String> = [
        "pub fn take(bytes: [u8; 32]) -> Option<Secret>",
        "fn bytes(&self) -> &[u8; 32]",
        "pub fn address(&self) -> Option<Address>",
        "pub(in crate::sign) fn with_sign_key<R>(&self, f: impl FnOnce(&[u8; 32]) -> R) -> R",
        "pub(in crate::sign) fn with_tx_key<R>(&self, f: impl FnOnce(&[u8; 32]) -> R) -> R",
        "pub(crate) fn ciphered(&self, key: &[u8; 16], iv: &[u8; 16]) -> Vec<u8>",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    want.sort();
    assert_eq!(fns, want, "Secret 的出口闭集变了");
    for exit in [
        "pub(in crate::sign) fn with_sign_key<R>(&self, f: impl FnOnce(&[u8; 32]) -> R)",
        "pub(crate) fn ciphered(&self, key: &[u8; 16], iv: &[u8; 16])",
        "pub(in crate::sign) fn with_tx_key<R>(&self, f: impl FnOnce(&[u8; 32]) -> R)",
        "pub fn reveal_once(slot: &mut Option<Secret>)",
    ] {
        assert!(code.contains(exit), "少了这一处出口:{exit}");
    }
    let ks = code_only(&read_src_file("keystore.rs").expect("读不出 keystore.rs"));
    assert!(ks.contains("secret.ciphered("), "keystore 要走密文那一口");
    assert!(!ks.contains(".bytes()"), "keystore 里不许再摸明文字节");
    for (口, 住处, 邻居, 处数) in [
        ("with_tx_key(", "sign.rs", "s.with_tx_key(", 1),
    ] {
        let mut sites = 0usize;
        let mut beside = false;
        for name in [
            "action.rs", "window.rs", "keystore.rs", "sign.rs", "key.rs",
        ] {
            let t = code_only(&read_src_file(name).expect("读不出源"));
            let defs = t.matches(&format!("fn {}", 口.trim_end_matches('('))).count();
            let n = t.matches(口).count().saturating_sub(defs);
            sites += n;
            if n > 0 {
                assert_eq!(name, 住处, "{口} 跑到 {name} 去了");
                beside = t.contains(邻居);
            }
        }
        assert_eq!(sites, 处数, "{口} 恰 {处数} 处调用,现读 {sites}");
        assert!(beside, "{口} 那一处的邻居该是 {邻居}");
    }
}

#[test]
fn the_home_lays_its_own_four_rooms() {
    let dir = std::env::temp_dir().join(format!("zk-test-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let h = app::home::Home::open_or_create(&dir).expect("建家");
    assert!(h.missing().is_empty(), "缺:{:?}", h.missing());
    for s in app::home::Slot::ALL {
        assert!(h.dir(s).is_dir(), "{} 没铺出来", s.as_str());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_lock_is_the_kernels_and_nothing_deletes_it() {
    let text = read_src_file("lock.rs").expect("读不出 lock.rs");
    let code = code_only(&text);
    assert!(code.contains("crate::writer::lock_now("), "锁要向内核要(经平台接口)");
    assert!(code_only(&std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-os/src/unix.rs")).expect("平台接口")).contains("fn flock("), "平台那一处向内核要 flock");
    for w in ["remove_file", "unlink", "stale", "陈旧", "pid ==", "SystemTime"] {
        assert!(!code.contains(w), "lock.rs 里有 {w}:陈旧锁的判词一旦出现,就是那条渐近线");
    }
    for (name, t) in shipped() {
        if name == "lock.rs" || name == "lockwire.rs" {
            continue;
        }
        assert!(
            !code_only(&t).contains("LOCK_FILE"),
            "{name} 动了锁档的名字:锁档只归 lock.rs"
        );
    }
}

#[test]
fn the_way_out_of_an_empty_locked_box_and_the_key_that_offers_it_ask_one_question() {
    let keybox = read_crate_file("eravon-seal", "keybox.rs").expect("读不出封存层 keybox.rs");
    let kcode = code_only(&keybox);
    assert!(
        kcode.contains("pub fn reset_empty(v: &Vault)") && kcode.contains("if recoverable(v)?"),
        "`reset_empty` 要问 `recoverable()`,不许自己再拼一遍条件"
    );
    let body = kcode.split("pub fn reset_empty(v: &Vault)").nth(1).unwrap_or("");
    let asked = body.find("recoverable(v)");
    let removed = body.find("remove_file");
    assert!(asked.is_some() && removed.is_some() && asked < removed, "`reset_empty` 要先问再删");
    let win = code_only(&read_src_file("window.rs").expect("读不出 window.rs"));
    assert_eq!(
        win.matches("!self.shell.vault_recoverable").count(),
        1,
        "门上那一枚「重置密钥库」的条件该只有一处"
    );
    assert!(win.contains("let empty_out = locked_out && !self.shell.vault_recoverable;"), "那一枚键的条件写法变了");
    assert!(win.contains("Action::ResetEmptyKeybox"), "门上要有那一枚键");
    let shell_src = code_only(&read_src_file("shell.rs").expect("读不出 shell.rs"));
    assert!(
        shell_src.contains("crate::keybox::recoverable().unwrap_or(true)"),
        "壳上那一格要现读 `recoverable`,读不成即当作「还有东西」(宁可不出那一枚键)"
    );
    assert!(!app::action::Action::ResetEmptyKeybox.needs_key(), "重置空库那一条不该要钥");
}

#[test]
fn whether_the_gate_covers_the_window_is_one_closed_table_over_the_four_vault_states() {
    use app::keybox::State;
    let all = [State::Absent, State::Locked { wrong: 0 }, State::Locked { wrong: 4 }, State::LockedOut, State::Open];
    for s in all {
        let gate = s.gate_up();
        let keys = s.keys_ready();
        let want_gate = matches!(s, State::Locked { .. } | State::LockedOut);
        let want_keys = matches!(s, State::Open);
        assert_eq!(gate, want_gate, "{s:?}:门在不在答错了");
        assert_eq!(keys, want_keys, "{s:?}:钥拿不拿得到答错了");
        if matches!(s, State::Absent) {
            assert!(!gate, "还没有库时门不该摆在屏上(否则首启走不出去)");
            assert!(!keys, "还没有库时钥也拿不到");
        }
    }
    let text = read_src_file("window.rs").expect("读不出 window.rs");
    let code = code_only(&text);
    assert_eq!(
        code.matches("vault.gate_up()").count() + code.matches("state.gate_up()").count(),
        6,
        "问「门在不在」的该是六处,都过 `State::gate_up`(现读到的不是六处)"
    );
    assert!(
        !code.contains("if !self.shell.unlocked() {"),
        "the window decides whether to draw from 'unlocked or not' again (the cause of the first-run bug)"
    );
    for wrong in [
        "State::Absent | crate::keybox::State::Open",
        "State::Locked { .. } | crate::keybox::State::LockedOut",
    ] {
        assert!(!code.contains(wrong), "窗里又自己拼门那一问了:{wrong}");
    }
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    for s in all {
        shell.vault = app::shell::Vault::Read(s);
        assert_eq!(shell.unlocked(), s.keys_ready(), "{s:?}:`Shell::unlocked` 与 `State::keys_ready` 答得不一样");
    }
    shell.vault = app::shell::Vault::Damaged(app::fault::Fault::known(app::fault::Known::KeyboxShape, String::new()));
    assert!(shell.vault.gate_up() && !shell.unlocked(), "库坏了:门在、钥不可得");
}

#[test]
fn the_shipped_binary_cannot_move_where_the_anchor_key_lives() {
    for (name, text) in shipped() {
        let code = code_only(&text);
        assert!(
            !code.contains("places::set"),
            "{name} calls places::set: only test code may set the landing places"
        );
        for gone in ["ZIKARON_KEY_ACCOUNT", "ZIKARON_MACHINE_DIR"] {
            assert!(!text.contains(gone), "{name} 里还留着环境变量 {gone}");
        }
        if name == "keybox.rs" {
            assert_eq!(
                code.matches("set_light_kdf").count(),
                1,
                "keybox.rs 里 set_light_kdf 只许有定义那一处"
            );
        } else {
            assert!(
                !code.contains("set_light_kdf"),
                "{name} calls set_light_kdf: only test code may select the light KDF setting"
            );
        }
    }
    assert_eq!(app::places::key_account(), app::places::ACCOUNT);
    let keybox = read_crate_file("eravon-seal", "keybox.rs").expect("读不出封存层 keybox.rs");
    let code = code_only(&keybox);
    assert!(code.contains("book.kdf = Some(params())"), "建库那一趟该把这一趟的取式档写进库档");
    assert!(code.contains("crate::kdf::in_range(n, r, pp)"), "库档读回来那一节要现验界");
    assert!(code.contains("fn kdf_of(b: &Book)"), "开档那一侧该有「照这一本库那一档」的一处正主");
    assert_eq!(app::keybox::params(), app::keystore::Params::standard(), "没摆过轻档时跑的就是标准档");
}

#[test]
fn the_two_languages_carry_the_same_key_set() {
    let bad = app::lang::trouble();
    assert!(bad.is_empty(), "键表不齐:{bad:?}");
    assert_eq!(app::lang::TABLE.len(), app::lang::Key::ALL.len());
    assert_eq!(app::lang::Lang::ALL.len(), 2);
    for l in app::lang::Lang::ALL {
        app::lang::set(l);
        assert_eq!(app::lang::lang(), l);
        for k in app::lang::Key::ALL {
            assert!(!app::lang::t(k).is_empty(), "{k:?} 在 {} 下是空的", l.as_str());
        }
    }
    app::lang::set(app::lang::Lang::Zh);
}

#[test]
fn the_window_carries_no_sentence_of_its_own() {
    let text = read_src_file("window.rs").expect("读不出 window.rs");
    for (i, l) in text.lines().enumerate() {
        if l.trim_start().starts_with("//") {
            continue;
        }
        let mut in_str = false;
        let mut prev = ' ';
        let mut lit = String::new();
        for c in l.chars() {
            if c == '"' && prev != '\\' {
                if in_str && lit.contains("UTC") {
                    panic!("window.rs:{} 里有写死的时区标:{}", i + 1, l.trim());
                }
                in_str = !in_str;
                lit.clear();
            } else if in_str {
                lit.push(c);
            }
            if in_str && ('\u{4e00}'..='\u{9fff}').contains(&c) {
                panic!("window.rs:{} 里有写死的话:{}", i + 1, l.trim());
            }
            prev = c;
        }
    }
}

#[test]
fn every_sentence_is_filled_with_as_many_values_as_it_names() {
    use app::lang::TABLE;
    let highest = |s: &str| -> Option<usize> {
        let mut top = None;
        let mut rest = s;
        while let Some(i) = rest.find('{') {
            let after = &rest[i + 1..];
            if let Some(j) = after.find('}') {
                if let Ok(n) = after[..j].parse::<usize>() {
                    top = Some(top.map_or(n, |t: usize| t.max(n)));
                }
            }
            rest = after;
        }
        top
    };
    let slots = |key: &str| -> Option<usize> {
        TABLE.iter().find(|(k, _, _)| format!("{k:?}") == key).map(|(_, zh, en)| highest(zh).max(highest(en)).map_or(0, |n| n + 1))
    };
    let ident = |s: &str| -> String { s.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect() };
    let mut bad = Vec::new();
    for (file, text) in shipped() {
        let code = code_only(&text);
        for (given, head) in [(1usize, "fill1("), (2, "fill2("), (3, "fill3("), (0, "t(")] {
            for (at, _) in code.match_indices(head) {
                let before = code[..at].chars().next_back();
                if before.map(|c| c.is_ascii_alphanumeric() || c == '_').unwrap_or(false) {
                    continue;
                }
                let tail = code[at + head.len()..].trim_start();
                let Some(named) = tail.strip_prefix("Key::") else { continue };
                let key = ident(named);
                if head == "t(" && !named[key.len()..].trim_start().starts_with(')') {
                    continue;
                }
                if let Some(n) = slots(&key) {
                    if n > given {
                        bad.push(format!("{file}: {key} has {n} slots, filled with {given}"));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "sentences shown with open slots: {bad:?}");
}

#[test]
fn there_is_one_way_to_fetch_a_sentence() {
    let text = read_src_file("lang.rs").expect("读不出 lang.rs");
    let code = code_only(&text);
    assert_eq!(code.matches("pub fn t(").count(), 1, "取词的路恰一条");
    assert!(code.contains("pub const TABLE"), "一行同时带两种话的那张表该在这里");
}

#[test]
fn a_restore_stages_everything_before_it_lands_anything() {
    let text = read_src_file("backup.rs").expect("读不出 backup.rs");
    let tail = text.split("pub fn restore(").nth(1).expect("没有 restore");
    let body = tail.split("\npub fn ").next().expect("函数体");
    let vault_at = body.find("stage_new(").expect("restore 里该先把新库摆在旁边");
    let stage_at = body.find("stage_next(").expect("restore 里该把机器档摆在旁边");
    let commit_at = body.find("commit_new(").expect("restore 里该有库改名那一下");
    let settle_at = body.find("settle_pending(").expect("restore 里该在改名之后落位");
    assert!(vault_at < stage_at, "新库先摆:它在盘上即什么都没生效");
    assert!(stage_at < commit_at, "摆齐了才改名:这一句靠次序成立");
    assert!(commit_at < settle_at, "改名之后才落位");
    assert!(!body.contains("keybox::write_book"), "restore 里不许直写库档");
    assert!(!body.contains("write_pointer"), "恢复不改机器指针:无身份的那一处家就地换");
    let settle = read_src_file("local.rs").expect("读不出 local.rs");
    let body = settle.split("pub fn settle_pending(").nth(1).expect("没有 settle_pending");
    assert!(body.contains("let never = crate::keybox::drop_staged()?"), "while a new key store is still staged nothing is settled; that is the only condition checked");
}

#[test]
fn the_single_source_column_comes_from_the_endpoint_law() {
    let text = read_crate_file("eravon-send", "node.rs").expect("读不出发送器 node.rs");
    let code = code_only(&text);
    assert!(code.contains("single_source: r.single_source"), "单源那一栏要照抄端点法给的");
    for (name, text) in [("node.rs", text.clone()), ("chainx.rs", read_src_file("chainx.rs").expect("读不出 chainx.rs"))] {
        assert!(!code_only(&text).contains("sources == 1"), "{name}:别在这一层自己判单源");
    }
}

#[test]
fn the_shipped_app_starts_no_child_process_at_all() {
    for (name, text) in shipped() {
        if name == "notifier.rs" {
            continue;
        }
        let code = code_only(&text);
        for w in ["Command::new", "std::process::Command"] {
            assert!(!code.contains(w), "{name} 里起了子进程:发布件一个也不许起");
        }
    }
}

#[test]
fn every_page_has_a_name_in_both_languages_and_an_icon() {
    use app::shell::Page;
    assert_eq!(Page::ALL.len(), 4, "四页(本家各页走本家视图)");
    let mut keys = Vec::new();
    for p in Page::ALL {
        let k = p.key();
        assert!(app::lang::TABLE.iter().any(|(x, _, _)| *x == k), "{p:?} 的名不在表里");
        let _ = p.icon();
        keys.push(format!("{k:?}"));
    }
    let n = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), n, "两页共用一个名");
    assert!(app::lang::trouble().is_empty(), "{:?}", app::lang::trouble());
}

#[test]
fn the_two_seats_carry_the_prototype_rails() {
    use app::lang::{set, t, Lang};
    use app::nav::{count, rail};
    use app::roles::Role;
    set(Lang::Zh);
    assert_eq!(count(Role::Author), 9);
    assert_eq!(count(Role::Grantee), 7);
    let shape = |r: Role| -> Vec<(String, Vec<String>)> {
        rail(r)
            .iter()
            .map(|g| (g.title.map(t).unwrap_or("").to_string(), g.items.iter().map(|i| t(i.name).to_string()).collect()))
            .collect()
    };
    let want_author = vec![("", vec!["首页", "订单", "待上链", "商品", "判官与行会", "导出", "钱包", "付款方式", "通知"])];
    let want_grantee = vec![("", vec!["首页", "订单", "待上链", "付款链接", "钱包", "通知", "导出"])];
    let own = |w: Vec<(&str, Vec<&str>)>| -> Vec<(String, Vec<String>)> {
        w.into_iter().map(|(g, i)| (g.to_string(), i.into_iter().map(str::to_string).collect())).collect()
    };
    assert_eq!(shape(Role::Author), own(want_author));
    assert_eq!(shape(Role::Grantee), own(want_grantee));
}

#[test]
fn every_alias_name_resolves_to_a_place() {
    use app::nav::{alias_names, place_named};
    for n in alias_names() {
        assert!(place_named(&n).is_some(), "{n} 认不出");
    }
    assert_eq!(alias_names().len(), 3 + app::nav::View::ALL.len() + app::nav::sections(app::roles::Role::Author).len());
    for old in ["Settings.seat", "Settings.chain", "Settings.cadence", "Settings.mirror"] {
        assert!(place_named(old).is_some(), "旧段名 {old} 该照认");
    }
    assert!(place_named("Settings.nowhere").is_none());
}

#[test]
fn the_language_choice_is_kept_on_the_machine_and_in_an_open_home() {
    if alone("the_language_choice_is_kept_on_the_machine_and_in_an_open_home") {
        return;
    }
    vault_open();
    use app::action::{apply, Action, Applied};
    use app::lang::{lang, set, Lang};
    let dir = std::env::temp_dir().join(format!("eravon-test-lang-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    shell.home = None;
    shell.lock = None;
    set(Lang::Zh);
    match apply(&mut shell, Action::SetLang { lang: Lang::En }) {
        Applied::Spoken(Lang::En) => {}
        other => panic!("没有家也该换成英文:{other:?}"),
    }
    assert_eq!(lang(), Lang::En, "说的是英文");
    assert_eq!(app::machine::read().expect("机器设置").lang, Some(Lang::En), "这台机器记着英文");
    assert_eq!(shell.settings.lang, None, "没有家,家里那一份不动");
    let home = app::home::Home::open_or_create(&dir).expect("建家");
    shell.home = Some(home);
    match apply(&mut shell, Action::SetLang { lang: Lang::Zh }) {
        Applied::Trouble(f) => assert!(f.said().starts_with("READ_ONLY"), "写不了的家该具名拒:{}", f.said()),
        other => panic!("写不了的家却换成了:{other:?}"),
    }
    assert_eq!(lang(), Lang::En, "拒了就不换");
    assert_eq!(app::machine::read().expect("机器设置").lang, Some(Lang::En), "机器那一份也不动");
    let lock = app::lock::take(shell.home.as_ref().unwrap()).expect("取锁");
    shell.lock = Some(lock);
    match apply(&mut shell, Action::SetLang { lang: Lang::Zh }) {
        Applied::Spoken(Lang::Zh) => {}
        other => panic!("该换成中文:{other:?}"),
    }
    let back = app::settings::Settings::read(shell.home.as_ref().unwrap()).expect("读回");
    assert_eq!(back.lang, Some(Lang::Zh), "盘上那一份记着中文");
    assert_eq!(app::machine::read().expect("机器设置").lang, Some(Lang::Zh), "这台机器也记着中文");
    set(Lang::Zh);
    drop(shell);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_homes_settings_keep_what_this_version_does_not_know() {
    if alone_in(module_path!(), "a_homes_settings_keep_what_this_version_does_not_know") {
        return;
    }
    vault_open();
    use app::action::{apply, Action, Applied};
    let dir = std::env::temp_dir().join(format!("eravon-test-extra-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    seated(&mut shell, &dir);
    answers!(apply(&mut shell, Action::OpenHome { root: dir.display().to_string() }), Applied::Homed { .. });
    let room = shell.home.as_ref().expect("a home").dir(app::home::Slot::Settings);
    answers!(apply(&mut shell, Action::SetZone { zone: app::when::Zone::System }), Applied::Zoned(_));
    let at = room.join(app::settings::FILE);
    let now = app::local::read(&at, app::local::Doc::Settings).expect("读").expect("在");
    let mut v = eravon::json::parse(&now).expect("JSON");
    if let eravon::json::Value::Obj(m) = &mut v {
        m.push(("futureCell".into(), eravon::json::parse(b"{\"x\":[1,2]}").expect("JSON")));
    }
    app::local::put(&room, app::settings::FILE, app::local::Doc::Settings, &eravon::json::canon_bytes(&v)).expect("落一份新版的设置");
    shell.settings = app::settings::Settings::read(shell.home.as_ref().unwrap()).expect("读");
    match apply(&mut shell, Action::SetZone { zone: app::when::Zone::Utc }) {
        Applied::Zoned(_) => {}
        other => panic!("该存下:{other:?}"),
    }
    let raw = app::local::read(&at, app::local::Doc::Settings).expect("读").expect("在");
    let text = String::from_utf8(raw).expect("utf-8");
    assert!(text.contains("\"futureCell\":{\"x\":[1,2]}") && text.contains("\"zone\":\"utc\""), "{text}");
    drop(shell);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_homes_settings_are_read_wide() {
    vault_open();
    let dir = std::env::temp_dir().join(format!("zk-test-wide-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = app::home::Home::open_or_create(&dir).expect("建家");
    let room = home.dir(app::home::Slot::Settings);
    let later = b"{\"capBytes\":0,\"endpoints\":[\"1=https://a.example\",5],\"role\":\"Grantee\"}";
    app::local::put(&room, app::settings::FILE, app::local::Doc::Settings, later).expect("落一份设置");
    let s = app::settings::Settings::read(&home).expect("读");
    assert_eq!((s.role, s.cap_bytes, s.endpoints.clone()), (app::roles::Role::Author, app::settings::CAP_DEFAULT, vec!["1=https://a.example".to_string()]));
    let _ = std::fs::remove_dir_all(&dir);
}

fn alone(test: &str) -> bool {
    const CHILD: &str = "ERAVON_CHECKS_ALONE";
    if std::env::var(CHILD).as_deref() == Ok(test) {
        return false;
    }
    let me = std::env::current_exe().expect("this test binary");
    let out = output_of(std::process::Command::new(me).args([test, "--exact", "--nocapture", "--test-threads=1"]).env(CHILD, test)).expect("the child runs");
    assert!(out.status.success() && String::from_utf8_lossy(&out.stdout).contains("1 passed"), "{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    true
}

#[test]
fn what_came_after_the_backup_is_behind_it() {
    if alone("what_came_after_the_backup_is_behind_it") {
        return;
    }
    vault_open();
    use app::action::{apply, Action, Applied};
    let base = std::env::temp_dir().join(format!("eravon-behind-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let behind = |m: &app::machine::Machine| app::machine::backup_behind(m.backup.as_ref(), app::backup::measured(m.backup.as_ref()).ok());
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let mut n = 0u64;
    let mut home = |shell: &mut app::shell::Shell, name: &str| {
        let dir = base.join(name);
        seated_anew(shell, &dir);
        assert!(matches!(apply(shell, Action::OpenHome { root: dir.display().to_string() }), Applied::Homed { .. }));
        n += 1;
        let id = format!("{n:040x}");
        let room = shell.home.as_ref().expect("a home").dir(app::home::Slot::Store).join("products").join(&id);
        std::fs::create_dir_all(&room).expect("the record's place");
        let envelope = eravon_store::record::Record { id, kind: eravon_store::record::Kind::Product, created: 1, updated: 1, writer: "behind".into(), fields: Vec::new(), parts: Vec::new() };
        app::local::put(&room, "record.product", app::local::Doc::Product, &envelope.seal()).expect("a record");
        dir
    };
    home(&mut shell, "a");
    let b = home(&mut shell, "b");

    let first = app::backup::export(&base.join("out"), "eravon-backup-probe", 1_800_000_000).expect("备份");
    let m = app::machine::read().expect("机器设置");
    let last = m.backup.clone().expect("记下了这一次备份");
    assert!(first.path.exists() && last.count == 2, "{last:?}");
    assert_eq!(behind(&m), Some(0), "刚备份");

    home(&mut shell, "c");
    assert_eq!(behind(&m), Some(1), "备份之后添一处、记一条");

    std::fs::remove_dir_all(&b).expect("删去一处数据夹");
    assert_eq!(behind(&m), Some(1), "删去一处之后");
    home(&mut shell, "d");
    assert_eq!(behind(&m), Some(2), "删去一处之后再记一条");

    let now = app::backup::count_now().expect("现数");
    let index_at = app::backup::index_path().expect("索引处");
    let index = std::fs::read(&index_at).expect("索引在");
    std::fs::write(&index_at, b"{not an index").expect("坏索引");
    assert_eq!(app::backup::measured(Some(&last)).ok(), Some(now), "索引读不成");
    std::fs::write(&index_at, &index).expect("放回");
    let text = String::from_utf8(app::local::read(&index_at, app::local::Doc::BackupIndex).expect("开得了").expect("索引在")).expect("索引是文字");
    let other = text.replacen("\"at\":1800000000", "\"at\":1800000100", 1);
    assert_ne!(other, text);
    let room = index_at.parent().expect("索引屋");
    app::local::put(room, app::backup::INDEX_FILE, app::local::Doc::BackupIndex, other.as_bytes()).expect("别次的索引");
    assert_eq!(app::backup::measured(Some(&last)).ok(), Some(now), "别次备份的索引");
    std::fs::write(&index_at, &index).expect("放回");
    assert_eq!(behind(&m), Some(2), "放回之后照按索引算");

    drop(shell);
    app::backup::restore(&first.path, "eravon-backup-probe", app::backup::From::Settings("27618394")).expect("恢复");
    let m = app::machine::read().expect("恢复后的机器设置");
    assert_eq!(m.backup.as_ref().map(|b| b.at), Some(1_800_000_000));
    assert_eq!(behind(&m), Some(0), "恢复之后");
    let restored_index = std::fs::read(app::backup::index_path().expect("索引处")).expect("恢复写下了索引");
    assert!(app::local::is_sealed(&restored_index) && restored_index != index, "恢复之后的索引封在新钥下");
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    home(&mut shell, "e");
    assert_eq!(behind(&m), Some(1), "恢复之后再记一条");
    drop(shell);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_proxy_choice_is_saved_as_read_or_refused_untouched() {
    vault_open();
    use app::action::{apply, Action, Applied};
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let on_disk = || std::fs::read(app::machine::path().expect("机器设置的路")).unwrap_or_default();
    for (typed, said, written) in [
        ("system", "system", "system"),
        ("  none  ", "none", "none"),
        (" SOCKS5://Proxy.Example:1080/ ", "socks5://proxy.example:1080", "socks5://proxy.example:1080"),
        ("http://[::1]:7890", "http://[::1]:7890", "http://[::1]:7890"),
        ("HTTP://127.0.0.1:8080", "http://127.0.0.1:8080", "http://127.0.0.1:8080"),
        ("https://u:p%20w@proxy.example:443", "https://u@proxy.example:443", "https://u:p%20w@proxy.example:443"),
        ("https://u@proxy.example:443", "https://u@proxy.example:443", "https://u:p%20w@proxy.example:443"),
        ("socks4a://id@127.0.0.1:1080", "socks4://id@127.0.0.1:1080", "socks4://id@127.0.0.1:1080"),
    ] {
        match apply(&mut shell, Action::SetProxy { choice: typed.into() }) {
            Applied::ProxySet(w) => assert_eq!(w, said, "{typed:?}"),
            other => panic!("{typed:?}: {other:?}"),
        }
        assert_eq!(app::machine::read().expect("读").proxy.as_deref(), Some(written));
    }
    for (typed, shown, tail_key) in [
        ("proxy.example:8080", "proxy.example:8080", app::lang::Key::TailProxyShape),
        ("ftp://127.0.0.1:1", "ftp://127.0.0.1:1", app::lang::Key::TailProxyShape),
        ("http://127.0.0.1", "http://127.0.0.1", app::lang::Key::TailProxyShape),
        ("", "", app::lang::Key::TailProxyShape),
        ("http://:secret@127.0.0.1:1", "http://@127.0.0.1:1", app::lang::Key::TailProxyCredentials),
        ("socks4://id:secret@127.0.0.1:1", "socks4://id@127.0.0.1:1", app::lang::Key::TailProxyCredentials),
    ] {
        let before = on_disk();
        match apply(&mut shell, Action::SetProxy { choice: typed.into() }) {
            Applied::Trouble(f) => {
                assert!(f.said().starts_with("SETTINGS_SHAPE"), "{typed:?}: {}", f.said());
                let tails: Vec<String> = app::lang::TABLE.iter().filter(|(k, _, _)| *k == tail_key).flat_map(|(_, zh, en)| [zh.to_string(), en.to_string()]).collect();
                assert!(tails.iter().any(|t| f.tail() == t.replace("{0}", shown)), "{typed:?}: {}", f.tail());
                assert!(!f.tail().contains("secret") && !f.said().contains("secret"), "{typed:?}");
            }
            other => panic!("{typed:?}: {other:?}"),
        }
        assert_eq!(on_disk(), before, "{typed:?}: refused, not one byte moved");
    }
}

#[test]
fn the_proxy_line_says_the_way_and_why_it_is_straight() {
    use eravon_net::{Choice, Kind, PacTrouble, Proxy, Reading, Script, Way};
    let texts = |k: app::lang::Key| -> Vec<String> { app::lang::TABLE.iter().filter(|(x, _, _)| *x == k).flat_map(|(_, zh, en)| [zh.to_string(), en.to_string()]).collect() };
    let plain = Reading { way: Way::Direct, then: Vec::new(), choice: Choice::System, loopback: false, local: false, system_unread: false, script: Script::None };
    let p = eravon_net::proxy_of("socks5://me:secret@127.0.0.1:1080").expect("typed");
    let via = app::machine::proxy_said(&Reading { way: Way::Through(p.clone()), choice: Choice::Off, ..plain.clone() });
    assert!(texts(app::lang::Key::ProxyNowVia).iter().any(|t| via == t.replace("{0}", "socks5://me@127.0.0.1:1080")), "{via}");
    assert!(!via.contains("secret"));
    let manual = Choice::Manual(Proxy { kind: Kind::Http, host: "p".into(), port: 1, auth: None });
    for (r, now) in [
        (Reading { loopback: true, choice: manual.clone(), ..plain.clone() }, app::lang::Key::ProxyNowLoopback),
        (Reading { local: true, choice: manual.clone(), ..plain.clone() }, app::lang::Key::ProxyNowLocal),
        (Reading { system_unread: true, ..plain.clone() }, app::lang::Key::ProxyNowUnread),
        (Reading { script: Script::Pending, ..plain.clone() }, app::lang::Key::ProxyNowScriptPending),
        (plain.clone(), app::lang::Key::ProxyNowDirect),
    ] {
        assert!(texts(now).contains(&app::machine::proxy_said(&r)), "{now:?}");
    }
    let troubled = app::machine::proxy_said(&Reading { script: Script::Trouble(PacTrouble::Timeout), ..plain.clone() });
    let direct = texts(app::lang::Key::ProxyNowDirect);
    assert!(texts(app::lang::Key::ProxyScriptTrouble).iter().any(|t| direct.iter().any(|d| troubled == t.replace("{0}", d).replace("{1}", "PAC_TIMEOUT"))), "{troubled}");
}

#[test]
fn a_failed_proxy_leg_is_said_as_the_proxys() {
    let leg = eravon_net::Fail::Proxy("via proxy http://127.0.0.1:9: 127.0.0.1:9 refused".into());
    for t in [eravon_chain::rpc::transport_said("https://node.example/", &leg), eravon_chain::rpc::proxy_leg("http://node.example/", "via proxy socks5://127.0.0.1:1080: SOCKS5 refused")] {
        let f = app::chainx::said_fault("https://node.example/", &t);
        assert_eq!(f.which(), Some(app::fault::Known::ProxyUnreachable), "{t:?}");
        assert!(app::fault::Known::NETWORK.contains(&app::fault::Known::ProxyUnreachable));
        assert_ne!(f.next(), app::lang::t(app::lang::Key::FaultNextNodeTls));
    }
}

fn rpc_node(answer: fn(&str) -> Option<String>) -> String {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut s = s;
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                let body = loop {
                    match s.read(&mut buf) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => raw.extend_from_slice(&buf[..n]),
                    }
                    let text = String::from_utf8_lossy(&raw).to_string();
                    if let Some(i) = text.find("\r\n\r\n") {
                        let len: usize = text[..i].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|n| n.trim().parse().ok())).unwrap_or(0);
                        if raw.len() >= i + 4 + len {
                            break text[i + 4..].to_string();
                        }
                    }
                };
                let method = body.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                let id = body.split("\"id\":").nth(1).and_then(|r| r.split(|c| c == ',' || c == '}').next()).unwrap_or("1").trim().to_string();
                let Some(result) = answer(&method) else { return };
                let (status, body) = if let Some(code) = result.strip_prefix("HTTP ") {
                    (code.to_string(), String::from("{}"))
                } else {
                    ("200 OK".to_string(), format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{result}}}"))
                };
                let _ = s.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    url
}

#[test]
fn the_send_card_says_when_its_cap_is_the_fallback() {
    use eravon_chain::send::Fees;
    let fallback = Fees::fallback();
    assert_eq!(app::chainx::fee_source_line(None), Some(app::lang::Key::U3FeeFallback), "no fees read");
    assert_eq!(app::chainx::fee_source_line(Some(&fallback)), Some(app::lang::Key::U3FeeFallback), "the fallback pair");
    let same_numbers = Fees { from_chain: true, ..fallback };
    assert_eq!(app::chainx::fee_source_line(Some(&same_numbers)), None, "from the chain, though the numbers are the same");
}

#[test]
fn the_app_reads_an_endpoint_spelling_as_the_command_line_does() {
    for s in [" 1=https://a.example", "1= https://a.example", "01=https://a.example", "+1=https://a.example", "1=https://a/?k=v", "1==x", "1", "=x", "-1=x", "1=", "1=  ", "18446744073709551616=x"] {
        let app = app::chainx::Endpoint::parse(s).map(|e| (e.chain, e.url));
        assert_eq!(app, eravon_chain::rpc::endpoint_spec(s).ok().map(|(c, u)| (c, app::chainx::NodeAddr::new(u))), "{s:?}");
    }
}

fn with_member(v: &eravon::json::Value, path: &[&str], to: Option<eravon::json::Value>) -> eravon::json::Value {
    use eravon::json::Value;
    let Value::Obj(m) = v else { return v.clone() };
    let mut m = m.clone();
    match path {
        [k] => {
            m.retain(|(x, _)| x != k);
            if let Some(t) = to {
                m.push((k.to_string(), t));
            }
        }
        [k, rest @ ..] => {
            for (x, inner) in m.iter_mut() {
                if x == k {
                    *inner = with_member(inner, rest, to.clone());
                }
            }
        }
        [] => {}
    }
    Value::Obj(m)
}

#[test]
fn a_node_address_that_does_not_read_is_its_shape_naming_the_port() {
    use app::chainx::{address_said, head_block, Endpoint};
    let said_k = |k: app::lang::Key, x: &str| -> Vec<String> { app::lang::TABLE.iter().filter(|(y, _, _)| *y == k).flat_map(|(_, zh, en)| [zh.replace("{0}", x), en.replace("{0}", x)]).collect() };
    let named = |u: &str| eravon_net::sayable(u);
    assert!(said_k(app::lang::Key::TailNodePort, &named("https://h:65536")).contains(&address_said("https://h:65536")));
    assert!(said_k(app::lang::Key::TailNodePort, &named("https://h:")).contains(&address_said("https://h:")));
    assert!(said_k(app::lang::Key::Tail093, &named("wss://h")).contains(&address_said("wss://h")));
    assert!(said_k(app::lang::Key::TailNodeAddress, &named("https://u@h")).contains(&address_said("https://u@h")));
    assert!(!address_said("https://u:S3CRET@h").contains("S3CRET"));
    assert_eq!(address_said("https://h:65535"), "");
    let ep = |u: &str| Endpoint { chain: 1, url: u.into() };
    match head_block(&[ep("https://h:65536"), ep("https://h:")], 1) {
        Err(f) => assert!(f.said().starts_with("SETTINGS_SHAPE") && f.tail().contains(&eravon_net::sayable("https://h:65536")) && f.tail().contains(&eravon_net::sayable("https://h:")), "{} {}", f.said(), f.tail()),
        Ok(_) => panic!("read"),
    }
    let dead = { let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind"); format!("http://{}", l.local_addr().expect("addr")) };
    match head_block(&[ep("https://h:65536"), ep(&dead)], 1) {
        Err(f) => assert!(!f.said().starts_with("SETTINGS_SHAPE"), "a node merely down beside it: the network's: {}", f.said()),
        Ok(_) => panic!("read"),
    }
}

#[test]
fn a_keystore_of_the_wrong_shape_is_said_as_that_before_any_work() {
    use eravon::json::Value;
    let secret = app::key::Secret::take([0x42; 32]).expect("a key");
    let ks = app::keystore::encrypt(&secret, "pw-probe", app::keystore::Params::light(), 1_700_000_000).expect("encrypted");
    let v = eravon::json::parse(&ks.json).expect("json");
    let open = |v: &Value| app::keystore::decrypt(&eravon::json::canon_bytes(v), "pw-probe").err().map(|f| f.said().split(':').next().unwrap_or("").to_string());
    assert_eq!(open(&v), None, "the file as written opens");
    for (form, path, to, want) in [
        ("mac absent", vec!["crypto", "mac"], None, "KEYSTORE_SHAPE"),
        ("mac empty", vec!["crypto", "mac"], Some(Value::Str(String::new())), "KEYSTORE_SHAPE"),
        ("mac a number", vec!["crypto", "mac"], Some(Value::Int(5)), "KEYSTORE_SHAPE"),
        ("mac 31 bytes", vec!["crypto", "mac"], Some(Value::Str("ab".repeat(31))), "KEYSTORE_SHAPE"),
        ("mac not hex", vec!["crypto", "mac"], Some(Value::Str("zz".repeat(32))), "KEYSTORE_SHAPE"),
        ("ciphertext absent", vec!["crypto", "ciphertext"], None, "KEYSTORE_SHAPE"),
        ("r absent", vec!["crypto", "kdfparams", "r"], None, "KEYSTORE_SHAPE"),
        ("r as text", vec!["crypto", "kdfparams", "r"], Some(Value::Str("8".into())), "KEYSTORE_SHAPE"),
        ("p absent", vec!["crypto", "kdfparams", "p"], None, "KEYSTORE_SHAPE"),
        ("r zero", vec!["crypto", "kdfparams", "r"], Some(Value::Int(0)), "KEYSTORE_PARAMS"),
        ("mac of another", vec!["crypto", "mac"], Some(Value::Str("ab".repeat(32))), "BAD_PASSWORD"),
    ] {
        assert_eq!(open(&with_member(&v, &path, to)).as_deref(), Some(want), "{form}");
    }
    let long = app::secret::Secret::from("x".repeat(1025));
    assert!(app::keystore::decrypt_typed(&ks.json, &long).err().map(|f| f.said().starts_with("PASSWORD_LONG")).unwrap_or(false), "past the cap on reading");
    let at_cap = app::secret::Secret::from("x".repeat(1024));
    assert!(app::keystore::decrypt_typed(&ks.json, &at_cap).err().map(|f| f.said().starts_with("BAD_PASSWORD")).unwrap_or(false), "at the cap: read, and wrong");
}

#[test]
fn a_second_take_of_a_home_reads_and_the_next_after_release_writes() {
    vault_open();
    let dir = std::env::temp_dir().join(format!("zk-test-lock2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = app::home::Home::open_or_create(&dir).expect("建家");
    let first = app::lock::take(&home).expect("取锁");
    let second = app::lock::take(&home).expect("再取");
    assert_eq!((first.mode(), second.mode()), (app::lock::Mode::Writer, app::lock::Mode::Reader));
    drop(second);
    drop(first);
    assert_eq!(app::lock::take(&home).expect("放手后再取").mode(), app::lock::Mode::Writer);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_three_actions_that_use_a_key_ask_the_one_passcode_gate() {
    let text = read_src_file("action.rs").expect("读不出 action.rs");
    let code = code_only(&text);
    let at = code.find("pub enum Action {").expect("Action 枚举");
    let body = &code[at..code[at..].find("\n}\n").map(|e| at + e).unwrap_or(code.len())];
    let mut carriers: Vec<String> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let Some((head, rest)) = line.split_once(" {") else { continue };
        if !rest.contains("pin") {
            continue;
        }
        let name = head.trim_end_matches(char::is_whitespace).to_string();
        if name.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
            carriers.push(name);
        }
    }
    assert!(carriers.len() >= 8, "带 pin 那一格的动作只数出 {} 员", carriers.len());
    let gate = code.find("pub fn pin_asked").expect("pin_asked 那一张闭表");
    let gate_body = &code[gate..code[gate..].find("\n    }\n").map(|e| gate + e).unwrap_or(code.len())];
    for name in &carriers {
        assert!(gate_body.contains(name.as_str()), "{name} 带着一格口令而没在 pin_asked 里报名");
    }
    use app::action::Action as A;
    for a in [
        A::BackupKey { pin: "1".into(), password: String::new().into(), again: String::new().into(), dir: String::new() },
        A::DeleteIdentity { id: String::new(), pin: "2".into() },
        A::RevealWords { pin: "3".into() },
    ] {
        assert!(a.pin_asked().is_some(), "{a:?} 该过口令闸");
    }
    for a in [
        A::SetPin { pin: "1".into(), again: "1".into() },
        A::Unlock { pin: "1".into() },
        A::ChangePin { old: "1".into(), pin: "2".into(), again: "2".into() },
        A::RecoverWords { words: String::new().into(), pin: "1".into(), again: "1".into() },
        A::RecoverKeystore { path: String::new(), password: String::new().into(), pin: "1".into(), again: "1".into() },
    ] {
        assert!(a.pin_asked().is_none(), "{a:?} 是口令门自己,不过这一闸");
    }
    assert_eq!(code.matches("a.pin_asked()").count(), 1, "过闸那一句只许一处");
    let run = code.find("a.pin_asked()").expect("执行那一句");
    let locked = code.find("a.needs_key() && !shell.unlocked()").expect("锁着即拒那一句");
    assert!(locked < run, "「锁着即拒」要摆在口令闸之前");
    assert!(A::BackupKey { pin: String::new().into(), password: String::new().into(), again: String::new().into(), dir: String::new() }.needs_key());
    assert!(A::DeleteIdentity { id: String::new(), pin: String::new().into() }.needs_key());
    assert!(!A::RevealWords { pin: String::new().into() }.needs_key());
}

#[test]
fn every_small_file_this_desk_lands_is_owner_only() {
    let home = code_only(&read_src_file("home.rs").expect("读不出 home.rs"));
    assert!(home.contains("eravon_os::put_at(dir, name, &put_tmp_name()?, bytes)"), "家那一处落档经平台件那一口");
    let os = code_only(&read_crate_file("eravon-os", "lib.rs").expect("读不出 eravon-os"));
    assert!(os.contains("owner_only(&mut o)"), "落档那一句建临时档时即只本人可读写");
    assert!(os.contains("create_new(true)"), "临时名撞上了即具名拒,不去截断别人那一份");
    assert!(os.contains("open_owner_only(&tmp)") && os.contains("rename_over(&tmp, &p)"), "临时档建时即只本人,替换进位");
    let dir = std::env::temp_dir().join(format!("eravon-owner-only-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    app::home::put_at(&dir, "probe.json", b"{}").expect("落得下");
    assert!(eravon_os::is_owner_only(&dir.join("probe.json")).expect("读得回"), "落下的档只本人可读写");
    let _ = std::fs::remove_dir_all(&dir);
    let keybox = code_only(&read_crate_file("eravon-seal", "keybox.rs").expect("读不出封存层 keybox.rs"));
    for own in ["std::fs::rename", "File::create"] {
        assert!(!keybox.contains(own), "keybox.rs 里又自己落档了:{own}");
    }
    assert!(keybox.contains("crate::land::put_at(&v.dir, name, &bytes)"), "库档(与摆在旁边的新库)该经落档那一处");
    assert!(keybox.contains("eravon_os::rename_over(&from, &to)"), "新库生效那一下改名也经平台件那一处");
}

#[test]
fn one_key_sits_on_one_seat_and_the_old_shape_cannot_be_built() {
    use app::identity::{Keys, Registry, Row};
    use app::key::Address;
    use app::roles::Role;
    let a = Address([0x11; 20]);
    let g = Address([0x22; 20]);
    let one = Row {
        id: a.hex(),
        keys: Keys::One { seat: Role::Grantee, addr: a },
        author_office: None,
        grantee_office: None,
        author_home: app::identity::UNSEATED.to_string(),
        grantee_home: "/x/g".to_string(),
        backed_words: false,
        backed_file: true,
        backup_at: app::identity::NO_BACKUP_AT.to_string(),
        label: app::identity::NO_LABEL.to_string(),
        created: app::identity::NO_CREATED.to_string(),
        network: None,
        unread_network: None,
    };
    assert_eq!(one.address(Role::Author), None);
    assert_eq!(one.address(Role::Grantee), Some(a));
    assert_eq!(one.home(Role::Author), None);
    assert_eq!(one.account(Role::Author), None);
    assert_eq!(one.accounts().len(), 1, "只占一席即只占一槽");
    assert_eq!(one.seats(), vec![Role::Grantee]);
    assert_eq!(one.first_seat(), Role::Grantee, "落在它占着的那一席上");
    let both = Row { keys: Keys::Both { author: a, grantee: g }, author_home: "/x/a".into(), ..one.clone() };
    assert_eq!(both.kind(), app::identity::Kind::Words);
    assert_eq!(one.kind(), app::identity::Kind::Existing);
    let reg = Registry { current: Some((one.id.clone(), Role::Grantee)), rows: vec![one.clone()], left: Vec::new() };
    assert_eq!(Registry::parse(&reg.to_bytes()).expect("读得回"), reg);
    let before_this_change = format!(
        concat!(
            "{{\"current\":{{\"identity\":\"{id}\",\"seat\":\"author\"}},",
            "\"identities\":[{{\"author\":\"{id}\",\"backup\":{{\"file\":true,\"words\":false}},",
            "\"grantee\":\"{id}\",\"homes\":{{\"author\":\"/x/a\",\"grantee\":\"/x/g\"}},",
            "\"id\":\"{id}\",\"kind\":\"existing\",\"slot\":\"own\"}}],",
            "\"shape\":\"eravon-desk/identities/1\"}}"
        ),
        id = a.hex()
    );
    assert!(Registry::parse(before_this_change.as_bytes()).is_err(), "两席同址那一行具名拒");
    let text = String::from_utf8(reg.to_bytes()).expect("utf8");
    for cell in ["\"created\":\"\",", "\"label\":\"\",", "\"at\":\"\","] {
        let stripped = text.replace(cell, "");
        assert_ne!(stripped, text, "{cell}");
        assert!(Registry::parse(stripped.as_bytes()).is_err(), "缺 {cell} 即拒");
    }
    let named = Row { label: "工作室主号".into(), ..one.clone() };
    assert_eq!(named.kind(), one.kind());
    assert_eq!(named.accounts(), one.accounts());
    assert_eq!(named.first_seat(), one.first_seat());
}

#[test]
fn a_backup_counts_only_when_the_file_is_really_on_disk() {
    let code = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    let check = code.find("landed_check(&path, ks.address)?;").expect("落地之后该现读回来比一次");
    let done = code.find("Done::Keystore(crate::task::Keystore::BackedUp").expect("交出已备份那一句");
    assert!(check < done, "先读回来比过,再交出「已备份」");
    assert!(code.contains("Known::BackupNotLanded"), "对不上要具名拒");
    let shell = code_only(&read_src_file("shell.rs").expect("读不出 shell.rs"));
    assert!(shell.contains("crate::identity::mark(reg, &id, false, true, Some(&path))"), "旗与落处一起记");
    let identity = code_only(&read_src_file("identity.rs").expect("读不出 identity.rs"));
    let at = identity.find("pub fn backup_seen").expect("读数口");
    let body = &identity[at..identity[at..].find("\n}\n").map(|e| at + e).unwrap_or(identity.len())];
    for writes in ["write(", "std::fs::write", "remove_file", "mark("] {
        assert!(!body.contains(writes), "读数口改了盘上的东西:{writes}");
    }
    let window = code_only(&read_src_file("window.rs").expect("读不出 window.rs"));
    assert_eq!(window.matches("crate::identity::backup_seen").count(), 1, "脸上只许在开层那一刻问一处");
    assert!(window.contains("self.shell.backup_seen"), "身份卡读换手那一刻存下的那一格");
    assert_eq!(shell.matches("crate::identity::backup_seen").count(), 1, "换手那一处问一次");
}

#[test]
fn opening_a_home_is_judged_in_one_place() {
    let mut takes = 0usize;
    let mut set_home = 0usize;
    let mut set_lock = 0usize;
    for (name, text) in shipped() {
        let code = code_only(&text);
        takes += code.matches("lock::take(").count();
        set_home += code.matches("shell.home = Some(").count();
        set_lock += code.matches("shell.lock = Some(").count();
        let _ = name;
    }
    assert_eq!(takes, 2, "取写者锁那一句在产品里现数 {takes} 处(该是开家一处与接存储箱那一线)");
    assert_eq!(set_home, 1, "摆「正开着的那一处家」现数 {set_home} 处");
    assert_eq!(set_lock, 1, "摆那一把锁现数 {set_lock} 处");
    let code = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    let at = code.find("fn opening(").expect("开家那一段");
    let body = &code[at..code[at..].find("\n}\n").map(|e| at + e).unwrap_or(code.len())];
    assert!(body.contains("crate::lock::holds_writer_at(ask.held.as_deref(), &home)"), "复用那一判要在开家那一处");
    assert_eq!(code.matches("crate::lock::holds_writer_at(").count(), 1, "复用那一判只许一处");
    for (caller, through) in [("fn open_home(", "open_home_at("), ("fn open_home_at(", "opening("), ("fn enter(", "entering("), ("fn entering(", "opening("), ("fn seat_start(", "entering(")] {
        let a = code.find(caller).unwrap_or_else(|| panic!("找不到 {caller}"));
        let b = &code[a..code[a..].find("\n}\n").map(|e| a + e).unwrap_or(code.len())];
        assert!(b.contains(through), "{caller} 该走开家那一处({through})");
    }
}

#[test]
fn the_exported_key_file_is_for_its_owner_only_and_the_rest_are_not() {
    let code = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    assert!(
        code.contains("land_bytes_for(eravon_store::landing::Readers::Owner, &path, &ks.json)"),
        "导出密钥文件那一路该具名 Readers::Owner"
    );
    let home = code_only(&read_src_file("home.rs").expect("读不出 home.rs"));
    let numbered = home.split("pub fn land_numbered(").nth(1).and_then(|b| b.split("\npub fn ").next()).unwrap_or_default();
    assert!(numbered.contains("land_bytes_for(eravon_store::landing::Readers::Owner, &p, bytes)"), "备份起名落档那一处该具名 Readers::Owner");
    let mut owner = 0usize;
    for (_, text) in shipped() {
        owner += code_only(&text).matches("Readers::Owner").count();
    }
    assert_eq!(owner, 2, "具名 Owner 的落档现数 {owner} 处(只该是密钥文件与整机备份两路)");
    let landing = std::fs::read_to_string(
        src().parent().expect("crates/app/src 的上一级").parent().expect("crates").join("eravon-glue").join("src").join("landing.rs"),
    )
    .expect("读不出 landing.rs");
    assert!(landing.contains("eravon_os::owner_only(&mut o)"), "只给本人那一路建临时地时即只本人可读写");
    assert_eq!(landing.matches("fn create_for(").count(), 1, "建临时地那一句只许一处");
    assert!(landing.contains("fn land_bytes_for("), "落档那一处该收「谁读得到」那一格");
}

#[test]
fn the_key_store_is_written_only_with_its_lock_in_hand() {
    let code = code_only(&read_crate_file("eravon-seal", "keybox.rs").expect("读不出封存层 keybox.rs"));
    assert!(code.contains("fn write_book(v: &Vault, _held: &Held, b: &Book)"), "写库那一处该收手里那一把锁");
    assert_eq!(code.matches("fn lock_book(v: &Vault)").count(), 1, "取库锁那一句只许一处");
    assert!(code.contains("eravon_os::lock_wait("), "库档那一把要等得起(拿不到即等着)");
    let writes = code.matches("write_book(v, &held").count() + code.matches("write_book(v, held,").count();
    assert_eq!(code.matches("write_book(").count(), writes + 1, "有一处写库没有把手里那一把递进去(+1 是定义那一处)");
    assert_eq!(code.matches("lock_book(v)?").count(), 15, "要改库的那几口各拿一次锁");
    let wiring = code_only(&read_src_file("keybox.rs").expect("读不出 keybox.rs"));
    assert!(wiring.contains("crate::places::keybox_lock_file()"), "锁座那一份档的名由 places 一处给");
    let places = code_only(&read_src_file("places.rs").expect("读不出 places.rs"));
    assert!(places.contains("fn keybox_lock_file()"), "锁座那一份档的名住 places");
    assert!(!code.contains("crate::lock::take("), "库那一把不许借家的写者锁那一口");
    let lock = code_only(&read_src_file("lock.rs").expect("读不出 lock.rs"));
    assert_eq!(lock.matches("pub fn grab(").count(), 1);
    assert_eq!(lock.matches("pub fn grab_waiting(").count(), 1);
}

#[test]
fn where_a_thing_lands_is_judged_in_one_place_and_says_why() {
    use app::home::{choose, Kind, Why, HOME_STEM};
    let base = std::env::temp_dir().join(format!("zk-landing-{}", std::process::id()));
    let empty = base.join("empty");
    let full = base.join("full");
    std::fs::create_dir_all(&empty).expect("铺一处空的");
    std::fs::create_dir_all(full.join("something")).expect("铺一处不空的");
    let a = choose(&Kind::Home, &empty);
    assert_eq!((a.at.clone(), a.why), (empty.clone(), Why::AsPicked));
    let b = choose(&Kind::Home, &full);
    assert_eq!((b.at.clone(), b.why), (full.join(HOME_STEM), Why::FolderNotEmpty));
    assert!(b.free(), "新起的名此刻该空着");
    assert!(b.is_new_name());
    let d1 = choose(&Kind::File { stem: "snap".into(), ext: "json".into() }, &empty);
    assert_eq!(d1.why, Why::AsPicked);
    std::fs::write(&d1.at, b"x").expect("占住那一个名");
    let d2 = choose(&Kind::File { stem: "snap".into(), ext: "json".into() }, &empty);
    assert_eq!((d2.at.clone(), d2.why), (empty.join("snap-2.json"), Why::NameTaken));
    let named = base.join("pick-me.zip");
    let e = choose(&Kind::Bundle { stem: "kit".into() }, &named);
    assert_eq!((e.at.clone(), e.why), (named, Why::AsPicked));
    let _ = std::fs::remove_dir_all(&base);
    for (name, text) in shipped() {
        let code = code_only(&text);
        for gone in ["kit_target(", "migrate_target(", "file_target("] {
            assert!(!code.contains(gone), "{name} 里还留着落处那一族的旧成员:{gone}");
        }
    }
    let home = code_only(&read_src_file("home.rs").expect("读不出 home.rs"));
    assert_eq!(home.matches("pub fn choose(").count(), 1, "落处那一问只许一处判");
    assert_eq!(home.matches("fn numbered(").count(), 1, "往后编号那一句只许一处");
}

#[test]
fn the_frame_body_asks_no_question_that_needs_the_disk() {
    let window = code_only(&read_src_file("window.rs").expect("读不出 window.rs"));
    assert!(!window.contains("crate::identity::backup_seen("), "备份档那一笔不许在帧里读(壳上那一格由动作层摆)");
    assert_eq!(window.matches("crate::local::Ledger::open(").count(), 0, "窗子不自己开账本目录");
    let at = window.find("fn id_layer_open").expect("开层那一处");
    let body = &window[at..window[at..].find("\n    }\n").map(|e| at + e).unwrap_or(window.len())];
    assert!(body.contains("crate::action::seats_with_entries"), "账本那一笔该在开层那一刻经动作层读");
    let action = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    let at = action.find("pub fn seats_with_entries(").expect("动作层那一口");
    let _reader = &action[at..action[at..].find("\n}\n").map(|e| at + e).unwrap_or(action.len())];

    assert!(
        window.contains("self.ux.id_delete_backup = row.as_ref().map(crate::identity::backup_seen)"),
        "备份档那一笔该在开层那一刻读"
    );
    assert!(!window.contains("crate::identity::from_words(&self.ux.id_words"), "帧里不许为了「按不按得动」去开一次熵源");

    let shell = code_only(&read_src_file("shell.rs").expect("读不出 shell.rs"));
    assert_eq!(shell.matches("pub fn seat_identities(").count(), 1, "摆身份表那一口只许一处");
    assert!(shell.contains("self.backup_seen = reg"), "那一笔读数与身份表同一趟换");
    assert!(shell.contains("backup_seen: _,"), "备份档那一笔随身份表走,不随账的来源走");
    for (name, text) in shipped() {
        let code = code_only(&text);
        let hits = code.matches(".identities = ").count();
        if name.ends_with("shell.rs") {
            assert_eq!(hits, 1, "{name} 里写身份表那一句只许一处(就在 seat_identities 里面)");
            let at = code.find("pub fn seat_identities(").expect("摆身份表那一口");
            let body = &code[at..code[at..].find("\n    }\n").map(|e| at + e).unwrap_or(code.len())];
            assert!(body.contains("self.identities = reg"), "那一句该在摆身份表那一口里面");
        } else {
            assert_eq!(hits, 0, "{name} 绕过了摆身份表那一口");
        }
    }
}

#[test]
fn which_identity_is_current_is_asked_in_two_named_places() {
    let identity = code_only(&read_src_file("identity.rs").expect("读不出 identity.rs"));
    let register = code_only(&read_src_file("register.rs").expect("读不出 register.rs"));
    assert_eq!(identity.matches("pub fn now_row(").count(), 1, "含账名底那一枚那一口只许一处");
    assert_eq!(register.matches("pub fn now_row_listed(").count(), 1, "只认登记表那一口只许一处");
    assert_eq!(identity.matches("view.now()").count(), 1, "那一读只在 now_row 里面");
    assert_eq!(register.matches("read()?.and_then(|r| r.now()").count(), 1, "那一读只在 now_row_listed 里面");
    let action = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    for (what, at) in [("显词", "fn reveal_words"), ("备份", "fn backup_key")] {
        let i = action.find(at).unwrap_or_else(|| panic!("{what}那一段"));
        let body = &action[i..action[i..].find("\n}\n").map(|e| i + e).unwrap_or(action.len())];
        assert!(body.contains("crate::identity::now_row("), "{what}那一路要问那一口");
    }
    for (name, text) in shipped() {
        if name == "identity.rs" || name == "register.rs" || name == "window.rs" || name == "shell.rs" || name == "house.rs" {
            continue;
        }
        let code = code_only(&text);
        assert!(!code.contains(".now()"), "{name} 绕过了那两口,自己问了一次「谁是当前」");
    }
}

#[test]
fn boot_never_lands_on_an_empty_seat_and_the_empty_seat_is_judged_once() {
    let identity = code_only(&read_src_file("identity.rs").expect("读不出 identity.rs"));
    let at = identity.find("pub fn land_at_boot(").expect("开机落席那一口");
    let body = &identity[at..identity[at..].find("\n}\n").map(|e| at + e).unwrap_or(identity.len())];
    assert!(body.contains("row.first_seat()"), "开机落席与切换身份同一条规矩");
    let action = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    assert_eq!(action.matches("pub fn boot_home(").count(), 1, "开机那一趟只有一处");
    assert!(action.contains("crate::register::change_listed(crate::identity::land_at_boot)"), "开机先落席再开家");
    let window = code_only(&read_src_file("window.rs").expect("读不出 window.rs"));
    assert!(window.contains("crate::action::start(&mut shell)"), "窗子开机走那一处");
    let at = action.find("pub fn start(").expect("开机那一口");
    let start = &action[at..action[at..].find("\n}\n").map(|e| at + e).unwrap_or(action.len())];
    assert!(start.contains("boot_home(shell)"), "开机那一口经开家那一处");
    assert!(!window.contains("crate::identity::home_now("), "窗子不自己问开哪一处家");
    let shell = code_only(&read_src_file("shell.rs").expect("读不出 shell.rs"));
    assert_eq!(shell.matches("pub fn seat_unseated(").count(), 1, "一处判");
    assert!(!window.contains("r.address(seat).is_none()"), "窗子里不再自写「这一席空着」");
    assert!(window.contains("let unseated = self.shell.seat_unseated();"), "身份卡问那一口");
    assert!(window.contains("let seat_ok = !self.shell.seat_unseated();"), "数据卡问同一口");
    for key in ["t(Key::SetChangeHome), Role::Secondary).enabled(seat_ok)", "t(Key::DoMeasure), Role::Secondary).enabled(seat_ok)"] {
        assert!(window.contains(key), "空席上这一键不可按:{key}");
    }
}

fn body_of<'a>(code: &'a str, head: &str) -> &'a str {
    let Some(at) = code.find(head) else { return "" };
    let rest = &code[at..];
    let end = rest[head.len()..]
        .find("\nfn ")
        .into_iter()
        .chain(rest[head.len()..].find("\npub fn "))
        .chain(rest[head.len()..].find("\n    fn "))
        .chain(rest[head.len()..].find("\n    pub fn "))
        .min()
        .map(|i| i + head.len())
        .unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn every_secret_field_is_the_secret_type() {
    let window = code_only(&read_src_file("window.rs").expect("读不出 window.rs"));
    let s = "crate::secret::Secret";
    for (field, ty) in [
        ("id_words", format!("[{s}; 12]")),
        ("id_hex", s.to_string()),
        ("id_ks_pw", s.to_string()),
        ("id_confirm", format!("[{s}; 3]")),
        ("id_pw", s.to_string()),
        ("id_pw2", s.to_string()),
        ("id_pin", s.to_string()),
        ("pin", s.to_string()),
        ("pin_again", s.to_string()),
        ("pin_old", s.to_string()),
        ("pin_words", format!("[{s}; 12]")),
        ("pin_ks_pw", s.to_string()),
    ] {
        assert!(window.contains(&format!("    {field}: {ty},")), "窗子那一格 `{field}` 的类型该是 {ty}");
    }
    let action = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    let enum_body = body_of(&action, "pub enum Action {");
    for line in enum_body.lines() {
        for secret in ["pin:", "again:", "old:", "words:", "password:"] {
            if line.contains(secret) {
                for part in line.split(',') {
                    if let Some(i) = part.find(secret) {
                        let ty = part[i + secret.len()..].trim().trim_end_matches('}').trim();
                        assert_eq!(ty, s, "动作里 `{}` 那一格的类型该是秘密型:{line}", secret.trim_end_matches(':'));
                    }
                }
            }
        }
    }
    assert!(action.contains("Words(crate::secret::Secret)") && action.contains("PrivateKey { key: crate::secret::Secret,"), "导入那几形的秘密也是秘密型");
    let keyfile = body_of(&action, "pub struct KeyFileOut {");
    assert!(keyfile.contains("pub password: crate::secret::Secret,") && keyfile.contains("pub again: crate::secret::Secret,"), "导入时那一份密钥文件的密码也是秘密型");
    let shell_src = code_only(&read_src_file("shell.rs").expect("读不出 shell.rs"));
    assert!(shell_src.contains("pub words: Option<Vec<crate::secret::Secret>>,"), "过口令后显示的十二词是秘密型");
    let identity_src = code_only(&read_src_file("identity.rs").expect("读不出 identity.rs"));
    assert!(identity_src.contains("pub fn words(&self) -> Vec<crate::secret::Secret> {"), "新生成的词的显示副本是秘密型");
    let pin_src = std::fs::read_to_string(src().join("..").join("..").join("eravon-ui").join("src").join("pin.rs")).expect("读不出 pin.rs");
    assert!(pin_src.contains("words: Option<&[crate::secret::Secret]>"), "遮住的显示收秘密型");
    let lib = read_crate_file("eravon-os", "secret.rs").expect("读不出 secret.rs");
    assert!(lib.contains("impl Drop for Secret") && lib.contains("impl std::fmt::Debug for Secret"), "秘密型要自己抹零、自己遮调试输出");
}

#[test]
fn the_twelve_boxes_are_masked_in_the_kit_and_the_gate_does_not_parse_words_each_frame() {
    let ui = |f: &str| code_only(&std::fs::read_to_string(src().join("..").join("..").join("eravon-ui").join("src").join(f)).expect("读不出件库"));
    let pin = ui("pin.rs");
    let grid = body_of(&pin, "pub fn words_grid_marked(");
    assert!(grid.contains("secret_numbered("), "the twelve cells are the masked numbered cell");
    assert!(!grid.contains("input::line") && !grid.contains("line_w("), "no plain cell among the twelve");
    let input = ui("input.rs");
    assert!(body_of(&input, "pub fn secret_numbered(").contains("secret_field("), "the numbered cell is the masked field");
    assert!(body_of(&input, "fn secret_field(").contains(".password(true)"), "遮住那一格恒遮");
    let window = code_only(&read_src_file("window.rs").expect("读不出 window.rs"));
    assert!(!window.contains("identity::from_words("), "窗子里不许每帧解一次助记词(凑不凑得成交给动作层)");
    assert!(!window.contains("mask_grid(ui, \"gate-words\"") && !window.contains("IdShowWords"), "恢复与导入两处不设显示明文的键");
    let back = body_of(&window, "fn gate_back(");
    assert!(back.contains("pin_words.iter_mut()") && back.contains("w.clear()"), "恢复面板按「返回」清掉十二格");
}

#[test]
fn deleting_an_identity_drops_its_recovery_seal() {
    let text = read_src_file("identity.rs").expect("读不出 identity.rs");
    let del = body_of(&text, "pub fn delete(");
    assert!(code_only(del).contains("crate::keybox::drop_recovery(&row.id)?"), "删身份要撤它的恢复封");
    assert!(code_only(del).contains("Known::PrimaryDelete"), "主身份不直接删");
}

#[test]
fn the_post_landing_test_hook_is_set_only_by_test_code() {
    for (name, text) in shipped() {
        let code = code_only(&text);
        if name == "action.rs" {
            assert_eq!(code.matches("set_landed_tamper").count(), 1, "action.rs 里 set_landed_tamper 只许有定义那一处");
        } else {
            assert!(!code.contains("set_landed_tamper"), "{name} calls set_landed_tamper: only test code may set the post-landing test hook");
        }
    }
}

#[test]
fn what_the_node_said_is_dispatched_in_one_place() {
    let sender: Vec<(&str, String)> = ["lib.rs", "node.rs", "batch.rs", "sent.rs"]
        .into_iter()
        .map(|f| (f, code_only(&read_crate_file("eravon-send", f).expect("读不出发送器源"))))
        .collect();
    for k in [
        "NonceUsed", "AlreadyPending", "Underpriced", "GasTooLow", "ContractRefused", "RateLimited", "MethodMissing",
        "NodeAuth", "ServiceUnserved", "ServiceKeyMissing", "WrongChain", "NodeRefused", "NodeTls", "NodeTimeout", "AnswerTooLong", "AnswerNotJson",
    ] {
        let uses: Vec<String> = shipped()
            .into_iter()
            .filter(|(n, _)| n != "fault.rs")
            .filter(|(_, t)| code_only(t).contains(&format!("Known::{k}")))
            .map(|(n, _)| n)
            .collect();
        assert!(uses.is_empty(), "Known::{k} 只许由发送器那一处分派给出,应用里出现在 {uses:?}");
        let given: Vec<&str> = sender.iter().filter(|(n, _)| *n != "lib.rs").filter(|(_, t)| t.contains(&format!("Code::{k}"))).map(|(n, _)| *n).collect();
        assert_eq!(given, vec!["node.rs"], "Code::{k} 只许由 node::said_trouble 那一处分派给出");
    }
    let node = code_only(&read_crate_file("eravon-send", "node.rs").expect("读不出发送器 node.rs"));
    let chainx = code_only(&read_crate_file("eravon-vault", "chainx.rs").expect("读不出 chainx.rs"));
    assert!(node.contains("rpc::ask_node("), "那一扇 https 端点经共用那一处问与读");
    for (name, code) in [("node.rs", &node), ("chainx.rs", &chainx)] {
        assert!(!code.contains("got.body") && !code.contains("member(\"result\")"), "{name}:不另读一份应答");
        assert!(!code.contains("const MARKS"), "{name}:拒因表只一份,在共用层");
    }
}

#[test]
fn what_the_node_said_reads_by_one_table_for_the_app_and_the_command_line() {
    use app::chainx::{said_fault, Refusal};
    use app::fault::Known;
    use eravon_chain::rpc::{self, Trouble};
    let node = |m: &str| Trouble::Node(format!("{{\"code\":-32000,\"message\":\"{m}\"}}"));
    for (said, r, k) in [
        ("insufficient funds", Refusal::Funds, Known::InsufficientFunds),
        ("nonce too low", Refusal::NonceUsed, Known::NonceUsed),
        ("already known", Refusal::Pending, Known::AlreadyPending),
        ("transaction underpriced", Refusal::Underpriced, Known::Underpriced),
        ("intrinsic gas too low", Refusal::GasTooLow, Known::GasTooLow),
        ("execution reverted", Refusal::Reverted, Known::ContractRefused),
        ("too many requests", Refusal::RateLimited, Known::RateLimited),
        ("method not found", Refusal::NoMethod, Known::MethodMissing),
        ("unauthorized", Refusal::Auth, Known::NodeAuth),
        ("wrong chain", Refusal::WrongChain, Known::WrongChain),
        ("something else", Refusal::Coded(String::new()), Known::NodeRefused),
    ] {
        let t = node(said);
        assert_eq!(said_fault("u", &t).which(), Some(k), "{said}");
        if matches!(r, Refusal::Coded(_)) {
            assert!(r.about_the_call() && Known::NETWORK.contains(&k), "{said}");
        } else {
            assert_eq!(r.about_the_call(), !Known::NETWORK.contains(&k), "{said}: the one table and the network family agree");
        }
    }
    let url = "https://n.example";
    assert_eq!(said_fault(url, &rpc::status(url, 429)).which(), Some(Known::RateLimited));
    assert_eq!(said_fault(url, &rpc::status(url, 401)).which(), Some(Known::NodeAuth));
    assert_eq!(said_fault(url, &rpc::status(url, 403)).which(), Some(Known::NodeAuth));
    let refused = said_fault(url, &rpc::status(url, 502));
    assert_eq!(refused.which(), Some(Known::NodeRefused));
    assert!(refused.tail().contains("502"), "the status is in the refusal: {}", refused.tail());
    assert_eq!(said_fault(url, &rpc::shapeless(url)).which(), Some(Known::ChainShape));
    assert_eq!(said_fault(url, &Trouble::Transport(rpc::Broke::NotJson)).which(), Some(Known::AnswerNotJson));
    use app::chainx::Layer;
    use eravon_send::node::{next_after, said_trouble, Next};
    assert_eq!(next_after(&said_trouble(url, &rpc::status(url, 429))), Next::Retry);
    assert_eq!(next_after(&said_trouble(url, &rpc::status(url, 502))), Next::NextEndpoint);
    assert_eq!(next_after(&said_trouble(url, &rpc::shapeless(url))), Next::NextEndpoint);
    assert_eq!(next_after(&said_trouble(url, &node("insufficient funds"))), Next::Stop);
    for l in Layer::ALL {
        assert_eq!(Layer::of(&format!("{} · words", l.code())), Some(l));
    }
    assert_eq!(Layer::of(&format!("{url} 答 HTTP 403")), None);
    assert_eq!(Known::NETWORK.len(), 23);
}

#[test]
fn a_passcode_is_eight_ascii_letters_or_digits_and_patterns_are_judged_on_digits() {
    use app::keybox::{pin_trouble, PinTrouble};
    assert_eq!(pin_trouble("K7m2Xq9p"), None);
    assert_eq!(pin_trouble("Aaaaaaaa"), None);
    assert_eq!(pin_trouble("abcdefgh"), None);
    assert_eq!(pin_trouble("aaaaaaaa"), Some(PinTrouble::AllSame));
    assert_eq!(pin_trouble("K7m2Xq9!"), Some(PinTrouble::Shape));
    assert_eq!(pin_trouble("K7m2Xq9"), Some(PinTrouble::Shape));
    assert_eq!(pin_trouble("2761839\u{ff41}"), Some(PinTrouble::Shape));
    assert_eq!(pin_trouble("23456789"), Some(PinTrouble::Run));
    assert_eq!(pin_trouble("19991231"), Some(PinTrouble::DateLike));
    assert_eq!(pin_trouble("27618394"), None);
}

#[test]
fn every_legal_token_has_its_own_plain_sentence() {
    let row = |k: app::lang::Key| app::lang::TABLE.iter().find(|(x, _, _)| *x == k).map(|(_, zh, en)| (*zh, *en));
    let mut seen: Vec<app::lang::Key> = Vec::new();
    for t in eravon::tokens::Token::ALL {
        let k = app::fault::entry_token_say(t);
        let (zh, en) = row(k).unwrap_or_else(|| panic!("{t:?} 的句不在表里"));
        assert!(!zh.is_empty() && !en.is_empty(), "{t:?} 缺一语");
        assert!(!seen.contains(&k), "{t:?} 与别的 token 共一句");
        seen.push(k);
        let f = app::fault::Fault::entry_refused(t);
        assert_eq!(f.human(), app::lang::t(k), "{t:?} 脸上那半句该是表里那一句");
        assert_eq!(f.tail(), format!("{t:?}"), "{t:?} 证据尾照旧是 token 的名");
        assert!(f.said().starts_with("ENTRY_REFUSED"), "{t:?} 码照旧");
    }
    use eravon_chain::scan::Refusal as R;
    let scans = [
        R::ChainIdMismatch { declared: 1, served: "0x2".into() },
        R::NoEndpoint(1),
        R::Unanswered { chain: 1, asked: eravon_chain::scan::Asked::NotLogs },
        R::TxNotItself("0x".into()),
        R::Malformed(eravon_chain::scan::Shape::Basis),
    ];
    let mut scan_keys: Vec<app::lang::Key> = Vec::new();
    for r in &scans {
        let k = app::fault::scan_refusal_say(r);
        let (zh, en) = row(k).unwrap_or_else(|| panic!("{} 的句不在表里", r.code()));
        assert!(!zh.is_empty() && !en.is_empty());
        assert!(!scan_keys.contains(&k), "{} 与别的拒因共一句", r.code());
        scan_keys.push(k);
        let f = app::fault::Fault::scan_refused(app::fault::Fault::scan_tail(r), std::slice::from_ref(r));
        assert_eq!(f.human(), app::lang::t(k));
        assert!(f.tail().starts_with(r.code()), "证据尾以拒因码起头");
        assert!(!f.tail().chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "证据尾是机器形,不带哪一种话:{}", f.tail());
    }
}

#[test]
fn a_masked_field_turns_the_input_method_off() {
    let input = code_only(&std::fs::read_to_string(src().join("../../eravon-ui/src/input.rs")).expect("读不出 input.rs"));
    let body = body_of(&input, "fn secret_field(");
    assert!(body.contains("egui::Event::Ime(_)"), "握着焦点时输入法事件整枚不收");
    assert!(body.contains("o.ime = None"), "画完撤掉这一帧的输入法请求");
    assert_eq!(input.matches(".password(true)").count(), 1, "遮住的格只此一件");
}

#[test]
fn the_known_deployments_are_compiled_in_and_one_entry_point_fills_a_home() {
    let d = app::deploy::named(app::deploy::DEFAULT).expect("默认行在表里");
    assert_eq!(d.chain_id, 1);
    assert_eq!(d.registry.to_lowercase(), "0x36ea8a857a5fe813429d4d9947000c644a88809a");
    assert_eq!(d.from_block, 26_087_229);
    let t = app::deploy::named("sepolia").expect("测试网行在表里");
    assert_eq!((t.chain_id, t.registry.to_lowercase().as_str(), t.from_block), (11_155_111, "0xc29410b882c4c3b77e33659d2f06ac563e7b08a3", 11_715_660));
    assert!(d.nodes.iter().all(|u| u.starts_with("https://")) && d.nodes[0] != d.nodes[1]);
    let _action = code_only(&read_src_file("action.rs").expect("读不出 action.rs"));
    let machine = code_only(&read_src_file("machine.rs").expect("读不出 machine.rs"));
    assert!(!machine.contains("11155111") && !machine.contains("11_155_111"), "机器级设置档不留链号的第二份说法");
}

#[test]
fn every_fetched_byte_goes_through_the_one_tls_client() {
    for (name, text) in shipped() {
        let t = code_only(&text);
        assert!(!t.contains("TcpStream::connect"), "{name} 自己开了一条连接");
        assert!(!t.contains("rustls::"), "{name} 自己碰了 TLS");
        assert!(!t.contains("drive_trust_root"), "{name} calls the trust root reserved for the test driver");
    }
    let net = code_only(&std::fs::read_to_string(src().join("../../eravon-net/src/lib.rs")).expect("读不出 eravon-net"));
    assert_eq!(net.matches("ClientConfig::builder").count(), 1, "TLS 配置只建一处");
}

#[test]
fn every_boolean_is_the_one_toggle_from_the_kit() {
    for (name, text) in shipped() {
        let t = code_only(&text);
        assert!(!t.contains("ui.checkbox"), "{name} 还有一枚对勾控件");
        assert!(!t.contains(".checkbox("), "{name} 还有一枚对勾控件");
    }
    let window = code_only(&read_src_file("window.rs").expect("读不出 window.rs"));
    assert!(!window.contains("Key::NoticeOn"), "「系统通知 · 已开启」那一行撤了");
    let ui = code_only(&std::fs::read_to_string(src().join("../../eravon-ui/src/toggle.rs")).expect("the switch file reads"));
    assert_eq!(ui.matches("pub fn toggle(").count(), 1, "one switch with a label");
    assert_eq!(ui.matches("pub fn switch(").count(), 1, "one bare switch");
}

#[test]
fn side_by_side_goes_through_the_one_grid() {
    for (name, text) in shipped() {
        assert!(!code_only(&text).contains("ui.columns("), "{name} 还直接调 ui.columns");
    }
    let grid = code_only(&std::fs::read_to_string(src().join("../../eravon-ui/src/grid.rs")).expect("读不出等分格件"));
    assert!(grid.contains("egui::Layout::top_down(egui::Align::Min)"), "格里的布局不两端对齐");
    assert!(!grid.contains("top_down_justified"), "格里的布局不两端对齐");
}

#[test]
fn the_solid_red_key_is_only_on_a_confirm_card() {
    let text = read_src_file("window.rs").expect("读不出 window.rs");
    let lines: Vec<&str> = text.lines().collect();
    let mut at: Vec<(usize, String)> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.contains("page::Pen::new()") {
            let owner = lines[..=i]
                .iter()
                .rev()
                .find(|x| ["fn ", "pub fn ", "pub(super) fn ", "pub(crate) fn "].iter().any(|v| x.trim_start().starts_with(v)))
                .map(|x| x.trim().to_string())
                .unwrap_or_default();
            at.push((i + 1, owner));
        }
    }
    let sheets = [
        "fn id_sheets(",
        "fn bk_sheets(",
    ];
    for (line, owner) in &at {
        assert!(sheets.iter().any(|s| owner.contains(s)), "window.rs:{line} takes the final-step token outside a confirmation sheet: {owner}");
    }
    for s in sheets {
        let want = if s == "fn bk_sheets(" { 2 } else { 1 };
        assert_eq!(at.iter().filter(|(_, o)| o.contains(s)).count(), want, "{s} holds exactly {want} final-step key(s)");
    }
    let erase: Vec<(usize, String)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("page::Erase::press(") || l.contains("page::Erase::press_why("))
        .map(|(i, _)| {
            let owner = lines[..=i].iter().rev().find(|x| ["fn ", "pub fn ", "pub(super) fn ", "pub(crate) fn "].iter().any(|v| x.trim_start().starts_with(v))).map(|x| x.trim().to_string()).unwrap_or_default();
            (i + 1, owner)
        })
        .collect();
    for s in ["fn id_sheets("] {
        assert_eq!(erase.iter().filter(|(_, o)| o.contains(s)).count(), 1, "{s} holds one erase key: {erase:?}");
    }
    assert_eq!(erase.len(), 1, "the erase key only ends a deletion: {erase:?}");
    let page = code_only(&std::fs::read_to_string(src().join("../../eravon-ui/src/page.rs")).expect("读不出 page.rs"));
    assert!(page.contains("pub struct Guide"), "引导键那一枚令牌在件库里");
    assert!(page.contains("pub fn commits_in"), "「确认卡之外零实心深红」那一条现算腿在件库里");
}

#[test]
fn the_embedded_faces_carry_their_licence() {
    use eravon_ui::fonts::{Place, Role, ROLES};
    let embedded: Vec<Role> = ROLES.iter().filter(|(_, _, _, p)| *p == Place::Embedded).map(|(r, _, _, _)| *r).collect();
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        assert_eq!(embedded, vec![Role::Latin, Role::Mono], "macOS 与 Windows 上内嵌只有拉丁与等宽两面");
    } else {
        assert_eq!(embedded.len(), Role::ALL.len(), "其余系统每一面都随二进制走");
    }
    let found = eravon_ui::fonts::find();
    for (role, _, _, place) in ROLES {
        let Some(f) = found.face(role) else {
            assert_eq!(place, Place::System, "{} 是内嵌的,不会找不到", role.as_str());
            continue;
        };
        assert!(f.bytes > 0, "{} 那一面报得出字节数", role.as_str());
        match f.place {
            Place::Embedded => {
                assert_eq!(f.licence(), Some(eravon_ui::fonts::OFL));
                assert!(f.path.is_none(), "内嵌那一面没有盘上的路");
            }
            Place::System => assert!(f.licence().is_none() && f.path.is_some(), "系统取那一面有路、不带许可"),
        }
    }
    let dir = src().join("../../eravon-ui/fonts");
    for name in ["JetBrainsMono-Regular.ttf", "JetBrainsMono-Bold.ttf", "Inter-Regular.ttf", "Inter-Medium.ttf", "Inter-SemiBold.ttf", "OFL-JetBrainsMono.txt", "OFL-Inter.txt"] {
        assert!(dir.join(name).is_file(), "{name} 随仓");
    }
    for name in ["OFL-JetBrainsMono.txt", "OFL-Inter.txt"] {
        let text = std::fs::read_to_string(dir.join(name)).expect("读得出许可");
        assert!(text.contains("SIL OPEN FONT LICENSE"), "{name} 是 OFL 全文");
    }
}

#[test]
fn the_desktop_says_the_connection_layers_before_it_reaches_the_sender() {
    let wiring = ["chainx.rs", "fault.rs"];
    for (name, text) in shipped() {
        let code = code_only(&text);
        if code.contains("eravon_send::") {
            assert!(wiring.contains(&name.as_str()), "{name} reaches the sender outside its wiring");
        }
    }
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-vault/src");
    for e in std::fs::read_dir(&vault).expect("the vault crate").flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.ends_with(".rs") && code_only(&std::fs::read_to_string(e.path()).expect("a source")).contains("eravon_send::") {
            assert!(wiring.contains(&name.as_str()), "{name} reaches the sender outside its wiring");
        }
    }
    let chainx = code_only(&read_crate_file("eravon-vault", "chainx.rs").expect("chainx"));
    for f in ["endpoint_at", "https", "ask", "ask_call", "tx_known", "tx_as_held", "head_block", "head_time", "serving", "balance", "fee_reading"] {
        let body = body_of(&chainx, &format!("pub fn {f}("));
        assert!(!body.is_empty(), "chainx::{f}");
        assert!(body.contains("speak();"), "chainx::{f} says the layers first");
    }
    let node = code_only(&read_crate_file("eravon-send", "node.rs").expect("node.rs"));
    assert_eq!(node.matches("LAYER_WORDS.set(").count(), 1, "the words are set in one place");
    assert!(!node.contains("lang::"), "the sender holds no sentence");
}

#[test]
fn a_small_file_write_syncs_its_directory_after_the_rename() {
    let dir = std::env::temp_dir().join(format!("zk-put-sync-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    app::home::put_at(&dir, "a.json", b"1").expect("lands");
    assert_eq!(std::fs::read(dir.join("a.json")).expect("reads back"), b"1");
    std::fs::write(dir.join(".staged"), b"3").expect("a staged file");
    eravon_os::pretend_sync_fails_at(Some(dir.clone()));
    let put = app::home::put_at(&dir, "a.json", b"2");
    let over = app::home::rename_over(&dir.join(".staged"), &dir.join("b.json"));
    eravon_os::pretend_sync_fails_at(None);
    for (form, r) in [("putAt", put), ("renameOver", over)] {
        let f = r.err().unwrap_or_else(|| panic!("{form}: a directory that cannot be synced makes the write not done"));
        assert!(f.tail().contains(&dir.display().to_string()), "{form}: said with the directory: {}", f.tail());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_homes_lock_file_is_created_owner_only() {
    vault_open();
    let dir = std::env::temp_dir().join(format!("zk-lock-owner-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = app::home::Home::open_or_create(&dir).expect("a home");
    let lock = app::lock::take(&home).expect("the lock");
    assert!(lock.mode().writable(), "the first instance writes");
    assert!(eravon_os::is_owner_only(lock.path()).expect("reads back"), "the lock file is its owner's only");
    drop(lock);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_revert_says_what_the_contract_said() {
    use app::fault::Known;
    use eravon_chain::rpc::Trouble;
    let mut data = vec![0x08, 0xc3, 0x79, 0xa0];
    data.extend([vec![0u8; 31], vec![32], vec![0u8; 31], vec![4], b"nope".to_vec(), vec![0u8; 28]].concat());
    let hex = eravon::hexfmt::encode(&data);
    let with = Trouble::Node(format!("{{\"code\":3,\"message\":\"execution reverted\",\"data\":\"{hex}\"}}"));
    let f = app::chainx::said_fault("http://127.0.0.1:8545", &with);
    assert_eq!(f.which(), Some(Known::ContractRefused));
    assert!(f.tail().ends_with(" \u{b7} reason: nope"), "{}", f.tail());
    let bare = Trouble::Node("{\"code\":3,\"message\":\"execution reverted\"}".into());
    let f = app::chainx::said_fault("http://127.0.0.1:8545", &bare);
    assert_eq!(f.which(), Some(Known::ContractRefused));
    assert!(!f.tail().contains("reason:"), "{}", f.tail());
    let funds = Trouble::Node(format!("{{\"code\":-32000,\"message\":\"insufficient funds\",\"data\":\"{hex}\"}}"));
    assert!(!app::chainx::said_fault("http://127.0.0.1:8545", &funds).tail().contains("reason:"), "only a revert reads the data");
}

#[test]
fn a_revert_asked_of_the_nodes_says_what_the_contract_said() {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let mut s = s;
            let mut buf = vec![0u8; 8192];
            let _ = s.read(&mut buf);
            let mut data = vec![0x08, 0xc3, 0x79, 0xa0];
            data.extend([vec![0u8; 31], vec![32], vec![0u8; 31], vec![4], b"nope".to_vec(), vec![0u8; 28]].concat());
            let body = format!("{{\"error\":{{\"code\":3,\"message\":\"execution reverted\",\"data\":\"{}\"}},\"id\":1,\"jsonrpc\":\"2.0\"}}", eravon::hexfmt::encode(&data));
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
        }
    });
    let eps = vec![app::chainx::Endpoint::at(31337, format!("http://127.0.0.1:{port}"))];
    let f = app::chainx::ask(&eps, "eth_estimateGas", &eravon::json::Value::Arr(Vec::new())).err().expect("refused");
    assert_eq!(f.which(), Some(app::fault::Known::ContractRefused));
    assert!(f.tail().contains("reason: nope"), "{}", f.tail());
}

#[test]
fn two_machine_settings_changes_at_once_both_stay() {
    vault_open();
    let before = app::machine::read().expect("reads");
    let looks = std::thread::spawn(|| {
        for i in 0..40usize {
            let a = app::machine::APPEARANCES[i % app::machine::APPEARANCES.len()];
            app::machine::update(|m| m.appearance = Some(a.to_string())).expect("written");
        }
    });
    let start = before.auto_lock;
    let locks = std::thread::spawn(move || {
        for i in 1..=41u32 {
            app::machine::update(|m| m.auto_lock = if i % 2 == 1 { !start } else { start }).expect("written");
        }
    });
    looks.join().expect("joined");
    locks.join().expect("joined");
    let m = app::machine::read().expect("reads");
    let last_look = app::machine::APPEARANCES[39 % app::machine::APPEARANCES.len()];
    assert_eq!(m.appearance.as_deref(), Some(last_look), "the appearance's last change stayed");
    assert_eq!(m.auto_lock, !start, "the auto-lock switch's last change stayed");
    let (appearance, auto_lock) = (before.appearance.clone(), before.auto_lock);
    app::machine::update(|m| {
        m.appearance = appearance;
        m.auto_lock = auto_lock;
    })
    .expect("put back");
}

fn labelled_home(tag: &str) -> app::home::Home {
    vault_open();
    let dir = std::env::temp_dir().join(format!("zk-std-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = app::home::Home::open_or_create(&dir).expect("建家");
    app::local::check_label(home.root(), None, true).expect("家标");
    home
}

#[test]
fn an_earlier_envelope_is_refused_and_never_written_over() {
    let home = labelled_home("old-env");
    let at = home.dir(app::home::Slot::Settings).join(app::settings::FILE);
    let key = app::keybox::local_key().expect("库开着");
    let plain = b"{\"capBytes\":3}";
    let tag = app::local::Doc::Settings.tag().as_bytes();
    let mut head = app::local::MAGIC.to_vec();
    head.push(tag.len() as u8);
    head.extend_from_slice(tag);
    head.extend_from_slice(&1u16.to_be_bytes());
    let nonce = [9u8; 24];
    let mut old = head.clone();
    old.extend_from_slice(&nonce);
    old.extend_from_slice(&app::cryptx::xchacha_seal(key.bytes(), &nonce, &head, plain).unwrap());
    std::fs::write(&at, &old).unwrap();
    let read = app::local::read(&at, app::local::Doc::Settings).expect_err("旧封具名拒");
    assert_eq!(app::local::Unread::of(&read), Some(app::local::Unread::NotSealed), "{read:?}");
    assert_eq!(std::fs::read(&at).unwrap(), old, "读不重写");
    let wrote = app::local::put(&at.parent().unwrap(), app::settings::FILE, app::local::Doc::Settings, b"{\"capBytes\":4}").expect_err("不盖");
    assert_eq!(app::local::Unread::of(&wrote), Some(app::local::Unread::NotSealed), "{wrote:?}");
    assert_eq!(std::fs::read(&at).unwrap(), old, "写也不盖");
    let _ = std::fs::remove_dir_all(home.root());
}

#[test]
fn a_writer_labels_a_home_and_a_label_of_another_identity_is_refused() {
    vault_open();
    let dir = std::env::temp_dir().join(format!("zk-std-label-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = app::home::Home::open_or_create(&dir).expect("建家");
    app::local::check_label(home.root(), None, false).expect("读者开");
    assert!(!app::local::label_path(home.root()).exists(), "读者不补家标");
    app::local::check_label(home.root(), None, true).expect("写者开");
    let l = app::local::read_label(home.root()).expect("家标读得成").expect("有家标");
    assert_eq!((l.number.len(), l.whose.clone()), (32, None));
    let other = app::local::Label { number: l.number.clone(), whose: Some((format!("0x{}", "aa".repeat(20)), app::roles::Role::Author)) };
    let at = app::local::label_path(home.root());
    std::fs::write(&at, app::local::seal_with(&app::keybox::local_key().unwrap(), &app::local::label_ident(), &other.to_bytes()).unwrap()).unwrap();
    let id_b = format!("0x{}", "bb".repeat(20));
    let f = app::local::check_label(home.root(), Some((&id_b, app::roles::Role::Author)), true).expect_err("别人的家标");
    assert_eq!(app::local::Unread::of(&f), Some(app::local::Unread::Swapped));
    let id_a = format!("0x{}", "AA".repeat(20));
    app::local::check_label(home.root(), Some((&id_a, app::roles::Role::Author)), true).expect("本人的照开");
    let nobody = app::local::Label { number: l.number.clone(), whose: None };
    std::fs::write(&at, app::local::seal_with(&app::keybox::local_key().unwrap(), &app::local::label_ident(), &nobody.to_bytes()).unwrap()).unwrap();
    app::local::check_label(home.root(), Some((&id_b, app::roles::Role::Author)), true).expect("不属于任何身份的家照开");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_task_whose_home_moved_on_writes_nothing() {
    if alone("a_task_whose_home_moved_on_writes_nothing") {
        return;
    }
    vault_open();
    let dir = std::env::temp_dir().join(format!("eravon-std-ticket-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let run = |kind: app::task::Kind, move_on: bool, name: &'static str| -> (bool, Option<app::fault::Known>) {
        let mut t = app::task::Tasks::new();
        let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
        let d = dir.clone();
        let _ = t.spawn(kind, move || {
            let _ = go_rx.recv();
            app::home::put_at(&d, name, b"x").map(|_| app::task::Done::Path(None))
        });
        if move_on {
            t.new_epoch();
        }
        let _ = go_tx.send(());
        let mut said = None;
        for _ in 0..2000 {
            let got = t.drain_at(1.0);
            if let Some(o) = got.iter().find(|o| o.kind == kind) {
                said = o.result.as_ref().err().and_then(|f| f.which());
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        (dir.join(name).exists(), said)
    };
    assert_eq!(run(app::task::Kind::Settings, true, "moved-on"), (false, Some(app::fault::Known::HomeUnreachable)));
    assert_eq!(run(app::task::Kind::Settings, false, "same-epoch"), (true, None));
    assert_eq!(run(app::task::Kind::Backup, true, "not-following"), (true, None));
    assert!(!app::task::ticket_void());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_ticket_is_asked_only_at_the_write_gates() {
    let mut at: Vec<String> = Vec::new();
    for (name, text) in shipped() {
        let n = code_only(&text).matches("ticket_void()").count();
        for _ in 0..n {
            at.push(name.clone());
        }
    }
    at.sort();
    assert_eq!(at, vec!["home.rs".to_string(), "home.rs".to_string(), "house.rs".to_string(), "keybox.rs".to_string(), "task.rs".to_string(), "task.rs".to_string()], "查票只在写口");
    let home = code_only(&read_src_file("home.rs").unwrap());
    let put_at = home.split("pub fn put_at(").nth(1).and_then(|b| b.split("\npub fn ").next()).unwrap_or_default();
    assert!(put_at.contains("ticket_void()"), "put_at 收票");
    let numbered = home.split("pub fn land_numbered(").nth(1).and_then(|b| b.split("\npub fn ").next()).unwrap_or_default();
    assert!(numbered.contains("ticket_void()"), "land_numbered 收票");
}

#[test]
fn a_home_marked_by_another_machine_is_read_only_until_taken_over() {
    vault_open();
    let dir = std::env::temp_dir().join(format!("eravon-std-mark-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = app::home::Home::open_or_create(&dir).expect("建家");
    let at = eravon_store::writer::Home::of(&home.dir(app::home::Slot::Settings).display().to_string());
    let me = app::lock::this_machine().expect("本机标");
    assert_eq!(app::lock::this_machine().unwrap(), me, "本机标不变");
    {
        let l = app::lock::take(&home).expect("取锁");
        assert_eq!(l.mode(), app::lock::Mode::Writer);
        assert_eq!(eravon_store::lock::mark_of(&at), Some(me.clone()), "没有标即记本机");
    }
    let other = "0123456789abcdef0123456789abcdef";
    std::fs::write(app::lock::mark_path(&home), format!("{other}\n")).unwrap();
    let mut l = app::lock::take(&home).expect("取锁");
    assert_eq!(l.mode(), app::lock::Mode::OtherMachine);
    assert!(!l.mode().writable());
    l.take_over(&at).expect("改由本机写");
    assert_eq!((l.mode(), eravon_store::lock::mark_of(&at)), (app::lock::Mode::Writer, Some(me.clone())));
    l.take_over(&at).expect("已是写者,再按无事");
    drop(l);
    std::fs::write(app::lock::mark_path(&home), "not a mark\n").unwrap();
    assert_eq!(app::lock::take(&home).expect("取锁").mode(), app::lock::Mode::OtherMachine);
    assert_eq!(std::fs::read_to_string(app::lock::mark_path(&home)).unwrap(), "not a mark\n");
    assert_eq!(eravon_store::lock::mark_of(&at), None);
    let _ = me;
    let _ = std::fs::remove_dir_all(&dir);
}

fn probe_shell(ctx: &eravon_ui::egui::Context) -> app::shell::Shell {
    vault_open();
    app::shell::Shell::boot(eravon_ui::skin::dress(ctx))
}

#[test]
fn the_shortcuts_read_the_key_the_layout_gives_the_press() {
    use eravon_ui::egui::{Event, Key, Modifiers};
    let settings = app::nav::Route::Root(app::nav::Place::SettingsHome).name();
    let home = app::nav::Route::Root(app::nav::Place::Home).name();
    let chord = |key: Key, physical: Option<Key>| Event::Key { key, physical_key: physical, pressed: true, repeat: false, modifiers: Modifiers::COMMAND };
    for (what, event, want) in [
        ("US: the key is , where , sits", chord(Key::Comma, Some(Key::Comma)), &settings),
        ("AZERTY: , is typed where M sits", chord(Key::Comma, Some(Key::M)), &settings),
        ("no physical key told", chord(Key::Comma, None), &settings),
        ("Dvorak: the key where , sits types W", chord(Key::W, Some(Key::Comma)), &home),
    ] {
        let ctx = eravon_ui::egui::Context::default();
        let (_, said) = app::window::probe_shell_input(&ctx, probe_shell(&ctx), app::nav::Place::Home, vec![(0.1, vec![event]), (0.4, vec![]), (0.4, vec![])], 1180.0, 760.0);
        assert_eq!(&said.last().expect("frames ran").0, want, "{what}");
    }
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock")).expect("the lock file");
    let at = lock.find("name = \"egui-winit\"").expect("egui-winit in the lock");
    assert!(lock[at..].lines().nth(1).is_some_and(|l| l.trim() == "version = \"0.34.3\""), "egui-winit moved: check its logical-or-physical key fallback again");
}

#[test]
fn a_double_click_on_the_windows_handle_maximizes_it() {
    use eravon_ui::egui::{pos2, Event, PointerButton};
    let at = pos2(eravon_ui::tokens::RAIL_W + 200.0, 4.0);
    let press = |p, down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Default::default() };
    let sent = |steps: Vec<(f64, Vec<Event>)>| -> Vec<String> {
        let ctx = eravon_ui::egui::Context::default();
        let (_, said) = app::window::probe_shell_input(&ctx, probe_shell(&ctx), app::nav::Place::Home, steps, 1180.0, 760.0);
        said.into_iter().flat_map(|(_, c)| c).collect()
    };
    let one = vec![(0.1, vec![Event::PointerMoved(at)]), (0.1, vec![press(at, true), press(at, false)]), (0.5, vec![])];
    let two = vec![(0.1, vec![Event::PointerMoved(at)]), (0.05, vec![press(at, true), press(at, false)]), (0.1, vec![press(at, true), press(at, false)]), (0.5, vec![])];
    let drag = vec![(0.1, vec![Event::PointerMoved(at)]), (0.05, vec![press(at, true)]), (0.05, vec![Event::PointerMoved(pos2(at.x + 30.0, at.y + 20.0))]), (0.05, vec![Event::PointerMoved(pos2(at.x + 60.0, at.y + 40.0))]), (0.05, vec![press(pos2(at.x + 60.0, at.y + 40.0), false)])];
    let maxed = |c: &Vec<String>| c.iter().any(|s| s.starts_with("Maximized(true)"));
    let (one, two, drag) = (sent(one), sent(two), sent(drag));
    assert!(!maxed(&one), "one click maximizes nothing: {one:?}");
    assert!(maxed(&two), "a double click maximizes: {two:?}");
    assert!(drag.iter().any(|s| s == "StartDrag") && !maxed(&drag), "dragging moves the window: {drag:?}");
}

#[test]
fn the_window_has_a_least_size_and_every_page_fits_it() {
    use app::nav::{Place, Section, View};
    use eravon_ui::tokens;
    assert_eq!(app::window::MIN_W, tokens::RAIL_W + 2.0 * tokens::PAGE_PAD + tokens::MAIN_MIN_W, "the least width is the layout's floors");
    let drive = read_src_file("window/drive.rs").expect("the window's start");
    assert_eq!(code_only(&drive).matches(".with_min_inner_size([MIN_W, MIN_H])").count(), 1, "the window asks the system for that least size, once");
    let anywhere: usize = shipped().iter().map(|(_, t)| code_only(t).matches("with_min_inner_size").count()).sum();
    assert_eq!(anywhere, 1, "and nowhere else in what ships asks for another least size");
    let ctx = eravon_ui::egui::Context::default();
    let mut places = vec![Place::Home, Place::SettingsHome];
    places.extend(View::ALL.iter().map(|v| Place::View(*v, 0)));
    places.extend(Section::ALL.iter().map(|s| Place::Settings(*s)));
    let mut shell = probe_shell(&ctx);
    for place in places {
        let (back, r) = app::window::probe_face(&ctx, shell, place, app::window::MIN_W, app::window::MIN_H);
        shell = back;
        assert!(r.right_text <= app::window::MIN_W + 0.5 && r.right_widget <= app::window::MIN_W + 0.5, "{place:?} runs past the least width: text to {}, widgets to {}", r.right_text, r.right_widget);
    }
}

#[test]
fn a_slot_is_filled_once_and_what_fills_it_is_never_read_again() {
    use app::lang::{fill1, fill2, fill3, t, Key};
    let with = |n: usize| Key::ALL.iter().copied().find(|k| (0..n).all(|i| t(*k).contains(&format!("{{{i}}}"))) && !t(*k).contains(&format!("{{{n}}}")));
    let k1 = with(1).expect("a sentence with one slot");
    let k2 = with(2).expect("a sentence with two slots");
    let k3 = with(3).expect("a sentence with three slots");
    let s = fill1(k1, "{0}{1}");
    assert!(s.contains("{0}{1}"), "slot 0's value stays as given: {s}");
    let s = fill2(k2, "a{1}b", "Z");
    assert!(s.contains("a{1}b") && s.matches('Z').count() == t(k2).matches("{1}").count(), "slot 1 is filled where the sentence has it, not inside slot 0's value: {s}");
    let s = fill3(k3, "{2}", "{1}", "C");
    assert!(s.contains("{2}") && s.contains("{1}") && s.contains('C'), "three slots, one pass: {s}");
    let s = fill1(k1, "");
    assert!(!s.contains("{0}"), "an empty value still fills its slot: {s}");
}

static PATH_ASKED: std::sync::Mutex<Vec<app::platform::Pick>> = std::sync::Mutex::new(Vec::new());
static PATH_GATE: std::sync::Mutex<Option<std::sync::mpsc::Receiver<Result<Option<String>, app::fault::Fault>>>> = std::sync::Mutex::new(None);

fn stand_in_asker(kind: app::platform::Pick) -> Result<app::platform::Wait, app::fault::Fault> {
    PATH_ASKED.lock().unwrap().push(kind);
    let rx = PATH_GATE.lock().unwrap().take().expect("an answer channel for each dialog");
    Ok(Box::new(move || rx.recv().unwrap_or(Ok(None))))
}

fn refusing_asker(_: app::platform::Pick) -> Result<app::platform::Wait, app::fault::Fault> {
    Err(app::fault::Fault::known(app::fault::Known::DialogUnavailable, String::new()))
}

#[test]
fn the_file_dialog_does_not_hold_the_frame_and_answers_the_place_that_asked() {
    use app::platform::Pick;
    use app::window::{probe_paths, PathFrame};
    let f = |ask: Vec<(&'static str, Pick)>, take: Vec<&'static str>, answer: bool| PathFrame { ask, take, answer };
    let unavailable = |r: &app::window::PathRead| r.faults.iter().filter(|x| x.which() == Some(app::fault::Known::DialogUnavailable)).count() == 1;
    let (tx, rx) = std::sync::mpsc::channel();
    *PATH_GATE.lock().unwrap() = Some(rx);
    PATH_ASKED.lock().unwrap().clear();
    let ctx = eravon_ui::egui::Context::default();
    let mut send = || tx.send(Ok(Some("/somewhere/条款.pdf".to_string()))).unwrap();
    let mut frames = vec![f(vec![("a", Pick::File)], vec![], false), f(vec![("b", Pick::Folder)], vec!["a", "b"], false)];
    frames.extend((0..4).map(|_| f(vec![], vec!["a", "b"], false)));
    frames.push(f(vec![], vec!["b", "a"], true));
    frames.push(f(vec![], vec!["a"], false));
    let (_, r) = probe_paths(&ctx, probe_shell(&ctx), stand_in_asker, &mut send, frames);
    assert!(r[..6].iter().all(|x| x.open), "the dialog stays open while the window draws frame after frame");
    assert!(r[..6].iter().all(|x| x.took.iter().all(|(_, p)| p.is_none())), "nothing lands before the person answers");
    assert_eq!(*PATH_ASKED.lock().unwrap(), vec![Pick::File], "the second ask opened nothing; the first dialog kept its kind");
    assert_eq!(r[6].took, vec![("b", None), ("a", Some("/somewhere/条款.pdf".to_string()))], "the path goes back to the place that asked, and only there");
    assert!(!r[6].open);
    assert_eq!(r[7].took, vec![("a", None)], "taken once");
    let (tx, rx) = std::sync::mpsc::channel();
    *PATH_GATE.lock().unwrap() = Some(rx);
    let (tx2, rx2) = std::sync::mpsc::channel();
    let mut sent = 0;
    let mut cancel = || {
        if sent == 0 {
            tx.send(Ok(None)).unwrap();
            *PATH_GATE.lock().unwrap() = Some(rx2.recv().unwrap());
        }
        sent += 1;
    };
    let (gate_tx, gate_rx) = std::sync::mpsc::channel();
    tx2.send(gate_rx).unwrap();
    let ctx = eravon_ui::egui::Context::default();
    let (_, r) = probe_paths(&ctx, probe_shell(&ctx), stand_in_asker, &mut cancel, vec![f(vec![("a", Pick::Folder)], vec![], false), f(vec![], vec!["a"], true), f(vec![("a", Pick::File)], vec![], false)]);
    assert_eq!(r[1].took, vec![("a", None)], "a cancel lands nothing");
    assert!(!r[1].open && r[2].open, "and the next ask opens a dialog again");
    assert_eq!(PATH_ASKED.lock().unwrap().len(), 3);
    drop(gate_tx);
    let ctx = eravon_ui::egui::Context::default();
    let (_, r) = probe_paths(&ctx, probe_shell(&ctx), refusing_asker, &mut || {}, vec![f(vec![("a", Pick::File)], vec!["a"], false), f(vec![], vec!["a"], false), f(vec![], vec![], false), f(vec![], vec![], false)]);
    assert!(unavailable(&r[0]) && !r[0].open && r[0].took == vec![("a", None)], "refused by name, nothing open, nothing landed");
    assert!(unavailable(&r[3]) && !r[3].open, "said once: the frames after it do not say it again");
    let (tx, rx) = std::sync::mpsc::channel();
    *PATH_GATE.lock().unwrap() = Some(rx);
    let ctx = eravon_ui::egui::Context::default();
    let mut fail = || tx.send(Err(app::fault::Fault::known(app::fault::Known::DialogUnavailable, String::new()))).unwrap();
    let (_, r) = probe_paths(&ctx, probe_shell(&ctx), stand_in_asker, &mut fail, vec![f(vec![("a", Pick::File)], vec![], false), f(vec![], vec!["a"], true), f(vec![], vec!["a"], false), f(vec![], vec![], false)]);
    assert!(!unavailable(&r[0]) && unavailable(&r[1]) && r[1].took == vec![("a", None)], "said by name when it fails");
    assert!(unavailable(&r[3]), "said once: the frames after it do not say it again");
    let (tx, rx) = std::sync::mpsc::channel();
    *PATH_GATE.lock().unwrap() = Some(rx);
    let ctx = eravon_ui::egui::Context::default();
    let mut choose = || tx.send(Ok(Some("/somewhere/x".to_string()))).unwrap();
    let (_, r) = probe_paths(&ctx, probe_shell(&ctx), stand_in_asker, &mut choose, vec![f(vec![("a", Pick::File)], vec![], false), f(vec![], vec![], true), f(vec![], vec![], false), f(vec![], vec![], false), f(vec![], vec!["a"], false)]);
    assert_eq!(r[4].took, vec![("a", None)], "the place that asked was gone for those frames: the answer was dropped");
    assert!(!app::task::Kind::Path.waited_at_quit());
    let mut asks = 0;
    for (name, text) in shipped() {
        let code = code_only(&text);
        assert!(!code.contains("runModal"), "{name}: a modal dialog holds the frame");
        let allowed = if name == "window.rs" { 1 } else { 0 };
        if name != "platform.rs" {
            assert_eq!(code.matches("platform::ask_path").count(), allowed, "{name} opens the dialog itself");
        }
        asks += code.matches("path_answer(").count();
    }
    let faces = code_only(&read_src_file("window/faces.rs").expect("faces"));
    assert_eq!(faces.matches("asker: crate::platform::ask_path").count(), 1, "the window's asker is the system's dialog");
    assert!(asks >= 6, "the places that offer \"choose…\" each ask through the mail: {asks}");
}

#[test]
fn every_landing_goes_through_one_registry() {
    use app::landing::{goes_back, of, Back};
    for k in app::task::Kind::ALL {
        assert_eq!(of(k).kind, k);
        assert_eq!(app::action::lands_said(k), goes_back(k, Back::Said), "{k:?}");
    }
    let landing = read_src_file("window/landing.rs").expect("the window's landing");
    let code = code_only(&landing);
    assert!(!code.contains("o.kind ==") && !code.contains("matches!(o.kind") && !code.contains("(o.kind, o.") && !code.contains("lands_said"), "the window's landing reads kinds through the registry");
    let draw = read_src_file("window/mod.rs").expect("window");
    assert!(code_only(&draw).contains("self.land(ctx, now);") && !code_only(&draw).contains("drain_at("), "draw takes its landings only through `land`");
    let shell = read_src_file("shell.rs").expect("shell");
    assert!(code_only(&shell).contains("crate::landing::goes_back(o.kind, crate::landing::Back::Vault)"), "the shell routes passcode answers by the registry");
}

#[test]
fn the_chain_reading_is_read_in_one_place() {
    let mut stray = Vec::new();
    for (name, text) in shipped() {
        for (n, line) in code_only(&text).lines().enumerate() {
            for who in ["shell.chain", "self.chain"] {
                let mut from = 0;
                while let Some(at) = line[from..].find(who) {
                    let i = from + at + who.len();
                    from = i;
                    let next = line[i..].chars().next().unwrap_or(' ');
                    if next.is_alphanumeric() || next == '_' {
                        continue;
                    }
                    let rest = line[i..].trim_start();
                    let write = rest.starts_with('=') && !rest.starts_with("==");
                    let the_one = name == "shell.rs" && line.contains("Ok(self.chain.as_ref())");
                    if !write && !the_one && (who == "shell.chain" || name == "shell.rs") {
                        stray.push(format!("{name}:{} {}", n + 1, line.trim()));
                    }
                }
            }
        }
    }
    assert!(stray.is_empty(), "the chain reading read outside `chain_reading`: {stray:?}");
}

const ALONE_CHILD: &str = "ERAVON_CHECKS_ALONE";

fn alone_in(module: &str, test: &str) -> bool {
    const CHILD: &str = ALONE_CHILD;
    let full = match module.split_once("::") {
        Some((_, m)) => format!("{m}::{test}"),
        None => test.to_string(),
    };
    if std::env::var(CHILD).as_deref() == Ok(full.as_str()) {
        return false;
    }
    let me = std::env::current_exe().expect("this test binary");
    let out = output_of(std::process::Command::new(me).args([full.as_str(), "--exact", "--nocapture", "--test-threads=1"]).env(CHILD, &full)).expect("the child runs");
    assert!(
        out.status.success() && String::from_utf8_lossy(&out.stdout).contains("1 passed"),
        "{full} (child {})\n  child stdout:\n{}\n  child stderr:\n{}",
        out.status,
        child_lines(&out.stdout),
        child_lines(&out.stderr)
    );
    true
}

fn child_lines(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).lines().map(|l| format!("    | {l}")).collect::<Vec<_>>().join("\n")
}

#[path = "checks_more/gaps.rs"]
mod gaps;
#[path = "checks_more/motion_in_place.rs"]
mod motion_in_place;
#[path = "checks_more/keystore_sync.rs"]
mod keystore_sync;
#[path = "checks_more/window_capture.rs"]
mod window_capture;
#[path = "checks_more/wizard_layout.rs"]
mod wizard_layout;
#[path = "checks_more/estimate_refusals.rs"]
mod estimate_refusals;
#[path = "checks_more/backup_index.rs"]
mod backup_index;
#[path = "checks_more/node_cell_locks.rs"]
mod node_cell_locks;
#[path = "checks_more/door_lock.rs"]
mod door_lock;
#[path = "checks_more/door_queue.rs"]
mod door_queue;
#[path = "checks_more/foreign_home.rs"]
mod foreign_home;
#[path = "checks_more/identity_secrets.rs"]
mod identity_secrets;
#[path = "checks_more/vault_gates.rs"]
mod vault_gates;
#[path = "checks_more/send_gates.rs"]
mod send_gates;
#[path = "checks_more/shell_skeleton.rs"]
mod shell_skeleton;
#[path = "checks_more/homes_landing.rs"]
mod homes_landing;
#[path = "checks_more/local_writes.rs"]
mod local_writes;
#[path = "checks_more/gate_landing.rs"]
mod gate_landing;
#[path = "checks_more/seat_switch.rs"]
mod seat_switch;
#[path = "checks_more/seat_landing.rs"]
mod seat_landing;
#[path = "checks_more/unsafe_confined.rs"]
mod unsafe_confined;
#[path = "checks_more/recovery_seals.rs"]
mod recovery_seals;
#[path = "checks_more/window_habits.rs"]
mod window_habits;
#[path = "checks_more/house_segments.rs"]
mod segments_in_place;
#[path = "checks_more/cli_writes.rs"]
mod cli_writes;
#[path = "checks_more/proxy_locked.rs"]
mod proxy_locked;
#[path = "checks_more/door_serial.rs"]
mod door_serial;
#[path = "checks_more/rest_rows.rs"]
mod rest_rows;
#[path = "checks_more/identity_page.rs"]
mod identity_page;
#[path = "checks_more/first_run.rs"]
mod first_run;
#[path = "checks_more/settled_kept.rs"]
mod settled_kept;
#[path = "checks_more/wiring_points.rs"]
mod wiring_points;
#[path = "checks_more/network_page.rs"]
mod network_page;
#[path = "checks_more/wizard_way_out.rs"]
mod wizard_way_out;
#[path = "checks_more/bilingual_evidence.rs"]
mod bilingual_evidence;
#[path = "checks_more/seat_homes.rs"]
mod seat_homes;
#[path = "checks_more/window_kit.rs"]
mod window_kit;
#[path = "checks_more/walks_live.rs"]
mod walks_live;

#[test]
fn a_broken_way_is_said_in_the_desktops_words_with_machine_evidence() {
    use eravon_chain::rpc::{Broke, Trouble};
    let url = "https://node.example";
    let words = |k: app::lang::Key| app::lang::TABLE.iter().find(|(key, _, _)| *key == k).map(|(_, zh, en)| (*zh, *en)).expect("a row");
    let (wire_zh, wire_en) = words(app::lang::Key::Tail090);
    let (node_zh, node_en) = words(app::lang::Key::Tail089);
    assert!(!wire_zh.is_empty() && !wire_en.is_empty() && !node_zh.is_empty() && !node_en.is_empty());
    let cjk = |x: &str| x.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    for (b, kind) in [
        (Broke::Late { node: url.into(), deadline: std::time::Duration::from_secs(5) }, (wire_zh, wire_en)),
        (Broke::Closed { node: url.into() }, (wire_zh, wire_en)),
        (Broke::NotJson, (wire_zh, wire_en)),
        (Broke::Status { node: url.into(), code: 503 }, (node_zh, node_en)),
    ] {
        let said = app::chainx::trouble_said(url, &Trouble::Transport(b.clone()));
        let evidence = b.evidence();
        assert!(said.ends_with(&evidence), "{said}");
        assert!(!cjk(&evidence), "{evidence}");
        let head = &said[..said.len() - evidence.len()];
        assert!(head.contains(kind.0) || head.contains(kind.1), "the kind in the table's words: {said}");
        assert_eq!(said.matches(url).count(), 1, "the node named once: {said}");
    }
}

#[test]
fn a_key_slot_holding_another_addresss_key_is_refused_for_that_slot_alone() {
    vault_open();
    let mine = app::key::Secret::take([0x41; 32]).expect("a key").address().expect("its address");
    let other = app::key::Secret::take([0x42; 32]).expect("a key").address().expect("its address");
    let slot = app::places::key_slot(&mine);
    assert_eq!(app::places::slot_address(&slot), Some(mine), "the slot name reads back to its address");
    app::keybox::put(&slot, &[0x42; 32]).expect("stored");
    let refused = app::key::load(Some(&slot)).err().expect("another address's key is not this slot's");
    assert_eq!(refused.which(), Some(app::fault::Known::KeyMalformed));
    assert_ne!(other, mine);
    let _ = app::keybox::drop_item(&slot);
    app::keybox::put(&slot, &[0x41; 32]).expect("stored");
    let read = app::key::load(Some(&slot)).expect("its own key reads").expect("a key");
    assert_eq!(read.address(), Some(mine));
    assert_eq!(app::places::slot_address(&app::places::seed_slot("x")), None, "a seed slot names no key's address");
    let _ = app::keybox::drop_item(&slot);
}

#[test]
fn a_file_where_a_folder_should_be_is_refused_by_name() {
    let base = std::env::temp_dir().join(format!("eravon-not-a-folder-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&base);
    let home = base.join("home");
    std::fs::write(&home, b"a plain file").expect("written");
    let at = home.join("settings").join("home-label.json");
    let e = std::fs::read(&at).err().expect("nothing reads below a file");
    let read = at.display().to_string();
    let f = app::fault::classify(&e, &read);
    assert_eq!(f.which(), Some(app::fault::Known::NotAFolder));
    assert_eq!(eravon_seal::Trouble::io(&e, &read).code, eravon_seal::Code::NotAFolder);
    assert_eq!(app::fault::Fault::from(eravon_seal::Trouble::io(&e, &read)).which(), Some(app::fault::Known::NotAFolder));
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_rest_rows_own_refusal_is_said_as_its_own() {
    use app::fault::{Fault, Known};
    use app::lang::{t, Key};
    use eravon_chain::rpc::{Broke, Trouble};
    use eravon_chain::scan::{Asked, Refusal};
    let url = "https://api.etherscan.io/v2/api";
    let unserved = Trouble::Transport(Broke::Unserved { node: url.into(), method: "eth_getBalance".into() });
    let no_key = Trouble::Transport(Broke::NoKey { node: url.into() });
    for (trouble, code, known, what) in [
        (&unserved, eravon_send::Code::ServiceUnserved, Known::ServiceUnserved, Key::FaultWhatServiceUnserved),
        (&no_key, eravon_send::Code::ServiceKeyMissing, Known::ServiceKeyMissing, Key::FaultWhatServiceKeyMissing),
    ] {
        let said = eravon_send::node::said_trouble(url, trouble);
        assert_eq!(said.code, code, "{trouble:?}");
        assert!(said.network(), "{trouble:?}: the next node is asked");
        let f = Fault::from(said);
        assert_eq!((f.which(), f.human()), (Some(known), t(what)), "{trouble:?}");
    }
    let scan = |asked: Asked| app::fault::scan_refusal_say(&Refusal::Unanswered { chain: 1, asked });
    assert_eq!(scan(Asked::Broke { method: "eth_getLogs".into(), broke: Broke::NoKey { node: url.into() } }), Key::TokScanNoKey);
    assert_eq!(scan(Asked::Broke { method: "eth_getLogs".into(), broke: Broke::Unserved { node: url.into(), method: "eth_getLogs".into() } }), Key::TokScanUnserved);
    assert_eq!(scan(Asked::Declined { method: "eth_getLogs".into(), said: "{\"code\":-32600,\"message\":\"query exceeds max block range 50000\"}".into(), width: Some((0, 250_000)) }), Key::TokScanTooWide);
    assert_eq!(scan(Asked::Declined { method: "eth_getLogs".into(), said: "{\"code\":-32000,\"message\":\"internal error\"}".into(), width: None }), Key::TokScanUnanswered);
    assert_eq!(scan(Asked::NotLogs), Key::TokScanUnanswered);
}
