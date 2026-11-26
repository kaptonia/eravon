#![windows_subsystem = "windows"]

#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use app::house::bundle;
use app::house::drive::{self, Run};
use app::house::i18n::{self, Lang};
use app::house::seam;
use app::house::steps as trace;
use eravon_literals::json::{Out, write};
use eravon_ui::report;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;
const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn main() -> ExitCode {
    eravon_ui::trace::install(eravon_literals::trace::mark_code);
    let console = app::house::platform::console();
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("eravon-app", app::house::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
    eravon_scan::evidence::keep_blocks_on_machine();
    eravon_scan::judged::keep_by(|| app::chainx::sealed_room().map(|r| r as std::sync::Arc<eravon_scan::judged::Room>));
    if args.is_empty() || args[0].starts_with("--") {
        return if launched_as_window() || !console { gui(&args) } else { misuse(&Misuse::verb(None)) };
    }
    if args[0] == seam::V_WINDOW {
        return gui(&args[1..]);
    }
    match run(&args) {
        Ok(v) => {
            print(&v);
            ExitCode::from(eravon_literals::schema::exit_of(&v))
        }
        Err(m) => misuse(&m),
    }
}

fn argv() -> Option<Vec<String>> {
    std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect()
}

fn launched_as_window() -> bool {
    std::env::var(seam::ENV_LAUNCH).is_ok_and(|v| v == seam::V_WINDOW)
}

fn print(v: &Out) {
    write_out(&write(v));
}


fn misuse(m: &Misuse) -> ExitCode {
    m.say("eravon-app");
    ExitCode::from(EXIT_MISUSE)
}


fn write_out(bytes: &[u8]) {
    use std::io::Write;
    let out = std::io::stdout();
    let mut h = out.lock();
    if h.write_all(&eravon_literals::echo::scrub(bytes)).is_err() || h.flush().is_err() {
        std::process::exit(EXIT_MISUSE as i32);
    }
}

struct Flags {
    pairs: Vec<(String, String)>,
}

impl Flags {
    fn one(&self, k: &str) -> Option<&str> {
        self.pairs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    fn need(&self, k: &str) -> Result<String, Misuse> {
        self.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k))
    }
    fn many(&self, k: &str) -> Vec<String> {
        self.pairs.iter().filter(|(a, _)| a == k).map(|(_, v)| v.clone()).collect()
    }
}

fn flags(args: &[String]) -> Result<Flags, Misuse> {
    Ok(Flags { pairs: eravon_literals::schema::read_pairs(args)? })
}

fn read_bounded(path: &str) -> Result<Vec<u8>, Misuse> {
    let m = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    if !m.is_file() || m.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
    }
    std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    use app::house::seam;

    const C_CHAIN: &str = "eravon.chain/1 · 端点与三分法闭集(E_CHAIN_USAGE · E_CHAIN_TRANSPORT · E_CHAIN_ENDPOINT · E_CHAIN_EXECUTION · E_CHAIN_DISAGREE · E_CHAIN_WRONG)";

    pub const COMMON: &[&str] = &["E_APP_IO"];

    macro_rules! scene {
        ($($own:expr),*) => {
            &[
                Arg::need("--data", T_PATH),
                Arg::need("--script", T_PATH),
                Arg::opt("--lang", T_STR),
                Arg::opt("--budget-ms", T_INT),
                Arg::opt("--now", T_INT),
                Arg::opt("--registry", T_ARR),
                Arg::opt("--home", T_PATH),
                Arg::opt("--passcode", T_STR),
                Arg::opt("--zone-offset", T_STR),
                $($own),*
            ]
        };
    }

    pub const ALL: &[Verb] = &[
        Verb::plain(seam::V_SCHEMA, &[]),
        Verb::plain(seam::V_FORM, &[]),
        Verb::plain(seam::V_SKIN, &[]),
        Verb::new(seam::V_I18N, &[Arg::opt("--lang", T_STR)], &["E_APP_I18N"], &[]),
        Verb::new(seam::V_DRIVE, scene!(), &["E_APP_SCRIPT"], &[C_CHAIN]),
        Verb::new(seam::V_PAGE_FRAMES, scene!(Arg::need("--page", T_ARR), Arg::opt("--width", T_INT), Arg::opt("--look", T_STR)), &["E_APP_SCRIPT"], &[]),
        Verb::new(seam::V_TRACE, &[Arg::need("--data", T_PATH)], &["E_APP_IO"], &[]),
        Verb::new(
            seam::V_BUNDLE,
            &[Arg::need("--at", T_PATH), Arg::opt("--data", T_PATH), Arg::opt("--from", T_PATH)],
            &["E_APP_IO"],
            &[],
        ),
        Verb::window(
            seam::V_WINDOW,
            &[
                Arg::opt("--data", T_PATH),
                Arg::opt("--lang", T_STR),
                Arg::opt("--url", T_STR),
            ],
            &["E_APP_NO_DATA"],
            &[],
        ),
    ];
}

fn known(verb: &str) -> bool {
    register::ALL.iter().any(|v| v.verb == verb)
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args[0].as_str();
    if !known(verb) {
        return Err(Misuse::verb(Some(verb)));
    }
    let f = flags(&args[1..])?;
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    match verb {
        seam::V_SCHEMA => {
            let mut rows = eravon_literals::schema::out_rows(eravon_literals::schema::register(
                &eravon_literals::schema::version("eravon-app"),
                register::ALL,
                register::COMMON,
            ));
            rows.push(("ok", Out::B(true)));
            rows.extend(app::house::vocab::schema_rows());
            for (k, v) in rows.iter_mut() {
                if *k != "register" {
                    continue;
                }
                if let Out::A(verbs) = v {
                    for row in verbs.iter_mut() {
                        if let Out::O(m) = row {
                            if m.iter().any(|(k, x)| *k == "verb" && matches!(x, Out::S(n) if n == seam::V_DRIVE)) {
                                m.push(("steps", Out::A(seam::STEPS.iter().map(|x| Out::S((*x).to_string())).collect())));
                                m.push(("pages", Out::A(seam::PAGES.iter().map(|x| Out::S((*x).to_string())).collect())));
                            }
                        }
                    }
                }
            }
            Ok(Out::O(rows))
        }
        seam::V_FORM => Ok(app::window::form_answer()),
        seam::V_SKIN => Ok(skin_answer()),
        seam::V_I18N => i18n_answer(&f),
        seam::V_DRIVE => drive_answer(&f),
        seam::V_PAGE_FRAMES => page_frames_answer(&f),
        seam::V_TRACE => trace_answer(&f),
        seam::V_BUNDLE => bundle_answer(&f),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn watch(budget_ms: u64) -> Result<(), String> {
    drive::watch(budget_ms, |said| {
        let v = refuse("E_APP_SCRIPT", said);
        print(&v);
        std::process::exit(i32::from(eravon_literals::schema::exit_of(&v)));
    })
}

fn refuse(token: &str, detail: String) -> Out {
    Out::O(vec![
        ("detail", Out::S(detail)),
        ("ok", Out::B(false)),
        ("token", Out::S(token.to_string())),
    ])
}

fn skin_answer() -> Out {
    let hex = |c: u32| Out::S(format!("#{c:06x}"));
    let keys: Vec<Out> = report::key_rows()
        .into_iter()
        .map(|(name, fill, text, line)| {
            Out::O(vec![
                ("fill", hex(fill)),
                ("line", hex(line)),
                ("role", Out::S(name.to_string())),
                ("text", hex(text)),
            ])
        })
        .collect();
    let found = eravon_ui::fonts::find();
    let faces: Vec<Out> = eravon_ui::fonts::Role::ALL
        .iter()
        .map(|r| {
            let mut m = vec![("role", Out::S(r.as_str().to_string()))];
            match found.face(*r) {
                Some(f) => {
                    m.push(("file", Out::S(f.file.to_string())));
                    m.push(("index", Out::N(f.index as u64)));
                    m.push(("place", Out::S(f.place.as_str().to_string())));
                    m.push(("path", Out::S(f.path.as_ref().map(|x| x.display().to_string()).unwrap_or_default())));
                    m.push((
                        "licence",
                        Out::S(f.licence().unwrap_or("system").to_string()),
                    ));
                }
                None => m.push(("file", Out::S(String::new()))),
            }
            m.sort_by(|a, b| a.0.cmp(b.0));
            Out::O(m)
        })
        .collect();
    let mut looks: Vec<(&'static str, Out)> = Vec::new();
    for l in report::LOOKS {
        let mut rows: Vec<(&'static str, Out)> = report::token_rows()
            .into_iter()
            .filter(|(look, ..)| *look == l)
            .map(|(_, name, rgb, a)| {
                let mut cell = vec![("hex", hex(rgb))];
                if a < 255 {
                    cell.push(("alpha", Out::N(a as u64)));
                }
                cell.sort_by(|x, y| x.0.cmp(y.0));
                (name, Out::O(cell))
            })
            .collect();
        rows.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        rows.push(("distinct", Out::N(report::look_distinct(l) as u64)));
        rows.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        looks.push((l, Out::O(rows)));
    }
    Out::O(vec![
        ("fonts", Out::A(faces)),
        ("keys", Out::A(keys)),
        ("looks", Out::O(looks)),
        ("ok", Out::B(true)),
    ])
}

fn i18n_answer(f: &Flags) -> Result<Out, Misuse> {
    if let Some(l) = f.one("--lang")
        && i18n::lang_of(l).is_none()
    {
        return Err(bad_lang(l));
    }
    let list = |v: Vec<&'static str>| {
        Out::A(v.into_iter().map(|k| Out::S(k.to_string())).collect::<Vec<_>>())
    };
    let check = i18n::self_check();
    let mut m = vec![
        ("byKind", by_kind()),
        ("count", Out::N(i18n::words().len() as u64)),
        ("keysEn", list(i18n::keys(Lang::En))),
        ("keysZh", list(i18n::keys(Lang::Zh))),
        ("ok", Out::B(true)),
        ("onlyEn", list(i18n::only_in(Lang::En, Lang::Zh))),
        ("onlyZh", list(i18n::only_in(Lang::Zh, Lang::En))),
        ("keysetChecker", Out::S(match i18n::keyset_self_check() {
            Ok(()) => "ok".to_string(),
            Err(e) => e.to_string(),
        })),
        ("checker", Out::S(match i18n::checker_self_check() {
            Ok(()) => "ok".to_string(),
            Err(e) => e.to_string(),
        })),
        ("wording", Out::S(match check {
            Ok(()) => "ok".to_string(),
            Err(e) => e.to_string(),
        })),
    ];
    if let Some(l) = f.one("--lang") {
        let lang = i18n::lang_of(l).ok_or_else(|| bad_lang(l))?;
        let mut words: Vec<Out> = i18n::words()
            .iter()
            .map(|w| {
                Out::O(vec![
                    ("key", Out::S(w.key.to_string())),
                    ("kind", Out::S(w.kind.name().to_string())),
                    ("text", Out::S(i18n::get_named(lang, w.key).0)),
                ])
            })
            .collect();
        words.extend(app::lang::TABLE.iter().map(|(k, zh, en)| {
            Out::O(vec![
                ("key", Out::S(format!("family.{k:?}"))),
                ("kind", Out::S("family".to_string())),
                ("text", Out::S(if lang == Lang::Zh { zh } else { en }.to_string())),
            ])
        }));
        m.push(("words", Out::A(words)));
    }
    m.sort_by(|a, b| a.0.cmp(b.0));
    Ok(Out::O(m))
}

fn by_kind() -> Out {
    let mut m: Vec<(&'static str, Out)> = Vec::new();
    for k in [i18n::Kind::Status, i18n::Kind::Button, i18n::Kind::Hint, i18n::Kind::Name] {
        let n = i18n::words().iter().filter(|w| w.kind == k).count();
        m.push((k.name(), Out::N(n as u64)));
    }
    m.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(m)
}

fn zone_probe(f: &Flags) -> Result<(), Misuse> {
    let Some(v) = f.one("--zone-offset") else { return Ok(()) };
    let p = if v == "unread" {
        eravon_orders::when::Probe::Unread
    } else {
        eravon_orders::when::Probe::Offset(v.parse::<i64>().map_err(|_| Misuse::bad("--zone-offset", v, eravon_literals::misuse::said("要是秒数(东为正)或 unread", "seconds (east positive) or unread")))?)
    };
    eravon_orders::when::inject(p);
    Ok(())
}

struct Scene {
    data: String,
    bytes: Vec<u8>,
    lang: Lang,
    budget_ms: u64,
    now: u64,
    registries: Vec<String>,
    home: String,
    passcode: Option<String>,
}

impl Scene {
    fn run(&self) -> Run<'_> {
        Run { data: &self.data, lang: self.lang, budget_ms: self.budget_ms, now: self.now, door_tallied: Default::default(), registries: self.registries.clone(), home: self.home.clone(), passcode: self.passcode.clone() }
    }
}

fn scene(f: &Flags) -> Result<Scene, Misuse> {
    let data = f.need("--data")?;
    let script = f.need("--script")?;
    let lang = match f.one("--lang") {
        None => Lang::Zh,
        Some(l) => i18n::lang_of(l).ok_or_else(|| bad_lang(l))?,
    };
    let budget_ms: u64 = match f.one("--budget-ms") {
        None => 4000,
        Some(s) => s.parse().map_err(|_| Misuse::not_int("--budget-ms", s))?,
    };
    let now: u64 = match f.one("--now") {
        None => 0,
        Some(s) => s.parse().map_err(|_| Misuse::not_int("--now", s))?,
    };
    let bytes = read_bounded(&script)?;
    let registries = f.many("--registry");
    let home = f.one("--home").unwrap_or_default().to_string();
    let passcode = f.one("--passcode").map(str::to_string);
    zone_probe(f)?;
    Ok(Scene { data, bytes, lang, budget_ms, now, registries, home, passcode })
}

fn drive_answer(f: &Flags) -> Result<Out, Misuse> {
    let sc = scene(f)?;
    let (r, bytes, budget_ms) = (sc.run(), &sc.bytes, sc.budget_ms);
    if let Err(e) = watch(budget_ms) {
        return Ok(refuse("E_APP_SCRIPT", e));
    }
    match drive::run(bytes, &r) {
        Err(e) => Ok(refuse("E_APP_SCRIPT", e)),
        Ok(d) => {
            let mut rows = drive::rows(&d);
            rows.push(("ok", Out::B(true)));
            rows.sort_by(|a, b| a.0.cmp(b.0));
            Ok(Out::O(rows))
        }
    }
}

fn page_frames_answer(f: &Flags) -> Result<Out, Misuse> {
    use app::house::seller::Page;
    use app::house::shell::Seat;
    let sc = scene(f)?;
    let width: u32 = f.one("--width").map_or(Ok(1180), |s| s.parse().map_err(|_| Misuse::not_int("--width", s)))?;
    let dark = match f.one("--look") {
        None | Some("light") => false,
        Some("dark") => true,
        Some(x) => return Err(Misuse::bad("--look", x, eravon_literals::misuse::said("要是 light 或 dark", "light or dark"))),
    };
    let mut pages: Vec<app::window::FrameTarget> = Vec::new();
    for p in f.many("--page") {
        if p == "here" {
            pages.push(app::window::FrameTarget::Here);
            continue;
        }
        let bad_page = || Misuse::bad("--page", &p, eravon_literals::misuse::said("要是 here 或 席.页(seller.orders、buyer.wallet 之类)", "here or seat.page (seller.orders, buyer.wallet and the like)"));
        let (seat, name) = p.split_once('.').ok_or_else(bad_page)?;
        let seat = match seat {
            "seller" => Seat::Seller,
            "buyer" => Seat::Buyer,
            _ => return Err(bad_page()),
        };
        let page = match name {
            "home" => None,
            "orders" => Some(Page::Orders),
            "goods" => Some(Page::Goods),
            "cands" => Some(Page::Cands),
            "payways" => Some(Page::Payways),
            "alerts" => Some(Page::Alerts),
            "settings" => Some(Page::Settings),
            "paylink" => Some(Page::PayLink),
            "wallet" => Some(Page::Wallet),
            "export" => Some(Page::Export),
            "queue" => Some(Page::Queue),
            _ => return Err(bad_page()),
        };
        pages.push(app::window::FrameTarget::Page(seat, page));
    }
    if pages.is_empty() {
        return Err(Misuse::missing("--page"));
    }
    let r = sc.run();
    if let Err(e) = watch(r.budget_ms) {
        return Ok(refuse("E_APP_SCRIPT", e));
    }
    match drive::run(&sc.bytes, &r) {
        Err(e) => Ok(refuse("E_APP_SCRIPT", e)),
        Ok(d) => {
            let read = match app::window::page_frames(d.shell, &pages, width as f32, dark) {
                Ok(r) => r,
                Err(e) => return Ok(refuse("E_APP_SCRIPT", e)),
            };
            Ok(Out::O(vec![("look", Out::S(if dark { "dark" } else { "light" }.to_string())), ("ok", Out::B(true)), ("pages", read), ("width", Out::N(u64::from(width)))]))
        }
    }
}

fn trace_answer(f: &Flags) -> Result<Out, Misuse> {
    let data = f.need("--data")?;
    match trace::load(&data) {
        Err(e) => Ok(refuse("E_APP_IO", e)),
        Ok(bytes) => {
            let doc = eravon_literals::json::read(&bytes).ok().map(|v| v.get("steps").cloned());
            let n = match doc {
                Some(Some(eravon_literals::json::In::Arr(a))) => a.len(),
                _ => 0,
            };
            Ok(Out::O(vec![
                ("bytes", Out::N(bytes.len() as u64)),
                ("ok", Out::B(true)),
                ("path", Out::S(seam::trace_path(&data))),
                ("steps", Out::N(n as u64)),
            ]))
        }
    }
}

fn bundle_answer(f: &Flags) -> Result<Out, Misuse> {
    let at = f.need("--at")?;
    let from = match f.one("--from") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_exe().map_err(|_| Misuse::missing("--from"))?,
    };
    match bundle::lay_out(&at, &from, f.one("--data")) {
        Err(e) => Ok(refuse("E_APP_IO", e)),
        Ok(l) => Ok(Out::O(vec![
            ("app", Out::S(l.app)),
            ("bundleId", Out::S(eravon_literals::literals::BUNDLE_ID.to_string())),
            ("cli", Out::S(l.cli)),
            ("executable", Out::S(l.exe)),
            ("launch", Out::S(l.launch)),
            ("ok", Out::B(true)),
            ("plist", Out::S(l.plist)),
            ("scheme", Out::S(eravon_literals::literals::URL_SCHEME.to_string())),
            ("urlDelivery", Out::S(l.url_delivery.to_string())),
        ])),
    }
}

fn gui(args: &[String]) -> ExitCode {
    let f = match flags(args) {
        Ok(f) => f,
        Err(m) => return misuse(&m),
    };
    if let Err(m) = eravon_literals::schema::form_gate(register::ALL, seam::V_WINDOW, &f.pairs) {
        return misuse(&m);
    }
    let data = f.one("--data").map(str::to_string).or_else(bundle::data_root_from_bundle);
    if let Some(l) = f.one("--lang") {
        match i18n::lang_of(l) {
            Some(x) => {
                i18n::set(x);
                app::lang::set(x);
            }
            None => return misuse(&bad_lang(l)),
        }
    }
    if let Some(data) = &data {
        if let Err(e) = app::records::bind(data) {
            app::house::platform::alert(e.said());
            let out = refuse("E_APP_NO_DATA", format!("{} · {}", e.said(), e.tail()));
            print(&out);
            return ExitCode::from(eravon_literals::schema::exit_of(&out));
        }
    }
    if let Ok(root) = app::house::data()
        && !app::house::openurl::hold_door(&root)
        && let Some(u) = f.one("--url")
        && app::house::openurl::leave(&root, u).is_ok()
    {
        return ExitCode::SUCCESS;
    }
    if let Some(u) = f.one("--url") {
        app::house::openurl::push(u.to_string());
    }
    if let Err(e) = app::house::openurl::arm() {
        app::house::platform::alert(&e);
    }
    ExitCode::from(u8::try_from(app::window::run()).unwrap_or(EXIT_MISUSE))
}

fn bad_lang(given: &str) -> Misuse {
    Misuse::bad("--lang", given, eravon_literals::misuse::said("要是 zh 或 en", "zh or en"))
}
