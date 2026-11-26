use eravon_galeed as k1;
use eravon_literals::literals as lit;
use eravon_terms as k2;
use k2::form;

use crate::listing::Listing;
use crate::token::{Fault, Token, Verdict, up};

#[derive(Clone, Debug)]
pub struct Receipt {
    pub terms_hash: String,
    pub candidate: u64,
    pub buyer_key: String,
    pub buyer_payout: String,
    pub open_by: u64,
    pub prev: String,
    pub sig: k1::Value,
    pub seq: Option<u64>,
    pub bytes: Vec<u8>,
}

const ZERO_PREV: &str = "0x0000000000000000000000000000000000000000";

const KEYS: [&str; 9] = ["v", "termsHash", "candidate", "buyerKey", "buyerPayout", "openBy", "prev", "sig", "seq"];

pub fn receipt(b: &[u8]) -> Result<Receipt, Fault> {
    checked(b, &KEYS, 8)
}

pub fn receipt_trial(b: &[u8]) -> Result<Receipt, Fault> {
    checked(b, &KEYS_TRIAL, 7)
}

pub fn receipt_under(logic: &str, b: &[u8]) -> Result<Receipt, Fault> {
    let trial = eravon_literals::constants::generation_of_logic(logic).is_some_and(|g| g.shape() == eravon_literals::constants::Shape::Trial);
    if !trial {
        return receipt(b);
    }
    receipt_trial(b).or_else(|e| receipt(b).ok().filter(|r| r.prev.eq_ignore_ascii_case(ZERO_PREV)).ok_or(e))
}

impl Receipt {
    pub fn current_form(&self) -> Vec<u8> {
        let Some(k1::Value::Obj(mut m)) = k1::canon::accept(&self.bytes).ok() else { return self.bytes.clone() };
        if m.iter().any(|(k, _)| k == "prev") {
            return self.bytes.clone();
        }
        m.push(("prev".into(), k1::Value::Str(self.prev.to_ascii_lowercase())));
        m.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        k1::canon::canonical(&k1::Value::Obj(m)).unwrap_or_else(|_| self.bytes.clone())
    }
}

const KEYS_TRIAL: [&str; 8] = ["v", "termsHash", "candidate", "buyerKey", "buyerPayout", "openBy", "sig", "seq"];

fn checked(b: &[u8], keys: &[&str], required: usize) -> Result<Receipt, Fault> {
    let v = k1::canon::accept(b).map_err(|x| Fault::new(up(k2::token::canon(x))))?;
    let Some(m) = form::as_obj(&v) else {
        return Err(Fault::new(Token::Receipt));
    };
    for k in &keys[..required] {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::new(Token::ReceiptMissing));
        }
    }
    if m.iter().any(|(key, _)| !keys.contains(&key.as_str())) {
        return Err(Fault::new(Token::ReceiptClosed));
    }
    let g = |k: &str| v.get(k).expect("必有成员已查过");
    if form::as_str(g("v")) != Some(lit::DOC_RECEIPT) {
        return Err(Fault::new(Token::ReceiptV));
    }
    if !form::is_nonzero_hex32(g("termsHash")) {
        return Err(Fault::new(Token::ReceiptTerms));
    }
    let Some(candidate) = form::as_int(g("candidate")) else {
        return Err(Fault::new(Token::ReceiptCandidate));
    };
    if !form::is_nonzero_hex20(g("buyerKey")) {
        return Err(Fault::new(Token::ReceiptBuyer));
    }
    if !form::is_nonzero_hex20(g("buyerPayout")) {
        return Err(Fault::new(Token::ReceiptPayout));
    }
    let Some(open_by) = form::as_int(g("openBy")) else {
        return Err(Fault::new(Token::ReceiptOpenBy));
    };
    let prev = match v.get("prev") {
        None if !keys.contains(&"prev") => ZERO_PREV,
        x => match x.and_then(form::as_str).filter(|p| k1::hexform::is_hex20(p)) {
            Some(p) => p,
            None => return Err(Fault::new(Token::ReceiptPrev)),
        },
    };
    if !slot_shape(g("sig")) {
        return Err(Fault::new(Token::ReceiptSig));
    }
    let seq = match v.get("seq") {
        None => None,
        Some(x) => match form::as_int(x) {
            None => return Err(Fault::new(Token::ReceiptSeq)),
            Some(n) => Some(n),
        },
    };
    Ok(Receipt {
        terms_hash: form::as_str(g("termsHash")).expect("已过形").to_string(),
        candidate,
        buyer_key: form::as_str(g("buyerKey")).expect("已过形").to_string(),
        buyer_payout: form::as_str(g("buyerPayout")).expect("已过形").to_string(),
        open_by,
        prev: prev.to_string(),
        sig: g("sig").clone(),
        seq,
        bytes: b.to_vec(),
    })
}


fn slot_shape(v: &k1::Value) -> bool {
    matches!(v, k1::Value::Str(s) if crate::listing::is_slot(s))
}

#[derive(Clone, Debug)]
pub struct Bound {
    pub salt: [u8; 32],
    pub instance: String,
    pub order_digest: [u8; 32],
    pub form: k2::sig::Form,
}

impl Bound {
    pub fn verdict(&self) -> &'static str {
        match self.form {
            k2::sig::Form::Call => "RECEIPT_BY_CALL",
            k2::sig::Form::Secp256k1 => "RECEIPT_BOUND",
        }
    }

    pub fn by_call(&self) -> bool {
        matches!(self.form, k2::sig::Form::Call)
    }
}

pub fn bind_receipt_bytes(listing_bytes: &[u8], receipt_bytes: &[u8]) -> Result<(Listing, Receipt, Bound), Verdict> {
    let s = crate::listing::listing(listing_bytes).map_err(|f| Verdict::new("LISTING_INVALID").own(f))?;
    let r = receipt(receipt_bytes).map_err(|f| Verdict::new("RECEIPT_INVALID").own(f))?;
    let b = bind_receipt(&s, &r)?;
    Ok((s, r, b))
}

pub fn bind_receipt(s: &Listing, r: &Receipt) -> Result<Bound, Verdict> {
    if r.terms_hash != s.terms_hash {
        return Err(Verdict::new("RECEIPT_TERMS_MISMATCH"));
    }
    if r.candidate >= s.candidate_count() as u64 {
        return Err(Verdict::new("RECEIPT_CANDIDATE_RANGE"));
    }
    let is_guild = r.candidate as usize >= s.judges.len();
    if is_guild != r.seq.is_some() {
        return Err(Verdict::new("RECEIPT_SEQ"));
    }
    let o = s.candidate_offer(r.candidate as usize);
    let ord = k2::derive::order(&o, s.chain_id, &s.logic, &r.buyer_key, &r.buyer_payout, r.open_by, &r.prev)
        .map_err(|f| Verdict::new("RECEIPT_ORDER").up(f))?;
    let form = k2::sig::verify_buyer(&ord.order_digest, &r.sig, &r.buyer_key)
        .map_err(|f| Verdict::new("RECEIPT_SIG").up(f))?;
    Ok(Bound { salt: ord.salt, instance: ord.instance, order_digest: ord.order_digest, form })
}
