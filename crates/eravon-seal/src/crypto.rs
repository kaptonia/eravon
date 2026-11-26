thread_local! {
    static DERIVATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub fn derivations() -> usize {
    DERIVATIONS.with(|d| d.get())
}

pub fn derived_once() {
    DERIVATIONS.with(|d| d.set(d.get() + 1));
}

pub fn scrypt(password: &[u8], salt: &[u8], n: usize, r: usize, p: usize, out: &mut [u8]) -> bool {
    if n < 2 || n & (n - 1) != 0 || r == 0 || p == 0 || r > u32::MAX as usize || p > u32::MAX as usize {
        return false;
    }
    derived_once();
    let Ok(params) = scrypt::Params::new(n.trailing_zeros() as u8, r as u32, p as u32, out.len()) else {
        return false;
    };
    scrypt::scrypt(password, salt, &params, out).is_ok()
}

pub fn aes128_ctr(key: &[u8; 16], iv: &[u8; 16], buf: &mut [u8]) {
    use ctr::cipher::{KeyIvInit, StreamCipher};
    let mut c = ctr::Ctr128BE::<aes::Aes128>::new(key.into(), iv.into());
    c.apply_keystream(buf);
}

pub fn hkdf_sha256(ikm: &[u8], info: &[u8], out: &mut [u8]) -> bool {
    hkdf::Hkdf::<sha2::Sha256>::new(None, ikm).expand(info, out).is_ok()
}

pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    use hmac::Mac;
    let mut m = <hmac::Hmac<sha2::Sha256> as hmac::Mac>::new_from_slice(key).expect("HMAC takes a key of any length");
    m.update(msg);
    m.finalize().into_bytes().into()
}

pub fn xchacha_seal(key: &[u8; 32], nonce: &[u8; 24], aad: &[u8], plain: &[u8]) -> Option<Vec<u8>> {
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    let c = chacha20poly1305::XChaCha20Poly1305::new(key.into());
    c.encrypt(nonce.into(), Payload { msg: plain, aad }).ok()
}

pub fn xchacha_open(key: &[u8; 32], nonce: &[u8; 24], aad: &[u8], sealed: &[u8]) -> Option<Vec<u8>> {
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    let c = chacha20poly1305::XChaCha20Poly1305::new(key.into());
    c.decrypt(nonce.into(), Payload { msg: sealed, aad }).ok()
}
