use crate::fault::{Fault, R};
use crate::order;

pub const TX_TYPE_1559: u8 = 0x02;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tx {
    pub chain_id: u64,
    pub nonce: u64,
    pub max_priority_fee: [u8; 32],
    pub max_fee: [u8; 32],
    pub gas_limit: u64,
    pub to: Option<[u8; 20]>,
    pub value: [u8; 32],
    pub data: Vec<u8>,
}

impl Tx {
    pub fn digest(&self) -> [u8; 32] {
        eravon_terms::derive::keccak(&self.signing_payload())
    }

    pub fn signing_payload(&self) -> Vec<u8> {
        let mut out = vec![TX_TYPE_1559];
        out.extend_from_slice(&rlp_list(&self.nine()));
        out
    }

    pub fn raw(&self, sig: &[u8]) -> R<Vec<u8>> {
        if sig.len() != 65 {
            return Err(Fault::Form(format!("签名该是六十五字节,给的是 {}", sig.len())));
        }
        let v = sig[64];
        let y_parity = match v {
            27 | 28 => u64::from(v - 27),
            0 | 1 => u64::from(v),
            other => {
                return Err(Fault::Form(format!("恢复位只收 27、28(或 0、1),给的是 {other}")));
            }
        };
        let mut items = self.nine();
        items.push(rlp_uint(y_parity));
        items.push(rlp_bytes(trim_left(&sig[..32])));
        items.push(rlp_bytes(trim_left(&sig[32..64])));
        let mut out = vec![TX_TYPE_1559];
        out.extend_from_slice(&rlp_list(&items));
        Ok(out)
    }

    pub fn hash(&self, sig: &[u8]) -> R<[u8; 32]> {
        Ok(eravon_terms::derive::keccak(&self.raw(sig)?))
    }

    fn nine(&self) -> Vec<Vec<u8>> {
        vec![
            rlp_uint(self.chain_id),
            rlp_uint(self.nonce),
            rlp_bytes(trim_left(&self.max_priority_fee)),
            rlp_bytes(trim_left(&self.max_fee)),
            rlp_uint(self.gas_limit),
            match &self.to {
                Some(a) => rlp_bytes(a),
                None => rlp_bytes(&[]),
            },
            rlp_bytes(trim_left(&self.value)),
            rlp_bytes(&self.data),
            rlp_list(&[]),
        ]
    }
}

#[allow(clippy::too_many_arguments)]
pub fn assemble(
    chain_id: u64,
    nonce: u64,
    max_priority_fee: &[u8; 32],
    max_fee: &[u8; 32],
    gas_limit: u64,
    to: &str,
    value: &[u8; 32],
    data: &[u8],
) -> R<Tx> {
    Ok(Tx {
        chain_id,
        nonce,
        max_priority_fee: *max_priority_fee,
        max_fee: *max_fee,
        gas_limit,
        to: Some(order::addr_of("交易的收件人", to)?),
        value: *value,
        data: data.to_vec(),
    })
}

pub fn rlp_bytes(b: &[u8]) -> Vec<u8> {
    if b.len() == 1 && b[0] < 0x80 {
        return b.to_vec();
    }
    let mut out = header(0x80, 0xb7, b.len());
    out.extend_from_slice(b);
    out
}

pub fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = items.iter().flat_map(|x| x.iter().copied()).collect();
    let mut out = header(0xc0, 0xf7, body.len());
    out.extend_from_slice(&body);
    out
}

pub fn rlp_uint(n: u64) -> Vec<u8> {
    rlp_bytes(trim_left(&n.to_be_bytes()))
}


fn header(short: u8, long: u8, len: usize) -> Vec<u8> {
    if len <= 55 {
        return vec![short + len as u8];
    }
    let be = len.to_be_bytes();
    let sig = trim_left(&be);
    let mut out = vec![long + sig.len() as u8];
    out.extend_from_slice(sig);
    out
}

fn trim_left(b: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < b.len() && b[i] == 0 {
        i += 1;
    }
    &b[i..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rlp_three_branches_follow_the_spec() {
        assert_eq!(rlp_bytes(&[0x7f]), vec![0x7f]);
        assert_eq!(rlp_bytes(&[]), vec![0x80]);
        assert_eq!(rlp_bytes(b"dog"), vec![0x83, b'd', b'o', b'g']);
        let long = vec![0x61u8; 56];
        assert_eq!(rlp_bytes(&long)[..2], [0xb8, 56]);
        assert_eq!(rlp_list(&[]), vec![0xc0]);
        assert_eq!(rlp_list(&[rlp_bytes(b"cat"), rlp_bytes(b"dog")]).len(), 9);
    }

    #[test]
    fn integers_drop_leading_zeros_and_zero_is_empty() {
        assert_eq!(rlp_uint(0), vec![0x80]);
        assert_eq!(rlp_uint(1), vec![0x01]);
        assert_eq!(rlp_uint(0x0400), vec![0x82, 0x04, 0x00]);
    }

    #[test]
    fn recovery_bit_accepts_both_forms_and_refuses_others() {
        let tx = Tx {
            chain_id: 1,
            nonce: 0,
            max_priority_fee: [0u8; 32],
            max_fee: [0u8; 32],
            gas_limit: 21000,
            to: Some([0u8; 20]),
            value: [0u8; 32],
            data: Vec::new(),
        };
        let mut sig = [0u8; 65];
        sig[64] = 28;
        assert!(tx.raw(&sig).is_ok());
        sig[64] = 1;
        assert!(tx.raw(&sig).is_ok());
        sig[64] = 7;
        assert!(tx.raw(&sig).is_err());
        assert!(tx.raw(&sig[..64]).is_err());
    }

    #[test]
    fn signing_payload_starts_with_the_type_byte() {
        let tx = Tx {
            chain_id: 31337,
            nonce: 3,
            max_priority_fee: [0u8; 32],
            max_fee: [0u8; 32],
            gas_limit: 21000,
            to: Some([0x11u8; 20]),
            value: [0u8; 32],
            data: vec![0xde, 0xad],
        };
        let p = tx.signing_payload();
        assert_eq!(p[0], TX_TYPE_1559);
        assert_eq!(tx.digest(), eravon_terms::derive::keccak(&p));
    }
}
