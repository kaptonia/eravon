mod common;

use common::{Home, PIN};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

fn node() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut s = s;
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                let body = loop {
                    match s.read(&mut buf) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => raw.extend_from_slice(&buf[..n]),
                    }
                    let text = String::from_utf8_lossy(&raw).to_string();
                    if let Some(i) = text.find("\r\n\r\n") {
                        let len: usize = text[..i].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|n| n.trim().parse().ok())).unwrap_or(0);
                        if raw.len() >= i + 4 + len {
                            break text[i + 4..].to_string();
                        }
                    }
                };
                let id = body.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                let result = if body.contains("eth_chainId") {
                    "\"0x7a69\""
                } else if body.contains("eth_blockNumber") {
                    "\"0x20\""
                } else if body.contains("eth_getBalance") {
                    "\"0x10\""
                } else {
                    "null"
                };
                let answer = format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{result}}}");
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}", answer.len()).as_bytes());
            });
        }
    });
    port
}

fn pipe(a: TcpStream, b: TcpStream) {
    let (mut a2, mut b2) = (a.try_clone().expect("clone"), b.try_clone().expect("clone"));
    let (mut a, mut b) = (a, b);
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut a2, &mut b2);
        let _ = b2.shutdown(std::net::Shutdown::Write);
    });
    let _ = std::io::copy(&mut b, &mut a);
    let _ = a.shutdown(std::net::Shutdown::Write);
}

fn stub(socks: bool, to: u16) -> (u16, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s2 = seen.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let seen = s2.clone();
            std::thread::spawn(move || {
                let mut s = s;
                let place = if socks {
                    let mut g = [0u8; 3];
                    if s.read_exact(&mut g).is_err() || s.write_all(&[5, 0]).is_err() {
                        return;
                    }
                    let mut h = [0u8; 5];
                    if s.read_exact(&mut h).is_err() {
                        return;
                    }
                    let mut rest = vec![0u8; h[4] as usize + 2];
                    if s.read_exact(&mut rest).is_err() {
                        return;
                    }
                    let n = rest.len() - 2;
                    let p = u16::from_be_bytes([rest[n], rest[n + 1]]);
                    let _ = s.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
                    format!("{}:{p}", String::from_utf8_lossy(&rest[..n]))
                } else {
                    let mut head = Vec::new();
                    let mut one = [0u8; 1];
                    while !head.ends_with(b"\r\n\r\n") {
                        if s.read(&mut one).unwrap_or(0) == 0 {
                            return;
                        }
                        head.push(one[0]);
                    }
                    let _ = s.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n");
                    String::from_utf8_lossy(&head).split_whitespace().nth(1).unwrap_or("").to_string()
                };
                seen.lock().expect("seen").push(place);
                if let Ok(up) = TcpStream::connect(("127.0.0.1", to)) {
                    pipe(s, up);
                }
            });
        }
    });
    (port, seen)
}

struct Ran {
    code: i32,
    out: Vec<u8>,
    err: String,
}

fn read(h: &Home, node: u16, proxy: Option<&str>, env: &[(&str, String)]) -> Ran {
    let endpoint = format!("http://node.invalid:{node}");
    let mut args = vec!["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--endpoint", &endpoint, "--chain", "31337", "--address", &h.address, "--passcode", PIN];
    if let Some(p) = proxy {
        args.extend(["--proxy", p]);
    }
    let r = h.run(&args, |c| {
        for v in ["https_proxy", "HTTPS_PROXY", "http_proxy", "HTTP_PROXY", "all_proxy", "ALL_PROXY", "no_proxy", "NO_PROXY"] {
            c.env_remove(v);
        }
        for (k, v) in env {
            c.env(k, v);
        }
    });
    Ran { code: r.code, out: r.text.into_bytes(), err: r.err }
}

#[test]
fn a_proxy_address_is_gone_through() {
    let h = Home::new("proxy-through");
    let n = node();
    let (socks, socks_seen) = stub(true, n);
    let (connect, connect_seen) = stub(false, n);
    let r = read(&h, n, Some(&format!("socks5://127.0.0.1:{socks}")), &[]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(socks_seen.lock().expect("seen").iter().all(|p| *p == format!("node.invalid:{n}")) && !socks_seen.lock().expect("seen").is_empty());
    for typed in [format!("http://127.0.0.1:{connect}"), format!("http://u:p@127.0.0.1:{connect}")] {
        let r = read(&h, n, Some(&typed), &[]);
        assert_eq!(r.code, 0, "{}", r.err);
        assert!(!r.err.contains(":p@") && !String::from_utf8_lossy(&r.out).contains(":p@"), "the password is never echoed: {}", r.err);
    }
    assert!(connect_seen.lock().expect("seen").iter().all(|p| *p == format!("node.invalid:{n}")) && !connect_seen.lock().expect("seen").is_empty());
}

#[test]
fn none_goes_straight() {
    let h = Home::new("proxy-none");
    let n = node();
    let r = read(&h, n, Some("none"), &[]);
    assert_eq!(r.code, 4, "{}", r.err);
}

#[test]
fn system_and_no_flag_follow_the_system() {
    let h = Home::new("proxy-system");
    let n = node();
    let (connect, seen) = stub(false, n);
    let env = [("http_proxy", format!("http://127.0.0.1:{connect}"))];
    for proxy in [Some("system"), None] {
        let r = read(&h, n, proxy, &env);
        assert_eq!(r.code, 0, "{proxy:?}: {}", r.err);
    }
    assert!(!seen.lock().expect("seen").is_empty(), "the terminal's proxy was gone through");
}

#[test]
fn any_other_proxy_is_misuse() {
    let h = Home::new("proxy-misuse");
    let n = node();
    for bad in ["proxy.example:8080", "http://:p@127.0.0.1:1", "ftp://127.0.0.1:1", "socks4://id:pw@127.0.0.1:1", "http://127.0.0.1", "http://127.0.0.1:0", "", "SYSTEM2"] {
        let r = read(&h, n, Some(bad), &[]);
        assert_eq!((r.code, r.out.is_empty()), (2, true), "{bad:?}: {}", r.err);
        assert!(r.err.lines().next().unwrap_or("").contains("--proxy"), "{bad:?}: {}", r.err);
        assert!(bad.is_empty() || !r.err.contains(bad), "{bad:?} is never echoed: {}", r.err);
    }
    let node = common::node();
    let mut args = h.send_args(&node);
    args.extend(["--proxy", "proxy.example:8080"]);
    let r = h.run(&args, |_| {});
    assert_eq!((r.code, r.text.is_empty()), (2, true), "the direct send takes the same flag: {}", r.err);
    assert_eq!(node.broadcasts(), 0);
}
