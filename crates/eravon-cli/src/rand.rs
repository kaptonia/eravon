use crate::fault::{Fault, R};

pub fn bytes(n: usize) -> R<Vec<u8>> {
    eravon_os::random(n).map_err(|e| Fault::Io(e.to_string()))
}

pub fn scalar32() -> R<[u8; 32]> {
    for _ in 0..100 {
        let b = bytes(32)?;
        let mut k = [0u8; 32];
        k.copy_from_slice(&b);
        if eravon_terms::sig::sign_secp256k1(&k, &[0u8; 32]).is_some() {
            return Ok(k);
        }
    }
    Err(Fault::Io("取不到一枚落在范围里的三十二字节".into()))
}
