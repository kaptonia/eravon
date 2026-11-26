use crate::form;
use crate::token::{Fault, Token};
use crate::u256::U256;

pub const LEN: usize = WORDS * 32;
pub const WORDS: usize = 15;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quote {
    pub token: String,
    pub premium_bps: u64,
    pub fee_unit: String,
    pub feed_token: String,
    pub feed_unit: String,
    pub max_stale: u64,
    pub fee_fixed: U256,
    pub fee_bps: u64,
    pub fee_min: U256,
    pub fee_max: U256,
    pub amount_cap: U256,
    pub ruling_window: u64,
    pub offer_window: u64,
    pub lapse_default: u64,
    pub auto_accept: bool,
    pub bytes: Vec<u8>,
}

fn word(b: &[u8], i: usize) -> U256 {
    let mut w = [0u8; 32];
    w.copy_from_slice(&b[i * 32..i * 32 + 32]);
    U256::from_be(&w)
}

fn is_address_word(b: &[u8], i: usize) -> bool {
    b[i * 32..i * 32 + 12].iter().all(|x| *x == 0)
}

fn address_of(b: &[u8], i: usize) -> String {
    eravon_galeed::hash::hex_encode(&b[i * 32 + 12..i * 32 + 32])
}

fn at_most(b: &[u8], i: usize, bound: u64) -> bool {
    let w = word(b, i);
    w.cmp_u(&U256::from_u64(bound)) != core::cmp::Ordering::Greater
}

fn word_ok(b: &[u8], i: usize) -> bool {
    match i {
        0 => is_address_word(b, 0) && !word(b, 0).is_zero(),
        2..=4 => is_address_word(b, i),
        1 | 7 => at_most(b, i, 10000),
        5 | 11 | 12 => at_most(b, i, form::U32_MAX),
        13 => at_most(b, 13, 2),
        14 => at_most(b, 14, 1),
        _ => true,
    }
}

pub fn decode(b: &[u8]) -> Result<Quote, Fault> {
    if b.len() != LEN {
        return Err(Fault::new(Token::QuoteLen));
    }
    for i in 0..WORDS {
        if !word_ok(b, i) {
            return Err(Fault::field(Token::QuoteField, i));
        }
    }
    let fee_unit = word(b, 2);
    if fee_unit.is_zero() && !(word(b, 3).is_zero() && word(b, 4).is_zero()) {
        return Err(Fault::new(Token::QuoteFeed));
    }
    Ok(build(b))
}


pub const TRIAL_WORDS: usize = 16;

pub fn decode_under(shape: eravon_literals::constants::Shape, b: &[u8]) -> Result<Quote, Fault> {
    if shape != eravon_literals::constants::Shape::Trial {
        return decode(b);
    }
    if b.len() != TRIAL_WORDS * 32 {
        return Err(Fault::new(Token::QuoteLen));
    }
    if !is_address_word(b, 1) {
        return Err(Fault::field(Token::QuoteField, 1));
    }
    let mut cur = b[..32].to_vec();
    cur.extend_from_slice(&b[64..]);
    let q = decode(&cur).map_err(|f| match f.field {
        Some(i) if i >= 1 => Fault::field(Token::QuoteField, i + 1),
        _ => f,
    })?;
    if q.lapse_default == 1 {
        return Err(Fault::field(Token::QuoteField, 14));
    }
    Ok(Quote { bytes: b.to_vec(), ..q })
}

fn build(b: &[u8]) -> Quote {
    Quote {
        token: address_of(b, 0),
        premium_bps: word(b, 1).0[0],
        fee_unit: address_of(b, 2),
        feed_token: address_of(b, 3),
        feed_unit: address_of(b, 4),
        max_stale: word(b, 5).0[0],
        fee_fixed: word(b, 6),
        fee_bps: word(b, 7).0[0],
        fee_min: word(b, 8),
        fee_max: word(b, 9),
        amount_cap: word(b, 10),
        ruling_window: word(b, 11).0[0],
        offer_window: word(b, 12).0[0],
        lapse_default: word(b, 13).0[0],
        auto_accept: !word(b, 14).is_zero(),
        bytes: b.to_vec(),
    }
}

impl Quote {
    pub fn hash(&self) -> [u8; 32] {
        eravon_galeed::hash::keccak256(&self.bytes)
    }
    pub fn fee_unit_is_zero(&self) -> bool {
        form::is_zero_hex(&self.fee_unit)
    }
}
