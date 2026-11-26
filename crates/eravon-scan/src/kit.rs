use eravon_galeed as k1;
use eravon_literals::json::In;

use crate::fault::{Fault, R};

use eravon_chain::read::{HeaderForm as Form, BESIDE_HEADER, HEADER_ALWAYS, HEADER_MEMBERS as HEADER};

fn data(what: &str, s: &str) -> R<Vec<u8>> {
    let d = s.strip_prefix("0x").ok_or_else(|| Fault::Form(format!("{what} 不是 0x 十六进制:{s}")))?;
    k1::hexform::decode_digits(d.as_bytes()).ok_or_else(|| Fault::Form(format!("{what} 不是偶数位十六进制:{s}")))
}

fn quantity(what: &str, s: &str) -> R<Vec<u8>> {
    let d = s.strip_prefix("0x").ok_or_else(|| Fault::Form(format!("{what} 不是 0x 数量:{s}")))?;
    let d = d.trim_start_matches('0');
    let even = if d.len() % 2 == 1 { format!("0{d}") } else { d.to_string() };
    k1::hexform::decode_digits(even.as_bytes()).ok_or_else(|| Fault::Form(format!("{what} 不是十六进制数量:{s}")))
}

fn member<'a>(v: &'a In, key: &str) -> Option<&'a str> {
    v.get(key).and_then(In::as_str)
}

fn list(items: &[Vec<u8>]) -> Vec<u8> {
    let parsed: Vec<k1::rlp::Rlp<'_>> = items.iter().map(|b| k1::rlp::decode(b).expect("每一项都是写好的 RLP")).collect();
    k1::rlp::encode(&k1::rlp::Rlp::List(parsed))
}

fn bytes(b: &[u8]) -> Vec<u8> {
    k1::rlp::encode(&k1::rlp::Rlp::Bytes(b))
}

pub fn header_rlp(block: &In) -> R<Vec<u8>> {
    if let In::Obj(m) = block {
        if let Some((k, _)) = m.iter().find(|(k, _)| !HEADER.iter().any(|(h, _)| h == k) && !BESIDE_HEADER.contains(&k.as_str())) {
            return Err(Fault::Form(format!("区块带本端不认的成员 {k}:区块头成员表要跟这一分叉加上它")));
        }
    }
    let mut items = Vec::new();
    for (i, (key, form)) in HEADER.iter().enumerate() {
        let Some(s) = member(block, key) else {
            if i < HEADER_ALWAYS {
                return Err(Fault::Form(format!("区块头缺 {key}")));
            }
            if HEADER[i..].iter().any(|(k, _)| member(block, k).is_some()) {
                return Err(Fault::Form(format!("区块头缺 {key} 而有其后之项")));
            }
            break;
        };
        let b = match form {
            Form::Quantity => quantity(key, s)?,
            Form::Data => data(key, s)?,
        };
        items.push(bytes(&b));
    }
    let raw = list(&items);
    let want = data("hash", member(block, "hash").ok_or_else(|| Fault::Form("区块缺 hash".into()))?)?;
    if k1::hash::keccak256(&raw).as_slice() != want.as_slice() {
        return Err(Fault::Form(format!("区块头照各成员重写之后不合块哈希 {}", member(block, "hash").unwrap_or_default())));
    }
    Ok(raw)
}

pub fn receipt_rlp(r: &In) -> R<Vec<u8>> {
    let need = |k: &str| member(r, k).ok_or_else(|| Fault::Form(format!("回执缺 {k}")));
    let ty = quantity("type", member(r, "type").unwrap_or("0x0"))?;
    let mut logs = Vec::new();
    for l in r.get("logs").and_then(In::as_arr).ok_or_else(|| Fault::Form("回执缺 logs".into()))? {
        let addr = data("address", member(l, "address").ok_or_else(|| Fault::Form("日志缺 address".into()))?)?;
        let mut topics = Vec::new();
        for t in l.get("topics").and_then(In::as_arr).ok_or_else(|| Fault::Form("日志缺 topics".into()))? {
            topics.push(bytes(&data("topic", t.as_str().ok_or_else(|| Fault::Form("topic 不是串".into()))?)?));
        }
        let d = data("data", member(l, "data").ok_or_else(|| Fault::Form("日志缺 data".into()))?)?;
        logs.push(list(&[bytes(&addr), list(&topics), bytes(&d)]));
    }
    let body = list(&[
        bytes(&quantity("status", need("status")?)?),
        bytes(&quantity("cumulativeGasUsed", need("cumulativeGasUsed")?)?),
        bytes(&data("logsBloom", need("logsBloom")?)?),
        list(&logs),
    ]);
    Ok(match ty.as_slice() {
        [] => body,
        [t] => [vec![*t], body].concat(),
        _ => return Err(Fault::Form("回执 type 超出一个字节".into())),
    })
}

pub fn tx_raw(tx: &In) -> Option<Vec<u8>> {
    let q = |k: &str| member(tx, k).and_then(|s| quantity(k, s).ok()).map(|b| bytes(&b));
    let d = |k: &str| member(tx, k).and_then(|s| data(k, s).ok()).map(|b| bytes(&b));
    let to = || match tx.get("to") {
        None | Some(In::Null) => Some(bytes(&[])),
        Some(In::Str(s)) => data("to", s).ok().map(|b| bytes(&b)),
        _ => None,
    };
    let access = |tx: &In| -> Option<Vec<u8>> {
        let mut out = Vec::new();
        for e in tx.get("accessList")?.as_arr()? {
            let mut keys = Vec::new();
            for k in e.get("storageKeys")?.as_arr()? {
                keys.push(bytes(&data("storageKey", k.as_str()?).ok()?));
            }
            out.push(list(&[bytes(&data("address", member(e, "address")?).ok()?), list(&keys)]));
        }
        Some(list(&out))
    };
    let parity = || q("yParity").or_else(|| q("v"));
    let ty = quantity("type", member(tx, "type").unwrap_or("0x0")).ok()?;
    let body = |fields: Vec<Option<Vec<u8>>>| fields.into_iter().collect::<Option<Vec<_>>>().map(|f| list(&f));
    let input = || d("input");
    match ty.as_slice() {
        [] => body(vec![q("nonce"), q("gasPrice"), q("gas"), to(), q("value"), input(), q("v"), q("r"), q("s")]),
        [0x01] => body(vec![q("chainId"), q("nonce"), q("gasPrice"), q("gas"), to(), q("value"), input(), access(tx), parity(), q("r"), q("s")]).map(|b| [vec![0x01], b].concat()),
        [0x02] => body(vec![q("chainId"), q("nonce"), q("maxPriorityFeePerGas"), q("maxFeePerGas"), q("gas"), to(), q("value"), input(), access(tx), parity(), q("r"), q("s")])
            .map(|b| [vec![0x02], b].concat()),
        [0x03] => {
            let mut hashes = Vec::new();
            for h in tx.get("blobVersionedHashes")?.as_arr()? {
                hashes.push(bytes(&data("blobVersionedHash", h.as_str()?).ok()?));
            }
            body(vec![
                q("chainId"),
                q("nonce"),
                q("maxPriorityFeePerGas"),
                q("maxFeePerGas"),
                q("gas"),
                to(),
                q("value"),
                input(),
                access(tx),
                q("maxFeePerBlobGas"),
                Some(list(&hashes)),
                parity(),
                q("r"),
                q("s"),
            ])
            .map(|b| [vec![0x03], b].concat())
        }
        [0x04] => {
            let mut auths = Vec::new();
            for a in tx.get("authorizationList")?.as_arr()? {
                let aq = |k: &str| member(a, k).and_then(|s| quantity(k, s).ok()).map(|b| bytes(&b));
                let parity = aq("yParity").or_else(|| aq("v"));
                auths.push(body(vec![aq("chainId"), member(a, "address").and_then(|s| data("address", s).ok()).map(|b| bytes(&b)), aq("nonce"), parity, aq("r"), aq("s")])?);
            }
            body(vec![
                q("chainId"),
                q("nonce"),
                q("maxPriorityFeePerGas"),
                q("maxFeePerGas"),
                q("gas"),
                to(),
                q("value"),
                input(),
                access(tx),
                Some(list(&auths)),
                parity(),
                q("r"),
                q("s"),
            ])
            .map(|b| [vec![0x04], b].concat())
        }
        _ => None,
    }
}

pub fn input_of(raw: &[u8]) -> Option<Vec<u8>> {
    let (body, at, len) = match *raw.first()? {
        f if f >= 0xc0 => (raw, 5, 9),
        0x01 => (&raw[1..], 6, 11),
        0x02 => (&raw[1..], 7, 12),
        0x03 => (&raw[1..], 7, 14),
        0x04 => (&raw[1..], 7, 13),
        _ => return None,
    };
    let item = k1::rlp::decode(body)?;
    let f = item.list()?;
    (f.len() == len).then(|| f[at].bytes().map(<[u8]>::to_vec)).flatten()
}

pub fn index_key(i: u64) -> Vec<u8> {
    bytes(&k1::rlp::uint_bytes(i))
}

fn nibbles(b: &[u8]) -> Vec<u8> {
    b.iter().flat_map(|x| [x >> 4, x & 0xf]).collect()
}

fn hex_prefix(path: &[u8], leaf: bool) -> Vec<u8> {
    let flag = if leaf { 2 } else { 0 } + (path.len() % 2) as u8;
    let mut out = Vec::with_capacity(path.len() / 2 + 1);
    let rest = if path.len() % 2 == 1 {
        out.push((flag << 4) | path[0]);
        &path[1..]
    } else {
        out.push(flag << 4);
        path
    };
    out.extend(rest.chunks(2).map(|c| (c[0] << 4) | c[1]));
    out
}

fn child(enc: &[u8]) -> Vec<u8> {
    if enc.len() < 32 { enc.to_vec() } else { bytes(&k1::hash::keccak256(enc)) }
}

fn node(pairs: &[(Vec<u8>, &[u8])], depth: usize, target: Option<&[u8]>, proof: &mut Vec<Vec<u8>>) -> Vec<u8> {
    let enc = if pairs.len() == 1 {
        let (k, v) = &pairs[0];
        list(&[bytes(&hex_prefix(&k[depth..], true)), bytes(v)])
    } else {
        let first = &pairs[0].0;
        let common = (depth..)
            .take_while(|&i| i < first.len() && pairs.iter().all(|(k, _)| k.len() > i && k[i] == first[i]))
            .count();
        if common > 0 {
            let sub = node(pairs, depth + common, target, proof);
            list(&[bytes(&hex_prefix(&first[depth..depth + common], false)), child(&sub)])
        } else {
            let mut items = Vec::with_capacity(17);
            for n in 0..16u8 {
                let group: Vec<(Vec<u8>, &[u8])> = pairs.iter().filter(|(k, _)| k.len() > depth && k[depth] == n).cloned().collect();
                if group.is_empty() {
                    items.push(bytes(&[]));
                } else {
                    let t = target.filter(|t| t.len() > depth && t[depth] == n);
                    items.push(child(&node(&group, depth + 1, t, proof)));
                }
            }
            let here = pairs.iter().find(|(k, _)| k.len() == depth).map(|(_, v)| *v).unwrap_or_default();
            items.push(bytes(here));
            list(&items)
        }
    };
    if target.is_some() {
        proof.push(enc.clone());
    }
    enc
}

pub fn trie(values: &[Vec<u8>], at: u64) -> ([u8; 32], Vec<Vec<u8>>) {
    if values.is_empty() {
        return (k1::hash::keccak256(&bytes(&[])), Vec::new());
    }
    let pairs: Vec<(Vec<u8>, &[u8])> = values.iter().enumerate().map(|(i, v)| (nibbles(&index_key(i as u64)), v.as_slice())).collect();
    let target = nibbles(&index_key(at));
    let mut proof = Vec::new();
    let root = node(&pairs, 0, Some(&target), &mut proof);
    proof.reverse();
    (k1::hash::keccak256(&root), proof)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_transaction_object_is_written_back_to_its_raw_bytes() {
        let t = eravon_standard::tx::Tx {
            chain_id: 11155111,
            nonce: 7,
            max_priority_fee: { let mut b = [0u8; 32]; b[31] = 2; b },
            max_fee: { let mut b = [0u8; 32]; b[30] = 1; b },
            gas_limit: 21_000,
            to: Some([0x11; 20]),
            value: { let mut b = [0u8; 32]; b[31] = 9; b },
            data: vec![0xab, 0xcd],
        };
        let mut sig = vec![0x22; 64];
        sig.push(28);
        let raw = t.raw(&sig).expect("签得成");
        let hex = |b: &[u8]| format!("0x{}", b.iter().map(|x| format!("{x:02x}")).collect::<String>());
        let obj = |ty: &str, extra: &str| {
            eravon_literals::json::read(
                format!(
                    r#"{{"type":"{ty}","chainId":"0xaa36a7","nonce":"0x7","maxPriorityFeePerGas":"0x2","maxFeePerGas":"0x100","gasPrice":"0x100","gas":"0x5208","to":"{}","value":"0x9","input":"0xabcd","accessList":[],"yParity":"0x1","v":"0x1","r":"{}","s":"{}"{extra}}}"#,
                    hex(&[0x11; 20]),
                    hex(&[0x22; 32]),
                    hex(&[0x22; 32])
                )
                .as_bytes(),
            )
            .expect("JSON")
        };
        assert_eq!(tx_raw(&obj("0x2", "")).expect("写得回"), raw, "动态费交易照原文写回");
        let legacy = tx_raw(&obj("0x0", "")).expect("旧式写得回");
        assert!(legacy[0] >= 0xc0, "旧式交易是一张 RLP 表,无类型前缀");
        assert!(tx_raw(&obj("0x7e", "")).is_none(), "不认的型不写,留给端点照原文给");
    }

    use super::*;

    #[test]
    fn every_index_of_the_ordered_trie_proves_under_its_root() {
        for n in [1usize, 2, 3, 16, 17, 129, 300] {
            let values: Vec<Vec<u8>> = (0..n).map(|i| format!("value-{i}-{}", "x".repeat(i % 40)).into_bytes()).collect();
            for at in [0, n / 2, n - 1] {
                let (root, proof) = trie(&values, at as u64);
                let got = k1::kit::verify_proof(root, &index_key(at as u64), &proof).expect("证明读得过");
                assert_eq!(got.as_deref(), Some(values[at].as_slice()), "n={n} at={at}");
            }
            let (root, proof) = trie(&values, 0);
            assert_eq!(k1::kit::verify_proof(root, &index_key(n as u64 + 5), &proof).ok().flatten(), None, "树外的键证不出值");
        }
    }

    #[test]
    fn the_empty_trie_has_its_known_root() {
        let (root, _) = trie(&[], 0);
        assert_eq!(k1::hash::hex_encode(&root), "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
    }

    fn block(extra: &[(&str, &str)]) -> In {
        let mut m: Vec<(String, In)> = vec![
            ("parentHash", "0x".to_string() + &"11".repeat(32)),
            ("sha3Uncles", "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347".into()),
            ("miner", "0x".to_string() + &"00".repeat(20)),
            ("stateRoot", "0x".to_string() + &"22".repeat(32)),
            ("transactionsRoot", "0x".to_string() + &"33".repeat(32)),
            ("receiptsRoot", "0x".to_string() + &"44".repeat(32)),
            ("logsBloom", "0x".to_string() + &"00".repeat(256)),
            ("difficulty", "0x0".into()),
            ("number", "0x2a".into()),
            ("gasLimit", "0x1c9c380".into()),
            ("gasUsed", "0x5208".into()),
            ("timestamp", "0x65000000".into()),
            ("extraData", "0x".into()),
            ("mixHash", "0x".to_string() + &"55".repeat(32)),
            ("nonce", "0x0000000000000000".into()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), In::Str(v)))
        .collect();
        m.extend(extra.iter().map(|(k, v)| (k.to_string(), In::Str(v.to_string()))));
        m.push(("hash".into(), In::Str("0x00".into())));
        In::Obj(m)
    }

    fn with_hash(b: In, h: &str) -> In {
        match b {
            In::Obj(mut m) => {
                m.retain(|(k, _)| k != "hash");
                m.push(("hash".into(), In::Str(h.into())));
                In::Obj(m)
            }
            x => x,
        }
    }

    #[test]
    fn a_header_is_taken_only_where_it_hashes_to_its_block() {
        let b = block(&[("baseFeePerGas", "0x7")]);
        let err = header_rlp(&b).expect_err("哈希对不上");
        assert!(err.detail().contains("不合块哈希"), "{}", err.detail());
        let mut items = Vec::new();
        for (key, form) in HEADER.iter().take(16) {
            let s = member(&b, key).unwrap();
            items.push(bytes(&match form {
                Form::Quantity => quantity(key, s).unwrap(),
                Form::Data => data(key, s).unwrap(),
            }));
        }
        let raw = list(&items);
        let good = with_hash(b, &k1::hash::hex_encode(&k1::hash::keccak256(&raw)));
        assert_eq!(header_rlp(&good).unwrap(), raw);
        let holed = block(&[("withdrawalsRoot", "0x00")]);
        assert!(header_rlp(&holed).expect_err("缺 baseFee 而有其后").detail().contains("其后"));
    }

    #[test]
    fn a_real_sepolia_block_rewrites_to_its_hash() {
        let fixture = eravon_literals::json::read(include_bytes!("../tests/fixtures/sepolia-block.json")).expect("夹具是 JSON");
        for side in ["tenderly", "ethpandaops"] {
            let b = fixture.get(side).expect("夹具有这一端的块");
            let raw = header_rlp(b).unwrap_or_else(|e| panic!("{side}:{}", e.detail()));
            assert_eq!(k1::hash::hex_encode(&k1::hash::keccak256(&raw)), member(b, "hash").unwrap(), "{side}");
            let unknown = match b.clone() {
                In::Obj(mut m) => {
                    m.push(("someNewRoot".into(), In::Str("0x00".into())));
                    In::Obj(m)
                }
                x => x,
            };
            assert!(header_rlp(&unknown).expect_err("不认的成员").detail().contains("someNewRoot"));
        }
    }

    #[test]
    fn a_receipt_is_written_as_its_trie_holds_it() {
        let r = |ty: &str, status: &str| {
            In::Obj(vec![
                ("type".into(), In::Str(ty.into())),
                ("status".into(), In::Str(status.into())),
                ("cumulativeGasUsed".into(), In::Str("0x5208".into())),
                ("logsBloom".into(), In::Str("0x".to_string() + &"00".repeat(256))),
                ("logs".into(), In::Arr(vec![In::Obj(vec![
                    ("address".into(), In::Str("0x".to_string() + &"ab".repeat(20))),
                    ("topics".into(), In::Arr(vec![In::Str("0x".to_string() + &"01".repeat(32))])),
                    ("data".into(), In::Str("0x".into())),
                ])])),
            ])
        };
        let typed = receipt_rlp(&r("0x2", "0x1")).unwrap();
        assert_eq!(typed[0], 2);
        let body = k1::rlp::decode(&typed[1..]).unwrap();
        assert_eq!(body.list().unwrap()[0].bytes(), Some(&[1u8][..]));
        let legacy = receipt_rlp(&r("0x0", "0x0")).unwrap();
        let body = k1::rlp::decode(&legacy).expect("旧式回执无类型前缀");
        assert_eq!(body.list().unwrap()[0].bytes(), Some(&[][..]));
    }
}
