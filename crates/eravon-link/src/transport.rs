use eravon_literals::literals as lit;
use eravon_literals::seam;
use eravon_terms::form;

use crate::token::{Fault, Token};

pub fn decode(text: &[u8], prefix: &str, t_prefix: Token, t_b64: Token) -> Result<Vec<u8>, Fault> {
    let Some(seg) = text.strip_prefix(prefix.as_bytes()) else {
        return Err(Fault::new(t_prefix));
    };
    let seg = core::str::from_utf8(seg).ok().filter(|s| !s.is_empty()).ok_or_else(|| Fault::new(t_b64))?;
    form::b64url_decode(seg).ok_or_else(|| Fault::new(t_b64))
}


pub const LINK_PREFIX: &str = lit::PREFIX_LINK_SCHEME;
pub const RECEIPT_PREFIX: &str = lit::PREFIX_RECEIPT;

pub fn encode_link(bytes: &[u8]) -> String {
    seam::link_scheme(&form::b64url_encode(bytes))
}

pub fn decode_link(text: &[u8]) -> Result<(Vec<u8>, crate::link::Link), Fault> {
    let raw = decode(text, LINK_PREFIX, Token::LinkPrefix, Token::LinkB64)?;
    let l = crate::link::link(&raw).map_err(|f| Fault::new(Token::LinkDoc).own(f))?;
    Ok((raw, l))
}

pub fn encode_receipt(bytes: &[u8]) -> String {
    seam::receipt_text(&form::b64url_encode(bytes))
}

pub fn decode_receipt(text: &[u8]) -> Result<(Vec<u8>, crate::receipt::Receipt), Fault> {
    let raw = decode(text, RECEIPT_PREFIX, Token::ReceiptPrefix, Token::ReceiptB64)?;
    let r = crate::receipt::receipt(&raw).map_err(|f| Fault::new(Token::ReceiptDoc).own(f))?;
    Ok((raw, r))
}
