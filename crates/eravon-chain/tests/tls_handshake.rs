use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

use eravon_chain::seam;
use eravon_chain::transport::{Tls, Transport};

const BODY: &[u8] = b"{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":\"0x7a69\"}";

const ROOT_PEM: &[u8] = include_bytes!("fixtures/tls-root-cert.pem");
const ROOT_CERT: &[u8] = include_bytes!("fixtures/tls-root-cert.der");
const ROOT_KEY: &[u8] = include_bytes!("fixtures/tls-root-key.pk8.der");
const OTHER_CERT: &[u8] = include_bytes!("fixtures/tls-other-cert.der");
const OTHER_KEY: &[u8] = include_bytes!("fixtures/tls-other-key.pk8.der");

fn pair(cert: &[u8], key: &[u8]) -> (rustls_pki_types::CertificateDer<'static>, rustls_pki_types::PrivateKeyDer<'static>) {
    let key = rustls_pki_types::PrivateKeyDer::try_from(key.to_vec()).expect("私钥读得回来");
    (rustls_pki_types::CertificateDer::from(cert.to_vec()), key)
}

#[test]
fn https_goes_through_a_real_handshake_and_brings_the_body_back() {
    let (cert_der, key_der) = pair(ROOT_CERT, ROOT_KEY);

    let listener = TcpListener::bind("127.0.0.1:0").expect("借得到一个空端口");
    let port = listener.local_addr().expect("拿得到端口号").port();

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server_cfg = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("协议版本表立得起来")
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("证书与私钥配得上");
    let server_cfg = Arc::new(server_cfg);

    let server = std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().expect("接得到一条连接");
        let mut conn = rustls::ServerConnection::new(server_cfg).expect("服务器这一侧起得来");
        let mut tls = rustls::Stream::new(&mut conn, &mut sock);
        let mut buf = Vec::new();
        let mut one = [0u8; 1];
        while !buf.ends_with(b"\r\n\r\n") {
            match tls.read(&mut one) {
                Ok(0) => break,
                Ok(_) => buf.push(one[0]),
                Err(_) => break,
            }
        }
        let head = String::from_utf8_lossy(&buf).to_string();
        let len: usize = head
            .lines()
            .find_map(|l| {
                let l = l.to_ascii_lowercase();
                l.strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0))
            })
            .unwrap_or(0);
        let mut req = vec![0u8; len];
        let _ = tls.read_exact(&mut req);
        let answer = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            BODY.len()
        );
        let _ = tls.write_all(answer.as_bytes());
        let _ = tls.write_all(BODY);
        let _ = tls.flush();
        let _ = conn.send_close_notify();
        let _ = rustls::Stream::new(&mut conn, &mut sock).flush();
        String::from_utf8_lossy(&req).to_string()
    });

    let t = Tls::with_pem_roots(ROOT_PEM).expect("根收得下");
    let url = seam::parse_url(&format!("https://localhost:{port}")).expect("地址拆得开");
    let got = t.post(&url, b"{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"eth_chainId\",\"params\":[]}");

    let sent = server.join().expect("服务器那一条线程收得了工");
    assert!(sent.contains("eth_chainId"), "服务器该收到产品发的那一份问:{sent}");
    let got = got.expect("这一趟该走通");
    assert_eq!((got.status, got.body.as_slice()), (200, BODY), "取回来的体该逐字节等于服务器发的那一份");
}

#[test]
fn an_unknown_root_is_refused_and_never_falls_back_to_plaintext() {
    let (cert_der, key_der) = pair(OTHER_CERT, OTHER_KEY);

    let listener = TcpListener::bind("127.0.0.1:0").expect("借得到一个空端口");
    let port = listener.local_addr().expect("拿得到端口号").port();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server_cfg = Arc::new(
        rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("协议版本表立得起来")
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .expect("证书与私钥配得上"),
    );
    std::thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            if let Ok(mut conn) = rustls::ServerConnection::new(server_cfg) {
                let mut tls = rustls::Stream::new(&mut conn, &mut sock);
                let mut one = [0u8; 1];
                let _ = tls.read(&mut one);
            }
        }
    });

    let t = Tls::with_pem_roots(ROOT_PEM).expect("根收得下");
    let url = seam::parse_url(&format!("https://localhost:{port}")).expect("地址拆得开");
    let f = match t.post(&url, b"{}") {
        Ok(b) => panic!("根不认得那一张,这一趟该拒,却答了 {}", b.status),
        Err(f) => f,
    };
    assert_eq!(f.name(), "E_CHAIN_TRANSPORT", "该是运输类的拒:{f:?}");
    assert!(f.detail().starts_with("certificate "), "证书那一层具名:{f:?}");
}

#[test]
fn neither_transport_quietly_takes_the_other_ones_scheme() {
    let http = eravon_chain::transport::Http::default();
    let https_url = seam::parse_url("https://example.invalid").expect("地址拆得开");
    let f = http.post(&https_url, b"{}").err().expect("明文那一种该拒 https");
    assert_eq!(f.name(), "E_CHAIN_USAGE");

    let tls = Tls::web_roots().expect("出厂根立得起来");
    let http_url = seam::parse_url("http://example.invalid").expect("地址拆得开");
    let f = tls.post(&http_url, b"{}").err().expect("TLS 那一种该拒 http");
    assert_eq!(f.name(), "E_CHAIN_USAGE");
}

#[test]
fn a_pem_with_no_certificate_in_it_is_refused() {
    let f = match Tls::with_pem_roots(b"not a pem at all\n") {
        Ok(_) => panic!("一张证书也没有的 PEM 该收不下"),
        Err(f) => f,
    };
    assert_eq!(f.name(), "E_CHAIN_USAGE");
}
