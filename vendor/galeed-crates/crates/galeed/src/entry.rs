//! A galeed/1 entry in the shape the family's shared desk code reads it: the envelope (§4) judged by
//! [`crate::envelope::accept`] and laid out member by member, plus the §5 signing steps by the names that code
//! calls them. Nothing is judged here that [`crate::envelope`] and [`crate::sign`] do not judge.

use crate::cryptox;
use crate::hexform;
use crate::json::{self, Value};
use crate::tokens::{EntryType, Token};

/// An entry that passes all thirteen steps of §4.3.
#[derive(Clone, Debug)]
pub struct Entry {
    pub bytes: Vec<u8>,
    pub id: [u8; 32],
    pub value: Value,
    /// The entryType bytes as they came (§6.8: an unlisted type is still an entry).
    pub entry_type: String,
    /// The same value read through the §6 type table; compare this, not the literal.
    pub kind: EntryType,
    /// The `judge` member (the ledger's author).
    pub author: String,
    pub seq: u64,
    /// `None` at seq 0 (§4.3 step 8).
    pub prev: Option<String>,
    pub body: Value,
}

impl Entry {
    pub fn id_hex(&self) -> String {
        hexform::encode(&self.id)
    }
}

/// The most bytes one entry may have, so that one transaction can carry it (§11: this grammar sets no ceiling, and
/// caps belong to the storage and transport layers, declared there; this is the one place the product declares its
/// cap).
///
/// The bound is the transport's: an entry is published only as the `entry` of `anchorCarried(bytes32 hash, bytes
/// entry)` (§9.1), sent as one signed transaction. Node transaction pools admit a transaction of at most 131,072
/// bytes (128 KiB; the size bound geth and Nethermind apply to a transaction they relay). Of those, the call's
/// framing takes 100 bytes (the 4-byte selector, the hash word, the offset word, the length word), and a type-2
/// envelope at most 192 more (type byte, list and data headers, chain id, nonce, both fee caps and the gas limit at
/// full width, recipient, value, access list, the three signature fields), and the entry's bytes travel padded to a
/// 32-byte word: (131,072 − 192 − 100) rounded down to a multiple of 32 is 130,752. Gas does not bind first: at this
/// size the calldata floor (EIP-7623, ten gas per token, four tokens a non-zero byte) and the log's eight gas a byte
/// stay near 5.3 million, under the per-transaction cap of 2^24 (EIP-7825) and far under the block gas limits read
/// from the preset nodes (60,000,000 on mainnet, 199,804,689 on Sepolia, 2026-10-20).
pub const CARRIED_MAX: usize = 130_752;

/// §2.1: every byte string has an entry_id.
pub fn entry_id(b: &[u8]) -> [u8; 32] {
    cryptox::sha256(b)
}

/// The thirteen steps of §4.3 ([`crate::envelope::accept`]), the accepted entry laid out.
pub fn check(b: &[u8]) -> Result<Entry, Token> {
    let e = crate::envelope::accept(b)?;
    let v = e.value;
    let text = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let entry_type = text("entryType");
    Ok(Entry {
        bytes: b.to_vec(),
        id: e.entry_id,
        kind: EntryType::of(&entry_type),
        entry_type,
        author: text("judge"),
        seq: v.get("seq").and_then(Value::as_int).unwrap_or_default(),
        prev: v.get("prev").and_then(Value::as_str).map(str::to_string),
        body: v.get("body").cloned().unwrap_or(Value::Null),
        value: v,
    })
}

/// §5.1: canonical bytes of the object without its top-level `sig` (a value outside the §3.1 universe has none:
/// empty).
pub fn b6_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Obj(ms) => crate::sign::b6_of(ms).unwrap_or_default(),
        other => json::canon_bytes(other),
    }
}

/// §5.2: `M = D || 0x0A || hex32(presig)`.
pub fn message(domain: &str, presig: &[u8; 32]) -> Vec<u8> {
    crate::sign::message(domain.as_bytes(), presig)
}

/// §5.3: the EIP-191 digest of `M`.
pub fn eip191_digest(m: &[u8]) -> [u8; 32] {
    crate::sign::digest(m)
}

/// presig and digest under a domain, computed together.
pub fn presig_and_digest(preimage: &[u8], domain: &str) -> ([u8; 32], [u8; 32]) {
    let presig = cryptox::sha256(preimage);
    (presig, eip191_digest(&message(domain, &presig)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cap is word-aligned, as the call pads the bytes to one, and the whole transaction at the bound fits the
    /// pools' bound while the next word does not. Its readers judge by it at the store (`galeed-store`) and the send
    /// (`galeed-chain::send`).
    #[test]
    fn the_cap_is_the_last_word_one_transaction_carries() {
        assert_eq!(CARRIED_MAX % 32, 0);
        // The whole transaction at the bound: framing and the widest envelope fit the pools' 131,072 bytes.
        assert!(CARRIED_MAX + 100 + 192 <= 131_072);
        assert!(CARRIED_MAX + 32 + 100 + 192 > 131_072, "下一个整字即越界");
    }
}
