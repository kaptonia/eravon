mod common;

use common::{Home, PIN};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const SECRET: &str = "S3CRETk3y";

fn read(h: &Home, values: &[&str]) -> common::Ran {
    let mut a = vec!["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", "1", "--chain", "31337", "--address", &h.address, "--passcode", PIN];
    for v in values {
        a.extend(["--endpoint", v]);
    }
    h.run(&a, |_| {})
}

fn refused(r: &common::Ran, word: &str, n: usize, len: usize) {
    assert_eq!((r.code, r.get("downToken")), (1, "E_CHAIN_USAGE".to_string()), "{}", r.text);
    let d = r.get("detail");
    assert!(d.contains(&format!("{word} 第 {n} 个端点")), "{word}: {d}");
    assert!(d.contains(&format!("({len} 字节)")), "the length: {d}");
    assert!(!r.text.contains(SECRET) && !r.err.contains(SECRET), "the key is echoed: {} {}", r.text, r.err);
}

#[test]
fn an_empty_value_is_named_by_place_and_length() {
    let h = Home::new("ep-empty");
    refused(&read(&h, &[""]), "scheme", 1, 0);
}

#[test]
fn a_scheme_out_of_the_set_says_the_chain_id_and_the_length_after() {
    let h = Home::new("ep-scheme");
    let v = format!("wss://node.example/{SECRET}");
    refused(&read(&h, &[&v]), "scheme", 1, v.len());
}

#[test]
fn a_port_past_16_bits_says_the_chain_id_and_the_length_after() {
    let h = Home::new("ep-port");
    let v = format!("https://node.example:65536/{SECRET}");
    refused(&read(&h, &[&v]), "port", 1, v.len());
}

#[test]
fn an_empty_port_says_the_chain_id_and_the_length_after() {
    let h = Home::new("ep-port-empty");
    let v = format!("https://node.example:/{SECRET}");
    refused(&read(&h, &[&v]), "port", 1, v.len());
}

#[test]
fn a_user_and_password_in_the_address_are_never_echoed() {
    let h = Home::new("ep-user");
    let v = format!("https://user:{SECRET}@node.example/");
    refused(&read(&h, &[&v]), "shape", 1, v.len());
}

#[test]
fn white_space_inside_the_address_is_named_by_place_and_length() {
    let h = Home::new("ep-space");
    let v = format!("https://node.example/a {SECRET}");
    refused(&read(&h, &[&v]), "shape", 1, v.len());
}

#[test]
fn a_line_end_inside_the_address_keeps_the_first_line_whole() {
    let h = Home::new("ep-newline");
    let v = format!("https://node.example/\n{SECRET}");
    refused(&read(&h, &[&v]), "shape", 1, v.len());
}

#[test]
fn a_tab_inside_the_address_is_named_by_place_and_length() {
    let h = Home::new("ep-tab");
    let v = format!("https://node.example/\t{SECRET}");
    refused(&read(&h, &[&v]), "shape", 1, v.len());
}

#[test]
fn white_space_at_either_end_reads() {
    let h = Home::new("ep-ends");
    let port = std::net::TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
    let r = read(&h, &[&format!(" \thttp://127.0.0.1:{port}\n ")]);
    assert_eq!(r.code, 4, "{}", r.text);
}

#[test]
fn scan_names_the_place_and_asks_no_node_before_judging_every_value() {
    let h = Home::new("ep-order");
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let good = format!("http://{}", l.local_addr().expect("addr"));
    let asked = Arc::new(AtomicUsize::new(0));
    let a = asked.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            a.fetch_add(1, Ordering::SeqCst);
            drop(s);
        }
    });
    let bad = format!("wss://node.example/{SECRET}");
    refused(&read(&h, &[&good, &bad]), "scheme", 2, bad.len());
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(asked.load(Ordering::SeqCst), 0, "no node asked");
}

#[test]
fn a_value_that_reads_is_not_misuse() {
    let h = Home::new("ep-reads");
    let port = std::net::TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
    let r = read(&h, &[&format!("http://127.0.0.1:{port}")]);
    assert_eq!(r.code, 4, "{}", r.text);
}

#[test]
fn a_key_in_a_node_address_is_never_said() {
    let h = Home::new("ep-key");
    let dead = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = dead.local_addr().expect("addr").port();
    drop(dead);
    for value in [format!("http://127.0.0.1:{port}/v3/{SECRET}"), format!("http://127.0.0.1:{port}/rpc?apikey={SECRET}")] {
        let r = read(&h, &[&value]);
        assert_eq!(r.code, 4, "{value}: {}", r.text);
        assert_eq!(r.get("downToken"), "E_CHAIN_TRANSPORT", "unreached: {}", r.text);
        assert!(r.text.contains(&format!("127.0.0.1:{port}")), "the node is named: {}", r.text);
        assert!(!r.text.contains(SECRET) && !r.err.contains(SECRET), "the key is said: {} {}", r.text, r.err);
    }
    let value = format!("http://door:{SECRET}@127.0.0.1:{port}/rpc");
    let r = read(&h, &[&value]);
    assert!(r.code != 0, "{value}: {}", r.text);
    assert!(!r.text.contains(SECRET) && !r.err.contains(SECRET), "the key is said: {} {}", r.text, r.err);
}
