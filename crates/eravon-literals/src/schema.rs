use crate::misuse::{self, Misuse};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    S(String),
    N(u64),
    B(bool),
    A(Vec<Cell>),
    O(Vec<(&'static str, Cell)>),
}

pub const T_STR: &str = "string";
pub const T_INT: &str = "int";
pub const T_BOOL: &str = "bool";
pub const T_ARR: &str = "array";
pub const T_OBJ: &str = "object";
pub const T_HEX: &str = T_STR;

pub const T_PATH: &str = "path";

pub const CONTRACT: u64 = 11;
pub const EXIT_MISUSE: u64 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Affirmed,
    Denied,
    Misuse,
    Partial,
    Unanswered,
}

impl Exit {
    pub const ALL: [Exit; 5] = [Exit::Affirmed, Exit::Denied, Exit::Misuse, Exit::Partial, Exit::Unanswered];

    pub const fn code(self) -> u8 {
        match self {
            Exit::Affirmed => 0,
            Exit::Denied => 1,
            Exit::Misuse => 2,
            Exit::Partial => 3,
            Exit::Unanswered => 4,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Exit::Affirmed => "affirmed",
            Exit::Denied => "denied",
            Exit::Misuse => "misuse",
            Exit::Partial => "partial",
            Exit::Unanswered => "unanswered",
        }
    }
}

pub const PARTIAL: &[&str] = &[
    "E_V_UNCONFIRMED",
    "E_QUEUED",
];

pub const UNANSWERED: &[&str] = &[
    "E_CHAIN_TRANSPORT",
    "E_CHAIN_ENDPOINT",
    "E_CHAIN_DISAGREE",
    "E_CHAIN_WRONG",
    "E_CHAIN_ECHO",
    "E_CHAIN_OVER_ASKS",
    "E_CHAIN_UNREAD",
    "E_STORE_IO",
    "E_STORE_LOCKED",
    "E_STORE_MACHINE",
    "E_VAULT_IO",
    "E_V_IO",
    "E_APP_IO",
    "E_ZK_IO",
    "E_QUOTE_FEED",
    "E_UNREACHABLE",
    "E_TX_NOT_YET",
    "E_DESKTOP",
    "E_OUTCOME_LOST",
    "E_NO_ENDPOINT",
    "E_DISAGREE",
    "E_SCAN_REFUSED",
    "E_CHAIN_SHAPE",
    "E_NODE_TLS",
    "E_PROXY_UNREACHABLE",
    "E_PROXY_CREDENTIALS_MISSING",
    "E_PROXY_CREDENTIALS_REFUSED",
    "E_NODE_TIMEOUT",
    "E_ANSWER_TOO_LONG",
    "E_ANSWER_NOT_JSON",
    "E_RATE_LIMITED",
    "E_METHOD_MISSING",
    "E_NODE_AUTH",
    "E_SERVICE_UNSERVED",
    "E_SERVICE_KEY_MISSING",
    "E_WRONG_CHAIN",
    "E_NODE_REFUSED",
];

pub fn class(token: &str) -> Exit {
    if PARTIAL.contains(&token) {
        Exit::Partial
    } else if UNANSWERED.contains(&token) {
        Exit::Unanswered
    } else {
        Exit::Denied
    }
}

pub fn exit_of(v: &crate::json::Out) -> u8 {
    use crate::json::Out;
    let Out::O(m) = v else { return Exit::Affirmed.code() };
    let get = |k: &str| m.iter().find(|(n, _)| *n == k).map(|(_, x)| x);
    let name = |k: &str| match get(k) {
        Some(Out::S(s)) => Some(s.as_str()),
        _ => None,
    };
    exit_by(matches!(get("ok"), Some(Out::B(false))), name("downToken").or(name("token")), matches!(get("txs"), Some(Out::A(a)) if !a.is_empty()))
}

pub fn exit_of_read(v: &crate::json::In) -> u8 {
    use crate::json::In;
    let name = |k: &str| v.get(k).and_then(|x| x.as_str());
    exit_by(matches!(v.get("ok"), Some(In::Bool(false))), name("downToken").or(name("token")), matches!(v.get("txs"), Some(In::Arr(a)) if !a.is_empty()))
}

fn exit_by(refused: bool, name: Option<&str>, sent: bool) -> u8 {
    if !refused {
        return Exit::Affirmed.code();
    }
    let c = class(name.unwrap_or_default());
    if sent && c == Exit::Denied { Exit::Partial.code() } else { c.code() }
}


pub fn judged(mut rows: Vec<(&'static str, Cell)>) -> Vec<(&'static str, Cell)> {
    rows.retain(|(k, _)| !matches!(*k, "exits" | "exitPartial" | "exitUnanswered"));
    let two = [Exit::Affirmed, Exit::Misuse];
    rows.push(("exits", Cell::O(two.iter().map(|e| (e.name(), Cell::N(u64::from(e.code())))).collect())));
    rows
}

pub fn exit_cells() -> Vec<(&'static str, Cell)> {
    vec![
        ("exitPartial", Cell::A(PARTIAL.iter().map(|t| Cell::S((*t).to_string())).collect())),
        ("exitUnanswered", Cell::A(UNANSWERED.iter().map(|t| Cell::S((*t).to_string())).collect())),
        ("exits", Cell::O(Exit::ALL.iter().map(|e| (e.name(), Cell::N(u64::from(e.code())))).collect())),
    ]
}

pub const SECRET_FLAGS: [&str; 6] = ["passcode", "new-passcode", "file-passcode", "again", "phrase", "key"];

pub const SECRET_FILE_SUFFIX: &str = "-file";

pub const SECRET_FILE_CAP: u64 = 4096;

pub fn read_pairs(args: &[String]) -> Result<Vec<(String, String)>, Misuse> {
    read_flags(args, &[]).map(|(p, _)| p)
}

pub fn read_flags(args: &[String], bare: &[&str]) -> Result<(Vec<(String, String)>, Vec<String>), Misuse> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut switches: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(k) = it.next() {
        if !k.starts_with("--") {
            return Err(Misuse::shape(k));
        }
        if bare.contains(&k.as_str()) {
            if switches.contains(k) {
                return Err(Misuse::repeated(k));
            }
            switches.push(k.clone());
            continue;
        }
        let v = it.next().ok_or_else(|| Misuse::no_value(k))?;
        if v.starts_with("--") {
            return Err(Misuse::no_value(k));
        }
        pairs.push((k.clone(), v.clone()));
    }
    let mut out: Vec<(String, String)> = Vec::with_capacity(pairs.len());
    for (k, v) in pairs.iter() {
        let Some(name) = k.strip_prefix("-").and_then(|r| r.strip_prefix("-")).and_then(|r| r.strip_suffix(SECRET_FILE_SUFFIX)).filter(|n| SECRET_FLAGS.contains(n)) else {
            out.push((k.clone(), v.clone()));
            continue;
        };
        let flag = format!("{}{name}", "-".repeat(2));
        if pairs.iter().any(|(b, _)| *b == flag) {
            return Err(Misuse::both(&[flag.as_str(), k.as_str()], misuse::SECRET_BOTH));
        }
        out.push((flag, secret_file(v)?));
    }
    Ok((out, switches))
}

fn secret_file(path: &str) -> Result<String, Misuse> {
    let meta = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, misuse::UNREADABLE))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(Misuse::unreadable(path, misuse::SECRET_FILE_OPEN));
        }
    }
    if !meta.is_file() {
        return Err(Misuse::unreadable(path, misuse::UNREADABLE));
    }
    if meta.len() > SECRET_FILE_CAP {
        return Err(Misuse::unreadable(path, misuse::SECRET_FILE_BIG));
    }
    let bytes = std::fs::read(path).map_err(|_| Misuse::unreadable(path, misuse::UNREADABLE))?;
    if bytes.len() as u64 > SECRET_FILE_CAP {
        return Err(Misuse::unreadable(path, misuse::SECRET_FILE_BIG));
    }
    let text = String::from_utf8(bytes).map_err(|_| Misuse::unreadable(path, misuse::SECRET_FILE_TEXT))?;
    let text = text.strip_suffix('\n').map(|t| t.strip_suffix('\r').unwrap_or(t)).unwrap_or(&text);
    Ok(text.to_string())
}

pub fn one_value(pairs: &[(String, String)], is_array: impl Fn(&str) -> bool) -> Result<(), Misuse> {
    for (i, (k, _)) in pairs.iter().enumerate() {
        if !is_array(k) && pairs[..i].iter().any(|(b, _)| b == k) {
            return Err(Misuse::repeated(k));
        }
    }
    Ok(())
}

pub const REFUSAL: [(&str, &str); 3] = [("ok", T_BOOL), ("token", T_STR), ("detail", T_STR)];

#[derive(Clone, Copy, Debug)]
pub struct Arg {
    pub name: &'static str,
    pub ty: &'static str,
    pub required: bool,
}

impl Arg {
    pub const fn need(name: &'static str, ty: &'static str) -> Arg {
        Arg { name, ty, required: true }
    }
    pub const fn opt(name: &'static str, ty: &'static str) -> Arg {
        Arg { name, ty, required: false }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Verb {
    pub verb: &'static str,
    pub args: &'static [Arg],
    pub tokens: &'static [&'static str],
    pub carries: &'static [&'static str],
    pub opens_window: bool,
}

impl Verb {
    pub const fn new(
        verb: &'static str,
        args: &'static [Arg],
        tokens: &'static [&'static str],
        carries: &'static [&'static str],
    ) -> Verb {
        Verb { verb, args, tokens, carries, opens_window: false }
    }

    pub const fn carry(verb: &'static str, args: &'static [Arg], carries: &'static [&'static str]) -> Verb {
        Verb { verb, args, tokens: &[], carries, opens_window: false }
    }

    pub const fn plain(verb: &'static str, args: &'static [Arg]) -> Verb {
        Verb { verb, args, tokens: &[], carries: &[], opens_window: false }
    }

    pub const fn window(
        verb: &'static str,
        args: &'static [Arg],
        tokens: &'static [&'static str],
        carries: &'static [&'static str],
    ) -> Verb {
        Verb { verb, args, tokens, carries, opens_window: true }
    }
}

fn pairs(list: &[(&'static str, &'static str)]) -> Cell {
    Cell::A(
        list.iter()
            .map(|(n, t)| {
                Cell::O(vec![("name", Cell::S((*n).to_string())), ("type", Cell::S((*t).to_string()))])
            })
            .collect(),
    )
}

fn args_of(v: &Verb) -> Cell {
    Cell::A(
        v.args
            .iter()
            .map(|a| {
                Cell::O(vec![
                    ("name", Cell::S(a.name.to_string())),
                    ("required", Cell::B(a.required)),
                    ("type", Cell::S(a.ty.to_string())),
                ])
            })
            .collect(),
    )
}

fn tokens_of(v: &Verb) -> Cell {
    Cell::A(v.tokens.iter().map(|t| Cell::S((*t).to_string())).collect())
}

pub fn register(v: &str, verbs: &[Verb], common: &[&'static str]) -> Vec<(&'static str, Cell)> {
    let mut rows = vec![
        ("commonTokens", Cell::A(common.iter().map(|t| Cell::S((*t).to_string())).collect())),
        ("contract", Cell::N(CONTRACT)),
        ("secretFlags", secret_cells()),
        (
            "register",
            Cell::A(
                verbs
                    .iter()
                    .map(|x| {
                        Cell::O(vec![
                            ("args", args_of(x)),
                            (
                                "carries",
                                Cell::A(x.carries.iter().map(|c| Cell::S((*c).to_string())).collect()),
                            ),
                            ("opensWindow", Cell::B(x.opens_window)),
                            ("tokens", tokens_of(x)),
                            ("verb", Cell::S(x.verb.to_string())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("refusal", pairs(&REFUSAL)),
        ("trailingNewline", Cell::B(false)),
        ("v", Cell::S(v.to_string())),
        ("verbs", Cell::A(verbs.iter().map(|x| Cell::S(x.verb.to_string())).collect())),
    ];
    rows.extend(exit_cells());
    rows
}

pub fn secret_cells() -> Cell {
    let dash = "-".repeat(2);
    Cell::A(SECRET_FLAGS.iter().map(|n| Cell::O(vec![("file", Cell::S(format!("{dash}{n}{SECRET_FILE_SUFFIX}"))), ("flag", Cell::S(format!("{dash}{n}")))])).collect())
}

pub fn form_gate(verbs: &[Verb], verb: &str, pairs: &[(String, String)]) -> Result<(), Misuse> {
    one_form(pairs)?;
    let Some(row) = verbs.iter().find(|v| v.verb == verb) else { return Ok(()) };
    if let Some((k, _)) = pairs.iter().find(|(k, _)| !row.args.iter().any(|a| a.name == k)) {
        return Err(Misuse::unknown_arg(verb, k));
    }
    one_value(pairs, |k| row.args.iter().any(|a| a.name == k && a.ty == T_ARR))?;
    for (k, v) in pairs {
        if !v.is_empty() {
            continue;
        }
        if row.args.iter().any(|a| a.name == k && a.ty == T_PATH) {
            return Err(Misuse::bad(k, v, misuse::EMPTY_PATH));
        }
    }
    Ok(())
}

pub fn positional_misuse(verbs: &[Verb], args: &[String]) -> Misuse {
    let Some(v) = args.first() else { return Misuse::verb(None) };
    match verbs.iter().find(|x| x.verb == v) {
        None => Misuse::verb(Some(v)),
        Some(row) => {
            let _ = row;
            Misuse::args(misuse::said("这个动词的位置参数数目不对(写法见 schema)", "this verb was given the wrong number of arguments (see schema)"))
        }
    }
}

pub const TEXT_SUFFIX: &str = "-text";

pub fn one_form(pairs: &[(String, String)]) -> Result<(), Misuse> {
    let both = pairs.iter().find_map(|(k, _)| k.strip_suffix(TEXT_SUFFIX).filter(|base| pairs.iter().any(|(b, _)| b == base)));
    match both {
        Some(base) => Err(Misuse::both(&[base, &format!("{base}{TEXT_SUFFIX}")], misuse::ONE_FORM)),
        None => Ok(()),
    }
}

pub fn version(bin: &str) -> String {
    format!("{bin}.schema/1")
}

impl From<Cell> for crate::json::Out {
    fn from(c: Cell) -> crate::json::Out {
        use crate::json::Out;
        match c {
            Cell::S(s) => Out::S(s),
            Cell::N(n) => Out::N(n),
            Cell::B(b) => Out::B(b),
            Cell::A(v) => Out::A(v.into_iter().map(Out::from).collect()),
            Cell::O(m) => Out::O(m.into_iter().map(|(k, v)| (k, Out::from(v))).collect()),
        }
    }
}

pub fn out_rows(rows: Vec<(&'static str, Cell)>) -> Vec<(&'static str, crate::json::Out)> {
    rows.into_iter().map(|(k, v)| (k, crate::json::Out::from(v))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_judged_register_says_two_codes_and_keeps_the_rest() {
        let rows = judged(register(&version("x"), &[], &[]));
        let exits: Vec<_> = rows.iter().filter(|(k, _)| k.starts_with("exit")).collect();
        assert_eq!(exits.len(), 1, "{exits:?}");
        match &exits[0].1 {
            Cell::O(m) => assert_eq!(m.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>(), vec![("affirmed", Cell::N(0)), ("misuse", Cell::N(2))]),
            other => panic!("{other:?}"),
        }
        assert!(rows.iter().any(|(k, _)| *k == "contract") && rows.iter().any(|(k, _)| *k == "secretFlags"));
    }
    use crate::json::Out;
    use crate::misuse::Reason;

    fn arg(name: &str) -> String {
        format!("{}{name}", "-".repeat(2))
    }

    fn file(dir: &std::path::Path, name: &str, body: &[u8], mode: u32) -> String {
        let p = dir.join(name);
        std::fs::write(&p, body).expect("写得下");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).expect("改得了权限");
        }
        #[cfg(not(unix))]
        let _ = mode;
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn every_secret_flag_reads_from_its_file_and_the_four_forms_are_judged() {
        let dir = std::env::temp_dir().join(format!("eravon-secret-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("建得了");
        for name in SECRET_FLAGS {
            let flag = arg(name);
            let p = file(&dir, "s", b"hunter2\n", 0o600);
            let got = read_pairs(&[format!("{flag}{SECRET_FILE_SUFFIX}"), p.clone()]).expect("读得出");
            assert_eq!(got, vec![(flag.clone(), "hunter2".to_string())], "{flag}");
        }
        let flag = arg(SECRET_FLAGS[0]);
        let flag = flag.as_str();
        let fileform = format!("{flag}{SECRET_FILE_SUFFIX}");
        let bare = file(&dir, "bare", b"hunter2", 0o600);
        assert_eq!(read_pairs(&[fileform.clone(), bare.clone()]).expect("没有换行也读得出")[0].1, "hunter2");
        let crlf = file(&dir, "crlf", b"hunter2\r\n", 0o600);
        assert_eq!(read_pairs(&[fileform.clone(), crlf]).expect("回车换行")[0].1, "hunter2");
        let at = file(&dir, "at", &vec![b'a'; 4096], 0o600);
        assert_eq!(read_pairs(&[fileform.clone(), at]).expect("恰 4096")[0].1.len(), 4096);
        let past = file(&dir, "past", &vec![b'a'; 4097], 0o600);
        assert_eq!(read_pairs(&[fileform.clone(), past]).expect_err("4097 拒").reason(), Reason::Unreadable);
        #[cfg(unix)]
        {
            let open = file(&dir, "open", b"hunter2", 0o644);
            assert_eq!(read_pairs(&[fileform.clone(), open]).expect_err("别人读得到即拒").words(), crate::misuse::SECRET_FILE_OPEN);
        }
        let both = read_pairs(&[flag.to_string(), "x".into(), fileform.clone(), bare]).expect_err("两形同给");
        assert_eq!(both.words(), crate::misuse::SECRET_BOTH);
        assert_eq!(read_pairs(&[fileform, dir.join("nope").to_string_lossy().into_owned()]).expect_err("读不到").reason(), Reason::Unreadable);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_one_value_flag_given_twice_is_misuse() {
        let one = arg("one");
        let many = arg("many");
        let args = [Arg::opt(Box::leak(one.clone().into_boxed_str()), T_STR), Arg::opt(Box::leak(many.clone().into_boxed_str()), T_ARR)];
        let row = [Verb::plain("v", Box::leak(Box::new(args)))];
        let row: &[Verb] = &row;
        let p = |xs: &[(&String, &str)]| xs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<Vec<_>>();
        assert!(form_gate(row, "v", &p(&[(&many, "a"), (&many, "b"), (&one, "c")])).is_ok());
        let m = form_gate(row, "v", &p(&[(&one, "a"), (&one, "b")])).expect_err("单值两次");
        assert_eq!(m.words(), crate::misuse::REPEATED);
    }

    #[test]
    fn each_exit_code_has_its_answers() {
        let refusal = |token: &str, down: Option<&str>| {
            let mut m = vec![("ok", Out::B(false)), ("token", Out::S(token.into())), ("detail", Out::S(String::new()))];
            if let Some(d) = down {
                m.push(("downToken", Out::S(d.into())));
            }
            Out::O(m)
        };
        assert_eq!(exit_of(&Out::O(vec![("ok", Out::B(true))])), 0);
        assert_eq!(exit_of(&refusal("E_V_STATE", None)), 1);
        assert_eq!(exit_of(&refusal("E_V_UNCONFIRMED", None)), 3);
        assert_eq!(exit_of(&refusal("E_CHAIN_TRANSPORT", None)), 4);
        assert_eq!(exit_of(&refusal("E_V_DOWN", Some("E_STORE_IO"))), 4);
        assert_eq!(exit_of(&refusal("E_V_DOWN", Some("E_VAULT_PASSCODE"))), 1);
        assert!(PARTIAL.iter().all(|t| !UNANSWERED.contains(t)), "两张表不相交");
        let codes: Vec<u8> = Exit::ALL.iter().map(|e| e.code()).collect();
        assert_eq!(codes, [0, 1, 2, 3, 4]);
    }
}
