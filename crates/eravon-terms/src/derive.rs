use eravon_galeed as k1;
use eravon_literals::constants::Shape;
use eravon_literals::literals as lit;

use crate::doc::{Rung, Terms};
use crate::form;
use crate::token::{Fault, Token};
use crate::u256::U256;

fn word_addr(hex20: &str) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[12..].copy_from_slice(&k1::hexform::decode_form(hex20));
    w
}

fn word_u64(x: u64) -> [u8; 32] {
    U256::from_u64(x).to_be()
}

fn word_bool(b: bool) -> [u8; 32] {
    word_u64(u64::from(b))
}

pub fn keccak(b: &[u8]) -> [u8; 32] {
    k1::hash::keccak256(b)
}

pub fn type_hash(s: &str) -> [u8; 32] {
    keccak(s.as_bytes())
}

#[derive(Clone, Debug)]
pub struct Offer {
    pub seller_key: String,
    pub judge: String,
    pub guild: String,
    pub seller_payout: String,
    pub seller_fund: String,
    pub token: String,
    pub amount: U256,
    pub delivery_window: u64,
    pub confirm_window: u64,
    pub open_window: u64,
    pub buyer_may_open: bool,
    pub ladder: Vec<Rung>,
    pub terms_hash: [u8; 32],
    pub amanah_hash: [u8; 32],
    pub quote_hash: [u8; 32],
    pub nonce: u64,
    pub expiry: u64,
}

pub fn zero_address() -> String {
    k1::hash::hex_encode(&[0u8; 20])
}

impl Offer {
    pub fn struct_hash(&self) -> [u8; 32] {
        let mut rungs: Vec<u8> = Vec::with_capacity(self.ladder.len() * 32);
        for r in &self.ladder {
            rungs.extend_from_slice(&rung_struct_hash(r));
        }
        let mut pre: Vec<u8> = Vec::with_capacity(18 * 32);
        pre.extend_from_slice(&type_hash(lit::TYPE_OFFER));
        pre.extend_from_slice(&word_addr(&self.seller_key));
        pre.extend_from_slice(&word_addr(&self.judge));
        pre.extend_from_slice(&word_addr(&self.guild));
        pre.extend_from_slice(&word_addr(&self.seller_payout));
        pre.extend_from_slice(&word_addr(&self.seller_fund));
        pre.extend_from_slice(&word_addr(&self.token));
        pre.extend_from_slice(&self.amount.to_be());
        pre.extend_from_slice(&word_u64(self.delivery_window));
        pre.extend_from_slice(&word_u64(self.confirm_window));
        pre.extend_from_slice(&word_u64(self.open_window));
        pre.extend_from_slice(&word_bool(self.buyer_may_open));
        pre.extend_from_slice(&k1::hash::keccak256(&rungs));
        pre.extend_from_slice(&self.terms_hash);
        pre.extend_from_slice(&self.amanah_hash);
        pre.extend_from_slice(&self.quote_hash);
        pre.extend_from_slice(&word_u64(self.nonce));
        pre.extend_from_slice(&word_u64(self.expiry));
        k1::hash::keccak256(&pre)
    }

    pub fn offer_digest(&self, chain_id: u64, logic: &str) -> [u8; 32] {
        digest(chain_id, logic, &self.struct_hash())
    }
}

pub fn rung_struct_hash(r: &Rung) -> [u8; 32] {
    let mut pre = Vec::with_capacity(4 * 32);
    pre.extend_from_slice(&type_hash(lit::TYPE_OUTCOME));
    pre.extend_from_slice(&word_u64(r.buyer_bps));
    pre.extend_from_slice(&word_u64(r.seller_bps));
    pre.extend_from_slice(&word_u64(r.fee_bearer));
    k1::hash::keccak256(&pre)
}

pub fn domain(chain_id: u64, verifying: &str) -> [u8; 32] {
    let mut pre = Vec::with_capacity(5 * 32);
    pre.extend_from_slice(&type_hash(lit::TYPE_EIP712_DOMAIN));
    pre.extend_from_slice(&k1::hash::keccak256(lit::DOMAIN_NAME.as_bytes()));
    pre.extend_from_slice(&k1::hash::keccak256(lit::DOMAIN_VERSION.as_bytes()));
    pre.extend_from_slice(&word_u64(chain_id));
    pre.extend_from_slice(&word_addr(verifying));
    k1::hash::keccak256(&pre)
}


pub fn digest(chain_id: u64, verifying: &str, struct_hash: &[u8; 32]) -> [u8; 32] {
    let mut pre = Vec::with_capacity(2 + 64);
    pre.extend_from_slice(&k1::hexform::decode_form(lit::EIP712_DIGEST_PREFIX));
    pre.extend_from_slice(&domain(chain_id, verifying));
    pre.extend_from_slice(struct_hash);
    k1::hash::keccak256(&pre)
}

pub fn initcode(logic: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(55);
    v.extend_from_slice(&k1::hexform::decode_form(lit::PROXY_INITCODE_PREFIX));
    v.extend_from_slice(&k1::hexform::decode_form(logic));
    v.extend_from_slice(&k1::hexform::decode_form(lit::PROXY_INITCODE_SUFFIX));
    v
}

pub fn runtime(logic: &str) -> Vec<u8> {
    initcode(logic)[10..].to_vec()
}

pub fn proxy_code_hash(logic: &str) -> [u8; 32] {
    k1::hash::keccak256(&runtime(logic))
}

pub fn create2(deployer: &str, salt: &[u8; 32], initcode_hash: &[u8; 32]) -> String {
    let mut pre = Vec::with_capacity(1 + 20 + 32 + 32);
    create2_lead(&mut pre);
    pre.extend_from_slice(&k1::hexform::decode_form(deployer));
    pre.extend_from_slice(salt);
    pre.extend_from_slice(initcode_hash);
    let h = k1::hash::keccak256(&pre);
    k1::hash::hex_encode(&h[12..])
}

fn create2_lead(pre: &mut Vec<u8>) {
    pre.push(0xff);
}


pub fn deployer_hex20() -> String {
    let d = eravon_literals::constants::PUBLIC_DEPLOYER;
    let bytes = k1::hexform::decode_digits(&d.as_bytes()[2..]).expect("定案常数里是四十位十六进制");
    k1::hash::hex_encode(&bytes)
}

pub fn offer(t: &Terms, c: u64) -> Result<Offer, Fault> {
    if c >= (t.judges.len() + t.guilds.len()) as u64 {
        return Err(Fault::new(Token::OfferCandidate));
    }
    Ok(build_offer(t, c))
}


pub fn offer_digest_at(t: &Terms, c: u64) -> Result<[u8; 32], Fault> {
    Ok(offer(t, c)?.offer_digest(t.chain_id, &t.logic))
}

fn candidate_fields(t: &Terms, c: u64) -> (String, String, String, String) {
    let ci = c as usize;
    let zero = zero_address();
    if ci < t.judges.len() {
        let j = &t.judges[ci];
        (j.judge.clone(), zero, j.amanah_hash.clone(), j.quote_hash.clone())
    } else {
        let g = &t.guilds[ci - t.judges.len()];
        (zero, g.address.clone(), g.amanah_hash.clone(), g.quote_hash.clone())
    }
}

fn hash32(hex32: &str) -> [u8; 32] {
    let mut h = [0u8; 32];
    h.copy_from_slice(&k1::hexform::decode_form(hex32));
    h
}

fn build_offer(t: &Terms, c: u64) -> Offer {
    let (judge, guild, amanah, quote) = candidate_fields(t, c);
    Offer {
        seller_key: t.seller_key.clone(),
        judge,
        guild,
        seller_payout: t.seller_payout.clone(),
        seller_fund: t.seller_fund.clone(),
        token: t.token.clone(),
        amount: t.amount,
        delivery_window: t.delivery_window,
        confirm_window: t.confirm_window,
        open_window: t.open_window,
        buyer_may_open: t.buyer_may_open,
        ladder: t.ladder.clone(),
        terms_hash: t.terms_hash,
        amanah_hash: hash32(&amanah),
        quote_hash: hash32(&quote),
        nonce: t.nonce,
        expiry: t.expiry,
    }
}

#[derive(Clone, Debug)]
pub struct Order {
    pub salt: [u8; 32],
    pub instance: String,
    pub order_digest: [u8; 32],
}

pub fn order_type(shape: Shape) -> String {
    match shape {
        Shape::Current => lit::TYPE_ORDER_TERMS.to_string(),
        Shape::Trial => lit::TYPE_ORDER_TERMS.replacen(",address prev)", ")", 1),
    }
}

pub fn order(
    o: &Offer,
    chain_id: u64,
    logic: &str,
    buyer_key: &str,
    buyer_payout: &str,
    open_by: u64,
    prev: &str,
) -> Result<Order, Fault> {
    let shape = eravon_literals::constants::generation_of_logic(logic).map_or(Shape::Current, |g| g.shape());
    order_of(o, chain_id, logic, buyer_key, buyer_payout, open_by, prev, shape)
}

#[allow(clippy::too_many_arguments)]
pub fn order_of(
    o: &Offer,
    chain_id: u64,
    logic: &str,
    buyer_key: &str,
    buyer_payout: &str,
    open_by: u64,
    prev: &str,
    shape: Shape,
) -> Result<Order, Fault> {
    if !k1::hexform::is_hex20(buyer_key)
        || form::is_zero_hex(buyer_key)
        || buyer_key == o.seller_key
        || buyer_key == o.judge
        || buyer_key == o.guild
    {
        return Err(Fault::new(Token::OrderBuyer));
    }
    if !k1::hexform::is_hex20(buyer_payout) || form::is_zero_hex(buyer_payout) {
        return Err(Fault::new(Token::OrderPayout));
    }
    if open_by > form::INT_MAX {
        return Err(Fault::new(Token::OrderOpenBy));
    }
    if !k1::hexform::is_hex20(prev) || (shape == Shape::Trial && !form::is_zero_hex(prev)) {
        return Err(Fault::new(Token::OrderPrev));
    }
    let mut pre = Vec::with_capacity(6 * 32);
    pre.extend_from_slice(&type_hash(&order_type(shape)));
    pre.extend_from_slice(&o.struct_hash());
    pre.extend_from_slice(&word_addr(buyer_key));
    pre.extend_from_slice(&word_addr(buyer_payout));
    pre.extend_from_slice(&word_u64(open_by));
    if shape == Shape::Current {
        pre.extend_from_slice(&word_addr(prev));
    }
    let salt = k1::hash::keccak256(&pre);
    let ich = k1::hash::keccak256(&initcode(logic));
    let instance = create2(&deployer_hex20(), &salt, &ich);
    let order_digest = digest(chain_id, &instance, &salt);
    Ok(Order { salt, instance, order_digest })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    ClaimDelivery {
        manifest: String,
    },
    Confirm,
    Dispute,
    Refund,
    Settle {
        buyer_bps: u64,
        expiry: U256,
    },
    Cancel,
}

impl Action {
    pub fn name(&self) -> &'static str {
        match self {
            Action::ClaimDelivery { .. } => "ClaimDelivery",
            Action::Confirm => "Confirm",
            Action::Dispute => "Dispute",
            Action::Refund => "Refund",
            Action::Settle { .. } => "Settle",
            Action::Cancel => "Cancel",
        }
    }

    fn type_string(&self) -> &'static str {
        match self {
            Action::ClaimDelivery { .. } => lit::TYPE_CLAIM_DELIVERY,
            Action::Confirm => lit::TYPE_CONFIRM,
            Action::Dispute => lit::TYPE_DISPUTE,
            Action::Refund => lit::TYPE_REFUND,
            Action::Settle { .. } => lit::TYPE_SETTLE,
            Action::Cancel => lit::TYPE_CANCEL,
        }
    }

    pub fn admits(&self, party: Party) -> bool {
        match self {
            Action::ClaimDelivery { .. } | Action::Refund => party == Party::Seller,
            Action::Confirm | Action::Dispute | Action::Cancel => party == Party::Buyer,
            Action::Settle { .. } => true,
        }
    }

    pub fn admits_call(&self) -> bool {
        match self {
            Action::Settle { .. } => true,
            Action::ClaimDelivery { .. } | Action::Confirm | Action::Dispute | Action::Refund | Action::Cancel => false,
        }
    }

    pub fn struct_hash(&self) -> [u8; 32] {
        let mut pre = Vec::with_capacity(3 * 32);
        pre.extend_from_slice(&type_hash(self.type_string()));
        match self {
            Action::ClaimDelivery { manifest } => {
                pre.extend_from_slice(&hex32_bytes(manifest));
            }
            Action::Confirm | Action::Dispute | Action::Refund | Action::Cancel => {}
            Action::Settle { buyer_bps, expiry } => {
                pre.extend_from_slice(&word_u64(*buyer_bps));
                pre.extend_from_slice(&expiry.to_be());
            }
        }
        k1::hash::keccak256(&pre)
    }
}

fn hex32_bytes(s: &str) -> Vec<u8> {
    if k1::hexform::is_hex32(s) { k1::hexform::decode_form(s) } else { Vec::new() }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Party {
    Buyer,
    Seller,
}

pub fn check_action(a: &Action) -> Result<(), Fault> {
    match a {
        Action::ClaimDelivery { manifest } => {
            if !k1::hexform::is_hex32(manifest) || form::is_zero_hex(manifest) {
                return Err(Fault::new(Token::ActionManifest));
            }
        }
        Action::Settle { buyer_bps, .. } => {
            if *buyer_bps > 10000 {
                return Err(Fault::new(Token::ActionBps));
            }
        }
        Action::Confirm | Action::Dispute | Action::Refund | Action::Cancel => {}
    }
    Ok(())
}


#[cfg(test)]
mod settle_width_tests {
    use super::*;

    #[test]
    fn the_widest_expiry_forms_a_digest() {
        let h = Action::Settle { buyer_bps: 5000, expiry: U256::MAX }.struct_hash();
        assert_eq!(k1::hash::hex_encode(&h), "0xe62f33754950a65e4116fdf0baef6b1b71bbadede0e5fd7ecf7c3bf392cf6188");
    }
}

#[cfg(test)]
mod generation_tests {
    #[test]
    fn the_trial_order_type_is_the_frozen_contracts() {
        let src = include_str!("../../eravon-standard/past/trial/src/Eravon.sol");
        let frozen = src.split("TH_ORDER = keccak256(").nth(1).and_then(|x| x.split('"').nth(1)).expect("冻结源里有 TH_ORDER");
        assert_eq!(super::order_type(eravon_literals::constants::Shape::Trial), frozen);
        assert_ne!(super::order_type(eravon_literals::constants::Shape::Current), frozen);
    }
}
