use eravon_galeed as k1;
use eravon_literals::literals as lit;
use eravon_terms as k2;
use k2::form;

use crate::pack::Pack;
use crate::token::{Fault, Token, Verdict, up};

#[derive(Clone, Debug)]
pub struct Settle {
    pub chain_id: u64,
    pub instance: String,
    pub buyer_bps: u64,
    pub expiry: u64,
    pub sig: String,
    pub bytes: Vec<u8>,
}

const KEYS: [&str; 6] = ["v", "chainId", "instance", "buyerBps", "expiry", "sig"];

pub fn settle(b: &[u8]) -> Result<Settle, Fault> {
    let v = k1::canon::accept(b).map_err(|x| Fault::new(up(k2::token::canon(x))))?;
    let Some(m) = form::as_obj(&v) else {
        return Err(Fault::new(Token::Settle));
    };
    for k in KEYS {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::new(Token::SettleMissing));
        }
    }
    if m.iter().any(|(key, _)| !KEYS.contains(&key.as_str())) {
        return Err(Fault::new(Token::SettleClosed));
    }
    let g = |k: &str| v.get(k).expect("必有成员已查过");
    if form::as_str(g("v")) != Some(lit::DOC_SETTLE) {
        return Err(Fault::new(Token::SettleV));
    }
    let Some(chain_id) = form::as_int(g("chainId")) else {
        return Err(Fault::new(Token::SettleChain));
    };
    if !form::is_nonzero_hex20(g("instance")) {
        return Err(Fault::new(Token::SettleInstance));
    }
    if !form::is_int_at_most(g("buyerBps"), 10000) {
        return Err(Fault::new(Token::SettleBps));
    }
    let Some(expiry) = form::as_int(g("expiry")) else {
        return Err(Fault::new(Token::SettleExpiry));
    };
    let sig = form::as_str(g("sig")).filter(|s| crate::listing::is_slot(s));
    let Some(sig) = sig else {
        return Err(Fault::new(Token::SettleSig));
    };
    Ok(Settle {
        chain_id,
        instance: form::as_str(g("instance")).expect("已过形").to_string(),
        buyer_bps: form::as_int(g("buyerBps")).expect("已过形"),
        expiry,
        sig: sig.to_string(),
        bytes: b.to_vec(),
    })
}


#[derive(Clone, Debug)]
pub struct SettleBound {
    pub digest: [u8; 32],
    pub form: k2::sig::Form,
}

impl SettleBound {
    pub fn verdict(&self) -> &'static str {
        match self.form {
            k2::sig::Form::Call => "SETTLE_BY_CALL",
            k2::sig::Form::Secp256k1 => "SETTLE_BOUND",
        }
    }
}

pub fn bind_settle_bytes(pack_bytes: &[u8], settle_bytes: &[u8]) -> Result<SettleBound, Verdict> {
    let k = crate::pack::pack(pack_bytes).map_err(|f| Verdict::new("PACK_INVALID").own(f))?;
    let s = settle(settle_bytes).map_err(|f| Verdict::new("SETTLE_INVALID").own(f))?;
    bind_settle(&k, &s)
}

pub fn bind_settle(k: &Pack, s: &Settle) -> Result<SettleBound, Verdict> {
    bind_settle_on(k.chain_id, &k.instance, &k.listing.order.seller_key, s)
}

pub fn bind_settle_on(chain_id: u64, instance: &str, seller_key: &str, s: &Settle) -> Result<SettleBound, Verdict> {
    if s.chain_id != chain_id {
        return Err(Verdict::new("SETTLE_CHAIN"));
    }
    if !s.instance.eq_ignore_ascii_case(instance) {
        return Err(Verdict::new("SETTLE_INSTANCE"));
    }
    let a = k2::derive::Action::Settle { buyer_bps: s.buyer_bps, expiry: k2::u256::U256::from_u64(s.expiry) };
    let d = k2::derive::digest(chain_id, &s.instance, &a.struct_hash());
    let (form, _) = k2::sig::verify_seller(&d, &s.sig, seller_key)
        .map_err(|f| Verdict::new("SETTLE_SIG").up(f))?;
    Ok(SettleBound { digest: d, form })
}
