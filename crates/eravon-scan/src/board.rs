use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed as k1;
use eravon_link as k3;
use eravon_literals::constants as k7;
use eravon_standard as a1;

use crate::fault::{Fault, R};
use crate::log::Log;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Post {
    pub tx: String,
    pub block: u64,
    pub log_index: u64,
    pub key: [u8; 32],
    pub hash: [u8; 32],
    pub poster: String,
    pub document: Vec<u8>,
}

fn is_board<T: Transport>(t: &T, table: &Table, addr: &str) -> R<bool> {
    let (code, _) = a2::read::code(t, table, addr, a2::seam::TAG_LATEST).map_err(Fault::from)?;
    Ok(!code.is_empty() && a1::order::hex_of(&k1::hash::keccak256(&code)).eq_ignore_ascii_case(k7::BOARD_CODE_HASH))
}


fn post_of(l: &Log) -> Option<Post> {
    if l.topics.len() != 4 || l.topics[0] != a1::sol::POSTED.topic0() {
        return None;
    }
    let vals = a1::event::decode(&l.topics, &l.data).ok()?;
    let document = vals.at(3).ok()?.bytes().ok()?.to_vec();
    let hash = l.topics[3];
    if k1::hash::sha256(&document) != hash {
        return None;
    }
    Some(Post {
        tx: l.tx.to_ascii_lowercase(),
        block: l.block,
        log_index: l.log_index,
        key: l.topics[2],
        hash,
        poster: l.topic_address(1).ok()?,
        document,
    })
}

pub fn by_tx<T: Transport>(t: &T, table: &Table, tx: &str, hash: &[u8; 32]) -> R<Vec<Post>> {
    crate::mark::mark(crate::mark::FEATURE);
    let r = a2::read::read_agreed(t, table, a2::seam::M_GET_RECEIPT, vec![eravon_literals::json::Out::S(tx.to_string())])
        .map_err(Fault::from)?;
    if matches!(r.value, eravon_chain::json_in::In::Null) {
        return Ok(Vec::new());
    }
    let logs = match r.value.get("logs") {
        Some(eravon_chain::json_in::In::Arr(a)) => a.clone(),
        _ => return Err(Fault::Form(format!("交易 {tx} 的回执上没有 logs 一列"))),
    };
    let mut out = Vec::new();
    let mut boards = Boards::default();
    for v in &logs {
        let l = Log::read(v)?;
        let Some(p) = post_of(&l) else { continue };
        if p.hash == *hash && boards.carries(t, table, &l.address)? {
            out.push(p);
        }
    }
    Ok(out)
}

#[derive(Default)]
struct Boards(Vec<(String, bool)>);

impl Boards {
    fn carries<T: Transport>(&mut self, t: &T, table: &Table, addr: &str) -> R<bool> {
        let addr = addr.to_ascii_lowercase();
        if let Some((_, ok)) = self.0.iter().find(|(a, _)| *a == addr) {
            return Ok(*ok);
        }
        let ok = is_board(t, table, &addr)?;
        self.0.push((addr, ok));
        Ok(ok)
    }
}

pub fn by_key<T: Transport>(t: &T, table: &Table, key: &[u8; 32], from: u64, to: a2::seam::Upto) -> R<Vec<Post>> {
    crate::mark::mark(crate::mark::FEATURE);
    let topics = [vec![a1::order::hex_of(&a1::sol::POSTED.topic0())], vec![], vec![a1::order::hex_of(key)]];
    let (rows, _) = a2::read::logs_any(t, table, None, &topics, from, to).map_err(Fault::from)?;
    let mut out: Vec<Post> = Vec::new();
    let mut boards = Boards::default();
    for v in &rows {
        let l = Log::read(v)?;
        let Some(p) = post_of(&l) else { continue };
        if boards.carries(t, table, &l.address)? {
            out.push(p);
        }
    }
    out.sort_by_key(|p| (p.block, p.log_index));
    Ok(out)
}

pub fn by_poster<T: Transport>(t: &T, table: &Table, posters: &[String], from: u64, to: a2::seam::Upto) -> R<Vec<Post>> {
    crate::mark::mark(crate::mark::FEATURE);
    if posters.is_empty() {
        return Ok(Vec::new());
    }
    let who = posters.iter().map(|k| crate::log::address_topic(k)).collect::<R<Vec<String>>>()?;
    let mut out: Vec<Post> = Vec::new();
    let mut boards = Boards::default();
    {
        let topics = [vec![a1::order::hex_of(&a1::sol::POSTED.topic0())], who.clone()];
        let (rows, _) = a2::read::logs_any(t, table, None, &topics, from, to).map_err(Fault::from)?;
        for v in &rows {
            let l = Log::read(v)?;
            let Some(p) = post_of(&l) else { continue };
            if boards.carries(t, table, &l.address)? {
                out.push(p);
            }
        }
    }
    out.sort_by_key(|p| (p.block, p.log_index));
    Ok(out)
}

pub fn listing_by_key<T: Transport>(
    t: &T,
    table: &Table,
    terms_hash: &[u8; 32],
    seller: &str,
    from: u64,
) -> R<Option<(Post, k3::listing::Listing)>> {
    let posts = by_key(t, table, terms_hash, from, a2::seam::Upto::Latest)?;
    Ok(latest_listing(&posts, terms_hash, seller))
}

pub fn latest_listing(posts: &[Post], terms_hash: &[u8; 32], seller: &str) -> Option<(Post, k3::listing::Listing)> {
    let want = a1::order::hex_of(terms_hash);
    posts.iter().rev().find_map(|p| {
        let s = k3::listing::listing(&p.document).ok()?;
        (s.terms_hash.eq_ignore_ascii_case(&want) && p.key == *terms_hash && s.order.seller_key.eq_ignore_ascii_case(seller))
            .then(|| (p.clone(), s))
    })
}

pub fn settles_by_case<T: Transport>(
    t: &T,
    table: &Table,
    case_id: &[u8; 32],
    chain_id: u64,
    instance: &str,
    seller: &str,
    from: u64,
) -> R<Vec<(Post, k3::settle::Settle)>> {
    let posts = by_key(t, table, case_id, from, a2::seam::Upto::Latest)?;
    Ok(posts
        .into_iter()
        .filter_map(|p| {
            let st = k3::settle::settle(&p.document).ok()?;
            k3::settle::bind_settle_on(chain_id, instance, seller, &st).ok()?;
            Some((p, st))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log_of(topics: Vec<[u8; 32]>, document: &[u8]) -> Log {
        Log {
            address: k7::BOARD.to_ascii_lowercase(),
            topics,
            data: a1::abi::encode(&[a1::abi::Val::Bytes(document.to_vec())]),
            block: 7,
            log_index: 2,
            tx: "0xAB".into(),
        }
    }

    fn poster() -> [u8; 32] {
        let mut p = [0u8; 32];
        p[12..].copy_from_slice(&[0x11; 20]);
        p
    }

    #[test]
    fn a_post_is_taken_only_where_its_bytes_hash_to_its_topic() {
        let doc = b"{\"v\":\"eravon.listing/1\"}";
        let key = [0x22; 32];
        let good = log_of(vec![a1::sol::POSTED.topic0(), poster(), key, k1::hash::sha256(doc)], doc);
        let p = post_of(&good).expect("合式的一条取得到");
        assert_eq!((p.key, p.document.as_slice(), p.tx.as_str()), (key, &doc[..], "0xab"));
        let bad = log_of(vec![a1::sol::POSTED.topic0(), poster(), key, [0x33; 32]], doc);
        assert!(post_of(&bad).is_none(), "哈希位与字节不合的不取");
        let short = log_of(vec![a1::sol::POSTED.topic0(), poster(), key], doc);
        assert!(post_of(&short).is_none(), "少一枚 topic 的不取");
        let other = log_of(vec![a1::sol::INSTANCE_EVENTS[0].topic0(), poster(), key, k1::hash::sha256(doc)], doc);
        assert!(post_of(&other).is_none(), "别的事件不取");
    }

    #[test]
    fn the_latest_listing_is_the_one_the_content_names() {
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base/eravon-conformance/link-corpus/listings/l001_ok_full.json");
        let doc = std::fs::read(corpus).expect("语料清单读得出");
        let s = k3::listing::listing(&doc).expect("语料清单合格");
        let key: [u8; 32] = a1::order::b32_of("termsHash", &s.terms_hash).expect("hex32");
        let mk = |block: u64, document: &[u8], key: [u8; 32]| Post {
            tx: format!("0x{block:064x}"),
            block,
            log_index: 0,
            key,
            hash: k1::hash::sha256(document),
            poster: "0x00".into(),
            document: document.to_vec(),
        };
        let posts = vec![mk(1, &doc, key), mk(2, b"not a listing", key), mk(3, &doc, key)];
        let (p, _) = latest_listing(&posts, &key, &s.order.seller_key).expect("认得出");
        assert_eq!(p.block, 3, "取最后一份");
        assert!(latest_listing(&posts, &key, "0x0000000000000000000000000000000000000001").is_none(), "别的卖家不认");
        let elsewhere = vec![mk(4, &doc, [0x44; 32])];
        assert!(latest_listing(&elsewhere, &key, &s.order.seller_key).is_none(), "贴在别的 key 下的不认");
    }
}
