//! §9.6 evidence kit judgment: a self-contained proof of one anchor. Two block headers, Merkle proofs
//! of the transaction and the receipt, and account proofs of the sender at two state boundaries. The
//! verifier binds both ends to a block hash it trusts (the two block hashes are answered; trusting them
//! is the caller's call), recovers the sender from the transaction, tests the one form of §9.1 and
//! settles the three verdicts of §9.3. Any missing piece, headers that are not parent and child, or
//! proofs at different indices show nothing. Cryptography only through [`crate::cryptox`].
//!
//! Transport form (read with `json::read_open`): `chainId` int, `hash` hex32, `emitter` hex20, `header` and
//! `parentHeader` (`0x` hex RLP headers; the latter omitted when the anchor block is block 0), `txIndex` int,
//! `txProof` and `receiptProof` (`0x` hex node lists), `accountProofAfter` and `accountProofBefore` (sender
//! account proofs against the stateRoot of the anchor block and of the block before; a missing one means that
//! boundary was not asked), `entry` (`0x` hex), the bytes the kit names as the log's, present where the kit
//! carries them, and `emitterProof` (the emitting account's proof against the anchor block's stateRoot), present
//! where the kit is meant for a recipient whose basis admits emitters by class (§9.6): the judgment then answers
//! the emitter's code hash, against which that recipient decides §9.1's admitted-emitter test under its own
//! basis. The object is open: a member beyond these (a capture side's `form`) is not read.

#![cfg_attr(feature = "ablate-k1-proof-kit", allow(dead_code, unused_imports))]

use crate::audit::{VERDICT_COUNTED, VERDICT_UNPROVEN, VERDICT_VOID};
use crate::cryptox;
use crate::hexform;
use crate::json::Value;
use crate::rlp::{self, Rlp};

/// Preimage of `keccak256("Anchored(address,bytes32,bytes)")`: topic 0 of the one form of §9.1, which the
/// reference registry's `anchorCarried` and `anchorCarriedMany` and a venue's ruling call emit (galeed.venue/1 §1).
/// A log under any other topic 0 is no anchor.
pub const ANCHORED_CARRIED_SIGNATURE: &str = "Anchored(address,bytes32,bytes)";

/// A judgment: an anchor shown with its verdict, or nothing shown (with a reason that never feeds a decision).
pub enum Judgment {
    Shows(Shown),
    Nothing(&'static str),
}

pub struct Shown {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx: [u8; 32],
    pub sender: [u8; 20],
    pub hash: [u8; 32],
    pub verdict: &'static str,
    pub block_hash: [u8; 32],
    pub parent_hash: [u8; 32],
    /// The entry bytes the kit establishes, where the kit names them; absent otherwise.
    pub entry: Option<Vec<u8>>,
    /// The `keccak256` of the code the emitting account held at the end of the anchor's block, where the kit carries
    /// the emitter's account proof (§9.6, for a recipient that admits emitters by class); absent otherwise.
    pub emitter_code_hash: Option<[u8; 32]>,
}

struct Header {
    parent_hash: [u8; 32],
    state_root: [u8; 32],
    tx_root: [u8; 32],
    receipts_root: [u8; 32],
    number: u64,
    timestamp: u64,
    hash: [u8; 32],
}

fn h32(b: &[u8]) -> Option<[u8; 32]> {
    b.try_into().ok()
}
fn h20(b: &[u8]) -> Option<[u8; 20]> {
    b.try_into().ok()
}

fn header(raw: &[u8]) -> Option<Header> {
    let l = rlp::decode(raw)?;
    let f = l.list()?;
    if f.len() < 15 {
        return None;
    }
    Some(Header {
        parent_hash: h32(f[0].bytes()?)?,
        state_root: h32(f[3].bytes()?)?,
        tx_root: h32(f[4].bytes()?)?,
        receipts_root: h32(f[5].bytes()?)?,
        number: f[8].uint()?,
        timestamp: f[11].uint()?,
        hash: cryptox::keccak256(raw),
    })
}

fn nibbles(key: &[u8]) -> Vec<u8> {
    key.iter().flat_map(|b| [b >> 4, b & 0xf]).collect()
}

fn hex_prefix(b: &[u8]) -> Option<(Vec<u8>, bool)> {
    let first = *b.first()?;
    let flag = first >> 4;
    let leaf = flag >= 2;
    let odd = flag & 1 == 1;
    if flag > 3 || (!odd && first & 0xf != 0) {
        return None;
    }
    let mut path: Vec<u8> = Vec::new();
    if odd {
        path.push(first & 0xf);
    }
    path.extend(nibbles(&b[1..]));
    Some((path, leaf))
}

enum Next<'a> {
    Hash([u8; 32]),
    Inline(&'a [u8]),
}

/// Fetch a node by reference among the proof nodes: a hash reference needs exactly one node with that
/// keccak; an inline reference is the node bytes.
fn node_of<'a>(next: &Next<'a>, proof: &'a [Vec<u8>]) -> Option<&'a [u8]> {
    match next {
        Next::Inline(b) => Some(b),
        Next::Hash(h) => proof.iter().find(|n| cryptox::keccak256(n) == *h).map(Vec::as_slice),
    }
}

/// A child reference has three states: empty slot (`Ok(None)`), hash or inline (`Ok(Some)`), malformed
/// (`Err`). Empty and malformed are different things.
fn child_ref<'a>(item: &Rlp<'a>, raw: &'a [u8]) -> Result<Option<Next<'a>>, ()> {
    match item {
        Rlp::Bytes(b) if b.is_empty() => Ok(None),
        Rlp::Bytes(b) => h32(b).map(|h| Some(Next::Hash(h))).ok_or(()),
        Rlp::List(_) => {
            let enc = rlp::encode(item);
            let start = raw.windows(enc.len()).position(|w| w == enc.as_slice()).ok_or(())?;
            Ok(Some(Next::Inline(&raw[start..start + enc.len()])))
        }
    }
}

/// Proof judgment: `Ok(Some(value))` the key is in the trie, `Ok(None)` an exclusion proof holds,
/// `Err(())` the proof fails or is malformed.
pub fn verify_proof(root: [u8; 32], key: &[u8], proof: &[Vec<u8>]) -> Result<Option<Vec<u8>>, ()> {
    let path = nibbles(key);
    let mut pos = 0usize;
    let mut next = Next::Hash(root);
    for _ in 0..=path.len() + 1 {
        let raw = node_of(&next, proof).ok_or(())?;
        let node = rlp::decode(raw).ok_or(())?;
        let items = node.list().ok_or(())?;
        match items.len() {
            17 => {
                if pos == path.len() {
                    let v = items[16].bytes().ok_or(())?;
                    return Ok(if v.is_empty() { None } else { Some(v.to_vec()) });
                }
                match child_ref(&items[usize::from(path[pos])], raw)? {
                    None => return Ok(None),
                    Some(n) => {
                        next = n;
                        pos += 1;
                    }
                }
            }
            2 => {
                let (p, leaf) = hex_prefix(items[0].bytes().ok_or(())?).ok_or(())?;
                let rest = &path[pos..];
                if leaf {
                    return Ok(if rest == p.as_slice() { Some(items[1].bytes().ok_or(())?.to_vec()) } else { None });
                }
                if !rest.starts_with(&p) {
                    return Ok(None);
                }
                pos += p.len();
                next = child_ref(&items[1], raw)?.ok_or(())?;
            }
            _ => return Err(()),
        }
    }
    Err(())
}

struct Tx {
    chain_id: u64,
    to: Option<[u8; 20]>,
    data: Vec<u8>,
    sender: [u8; 20],
    hash: [u8; 32],
}

fn scalar32(b: &[u8]) -> Option<[u8; 32]> {
    if b.len() > 32 || (b.len() > 1 && b[0] == 0) {
        return None;
    }
    let mut out = [0u8; 32];
    out[32 - b.len()..].copy_from_slice(b);
    Some(out)
}

/// Recover the sender from the signing digest: r and s in range, s low (EIP-2), recovery succeeds.
fn sender_of(digest: &[u8; 32], r: &[u8; 32], s: &[u8; 32], parity: u8) -> Option<[u8; 20]> {
    if !cryptox::in_range(r) || !cryptox::in_range(s) || !cryptox::is_low_s(s) || parity > 1 {
        return None;
    }
    cryptox::recover_address(digest, r, s, parity)
}

/// The §9.1 sender and the bytes it signed: legacy (EIP-155; without a chain id it is not an anchor)
/// and the four envelopes 0x01 to 0x04.
fn tx(raw: &[u8]) -> Option<Tx> {
    let first = *raw.first()?;
    let hash = cryptox::keccak256(raw);
    if first >= 0xc0 {
        let l = rlp::decode(raw)?;
        let f = l.list()?;
        if f.len() != 9 {
            return None;
        }
        let v = f[6].uint()?;
        if v < 35 {
            return None; // The signature names no chain: valid on every chain, so its destination cannot be read.
        }
        let chain_id = (v - 35) / 2;
        let parity = ((v - 35) % 2) as u8;
        let cid = rlp::uint_bytes(chain_id);
        let unsigned = Rlp::List(vec![
            f[0].clone(), f[1].clone(), f[2].clone(), f[3].clone(), f[4].clone(), f[5].clone(),
            Rlp::Bytes(&cid), Rlp::Bytes(&[]), Rlp::Bytes(&[]),
        ]);
        let digest = cryptox::keccak256(&rlp::encode(&unsigned));
        let sender = sender_of(&digest, &scalar32(f[7].bytes()?)?, &scalar32(f[8].bytes()?)?, parity)?;
        return Some(Tx { chain_id, to: to_of(&f[3])?, data: f[5].bytes()?.to_vec(), sender, hash });
    }
    let (to_idx, len) = match first {
        0x01 => (4, 11),
        0x02 => (5, 12),
        0x03 => (5, 14),
        0x04 => (5, 13),
        _ => return None,
    };
    let l = rlp::decode(&raw[1..])?;
    let f = l.list()?;
    if f.len() != len {
        return None;
    }
    let unsigned = Rlp::List(f[..len - 3].to_vec());
    let mut pre = vec![first];
    pre.extend_from_slice(&rlp::encode(&unsigned));
    let digest = cryptox::keccak256(&pre);
    let parity = f[len - 3].uint()?;
    if parity > 1 {
        return None;
    }
    let sender = sender_of(&digest, &scalar32(f[len - 2].bytes()?)?, &scalar32(f[len - 1].bytes()?)?, parity as u8)?;
    Some(Tx { chain_id: f[0].uint()?, to: to_of(&f[to_idx])?, data: f[to_idx + 2].bytes()?.to_vec(), sender, hash })
}

/// `to`: empty means contract creation (no recipient), otherwise exactly 20 bytes.
fn to_of(item: &Rlp<'_>) -> Option<Option<[u8; 20]>> {
    let b = item.bytes()?;
    if b.is_empty() { Some(None) } else { h20(b).map(Some) }
}

struct Log {
    address: [u8; 20],
    topics: Vec<[u8; 32]>,
    data: Vec<u8>,
}

/// Receipt: the logs of a status-1 receipt; no status field (the pre-Byzantium 32-byte root) or a status
/// other than 1 is None.
fn receipt_logs(raw: &[u8]) -> Option<Vec<Log>> {
    let body = if matches!(raw.first()?, 0x01..=0x04) { &raw[1..] } else { raw };
    let l = rlp::decode(body)?;
    let f = l.list()?;
    if f.len() != 4 {
        return None;
    }
    let status = f[0].bytes()?;
    if status.len() > 1 || status != [1u8] {
        return None;
    }
    f[3].list()?.iter().map(|lg| {
        let p = lg.list()?;
        if p.len() != 3 {
            return None;
        }
        Some(Log {
            address: h20(p[0].bytes()?)?,
            topics: p[1].list()?.iter().map(|t| t.bytes().and_then(h32)).collect::<Option<_>>()?,
            data: p[2].bytes()?.to_vec(),
        })
    }).collect()
}

/// §9.1 containment: the 32-byte topic 2 appears whole in calldata at a multiple of 32, or at 4 plus a
/// multiple of 32. **The one containment test**: the kit judgment here and the scanner (`galeed-scan`) both read it.
#[cfg(not(feature = "ablate-k1-contained"))]
pub fn contained(data: &[u8], word: &[u8; 32]) -> bool {
    (0..data.len().saturating_sub(31)).filter(|o| o % 32 == 0 || o % 32 == 4).any(|o| &data[o..o + 32] == word)
}

/// Fault-injection variant for the conformance gauges: the 32 bytes may appear at any offset.
#[cfg(feature = "ablate-k1-contained")]
pub fn contained(data: &[u8], word: &[u8; 32]) -> bool {
    data.windows(32).any(|w| w == word)
}

/// §9.1 the data of an anchor: exactly the ABI encoding of one `bytes` value (offset word 32, length word `L`,
/// `L` bytes, the fewest zero bytes padding to a multiple of 32), and the `sha256` of those `L` bytes equals
/// topic 2. Only then does the record carry these entry bytes; any other data is an anchor that carries
/// nothing, and not a byte of it is read.
///
/// Data is read only after the log is settled to be an anchor: it cannot decide that (§9.1). The
/// scanner reads carried logs through this same function.
#[cfg(not(feature = "ablate-k1-anchor-form-carried"))]
pub fn carried_anchor(data: &[u8], hash: &[u8; 32]) -> Option<Vec<u8>> {
    let mut offset = [0u8; 32];
    offset[31] = 32;
    if data.len() < 64 || data[..32] != offset || data[32..56].iter().any(|b| *b != 0) {
        return None;
    }
    let len = usize::try_from(u64::from_be_bytes(data[56..64].try_into().ok()?)).ok()?;
    let padded = len.checked_add(31)? / 32 * 32;
    if data.len() != 64usize.checked_add(padded)? || data[64 + len..].iter().any(|b| *b != 0) {
        return None;
    }
    let bytes = &data[64..64 + len];
    (cryptox::sha256(bytes) == *hash).then(|| bytes.to_vec())
}

/// Fault-injection variant for the conformance gauges: the data itself is taken as the carried bytes,
/// with no ABI frame, padding or sha256 check.
#[cfg(feature = "ablate-k1-anchor-form-carried")]
pub fn carried_anchor(data: &[u8], hash: &[u8; 32]) -> Option<Vec<u8>> {
    let _ = hash;
    Some(data.to_vec())
}

/// Account proof reading: the account's code hash, the keccak of the empty string for an account absent from the
/// state trie (an absent address has no code); `None` the proof fails.
/// §9.6: the emitter's code hash at the anchor's block, from the kit's `emitterProof` against that block's stateRoot:
/// `None` where the kit carries no proof; a proof present that does not prove the account, or does not read, fails.
#[cfg(not(feature = "ablate-k1-emitter-proof"))]
fn emitter_proof(kit: &Value, root: [u8; 32], emitter: &[u8; 20]) -> Result<Option<[u8; 32]>, ()> {
    match hex_list(kit, "emitterProof")? {
        None => Ok(None),
        Some(proof) => account_code_hash(root, emitter, &proof).map(Some).ok_or(()),
    }
}

/// Fault-injection variant for the conformance gauges: the emitter's proof is not read, so a kit for a recipient that
/// admits emitters by class shows no code hash and the recipient cannot tell which code emitted the anchor.
#[cfg(feature = "ablate-k1-emitter-proof")]
fn emitter_proof(kit: &Value, root: [u8; 32], emitter: &[u8; 20]) -> Result<Option<[u8; 32]>, ()> {
    let _ = (kit, root, emitter);
    Ok(None)
}

fn account_code_hash(root: [u8; 32], address: &[u8; 20], proof: &[Vec<u8>]) -> Option<[u8; 32]> {
    match verify_proof(root, &cryptox::keccak256(address), proof).ok()? {
        None => Some(cryptox::keccak256(b"")),
        Some(acc) => {
            let l = rlp::decode(&acc)?;
            let f = l.list()?;
            if f.len() != 4 {
                return None;
            }
            h32(f[3].bytes()?)
        }
    }
}

/// Account proof reading: `Some(true)` has code, `Some(false)` has none (account absent, or codeHash is
/// the keccak of the empty string), `None` the proof fails.
fn account_has_code(root: [u8; 32], sender: &[u8; 20], proof: &[Vec<u8>]) -> Option<bool> {
    account_code_hash(root, sender, proof).map(|h| h != cryptox::keccak256(b""))
}

fn hex_field(v: &Value, k: &str) -> Option<Vec<u8>> {
    let s = v.get(k)?.as_str()?;
    hexform::decode_digits(s.strip_prefix("0x")?.as_bytes())
}
/// Three states: key absent (`Ok(None)`), present and well formed (`Ok(Some)`), present and unreadable
/// (`Err`). Absent and broken are different things.
fn hex_list(v: &Value, k: &str) -> Result<Option<Vec<Vec<u8>>>, ()> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Arr(a)) => a.iter().map(|x| hexform::decode_digits(x.as_str().ok_or(())?.strip_prefix("0x").ok_or(())?.as_bytes()).ok_or(())).collect::<Result<Vec<_>, ()>>().map(Some),
        Some(_) => Err(()),
    }
}
fn int_field(v: &Value, k: &str) -> Option<u64> {
    match v.get(k)? {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}

pub fn judge(kit: &Value) -> Judgment {
    let Some(chain_id) = int_field(kit, "chainId") else { return Judgment::Nothing("chainId") };
    let Some(hash) = hex_field(kit, "hash").and_then(|b| h32(&b)) else { return Judgment::Nothing("hash") };
    let Some(head) = hex_field(kit, "header").as_deref().and_then(header) else { return Judgment::Nothing("header") };
    let parent = match (head.number, hex_field(kit, "parentHeader")) {
        (0, _) => None,
        (_, Some(raw)) => match header(&raw) {
            Some(p) if p.hash == head.parent_hash && p.number + 1 == head.number => Some(p),
            _ => return Judgment::Nothing("parent-header"),
        },
        (_, None) => return Judgment::Nothing("parent-header"),
    };
    let Some(index) = int_field(kit, "txIndex") else { return Judgment::Nothing("txIndex") };
    let key = rlp::encode(&Rlp::Bytes(&rlp::uint_bytes(index)));
    let Ok(Some(tx_proof)) = hex_list(kit, "txProof") else { return Judgment::Nothing("txProof") };
    let Ok(Some(rc_proof)) = hex_list(kit, "receiptProof") else { return Judgment::Nothing("receiptProof") };
    let Ok(Some(tx_raw)) = verify_proof(head.tx_root, &key, &tx_proof) else { return Judgment::Nothing("tx-inclusion") };
    let Ok(Some(rc_raw)) = verify_proof(head.receipts_root, &key, &rc_proof) else { return Judgment::Nothing("receipt-inclusion") };
    let Some(t) = tx(&tx_raw) else { return Judgment::Nothing("transaction") };
    if t.chain_id != chain_id {
        return Judgment::Nothing("chain");
    }
    let Some(logs) = receipt_logs(&rc_raw) else { return Judgment::Nothing("status") };
    // §9.1, the one form: the emitter's log in a status-1 receipt carries exactly three topics, topic 0 the carried
    // form's, topic 1 the recovered sender, topic 2 the claimed hash; topic 2 whole on a word boundary of the
    // transaction's own calldata; a creation carries no calldata and anchors nothing. The data is read only after
    // the log is settled to be an anchor, and decides nothing about whether it is one.
    let Some(emitter) = hex_field(kit, "emitter").and_then(|b| h20(&b)) else { return Judgment::Nothing("emitter") };
    if t.to.is_none() {
        return Judgment::Nothing("creation");
    }
    let topic0 = cryptox::keccak256(ANCHORED_CARRIED_SIGNATURE.as_bytes());
    let mut topic1 = [0u8; 32];
    topic1[12..].copy_from_slice(&t.sender);
    let hit = logs.iter().find(|lg| lg.address == emitter && lg.topics.len() == 3 && lg.topics[0] == topic0 && lg.topics[1] == topic1 && lg.topics[2] == hash);
    let anchored = hit.is_some() && contained(&t.data, &hash);
    // §9.6: the bytes are a part of the kit. A kit that names them has them checked against the data's decoding and
    // its sha256; a kit that names none establishes an anchor carrying nothing.
    let mut entry: Option<Vec<u8>> = None;
    if let (true, Some(lg), Some(named)) = (anchored, hit, hex_field(kit, "entry")) {
        match carried_anchor(&lg.data, &hash) {
            Some(bytes) if bytes == named => entry = Some(bytes),
            _ => return Judgment::Nothing("entry"),
        }
    }
    if !anchored {
        return Judgment::Nothing("not-an-anchor");
    }
    let verdict = codeless_verdict(&head, parent.as_ref(), &t.sender, kit);
    let Some(verdict) = verdict else { return Judgment::Nothing("account-proof") };
    // §9.6: the emitter's account proof, where the kit carries one, fixes its code hash at the anchor's block; a
    // present proof that fails shows nothing (never "not asked").
    let Ok(emitter_code_hash) = emitter_proof(kit, head.state_root, &emitter) else { return Judgment::Nothing("emitter-proof") };
    Judgment::Shows(Shown {
        chain_id,
        block_number: head.number,
        block_timestamp: head.timestamp,
        tx: t.hash,
        sender: t.sender,
        hash,
        verdict,
        block_hash: head.hash,
        parent_hash: parent.map(|p| p.hash).unwrap_or(head.parent_hash),
        entry,
        emitter_code_hash,
    })
}

/// The two boundaries of §9.3: the end of the block before (for block 0, before genesis: asked, no code)
/// and the end of the anchor block. A boundary without a proof was not asked → UNPROVEN; both asked and
/// either has code → VOID; neither has code → counted; a failed proof → None.
#[cfg(not(feature = "ablate-k1-proof-kit"))]
fn codeless_verdict(head: &Header, parent: Option<&Header>, sender: &[u8; 20], kit: &Value) -> Option<&'static str> {
    // An absent proof means "not asked" (UNPROVEN); a present proof that is unreadable or fails shows
    // nothing (None).
    let before = match parent {
        None => Some(false),
        Some(p) => match hex_list(kit, "accountProofBefore").ok()? {
            None => None,
            Some(proof) => Some(account_has_code(p.state_root, sender, &proof)?),
        },
    };
    let after = match hex_list(kit, "accountProofAfter").ok()? {
        None => None,
        Some(proof) => Some(account_has_code(head.state_root, sender, &proof)?),
    };
    Some(match (before, after) {
        (Some(false), Some(false)) => VERDICT_COUNTED,
        (Some(_), Some(_)) => VERDICT_VOID,
        _ => VERDICT_UNPROVEN,
    })
}

/// Fault-injection variant for the conformance gauges: account proofs are not asked and every anchor
/// is counted.
#[cfg(feature = "ablate-k1-proof-kit")]
fn codeless_verdict(head: &Header, parent: Option<&Header>, sender: &[u8; 20], kit: &Value) -> Option<&'static str> {
    let _ = (head, parent, sender, kit);
    Some(VERDICT_COUNTED)
}

/// Transport form of a judgment: when shown, the seven §9.2 members plus two block hashes; otherwise
/// `{ok:false, reason}`.
pub fn judgment_value(j: &Judgment) -> Value {
    let s = |x: &str| Value::Str(x.to_string());
    match j {
        Judgment::Nothing(why) => Value::Obj(vec![("ok".into(), Value::Bool(false)), ("reason".into(), s(why))]),
        Judgment::Shows(k) => {
            let mut m = vec![
            ("ok".into(), Value::Bool(true)),
            ("chainId".into(), Value::Int(k.chain_id)),
            ("blockNumber".into(), Value::Int(k.block_number)),
            ("blockTimestamp".into(), Value::Int(k.block_timestamp)),
            ("tx".into(), s(&hexform::encode(&k.tx))),
            ("sender".into(), s(&hexform::encode(&k.sender))),
            ("hash".into(), s(&hexform::encode(&k.hash))),
            ("verdict".into(), s(k.verdict)),
            ("blockHash".into(), s(&hexform::encode(&k.block_hash))),
            ("parentHash".into(), s(&hexform::encode(&k.parent_hash))),
            ];
            // One more member for a kit that names its bytes (§9.2: `entry`, present where the bytes are carried).
            if let Some(e) = &k.entry {
                m.push(("entry".into(), s(&hexform::encode(e))));
            }
            // And one for a kit that carries the emitter's account proof (§9.6): the code hash a class item is tested on.
            if let Some(h) = &k.emitter_code_hash {
                m.push(("emitterCodeHash".into(), s(&hexform::encode(h))));
            }
            Value::Obj(m)
        }
    }
}

#[cfg(test)]
mod tests {
    //! A small chain built here: five transactions (0: a legacy transaction to its own sender carrying the hash, 1: a
    //! registry call whose log has the two-topic signature `Anchored(address,bytes32)`, 2: a call whose log is the
    //! §9.1 form with data that encodes the bytes, 3: the same form with data that encodes no `bytes` value, 4: the
    //! two-topic signature with data), the receipt trie, state tries before and after, parent and child headers;
    //! the kits over them, each judged. With `K1_KIT_FIXTURE_OUT=<dir>` the K1 journey's kits are written out.
    use super::*;
    use crate::cryptox::Privkey;
    use crate::json;

    // MPT construction for tests only (the product verifies proofs and never builds tries).
    enum Node {
        Empty,
        Leaf(Vec<u8>, Vec<u8>),
        Ext(Vec<u8>, Box<Node>),
        Branch(Vec<Node>, Option<Vec<u8>>),
    }

    fn branch() -> Node {
        Node::Branch((0..16).map(|_| Node::Empty).collect(), None)
    }

    fn insert(node: Node, path: &[u8], value: Vec<u8>) -> Node {
        match node {
            Node::Empty => Node::Leaf(path.to_vec(), value),
            Node::Leaf(p, v) => {
                if p == path {
                    return Node::Leaf(p, value);
                }
                let common = p.iter().zip(path).take_while(|(a, b)| a == b).count();
                let mut b = branch();
                b = insert(b, &p[common..], v);
                b = insert(b, &path[common..], value);
                if common == 0 { b } else { Node::Ext(p[..common].to_vec(), Box::new(b)) }
            }
            Node::Ext(p, child) => {
                let common = p.iter().zip(path).take_while(|(a, b)| a == b).count();
                if common == p.len() {
                    return Node::Ext(p, Box::new(insert(*child, &path[common..], value)));
                }
                let mut b = branch();
                let rest = p[common..].to_vec();
                let sub = if rest.len() == 1 { *child } else { Node::Ext(rest[1..].to_vec(), child) };
                if let Node::Branch(ref mut kids, _) = b {
                    kids[usize::from(rest[0])] = sub;
                }
                b = insert(b, &path[common..], value);
                if common == 0 { b } else { Node::Ext(p[..common].to_vec(), Box::new(b)) }
            }
            Node::Branch(mut kids, v) => {
                if path.is_empty() {
                    return Node::Branch(kids, Some(value));
                }
                let i = usize::from(path[0]);
                let old = std::mem::replace(&mut kids[i], Node::Empty);
                kids[i] = insert(old, &path[1..], value);
                Node::Branch(kids, v)
            }
        }
    }

    fn hp(path: &[u8], leaf: bool) -> Vec<u8> {
        let flag = if leaf { 2 } else { 0 } + (path.len() % 2) as u8;
        let mut out = Vec::new();
        let mut it = path.iter();
        if path.len() % 2 == 1 {
            out.push((flag << 4) | it.next().unwrap());
        } else {
            out.push(flag << 4);
        }
        let rest: Vec<u8> = it.copied().collect();
        for pair in rest.chunks(2) {
            out.push((pair[0] << 4) | pair[1]);
        }
        out
    }

    fn list(items: Vec<Vec<u8>>) -> Vec<u8> {
        let decoded: Vec<Rlp<'_>> = items.iter().map(|x| rlp::decode(x).unwrap()).collect();
        rlp::encode(&Rlp::List(decoded))
    }
    fn bstr(b: &[u8]) -> Vec<u8> {
        rlp::encode(&Rlp::Bytes(b))
    }

    fn enc(node: &Node, proof: &mut Vec<Vec<u8>>) -> Vec<u8> {
        let raw = match node {
            Node::Empty => bstr(&[]),
            Node::Leaf(p, v) => list(vec![bstr(&hp(p, true)), bstr(v)]),
            Node::Ext(p, c) => list(vec![bstr(&hp(p, false)), child(c, proof)]),
            Node::Branch(kids, v) => {
                let mut items: Vec<Vec<u8>> = kids.iter().map(|k| child(k, proof)).collect();
                items.push(bstr(v.as_deref().unwrap_or(&[])));
                list(items)
            }
        };
        raw
    }
    fn child(node: &Node, proof: &mut Vec<Vec<u8>>) -> Vec<u8> {
        if matches!(node, Node::Empty) {
            return bstr(&[]);
        }
        let raw = enc(node, proof);
        if raw.len() >= 32 {
            proof.push(raw.clone());
            bstr(&cryptox::keccak256(&raw))
        } else {
            raw
        }
    }
    fn trie(pairs: &[(Vec<u8>, Vec<u8>)]) -> ([u8; 32], Vec<Vec<u8>>) {
        let mut root = Node::Empty;
        for (k, v) in pairs {
            root = insert(root, &nibbles(k), v.clone());
        }
        let mut proof = Vec::new();
        let raw = enc(&root, &mut proof);
        proof.push(raw.clone());
        (cryptox::keccak256(&raw), proof)
    }

    fn u(n: u64) -> Vec<u8> {
        bstr(&rlp::uint_bytes(n))
    }
    fn strip(b: &[u8; 32]) -> Vec<u8> {
        rlp::uint_bytes_of(b)
    }

    fn header_raw(parent: [u8; 32], number: u64, ts: u64, state: [u8; 32], txs: [u8; 32], rcs: [u8; 32]) -> Vec<u8> {
        list(vec![
            bstr(&parent), bstr(&[0u8; 32]), bstr(&[0u8; 20]), bstr(&state), bstr(&txs), bstr(&rcs), bstr(&[0u8; 256]),
            u(0), u(number), u(30_000_000), u(50_000), u(ts), bstr(b""), bstr(&[0u8; 32]), bstr(&[0u8; 8]), u(7),
        ])
    }

    struct Chain {
        header: Vec<u8>,
        parent: Vec<u8>,
        tx_proof: Vec<Vec<u8>>,
        rc_proof: Vec<Vec<u8>>,
        before: Vec<Vec<u8>>,
        after: Vec<Vec<u8>>,
        registry: [u8; 20],
        hash: [u8; 32],
        sender: [u8; 20],
        /// The bytes the carried-form transactions commit to (`hash` is their sha256).
        entry: Vec<u8>,
    }

    /// ABI encoding of one `bytes` value: offset word 32, length word, bytes, zero padding to a multiple of 32.
    fn abi_bytes(b: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; 32];
        out[31] = 32;
        let mut len = [0u8; 32];
        len[24..].copy_from_slice(&(b.len() as u64).to_be_bytes());
        out.extend_from_slice(&len);
        out.extend_from_slice(b);
        out.extend(std::iter::repeat_n(0u8, (32 - b.len() % 32) % 32));
        out
    }

    const CHAIN_ID: u64 = 7;

    /// The signature of the registry's retired event, spelled here only to build a log no reader may take as an
    /// anchor (its topic 0 is not the one form's).
    const TWO_TOPIC_SIGNATURE: &str = "Anchored(address,bytes32)";

    fn build(code_after: bool) -> Chain {
        let mut kb = [0u8; 32];
        kb[31] = 1;
        let key = Privkey::from_bytes(&kb).unwrap();
        let sender = key.address();
        let hash = cryptox::sha256(b"entry");
        let registry = [0xabu8; 20];
        // 0: a legacy bare anchor, to = self, calldata = one word.
        let unsigned0 = list(vec![u(0), u(1), u(21_000), bstr(&sender), u(0), bstr(&hash), u(CHAIN_ID), bstr(b""), bstr(b"")]);
        let sig0 = key.sign(&cryptox::keccak256(&unsigned0));
        let v0 = CHAIN_ID * 2 + 35 + u64::from(sig0.v - 27);
        let tx0 = list(vec![u(0), u(1), u(21_000), bstr(&sender), u(0), bstr(&hash), u(v0), bstr(&strip(&sig0.r)), bstr(&strip(&sig0.s))]);
        // A type-2 transaction calling the registry, carrying the hash on a word boundary of calldata.
        let type2 = |nonce: u64, data: Vec<u8>| -> Vec<u8> {
            let fields = vec![u(CHAIN_ID), u(nonce), u(1), u(2), u(100_000), bstr(&registry), u(0), bstr(&data), list(vec![])];
            let mut pre = vec![0x02];
            pre.extend_from_slice(&list(fields.clone()));
            let sig = key.sign(&cryptox::keccak256(&pre));
            let mut all = fields;
            all.extend([u(u64::from(sig.v - 27)), bstr(&strip(&sig.r)), bstr(&strip(&sig.s))]);
            let mut tx = vec![0x02];
            tx.extend_from_slice(&list(all));
            tx
        };
        let call = |selector: [u8; 4], tail: &[u8]| -> Vec<u8> {
            let mut data = selector.to_vec();
            data.extend_from_slice(&hash);
            data.extend_from_slice(tail);
            data
        };
        let entry = b"entry".to_vec();
        // 1: anchor(bytes32). 2: a call with one `bytes` value whose data is exactly its ABI encoding.
        // 3: the same carried-form log with data that encodes no `bytes` value (anchors, carries nothing).
        // 4: the plain form's topic 0 with non-empty data: any data byte means not an anchor.
        let tx1 = type2(1, call([0x11, 0x22, 0x33, 0x44], &[]));
        let tx2 = type2(2, call([0x55, 0x66, 0x77, 0x88], &abi_bytes(&entry)));
        let tx3 = type2(3, call([0x55, 0x66, 0x77, 0x88], b"not-an-abi-bytes-value"));
        let tx4 = type2(4, call([0x11, 0x22, 0x33, 0x44], &[]));
        let rc0 = list(vec![u(1), u(21_000), bstr(&[0u8; 256]), list(vec![])]);
        let topic0 = cryptox::keccak256(TWO_TOPIC_SIGNATURE.as_bytes());
        let carried0 = cryptox::keccak256(ANCHORED_CARRIED_SIGNATURE.as_bytes());
        let mut topic1 = [0u8; 32];
        topic1[12..].copy_from_slice(&sender);
        let anchored_log = |t0: &[u8; 32], data: &[u8]| list(vec![bstr(&registry), list(vec![bstr(t0), bstr(&topic1), bstr(&hash)]), bstr(data)]);
        let receipt2 = |log: Vec<u8>| {
            let mut rc = vec![0x02];
            rc.extend_from_slice(&list(vec![u(1), u(50_000), bstr(&[0u8; 256]), list(vec![log])]));
            rc
        };
        let rc1 = receipt2(anchored_log(&topic0, b""));
        let rc2 = receipt2(anchored_log(&carried0, &abi_bytes(&entry)));
        let rc3 = receipt2(anchored_log(&carried0, b"not-an-abi-bytes-value"));
        let rc4 = receipt2(anchored_log(&topic0, &[0u8; 32]));
        let txs = vec![(bstr(&[]), tx0), (vec![0x01], tx1), (vec![0x02], tx2), (vec![0x03], tx3), (vec![0x04], tx4)];
        let rcs = vec![(bstr(&[]), rc0), (vec![0x01], rc1), (vec![0x02], rc2), (vec![0x03], rc3), (vec![0x04], rc4)];
        let (tx_root, tx_proof) = trie(&txs);
        let (rc_root, rc_proof) = trie(&rcs);
        // State: the sender and one other account; the after-state may carry code.
        let empty_root = cryptox::keccak256(&bstr(&[]));
        let account = |code: bool| list(vec![u(1), u(1_000_000), bstr(&empty_root), bstr(&if code { cryptox::keccak256(b"code") } else { cryptox::keccak256(b"") })]);
        let other = cryptox::keccak256(b"someone-else");
        let (state_before, before) = trie(&[(cryptox::keccak256(&sender).to_vec(), account(false)), (other.to_vec(), account(true))]);
        // The emitting registry stands in the after-state with code, so a kit can carry its account proof (§9.6).
        let (state_after, after) = trie(&[(cryptox::keccak256(&sender).to_vec(), account(code_after)), (other.to_vec(), account(true)), (cryptox::keccak256(&registry).to_vec(), account(true))]);
        let parent = header_raw([0x55u8; 32], 9, 1000, state_before, empty_root, empty_root);
        let header = header_raw(cryptox::keccak256(&parent), 10, 1010, state_after, tx_root, rc_root);
        Chain { header, parent, tx_proof, rc_proof, before, after, registry, hash, sender, entry }
    }

    fn hexs(b: &[u8]) -> Value {
        Value::Str(hexform::encode(b))
    }
    fn hexl(l: &[Vec<u8>]) -> Value {
        Value::Arr(l.iter().map(|x| hexs(x)).collect())
    }

    fn kit(c: &Chain, index: u64, hash: &[u8; 32], with_before: bool, parent_ok: bool) -> Value {
        let mut m: Vec<(String, Value)> = vec![
            ("chainId".into(), Value::Int(CHAIN_ID)),
            ("hash".into(), hexs(hash)),
            ("emitter".into(), hexs(&c.registry)),
            ("header".into(), hexs(&c.header)),
            ("txIndex".into(), Value::Int(index)),
            ("txProof".into(), hexl(&c.tx_proof)),
            ("receiptProof".into(), hexl(&c.rc_proof)),
            ("accountProofAfter".into(), hexl(&c.after)),
        ];
        let mut parent = c.parent.clone();
        if !parent_ok {
            parent[100] ^= 1;
        }
        m.push(("parentHeader".into(), hexs(&parent)));
        if with_before {
            m.push(("accountProofBefore".into(), hexl(&c.before)));
        }
        Value::Obj(m)
    }

    /// The kit names the bytes it carries (§9.6: when named, decoding and sha256 are checked; a kit that
    /// names none establishes the anchor alone).
    fn naming(k: &Value, bytes: &[u8]) -> Value {
        let mut m: Vec<(String, Value)> = k.as_obj().unwrap().to_vec();
        m.push(("entry".to_string(), hexs(bytes)));
        Value::Obj(m)
    }

    fn entry_of(k: &Value) -> Option<Vec<u8>> {
        match judge(k) {
            Judgment::Shows(s) => s.entry,
            Judgment::Nothing(_) => None,
        }
    }

    fn verdict_of(k: &Value) -> Result<&'static str, &'static str> {
        match judge(k) {
            Judgment::Shows(s) => Ok(s.verdict),
            Judgment::Nothing(why) => Err(why),
        }
    }

    #[test]
    fn kits() {
        let clean = build(false);
        let coded = build(true);
        // §9.1, the one form, at index 2: the log's data encodes the bytes.
        let counted = kit(&clean, 2, &clean.hash, true, true);
        let named = naming(&counted, &clean.entry);
        let void = kit(&coded, 2, &coded.hash, true, true);
        let unproven = kit(&clean, 2, &clean.hash, false, true);
        let wrong_hash = kit(&clean, 2, &cryptox::sha256(b"other"), true, true);
        let bad_parent = kit(&clean, 2, &clean.hash, true, false);
        assert_eq!(verdict_of(&counted), Ok(VERDICT_COUNTED), "§9.1 的那一形是锚");
        assert_eq!(entry_of(&counted), None, "不点名字节的包只立锚,不携带字节");
        assert_eq!(verdict_of(&named), Ok(VERDICT_COUNTED));
        assert_eq!(entry_of(&named), Some(clean.entry.clone()), "点名的字节经解码与 sha256 立住");
        assert_eq!(verdict_of(&void), Ok(VERDICT_VOID), "锚块末发送者带码");
        assert_eq!(verdict_of(&unproven), Ok(VERDICT_UNPROVEN), "前一块的边界没问");
        assert_eq!(verdict_of(&wrong_hash), Err("not-an-anchor"));
        assert_eq!(verdict_of(&bad_parent), Err("parent-header"));
        // The retired shapes are no anchor: a transaction to its own sender with no log, and a log under the
        // two-topic signature, with data or without.
        assert_eq!(verdict_of(&kit(&clean, 0, &clean.hash, true, true)), Err("not-an-anchor"), "无日志的交易不是锚");
        assert_eq!(verdict_of(&kit(&clean, 1, &clean.hash, true, true)), Err("not-an-anchor"), "两 topic 签名的日志不是锚");
        assert_eq!(verdict_of(&kit(&clean, 4, &clean.hash, true, true)), Err("not-an-anchor"));
        // A capture side's `form` member is not read, whatever it says.
        let mut with_form = named.as_obj().unwrap().to_vec();
        with_form.push(("form".into(), Value::Str("bare".into())));
        assert_eq!(verdict_of(&Value::Obj(with_form)), Ok(VERDICT_COUNTED));
        // A present but unreadable account proof is not "not asked": it shows nothing, never UNPROVEN.
        let mut malformed = void.clone();
        if let Value::Obj(m) = &mut malformed {
            for (k, v) in m.iter_mut() {
                if k == "accountProofBefore" {
                    *v = Value::Arr(vec![Value::Str("0xZZ".into())]);
                }
            }
        }
        assert_eq!(verdict_of(&malformed), Err("account-proof"));
        // §9.6, B032: a kit carrying the emitter's account proof answers the emitter's code hash (a recipient admitting
        // emitters by class decides on it); a kit carrying none answers none; a present proof that fails shows nothing.
        let code_hash_of = |k: &Value| match judge(k) {
            Judgment::Shows(s) => Ok(s.emitter_code_hash),
            Judgment::Nothing(why) => Err(why),
        };
        let with_emitter = |k: &Value, proof: Value| {
            let mut m = k.as_obj().unwrap().to_vec();
            m.push(("emitterProof".into(), proof));
            Value::Obj(m)
        };
        assert_eq!(code_hash_of(&named), Ok(None), "不带发出者账户证明即不答码哈希");
        let classed = with_emitter(&named, hexl(&clean.after));
        assert_eq!(code_hash_of(&classed), Ok(Some(cryptox::keccak256(b"code"))), "发出者账户证明立住其码哈希");
        assert_eq!(judgment_value(&judge(&classed)).get("emitterCodeHash").and_then(Value::as_str), Some(hexform::encode(&cryptox::keccak256(b"code")).as_str()));
        assert_eq!(code_hash_of(&with_emitter(&named, hexl(&clean.before))), Err("emitter-proof"), "证明对不上锚块的状态根即什么也不立");
        assert_eq!(code_hash_of(&with_emitter(&named, Value::Str("0x00".into()))), Err("emitter-proof"), "形不合的证明不是没问");
        if let Judgment::Shows(s) = judge(&named) {
            assert_eq!(s.sender, clean.sender);
            assert_eq!(s.block_number, 10);
            assert_eq!(s.block_timestamp, 1010);
            assert_eq!(s.chain_id, CHAIN_ID);
        }
        // Data that encodes no `bytes` value: the log is an anchor carrying nothing, so a kit naming bytes shows
        // nothing and a kit naming none establishes the anchor.
        let unframed = kit(&clean, 3, &clean.hash, true, true);
        let unframed_named = naming(&unframed, &clean.entry);
        let other_bytes = naming(&counted, b"other");
        assert_eq!(verdict_of(&unframed_named), Err("entry"), "data 不是一个 bytes 值的编码:立锚而不携带字节");
        assert_eq!(verdict_of(&unframed), Ok(VERDICT_COUNTED), "同一笔,不点名字节即只立锚");
        assert_eq!(verdict_of(&other_bytes), Err("entry"), "点名的字节不是 data 里那一份");
        if let Ok(dir) = std::env::var("K1_KIT_FIXTURE_OUT") {
            for (name, k) in [
                ("kit-void", &void),
                ("kit-unproven", &unproven),
                ("kit-nothing", &wrong_hash),
                ("kit-malformed-proof", &malformed),
                ("kit-carried-counted", &named),
                ("kit-carried-no-bytes", &counted),
                ("kit-carried-data-unframed", &unframed_named),
                ("kit-carried-data-unframed-no-bytes", &unframed),
                ("kit-no-log-not-anchor", &kit(&clean, 0, &clean.hash, true, true)),
                ("kit-two-topic-log-not-anchor", &kit(&clean, 1, &clean.hash, true, true)),
            ] {
                std::fs::write(format!("{dir}/{name}.json"), json::canonical(k).unwrap()).unwrap();
            }
        }
        // Edges of the ABI frame: offset word not 32, length disagreeing with the bytes, non-zero padding, one
        // trailing byte, the empty byte string.
        let framed = abi_bytes(&clean.entry);
        assert_eq!(carried_anchor(&framed, &clean.hash), Some(clean.entry.clone()));
        let mut bad_offset = framed.clone();
        bad_offset[31] = 33;
        assert_eq!(carried_anchor(&bad_offset, &clean.hash), None, "偏移字非 32");
        let mut bad_len = framed.clone();
        bad_len[63] += 1;
        assert_eq!(carried_anchor(&bad_len, &clean.hash), None, "长度字与字节不符");
        let mut bad_pad = framed.clone();
        *bad_pad.last_mut().unwrap() = 1;
        assert_eq!(carried_anchor(&bad_pad, &clean.hash), None, "补零非零");
        let mut long = framed.clone();
        long.push(0);
        assert_eq!(carried_anchor(&long, &clean.hash), None, "尾多一字节");
        assert_eq!(carried_anchor(&framed, &cryptox::sha256(b"other")), None, "sha256 不等于 topic 2");
        let empty = abi_bytes(b"");
        assert_eq!(carried_anchor(&empty, &cryptox::sha256(b"")), Some(Vec::new()), "恰空字节串");
    }

    #[test]
    fn proof_walls() {
        let (root, proof) = trie(&[(b"abc".to_vec(), b"v1".to_vec()), (b"abd".to_vec(), b"v2".to_vec()), (b"xyz".to_vec(), vec![9u8; 40])]);
        assert_eq!(verify_proof(root, b"abc", &proof), Ok(Some(b"v1".to_vec())));
        assert_eq!(verify_proof(root, b"xyz", &proof), Ok(Some(vec![9u8; 40])));
        assert_eq!(verify_proof(root, b"abe", &proof), Ok(None), "排除证明");
        assert_eq!(verify_proof(root, b"q", &proof), Ok(None));
        assert_eq!(verify_proof([1u8; 32], b"abc", &proof), Err(()), "根不对");
        let mut tampered = proof.clone();
        tampered.iter_mut().for_each(|n| n.push(0));
        assert_eq!(verify_proof(root, b"abc", &tampered), Err(()));
    }
}
