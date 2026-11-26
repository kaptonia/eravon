#![allow(dead_code)]

use super::common::{self, Home, PIN};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

pub fn node() -> u16 {
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

pub fn pipe(a: TcpStream, b: TcpStream) {
    let (mut a2, mut b2) = (a.try_clone().expect("clone"), b.try_clone().expect("clone"));
    let (mut a, mut b) = (a, b);
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut a2, &mut b2);
        let _ = b2.shutdown(std::net::Shutdown::Write);
    });
    let _ = std::io::copy(&mut b, &mut a);
    let _ = a.shutdown(std::net::Shutdown::Write);
}

pub fn stub(to: u16) -> (u16, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s2 = seen.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let seen = s2.clone();
            std::thread::spawn(move || {
                let mut s = s;
                let mut head = Vec::new();
                let mut one = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    if s.read(&mut one).unwrap_or(0) == 0 {
                        return;
                    }
                    head.push(one[0]);
                }
                let _ = s.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n");
                seen.lock().expect("seen").push(String::from_utf8_lossy(&head).split_whitespace().nth(1).unwrap_or("").to_string());
                if let Ok(up) = TcpStream::connect(("127.0.0.1", to)) {
                    pipe(s, up);
                }
            });
        }
    });
    (port, seen)
}

pub const NOW: &str = "1758412800";

pub fn clean(c: &mut std::process::Command, env: &[(&str, String)]) {
    for v in ["https_proxy", "HTTPS_PROXY", "http_proxy", "HTTP_PROXY", "all_proxy", "ALL_PROXY", "no_proxy", "NO_PROXY"] {
        c.env_remove(v);
    }
    for (k, v) in env {
        c.env(k, v);
    }
}

pub fn balance(h: &Home, endpoint: Option<&str>, env: &[(&str, String)]) -> common::Ran {
    let mut args = vec!["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--chain", "31337", "--address", &h.address, "--passcode", PIN];
    if let Some(e) = endpoint {
        args.extend(["--endpoint", e]);
    }
    h.run(&args, |c| clean(c, env))
}

pub fn way(h: &Home, way: &str) {
    let r = h.run(&["buyer-settings-proxy", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--way", way], |c| clean(c, &[]));
    assert_eq!(r.code, 0, "the way is written: {} {}", r.text, r.err);
}
