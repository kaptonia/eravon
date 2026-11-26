use galeed::cryptox;
use galeed::hexform;
use galeed::json::Value;
use galeed::rlp::{self, Rlp};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tx {
    pub hash: String,
    pub sender: Option<String>,
    pub to: Option<String>,
    pub input: Vec<u8>,
    pub creation: bool,
    pub chain_id: Option<u64>,
    pub tx_type: u64,
    pub block_number: Option<u64>,
}

pub fn qty(s: &str) -> Option<Vec<u8>> {
    let d = s.strip_prefix("0x")?;
    if d.is_empty() {
        return Some(Vec::new());
    }
    let padded = if d.len() % 2 == 1 { format!("0{d}") } else { d.to_string() };
    let raw = hexform::decode_digits(padded.as_bytes())?;
    Some(match raw.iter().position(|x| *x != 0) {
        Some(i) => raw[i..].to_vec(),
        None => Vec::new(),
    })
}

pub fn bytes(s: &str) -> Option<Vec<u8>> {
    let d = s.strip_prefix("0x")?;
    if d.is_empty() {
        return Some(Vec::new());
    }
    hexform::decode_digits(d.as_bytes())
}

fn qty_of(v: &Value, k: &str) -> Option<Vec<u8>> {
    qty(v.get(k)?.as_str()?)
}
fn bytes_of(v: &Value, k: &str) -> Option<Vec<u8>> {
    bytes(v.get(k)?.as_str()?)
}
pub fn u64_of_qty(b: &[u8]) -> Option<u64> {
    (b.len() <= 8).then(|| b.iter().fold(0u64, |a, x| (a << 8) | u64::from(*x)))
}

fn num_of(v: &Value, k: &str) -> Option<u64> {
    u64_of_qty(&qty_of(v, k)?)
}

fn word(b: &[u8]) -> Option<[u8; 32]> {
    (b.len() <= 32).then(|| {
        let mut w = [0u8; 32];
        w[32 - b.len()..].copy_from_slice(b);
        w
    })
}

fn access_list(v: Option<&Value>) -> Option<Vec<(Vec<u8>, Vec<Vec<u8>>)>> {
    match v {
        None | Some(Value::Null) => Some(Vec::new()),
        Some(Value::Arr(a)) => a
            .iter()
            .map(|e| {
                let addr = bytes_of(e, "address")?;
                let keys = match e.get("storageKeys") {
                    Some(Value::Arr(ks)) => ks.iter().map(|k| bytes(k.as_str()?)).collect::<Option<Vec<_>>>()?,
                    _ => Vec::new(),
                };
                Some((addr, keys))
            })
            .collect(),
        _ => None,
    }
}

fn auth_list(v: Option<&Value>) -> Option<Vec<[Vec<u8>; 6]>> {
    match v {
        None | Some(Value::Null) => Some(Vec::new()),
        Some(Value::Arr(a)) => a
            .iter()
            .map(|e| {
                Some([
                    qty_of(e, "chainId")?,
                    bytes_of(e, "address")?,
                    qty_of(e, "nonce")?,
                    qty_of(e, "yParity").or_else(|| qty_of(e, "v"))?,
                    qty_of(e, "r")?,
                    qty_of(e, "s")?,
                ])
            })
            .collect(),
        _ => None,
    }
}

fn al_rlp(al: &[(Vec<u8>, Vec<Vec<u8>>)]) -> Rlp<'_> {
    Rlp::List(
        al.iter()
            .map(|(addr, keys)| Rlp::List(vec![Rlp::Bytes(addr), Rlp::List(keys.iter().map(|k| Rlp::Bytes(k)).collect())]))
            .collect(),
    )
}

fn auth_rlp(al: &[[Vec<u8>; 6]]) -> Rlp<'_> {
    Rlp::List(al.iter().map(|a| Rlp::List(a.iter().map(|x| Rlp::Bytes(x)).collect())).collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxFault {
    Unreadable,
    UndecodableEnvelope,
}

pub fn read(v: &Value) -> Option<Tx> {
    read_envelope(v).ok()
}

pub fn read_envelope(v: &Value) -> Result<Tx, TxFault> {
    use TxFault::Unreadable;
    let hash = v.get("hash").and_then(Value::as_str).filter(|h| hexform::is_hex32(h)).ok_or(Unreadable)?.to_string();
    let zero = || Some(Vec::new());
    let field = |k: &str| match v.get(k) {
        None | Some(Value::Null) => zero(),
        Some(_) => qty_of(v, k),
    };
    let type_bytes = field("type").ok_or(Unreadable)?;
    let tx_type = u64_of_qty(&type_bytes);
    let to_raw = match v.get("to") {
        None | Some(Value::Null) => None,
        Some(t) => Some(bytes(t.as_str().ok_or(Unreadable)?).ok_or(Unreadable)?),
    };
    let creation = to_raw.is_none();
    let input = match v.get("input") {
        None | Some(Value::Null) => Vec::new(),
        Some(_) => bytes_of(v, "input").ok_or(Unreadable)?,
    };
    let (r, s) = (field("r").ok_or(Unreadable)?, field("s").ok_or(Unreadable)?);
    if r.is_empty() && s.is_empty() {
        return Ok(unsigned(hash, to_raw, input, creation, None, tx_type.unwrap_or(u64::MAX)));
    }
    let Some(tx_type @ 0..=4) = tx_type else { return Err(TxFault::UndecodableEnvelope) };
    let (nonce, gas, value) = (qty_of(v, "nonce").ok_or(Unreadable)?, qty_of(v, "gas").ok_or(Unreadable)?, qty_of(v, "value").ok_or(Unreadable)?);
    let to_b = to_raw.clone().unwrap_or_default();
    let al = access_list(v.get("accessList")).ok_or(Unreadable)?;
    let (payload, signed, chain_id, parity): (Option<Vec<u8>>, Vec<u8>, Option<u64>, Option<u64>) = match tx_type {
        0 => {
            let gas_price = qty_of(v, "gasPrice").ok_or(Unreadable)?;
            let v_bytes = field("v").ok_or(Unreadable)?;
            let signed = rlp::encode(&Rlp::List(vec![
                Rlp::Bytes(&nonce),
                Rlp::Bytes(&gas_price),
                Rlp::Bytes(&gas),
                Rlp::Bytes(&to_b),
                Rlp::Bytes(&value),
                Rlp::Bytes(&input),
                Rlp::Bytes(&v_bytes),
                Rlp::Bytes(&r),
                Rlp::Bytes(&s),
            ]));
            match u64_of_qty(&v_bytes).filter(|n| *n >= 35) {
                Some(v_raw) => {
                    let chain_id = (v_raw - 35) / 2;
                    let cid = rlp::uint_bytes(chain_id);
                    let payload = rlp::encode(&Rlp::List(vec![
                        Rlp::Bytes(&nonce),
                        Rlp::Bytes(&gas_price),
                        Rlp::Bytes(&gas),
                        Rlp::Bytes(&to_b),
                        Rlp::Bytes(&value),
                        Rlp::Bytes(&input),
                        Rlp::Bytes(&cid),
                        Rlp::Bytes(&[]),
                        Rlp::Bytes(&[]),
                    ]));
                    (Some(payload), signed, Some(chain_id), Some((v_raw - 35) % 2))
                }
                None => (None, signed, None, None),
            }
        }
        _ => {
            let cid = qty_of(v, "chainId").ok_or(Unreadable)?;
            let parity_bytes = match v.get("yParity") {
                None | Some(Value::Null) => field("v").ok_or(Unreadable)?,
                Some(_) => qty_of(v, "yParity").ok_or(Unreadable)?,
            };
            let mut fields: Vec<Rlp<'_>> = Vec::new();
            let (gas_price, prio, fee) = (qty_of(v, "gasPrice").unwrap_or_default(), qty_of(v, "maxPriorityFeePerGas").unwrap_or_default(), qty_of(v, "maxFeePerGas").unwrap_or_default());
            let blob_fee = qty_of(v, "maxFeePerBlobGas").unwrap_or_default();
            let blobs: Vec<Vec<u8>> = match v.get("blobVersionedHashes") {
                Some(Value::Arr(a)) => a.iter().map(|x| bytes(x.as_str()?)).collect::<Option<_>>().ok_or(Unreadable)?,
                _ => Vec::new(),
            };
            let auths = auth_list(v.get("authorizationList")).ok_or(Unreadable)?;
            fields.push(Rlp::Bytes(&cid));
            fields.push(Rlp::Bytes(&nonce));
            if tx_type == 1 {
                fields.push(Rlp::Bytes(&gas_price));
            } else {
                fields.push(Rlp::Bytes(&prio));
                fields.push(Rlp::Bytes(&fee));
            }
            fields.push(Rlp::Bytes(&gas));
            fields.push(Rlp::Bytes(&to_b));
            fields.push(Rlp::Bytes(&value));
            fields.push(Rlp::Bytes(&input));
            fields.push(al_rlp(&al));
            if tx_type == 3 {
                fields.push(Rlp::Bytes(&blob_fee));
                fields.push(Rlp::List(blobs.iter().map(|b| Rlp::Bytes(b)).collect()));
            }
            if tx_type == 4 {
                fields.push(auth_rlp(&auths));
            }
            let mut payload = vec![tx_type as u8];
            payload.extend_from_slice(&rlp::encode(&Rlp::List(fields.clone())));
            let mut all = fields;
            all.extend([Rlp::Bytes(&parity_bytes), Rlp::Bytes(&r), Rlp::Bytes(&s)]);
            let mut signed = vec![tx_type as u8];
            signed.extend_from_slice(&rlp::encode(&Rlp::List(all)));
            (Some(payload), signed, u64_of_qty(&cid), u64_of_qty(&parity_bytes))
        }
    };
    if hexform::encode(&cryptox::keccak256(&signed)) != hash {
        return Err(Unreadable);
    }
    let block_number = block_of(v);
    let sender = match (payload, parity) {
        (Some(payload), Some(parity @ 0..=1)) => match (word(&r), word(&s)) {
            (Some(rw), Some(sw)) => cryptox::recover_address(&cryptox::keccak256(&payload), &rw, &sw, parity as u8).map(|a| hexform::encode(&a)),
            _ => None,
        },
        _ => None,
    };
    Ok(Tx { hash, sender, to: to_raw.map(|b| hexform::encode(&b)), input, creation, chain_id, tx_type, block_number })
}

fn unsigned(hash: String, to: Option<Vec<u8>>, input: Vec<u8>, creation: bool, chain_id: Option<u64>, tx_type: u64) -> Tx {
    Tx { hash, sender: None, to: to.map(|b| hexform::encode(&b)), input, creation, chain_id, tx_type, block_number: None }
}

fn block_of(v: &Value) -> Option<u64> {
    match v.get("blockNumber") {
        Some(Value::Str(s)) => num_of(&Value::Obj(vec![("n".to_string(), Value::Str(s.clone()))]), "n"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_bytes_of_quantity_are_unreadable_not_folded() {
        assert_eq!(u64_of_qty(&[0xff; 8]), Some(u64::MAX));
        assert_eq!(u64_of_qty(&[]), Some(0));
        let mut nine = vec![0x01u8];
        nine.extend_from_slice(&31_337u64.to_be_bytes());
        assert_eq!(nine.len(), 9);
        assert_eq!(u64_of_qty(&nine), None, "九字节读不出,不顶替");
    }

    #[test]
    fn quantities_and_byte_strings_are_two_readings() {
        assert_eq!(qty("0x0"), Some(vec![]));
        assert_eq!(qty("0x1c9c380"), Some(vec![0x01, 0xc9, 0xc3, 0x80]));
        assert_eq!(bytes("0x"), Some(vec![]));
        assert_eq!(bytes("0xabcd"), Some(vec![0xab, 0xcd]));
        assert_eq!(bytes("0xabc"), None, "字节串的位数必偶");
    }
}
