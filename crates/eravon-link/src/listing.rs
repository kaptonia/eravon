use eravon_galeed as k1;
use eravon_literals::literals as lit;
use eravon_terms as k2;
use k2::doc::{GuildEl, JudgeEl, Rung, Terms};
use k2::form;
use k2::token::Fault as TFault;
use k2::u256::U256;

use crate::token::{Fault, Token, up};

#[derive(Clone, Debug)]
pub struct Listing {
    pub chain_id: u64,
    pub token: String,
    pub logic: String,
    pub terms_hash: String,
    pub terms_text: String,
    pub terms: Terms,
    pub order: OrderMembers,
    pub judges: Vec<JudgeEl>,
    pub judge_sigs: Vec<String>,
    pub guilds: Vec<GuildEl>,
    pub guild_seqs: Vec<u64>,
    pub guild_panels: Vec<String>,
    pub guild_sigs: Vec<String>,
    pub by_call: Vec<(usize, String)>,
    pub bytes: Vec<u8>,
    pub value: k1::Value,
}

#[derive(Clone, Debug)]
pub struct OrderMembers {
    pub seller_key: String,
    pub seller_payout: String,
    pub seller_fund: String,
    pub amount: U256,
    pub delivery_window: u64,
    pub confirm_window: u64,
    pub open_window: u64,
    pub buyer_may_open: bool,
    pub ladder: Vec<Rung>,
    pub nonce: u64,
    pub expiry: u64,
}

const KEYS: [&str; 9] = ["v", "chainId", "token", "logic", "proxyCodeHash", "terms", "order", "judges", "guilds"];

const ORDER_KEYS: [&str; 11] = [
    "sellerKey",
    "sellerPayout",
    "sellerFund",
    "amount",
    "deliveryWindow",
    "confirmWindow",
    "openWindow",
    "buyerMayOpen",
    "ladder",
    "nonce",
    "expiry",
];

pub(crate) fn pass_up(f: TFault) -> Fault {
    let mut out = Fault { token: up(f.token), index: f.index, inner: None };
    if let Some(inner) = f.inner {
        out = out.up(*inner);
    }
    out
}

pub fn listing(b: &[u8]) -> Result<Listing, Fault> {
    let v = k1::canon::accept(b).map_err(|x| Fault::new(up(k2::token::canon(x))))?;
    let Some(m) = form::as_obj(&v) else {
        return Err(Fault::new(Token::Listing));
    };
    if KEYS.iter().any(|k| !m.iter().any(|(key, _)| key == k)) {
        return Err(Fault::new(Token::ListingMissing));
    }
    if m.iter().any(|(key, _)| !KEYS.contains(&key.as_str())) {
        return Err(Fault::new(Token::ListingClosed));
    }
    let get = |k: &str| v.get(k).expect("必有成员已查过");

    if form::as_str(get("v")) != Some(lit::DOC_LISTING) {
        return Err(Fault::new(Token::ListingV));
    }
    let Some(chain_id) = form::as_int(get("chainId")) else {
        return Err(Fault::new(Token::ListingChain));
    };
    if !form::is_nonzero_hex20(get("token")) {
        return Err(Fault::new(Token::ListingToken));
    }
    if !form::is_nonzero_hex20(get("logic")) {
        return Err(Fault::new(Token::ListingLogic));
    }
    let logic = form::as_str(get("logic")).expect("已过形").to_string();
    let pch = k1::hash::hex_encode(&k2::derive::proxy_code_hash(&logic));
    if !form::is_hex32(get("proxyCodeHash")) || form::as_str(get("proxyCodeHash")) != Some(pch.as_str()) {
        return Err(Fault::new(Token::ListingCodehash));
    }

    let (terms_hash, terms_text, terms) = read_terms(get("terms"))?;
    let order = read_order(get("order"))?;

    let Some(ja) = form::as_arr(get("judges")) else {
        return Err(Fault::new(Token::ListingJudges));
    };
    let mut judges = Vec::with_capacity(ja.len());
    let mut judge_sigs = Vec::with_capacity(ja.len());
    for (i, el) in ja.iter().enumerate() {
        let j = k2::doc::judge_element(el, i, &order.seller_key, chain_id, &["sig"], &[]).map_err(pass_up)?;
        let Some(sig) = form::as_str(el.get("sig").expect("必有")).filter(|s| is_slot(s)) else {
            return Err(Fault::at(Token::JudgeSig, i));
        };
        judges.push(j);
        judge_sigs.push(sig.to_string());
    }
    for i in 0..judges.len() {
        if judges[..i].iter().any(|e| e.judge == judges[i].judge) {
            return Err(Fault::at(Token::ListingDupJudge, i));
        }
    }

    let Some(ga) = form::as_arr(get("guilds")) else {
        return Err(Fault::new(Token::ListingGuilds));
    };
    let mut guilds = Vec::with_capacity(ga.len());
    let (mut guild_seqs, mut guild_panels, mut guild_sigs) = (Vec::new(), Vec::new(), Vec::new());
    for (i, el) in ga.iter().enumerate() {
        let g = k2::doc::guild_element(el, i, &order.seller_key, chain_id, &["seq", "panelHash", "sig"], &[])
            .map_err(pass_up)?;
        let Some(seq) = form::as_int(el.get("seq").expect("必有")) else {
            return Err(Fault::at(Token::GuildSeq, i));
        };
        if !form::is_nonzero_hex32(el.get("panelHash").expect("必有")) {
            return Err(Fault::at(Token::GuildPanel, i));
        }
        let Some(sig) = form::as_str(el.get("sig").expect("必有")).filter(|s| is_slot(s)) else {
            return Err(Fault::at(Token::GuildSig, i));
        };
        guilds.push(g);
        guild_seqs.push(seq);
        guild_panels.push(form::as_str(el.get("panelHash").expect("必有")).expect("已过形").to_string());
        guild_sigs.push(sig.to_string());
    }
    for i in 0..guilds.len() {
        if guilds[..i].iter().any(|e| e.address == guilds[i].address) {
            return Err(Fault::at(Token::ListingDupGuild, i));
        }
    }
    if judges.is_empty() && guilds.is_empty() {
        return Err(Fault::new(Token::ListingNoCandidate));
    }

    let mut s = Listing {
        chain_id,
        token: form::as_str(get("token")).expect("已过形").to_string(),
        logic,
        terms_hash,
        terms_text,
        terms,
        order,
        judges,
        judge_sigs,
        guilds,
        guild_seqs,
        guild_panels,
        guild_sigs,
        by_call: Vec::new(),
        bytes: b.to_vec(),
        value: v.clone(),
    };
    crate::bind::compare(&s, &s.terms).map_err(|verd| Fault::new(Token::ListingBind).verdict(verd))?;
    s.by_call = check_candidates(&s)?;
    Ok(s)
}


fn read_terms(v: &k1::Value) -> Result<(String, String, Terms), Fault> {
    let shaped = form::as_obj(v).is_some_and(|m| {
        m.len() == 2 && m.iter().any(|(k, _)| k == "hash") && m.iter().any(|(k, _)| k == "inline_md")
    });
    if !shaped {
        return Err(Fault::new(Token::ListingTerms));
    }
    let hash = v.get("hash").expect("已查过");
    let Some(text) = form::as_str(v.get("inline_md").expect("已查过")).filter(|_| form::is_nonzero_hex32(hash)) else {
        return Err(Fault::new(Token::ListingTerms));
    };
    let hash = form::as_str(hash).expect("已过形").to_string();
    if k1::hash::hex_encode(&k1::hash::sha256(text.as_bytes())) != hash {
        return Err(Fault::new(Token::ListingTermsHash));
    }
    let terms = k2::doc::terms(text.as_bytes()).map_err(|f| Fault::new(Token::ListingTermsDoc).up(f))?;
    Ok((hash, text.to_string(), terms))
}

fn read_order(v: &k1::Value) -> Result<OrderMembers, Fault> {
    let Some(m) = form::as_obj(v) else {
        return Err(Fault::new(Token::ListingOrderMissing));
    };
    if ORDER_KEYS.iter().any(|k| !m.iter().any(|(key, _)| key == k)) {
        return Err(Fault::new(Token::ListingOrderMissing));
    }
    if m.iter().any(|(key, _)| !ORDER_KEYS.contains(&key.as_str())) {
        return Err(Fault::new(Token::ListingOrderClosed));
    }
    let g = |k: &str| v.get(k).expect("已查过");
    for (k, t) in [
        ("sellerKey", Token::ListingSeller),
        ("sellerPayout", Token::ListingPayout),
        ("sellerFund", Token::ListingFund),
    ] {
        if !form::is_nonzero_hex20(g(k)) {
            return Err(Fault::new(t));
        }
    }
    let amount = form::as_str(g("amount"))
        .and_then(U256::from_dec)
        .filter(|x| !x.is_zero())
        .ok_or_else(|| Fault::new(Token::ListingAmount))?;
    for (k, t) in [
        ("deliveryWindow", Token::ListingDeliveryWindow),
        ("confirmWindow", Token::ListingConfirmWindow),
        ("openWindow", Token::ListingOpenWindow),
    ] {
        if !form::is_u32(g(k)) {
            return Err(Fault::new(t));
        }
    }
    let Some(buyer_may_open) = form::as_bool(g("buyerMayOpen")) else {
        return Err(Fault::new(Token::ListingBuyerOpen));
    };
    let Some(la) = form::as_arr(g("ladder")).filter(|a| (2..=16).contains(&a.len())) else {
        return Err(Fault::new(Token::ListingLadder));
    };
    let mut ladder = Vec::with_capacity(la.len());
    for (i, el) in la.iter().enumerate() {
        ladder.push(k2::doc::read_rung_with(el, i, false).map_err(pass_up)?);
    }
    k2::doc::ladder_wide(&ladder).map_err(pass_up)?;
    let Some(nonce) = form::as_int(g("nonce")) else {
        return Err(Fault::new(Token::ListingNonce));
    };
    let Some(expiry) = form::as_int(g("expiry")) else {
        return Err(Fault::new(Token::ListingExpiry));
    };
    let text = |k: &str| form::as_str(g(k)).expect("已过形").to_string();
    let int = |k: &str| form::as_int(g(k)).expect("已过形");
    Ok(OrderMembers {
        seller_key: text("sellerKey"),
        seller_payout: text("sellerPayout"),
        seller_fund: text("sellerFund"),
        amount,
        delivery_window: int("deliveryWindow"),
        confirm_window: int("confirmWindow"),
        open_window: int("openWindow"),
        buyer_may_open,
        ladder,
        nonce,
        expiry,
    })
}

impl Listing {
    pub fn candidate_count(&self) -> usize {
        self.judges.len() + self.guilds.len()
    }

    pub fn candidate_offer(&self, c: usize) -> k2::derive::Offer {
        let zero = k2::derive::zero_address();
        let (judge, guild, amanah, quote) = if c < self.judges.len() {
            let j = &self.judges[c];
            (j.judge.clone(), zero, j.amanah_hash.clone(), j.quote_hash.clone())
        } else {
            let g = &self.guilds[c - self.judges.len()];
            (zero, g.address.clone(), g.amanah_hash.clone(), g.quote_hash.clone())
        };
        let word = |h: &str| {
            let mut w = [0u8; 32];
            w.copy_from_slice(&k1::hexform::decode_form(h));
            w
        };
        let o = &self.order;
        k2::derive::Offer {
            seller_key: o.seller_key.clone(),
            judge,
            guild,
            seller_payout: o.seller_payout.clone(),
            seller_fund: o.seller_fund.clone(),
            token: self.token.clone(),
            amount: o.amount,
            delivery_window: o.delivery_window,
            confirm_window: o.confirm_window,
            open_window: o.open_window,
            buyer_may_open: o.buyer_may_open,
            ladder: o.ladder.clone(),
            terms_hash: word(&self.terms_hash),
            amanah_hash: word(&amanah),
            quote_hash: word(&quote),
            nonce: o.nonce,
            expiry: o.expiry,
        }
    }

    pub fn candidate_sig(&self, c: usize) -> &str {
        if c < self.judges.len() { &self.judge_sigs[c] } else { &self.guild_sigs[c - self.judges.len()] }
    }

    pub fn named_hashes(&self) -> Vec<String> {
        ["goodsHash", "deliveryHash"]
            .iter()
            .filter_map(|k| self.terms.block.get(k).and_then(form::as_str).map(str::to_string))
            .collect()
    }
}

pub fn is_slot(s: &str) -> bool {
    k1::hexform::is_hex65(s) || k2::sig::is_empty_slot(s)
}

fn check_candidates(s: &Listing) -> Result<Vec<(usize, String)>, Fault> {
    let mut by_call = Vec::new();
    for c in 0..s.candidate_count() {
        let d = s.candidate_offer(c).offer_digest(s.chain_id, &s.logic);
        match k2::sig::verify_seller(&d, s.candidate_sig(c), &s.order.seller_key) {
            Ok((k2::sig::Form::Call, _)) => by_call.push((c, k1::hash::hex_encode(&d))),
            Ok(_) => {}
            Err(f) => return Err(Fault::at(Token::ListingSig, c).up(f)),
        }
    }
    Ok(by_call)
}

