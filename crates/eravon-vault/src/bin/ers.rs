#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use eravon_literals::json::{Out, write};
use eravon_store as a5;
use a5::fault::Fault;
use a5::record::Kind;
use a5::seam::Box_;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;
const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("ers", eravon_store::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
    match eravon_chain::task::entry(|| run(&args)) {
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

fn print(v: &Out) {
    write_out(&write(v));
}


fn misuse(m: &Misuse) -> ExitCode {
    m.say("ers");
    ExitCode::from(EXIT_MISUSE)
}


fn write_out(bytes: &[u8]) {
    use std::io::Write;
    let out = std::io::stdout();
    let mut h = out.lock();
    if h.write_all(bytes).is_err() || h.flush().is_err() {
        std::process::exit(EXIT_MISUSE as i32);
    }
}

struct Flags {
    pairs: Vec<(String, String)>,
    bare: Vec<String>,
}

impl Flags {
    fn one(&self, k: &str) -> Option<&str> {
        self.pairs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    fn many(&self, k: &str) -> Vec<&str> {
        self.pairs.iter().filter(|(a, _)| a == k).map(|(_, v)| v.as_str()).collect()
    }
    fn has(&self, k: &str) -> bool {
        self.bare.iter().any(|a| a == k)
    }
}

const BARE: &[&str] = &["--seller-ready", "--buyer-ready"];

fn flags(args: &[String]) -> Result<Flags, Misuse> {
    let (pairs, bare) = eravon_literals::schema::read_flags(args.get(1..).unwrap_or_default(), BARE)?;
    Ok(Flags { pairs, bare })
}

fn split_once(arg: &str, s: &str, c: char) -> Result<(String, String), Misuse> {
    let i = s.find(c).ok_or_else(|| Misuse::bad(arg, s, eravon_literals::misuse::said("要写成 名与值以分隔符相连", "a name and a value joined by the separator")))?;
    Ok((s[..i].to_string(), s[i + c.len_utf8()..].to_string()))
}

fn read_bounded(path: &str) -> Result<Vec<u8>, Misuse> {
    let m = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    if !m.is_file() || m.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
    }
    std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))
}

fn ok(mut m: Vec<(&'static str, Out)>) -> Out {
    m.push(("ok", Out::B(true)));
    Out::O(m)
}

fn refused(f: &Fault) -> Out {
    Out::O(vec![
        ("detail", Out::S(f.detail())),
        ("ok", Out::B(false)),
        ("token", Out::S(f.name().to_string())),
    ])
}

fn ans(r: a5::fault::R<Out>) -> Result<Out, Misuse> {
    Ok(match r {
        Ok(v) => v,
        Err(f) => refused(&f),
    })
}

fn need(f: &Flags, k: &str) -> Result<String, Misuse> {
    f.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k))
}

fn triple(f: &Flags) -> Result<(String, String, u64), Misuse> {
    Ok((
        need(f, "--data")?,
        need(f, "--writer")?,
        { let v = need(f, "--now")?; v.parse::<u64>().map_err(|_| Misuse::not_int("--now", &v))? },
    ))
}

fn box_of(f: &Flags) -> Result<Box_, Misuse> {
    let b = need(f, "--box")?;
    Box_::from_dir(&b).ok_or_else(|| Misuse::bad("--box", &b, eravon_literals::misuse::said("要是存储表里的一格", "a box of the storage table")))
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    pub const COMMON: &[&str] = &["E_STORE_IO", "E_STORE_NO_KEY", "E_STORE_SEALED"];
    const W: [&str; 3] = ["E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE"];
    pub const ALL: &[Verb] = &[
        Verb::new("init", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT)], &["E_STORE_TIME", "E_STORE_NAME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE", "E_STORE_UNREAD"], &[]),
        Verb::new("identity-init", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT), Arg::need("--address", T_STR)], &["E_STORE_TIME", "E_STORE_NAME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE", "E_STORE_UNREAD"], &[]),
        Verb::new("put", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT), Arg::need("--address", T_STR), Arg::need("--id", T_STR), Arg::need("--kind", T_STR), Arg::opt("--field", T_ARR), Arg::opt("--part", T_ARR)], &["E_STORE_KIND", "E_STORE_NAME", "E_STORE_TIME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE", "E_STORE_UNREAD", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("delete", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT), Arg::need("--address", T_STR), Arg::need("--kind", T_STR), Arg::need("--id", T_STR)], &["E_STORE_TIME", "E_STORE_NAME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE", "E_STORE_MISSING", "E_STORE_KIND"], &[]),
        Verb::new("get", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--address", T_STR), Arg::need("--box", T_STR), Arg::need("--id", T_STR)], &["E_STORE_ROOT", "E_STORE_NAME", "E_STORE_MISSING", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("part", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--address", T_STR), Arg::need("--box", T_STR), Arg::need("--id", T_STR), Arg::need("--name", T_STR), Arg::opt("--out", T_PATH)], &["E_STORE_ROOT", "E_STORE_NAME", "E_STORE_MISSING", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("open-file", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--path", T_STR), Arg::opt("--out", T_PATH)], &["E_STORE_NAME", "E_STORE_MISSING", "E_STORE_VERSION", "E_STORE_FORM", "E_STORE_KIND"], &[]),
        Verb::new("list", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--address", T_STR), Arg::need("--box", T_STR)], &["E_STORE_ROOT", "E_STORE_NAME", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("identities", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR)], &["E_STORE_FORM", "E_STORE_VERSION"], &[]),
        Verb::new("name", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--kind", T_STR), Arg::need("--logical", T_STR)], &["E_STORE_NAME"], &[]),
        Verb::new("settings-put", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT), Arg::opt("--active-seller", T_STR), Arg::opt("--active-buyer", T_STR), Arg::opt("--language", T_STR), Arg::opt("--seller-ready", T_BOOL), Arg::opt("--buyer-ready", T_BOOL), Arg::opt("--past-cases", T_BOOL)], &["E_STORE_TIME", "E_STORE_NAME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE", "E_STORE_UNREAD"], &[]),
        Verb::new("settings-get", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR)], &["E_STORE_MISSING", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("identity-settings-put", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT), Arg::need("--address", T_STR), Arg::opt("--payout", T_STR), Arg::opt("--chain", T_INT), Arg::opt("--endpoint", T_ARR)], &["E_STORE_TIME", "E_STORE_NAME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE", "E_STORE_UNREAD", "E_STORE_FORM"], &[]),
        Verb::new("identity-settings-get", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--address", T_STR)], &["E_STORE_NAME", "E_STORE_MISSING", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("candidates-put", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT), Arg::need("--address", T_STR), Arg::opt("--entry", T_ARR)], &["E_STORE_TIME", "E_STORE_NAME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE", "E_STORE_UNREAD", "E_STORE_CAP"], &[]),
        Verb::new("candidates-get", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--address", T_STR)], &["E_STORE_ROOT", "E_STORE_NAME", "E_STORE_MISSING", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("export", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT), Arg::need("--out", T_PATH)], &["E_STORE_TIME", "E_STORE_ARCHIVE", "E_STORE_VERSION", "E_STORE_FORM"], &[]),
        Verb::new("import", &[Arg::need("--bundle", T_PATH), Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR)], &["E_STORE_ARCHIVE", "E_STORE_VERSION", "E_STORE_FORM", "E_STORE_EXISTS"], &[]),
        Verb::new("hold", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR), Arg::need("--writer", T_STR), Arg::need("--now", T_INT)], &["E_STORE_TIME", "E_STORE_NAME", "E_STORE_LOCKED", "E_STORE_OTHER_MACHINE", "E_STORE_MACHINE"], &[]),
        Verb::new("lock-status", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR)], &[], &[]),
        Verb::new("take-over", &[Arg::need("--data", T_PATH), Arg::opt("--passcode", T_STR)], &W, &[]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args.first().map(String::as_str).ok_or_else(|| Misuse::verb(None))?;
    let f = flags(args)?;
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    let mut session = None;
    if let (Some(p), Some(data)) = (f.one("--passcode"), f.one("--data")) {
        let mut s = eravon_vault::session::Session::locked(data);
        if let Err(e) = s.unlock(p) {
            return Ok(Out::O(vec![
                ("detail", Out::S(e.detail())),
                ("ok", Out::B(false)),
                ("token", Out::S(e.name().to_string())),
            ]));
        }
        session = Some(s);
    }
    let answered = dispatch(verb, &f);
    drop(session);
    answered
}

fn dispatch(verb: &str, f: &Flags) -> Result<Out, Misuse> {
    match verb {
        "init" => init(&f),
        "identity-init" => identity_init(&f),
        "put" => put(&f),
        "get" => get(&f),
        "delete" => delete(&f),
        "part" => part(&f),
        "open-file" => open_file(&f),
        "list" => list(&f),
        "identities" => identities(&f),
        "name" => name(&f),
        "settings-put" => settings_put(&f),
        "settings-get" => settings_get(&f),
        "identity-settings-put" => identity_settings_put(&f),
        "identity-settings-get" => identity_settings_get(&f),
        "candidates-put" => candidates_put(&f),
        "candidates-get" => candidates_get(&f),
        "export" => export(&f),
        "import" => import(&f),
        "hold" => hold(&f),
        "lock-status" => lock_status(&f),
        "take-over" => take_over(&f),
        "schema" => Ok(ok(eravon_literals::schema::out_rows(eravon_literals::schema::register(
            &eravon_literals::schema::version("ers"),
            register::ALL,
            register::COMMON,
        )))),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn init(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    ans((|| {
        let s = eravon_vault::store::open_write(&data, &writer, now)?;
        let g = a5::settings::Global::default();
        s.init(&g)?;
        Ok(ok(vec![
            ("data", Out::S(data.clone())),
            ("identities", Out::N(s.identities()?.len() as u64)),
        ]))
    })())
}

fn identity_init(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    let addr = need(f, "--address")?;
    ans((|| {
        let s = eravon_vault::store::open_write(&data, &writer, now)?;
        s.init_identity(&addr, now)?;
        Ok(ok(vec![
            ("address", Out::S(addr.clone())),
            ("boxes", Out::N(Box_::ALL.len() as u64)),
        ]))
    })())
}

fn put(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    let addr = need(f, "--address")?;
    let id = need(f, "--id")?;
    let kind_word = need(f, "--kind")?;
    let mut fields = Vec::new();
    for kv in f.many("--field") {
        fields.push(split_once("--field", kv, '=')?);
    }
    let mut parts = Vec::new();
    for np in f.many("--part") {
        let (n, path) = split_once("--part", np, '=')?;
        parts.push((n, read_bounded(&path)?));
    }
    ans((|| {
        let kind = Kind::need(&kind_word)?;
        let s = eravon_vault::store::open_write(&data, &writer, now)?;
        let rec = s.put(&addr, kind, &id, &fields, &parts, now)?;
        Ok(ok(vec![("record", rec.row())]))
    })())
}

fn delete(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    let addr = need(f, "--address")?;
    let id = need(f, "--id")?;
    let kind = need(f, "--kind")?;
    ans((|| {
        let k = a5::record::Kind::from_word(&kind)
            .ok_or_else(|| a5::fault::Fault::Kind(format!("{kind} 不在十二类闭集里")))?;
        let s = eravon_vault::store::open_write(&data, &writer, now)?;
        s.delete(&addr, k, &id)?;
        Ok(ok(vec![("deleted", Out::S(id.clone())), ("kind", Out::S(kind.clone()))]))
    })())
}

fn get(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    let addr = need(f, "--address")?;
    let b = box_of(f)?;
    let id = need(f, "--id")?;
    ans((|| {
        let s = eravon_vault::store::open_read(&data)?;
        let rec = s.get(&addr, b, &id)?;
        let at = eravon_vault::records::record_path(&addr, b, &id, rec.kind).map_err(eravon_vault::records::store_fault)?;
        let logical = at.strip_prefix(&data).map(|r| r.display().to_string()).unwrap_or_else(|_| at.display().to_string());
        let mut fields = rec.fields.clone();
        fields.sort_by(|x, y| x.0.as_bytes().cmp(y.0.as_bytes()));
        Ok(ok(vec![
            (
                "fields",
                Out::A(
                    fields
                        .into_iter()
                        .map(|(k, v)| Out::O(vec![("k", Out::S(k)), ("v", Out::S(v))]))
                        .collect(),
                ),
            ),
            ("logical", Out::S(logical)),
            ("partNames", Out::A(rec.parts.iter().map(|(n, _)| Out::S(n.clone())).collect())),
            ("record", rec.row()),
        ]))
    })())
}

fn part(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    let addr = need(f, "--address")?;
    let b = box_of(f)?;
    let id = need(f, "--id")?;
    let name = need(f, "--name")?;
    let out = f.one("--out").map(str::to_string);
    ans((|| {
        let s = eravon_vault::store::open_read(&data)?;
        let bytes = s.part(&addr, b, &id, &name)?;
        if let Some(o) = &out {
            a5::atomic::write_atomic(o, &bytes)?;
        }
        Ok(ok(vec![
            ("bytes", Out::N(bytes.len() as u64)),
            ("name", Out::S(name.clone())),
        ]))
    })())
}

fn open_file(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    let rel = need(f, "--path")?;
    let out = f.one("--out").map(str::to_string);
    ans((|| {
        use eravon_vault::records::store_fault;
        let p = std::path::Path::new(&rel);
        if p.is_absolute() || p.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            return Err(a5::fault::Fault::Name(format!("{rel} 要是数据根里的相对路径")));
        }
        eravon_vault::records::bind(&data).map_err(store_fault)?;
        let at = std::path::Path::new(&data).join(&rel);
        let found = eravon_vault::local::all_files().map_err(store_fault)?.into_iter().find(|x| x.at == at);
        let Some(x) = found else { return Err(a5::fault::Fault::Sealed(format!("swapped · {rel}"))) };
        let plain = eravon_vault::local::read(&x.at, x.doc).map_err(store_fault)?.ok_or_else(|| a5::fault::Fault::Missing(rel.clone()))?;
        let logical = eravon_vault::local::logical_rel(x.doc, &x.rel, &plain).map_err(store_fault)?;
        if let Some(o) = &out {
            a5::atomic::write_atomic(o, &plain)?;
        }
        Ok(ok(vec![
            ("bytes", Out::N(plain.len() as u64)),
            ("disk", Out::S(rel.clone())),
            ("kind", Out::S(x.doc.tag().to_string())),
            ("logical", Out::S(logical)),
        ]))
    })())
}

fn identities(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    ans((|| {
        let s = eravon_vault::store::open_read(&data)?;
        let ids = s.identities()?;
        Ok(ok(vec![("count", Out::N(ids.len() as u64)), ("identities", Out::A(ids.into_iter().map(Out::S).collect()))]))
    })())
}

fn name(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    let kind_word = need(f, "--kind")?;
    let kind = a5::seam::NameKind::of_tag(&kind_word).ok_or_else(|| Misuse::bad("--kind", &kind_word, eravon_literals::misuse::ONE_OF))?;
    let logical = need(f, "--logical")?;
    ans((|| {
        use eravon_vault::names::Logical as L;
        use eravon_vault::records::store_fault;
        eravon_vault::records::bind(&data).map_err(store_fault)?;
        let l = match kind {
            a5::seam::NameKind::Identity => L::Home(&logical),
            a5::seam::NameKind::Box(Box_::Products) => L::Products(&logical),
            a5::seam::NameKind::Box(Box_::Orders) => L::Orders(&logical),
            a5::seam::NameKind::Box(Box_::Cases) => L::Cases(&logical),
            a5::seam::NameKind::Box(Box_::Packs) => L::Packs(&logical),
            a5::seam::NameKind::Staging => L::Staging(&logical),
        };
        let token = eravon_vault::names::key().map_err(store_fault)?.name(l);
        Ok(ok(vec![("kind", Out::S(kind.tag().to_string())), ("token", Out::S(token))]))
    })())
}

fn list(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    let addr = need(f, "--address")?;
    let b = box_of(f)?;
    ans((|| {
        let s = eravon_vault::store::open_read(&data)?;
        let rows = s.list(&addr, b)?;
        Ok(ok(vec![
            ("box", Out::S(b.dir().to_string())),
            ("count", Out::N(rows.len() as u64)),
            ("rows", Out::A(rows.iter().map(a5::record::Record::row).collect())),
        ]))
    })())
}

fn settings_put(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    ans((|| {
        let s = eravon_vault::store::open_write(&data, &writer, now)?;
        let mut g = eravon_vault::store::global_to_change(&data)?;
        if f.one("--active-seller").is_some() && f.one("--active-buyer").is_some() {
            return Err(a5::fault::Fault::Form("--active-seller 与 --active-buyer 只给一个:本机同一时刻只有一个当前身份".into()));
        }
        for (flag, seat) in [("--active-seller", a5::vault::Seat::Seller), ("--active-buyer", a5::vault::Seat::Buyer)] {
            if let Some(v) = f.one(flag) {
                eravon_vault::accounts::switch(&data, seat, v).map_err(|e| a5::fault::Fault::Missing(format!("{} {}", e.name(), e.detail())))?;
            }
        }
        if let Some(v) = f.one("--language") {
            let lang = eravon_vault::lang::Lang::ALL.into_iter().find(|l| l.as_str() == v).ok_or_else(|| a5::fault::Fault::Form(format!("--language 只收 zh 或 en,给的是 {v}")))?;
            eravon_vault::machine::update(|m| m.lang = Some(lang)).map_err(eravon_vault::records::store_fault)?;
        }
        if f.has("--seller-ready") {
            g.seller_ready = true;
        }
        if f.has("--buyer-ready") {
            g.buyer_ready = true;
        }
        if let Some(v) = f.one("--past-cases") {
            g.past_cases = match v {
                "true" => true,
                "false" => false,
                _ => return Err(a5::fault::Fault::Form(format!("--past-cases 只收 true 或 false,给的是 {v}"))),
            };
        }
        g.last_writer = s.writer().to_string();
        eravon_vault::store::write_global(&data, &g)?;
        Ok(ok(vec![("lastWriter", Out::S(g.last_writer.clone()))]))
    })())
}

fn settings_get(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    ans((|| {
        let g = eravon_vault::store::read_global(&data)?;
        let active = |seat| eravon_vault::accounts::active(&data, seat).map(Option::unwrap_or_default).map_err(|e| a5::fault::Fault::NoKey(format!("{} {}", e.name(), e.detail())));
        Ok(ok(vec![
            ("activeBuyer", Out::S(active(a5::vault::Seat::Buyer)?)),
            ("activeSeller", Out::S(active(a5::vault::Seat::Seller)?)),
            ("buyerReady", Out::B(g.buyer_ready)),
            ("language", Out::S(eravon_vault::machine::read().map_err(eravon_vault::records::store_fault)?.lang.map(|l| l.as_str().to_string()).unwrap_or_default())),
            ("lastWriter", Out::S(g.last_writer.clone())),
            ("sellerReady", Out::B(g.seller_ready)),
        ]))
    })())
}

fn identity_settings_put(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    let addr = need(f, "--address")?;
    ans((|| {
        let _s = eravon_vault::store::open_write(&data, &writer, now)?;
        let mut t = eravon_vault::store::identity_to_change(&data, &addr)?;
        if let Some(v) = f.one("--payout") {
            t.payout = v.to_string();
        }
        eravon_vault::store::write_identity(&data, &addr, &t)?;
        if let Some(c) = f.one("--chain") {
            let c: u64 = c.parse().map_err(|_| eravon_store::fault::Fault::Form(format!("--chain 要链号:{c}")))?;
            eravon_vault::store::choose_network(&data, &addr, c)?;
        }
        let e = f.many("--endpoint");
        if !e.is_empty() {
            let _ = eravon_vault::store::endpoints(&data, &addr)?;
            eravon_vault::store::write_endpoints(&data, &addr, &e.iter().map(|x| x.to_string()).collect::<Vec<_>>())?;
        }
        Ok(ok(vec![("address", Out::S(addr.clone()))]))
    })())
}

fn identity_settings_get(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    let addr = need(f, "--address")?;
    ans((|| {
        let t = eravon_vault::store::read_identity(&data, &addr)?;
        Ok(ok(vec![
            ("company", Out::S(t.company.clone())),
            ("companyMode", Out::B(t.company_on())),
            ("companyPast", Out::A(t.company_past.iter().map(|s| Out::S(s.clone())).collect())),
            ("endpoints", Out::A(eravon_vault::store::rpc_endpoints(&data, &addr)?.into_iter().map(Out::S).collect())),
            ("payout", Out::S(t.payout.clone())),
            ("timeCheck", Out::B(t.time_check_on())),
        ]))
    })())
}

fn candidates_put(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    let addr = need(f, "--address")?;
    let mut rows = Vec::new();
    for e in f.many("--entry") {
        let p: Vec<&str> = e.split(':').collect();
        if p.len() != 4 {
            return Err(Misuse::bad("--entry", e, eravon_literals::misuse::said("要是四段以冒号分开", "four parts joined by colons")));
        }
        let t: u64 = p[3].parse().map_err(|_| Misuse::bad("--entry", e, eravon_literals::misuse::said("要是四段以冒号分开,末段是十进制整数", "four parts joined by colons, the last a decimal integer")))?;
        rows.push((p[0].to_string(), p[1].to_string(), p[2].to_string(), t));
    }
    ans((|| {
        let s = eravon_vault::store::open_write(&data, &writer, now)?;
        s.put_candidates(&addr, &rows, now)?;
        Ok(ok(vec![("count", Out::N(rows.len() as u64))]))
    })())
}

fn candidates_get(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    let addr = need(f, "--address")?;
    ans((|| {
        let s = eravon_vault::store::open_read(&data)?;
        let rows = s.candidates(&addr)?;
        Ok(ok(vec![
            ("count", Out::N(rows.len() as u64)),
            (
                "entries",
                Out::A(
                    rows.iter()
                        .map(|(k, a, l, t)| {
                            Out::O(vec![
                                ("addedAt", Out::N(*t)),
                                ("address", Out::S(a.clone())),
                                ("kind", Out::S(k.clone())),
                                ("label", Out::S(l.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]))
    })())
}

fn export(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    let out = need(f, "--out")?;
    ans((|| {
        let bytes = a5::archive::export(&data, &writer, now)?;
        a5::atomic::write_atomic(&out, &bytes)?;
        let (dirs, files, _, _) = a5::archive::read_bundle(&bytes)?;
        Ok(ok(vec![
            ("bundleBytes", Out::N(bytes.len() as u64)),
            ("dirs", Out::N(dirs.len() as u64)),
            ("files", Out::N(files.len() as u64)),
        ]))
    })())
}

fn import(f: &Flags) -> Result<Out, Misuse> {
    let from = need(f, "--bundle")?;
    let bundle = read_bounded(&from)?;
    let data = need(f, "--data")?;
    ans((|| {
        let n = a5::archive::import(&bundle, &data, &from)?;
        Ok(ok(vec![("files", Out::N(n as u64))]))
    })())
}

fn hold(f: &Flags) -> Result<Out, Misuse> {
    let (data, writer, now) = triple(f)?;
    ans((|| {
        let s = eravon_vault::store::open_write(&data, &writer, now)?;
        ready_line(&writer, now);
        let mut sink = String::new();
        let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut sink);
        drop(s);
        Ok(ok(vec![("held", Out::B(false)), ("released", Out::B(true)), ("writer", Out::S(writer))]))
    })())
}

fn ready_line(writer: &str, now: u64) {
    use std::io::Write;
    let line = eravon_literals::json::write(&ok(vec![
        ("held", Out::B(true)),
        ("pid", Out::N(u64::from(std::process::id()))),
        ("since", Out::N(now)),
        ("writer", Out::S(writer.to_string())),
    ]));
    let err = std::io::stderr();
    let mut h = err.lock();
    let _ = h.write_all(&line);
    let _ = h.write_all(b"\n");
    let _ = h.flush();
}

fn lock_status(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    ans((|| {
        Ok(match a5::writer::probe(&data)? {
            Some(h) => ok(vec![
                ("held", Out::B(true)),
                ("pid", Out::N(u64::from(h.pid))),
                ("since", Out::N(h.since)),
                ("writer", Out::S(h.writer)),
            ]),
            None => ok(vec![("held", Out::B(false))]),
        })
    })())
}

fn take_over(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    ans((|| {
        let mode = a5::writer::take_over(&data)?;
        if mode == a5::lock::Mode::Reader {
            return Err(Fault::Locked { holder: format!("{:?}", a5::writer::probe(&data)?.map(|w| w.writer).unwrap_or_default()) });
        }
        Ok(ok(vec![("mode", Out::S(mode.as_str().to_string()))]))
    })())
}
