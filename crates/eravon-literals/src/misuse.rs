use std::cell::RefCell;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Args,
    Unreadable,
    Key,
}

impl Reason {
    pub const ALL: [Reason; 3] = [Reason::Args, Reason::Unreadable, Reason::Key];

    pub const fn as_str(self) -> &'static str {
        match self {
            Reason::Args => "E_ARGS",
            Reason::Unreadable => "E_UNREADABLE",
            Reason::Key => "E_KEY",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    Program,
    Flags(Vec<&'static str>),
    Placed { at: usize, bytes: usize },
    Unplaced { bytes: usize },
}

impl Subject {
    pub fn flag(name: &str) -> Subject {
        let typed_here = ARGUMENTS.with(|a| a.borrow().iter().any(|x| x == name || x.split('=').next() == Some(name)));
        match flag_static(name) {
            Some(n) if !typed_here => Subject::Flags(vec![n]),
            _ => typed(name),
        }
    }

    pub fn spelled(&self, bin: &str) -> String {
        match self {
            Subject::Program => bin.to_string(),
            Subject::Flags(names) => names.join(" "),
            Subject::Placed { at, bytes } => format!("#{at} ({bytes} bytes)"),
            Subject::Unplaced { bytes } => format!("({bytes} bytes)"),
        }
    }
}

fn flag_static(name: &str) -> Option<&'static str> {
    let shaped = name.strip_prefix("--").is_some_and(|r| !r.is_empty() && r.len() <= 40 && r.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'));
    if !shaped {
        return None;
    }
    let mut kept = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(n) = kept.iter().find(|n| **n == name) {
        return Some(n);
    }
    let n: &'static str = Box::leak(name.to_string().into_boxed_str());
    kept.push(n);
    Some(n)
}

static NAMES: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

thread_local! {
    static ARGUMENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn keep_arguments(args: &[String]) {
    ARGUMENTS.with(|a| *a.borrow_mut() = args.to_vec());
}

pub fn typed(word: &str) -> Subject {
    let args = ARGUMENTS.with(|a| a.borrow().clone());
    if let Some(i) = args.iter().position(|a| a == word || a.split('=').any(|piece| piece == word)) {
        return Subject::Placed { at: i + 1, bytes: args[i].len() };
    }
    let sep = |b: u8| b == b'/' || b == b'\\';
    let under = |a: &String| !a.is_empty() && word.len() > a.len() && word.starts_with(a.as_str()) && (a.bytes().last().is_some_and(sep) || sep(word.as_bytes()[a.len()]));
    match args.iter().enumerate().filter(|(_, a)| under(a)).max_by_key(|(_, a)| a.len()) {
        Some((i, a)) => Subject::Placed { at: i + 1, bytes: a.len() },
        None => Subject::Unplaced { bytes: word.len() },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

pub fn lang() -> Lang {
    let first = ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"].iter().find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()));
    if first.is_some_and(|v| v.starts_with("zh")) { Lang::Zh } else { Lang::En }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Said {
    pub zh: &'static str,
    pub en: &'static str,
}

pub const fn said(zh: &'static str, en: &'static str) -> Said {
    Said { zh, en }
}

impl Said {
    pub const fn text(self, l: Lang) -> &'static str {
        match l {
            Lang::Zh => self.zh,
            Lang::En => self.en,
        }
    }
}

pub const MISSING: Said = said("缺这一旗", "this flag is missing");
pub const NEEDS_VALUE: Said = said("这一旗要一个值", "this flag needs a value");
pub const REPEATED: Said = said("这一旗只收一个值,给了不止一次", "this flag takes one value and was given more than once");
pub const NOT_INT: Said = said("不是一个十进制整数", "not a decimal integer");
pub const NO_VERB: Said = said("缺动词", "a verb is missing");
pub const NOT_A_VERB: Said = said("不是这个程序的动词", "not a verb of this program");
pub const NOT_THIS_VERBS_FLAG: Said = said("不在这个动词的旗单里", "not among this verb's flags");
pub const NOT_A_FLAG: Said = said("参数要成对写作 --名字 值", "arguments go in pairs as --name value");
pub const NOT_UTF8: Said = said("有一个参数不是合法的 UTF-8 文本", "an argument is not valid UTF-8 text");
pub const EMPTY_PATH: Said = said("路径不许是空的", "a path may not be empty");
pub const ONE_FORM: Said = said("同一个值的两种写法只给一种", "give one of the two forms of this value, not both");
pub const UNREADABLE: Said = said("读不出", "cannot be read");
pub const TOO_BIG: Said = said("文件超过了上限", "the file is past its size limit");
pub const NOT_JSON: Said = said("不是 JSON", "not JSON");
pub const SECRET_FILE_OPEN: Said = said("这份档别的账户也读得到(只许本人可读)", "other accounts can read this file (it must be readable by its owner only)");
pub const SECRET_FILE_BIG: Said = said("这份档超过 4096 字节", "this file is past 4096 bytes");
pub const SECRET_FILE_TEXT: Said = said("这份档不是 UTF-8 文本", "this file is not UTF-8 text");
pub const SECRET_BOTH: Said = said("明文旗与它的 -file 档形只给一种", "give the flag or its -file form, not both");
pub const KEY_SHAPE: Said = said("不是六十四位十六进制,或不在曲线的范围里", "not 64 hex digits, or not within the curve's range");
pub const KEY_CANNOT: Said = said("这把私钥用不成(签不出或算不出地址)", "this private key cannot be used (it does not sign or yield an address)");
pub const INT_SPELLING: Said = said("要是 galeed/1 §3.2 的整数写法", "an integer as galeed/1 §3.2 writes it");
pub const INT_OR_HEX: Said = said("要是十进制整数或 0x 十六进制", "a decimal integer or 0x hex");
pub const HEX20: Said = said("要是 0x 加四十位十六进制", "0x and 40 hex digits");
pub const HEX32: Said = said("要是 0x 加六十四位十六进制", "0x and 64 hex digits");
pub const HEX_EVEN: Said = said("要是 0x 加偶数位十六进制", "0x and an even number of hex digits");
pub const HEX_N: Said = said("十六进制的位数不对", "the wrong number of hex digits");
pub const CHARSET: Said = said("要是骨架字符集里的字(galeed/1 §3.3)", "characters of the skeleton set (galeed/1 §3.3)");
pub const SEAT: Said = said("要是 seller 或 buyer", "seller or buyer");
pub const ONE_OF: Said = said("不是这一旗认得的几个值之一", "not one of the values this flag takes");
pub const POSITIONAL: Said = said("位置参数多了或少了", "too many or too few positional arguments");
pub const SCHEMA_BARE: Said = said("schema 不带参数", "schema takes no arguments");
pub const PROXY: Said = said(
    "出路只收 system、none,或 http、https、socks5、socks4 的 协议://[用户[:口令]@]主机:端口",
    "system, none, or a proxy as scheme://[user[:password]@]host:port with the scheme http, https, socks5 or socks4",
);
pub const NEED_ENDPOINTS: Said = said("缺端点表:给 --endpoint(可多枚)或 --table", "no endpoint table: give --endpoint (one or more) or --table");
pub const NOT_FOLDER: Said = said("不是一个读得出的文件夹,或里面有一份档读不出或过了上限", "not a readable folder, or a file in it does not read or is past its limit");
pub const SELF_CHECK: Said = said("本程序的自检没过", "this program's self-check failed");
pub const NO_WINDOW: Said = said("开不成窗(没有显示或 GPU)", "the window cannot open (no display or GPU)");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Misuse {
    reason: Reason,
    subject: Subject,
    said: Said,
}

impl Misuse {
    pub fn new(reason: Reason, subject: Subject, said: Said) -> Misuse {
        Misuse { reason, subject, said }
    }

    pub fn args(said: Said) -> Misuse {
        Misuse::new(Reason::Args, Subject::Program, said)
    }

    pub fn missing(flag: &str) -> Misuse {
        Misuse::new(Reason::Args, Subject::flag(flag), MISSING)
    }

    pub fn bad(flag: &str, given: &str, want: Said) -> Misuse {
        let subject = if given.is_empty() { Subject::flag(flag) } else { typed(given) };
        Misuse::new(Reason::Args, subject, want)
    }

    pub fn no_value(flag: &str) -> Misuse {
        Misuse::new(Reason::Args, Subject::flag(flag), NEEDS_VALUE)
    }

    pub fn repeated(flag: &str) -> Misuse {
        Misuse::new(Reason::Args, Subject::flag(flag), REPEATED)
    }

    pub fn both(flags: &[&str], said: Said) -> Misuse {
        let subject = flags.first().map_or(Subject::Program, |f| Subject::flag(f));
        Misuse::new(Reason::Args, subject, said)
    }

    pub fn not_int(flag: &str, given: &str) -> Misuse {
        Misuse::bad(flag, given, NOT_INT)
    }

    pub fn verb(given: Option<&str>) -> Misuse {
        match given {
            None => Misuse::new(Reason::Args, Subject::Program, NO_VERB),
            Some(v) => Misuse::new(Reason::Args, typed(v), NOT_A_VERB),
        }
    }

    pub fn unknown_arg(_verb: &str, arg: &str) -> Misuse {
        Misuse::new(Reason::Args, typed(arg), NOT_THIS_VERBS_FLAG)
    }

    pub fn shape(at: &str) -> Misuse {
        Misuse::new(Reason::Args, typed(at), NOT_A_FLAG)
    }

    pub fn not_text() -> Misuse {
        Misuse::new(Reason::Args, Subject::Program, NOT_UTF8)
    }

    pub fn unreadable(word: &str, said: Said) -> Misuse {
        Misuse::new(Reason::Unreadable, typed(word), said)
    }

    pub fn key(word: &str) -> Misuse {
        Misuse::new(Reason::Key, typed(word), KEY_SHAPE)
    }

    pub fn key_cannot() -> Misuse {
        Misuse::new(Reason::Key, Subject::Program, KEY_CANNOT)
    }

    pub fn reason(&self) -> Reason {
        self.reason
    }

    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    pub fn said(&self) -> Said {
        self.said
    }

    pub fn words(&self) -> Said {
        self.said
    }

    pub fn line(&self, bin: &str) -> String {
        format!("{} {}", self.reason.as_str(), self.subject.spelled(bin))
    }

    pub fn say(&self, bin: &str) {
        use std::io::Write;
        let _ = writeln!(std::io::stderr().lock(), "{}\n{}", self.line(bin), self.said.text(lang()));
    }
}

impl fmt::Display for Misuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} · {}", self.reason.as_str(), self.subject.spelled("-"), self.said.zh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg(name: &str) -> String {
        format!("{}{name}", "-".repeat(2))
    }

    #[test]
    fn the_first_line_names_a_typed_word_by_place_and_length_only() {
        let p = arg("passcode");
        keep_arguments(&["vault-init".into(), p.clone(), "hunter2".into(), "abandon".into()]);
        let m = Misuse::bad(&p, "hunter2", NOT_INT);
        assert_eq!(m.line("eravon"), "E_ARGS #3 (7 bytes)");
        assert!(!format!("{}{}", m.line("eravon"), m.words().zh).contains("hunter2"));
        assert_eq!(Misuse::shape("abandon").line("eravon"), "E_ARGS #4 (7 bytes)", "a stray word may be part of a phrase");
        assert_eq!(Misuse::no_value(&p).line("eravon"), "E_ARGS #2 (10 bytes)");
        let stray = arg("sk-live-abc123");
        keep_arguments(&["balance".into(), stray.clone()]);
        assert_eq!(Misuse::no_value(&stray).line("erc"), "E_ARGS #2 (16 bytes)");
        assert_eq!(Misuse::both(&[stray.as_str(), "x"], ONE_FORM).line("erc"), "E_ARGS #2 (16 bytes)");
        assert_eq!(Misuse::missing(&p).line("eravon"), format!("E_ARGS {p}"), "missing: the code's own name");
        assert_eq!(Misuse::verb(None).line("eravon"), "E_ARGS eravon");
        assert_eq!(Misuse::not_text().reason(), Reason::Args);
        assert_eq!(Misuse::unreadable("/no/such/file", UNREADABLE).line("x"), "E_UNREADABLE (13 bytes)");
        assert_eq!(Subject::flag("--Pass Code"), typed("--Pass Code"));
    }

    #[test]
    fn the_reason_table_is_closed() {
        let names: Vec<&str> = Reason::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(names, ["E_ARGS", "E_UNREADABLE", "E_KEY"]);
    }

    #[test]
    fn a_path_under_a_folder_given_is_named_by_that_folder() {
        keep_arguments(&["verify".into(), arg("dossier"), "/tmp/d".into()]);
        assert_eq!(typed("/tmp/d/a.json"), Subject::Placed { at: 3, bytes: 6 });
    }
}
