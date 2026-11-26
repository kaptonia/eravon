use eravon_chain::json_in::In;

use crate::fault::{Fault, R};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    pub address: String,
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
    pub block: u64,
    pub log_index: u64,
    pub tx: String,
}

fn hex_bytes(what: &str, s: &str) -> R<Vec<u8>> {
    let b = s.strip_prefix("0x").unwrap_or(s);
    if b.len() % 2 != 0 || !b.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Form(format!("{what} 要是 0x 加偶数位十六进制:{s:?}")));
    }
    let mut out = Vec::with_capacity(b.len() / 2);
    for p in b.as_bytes().chunks(2) {
        let v = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
        out.push((v(p[0]) << 4) | v(p[1]));
    }
    Ok(out)
}

fn quantity(what: &str, s: &str) -> R<u64> {
    let b = s.strip_prefix("0x").ok_or_else(|| Fault::Form(format!("{what} 要以 0x 起头")))?;
    u64::from_str_radix(b, 16).map_err(|_| Fault::Form(format!("{what} 不是十六进制的数")))
}

fn field(v: &In, key: &str) -> R<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .ok_or_else(|| Fault::Form(format!("日志上缺 {key},或它不是文本")))
}

impl Log {
    pub fn read(v: &In) -> R<Log> {
        let mut topics = Vec::new();
        match v.get("topics") {
            Some(In::Arr(a)) => {
                for t in a {
                    let s = t.as_str().ok_or_else(|| Fault::Form("topic 要是文本".into()))?;
                    let b = hex_bytes("topic", s)?;
                    if b.len() != 32 {
                        return Err(Fault::Form(format!("topic 要恰三十二字节:{s}")));
                    }
                    topics.push(b.try_into().expect("三十二字节"));
                }
            }
            other => return Err(Fault::Form(format!("日志上的 topics 要是一列:{other:?}"))),
        }
        Ok(Log {
            address: field(v, "address")?.to_ascii_lowercase(),
            topics,
            data: hex_bytes("data", &field(v, "data")?)?,
            block: quantity("blockNumber", &field(v, "blockNumber")?)?,
            log_index: quantity("logIndex", &field(v, "logIndex")?)?,
            tx: field(v, "transactionHash")?,
        })
    }

    pub fn topic_hex(&self, i: usize) -> R<String> {
        let t = self
            .topics
            .get(i)
            .ok_or_else(|| Fault::Form(format!("这条日志没有第 {i} 枚 topic")))?;
        Ok(eravon_standard::order::hex_of(t))
    }

    pub fn topic_address(&self, i: usize) -> R<String> {
        let t = self
            .topics
            .get(i)
            .ok_or_else(|| Fault::Form(format!("这条日志没有第 {i} 枚 topic")))?;
        if t[..12].iter().any(|x| *x != 0) {
            return Err(Fault::Form("topic 上的地址字高十二字节该是零".into()));
        }
        Ok(eravon_standard::order::hex_of(&t[12..]))
    }
}

pub fn address_topic(addr: &str) -> R<String> {
    let a = addr.strip_prefix("0x").unwrap_or(addr);
    if a.len() != 40 || !a.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Form(format!("地址要 0x 加四十位十六进制:{addr:?}")));
    }
    Ok(format!("0x{}{}", "0".repeat(24), a.to_ascii_lowercase()))
}
