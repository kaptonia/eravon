use eravon_galeed as k1;

use crate::derive::{Action, Party};
use crate::token::{Fault, Token};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Secp256k1,
    Call,
}

impl Form {
    pub fn name(self) -> &'static str {
        match self {
            Form::Secp256k1 => "secp256k1",
            Form::Call => "call",
        }
    }
}

pub fn verify_secp256k1(digest: &[u8; 32], sig: &str, key: &str) -> Result<String, Fault> {
    if !k1::hexform::is_hex65(sig) {
        return Err(Fault::new(Token::SigForm));
    }
    let raw = k1::hexform::decode_form(sig);
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&raw[..32]);
    s.copy_from_slice(&raw[32..64]);
    let v = raw[64];
    if v != 27 && v != 28 {
        return Err(Fault::new(Token::SigV));
    }
    if !k1::sign::scalar_in_range(&r) || !k1::sign::scalar_in_range(&s) {
        return Err(Fault::new(Token::SigRange));
    }
    if k1::sign::is_high_s(&s) {
        return Err(Fault::new(Token::SigHighS));
    }
    let a = k1::sign::recover(digest, &r, &s, v - 27).ok_or_else(|| Fault::new(Token::SigRecover))?;
    let got = k1::hash::hex_encode(&a);
    if got != key {
        return Err(Fault::new(Token::SigSigner));
    }
    Ok(got)
}


pub fn is_empty_slot(sig: &str) -> bool {
    sig.is_empty()
}


pub fn verify_seller(digest: &[u8; 32], sig: &str, seller_key: &str) -> Result<(Form, Option<String>), Fault> {
    if is_empty_slot(sig) {
        return Ok((Form::Call, None));
    }
    verify_secp256k1(digest, sig, seller_key).map(|signer| (Form::Secp256k1, Some(signer)))
}

pub fn verify_buyer(digest: &[u8; 32], sig: &k1::Value, buyer_key: &str) -> Result<Form, Fault> {
    match sig {
        k1::Value::Str(s) if is_empty_slot(s) => Ok(Form::Call),
        k1::Value::Str(s) => verify_secp256k1(digest, s, buyer_key).map(|_| Form::Secp256k1),
        _ => Err(Fault::new(Token::SigForm)),
    }
}

pub fn verify_action(
    act: &Action,
    party: Party,
    digest: &[u8; 32],
    sig: &k1::Value,
    seller_key: &str,
    buyer_key: &str,
) -> Result<Form, Fault> {
    if let k1::Value::Str(s) = sig {
        if is_empty_slot(s) {
            return if act.admits_call() { Ok(Form::Call) } else { Err(Fault::new(Token::SigForm)) };
        }
    }
    match party {
        Party::Seller => match sig {
            k1::Value::Str(s) => verify_secp256k1(digest, s, seller_key).map(|_| Form::Secp256k1),
            _ => Err(Fault::new(Token::SigForm)),
        },
        Party::Buyer => verify_buyer(digest, sig, buyer_key),
    }
}

pub fn is_signature_shape(v: &k1::Value) -> bool {
    matches!(v, k1::Value::Str(s) if k1::hexform::is_hex65(s))
}

pub fn sign_secp256k1(privkey: &[u8; 32], digest: &[u8; 32]) -> Option<(String, String)> {
    let key = k1::sign::Privkey::from_bytes(privkey)?;
    let sig = k1::sign::sign_digest(&key, digest);
    let mut raw = [0u8; 65];
    raw[..32].copy_from_slice(&sig.r);
    raw[32..64].copy_from_slice(&sig.s);
    raw[64] = sig.v;
    Some((k1::hash::hex_encode(&raw), k1::hash::hex_encode(&key.address())))
}
