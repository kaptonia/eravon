use eravon_literals::json::{write as canon, Out};

use crate::token::Fault;

pub struct ListingParts {
    pub chain_id: u64,
    pub token: String,
    pub logic: String,
    pub proxy_code_hash: String,
    pub terms_text: String,
    pub order: Out,
    pub judges: Vec<Out>,
    pub guilds: Vec<Out>,
}

pub fn listing(p: &ListingParts) -> Result<Vec<u8>, Fault> {
    let bytes = assemble(p);
    crate::listing::listing(&bytes)?;
    Ok(bytes)
}

pub fn assemble(p: &ListingParts) -> Vec<u8> {
    let hash = eravon_galeed::hash::hex_encode(&eravon_galeed::hash::sha256(p.terms_text.as_bytes()));
    canon(&Out::O(vec![
        ("chainId", Out::N(p.chain_id)),
        ("guilds", Out::A(p.guilds.clone())),
        ("judges", Out::A(p.judges.clone())),
        ("logic", Out::S(p.logic.clone())),
        ("order", p.order.clone()),
        ("proxyCodeHash", Out::S(p.proxy_code_hash.clone())),
        ("terms", Out::O(vec![("hash", Out::S(hash)), ("inline_md", Out::S(p.terms_text.clone()))])),
        ("token", Out::S(p.token.clone())),
        ("v", Out::S(eravon_literals::literals::DOC_LISTING.to_string())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eravon_galeed::Value;

    fn out(v: &Value) -> Out {
        match v {
            Value::Null => Out::Null,
            Value::Bool(b) => Out::B(*b),
            Value::Int(n) => Out::N(*n),
            Value::Str(s) => Out::S(s.clone()),
            Value::Arr(a) => Out::A(a.iter().map(out).collect()),
            Value::Obj(m) => Out::O(m.iter().map(|(k, x)| (&*Box::leak(k.clone().into_boxed_str()), out(x))).collect()),
        }
    }

    fn parts_of(b: &[u8]) -> ListingParts {
        let v = eravon_galeed::canon::accept(b).expect("正典");
        let s = |k: &str| v.get(k).and_then(|x| match x {
            Value::Str(s) => Some(s.clone()),
            _ => None,
        });
        let list = |k: &str| match v.get(k) {
            Some(Value::Arr(a)) => a.iter().map(out).collect(),
            _ => panic!("{k} 不是数组"),
        };
        let terms = v.get("terms").expect("terms");
        ListingParts {
            chain_id: match v.get("chainId") {
                Some(Value::Int(n)) => *n,
                _ => panic!("chainId"),
            },
            token: s("token").expect("token"),
            logic: s("logic").expect("logic"),
            proxy_code_hash: s("proxyCodeHash").expect("proxyCodeHash"),
            terms_text: match terms.get("inline_md") {
                Some(Value::Str(t)) => t.clone(),
                _ => panic!("inline_md"),
            },
            order: out(v.get("order").expect("order")),
            judges: list("judges"),
            guilds: list("guilds"),
        }
    }

    fn corpus_listing() -> Vec<u8> {
        let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../base/eravon-conformance/link-corpus/listings/l001_ok_full.json");
        std::fs::read(&at).expect("语料清单读得出")
    }

    #[test]
    fn the_gate_rebuilds_a_corpus_listing_byte_for_byte() {
        let b = corpus_listing();
        crate::listing::listing(&b).expect("语料清单本身合格");
        assert_eq!(listing(&parts_of(&b)).expect("闸门收"), b);
    }

    #[test]
    fn the_gate_refuses_what_does_not_read_back() {
        let mut p = parts_of(&corpus_listing());
        p.terms_text.push('x');
        assert_eq!(listing(&p).unwrap_err().token, crate::token::Token::ListingTermsDoc);
    }
}
