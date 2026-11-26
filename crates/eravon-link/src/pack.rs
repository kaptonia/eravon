use eravon_galeed as k1;
use eravon_literals::literals as lit;
use eravon_terms as k2;
use k2::form;

use crate::link::{self, Link};
use crate::listing::{self, Listing};
use crate::receipt::{self, Bound, Receipt};
use crate::token::{Fault, Token, up};

#[derive(Clone, Debug)]
pub struct Pack {
    pub chain_id: u64,
    pub instance: String,
    pub case_id: String,
    pub link: Link,
    pub listing: Listing,
    pub receipt: Receipt,
    pub bound: Bound,
    pub attachments: Vec<Attachment>,
}

#[derive(Clone, Debug)]
pub struct Attachment {
    pub sha256: String,
    pub text_md: String,
}

const KEYS: [&str; 8] = ["v", "chainId", "instance", "caseId", "link", "listing", "receipt", "attachments"];

fn bytes_of(v: &k1::Value) -> Vec<u8> {
    k1::canon::canonical(v).unwrap_or_default()
}

pub fn pack(b: &[u8]) -> Result<Pack, Fault> {
    let v = k1::canon::accept(b).map_err(|x| Fault::new(up(k2::token::canon(x))))?;
    let Some(m) = form::as_obj(&v) else {
        return Err(Fault::new(Token::Pack));
    };
    if KEYS[..7].iter().any(|k| !m.iter().any(|(key, _)| key == k)) {
        return Err(Fault::new(Token::PackMissing));
    }
    if m.iter().any(|(key, _)| !KEYS.contains(&key.as_str())) {
        return Err(Fault::new(Token::PackClosed));
    }
    let g = |k: &str| v.get(k).expect("必有成员已查过");
    if form::as_str(g("v")) != Some(lit::DOC_PACK) {
        return Err(Fault::new(Token::PackV));
    }
    let Some(chain_id) = form::as_int(g("chainId")) else {
        return Err(Fault::new(Token::PackChain));
    };
    if !form::as_str(g("instance")).is_some_and(k1::hexform::is_hex20) {
        return Err(Fault::new(Token::PackInstance));
    }
    if !form::is_hex32(g("caseId")) {
        return Err(Fault::new(Token::PackCase));
    }

    if form::as_obj(g("link")).is_none() {
        return Err(Fault::new(Token::PackLink).own(Fault::new(Token::Link)));
    }
    let ln = link::link(&bytes_of(g("link"))).map_err(|f| Fault::new(Token::PackLink).own(f))?;
    if form::as_obj(g("listing")).is_none() {
        return Err(Fault::new(Token::PackListing).own(Fault::new(Token::Listing)));
    }
    let listing_bytes = bytes_of(g("listing"));
    let s = listing::listing(&listing_bytes).map_err(|f| Fault::new(Token::PackListing).own(f))?;
    if k1::hash::hex_encode(&k1::hash::sha256(&listing_bytes)) != ln.hash {
        return Err(Fault::new(Token::PackHash));
    }
    if form::as_obj(g("receipt")).is_none() {
        return Err(Fault::new(Token::PackReceipt).own(Fault::new(Token::Receipt)));
    }
    let r = receipt::receipt_under(&s.logic, &bytes_of(g("receipt"))).map_err(|f| Fault::new(Token::PackReceipt).own(f))?;
    let bound = receipt::bind_receipt(&s, &r).map_err(|verd| Fault::new(Token::PackOrder).verdict(verd))?;

    if chain_id != s.chain_id || chain_id != ln.chain_id {
        return Err(Fault::new(Token::PackChainMismatch));
    }
    let instance = form::as_str(g("instance")).expect("已过形").to_string();
    if instance != bound.instance {
        return Err(Fault::new(Token::PackInstanceMismatch));
    }
    let case_id = form::as_str(g("caseId")).expect("已过形").to_string();
    if case_id != k1::hash::hex_encode(&bound.salt) {
        return Err(Fault::new(Token::PackCaseMismatch));
    }

    let mut attachments = Vec::new();
    if let Some(av) = v.get("attachments") {
        let Some(a) = form::as_arr(av) else {
            return Err(Fault::new(Token::PackAttachments));
        };
        let named = s.named_hashes();
        for (i, el) in a.iter().enumerate() {
            attachments.push(attachment(el, i, &named)?);
        }
    }

    Ok(Pack { chain_id, instance, case_id, link: ln, listing: s, receipt: r, bound, attachments })
}


fn attachment(el: &k1::Value, i: usize, named: &[String]) -> Result<Attachment, Fault> {
    let shaped = form::as_obj(el).is_some_and(|m| {
        m.len() == 2 && m.iter().any(|(k, _)| k == "sha256") && m.iter().any(|(k, _)| k == "text_md")
    });
    let parts = shaped
        .then(|| (el.get("sha256").and_then(form::as_str), el.get("text_md").and_then(form::as_str)))
        .and_then(|(h, t)| Some((h?, t?)))
        .filter(|(h, _)| k1::hexform::is_hex32(h));
    let Some((hash, text)) = parts else {
        return Err(Fault::at(Token::PackAttachment, i));
    };
    if k1::hash::hex_encode(&k1::hash::sha256(text.as_bytes())) != hash {
        return Err(Fault::at(Token::PackAttachmentHash, i));
    }
    if !named.iter().any(|n| n == hash) {
        return Err(Fault::at(Token::PackAttachmentUnnamed, i));
    }
    Ok(Attachment { sha256: hash.to_string(), text_md: text.to_string() })
}
