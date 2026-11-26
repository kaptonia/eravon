use crate::rpc::{self, Trouble};
use crate::said::{self, Refusal};
use crate::seam::NodeAt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Transport(String),
    Endpoint(String),
    Execution(String),
    Disagree(String),
    WrongChain(String),
    Usage(String),
    Unread(String),
    Echo(String),
    OverAsks(String),
}

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::Transport(_) => "E_CHAIN_TRANSPORT",
            Fault::Endpoint(_) => "E_CHAIN_ENDPOINT",
            Fault::Execution(_) => "E_CHAIN_EXECUTION",
            Fault::Disagree(_) => "E_CHAIN_DISAGREE",
            Fault::WrongChain(_) => "E_CHAIN_WRONG",
            Fault::Usage(_) => "E_CHAIN_USAGE",
            Fault::Unread(_) => "E_CHAIN_UNREAD",
            Fault::Echo(_) => "E_CHAIN_ECHO",
            Fault::OverAsks(_) => "E_CHAIN_OVER_ASKS",
        }
    }

    pub fn chain_class(&self) -> Option<&'static str> {
        match self {
            Fault::Transport(_) => Some("transport"),
            Fault::Endpoint(_) => Some("endpoint"),
            Fault::Execution(_) => Some("execution"),
            _ => None,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Transport(s)
            | Fault::Endpoint(s)
            | Fault::Execution(s)
            | Fault::Disagree(s)
            | Fault::WrongChain(s)
            | Fault::Usage(s)
            | Fault::Unread(s)
            | Fault::Echo(s)
            | Fault::OverAsks(s) => s.clone(),
        }
    }
}

pub type R<T> = Result<T, Fault>;

pub fn word(r: &Refusal) -> &'static str {
    match r {
        Refusal::Funds => "funds",
        Refusal::NonceUsed => "nonce-used",
        Refusal::Pending => "pending",
        Refusal::Underpriced => "underpriced",
        Refusal::GasTooLow => "gas-too-low",
        Refusal::Reverted => "reverted",
        Refusal::RateLimited => "rate-limited",
        Refusal::NoMethod => "no-method",
        Refusal::Auth => "auth",
        Refusal::WrongChain => "wrong-chain",
        Refusal::Coded(_) => "coded",
        Refusal::Other(_) => "other",
    }
}

fn of_word(first: &str, rest: &str) -> Option<Refusal> {
    Some(match first {
        "funds" => Refusal::Funds,
        "nonce-used" => Refusal::NonceUsed,
        "pending" => Refusal::Pending,
        "underpriced" => Refusal::Underpriced,
        "gas-too-low" => Refusal::GasTooLow,
        "reverted" => Refusal::Reverted,
        "rate-limited" => Refusal::RateLimited,
        "no-method" => Refusal::NoMethod,
        "auth" => Refusal::Auth,
        "wrong-chain" => Refusal::WrongChain,
        "coded" => Refusal::Coded(rest.to_string()),
        "other" => Refusal::Other(rest.to_string()),
        _ => return None,
    })
}

pub fn node_refusal(err: &str) -> Fault {
    let w = crate::wire::parse(err.as_bytes());
    let r = refusal_here(err, w.as_ref());
    let code = match w.as_ref().and_then(|x| x.member("code")).map(|c| &c.body) {
        Some(crate::wire::Body::Num(n)) => n.clone(),
        _ => "?".to_string(),
    };
    let message = w.as_ref().and_then(|x| x.member("message")).and_then(|m| m.as_str()).map_or_else(|| err.to_string(), str::to_string);
    let mut text = format!("{} 码 {code}:{message}", word(&r));
    if r == Refusal::Reverted {
        if let Some(said) = said::reverted(err) {
            text.push_str(&format!(" · {}", said.evidence()));
        }
    }
    let chain_range = code.parse::<i64>().ok().is_some_and(|c| c == 3 || (-32099..=-32000).contains(&c));
    let execution = match &r {
        Refusal::Coded(_) => chain_range,
        other => other.about_the_call(),
    };
    if execution { Fault::Execution(text) } else { Fault::Endpoint(text) }
}

pub fn refusal_here(err: &str, w: Option<&crate::wire::W>) -> Refusal {
    let code = w.and_then(|x| x.member("code")).and_then(|c| match &c.body {
        crate::wire::Body::Num(n) => n.parse::<i64>().ok(),
        _ => None,
    });
    let says_reverted = w.and_then(|x| x.member("message")).and_then(|m| m.as_str()).is_some_and(|m| m.trim_start().to_lowercase().starts_with("execution reverted"));
    if code == Some(3) || says_reverted || said::reverted(err).is_some() {
        return Refusal::Reverted;
    }
    said::refusal_of(err)
}

pub fn of_trouble(node: &NodeAt, t: Trouble, kept: Option<Fault>) -> Fault {
    if let Some(f) = kept {
        return f;
    }
    match t {
        Trouble::Node(e) => node_refusal(&e),
        Trouble::Transport(b) => match said::refusal(&Trouble::Transport(b.clone())) {
            Some(r) => Fault::Endpoint(format!("{} {}", word(&r), b.evidence())),
            None => match b {
                rpc::Broke::NotJson => Fault::Endpoint(format!("malformed {node} {}", b.evidence())),
                rpc::Broke::Shapeless { .. } => Fault::Endpoint(format!("malformed {}", b.evidence())),
                rpc::Broke::Unserved { .. } | rpc::Broke::NoKey { .. } => Fault::Endpoint(b.evidence()),
                _ => Fault::Transport(b.evidence()),
            },
        },
        Trouble::NotServed(k) => Fault::Unread(k),
        Trouble::Contradiction(k) => Fault::Endpoint(format!("malformed {k}")),
    }
}

pub fn refusal(f: &Fault) -> Option<Refusal> {
    let (Fault::Execution(d) | Fault::Endpoint(d)) = f else { return None };
    let (first, rest) = d.split_once(' ').unwrap_or((d.as_str(), ""));
    of_word(first, rest)
}

pub fn refusal_word(name: &str, detail: &str) -> Option<Refusal> {
    let f = if name == Fault::Execution(String::new()).name() {
        Fault::Execution(detail.to_string())
    } else if name == Fault::Endpoint(String::new()).name() {
        Fault::Endpoint(detail.to_string())
    } else {
        return None;
    };
    refusal(&f)
}

pub fn reverted(f: &Fault) -> bool {
    refusal(f) == Some(Refusal::Reverted)
}

pub fn reverted_word(name: &str, detail: &str) -> bool {
    refusal_word(name, detail) == Some(Refusal::Reverted)
}

pub fn status_of(f: &Fault) -> Option<u16> {
    let Fault::Endpoint(d) = f else { return None };
    let said = d.split(crate::ask::BODY_SEP).next().unwrap_or(d);
    said.split(' ').find_map(|w| w.strip_prefix("HTTP_")).and_then(|c| c.parse().ok()).filter(|c: &u16| (100..1000).contains(c))
}


#[cfg(test)]
mod tests {
    use super::*;

    fn node(m: &str, code: i64) -> String {
        format!("{{\"code\":{code},\"message\":\"{m}\"}}")
    }

    #[test]
    fn a_refusal_travels_as_the_first_word_of_its_detail() {
        for (said, want) in [
            ("insufficient funds for gas * price + value", Refusal::Funds),
            ("nonce too low", Refusal::NonceUsed),
            ("already known", Refusal::Pending),
            ("replacement transaction underpriced", Refusal::Underpriced),
            ("intrinsic gas too low", Refusal::GasTooLow),
            ("execution reverted", Refusal::Reverted),
            ("daily request limit exceeded", Refusal::RateLimited),
            ("the method eth_x does not exist/is not available", Refusal::NoMethod),
            ("unauthorized: bad api key", Refusal::Auth),
            ("invalid chain id for signer", Refusal::WrongChain),
            ("execution reverted: rate limit", Refusal::Reverted),
        ] {
            assert_eq!(refusal(&node_refusal(&node(said, -32000))), Some(want), "{said}: read back from the detail");
        }
        for (said, code) in [("execution reverted: insufficient balance", -32000), ("execution reverted: invalid chain id", -32000), ("insufficient balance", 3), ("nonce has already been used", 3)] {
            assert_eq!(refusal(&node_refusal(&node(said, code))), Some(Refusal::Reverted), "{said} · {code}");
        }
        assert!(matches!(refusal(&node_refusal(&node("exceeds block gas limit", -32000))), Some(Refusal::Coded(_))));
        assert!(matches!(refusal(&node_refusal("not json at all")), Some(Refusal::Other(_))));
        let at = |s: u16| of_trouble(&"https://n.example".into(), crate::rpc::status("https://n.example", s), None);
        assert_eq!(refusal(&at(429)), Some(Refusal::RateLimited));
        assert_eq!(refusal(&at(401)), Some(Refusal::Auth));
        assert_eq!(refusal(&at(403)), Some(Refusal::Auth));
        for s in [400, 404, 428, 430, 500, 502, 503, 301] {
            let f = at(s);
            assert!(matches!(refusal(&f), Some(Refusal::Other(_))) && status_of(&f) == Some(s), "{f:?}");
        }
        for f in [Fault::Transport("late x".into()), Fault::Disagree("x".into()), Fault::Usage("x".into())] {
            assert_eq!(refusal(&f), None, "{f:?}");
        }
    }

    #[test]
    fn a_refusal_about_the_call_is_the_chains_and_one_about_the_node_is_the_endpoints() {
        assert_eq!(node_refusal(&node("insufficient funds", -32000)).name(), "E_CHAIN_EXECUTION");
        assert_eq!(node_refusal(&node("rate limit", -32000)).name(), "E_CHAIN_ENDPOINT");
        assert_eq!(node_refusal(&node("x", -32601)).name(), "E_CHAIN_ENDPOINT");
        assert_eq!(node_refusal(&node("header not found", -32000)).name(), "E_CHAIN_EXECUTION", "服务端段的码照旧归链上类");
        assert_eq!(node_refusal(&node("weird", 35)).name(), "E_CHAIN_ENDPOINT", "段外的码归端点类");
    }

    #[test]
    fn only_a_rate_limit_said_in_words_or_by_429_is_waited_out() {
        assert!(said::says_rate_limited(&Trouble::Node(node("rate limit exceeded", -32005))));
        assert!(!said::says_rate_limited(&Trouble::Node(node("query returned more than 10000 results", -32005))));
        assert!(said::says_rate_limited(&crate::rpc::status("https://n.example", 429)));
        assert!(!said::says_rate_limited(&crate::rpc::status("https://n.example", 503)));
    }

    #[test]
    fn a_revert_says_what_the_contract_said() {
        let error_string = [0x08, 0xc3, 0x79, 0xa0];
        let panic_sel = [0x4e, 0x48, 0x7b, 0x71];
        let encode = |reason: &[u8]| {
            let mut enc = error_string.to_vec();
            let mut w = [0u8; 32];
            w[31] = 32;
            enc.extend_from_slice(&w);
            let mut l = [0u8; 32];
            l[30] = ((reason.len() >> 8) & 0xff) as u8;
            l[31] = (reason.len() & 0xff) as u8;
            enc.extend_from_slice(&l);
            let mut body = reason.to_vec();
            body.resize(reason.len().div_ceil(32) * 32, 0);
            enc.extend_from_slice(&body);
            enc
        };
        assert_eq!(said::reverted_of(&encode(b"Not seller")), said::Reverted::Text("Not seller".into()));
        let mut panic = panic_sel.to_vec();
        let mut p = [0u8; 32];
        p[31] = 0x11;
        panic.extend_from_slice(&p);
        assert_eq!(said::reverted_of(&panic).evidence(), "panic 0x11: arithmetic overflow or underflow");
        assert_eq!(said::reverted_of(&[1, 2, 3, 4, 5, 6]), said::Reverted::Custom([1, 2, 3, 4], 2));
        assert_eq!(said::reverted_of(&[]), said::Reverted::Silent);
        assert!(matches!(said::reverted_of(&[1, 2]), said::Reverted::Unreadable(_)));
        assert!(matches!(said::reverted_of(&panic_sel), said::Reverted::Unreadable(_)));
        let long = "x".repeat(said::REASON_CAP + 5);
        assert!(matches!(said::reverted_of(&encode(long.as_bytes())), said::Reverted::Text(t) if t.ends_with("(5 more characters)")));
        let err = format!("{{\"code\":3,\"message\":\"execution reverted\",\"data\":\"{}\"}}", eravon::hexfmt::encode(&encode(b"Not seller")));
        let f = node_refusal(&err);
        assert_eq!(f.name(), "E_CHAIN_EXECUTION");
        assert!(f.detail().starts_with("reverted 码 3:execution reverted · reason: Not seller"), "{}", f.detail());
    }
}
