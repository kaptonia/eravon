use crate::codes::{Exit, Key, Reason};
use std::io::Write;
use eravon::json::{self, Value};

pub struct Answer {
    pub exit: Exit,
    pub value: Value,
    pub told: Option<String>,
}

impl Answer {
    pub fn telling(self, line: String) -> Answer {
        Answer { told: Some(line), ..self }
    }

    pub fn with(self, members: Vec<(Key, Value)>) -> Answer {
        let value = match self.value {
            Value::Obj(mut ms) => {
                ms.extend(members.into_iter().map(|(k, v)| (k.as_str().to_string(), v)));
                Value::Obj(ms)
            }
            other => other,
        };
        Answer { value, ..self }
    }
}

pub(crate) fn obj(members: Vec<(Key, Value)>) -> Value {
    Value::Obj(
        members
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v))
            .collect(),
    )
}

pub fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

pub fn affirmed(members: Vec<(Key, Value)>) -> Answer {
    let mut ms = vec![(Key::Ok, Value::Bool(true))];
    ms.extend(members);
    Answer {
        exit: Exit::Affirmed,
        value: obj(ms),
        told: None,
    }
}

pub fn denied(reason: Reason, members: Vec<(Key, Value)>) -> Answer {
    at(Exit::Denied, reason, members)
}

pub fn partial(reason: Reason, members: Vec<(Key, Value)>) -> Answer {
    at(Exit::Partial, reason, members)
}

pub fn unanswered(reason: Reason, members: Vec<(Key, Value)>) -> Answer {
    let asked = members.iter().find_map(|(k, v)| match (k, v) {
        (Key::Detail, Value::Str(d)) => eravon_chain::rpc::proxy_credentials_of(d),
        _ => None,
    });
    match asked {
        Some(given) => {
            let (token, said) = if given { ("PROXY_CREDENTIALS_REFUSED", Said::ProxyCredentialsRefused) } else { ("PROXY_CREDENTIALS_MISSING", Said::ProxyCredentialsMissing) };
            let mut ms = members;
            if reason == Reason::Unreachable && !ms.iter().any(|(k, _)| *k == Key::Token) {
                ms.push((Key::Token, s(token)));
            }
            at(Exit::Unanswered, reason, ms).telling(said.text().to_string())
        }
        None => match eravon_chain::rpc::script_trouble_last().filter(|_| reason == Reason::Unreachable) {
            Some(x) => at(Exit::Unanswered, reason, members).telling(script_line(x)),
            None => at(Exit::Unanswered, reason, members),
        },
    }
}

pub fn over_asks_line() -> String {
    let (what, next) = (eravon_glue::book::OVER_ASKS_WORDS, eravon_glue::book::OVER_ASKS_NEXT_WORDS);
    format!("{} {}", say(what.0, what.1), say(next.0, next.1))
}

fn over_asks_in(value: &Value) -> bool {
    let over = eravon_chain::fault::Fault::OverAsks(String::new()).name();
    let named = [Key::Reason, Key::Token].iter().any(|k| value.member(k.as_str()).and_then(Value::as_str) == Some(over));
    named || value.member(Key::Detail.as_str()).and_then(Value::as_str).and_then(crate::fault::down_of).is_some_and(|(_, token, _)| token == over)
}

pub fn unanswered_as_is(value: Value) -> Answer {
    let asked = value.member(Key::Detail.as_str()).and_then(Value::as_str).and_then(eravon_chain::rpc::proxy_credentials_of);
    let over = over_asks_in(&value);
    let a = verbatim(Exit::Unanswered, value);
    match asked {
        Some(true) => a.telling(Said::ProxyCredentialsRefused.text().to_string()),
        Some(false) => a.telling(Said::ProxyCredentialsMissing.text().to_string()),
        None if over => a.telling(over_asks_line()),
        None => match eravon_chain::rpc::script_trouble_last() {
            Some(x) => a.telling(script_line(x)),
            None => a,
        },
    }
}

fn script_line(x: eravon_chain::rpc::PacTrouble) -> String {
    match x {
        eravon_chain::rpc::PacTrouble::RemoteHttp => Said::PacRemoteHttp.text().to_string(),
        eravon_chain::rpc::PacTrouble::Unfetched => Said::PacUnfetched.text().to_string(),
        other => Said::PacFailed.text().replace("{0}", other.code()),
    }
}

fn at(exit: Exit, reason: Reason, members: Vec<(Key, Value)>) -> Answer {
    let mut ms = vec![
        (Key::Ok, Value::Bool(false)),
        (Key::Reason, s(reason.as_str())),
    ];
    ms.extend(members);
    Answer {
        exit,
        value: obj(ms),
        told: None,
    }
}

pub fn verbatim(exit: Exit, value: Value) -> Answer {
    Answer { exit, value, told: None }
}

pub fn emit(a: Answer) -> ! {
    if let Some(line) = &a.told {
        eprintln!("{line}");
    }
    let code = a.exit.code();
    let bytes = crate::verbwire::scrub(json::canon_bytes(&a.value));
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(&bytes);
    let _ = out.flush();
    drop(out);
    std::process::exit(code as i32)
}

pub fn frame(v: &Value) {
    let mut bytes = json::canon_bytes(v);
    const ERROR_CODE: &[u8] = b"{\"error\":{\"code\":";
    if v.member("error").and_then(|e| e.member("code")).is_some() && bytes.starts_with(ERROR_CODE) {
        bytes.insert(ERROR_CODE.len(), b'-');
    }
    bytes.push(b'\n');
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(&bytes);
    let _ = out.flush();
}

pub fn finish() -> ! {
    let _ = std::io::stdout().lock().flush();
    std::process::exit(Exit::Affirmed.code() as i32)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Zh,
    En,
}

pub fn lang_of(var: impl Fn(&str) -> Option<String>) -> Lang {
    let set = ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"].iter().find_map(|k| var(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()));
    match set {
        Some(v) if v.split(':').next().unwrap_or("").to_ascii_lowercase().starts_with("zh") => Lang::Zh,
        _ => Lang::En,
    }
}

pub fn lang() -> Lang {
    static L: std::sync::OnceLock<Lang> = std::sync::OnceLock::new();
    *L.get_or_init(|| lang_of(|k| std::env::var(k).ok()))
}

pub fn pick(zh: String, en: String) -> String {
    match lang() {
        Lang::Zh => zh,
        Lang::En => en,
    }
}

pub fn say(zh: &'static str, en: &'static str) -> &'static str {
    match lang() {
        Lang::Zh => zh,
        Lang::En => en,
    }
}

pub fn address_said(url: &str) -> Said {
    match eravon_chain::rpc::read_address(url) {
        Err(eravon_chain::rpc::NotAnAddress::Port) => Said::EndpointPort,
        Err(eravon_chain::rpc::NotAnAddress::Scheme) | Ok(_) => Said::EndpointScheme,
        Err(_) => Said::EndpointAddress,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    Program,
    Flags(Vec<&'static str>),
    Placed { at: usize, bytes: usize },
    Endpoint { at: usize, chain: u64, bytes: usize },
    Unplaced { bytes: usize },
}

impl Subject {
    pub fn flag(name: &'static str) -> Subject {
        Subject::Flags(vec![name])
    }

    pub fn spelled(&self) -> String {
        match self {
            Subject::Program => "eravon".to_string(),
            Subject::Flags(names) => names.iter().map(|n| format!("--{n}")).collect::<Vec<_>>().join(" "),
            Subject::Placed { at, bytes } => format!("#{at} ({bytes} bytes)"),
            Subject::Endpoint { at, chain, bytes } => format!("#{at} ({chain}= + {bytes} bytes)"),
            Subject::Unplaced { bytes } => format!("({bytes} bytes)"),
        }
    }
}

static ARGUMENTS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub fn keep_arguments(args: Vec<String>) {
    *ARGUMENTS.lock().unwrap_or_else(|e| e.into_inner()) = args;
}

pub fn typed(word: &str) -> Subject {
    let kept = ARGUMENTS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let args = kept.as_slice();
    if let Some(i) = args.iter().position(|a| a == word || a.split('=').any(|piece| piece == word)) {
        return Subject::Placed { at: i + 1, bytes: args[i].len() };
    }
    let sep = |b: u8| b == b'/' || b == b'\\';
    let under = |a: &String| {
        !a.is_empty() && word.len() > a.len() && word.starts_with(a.as_str()) && (a.bytes().last().is_some_and(sep) || sep(word.as_bytes()[a.len()]))
    };
    match args.iter().enumerate().filter(|(_, a)| under(a)).max_by_key(|(_, a)| a.len()) {
        Some((i, a)) => Subject::Placed { at: i + 1, bytes: a.len() },
        None => Subject::Unplaced { bytes: word.len() },
    }
}

pub fn unechoed_endpoint(at: usize, spec: &str) -> Subject {
    use eravon_chain::rpc::{endpoint_spec, NotAnEndpoint};
    let chain = match endpoint_spec(spec) {
        Ok((c, _)) => Some(c),
        Err(NotAnEndpoint::NoAddress) => spec.split_once('=').and_then(|(c, _)| c.trim().parse::<u64>().ok()),
        Err(_) => None,
    };
    match (chain, spec.split_once('=')) {
        (Some(chain), Some((_, address))) => Subject::Endpoint { at, chain, bytes: address.len() },
        _ => Subject::Placed { at, bytes: spec.len() },
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Said {
    Usage,
    NotAVerb,
    NotAFlag,
    NeedsValue,
    NotThisVerbsFlag,
    Repeated,
    Missing,
    OneOf,
    AnyOf,
    NotInt,
    KeyNotHex,
    KeyOutOfRange,
    Unreadable,
    NotJson,
    NotLawJson,
    NotUtf8,
    Sealed,
    EndpointShape,
    EndpointScheme,
    ProxyShape,
    NeedEndpoint,
    OneEndpoint,
    ChainIdNotInt,
    NotHex,
    Not32,
    Not20,
    RpcNotObject,
    FormWord,
    NeedHashOrCalldata,
    FileShape,
    EntryNotId,
    SeqPrevPair,
    NotWithFixture,
    EndpointPort,
    EndpointAddress,
    PastIntCeiling,
    KeyFileOpen,
    NotWithHome,
    DesktopLocked,
    DoorAlone,
    DesktopClosed,
    DoorPathLong,
    DoorNotYou,
    DoorBroken,
    NoMachine,
    OnDesktop,
    Queued,
    Words(&'static str, &'static str),
    ProxyCredentialsMissing,
    ProxyCredentialsRefused,
    PacRemoteHttp,
    PacUnfetched,
    PacFailed,
    McpAbsolute,
    McpNotAParameter,
    McpNotText,
    McpNotTexts,
    McpNoProgram,
    McpNotThroughDesktop,
    CredentialExpired,
    CredentialMissing,
    CredentialShape,
    CredentialPassMissing,
    CredentialPassUnknown,
}

impl Said {
    pub const ALL: [Said; 63] = [
        Said::Usage,
        Said::NotAVerb,
        Said::NotAFlag,
        Said::NeedsValue,
        Said::NotThisVerbsFlag,
        Said::Repeated,
        Said::Missing,
        Said::OneOf,
        Said::AnyOf,
        Said::NotInt,
        Said::KeyNotHex,
        Said::KeyOutOfRange,
        Said::Unreadable,
        Said::NotJson,
        Said::NotLawJson,
        Said::NotUtf8,
        Said::Sealed,
        Said::EndpointShape,
        Said::EndpointScheme,
        Said::ProxyShape,
        Said::NeedEndpoint,
        Said::OneEndpoint,
        Said::ChainIdNotInt,
        Said::NotHex,
        Said::Not32,
        Said::Not20,
        Said::RpcNotObject,
        Said::FormWord,
        Said::NeedHashOrCalldata,
        Said::FileShape,
        Said::EntryNotId,
        Said::SeqPrevPair,
        Said::NotWithFixture,
        Said::EndpointPort,
        Said::EndpointAddress,
        Said::PastIntCeiling,
        Said::KeyFileOpen,
        Said::NotWithHome,
        Said::DesktopLocked,
        Said::DoorAlone,
        Said::DesktopClosed,
        Said::DoorPathLong,
        Said::DoorNotYou,
        Said::DoorBroken,
        Said::NoMachine,
        Said::OnDesktop,
        Said::Queued,
        Said::ProxyCredentialsMissing,
        Said::ProxyCredentialsRefused,
        Said::PacRemoteHttp,
        Said::PacUnfetched,
        Said::PacFailed,
        Said::McpAbsolute,
        Said::McpNotAParameter,
        Said::McpNotText,
        Said::McpNotTexts,
        Said::McpNoProgram,
        Said::McpNotThroughDesktop,
        Said::CredentialExpired,
        Said::CredentialMissing,
        Said::CredentialShape,
        Said::CredentialPassMissing,
        Said::CredentialPassUnknown,
    ];

    pub fn words(self) -> (&'static str, &'static str) {
        match self {
            Said::Usage => (crate::args::usage(), crate::args::usage()),
            Said::NotAVerb => ("不是一个动词", "not a verb"),
            Said::NotAFlag => ("不是 --名字 的形", "not in the form --name"),
            Said::NeedsValue => ("这一旗要一个值", "this flag needs a value"),
            Said::NotThisVerbsFlag => ("不在这个动词的旗单里", "not among this verb's flags"),
            Said::Repeated => ("给了不止一次", "given more than once"),
            Said::Missing => ("缺这一旗", "this flag is missing"),
            Said::OneOf => ("这几旗之中恰要一面", "exactly one of these flags is wanted"),
            Said::AnyOf => ("这几旗之中至少要一面", "at least one of these flags is wanted"),
            Said::NotInt => ("不是一个整数", "not an integer"),
            Said::KeyNotHex => ("不是六十四位十六进制", "not 64 hex digits"),
            Said::KeyOutOfRange => ("不在曲线的范围里", "not within the curve's range"),
            Said::Unreadable => ("读不出", "cannot be read"),
            Said::NotJson => ("不是 JSON", "not JSON"),
            Said::NotLawJson => ("不是法 §3.5 收的 JSON", "not JSON the law takes (§3.5)"),
            Said::NotUtf8 => ("参数不是合法 UTF-8", "an argument is not valid UTF-8"),
            Said::Sealed => ("已锁定:这是 ERAVON Desk 封存的本机数据;命令行只凭凭据读桌面所管之家的账本", "Locked: this is local data ERAVON Desk keeps sealed; the command line reads it only as the ledger of a home the desktop keeps, with the credential"),
            Said::EndpointShape => ("--endpoint 的形是 <链号>=<url>", "--endpoint is <chain id>=<url>"),
            Said::EndpointScheme => ("端点只认 http:// 或 https:// 地址", "an endpoint is an http:// or https:// address"),
            Said::ProxyShape => ("--proxy 取 system、none 或代理地址", "--proxy takes system, none or a proxy address"),
            Said::NeedEndpoint => ("至少要一个 --endpoint <链号>=<url>", "at least one --endpoint <chain id>=<url> is wanted"),
            Said::OneEndpoint => ("anchor 只往一条链上发,各 --endpoint 要同一链号", "anchor sends to one chain: every --endpoint names the same chain"),
            Said::ChainIdNotInt => ("链号不是十进制整数", "the chain id is not a decimal integer"),
            Said::NotHex => ("不是十六进制", "not hex"),
            Said::Not32 => ("不是三十二字节", "not 32 bytes"),
            Said::Not20 => ("不是二十字节", "not 20 bytes"),
            Said::RpcNotObject => ("录制的 rpc 不是对象", "the recording's rpc is not an object"),
            Said::FormWord => ("--form 只认 registry", "--form takes registry"),
            Said::NeedHashOrCalldata => ("至少要一个 --hash,或一段 --calldata", "at least one --hash, or a --calldata, is wanted"),
            Said::FileShape => ("--file 的形是 <包内路径>=<盘上的路>", "--file is <path in the bundle>=<path on disk>"),
            Said::EntryNotId => ("不是六十四位小写十六进制", "not 64 lowercase hex digits"),
            Said::SeqPrevPair => ("--seq 与 --prev 要么都给,要么都不给", "--seq and --prev go together or not at all"),
            Said::NotWithFixture => ("读录制那一路不用这一旗", "a run over recordings does not take this flag"),
            Said::EndpointPort => ("端点地址的端口要是 0 到 65535 之间的整数", "an endpoint's port must be a whole number from 0 to 65535"),
            Said::EndpointAddress => ("不是一处端点地址(主机缺、方括号里不是 IPv6 地址，或带了用户名或空白)", "not an endpoint address (no host, a bracket that is not an IPv6 address, or a user name or white space in it)"),
            Said::PastIntCeiling => ("过了整数上界 9007199254740991", "past the whole-number ceiling 9007199254740991"),
            Said::KeyFileOpen => ("这份密钥档别的账户也读得到(只许本人可读)", "other accounts can read this key file (it must be readable by its owner only)"),
            Said::NotWithHome => ("带 --desktop 时不收这一旗(桌面那一侧用它自己开着的金库)", "not taken together with --desktop (the desktop uses its own open vault)"),
            Said::DesktopLocked => ("桌面锁着、没有运行、没有以写者开着这个数据目录,或「经桌面 App 操作」关着;解锁桌面、在设置里打开「经桌面 App 操作」,或把 --desktop 改成桌面开着的那个数据目录", "The desktop is locked, not running, not holding this data folder as its writer, or has “Through the desktop app” off; unlock it, turn that switch on in Settings, or give --desktop the data folder the desktop holds"),
            Said::DoorAlone => ("纯命令行写入开着,桌面不收 --desktop;去掉 --desktop 直写", "Writing alone is on, so the desktop takes no --desktop; write without --desktop"),
            Said::DesktopClosed => ("桌面在答之前锁上、关了这处数据或退出了", "The desktop locked, closed this data folder or quit before it answered"),
            Said::DoorPathLong => ("本机数据所在文件夹的路径太长,连不到桌面", "The path of this machine's data folder is too long to reach the desktop"),
            Said::DoorNotYou => ("桌面那一端不是本机这位用户", "The desktop at the other end is another user's"),
            Said::DoorBroken => ("桌面答到一半断了,或答的形这一版读不出", "The desktop broke off, or answered in a form this version cannot read"),
            Said::NoMachine => ("找不到本机数据所在的文件夹", "This machine's data folder cannot be found"),
            Said::OnDesktop => ("此动作须在桌面上做", "This action must be done on the desktop"),
            Said::Queued => ("已在队列,待用户在桌面上发", "Queued; waiting for the user to send it from the desktop"),
            Said::Words(zh, en) => (zh, en),
            Said::ProxyCredentialsMissing => (
                "代理要用户名与密码:在 --proxy 写成 协议://用户名:密码@主机:端口(其中的 @、: 写成 %40、%3A)",
                "The proxy asks for a user name and password: write them in --proxy as scheme://user:password@host:port (an @ or : in them as %40, %3A)",
            ),
            Said::ProxyCredentialsRefused => (
                "代理不收所给的用户名与密码:请核对 --proxy 里写的(其中的 @、: 写成 %40、%3A)",
                "The proxy refused the user name and password given: check the ones written in --proxy (an @ or : in them as %40, %3A)",
            ),
            Said::PacRemoteHttp => ("代理脚本在远程 http 上,未用:请用 --proxy 指定代理", "Proxy script is on remote http, not used: give a proxy with --proxy"),
            Said::PacUnfetched => ("取不到代理脚本:请开启代理软件,或用 --proxy 指定代理", "Can't fetch the proxy script: start your proxy app, or give a proxy with --proxy"),
            Said::PacFailed => ("代理脚本出错({0}):请用 --proxy 指定代理", "Proxy script failed ({0}): give a proxy with --proxy"),
            Said::McpAbsolute => ("要一条完整路径(从根起)", "a full path is needed (from the root)"),
            Said::McpNotAParameter => ("不是这个工具的参数", "not a parameter of this tool"),
            Said::McpNotText => ("要一段文字", "text is needed"),
            Said::McpNotTexts => ("要一列文字", "a list of texts is needed"),
            Said::McpNoProgram => ("起不了命令行程序", "the command line program could not be started"),
            Said::McpNotThroughDesktop => ("经桌面这一路不收这一参", "the way through the desktop does not take this parameter"),
            Said::CredentialExpired => ("命令行凭据已过期;到桌面重新打开纯命令行写入", "The command-line credential has expired; turn writing alone on again on the desktop"),
            Said::CredentialMissing => ("本机没有命令行凭据;到桌面打开纯命令行写入", "This machine has no command-line credential; turn writing alone on in the desktop"),
            Said::CredentialShape => ("命令行凭据无效;到桌面关掉再打开纯命令行写入", "The command-line credential is not valid; turn writing alone off and on again on the desktop"),
            Said::CredentialPassMissing => ("没有带程序钥:把钥放进 ERAVON_CLI_PASS", "No program key brought: put it in ERAVON_CLI_PASS"),
            Said::CredentialPassUnknown => ("这把程序钥开不了凭据:到桌面添一把新钥", "This program key opens nothing: add a new one on the desktop"),
        }
    }

    pub fn text(self) -> &'static str {
        let (zh, en) = self.words();
        match lang() {
            Lang::Zh => zh,
            Lang::En => en,
        }
    }
}

pub fn body_line(member: &str, (zh, en): (&str, &str)) -> String {
    match lang() {
        Lang::Zh => format!("{member}: 缺或不成形,由 {zh} 给"),
        Lang::En => format!("{member}: missing or malformed; given by {en}"),
    }
}

pub fn misuse(reason: Reason, subject: Subject, said: Said) -> ! {
    eprintln!("{} {}", reason.as_str(), subject.spelled());
    eprintln!("{}", said.text());
    std::process::exit(Exit::Misuse.code() as i32)
}
