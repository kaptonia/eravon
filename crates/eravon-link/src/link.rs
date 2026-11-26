use eravon_galeed as k1;
use eravon_literals::literals as lit;
use eravon_terms as k2;
use k2::form;

use crate::listing::Listing;
use crate::settle::Settle;
use crate::token::{Fault, Token, Verdict, up};

#[derive(Clone, Debug)]
pub struct Link {
    pub chain_id: u64,
    pub tx: String,
    pub hash: String,
}

pub const E_LINK_OTHER_CHAIN: &str = "E_LINK_OTHER_CHAIN";

pub fn other_chain(link: &Link, here: u64) -> Option<String> {
    (link.chain_id != here).then(|| format!("链接在{},家所在的是{}", net_said(link.chain_id), net_said(here)))
}

fn net_said(c: u64) -> String {
    match eravon_literals::constants::network_of(c) {
        Some(n) => format!(" {} (链 {c})", n.name),
        None => format!("链 {c}"),
    }
}

const KEYS: [&str; 4] = ["v", "chainId", "tx", "hash"];

pub fn link(b: &[u8]) -> Result<Link, Fault> {
    let v = k1::canon::accept(b).map_err(|x| Fault::new(up(k2::token::canon(x))))?;
    let Some(m) = form::as_obj(&v) else {
        return Err(Fault::new(Token::Link));
    };
    if KEYS.iter().any(|k| !m.iter().any(|(key, _)| key == k)) {
        return Err(Fault::new(Token::LinkMissing));
    }
    if m.iter().any(|(key, _)| !KEYS.contains(&key.as_str())) {
        return Err(Fault::new(Token::LinkClosed));
    }
    let g = |k: &str| v.get(k).expect("必有成员已查过");
    if form::as_str(g("v")) != Some(lit::DOC_LINK) {
        return Err(Fault::new(Token::LinkV));
    }
    let Some(chain_id) = form::as_int(g("chainId")) else {
        return Err(Fault::new(Token::LinkChain));
    };
    if !form::is_hex32(g("tx")) {
        return Err(Fault::new(Token::LinkTx));
    }
    if !form::is_hex32(g("hash")) {
        return Err(Fault::new(Token::LinkHash));
    }
    let text = |k: &str| form::as_str(g(k)).expect("已过形").to_string();
    Ok(Link { chain_id, tx: text("tx"), hash: text("hash") })
}

#[derive(Clone, Debug)]
pub enum Resolved {
    Listing(Box<Listing>),
    Settle(Settle),
}

fn kind_of(b: &[u8]) -> Option<String> {
    let v = k1::canon::accept(b).ok()?;
    form::as_obj(&v)?;
    v.get("v").and_then(form::as_str).map(str::to_string)
}

pub fn resolve(link_bytes: &[u8], document: &[u8]) -> Result<Resolved, Verdict> {
    let ln = link(link_bytes).map_err(|f| Verdict::new("LINK_INVALID").own(f))?;
    if k1::hash::hex_encode(&k1::hash::sha256(document)) != ln.hash {
        return Err(Verdict::new("LINK_HASH"));
    }
    let Some(kind) = kind_of(document) else {
        return Err(Verdict::new("LINK_KIND"));
    };
    if kind == lit::DOC_LISTING {
        let s = crate::listing::listing(document).map_err(|f| Verdict::new("LINK_LISTING").own(f))?;
        if s.chain_id != ln.chain_id {
            return Err(Verdict::new("LINK_CHAIN"));
        }
        return Ok(Resolved::Listing(Box::new(s)));
    }
    if kind == lit::DOC_SETTLE {
        let st = crate::settle::settle(document).map_err(|f| Verdict::new("LINK_SETTLE").own(f))?;
        if st.chain_id != ln.chain_id {
            return Err(Verdict::new("LINK_CHAIN"));
        }
        return Ok(Resolved::Settle(st));
    }
    Err(Verdict::new("LINK_UNKNOWN"))
}
