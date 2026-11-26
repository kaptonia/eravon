use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed as k1;
use eravon_literals::json::In;

use crate::fault::{Fault, R};
use crate::kit;
use crate::log::Log;

pub struct OnChain<'a, T: Transport> {
    pub t: &'a T,
    pub table: &'a Table,
}

struct Lists {
    raw_txs: Vec<Vec<u8>>,
}

const SEEN_BLOCKS: usize = 64;

struct Seen<V> {
    rows: std::sync::Mutex<std::collections::VecDeque<(String, V)>>,
}

impl<V: Clone> Seen<V> {
    const fn new() -> Seen<V> {
        Seen { rows: std::sync::Mutex::new(std::collections::VecDeque::new()) }
    }

    fn get(&self, key: &str) -> Option<V> {
        self.rows.lock().ok()?.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    fn put(&self, key: String, v: V) {
        if let Ok(mut g) = self.rows.lock() {
            if g.iter().any(|(k, _)| *k == key) {
                return;
            }
            if g.len() >= SEEN_BLOCKS {
                g.pop_front();
            }
            g.push_back((key, v));
        }
    }
}

static LISTS: Seen<std::sync::Arc<Lists>> = Seen::new();

mod kept {
    use super::{In, Lists};
    use std::path::{Path, PathBuf};

    pub const KEPT_DIR: &str = "checked-blocks";

    pub const KEPT_BLOCKS: usize = 256;

    static AT: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

    pub fn set(dir: Option<PathBuf>) {
        if let Ok(mut g) = AT.lock() {
            *g = dir;
        }
    }

    pub fn dir() -> Option<PathBuf> {
        AT.lock().ok()?.clone()
    }

    pub fn file(hash: &str) -> Option<String> {
        let h = hash.strip_prefix("0x")?;
        (h.len() == 64 && h.bytes().all(|c| c.is_ascii_hexdigit())).then(|| format!("{}.json", h.to_ascii_lowercase()))
    }

    pub fn load(dir: &Path, hash: &str) -> Option<Lists> {
        let v = eravon_literals::json::read(&std::fs::read(dir.join(file(hash)?)).ok()?).ok()?;
        let raw_txs = v
            .get("rawTxs")?
            .as_arr()?
            .iter()
            .map(|x| eravon_chain::seam::bytes_of("kept", x.as_str()?).ok())
            .collect::<Option<Vec<Vec<u8>>>>()?;
        Some(Lists { raw_txs })
    }

    pub fn save(dir: &Path, hash: &str, l: &Lists) {
        let Some(name) = file(hash) else { return };
        let raw = l.raw_txs.iter().map(|r| In::Str(eravon_galeed::hash::hex_encode(r))).collect();
        let v = In::Obj(vec![("rawTxs".into(), In::Arr(raw))]);
        let tmp = format!(".{name}.{}.part", std::process::id());
        if eravon_os::put_at(dir, &name, &tmp, &eravon_literals::json::write_in(&v)).is_ok() {
            trim(dir);
        }
    }

    fn trim(dir: &Path) {
        let Ok(list) = std::fs::read_dir(dir) else { return };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = list
            .flatten()
            .filter(|e| e.file_name().to_str().and_then(|n| Some(file(&format!("0x{}", n.strip_suffix(".json")?))? == n)) == Some(true))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        if files.len() <= KEPT_BLOCKS {
            return;
        }
        files.sort();
        for (_, p) in &files[..files.len() - KEPT_BLOCKS] {
            let _ = std::fs::remove_file(p);
        }
    }
}

pub use kept::{KEPT_BLOCKS, KEPT_DIR};

pub fn keep_blocks_on_machine() {
    kept::set(eravon_os::machine::here().ok().map(|(m, _)| m).filter(|m| m.is_dir()).map(|m| m.join(KEPT_DIR)));
}

pub fn keep_blocks_in(dir: Option<std::path::PathBuf>) {
    kept::set(dir);
}

fn root_of(b: &In, member: &str) -> Result<String, String> {
    b.get(member).and_then(In::as_str).map(str::to_ascii_lowercase).ok_or_else(|| format!("核时区块头缺 {member}"))
}

fn txs_hold(hashes: &[String], tx_root: &str, raws: &[Vec<u8>]) -> Result<(), String> {
    if raws.len() != hashes.len() {
        return Err(format!("{} 笔,块内交易 {} 笔", raws.len(), hashes.len()));
    }
    if let Some((h, _)) = hashes.iter().zip(raws).find(|(h, raw)| k1::hash::hex_encode(&k1::hash::keccak256(raw)) != h.as_str()) {
        return Err(format!("不哈希到 {h}"));
    }
    let got = k1::hash::hex_encode(&kit::trie(raws, 0).0);
    if got != tx_root {
        return Err(format!("重建的交易树根 {got} 不合区块头的 transactionsRoot {tx_root}"));
    }
    Ok(())
}


fn key(table: &Table, hash: &str) -> String {
    format!("{}\n{}", table.list.join("\n"), hash.to_ascii_lowercase())
}

pub fn block_txs_at<T: Transport>(t: &T, table: &Table, n: u64) -> Result<(Vec<String>, Vec<Vec<u8>>), a2::fault::Fault> {
    let (b, _) = a2::read::block_by_number(t, table, n)?;
    let Some(hash) = b.get("hash").and_then(In::as_str).map(str::to_ascii_lowercase) else {
        return Err(a2::fault::Fault::Endpoint(format!("块 {n} 读不出它的哈希(还没出块)")));
    };
    let bad = |why: String| a2::fault::Fault::Endpoint(format!("块 {hash}:{why}"));
    kit::header_rlp(&b).map_err(|e| bad(e.detail()))?;
    let hashes: Vec<String> = b
        .get("transactions")
        .and_then(In::as_arr)
        .ok_or_else(|| bad("块缺 transactions".into()))?
        .iter()
        .map(|x| x.as_str().map(str::to_ascii_lowercase).ok_or_else(|| bad("transactions 不是哈希一列".into())))
        .collect::<Result<_, _>>()?;
    let tx_root = root_of(&b, "transactionsRoot").map_err(bad)?;
    let k = key(table, &hash);
    if let Some(l) = LISTS.get(&k).or_else(|| kept::dir().as_deref().and_then(|d| kept::load(d, &hash)).map(std::sync::Arc::new)).filter(|l| txs_hold(&hashes, &tx_root, &l.raw_txs).is_ok()) {
        LISTS.put(k, l.clone());
        return Ok((hashes, l.raw_txs.clone()));
    }
    let raws = a2::read::block_transactions_whole(t, table, &hash, &hashes, kit::tx_raw, |raws| txs_hold(&hashes, &tx_root, raws))?;
    let l = std::sync::Arc::new(Lists { raw_txs: raws });
    if let Some(d) = kept::dir() {
        kept::save(&d, &hash, &l);
    }
    LISTS.put(k, l.clone());
    Ok((hashes, l.raw_txs.clone()))
}

impl<T: Transport> crate::ledger::Evidence for OnChain<'_, T> {
    fn tx(&self, chain_id: u64, tx: &str) -> R<Option<k1::Value>> {
        if let Some(v) = hash32(tx).and_then(|h| a2::read::proven_transaction(self.t, self.table, chain_id, &h)) {
            return Ok(Some(galeed_of(&v)));
        }
        let (v, _) = a2::read::transaction(self.t, self.table, tx).map_err(Fault::from)?;
        Ok((!matches!(v, In::Null)).then(|| galeed_of(&v)))
    }

    fn status(&self, tx: &str) -> R<Option<bool>> {
        let r = a2::read::read_agreed(self.t, self.table, a2::seam::M_GET_RECEIPT, vec![eravon_literals::json::Out::S(tx.to_string())]).map_err(Fault::from)?;
        if matches!(r.value, In::Null) {
            return Ok(None);
        }
        match r.value.get("status") {
            None | Some(In::Null) => Ok(Some(false)),
            Some(In::Str(q)) => a2::seam::read_quantity(q).map(|n| Some(n == 1)).map_err(Fault::from),
            Some(_) => Err(Fault::Form(format!("交易 {tx} 回执的 status 不是数量"))),
        }
    }

    fn time(&self, chain_id: u64, block_hash: &str, number: u64) -> R<u64> {
        if let Some(t) = hash32(block_hash).and_then(|h| a2::read::proved_time(chain_id, &h, number)) {
            return Ok(t);
        }
        crate::scan::block_stamp_at(self.t, self.table, number, Some(block_hash)).map_err(Fault::from)
    }

    fn code_at(&self, chain_id: u64, block_hash: &str, who: &str, number: u64) -> Option<bool> {
        let at = hash32(block_hash);
        let addr = eravon_galeed_scan::txform::bytes(who).and_then(|b| <[u8; 20]>::try_from(b).ok());
        if let (Some(h), Some(a)) = (at, addr) {
            if let Some(coded) = a2::read::proved_code(chain_id, &h, &a, number) {
                return Some(coded);
            }
        }
        a2::read::code(self.t, self.table, who, &a2::seam::quantity(number)).ok().map(|(code, _)| !code.is_empty())
    }

    fn code_hash_at(&self, addr: &str, block: u64) -> R<String> {
        let (code, _) = a2::read::code(self.t, self.table, addr, &a2::seam::quantity(block)).map_err(Fault::from)?;
        Ok(k1::hash::hex_encode(&k1::hash::keccak256(&code)))
    }

    fn ahead(&self, chain_id: u64, wants: &[(&Log, &str)]) {
        let wanted: Vec<a2::ahead::Want> = wants
            .iter()
            .filter_map(|(l, h)| {
                let who = l.topic_address(1).ok().and_then(|a| eravon_galeed_scan::txform::bytes(&a)).and_then(|b| <[u8; 20]>::try_from(b).ok())?;
                Some(a2::ahead::Want { tx: hash32(&l.tx)?, block: hash32(h)?, number: l.block, who, hash: [0u8; 32] })
            })
            .collect();
        a2::read::ahead_anchors(self.t, self.table, chain_id, &wanted);
    }

    fn ahead_codes(&self, wants: &[(String, u64)]) {
        a2::read::ahead_codes_by(self.t, self.table, wants);
    }
}

fn hash32(h: &str) -> Option<[u8; 32]> {
    eravon_galeed_scan::txform::bytes(h).and_then(|b| <[u8; 32]>::try_from(b).ok())
}

fn galeed_of(v: &In) -> k1::Value {
    match v {
        In::Null => k1::Value::Null,
        In::Bool(b) => k1::Value::Bool(*b),
        In::Int(n) => k1::Value::Int(u64::try_from(*n).unwrap_or(0)),
        In::Str(s) => k1::Value::Str(s.clone()),
        In::Arr(a) => k1::Value::Arr(a.iter().map(galeed_of).collect()),
        In::Obj(m) => k1::Value::Obj(m.iter().map(|(k, x)| (k.clone(), galeed_of(x))).collect()),
    }
}
