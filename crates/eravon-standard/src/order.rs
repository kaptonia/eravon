use eravon_terms as k2;

use crate::abi::Val;
use crate::fault::{Fault, R};

#[derive(Clone, Debug)]
pub struct Order {
    pub offer: k2::derive::Offer,
    pub buyer_key: String,
    pub buyer_payout: String,
    pub open_by: u64,
    pub prev: String,
}

pub const NO_PREV: &str = "0x0000000000000000000000000000000000000000";

fn hex20(s: &str) -> R<[u8; 20]> {
    let b = s.strip_prefix("0x").unwrap_or(s);
    if b.len() != 40 || !b.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Form(format!("要 0x 加四十位十六进制,给的是 {s:?}")));
    }
    let mut out = [0u8; 20];
    for (i, p) in b.as_bytes().chunks(2).enumerate() {
        let v = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
        out[i] = (v(p[0]) << 4) | v(p[1]);
    }
    Ok(out)
}

fn n(x: u64) -> Val {
    Val::u64(x)
}

impl Order {
    pub fn derive(&self, chain_id: u64, logic: &str) -> R<k2::derive::Order> {
        crate::mark::mark(crate::mark::FEATURE);
        k2::derive::order(&self.offer, chain_id, logic, &self.buyer_key, &self.buyer_payout, self.open_by, &self.prev)
            .map_err(|f| Fault::Law { token: f.token.name().to_string(), detail: format!("{f:?}") })
    }


    pub fn offer_abi(&self) -> R<Val> {
        let o = &self.offer;
        let mut ladder = Vec::with_capacity(o.ladder.len());
        for r in &o.ladder {
            ladder.push(Val::Tuple(vec![n(r.buyer_bps), n(r.seller_bps), n(r.fee_bearer)]));
        }
        Ok(Val::Tuple(vec![
            Val::Addr(hex20(&o.seller_key)?),
            Val::Addr(hex20(&o.judge)?),
            Val::Addr(hex20(&o.guild)?),
            Val::Addr(hex20(&o.seller_payout)?),
            Val::Addr(hex20(&o.seller_fund)?),
            Val::Addr(hex20(&o.token)?),
            Val::U256(o.amount.to_be()),
            n(o.delivery_window),
            n(o.confirm_window),
            n(o.open_window),
            n(u64::from(o.buyer_may_open)),
            Val::Arr(ladder),
            Val::B32(o.terms_hash),
            Val::B32(o.amanah_hash),
            Val::B32(o.quote_hash),
            Val::U256(u64_word(o.nonce)),
            Val::U256(u64_word(o.expiry)),
        ]))
    }

    pub fn abi_of(&self, shape: eravon_literals::constants::Shape) -> R<Val> {
        if shape == eravon_literals::constants::Shape::Trial {
            if !self.prev.eq_ignore_ascii_case(NO_PREV) {
                return Err(Fault::Form(format!("往代合约的订单没有前段,这一单的 prev 是 {}", self.prev)));
            }
            return Ok(Val::Tuple(vec![
                self.offer_abi()?,
                Val::Addr(hex20(&self.buyer_key)?),
                Val::Addr(hex20(&self.buyer_payout)?),
                n(self.open_by),
            ]));
        }
        self.abi()
    }

    pub fn abi(&self) -> R<Val> {
        Ok(Val::Tuple(vec![
            self.offer_abi()?,
            Val::Addr(hex20(&self.buyer_key)?),
            Val::Addr(hex20(&self.buyer_payout)?),
            n(self.open_by),
            Val::Addr(hex20(&self.prev)?),
        ]))
    }
}

fn u64_word(x: u64) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[24..].copy_from_slice(&x.to_be_bytes());
    w
}

pub fn bytes_of(what: &str, s: &str) -> R<Vec<u8>> {
    let b = s.strip_prefix("0x").unwrap_or(s);
    if b.len() % 2 != 0 || !b.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Form(format!("{what} 要是 0x 加偶数位十六进制,给的是 {s:?}")));
    }
    let mut out = Vec::with_capacity(b.len() / 2);
    for p in b.as_bytes().chunks(2) {
        let v = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
        out.push((v(p[0]) << 4) | v(p[1]));
    }
    Ok(out)
}

pub fn hex_of(b: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(2 + b.len() * 2);
    s.push_str("0x");
    for x in b {
        s.push(D[(x >> 4) as usize] as char);
        s.push(D[(x & 15) as usize] as char);
    }
    s
}

pub fn b32_of(what: &str, s: &str) -> R<[u8; 32]> {
    let v = bytes_of(what, s)?;
    if v.len() != 32 {
        return Err(Fault::Form(format!("{what} 要恰三十二字节,给的是 {} 字节", v.len())));
    }
    Ok(v.try_into().expect("三十二字节"))
}

pub fn addr_norm(what: &str, s: &str) -> R<String> {
    let b = s.strip_prefix("0x").unwrap_or(s);
    if b.len() != 40 || !b.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Form(format!("{what} 要 0x 加四十位十六进制,给的是 {s:?}")));
    }
    Ok(format!("0x{}", b.to_ascii_lowercase()))
}

pub fn addr_of(what: &str, s: &str) -> R<[u8; 20]> {
    hex20(s).map_err(|_| Fault::Form(format!("{what} 要 0x 加四十位十六进制,给的是 {s:?}")))
}

#[derive(Clone, Debug)]
pub struct Init {
    pub order: Order,
    pub quote: Vec<u8>,
    pub sig_buyer: Vec<u8>,
    pub sig_seller: Vec<u8>,
    pub members: Vec<String>,
    pub amanah_hashes: Vec<[u8; 32]>,
}

pub fn decode_init(data: &[u8]) -> R<Init> {
    use crate::abi::decode;
    let want = crate::sol::INIT.selector();
    let f = if data.len() >= 4 && data[..4] == crate::sol::TRIAL_INIT.selector() { crate::sol::TRIAL_INIT } else { crate::sol::INIT };
    if data.len() < 4 || data[..4] != f.selector() {
        return Err(Fault::Abi(format!(
            "这一段的选择子不是 init 的那一枚({} 对 {})",
            hex_of(&data[..data.len().min(4)]),
            hex_of(&want)
        )));
    }
    let sig = f.sig;
    let open = sig.find('(').ok_or_else(|| Fault::Abi("init 的签名不成形".into()))?;
    let vals = decode(&crate::abi::tuple_types(&sig[open..])?, &data[4..])?;
    use crate::sol::src_abi as k;
    let t = vals[0].items()?;
    let o = t[k::ERAVON_T_ORDER_TERMS_COL_OFFER].items()?;
    let mut ladder = Vec::new();
    for r in o[k::ERAVON_T_OFFER_COL_LADDER].items()? {
        let c = r.items()?;
        ladder.push(k2::doc::Rung {
            buyer_bps: c[k::ERAVON_T_RUNG_COL_BUYER_BPS].small()?,
            seller_bps: c[k::ERAVON_T_RUNG_COL_SELLER_BPS].small()?,
            fee_bearer: c[k::ERAVON_T_RUNG_COL_FEE_BEARER].small()?,
        });
    }
    let quote = vals[1].bytes()?.to_vec();
    let offer = k2::derive::Offer {
        seller_key: hex_of(&o[k::ERAVON_T_OFFER_COL_SELLER_KEY].addr()?),
        judge: hex_of(&o[k::ERAVON_T_OFFER_COL_JUDGE].addr()?),
        guild: hex_of(&o[k::ERAVON_T_OFFER_COL_GUILD].addr()?),
        seller_payout: hex_of(&o[k::ERAVON_T_OFFER_COL_SELLER_PAYOUT].addr()?),
        seller_fund: hex_of(&o[k::ERAVON_T_OFFER_COL_SELLER_FUND].addr()?),
        token: hex_of(&o[k::ERAVON_T_OFFER_COL_TOKEN].addr()?),
        amount: k2::u256::U256::from_be(&o[k::ERAVON_T_OFFER_COL_AMOUNT].word()?),
        delivery_window: o[k::ERAVON_T_OFFER_COL_DELIVERY_WINDOW].small()?,
        confirm_window: o[k::ERAVON_T_OFFER_COL_CONFIRM_WINDOW].small()?,
        open_window: o[k::ERAVON_T_OFFER_COL_OPEN_WINDOW].small()?,
        buyer_may_open: o[k::ERAVON_T_OFFER_COL_BUYER_MAY_OPEN].boolean()?,
        ladder,
        terms_hash: o[k::ERAVON_T_OFFER_COL_TERMS_HASH].word()?,
        amanah_hash: o[k::ERAVON_T_OFFER_COL_AMANAH_HASH].word()?,
        quote_hash: o[k::ERAVON_T_OFFER_COL_QUOTE_HASH].word()?,
        nonce: o[k::ERAVON_T_OFFER_COL_NONCE].small()?,
        expiry: o[k::ERAVON_T_OFFER_COL_EXPIRY].small()?,
    };
    let mut members = Vec::new();
    for m in vals[4].items()? {
        members.push(hex_of(&m.addr()?));
    }
    let mut words = Vec::new();
    for w in vals[5].items()? {
        words.push(w.word()?);
    }
    Ok(Init {
        order: Order {
            offer,
            buyer_key: hex_of(&t[k::ERAVON_T_ORDER_TERMS_COL_BUYER_KEY].addr()?),
            buyer_payout: hex_of(&t[k::ERAVON_T_ORDER_TERMS_COL_BUYER_PAYOUT].addr()?),
            open_by: t[k::ERAVON_T_ORDER_TERMS_COL_OPEN_BY].small()?,
            prev: match t.get(k::ERAVON_T_ORDER_TERMS_COL_PREV) {
                Some(p) => hex_of(&p.addr()?),
                None => NO_PREV.to_string(),
            },
        },
        quote,
        sig_buyer: vals[2].bytes()?.to_vec(),
        sig_seller: vals[3].bytes()?.to_vec(),
        members,
        amanah_hashes: words,
    })
}

use eravon_literals::json::In;

pub fn str_member(v: &In, k: &str) -> R<String> {
    match v.get(k) {
        Some(In::Str(s)) => Ok(s.clone()),
        other => Err(Fault::Form(format!("订单档上的 {k} 该是字符串,是 {other:?}"))),
    }
}

fn n_of(v: &In, k: &str) -> R<u64> {
    match v.get(k) {
        Some(In::Int(n)) if *n >= 0 => Ok(*n as u64),
        other => Err(Fault::Form(format!("订单档上的 {k} 该是非负整数,是 {other:?}"))),
    }
}

pub fn read_json(bytes: &[u8]) -> R<Order> {
    let v = eravon_literals::json::read(bytes).map_err(|e| Fault::Form(format!("订单档不成 JSON:{e}")))?;
    let mut ladder = Vec::new();
    match v.get("ladder") {
        Some(In::Arr(rows)) => {
            for r in rows {
                ladder.push(k2::doc::Rung {
                    buyer_bps: n_of(r, "buyerBps")?,
                    seller_bps: n_of(r, "sellerBps")?,
                    fee_bearer: n_of(r, "feeBearer")?,
                });
            }
        }
        other => return Err(Fault::Form(format!("ladder 该是一列,是 {other:?}"))),
    }
    let amount = k2::u256::U256::from_dec(&str_member(&v, "amount")?)
        .ok_or_else(|| Fault::Form("amount 该是一串十进制数".into()))?;
    let offer = k2::derive::Offer {
        seller_key: addr_norm("sellerKey", &str_member(&v, "sellerKey")?)?,
        judge: addr_norm("judge", &str_member(&v, "judge")?)?,
        guild: addr_norm("guild", &str_member(&v, "guild")?)?,
        seller_payout: addr_norm("sellerPayout", &str_member(&v, "sellerPayout")?)?,
        seller_fund: addr_norm("sellerFund", &str_member(&v, "sellerFund")?)?,
        token: addr_norm("token", &str_member(&v, "token")?)?,
        amount,
        delivery_window: n_of(&v, "deliveryWindow")?,
        confirm_window: n_of(&v, "confirmWindow")?,
        open_window: n_of(&v, "openWindow")?,
        buyer_may_open: match v.get("buyerMayOpen") {
            Some(In::Bool(b)) => *b,
            other => return Err(Fault::Form(format!("订单档上的 buyerMayOpen 该是布尔,是 {other:?}"))),
        },
        ladder,
        terms_hash: b32_of("termsHash", &str_member(&v, "termsHash")?)?,
        amanah_hash: b32_of("amanahHash", &str_member(&v, "amanahHash")?)?,
        quote_hash: b32_of("quoteHash", &str_member(&v, "quoteHash")?)?,
        nonce: n_of(&v, "nonce")?,
        expiry: n_of(&v, "expiry")?,
    };
    Ok(Order {
        offer,
        buyer_key: addr_norm("buyerKey", &str_member(&v, "buyerKey")?)?,
        buyer_payout: addr_norm("buyerPayout", &str_member(&v, "buyerPayout")?)?,
        open_by: n_of(&v, "openBy")?,
        prev: match v.get("prev") {
            None => NO_PREV.to_string(),
            Some(_) => addr_norm("prev", &str_member(&v, "prev")?)?,
        },
    })
}

