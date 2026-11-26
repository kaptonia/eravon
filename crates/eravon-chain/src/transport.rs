use crate::fault::{Fault, R};
use crate::seam::Url;

pub use eravon_net::Answer;

pub trait Transport: Sync + Send + Clone + 'static {
    fn post(&self, url: &Url, body: &[u8]) -> R<Answer>;

    fn post_once(&self, url: &Url, body: &[u8]) -> R<Answer> {
        self.post(url, body)
    }

    fn post_way(&self, url: &Url, body: &[u8], once: bool) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        let got = if once { self.post_once(url, body) } else { self.post(url, body) };
        got.map_err(|f| {
            let b = crate::rpc::Broke::Said(f.detail());
            (f, b)
        })
    }

    fn post_within(&self, url: &Url, body: &[u8], once: bool, within: std::time::Duration) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        let _ = within;
        self.post_way(url, body, once)
    }

    fn rest_node(&self, lane: &str, said: Box<dyn Fn(eravon_net::Fail) -> crate::rpc::Trouble + Send>) -> Option<Box<dyn crate::rpc::Endpoint + Send>> {
        crate::rest::Rest::open(lane, said).map(|r| Box::new(r) as Box<dyn crate::rpc::Endpoint + Send>)
    }

    fn get(&self, url: &Url) -> R<Vec<u8>> {
        Err(Fault::Usage(format!("这一种运输不取文档:{}://{}", url.scheme, url.authority())))
    }
}

pub use eravon_net::Choice;

pub use eravon_net::USER_AGENT as UA_PRODUCT;

pub fn fail_word(f: &eravon_net::Fail) -> &'static str {
    match f {
        eravon_net::Fail::Name(_) => "name",
        eravon_net::Fail::Connect(_) => "connect",
        eravon_net::Fail::Handshake(_) => "handshake",
        eravon_net::Fail::Certificate(_) => "certificate",
        eravon_net::Fail::Late(_) => "late",
        eravon_net::Fail::Overlong(_) => "overlong",
        eravon_net::Fail::Stream(_) => "stream",
        eravon_net::Fail::Proxy(_) => "proxy",
        eravon_net::Fail::ProxyCredentials { .. } => "proxy-credentials",
        eravon_net::Fail::Closed => "closed",
    }
}

pub fn transport_fault(url: &Url, f: &eravon_net::Fail) -> Fault {
    let words = match f {
        eravon_net::Fail::Name(x)
        | eravon_net::Fail::Connect(x)
        | eravon_net::Fail::Handshake(x)
        | eravon_net::Fail::Certificate(x)
        | eravon_net::Fail::Stream(x)
        | eravon_net::Fail::Proxy(x) => x.clone(),
        eravon_net::Fail::ProxyCredentials { given, said } => format!("{} {said}", if *given { crate::rpc::PROXY_REFUSES } else { crate::rpc::PROXY_ASKS }),
        eravon_net::Fail::Late(_) | eravon_net::Fail::Overlong(_) | eravon_net::Fail::Closed => broke_of(url, f).evidence(),
    };
    Fault::Transport(format!("{} {}://{}: {words}", fail_word(f), url.scheme, url.authority()))
}

pub(crate) fn broke_of(url: &Url, f: &eravon_net::Fail) -> crate::rpc::Broke {
    match crate::rpc::transport_said(&format!("{}://{}", url.scheme, url.authority()), f) {
        crate::rpc::Trouble::Transport(b) => b,
        other => crate::rpc::Broke::Said(format!("{other:?}")),
    }
}

pub fn document(url: &Url, a: Answer) -> R<Vec<u8>> {
    if (200..300).contains(&a.status) {
        return Ok(a.body);
    }
    Err(Fault::Endpoint(format!(
        "{}://{} 答 HTTP {}:{}",
        url.scheme,
        url.authority(),
        a.status,
        String::from_utf8_lossy(&a.body[..a.body.len().min(200)])
    )))
}

fn limits() -> eravon_net::Limits {
    eravon_net::Limits::from_env()
}

fn carry(url: &Url, body: &[u8], limits: &eravon_net::Limits, how: eravon_net::Ask) -> Result<Answer, (Fault, crate::rpc::Broke)> {
    eravon_net::post_json(&url.target(), body, limits, how).map_err(|f| (transport_fault(url, &f), broke_of(url, &f)))
}

fn how(once: bool) -> eravon_net::Ask {
    if once { eravon_net::Ask::Once } else { eravon_net::Ask::Read }
}

fn fetch(url: &Url, limits: &eravon_net::Limits) -> R<Vec<u8>> {
    let a = eravon_net::get(&url.target(), limits).map_err(|f| transport_fault(url, &f))?;
    document(url, a)
}

#[derive(Clone, Debug)]
pub struct Http {
    limits: eravon_net::Limits,
}

impl Default for Http {
    fn default() -> Self {
        Http { limits: limits() }
    }
}

impl Http {
    pub fn with_timeout(timeout: std::time::Duration) -> Http {
        Http { limits: eravon_net::Limits { deadline: timeout, ..limits() } }
    }

    fn plain_only(url: &Url) -> R<()> {
        if url.scheme == "https" {
            return Err(Fault::Usage(format!(
                "{}://{} 要 TLS,而这一种运输只走明文;https 的端点走 [`Tls`] 那一种(生产入口拿的 [`Auto`] 按 scheme 分路)",
                url.scheme,
                url.authority()
            )));
        }
        Ok(())
    }
}

impl Transport for Http {
    fn post(&self, url: &Url, body: &[u8]) -> R<Answer> {
        self.post_way(url, body, false).map_err(|(f, _)| f)
    }

    fn post_once(&self, url: &Url, body: &[u8]) -> R<Answer> {
        self.post_way(url, body, true).map_err(|(f, _)| f)
    }

    fn post_way(&self, url: &Url, body: &[u8], once: bool) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        self.post_within(url, body, once, std::time::Duration::ZERO)
    }

    fn post_within(&self, url: &Url, body: &[u8], once: bool, within: std::time::Duration) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        Http::plain_only(url).map_err(|f| (f.clone(), crate::rpc::Broke::Said(f.detail())))?;
        carry(url, body, &crate::rpc::within(self.limits, within), how(once))
    }

    fn get(&self, url: &Url) -> R<Vec<u8>> {
        Http::plain_only(url)?;
        fetch(url, &self.limits)
    }
}

#[derive(Clone, Debug)]
pub struct Tls {
    limits: eravon_net::Limits,
}

impl Tls {
    pub fn web_roots() -> R<Tls> {
        Ok(Tls { limits: limits() })
    }

    pub fn with_pem_roots(pem: &[u8]) -> R<Tls> {
        use rustls_pki_types::pem::PemObject;
        let ders = rustls_pki_types::CertificateDer::pem_slice_iter(pem)
            .map(|c| c.map(|c| c.as_ref().to_vec()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Fault::Usage(format!("根证书收不下:PEM 里有一张读不成:{e}")))?;
        let [der] = ders.as_slice() else {
            return Err(Fault::Usage(format!("根证书收不下:PEM 里要恰一张证书,有 {} 张", ders.len())));
        };
        static GIVEN: std::sync::Mutex<Option<Vec<u8>>> = std::sync::Mutex::new(None);
        let mut given = GIVEN.lock().unwrap_or_else(|e| e.into_inner());
        match given.as_ref() {
            Some(d) if d == der => {}
            Some(_) => return Err(Fault::Usage("根证书收不下:这一个进程已收了另一张".into())),
            None if eravon_net::drive_trust_root(der) => *given = Some(der.clone()),
            None => return Err(Fault::Usage("根证书收不下:握手的配置已先建好".into())),
        }
        Ok(Tls { limits: limits() })
    }

    fn tls_only(url: &Url) -> R<()> {
        if url.scheme != "https" {
            return Err(Fault::Usage(format!("这一种运输只走 https,给的是 {}", url.scheme)));
        }
        Ok(())
    }
}

impl Transport for Tls {
    fn post(&self, url: &Url, body: &[u8]) -> R<Answer> {
        self.post_way(url, body, false).map_err(|(f, _)| f)
    }

    fn post_once(&self, url: &Url, body: &[u8]) -> R<Answer> {
        self.post_way(url, body, true).map_err(|(f, _)| f)
    }

    fn post_way(&self, url: &Url, body: &[u8], once: bool) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        self.post_within(url, body, once, std::time::Duration::ZERO)
    }

    fn post_within(&self, url: &Url, body: &[u8], once: bool, within: std::time::Duration) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        Tls::tls_only(url).map_err(|f| (f.clone(), crate::rpc::Broke::Said(f.detail())))?;
        carry(url, body, &crate::rpc::within(self.limits, within), how(once))
    }

    fn get(&self, url: &Url) -> R<Vec<u8>> {
        Tls::tls_only(url)?;
        fetch(url, &self.limits)
    }
}

#[derive(Clone, Debug)]
pub struct Auto {
    plain: Http,
    secure: Tls,
}

impl Auto {
    pub fn web_roots() -> R<Auto> {
        Ok(Auto { plain: Http::default(), secure: Tls::web_roots()? })
    }
    pub fn with_pem_roots(pem: &[u8]) -> R<Auto> {
        Ok(Auto { plain: Http::default(), secure: Tls::with_pem_roots(pem)? })
    }
    pub fn for_ca_file(path: Option<&str>) -> R<Auto> {
        match path {
            None => Auto::web_roots(),
            Some(p) => {
                let pem = std::fs::read(p).map_err(|e| Fault::Usage(format!("读不到根证书档 {p}:{e}")))?;
                Auto::with_pem_roots(&pem)
            }
        }
    }
    pub fn for_flags(ca: Option<&str>, proxy: Option<&str>) -> R<Auto> {
        if let Some(p) = proxy {
            set_way(choice_of(p)?);
        }
        Auto::for_ca_file(ca)
    }
    pub fn with_timeout(mut self, t: std::time::Duration) -> Auto {
        self.plain.limits.deadline = t;
        self.secure.limits.deadline = t;
        self
    }
}

impl Transport for Auto {
    fn post(&self, url: &Url, body: &[u8]) -> R<Answer> {
        if url.scheme == "https" { self.secure.post(url, body) } else { self.plain.post(url, body) }
    }

    fn post_once(&self, url: &Url, body: &[u8]) -> R<Answer> {
        if url.scheme == "https" { self.secure.post_once(url, body) } else { self.plain.post_once(url, body) }
    }

    fn post_way(&self, url: &Url, body: &[u8], once: bool) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        if url.scheme == "https" { self.secure.post_way(url, body, once) } else { self.plain.post_way(url, body, once) }
    }

    fn post_within(&self, url: &Url, body: &[u8], once: bool, within: std::time::Duration) -> Result<Answer, (Fault, crate::rpc::Broke)> {
        if url.scheme == "https" { self.secure.post_within(url, body, once, within) } else { self.plain.post_within(url, body, once, within) }
    }

    fn get(&self, url: &Url) -> R<Vec<u8>> {
        if url.scheme == "https" { self.secure.get(url) } else { self.plain.get(url) }
    }
}

pub fn choice_of(text: &str) -> R<eravon_net::Choice> {
    match text.trim() {
        "system" => Ok(eravon_net::Choice::System),
        "none" => Ok(eravon_net::Choice::Off),
        typed => eravon_net::proxy_of(typed).map(eravon_net::Choice::Manual).map_err(|why| {
            let word = match why {
                eravon_net::NotAProxy::Shape => "shape",
                eravon_net::NotAProxy::Credentials => "credentials",
            };
            Fault::Usage(format!("{word} 出路只收 system、none 或 http、https、socks5、socks4 的 协议://[用户[:口令]@]主机:端口,给的那一串 {} 字节读不成", typed.len()))
        }),
    }
}

pub fn choice_word(c: &eravon_net::Choice) -> String {
    match c {
        eravon_net::Choice::System => "system".into(),
        eravon_net::Choice::Off => "none".into(),
        eravon_net::Choice::Manual(p) => p.spelled(),
    }
}

pub fn set_way(c: eravon_net::Choice) {
    eravon_net::set_choice(c);
}

pub fn read_way(choice: fn() -> eravon_net::Choice) {
    eravon_net::read_choice(choice);
}

pub fn close_down() {
    eravon_net::close_down();
}

#[cfg(test)]
mod slow_drip {
    use super::*;

    #[test]
    fn a_refused_way_never_says_what_was_typed() {
        let e = choice_of("socks4://u:hunter2@h:1080").expect_err("credentials refused");
        assert!(e.detail().starts_with("credentials") && !e.detail().contains("hunter2") && !e.detail().contains("u:"), "{}", e.detail());
        let c = choice_of("http://u:hunter2@h:8080").expect("credentials kept");
        assert!(!choice_word(&c).contains("hunter2") && !format!("{c:?}").contains("hunter2"), "{c:?}");
        let e = choice_of("not a proxy").expect_err("shape refused");
        assert!(e.detail().starts_with("shape") && !e.detail().contains("not a proxy"), "{}", e.detail());
    }
    use std::io::Write;
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    fn drip(head: &'static [u8], gap: Duration) -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").expect("要得到本地端口");
        let port = l.local_addr().expect("有端口").port();
        std::thread::spawn(move || {
            if let Ok((mut c, _)) = l.accept() {
                let _ = c.write_all(head);
                loop {
                    std::thread::sleep(gap);
                    if c.write_all(b"a").is_err() {
                        break;
                    }
                }
            }
        });
        port
    }

    const DEADLINE: Duration = Duration::from_millis(600);

    fn ceiling() -> Duration {
        DEADLINE + Duration::from_millis(700)
    }

    #[test]
    fn plain_get_stops_at_the_deadline_and_says_late() {
        let port = drip(b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\n\r\n", Duration::from_millis(100));
        let t = Http::with_timeout(DEADLINE);
        let url = crate::seam::parse_url(&format!("http://127.0.0.1:{port}/doc")).expect("地址合形");
        let start = Instant::now();
        let r = t.get(&url);
        assert!(matches!(&r, Err(Fault::Transport(d)) if d.starts_with("late ")), "慢滴对端该以 late 收场:{r:?}");
        assert!(start.elapsed() < ceiling(), "该在期限附近收场,用了 {:?}", start.elapsed());
    }

}

#[cfg(test)]
mod kept {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    fn find(h: &[u8], n: &[u8]) -> Option<usize> {
        h.windows(n.len()).position(|w| w == n)
    }

    fn peer(per_conn: usize) -> (u16, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
        let l = TcpListener::bind("127.0.0.1:0").expect("要得到本地端口");
        let port = l.local_addr().expect("有端口").port();
        let accepted = Arc::new(AtomicUsize::new(0));
        let heads = Arc::new(Mutex::new(Vec::new()));
        let (count, kept) = (accepted.clone(), heads.clone());
        std::thread::spawn(move || {
            for c in l.incoming() {
                let Ok(mut c) = c else { break };
                count.fetch_add(1, Ordering::SeqCst);
                for i in 0..per_conn {
                    let mut got = Vec::new();
                    let mut buf = [0u8; 4096];
                    let need = loop {
                        if let Some(at) = find(&got, b"\r\n\r\n") {
                            let h = String::from_utf8_lossy(&got[..at]).to_string();
                            kept.lock().unwrap().push(h.clone());
                            let n: usize = h.to_ascii_lowercase().lines().find_map(|l| l.strip_prefix("content-length:").map(str::to_string)).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
                            break at + 4 + n;
                        }
                        match c.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => got.extend_from_slice(&buf[..n]),
                        }
                    };
                    while got.len() < need {
                        match c.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => got.extend_from_slice(&buf[..n]),
                        }
                    }
                    let body = format!("{{\"n\":{i}}}");
                    let _ = c.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes());
                }
            }
        });
        (port, accepted, heads)
    }

    #[test]
    fn calls_to_one_endpoint_share_one_connection_and_carry_the_product_name_alone() {
        let (port, accepted, heads) = peer(10);
        let t = Http::default();
        let url = crate::seam::parse_url(&format!("http://127.0.0.1:{port}/")).expect("地址合形");
        for i in 0..3 {
            assert_eq!(t.post(&url, b"{}").expect("答得出").body, format!("{{\"n\":{i}}}").into_bytes());
        }
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
        let heads = heads.lock().unwrap();
        for h in heads.iter() {
            let ua: Vec<&str> = h.lines().filter(|l| l.to_ascii_lowercase().starts_with("user-agent:")).collect();
            {
                assert_eq!(ua, vec!["User-Agent: ERAVON"], "恰一行、只写产品名:{h}");
            }
        }
    }

    #[test]
    fn a_status_outside_200_comes_back_with_its_body() {
        let l = TcpListener::bind("127.0.0.1:0").expect("要得到本地端口");
        let port = l.local_addr().expect("有端口").port();
        std::thread::spawn(move || {
            if let Ok((mut c, _)) = l.accept() {
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf);
                let body = br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32005,"message":"rate limit"}}"#;
                let _ = c.write_all(format!("HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\n\r\n", body.len()).as_bytes());
                let _ = c.write_all(body);
            }
        });
        let url = crate::seam::parse_url(&format!("http://127.0.0.1:{port}/")).expect("地址合形");
        let a = Http::default().post(&url, b"{}").expect("答回来了");
        assert_eq!(a.status, 429);
        assert!(String::from_utf8_lossy(&a.body).contains("-32005"));
    }

    #[test]
    fn a_kept_connection_the_peer_closed_is_replaced_once() {
        let (port, accepted, _) = peer(1);
        let t = Http::default();
        let url = crate::seam::parse_url(&format!("http://127.0.0.1:{port}/")).expect("地址合形");
        assert!(t.post(&url, b"{}").is_ok());
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(t.post(&url, b"{}").expect("换一条新连接答得出").body, b"{\"n\":0}".to_vec());
        assert_eq!(accepted.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn an_answer_of_exactly_the_cap_is_read_and_one_byte_more_is_overlong() {
        const WHOLE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}";
        let l = TcpListener::bind("127.0.0.1:0").expect("要得到本地端口");
        let port = l.local_addr().expect("有端口").port();
        std::thread::spawn(move || {
            for c in l.incoming() {
                let Ok(mut c) = c else { break };
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf);
                let _ = c.write_all(WHOLE);
            }
        });
        let url = crate::seam::parse_url(&format!("http://127.0.0.1:{port}/")).unwrap();
        let at = Http { limits: eravon_net::Limits { max_answer: WHOLE.len(), ..limits() } };
        assert_eq!(at.post(&url, b"{}").expect("exactly the cap").body, b"{}");
        let under = Http { limits: eravon_net::Limits { max_answer: WHOLE.len() - 1, ..limits() } };
        match under.post(&url, b"{}") {
            Err(Fault::Transport(d)) => {
                assert!(d.starts_with("overlong"), "{d}");
                assert!(d.contains(&format!("OVERSIZE {}://{} ", url.scheme, url.authority())) && !d.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{d}");
            }
            other => panic!("{:?}", other.err()),
        }
        for f in [eravon_net::Fail::Late(std::time::Duration::from_millis(1500)), eravon_net::Fail::Closed] {
            let d = transport_fault(&url, &f).detail();
            assert!(d.starts_with(fail_word(&f)) && !d.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{d}");
        }
        assert!(transport_fault(&url, &eravon_net::Fail::Late(std::time::Duration::from_millis(1500))).detail().contains("TIMEOUT ") , "the deadline by the family's word");
    }

    #[test]
    fn a_failure_says_its_layer_first() {
        let word = |r: R<Answer>| match r {
            Err(Fault::Transport(d)) => d.split(' ').next().unwrap_or("").to_string(),
            other => format!("{:?}", other.err()),
        };
        let free = TcpListener::bind("127.0.0.1:0").expect("借一个端口").local_addr().expect("端口").port();
        let t = Http::default();
        assert_eq!(word(t.post(&crate::seam::parse_url(&format!("http://127.0.0.1:{free}/")).unwrap(), b"{}")), "connect");
        assert_eq!(fail_word(&eravon_net::Fail::Name("node.example".into())), "name", "名字解析不成(解析本身由 eravon-net 的可注入解析器测,不走系统 DNS)");
        let l = TcpListener::bind("127.0.0.1:0").expect("要得到本地端口");
        let port = l.local_addr().expect("有端口").port();
        std::thread::spawn(move || {
            for (i, c) in l.incoming().enumerate() {
                let Ok(mut c) = c else { break };
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf);
                let _ = if i == 0 { c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nabc") } else { c.write_all(&[b'x'; 200]) };
            }
        });
        let url = crate::seam::parse_url(&format!("http://127.0.0.1:{port}/")).unwrap();
        assert_eq!(word(t.post(&url, b"{}")), "stream", "答一半对端就关了");
        let small = Http { limits: eravon_net::Limits { max_answer: 64, ..limits() } };
        assert_eq!(word(small.post(&url, b"{}")), "overlong");
        let plain = TcpListener::bind("127.0.0.1:0").expect("要得到本地端口");
        let port = plain.local_addr().expect("有端口").port();
        std::thread::spawn(move || {
            for c in plain.incoming() {
                let Ok(mut c) = c else { break };
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf);
                let _ = c.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
            }
        });
        let tls = Auto::web_roots().expect("出厂根载得起");
        assert_eq!(word(tls.post(&crate::seam::parse_url(&format!("https://localhost:{port}/")).unwrap(), b"{}")), "handshake");
    }
}

#[cfg(test)]
mod bare_query {
    use super::*;

    #[test]
    fn a_bare_query_reaches_no_transport_sentence() {
        const KEY: &str = "k3yBareQuery";
        let url = crate::seam::parse_url(&format!("https://node.example?apikey={KEY}")).expect("reads");
        assert!(!url.host.contains(KEY) && !url.authority().contains(KEY), "{url:?}");
        let f = transport_fault(&url, &eravon_net::Fail::Name(url.host.clone()));
        assert!(f.name() == "E_CHAIN_TRANSPORT" && !f.detail().contains(KEY), "{}", f.detail());
    }
}
