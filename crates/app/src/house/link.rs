use eravon_link::transport;
use eravon_link::token::Token;

use crate::house::seam;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dispatched {
    pub kind: &'static str,
    pub bytes: usize,
    pub body: Vec<u8>,
}

pub fn dispatch(text: &str) -> Dispatched {
    match transport::decode(text.as_bytes(), transport::LINK_PREFIX, Token::LinkPrefix, Token::LinkB64) {
        Ok(b) => Dispatched { kind: seam::LINK_OPEN, bytes: b.len(), body: b },
        Err(_) => Dispatched { kind: seam::LINK_UNKNOWN, bytes: 0, body: Vec::new() },
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Pay,
    Unknown,
}

pub fn route(d: &Dispatched) -> Route {
    match d.kind {
        seam::LINK_OPEN => Route::Pay,
        _ => Route::Unknown,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use eravon_terms::form;

    fn url(prefix: &str, body: &[u8]) -> String {
        format!("{prefix}{}", form::b64url_encode(body))
    }

    #[test]
    fn a_link_comes_apart_and_anything_else_is_named() {
        let p = b"0123456789";
        assert_eq!(dispatch(&url("eravon://open?e=", p)), Dispatched { kind: seam::LINK_OPEN, bytes: 10, body: p.to_vec() });
        assert_eq!(dispatch("eravon://nope?x=AAAA"), Dispatched { kind: seam::LINK_UNKNOWN, bytes: 0, body: Vec::new() });
        assert_eq!(route(&dispatch(&url("eravon://open?e=", p))), Route::Pay);
        assert_eq!(route(&dispatch("eravon://nope?x=AAAA")), Route::Unknown);
    }
}
