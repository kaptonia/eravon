use crate::fault::{Fault, R};

pub const JSONRPC: &str = "2.0";

pub const M_CHAIN_ID: &str = "eth_chainId";
pub const M_BLOCK_NUMBER: &str = "eth_blockNumber";
pub const M_GET_BLOCK: &str = "eth_getBlockByNumber";
pub const M_GET_BALANCE: &str = "eth_getBalance";
pub const M_CALL: &str = "eth_call";
pub const M_GET_LOGS: &str = "eth_getLogs";

pub const LOG_WINDOW: u64 = 10_000;

pub const FILTER_FAN: usize = 32;
pub const BATCH_MAX: usize = 20;
pub const BATCHES_IN_FLIGHT: usize = 2;
pub const TRACES_IN_FLIGHT: usize = 2;
pub const ITEMS_AT_ONCE: usize = 6;

pub const HEIGHT_SLACK: u64 = 64;
pub const M_ESTIMATE_GAS: &str = "eth_estimateGas";
pub const M_GAS_PRICE: &str = "eth_gasPrice";
pub const M_FEE_HISTORY: &str = "eth_feeHistory";
pub const M_GET_CODE: &str = "eth_getCode";
pub const M_GET_TX: &str = "eth_getTransactionByHash";
pub const M_SEND_RAW: &str = "eth_sendRawTransaction";
pub const M_GET_RECEIPT: &str = "eth_getTransactionReceipt";
pub const M_GET_TX_COUNT: &str = "eth_getTransactionCount";

pub const M_GET_BLOCK_BY_HASH: &str = "eth_getBlockByHash";
pub const M_GET_RAW_TX: &str = "eth_getRawTransactionByHash";
pub const M_GET_BLOCK_RECEIPTS: &str = "eth_getBlockReceipts";
pub const M_GET_PROOF: &str = "eth_getProof";

pub const METHODS: [&str; 18] = [
    M_CHAIN_ID,
    M_BLOCK_NUMBER,
    M_GET_BLOCK,
    M_GET_BALANCE,
    M_CALL,
    M_GET_LOGS,
    M_ESTIMATE_GAS,
    M_GAS_PRICE,
    M_FEE_HISTORY,
    M_GET_CODE,
    M_GET_TX,
    M_SEND_RAW,
    M_GET_RECEIPT,
    M_GET_TX_COUNT,
    M_GET_BLOCK_BY_HASH,
    M_GET_RAW_TX,
    M_GET_BLOCK_RECEIPTS,
    M_GET_PROOF,
];

pub const TAG_LATEST: &str = "latest";

pub const TAG_SAFE: &str = "safe";

pub const TAG_FINALIZED: &str = "finalized";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upto {
    Block(u64),
    Latest,
}

impl Upto {
    pub fn tag(self) -> String {
        match self {
            Upto::Block(n) => quantity(n),
            Upto::Latest => TAG_LATEST.to_string(),
        }
    }
}

impl Upto {
    pub fn of_range(from: u64, to: Option<u64>, blocks: Option<u64>) -> crate::fault::R<Upto> {
        use crate::fault::Fault;
        match (to, blocks) {
            (Some(_), Some(_)) => Err(Fault::Usage("上界只许一种写法:--to 与 --blocks 不能同给".into())),
            (Some(n), None) => Ok(Upto::Block(n)),
            (None, Some(0)) => Err(Fault::Usage("--blocks 至少一块:零块是空区间".into())),
            (None, Some(k)) => from
                .checked_add(last_offset(k))
                .map(Upto::Block)
                .ok_or_else(|| Fault::Usage(format!("--from {from} 加 --blocks {k} 越了块号的界"))),
            (None, None) => Ok(Upto::Latest),
        }
    }
}

fn last_offset(k: u64) -> u64 {
    k - 1
}


impl From<u64> for Upto {
    fn from(n: u64) -> Upto {
        Upto::Block(n)
    }
}

#[cfg(test)]
mod range_tests {
    use super::Upto;

    #[test]
    fn one_reading_for_both_upper_bounds() {
        assert_eq!(Upto::of_range(5, None, None).unwrap(), Upto::Latest);
        assert_eq!(Upto::of_range(5, Some(9), None).unwrap(), Upto::Block(9));
        assert_eq!(Upto::of_range(5, None, Some(3)).unwrap(), Upto::Block(7));
        assert_eq!(Upto::of_range(5, None, Some(1)).unwrap(), Upto::Block(5));
        assert!(Upto::of_range(5, Some(9), Some(3)).is_err());
        assert!(Upto::of_range(5, None, Some(0)).is_err());
        assert!(Upto::of_range(u64::MAX, None, Some(2)).is_err());
    }
}

pub fn home_chain_id() -> u64 {
    eravon_literals::constants::active_standard().chain_id
}

pub const BENCH_CHAIN_ID: u64 = 31337;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl Url {
    pub fn authority(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn target(&self) -> eravon_net::Target {
        let scheme = if self.scheme == "https" { eravon_net::Scheme::Https } else { eravon_net::Scheme::Http };
        eravon_net::Target { scheme, host: self.host.clone(), port: self.port, path: self.path.clone() }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAt(String);

impl NodeAt {
    pub fn new(written: impl Into<String>) -> NodeAt {
        NodeAt(written.into())
    }

    pub fn said(&self) -> String {
        eravon_net::sayable(&self.0)
    }


    pub(crate) fn url(&self) -> R<Url> {
        parse_url(&self.0)
    }

    pub(crate) fn place(&self) -> String {
        crate::endpoints::place_of(&self.0)
    }

    pub(crate) fn written(&self) -> &str {
        &self.0
    }
}

impl From<String> for NodeAt {
    fn from(s: String) -> NodeAt {
        NodeAt(s)
    }
}

impl From<&str> for NodeAt {
    fn from(s: &str) -> NodeAt {
        NodeAt(s.to_string())
    }
}

impl std::fmt::Display for NodeAt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.said())
    }
}

impl std::fmt::Debug for NodeAt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NodeAt").field(&self.said()).finish()
    }
}

fn address_word(why: eravon_net::NotAnAddress) -> &'static str {
    match why {
        eravon_net::NotAnAddress::Scheme => "scheme",
        eravon_net::NotAnAddress::Port => "port",
        eravon_net::NotAnAddress::Host => "host",
        eravon_net::NotAnAddress::Shape => "shape",
    }
}

pub fn parse_url(s: &str) -> R<Url> {
    parse_url_at(None, s)
}

pub fn parse_url_at(n: Option<usize>, s: &str) -> R<Url> {
    let t = eravon_net::read_address(s).map_err(|why| {
        let what = match why {
            eravon_net::NotAnAddress::Scheme => "端点地址要带 scheme,且只收 http 与 https",
            eravon_net::NotAnAddress::Port => "端点地址的端口要是十六位以内的数",
            eravon_net::NotAnAddress::Host => "端点地址缺主机,或方括号里不是 IPv6 地址",
            eravon_net::NotAnAddress::Shape => "端点地址里不许有用户信息、空白或控制字符",
        };
        let place = n.map(|n| format!("第 {n} 个端点 ")).unwrap_or_default();
        Fault::Usage(format!("{} {place}{what}({} 字节)", address_word(why), s.len()))
    })?;
    let said = NodeAt::new(s).said();
    if said != s {
        eravon_literals::echo::hide(s, &said);
    }
    Ok(Url { scheme: t.scheme.as_str().to_string(), host: t.host, port: t.port, path: t.path })
}

pub fn node_at(n: usize, s: &str) -> R<()> {
    let Some((base, _)) = crate::rest::of_lane(s) else {
        return parse_url_at(Some(n), s).map(|_| ());
    };
    let word = match crate::rest::base_of(base) {
        Ok(_) => {
            let said = NodeAt::new(s).said();
            if said != s {
                eravon_literals::echo::hide(s, &said);
            }
            return Ok(());
        }
        Err(crate::rest::NotABase::Scheme) => "scheme",
        Err(crate::rest::NotABase::Query) => "shape",
        Err(crate::rest::NotABase::Address(why)) => address_word(why),
    };
    Err(Fault::Usage(format!("{word} 第 {n} 个端点 REST 行的 Base URL 要 https:// 起头、不带查询串({} 字节)", base.len())))
}

pub fn quantity(n: u64) -> String {
    format!("0x{n:x}")
}

pub fn hex_of(b: &[u8]) -> String {
    let mut s = String::with_capacity(2 + b.len() * 2);
    s.push_str("0x");
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

pub fn bytes_of(what: &str, s: &str) -> R<Vec<u8>> {
    let body = s
        .strip_prefix("0x")
        .ok_or_else(|| Fault::Usage(format!("{what} 要以 0x 起头:{s:?}")))?;
    if body.len() % 2 != 0 {
        return Err(Fault::Usage(format!("{what} 的位数要是偶数:{s:?}")));
    }
    let mut out = Vec::with_capacity(body.len() / 2);
    for pair in body.as_bytes().chunks(2) {
        let two = std::str::from_utf8(pair).map_err(|_| Fault::Usage(format!("{what} 不合形")))?;
        out.push(
            u8::from_str_radix(two, 16)
                .map_err(|_| Fault::Usage(format!("{what} 里有不是十六进制的字符:{s:?}")))?,
        );
    }
    Ok(out)
}

pub fn need_quantity(what: &str, s: &str) -> R<()> {
    let body = s
        .strip_prefix("0x")
        .ok_or_else(|| Fault::Usage(format!("{what} 要以 0x 起头:{s:?}")))?;
    if body.is_empty() {
        return Err(Fault::Usage(format!("{what} 至少要一位十六进制:{s:?}")));
    }
    if !body.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
        return Err(Fault::Usage(format!("{what} 里有不是小写十六进制的字符:{s:?}")));
    }
    if body.len() > 1 && body.starts_with('0') {
        return Err(Fault::Usage(format!("{what} 要最短的拼法(零写作 0x0):{s:?}")));
    }
    Ok(())
}

pub fn read_quantity(s: &str) -> R<u64> {
    let body = s
        .strip_prefix("0x")
        .ok_or_else(|| Fault::Endpoint(format!("数量形要以 0x 起头:{s:?}")))?;
    if body.is_empty() || body.len() > 16 {
        return Err(Fault::Endpoint(format!("数量形的位数不合:{s:?}")));
    }
    u64::from_str_radix(body, 16)
        .map_err(|_| Fault::Endpoint(format!("数量形里有不是十六进制的字符:{s:?}")))
}

pub fn read_big_decimal(s: &str) -> R<String> {
    let body = s
        .strip_prefix("0x")
        .ok_or_else(|| Fault::Endpoint(format!("数量形要以 0x 起头:{s:?}")))?;
    if body.is_empty() || !body.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Endpoint(format!("数量形不合:{s:?}")));
    }
    let mut digits: Vec<u8> = vec![0];
    for c in body.bytes() {
        let v = (c as char).to_digit(16).unwrap_or(0) as u32;
        let mut carry = v;
        for d in digits.iter_mut() {
            let x = u32::from(*d) * 16 + carry;
            *d = (x % 10) as u8;
            carry = x / 10;
        }
        while carry > 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    while digits.len() > 1 && *digits.last().unwrap() == 0 {
        digits.pop();
    }
    Ok(digits.iter().rev().map(|d| char::from(b'0' + d)).collect())
}

pub fn address_ok(a: &str) -> bool {
    a.len() == 42 && a.starts_with("0x") && a[2..].bytes().all(|c| c.is_ascii_hexdigit())
}

pub fn need_address(a: &str) -> R<()> {
    if address_ok(a) {
        Ok(())
    } else {
        Err(Fault::Usage(format!("地址要 0x 加四十位十六进制,给的是 {a:?}")))
    }
}

pub fn need_hex_bytes(what: &str, s: &str) -> R<()> {
    let body = s
        .strip_prefix("0x")
        .ok_or_else(|| Fault::Usage(format!("{what} 要以 0x 起头:{s:?}")))?;
    if body.len() % 2 == 0 && body.bytes().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(Fault::Usage(format!("{what} 要是 0x 加偶数位十六进制:{s:?}")))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_node_shows_only_its_sayable_form() {
        for (written, said) in [
            ("https://node.example/v3/S3CRETk3y", "https://node.example"),
            ("https://node.example:8443/rpc?apikey=S3CRETk3y", "https://node.example:8443"),
            ("http://127.0.0.1:8545", "http://127.0.0.1:8545"),
        ] {
            let n = NodeAt::new(written);
            assert_eq!((n.said(), n.to_string(), format!("{n:?}")), (said.to_string(), said.to_string(), format!("NodeAt({said:?})")), "{written}");
        }
    }

    #[test]
    fn an_endpoint_read_is_hidden_as_it_is_said() {
        let written = "https://node.example:8443/v3/k3yHiddenAsSaid";
        parse_url(written).expect("reads");
        let scrubbed = eravon_literals::echo::scrub(format!("at {written} here").as_bytes());
        assert_eq!(String::from_utf8(scrubbed).expect("text"), format!("at {} here", NodeAt::new(written).said()));
    }

    use super::*;

    #[test]
    fn urls_come_apart_in_one_place_only() {
        let u = parse_url("http://127.0.0.1:8545").unwrap();
        assert_eq!((u.scheme.as_str(), u.host.as_str(), u.port, u.path.as_str()), ("http", "127.0.0.1", 8545, "/"));
        let u = parse_url("https://rpc.example.com/v1/abc").unwrap();
        assert_eq!((u.scheme.as_str(), u.port, u.path.as_str()), ("https", 443, "/v1/abc"));
        let up = parse_url("HTTPS://RPC.Example.com/v1").unwrap();
        assert_eq!((up.scheme.as_str(), up.host.as_str(), up.port), ("https", "rpc.example.com", 443));
        assert!(parse_url("ws://x").is_err());
        assert!(parse_url("127.0.0.1:8545").is_err());
        assert!(parse_url("http://").is_err());
        assert!(parse_url("http://host:notaport").is_err());
    }

    #[test]
    fn an_address_read_with_a_path_is_said_without_it() {
        parse_url("https://rpc.example/v3/PATHKEY?q=QUERYKEY").expect("reads");
        parse_url("http://127.0.0.1:8545").expect("reads");
        let out = eravon_literals::echo::scrub(br#"{"a":"https://rpc.example/v3/PATHKEY?q=QUERYKEY","b":"http://127.0.0.1:8545"}"#);
        assert_eq!(String::from_utf8(out).expect("utf-8"), r#"{"a":"https://rpc.example","b":"http://127.0.0.1:8545"}"#);
    }

    #[test]
    fn an_address_that_does_not_read_says_which_of_four_reasons() {
        let word = |s: &str| parse_url(s).err().map(|f| (f.name(), f.detail().split(' ').next().unwrap_or("").to_string()));
        assert_eq!(word("ws://h/"), Some(("E_CHAIN_USAGE", "scheme".into())));
        assert_eq!(word("https://h:65536/"), Some(("E_CHAIN_USAGE", "port".into())));
        assert_eq!(word("https://[nope]/"), Some(("E_CHAIN_USAGE", "host".into())));
        assert_eq!(word("https://user@h/"), Some(("E_CHAIN_USAGE", "shape".into())));
        assert_eq!(word("https://h x/"), Some(("E_CHAIN_USAGE", "shape".into())));
        let keyed = "https://rpc.example.com:99999/v1/SECRETKEY123";
        let f = parse_url_at(Some(2), keyed).expect_err("端口越界");
        assert!(f.detail().starts_with("port 第 2 个端点 ") && !f.detail().contains("SECRETKEY123") && f.detail().contains(&format!("({} 字节)", keyed.len())), "{}", f.detail());
        let u = parse_url("http://[::1]:8545/rpc").unwrap();
        assert_eq!((u.host.as_str(), u.port, u.target().dial_host().to_string()), ("[::1]", 8545, "::1".to_string()));
    }

    #[test]
    fn quantities_go_there_and_back() {
        assert_eq!(quantity(0), "0x0");
        assert_eq!(quantity(31337), "0x7a69");
        assert_eq!(read_quantity("0x7a69").unwrap(), 31337);
        assert!(read_quantity("7a69").is_err());
        assert!(read_quantity("0x").is_err());
        assert!(read_quantity("0xzz").is_err());
    }

    #[test]
    fn a_balance_past_two_to_the_sixty_four_reads_back_whole() {
        assert_eq!(read_big_decimal("0x21e19e0c9bab2400000").unwrap(), "10000000000000000000000");
        assert_eq!(read_big_decimal("0x0").unwrap(), "0");
    }

    #[test]
    fn the_home_chain_is_the_one_in_the_standard_table() {
        assert_eq!(home_chain_id(), eravon_literals::constants::current_network().standard.chain_id);
    }
}
