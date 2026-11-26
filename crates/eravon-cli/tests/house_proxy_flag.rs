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

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
enum Ask {
    Never,
    Credentials,
}

fn stub(socks: bool, to: u16, ask: Ask) -> (u16, Arc<Mutex<Vec<String>>>) {
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
                    let head = String::from_utf8_lossy(&head).to_string();
                    if ask == Ask::Credentials && !head.to_ascii_lowercase().contains("proxy-authorization:") {
                        let _ = s.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\n\r\n");
                        return;
                    }
                    let _ = s.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n");
                    head.split_whitespace().nth(1).unwrap_or("").to_string()
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
fn a_proxy_asking_for_credentials_is_said_so() {
    let h = Home::new("proxy-asks");
    let n = node();
    let (connect, seen) = stub(false, n, Ask::Credentials);
    let r = read(&h, n, Some(&format!("http://127.0.0.1:{connect}")), &[]);
    assert_eq!(r.code, 4, "{}", r.err);
    assert!(r.err.contains("--proxy"), "the way to give them: {}", r.err);
    let r = read(&h, n, Some(&format!("http://u:pw7731@127.0.0.1:{connect}")), &[]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(!r.err.contains("pw7731") && !String::from_utf8_lossy(&r.out).contains("pw7731"), "the password is never echoed");
    assert!(!seen.lock().expect("seen").is_empty());
}
