#![forbid(unsafe_code)]

use eravon_literals::misuse::Misuse;
use std::path::Path;
use std::process::ExitCode;

use eravon_evidence as k6;
use eravon_galeed as k1;
use eravon_galeed::Value;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_MISUSE: u8 = 2;
const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn argv() -> Option<Vec<String>> {
    std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect()
}

fn main() -> ExitCode {
    let Some(args) = argv() else {
        return misuse(&Misuse::not_text());
    };
    eravon_literals::misuse::keep_arguments(&args);
    eravon_literals::trace::entry("erv", eravon_evidence::mark::FEATURE, args.first().map(String::as_str).unwrap_or(""));
    match run(&args) {
        Ok(v) => {
            print(&v);
            ExitCode::from(eravon_galeed::schema::exit_of(&v))
        }
        Err(m) => misuse(&m),
    }
}

fn print(v: &Value) {
    write_out(&eravon_galeed::schema::answer_bytes(v));
}


fn misuse(m: &Misuse) -> ExitCode {
    m.say("erv");
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

fn s(k: &str, v: String) -> (String, Value) {
    (k.into(), Value::Str(v))
}

fn read_file(path: &str) -> Result<Vec<u8>, Misuse> {
    let meta = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))?;
    if !meta.is_file() || meta.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, eravon_literals::misuse::TOO_BIG));
    }
    std::fs::read(path).map_err(|_| Misuse::unreadable(path, eravon_literals::misuse::UNREADABLE))
}

mod register {
    use eravon_literals::schema::{Arg, Verb};
    #[allow(unused_imports)]
    use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

    const C_EVIDENCE: &str = "courtevidence/1 · 清单接入闭集(galeed_evidence::Fault)";
    pub const ALL: &[Verb] = &[
        Verb::carry("check", &[Arg::need("sheet-file", T_PATH)], &[C_EVIDENCE]),
        Verb::plain("digest", &[Arg::need("any-file", T_PATH)]),
        Verb::new("recipient-check", &[Arg::need("recipient", T_STR)], &["E_RECIPIENT"], &[]),
        Verb::carry("listing", &[Arg::need("listing-dir", T_PATH), Arg::need("subject-file", T_PATH), Arg::need("party", T_STR), Arg::need("media-type", T_STR)], &[C_EVIDENCE]),
        Verb::plain("schema", &[]),
    ];
}

fn run(args: &[String]) -> Result<Value, Misuse> {
    match (args.first().map(String::as_str), args.len()) {
        (Some("schema"), 1) => {
            let mut m = k1::schema::value_rows(eravon_literals::schema::register(
                &eravon_literals::schema::version("erv"),
                register::ALL,
                &[],
            ));
            m.push(("ok".into(), Value::Bool(true)));
            Ok(Value::Obj(m))
        }
        (Some("check"), 2) => {
            let b = read_file(&args[1])?;
            Ok(match k6::sheet::check(&b).and_then(|_| k6::sheet::manifest(&b)) {
                Ok(d) => Value::Obj(vec![
                    s("manifest", k1::hash::hex_encode(&d)),
                    ("ok".into(), Value::Bool(true)),
                ]),
                Err(f) => fault(&f),
            })
        }
        (Some("digest"), 2) => {
            let b = read_file(&args[1])?;
            let (len, sha) = k6::sheet::file_view(&b);
            Ok(Value::Obj(vec![
                ("bytes".into(), Value::Int(len)),
                s("sha256", k1::hash::hex_encode(&sha)),
            ]))
        }
        (Some("recipient-check"), 2) => Ok(Value::Obj(if k6::recipients::form_ok(&args[1]) {
            vec![("ok".into(), Value::Bool(true))]
        } else {
            vec![("ok".into(), Value::Bool(false)), s("token", "E_RECIPIENT".into())]
        })),
        (Some("listing"), 5) => listing(&args[1], &args[2], &args[3], &args[4]),
        _ => Err(eravon_literals::schema::positional_misuse(register::ALL, args)),
    }
}

fn fault(f: &k6::sheet::Fault) -> Value {
    let mut m: Vec<(String, Value)> =
        vec![("ok".into(), Value::Bool(false)), s("token", f.name().into())];
    if let Some(i) = f.index() {
        m.push(("index".into(), Value::Int(i as u64)));
    }
    Value::Obj(m)
}

fn list_said(e: &k6::listing::ListError) -> eravon_literals::misuse::Said {
    use eravon_literals::misuse::said;
    use k6::listing::ListError;
    match e {
        ListError::NotDirectory => said("不是一个列得出的文件夹", "not a folder whose entries can be listed"),
        ListError::Unreadable => said("文件夹里有一份档读不出", "a file in the folder cannot be read"),
        ListError::NameNotUtf8 => said("文件夹里有一个名字不是 UTF-8", "a name in the folder is not UTF-8"),
        ListError::TooBig => said("文件夹里有一份档超过 16 MiB 的上限", "a file in the folder is past the 16 MiB limit"),
        ListError::Unlisted(_) => said("文件夹里有一份档证据核心不收(名不是包内相对路径,或媒体类型空)", "the evidence core does not list a file in the folder (its name is no package-relative path, or the media type is empty)"),
        ListError::NameNotPortable(why) => {
            let table = [
                ("设备名", said("有一个名字有一段是 Windows 的设备名(在那里打开的是设备,不是档)", "a name has a part that is a Windows device name (there it opens the device, not a file)")),
                ("点或空格收尾", said("有一个名字有一段以点或空格收尾(Windows 会把它去掉,两个名落到同一个档)", "a name has a part ending in a dot or a space (Windows drops it, and two names land on one file)")),
                ("控制字符", said("有一个名字有一段带控制字符", "a name has a part with a control character")),
                ("冒号", said("有一个名字带冒号(盘符或流名)", "a name has a colon (a drive letter or a stream name)")),
                ("反斜杠", said("有一个名字带反斜杠(别的系统的分隔符)", "a name has a backslash (another system's separator)")),
                (". 或 ..", said("有一个名字有一段是 . 或 ..", "a name has a part that is . or ..")),
                ("空段", said("有一个名字有空段", "a name has an empty part")),
                ("是空的", said("有一个名字是空的", "a name is empty")),
            ];
            table.iter().find(|(k, _)| why.contains(k)).map(|(_, s)| *s).unwrap_or(said("有一个名字在别的系统上存不回", "a name does not save back on every system"))
        }
    }
}

fn listing(dir: &str, subject_path: &str, party: &str, media_type: &str) -> Result<Value, Misuse> {
    let sb = read_file(subject_path)?;
    let subject = k1::canon::read_open(&sb).ok_or_else(|| Misuse::unreadable(subject_path, eravon_literals::misuse::NOT_JSON))?;
    if !k1::canon::is_skeleton(party) {
        return Err(Misuse::bad("party", party, eravon_literals::misuse::CHARSET));
    }
    if !k1::canon::is_skeleton(media_type) {
        return Err(Misuse::bad("media-type", media_type, eravon_literals::misuse::CHARSET));
    }
    let m = k6::listing::from_directory(Path::new(dir), subject, party, media_type, None)
        .map_err(|e| Misuse::unreadable(dir, list_said(&e)))?;
    let (bytes, param) = match k6::sheet::build(&m).and_then(|b| Ok((b, k6::listing::claim_parameter(&m)?))) {
        Ok(x) => x,
        Err(t) => return Ok(Value::Obj(vec![("ok".into(), Value::Bool(false)), s("token", t.name().into())])),
    };
    Ok(match k6::sheet::check(&bytes) {
        Err(f) => fault(&f),
        Ok(_) => Value::Obj(vec![
            ("files".into(), Value::Int(m.files.len() as u64)),
            s("manifest", k1::hash::hex_encode(&param)),
            ("ok".into(), Value::Bool(true)),
            s("sheet", k1::hash::hex_encode(&bytes)),
        ]),
    })
}
