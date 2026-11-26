#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::process::ExitCode;

use eravon_literals::json::{Out, write};
use eravon_zikaron as a4;
use a4::fault::{Absent, Fault};
use a4::seam;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erz", eravon_zikaron::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
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

fn print(v: &Out) {
    write_out(&write(v));
}


fn misuse(m: &Misuse) -> ExitCode {
    m.say("erz");
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
}

impl Flags {
    fn one(&self, k: &str) -> Option<&str> {
        self.pairs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    fn many(&self, k: &str) -> Vec<&str> {
        self.pairs.iter().filter(|(a, _)| a == k).map(|(_, v)| v.as_str()).collect()
    }
    fn need(&self, k: &str) -> Result<String, Misuse> {
        self.one(k).map(str::to_string).ok_or_else(|| Misuse::missing(k))
    }
}

fn flags(args: &[String]) -> Result<Flags, Misuse> {
    Ok(Flags { pairs: eravon_literals::schema::read_pairs(args.get(1..).unwrap_or_default())? })
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_SHEET: &str = "courtevidence/1 · 清单判定与 manifest(b)(K6 eravon-evidence)";

    pub const COMMON: &[&str] = &["E_ZK_IO"];

    pub const ALL: &[Verb] = &[
        Verb::new(
            "status",
            &[Arg::need("--home", T_PATH), Arg::opt("--machine", T_PATH)],
            &["E_ZK_INDEX_FORM", "E_ZK_INDEX_SHAPE"],
            &[],
        ),
        Verb::new(
            "kits",
            &[Arg::need("--home", T_PATH), Arg::opt("--machine", T_PATH)],
            &["E_ZK_INDEX_FORM", "E_ZK_INDEX_SHAPE"],
            &[],
        ),
        Verb::new(
            "pack",
            &[
                Arg::need("--home", T_PATH),
                Arg::need("--kit", T_STR),
                Arg::opt("--machine", T_PATH),
                Arg::opt("--file", T_PATH),
                Arg::opt("--media-type", T_STR),
                Arg::opt("--party", T_STR),
                Arg::opt("--subject", T_STR),
                Arg::opt("--note", T_STR),
            ],
            &["E_ZK_INDEX_FORM", "E_ZK_INDEX_SHAPE", "E_ZK_NO_SUCH_KIT", "E_ZK_PACK"],
            &[C_SHEET],
        ),
        Verb::plain("schema", &[]),
    ];
}

fn known(verb: &str) -> bool {
    register::ALL.iter().any(|v| v.verb == verb)
}

fn run(args: &[String]) -> Result<Out, Misuse> {
    let verb = args.first().ok_or_else(|| Misuse::verb(None))?.as_str();
    if !known(verb) {
        return Err(Misuse::verb(Some(verb)));
    }
    let f = flags(args)?;
    eravon_literals::schema::form_gate(register::ALL, verb, &f.pairs)?;
    match verb {
        "schema" => {
            let mut rows = eravon_literals::schema::out_rows(eravon_literals::schema::register(
                &eravon_literals::schema::version("erz"),
                register::ALL,
                register::COMMON,
            ));
            rows.push(("ok", Out::B(true)));
            Ok(Out::O(rows))
        }
        "status" => Ok(status(&f)?),
        "kits" => Ok(kits(&f)?),
        "pack" => Ok(pack(&f)?),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn refuse(e: &Fault) -> Out {
    Out::O(vec![
        ("detail", Out::S(e.detail())),
        ("ok", Out::B(false)),
        ("token", Out::S(e.name().to_string())),
    ])
}

fn absent_rows(a: &Absent) -> Vec<(&'static str, Out)> {
    vec![
        ("absent", Out::S(a.name().to_string())),
        ("detail", Out::S(a.detail())),
    ]
}

fn manual(f: &Flags) -> Option<String> {
    f.one("--machine").map(str::to_string)
}

fn status(f: &Flags) -> Result<Out, Misuse> {
    let home = f.need("--home")?;
    let m = manual(f);
    match a4::survey(&home, m.as_deref()) {
        Err(e) => match e.absent() {
            Some(a) => {
                let mut rows = absent_rows(a);
                rows.push(("connected", Out::B(false)));
                if let Some(at) = a.at() {
                    rows.push(("at", Out::S(at.to_string())));
                }
                rows.push(("ok", Out::B(true)));
                rows.sort_by(|x, y| x.0.cmp(y.0));
                Ok(Out::O(rows))
            }
            None => Ok(refuse(&e)),
        },
        Ok(s) => {
            let mut rows = vec![
                ("at", Out::S(s.link.at.clone())),
                ("connected", Out::B(true)),
                ("count", Out::N(s.kits.len() as u64)),
                ("layer", Out::S(s.link.layer.name().to_string())),
                ("ok", Out::B(true)),
            ];
            if let Some(a) = &s.absent {
                rows.extend(absent_rows(a));
            }
            rows.sort_by(|x, y| x.0.cmp(y.0));
            Ok(Out::O(rows))
        }
    }
}

fn kits(f: &Flags) -> Result<Out, Misuse> {
    let home = f.need("--home")?;
    let m = manual(f);
    match a4::survey(&home, m.as_deref()) {
        Err(e) => match e.absent() {
            Some(a) => {
                let mut rows = absent_rows(a);
                rows.push(("connected", Out::B(false)));
                rows.push(("kits", Out::A(Vec::new())));
                rows.push(("ok", Out::B(true)));
                rows.sort_by(|x, y| x.0.cmp(y.0));
                Ok(Out::O(rows))
            }
            None => Ok(refuse(&e)),
        },
        Ok(s) => {
            let list: Vec<Out> = s
                .kits
                .iter()
                .map(|k| {
                    Out::O(vec![
                        ("contents", Out::N(k.contents.len() as u64)),
                        ("created", Out::N(k.created)),
                        ("id", Out::S(k.id.clone())),
                        ("link", Out::S(k.link.clone())),
                        ("noteMd", Out::S(k.note_md.clone())),
                        ("path", Out::S(k.path.clone())),
                        ("root", Out::S(k.root.clone())),
                    ])
                })
                .collect();
            let mut rows = vec![
                ("at", Out::S(s.link.at.clone())),
                ("connected", Out::B(true)),
                ("kits", Out::A(list)),
                ("layer", Out::S(s.link.layer.name().to_string())),
                ("ok", Out::B(true)),
            ];
            if let Some(a) = &s.absent {
                rows.extend(absent_rows(a));
            }
            rows.sort_by(|x, y| x.0.cmp(y.0));
            Ok(Out::O(rows))
        }
    }
}

fn pack(f: &Flags) -> Result<Out, Misuse> {
    let home = f.need("--home")?;
    let id = f.need("--kit")?;
    let m = manual(f);
    let media = f.one("--media-type").unwrap_or(seam::MEDIA_DEFAULT).to_string();
    let party = f.one("--party").unwrap_or("").to_string();
    let subject = f.one("--subject").unwrap_or("").to_string();
    let note = f.one("--note").map(str::to_string);
    let plain: Vec<String> = f.many("--file").into_iter().map(str::to_string).collect();

    let s = match a4::survey(&home, m.as_deref()) {
        Ok(s) => s,
        Err(e) => return Ok(refuse(&e)),
    };
    if let Some(a) = &s.absent {
        let mut rows = absent_rows(a);
        rows.push(("at", Out::S(s.link.at.clone())));
        rows.push(("connected", Out::B(true)));
        rows.push(("ok", Out::B(true)));
        rows.sort_by(|x, y| x.0.cmp(y.0));
        return Ok(Out::O(rows));
    }
    let kit = match a4::index::pick(&s.kits, &id) {
        Ok(k) => k,
        Err(e) => return Ok(refuse(&e)),
    };
    let mut files = match a4::pack::items(kit, &media) {
        Ok(v) => v,
        Err(e) => return Ok(refuse(&e)),
    };
    let from_kit = files.len() as u64;
    for p in &plain {
        match plain_item(p, &media) {
            Ok(it) => files.push(it),
            Err(e) => return Ok(refuse(&e)),
        }
    }
    let subject_ref = if subject.is_empty() {
        eravon_galeed::Value::Null
    } else {
        eravon_galeed::Value::Str(subject.clone())
    };
    let (_m, bytes, digest) = match a4::pack::manifest(subject_ref, &party, files.clone(), note) {
        Ok(x) => x,
        Err(t) => return Ok(Out::O(vec![("ok", Out::B(false)), ("token", Out::S(t.name().to_string()))])),
    };
    let items: Vec<Out> = files
        .iter()
        .map(|x| {
            Out::O(vec![
                ("bytes", Out::N(x.bytes())),
                ("mediaType", Out::S(x.media_type().to_string())),
                ("name", Out::S(x.name().to_string())),
                ("sha256", Out::S(x.sha256().to_string())),
            ])
        })
        .collect();
    Ok(Out::O(vec![
        ("at", Out::S(s.link.at.clone())),
        ("files", Out::A(items)),
        ("fromKit", Out::N(from_kit)),
        ("kitId", Out::S(kit.id.clone())),
        ("link", Out::S(kit.link.clone())),
        ("manifest", Out::S(hex_of(&digest))),
        ("manifestBytes", Out::N(bytes.len() as u64)),
        ("ok", Out::B(true)),
        ("prefix", Out::S(seam::kit_prefix().to_string())),
    ]))
}

fn plain_item(path: &str, media: &str) -> a4::fault::R<a4::pack::FileItem> {
    let bytes = eravon_store::atomic::read_file(path)
        .map_err(|e| Fault::Io(format!("读 {path}:{}", e.detail())))?;
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| Fault::Arg(format!("{path} 的档名不是 UTF-8")))?
        .to_string();
    eravon_evidence::sheet::listed(&name, &bytes, media).map_err(|u| Fault::Arg(format!("{path}:{}", u.name())))
}

fn hex_of(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(2 + 64);
    s.push_str("0x");
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
