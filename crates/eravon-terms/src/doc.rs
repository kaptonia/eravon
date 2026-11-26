use eravon_galeed as k1;

use crate::form;
use crate::token::{Fault, Token, canon};

#[derive(Clone, Debug)]
pub struct Terms {
    pub terms_hash: [u8; 32],
    pub block: k1::Value,
    pub chain_id: u64,
    pub logic: String,
    pub seller_key: String,
    pub seller_payout: String,
    pub seller_fund: String,
    pub token: String,
    pub amount: crate::u256::U256,
    pub delivery_window: u64,
    pub confirm_window: u64,
    pub open_window: u64,
    pub buyer_may_open: bool,
    pub ladder: Vec<Rung>,
    pub judges: Vec<JudgeEl>,
    pub guilds: Vec<GuildEl>,
    pub nonce: u64,
    pub expiry: u64,
    pub series: Option<Series>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Series {
    pub id: String,
    pub stage: u64,
    pub stages: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rung {
    pub buyer_bps: u64,
    pub seller_bps: u64,
    pub fee_bearer: u64,
}

#[derive(Clone, Debug)]
pub struct JudgeEl {
    pub judge: String,
    pub amanah_hash: String,
    pub quote_hash: String,
}

#[derive(Clone, Debug)]
pub struct GuildEl {
    pub address: String,
    pub amanah_hash: String,
    pub quote_hash: String,
}

pub fn terms_hash(b: &[u8]) -> [u8; 32] {
    k1::hash::sha256(b)
}

const REQUIRED: [&str; 17] = [
    "v",
    "chainId",
    "logic",
    "sellerKey",
    "sellerPayout",
    "sellerFund",
    "token",
    "amount",
    "deliveryWindow",
    "confirmWindow",
    "openWindow",
    "buyerMayOpen",
    "ladder",
    "judges",
    "guilds",
    "nonce",
    "expiry",
];

const OPTIONAL: [&str; 6] = ["goods_md", "goodsHash", "delivery_md", "deliveryHash", "compliance_md", "series"];

fn lines(b: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (i, c) in b.iter().enumerate() {
        if *c == b'\n' {
            out.push(&b[start..i]);
            start = i + 1;
        }
    }
    out.push(&b[start..]);
    out
}

fn block_of(b: &[u8]) -> Result<Vec<u8>, Fault> {
    let open = eravon_literals::literals::TERMS_FENCE_OPEN.as_bytes();
    let close = eravon_literals::literals::TERMS_FENCE_CLOSE.as_bytes();
    let ls = lines(b);
    let opens: Vec<usize> = ls.iter().enumerate().filter(|(_, l)| **l == open).map(|(i, _)| i).collect();
    if opens.is_empty() {
        return Err(Fault::new(Token::TermsBlock));
    }
    if opens.len() > 1 {
        return Err(Fault::new(Token::TermsFences));
    }
    let o = opens[0];
    let c = ls.iter().enumerate().skip(o + 1).find(|(_, l)| **l == close).map(|(i, _)| i);
    let Some(c) = c else {
        return Err(Fault::new(Token::TermsUnclosed));
    };
    if ls[..o].iter().chain(ls[c + 1..].iter()).any(|l| !l.is_empty()) {
        return Err(Fault::new(Token::TermsOutside));
    }
    Ok(ls[o + 1..c].join(&b'\n'))
}


fn exactly_one(v: &k1::Value, text_key: &str, hash_key: &str, t: Token) -> Result<(), Fault> {
    match (v.get(text_key), v.get(hash_key)) {
        (Some(x), None) if form::as_str(x).is_some() => Ok(()),
        (None, Some(h)) if form::is_hex32(h) => Ok(()),
        _ => Err(Fault::new(t)),
    }
}

pub fn terms(b: &[u8]) -> Result<Terms, Fault> {
    if core::str::from_utf8(b).is_err() {
        return Err(Fault::new(Token::TermsUtf8));
    }
    let block = block_of(b)?;
    let v = k1::canon::accept(&block).map_err(|t| Fault::new(canon(t)))?;
    let Some(m) = form::as_obj(&v) else {
        return Err(Fault::new(Token::Terms));
    };
    for k in &REQUIRED {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::new(Token::TermsMissing));
        }
    }
    if m.iter().any(|(key, _)| !REQUIRED.contains(&key.as_str()) && !OPTIONAL.contains(&key.as_str())) {
        return Err(Fault::new(Token::TermsClosed));
    }
    let get = |k: &str| v.get(k).expect("必有成员已在上面查过");

    if form::as_str(get("v")) != Some(eravon_literals::literals::DOC_TERMS) {
        return Err(Fault::new(Token::TermsV));
    }
    let Some(chain_id) = form::as_int(get("chainId")) else {
        return Err(Fault::new(Token::TermsChain));
    };
    for (k, t) in [
        ("logic", Token::TermsLogic),
        ("sellerKey", Token::TermsSeller),
        ("sellerPayout", Token::TermsPayout),
        ("sellerFund", Token::TermsFund),
        ("token", Token::TermsToken),
    ] {
        if !form::is_nonzero_hex20(get(k)) {
            return Err(Fault::new(t));
        }
    }
    let seller_key = form::as_str(get("sellerKey")).expect("上面已过形").to_string();
    let amount = form::as_str(get("amount"))
        .and_then(crate::u256::U256::from_dec)
        .filter(|x| !x.is_zero() && x.cmp_u(&amount_bound()) != core::cmp::Ordering::Greater)
        .ok_or_else(|| Fault::new(Token::TermsAmount))?;
    for (k, t) in [
        ("deliveryWindow", Token::TermsDeliveryWindow),
        ("confirmWindow", Token::TermsConfirmWindow),
        ("openWindow", Token::TermsOpenWindow),
    ] {
        if !form::is_u32(get(k)) {
            return Err(Fault::new(t));
        }
    }
    let Some(buyer_may_open) = form::as_bool(get("buyerMayOpen")) else {
        return Err(Fault::new(Token::TermsBuyerOpen));
    };
    exactly_one(&v, "goods_md", "goodsHash", Token::TermsGoods)?;
    exactly_one(&v, "delivery_md", "deliveryHash", Token::TermsDelivery)?;

    let ladder = read_ladder(get("ladder"))?;
    let judges = read_judges(get("judges"), &seller_key, chain_id)?;
    let guilds = read_guilds(get("guilds"), &seller_key, chain_id)?;
    if judges.is_empty() && guilds.is_empty() {
        return Err(Fault::new(Token::TermsNoCandidate));
    }
    let Some(nonce) = form::as_int(get("nonce")) else {
        return Err(Fault::new(Token::TermsNonce));
    };
    let Some(expiry) = form::as_int(get("expiry")) else {
        return Err(Fault::new(Token::TermsExpiry));
    };
    if let Some(c) = v.get("compliance_md")
        && form::as_obj(c).is_none()
    {
        return Err(Fault::new(Token::TermsCompliance));
    }
    let series = match v.get("series") {
        Some(sv) => Some(read_series(sv).ok_or_else(|| Fault::new(Token::TermsSeries))?),
        None => None,
    };
    let int = |k: &str| form::as_int(get(k)).expect("上面已过形");
    let text = |k: &str| form::as_str(get(k)).expect("上面已过形").to_string();

    Ok(Terms {
        terms_hash: terms_hash(b),
        block: v.clone(),
        chain_id,
        logic: text("logic"),
        seller_key,
        seller_payout: text("sellerPayout"),
        seller_fund: text("sellerFund"),
        token: text("token"),
        amount,
        delivery_window: int("deliveryWindow"),
        confirm_window: int("confirmWindow"),
        open_window: int("openWindow"),
        buyer_may_open,
        ladder,
        judges,
        guilds,
        nonce,
        expiry,
        series,
    })
}

fn read_series(v: &k1::Value) -> Option<Series> {
    let m = form::as_obj(v)?;
    if m.len() != 3 || !m.iter().all(|(k, _)| matches!(k.as_str(), "id" | "stage" | "stages")) {
        return None;
    }
    let get = |k: &str| m.iter().find(|(key, _)| key == k).map(|(_, x)| x);
    let id = form::as_str(get("id")?)?;
    if !k1::hexform::is_hex32(id) || form::is_zero_hex(id) {
        return None;
    }
    let stage = form::as_int(get("stage")?)?;
    let stages = form::as_int(get("stages")?)?;
    (stage >= 1 && stages >= stage).then(|| Series { id: id.to_string(), stage, stages })
}

pub fn amount_bound() -> crate::u256::U256 {
    crate::u256::U256::MAX.div_rem_small(10000).0
}

pub fn element_shape(
    el: &k1::Value,
    index: usize,
    required: &[&str],
    optional: &[&str],
    t_obj: Token,
    t_missing: Token,
    t_closed: Token,
) -> Result<(), Fault> {
    let Some(m) = form::as_obj(el) else {
        return Err(Fault::at(t_obj, index));
    };
    for k in required {
        if !m.iter().any(|(key, _)| key == k) {
            return Err(Fault::at(t_missing, index));
        }
    }
    if m.iter().any(|(key, _)| {
        !required.contains(&key.as_str()) && !optional.contains(&key.as_str())
    }) {
        return Err(Fault::at(t_closed, index));
    }
    Ok(())
}

fn read_ladder(v: &k1::Value) -> Result<Vec<Rung>, Fault> {
    let Some(a) = form::as_arr(v).filter(|a| (2..=16).contains(&a.len())) else {
        return Err(Fault::new(Token::TermsLadder));
    };
    let mut out = Vec::with_capacity(a.len());
    for (i, el) in a.iter().enumerate() {
        out.push(read_rung(el, i)?);
    }
    ladder_wide(&out)?;
    Ok(out)
}

pub fn read_rung_with(el: &k1::Value, index: usize, note_ok: bool) -> Result<Rung, Fault> {
    let optional: &[&str] = if note_ok { &["note_md"] } else { &[] };
    element_shape(
        el,
        index,
        &["buyerBps", "sellerBps", "feeBearer"],
        optional,
        Token::Rung,
        Token::RungMissing,
        Token::RungClosed,
    )?;
    let buyer = el.get("buyerBps").expect("必有");
    if !form::is_int_at_most(buyer, 10000) {
        return Err(Fault::at(Token::RungBuyer, index));
    }
    let seller = el.get("sellerBps").expect("必有");
    let Some(seller_bps) = form::as_int(seller) else {
        return Err(Fault::at(Token::RungSeller, index));
    };
    if !form::is_int_at_most(el.get("feeBearer").expect("必有"), 2) {
        return Err(Fault::at(Token::RungBearer, index));
    }
    if note_ok
        && let Some(n) = el.get("note_md")
        && form::as_str(n).is_none()
    {
        return Err(Fault::at(Token::RungNote, index));
    }
    let buyer_bps = form::as_int(buyer).expect("已过形");
    if buyer_bps + seller_bps != 10000 {
        return Err(Fault::at(Token::RungSum, index));
    }
    Ok(Rung {
        buyer_bps,
        seller_bps,
        fee_bearer: form::as_int(el.get("feeBearer").expect("必有")).expect("已过形"),
    })
}

fn read_rung(el: &k1::Value, index: usize) -> Result<Rung, Fault> {
    read_rung_with(el, index, true)
}

pub fn ladder_wide(l: &[Rung]) -> Result<(), Fault> {
    if l[0].buyer_bps != 0 {
        return Err(Fault::new(Token::LadderFirst));
    }
    if l[l.len() - 1].buyer_bps != 10000 {
        return Err(Fault::new(Token::LadderLast));
    }
    for i in 1..l.len() {
        if l[i].buyer_bps < l[i - 1].buyer_bps {
            return Err(Fault::at(Token::LadderOrder, i));
        }
    }
    Ok(())
}

pub fn judge_element(
    el: &k1::Value,
    i: usize,
    seller_key: &str,
    chain_id: u64,
    extra_required: &[&str],
    extra_optional: &[&str],
) -> Result<JudgeEl, Fault> {
    let mut req: Vec<&str> = vec!["judge", "amanahHash", "quoteHash"];
    req.extend_from_slice(extra_required);
    let mut opt: Vec<&str> = vec!["source"];
    opt.extend_from_slice(extra_optional);
    element_shape(el, i, &req, &opt, Token::Judge, Token::JudgeMissing, Token::JudgeClosed)?;
    let judge = el.get("judge").expect("必有");
    if !form::is_nonzero_hex20(judge) || form::as_str(judge) == Some(seller_key) {
        return Err(Fault::at(Token::JudgeAddress, i));
    }
    if !form::is_nonzero_hex32(el.get("amanahHash").expect("必有")) {
        return Err(Fault::at(Token::JudgeAmanah, i));
    }
    if !form::is_nonzero_hex32(el.get("quoteHash").expect("必有")) {
        return Err(Fault::at(Token::JudgeQuote, i));
    }
    if let Some(s) = el.get("source") {
        let ok = form::as_str(s).and_then(form::pointer).is_some_and(|p| p.chain_id == chain_id);
        if !ok {
            return Err(Fault::at(Token::JudgeSource, i));
        }
    }
    let text = |k: &str| form::as_str(el.get(k).expect("必有")).expect("已过形").to_string();
    Ok(JudgeEl { judge: text("judge"), amanah_hash: text("amanahHash"), quote_hash: text("quoteHash") })
}

fn read_judges(v: &k1::Value, seller_key: &str, chain_id: u64) -> Result<Vec<JudgeEl>, Fault> {
    let Some(a) = form::as_arr(v) else {
        return Err(Fault::new(Token::TermsJudges));
    };
    let mut out = Vec::with_capacity(a.len());
    for (i, el) in a.iter().enumerate() {
        out.push(judge_element(el, i, seller_key, chain_id, &[], &[])?);
    }
    for i in 0..out.len() {
        if out[..i].iter().any(|e| e.judge == out[i].judge) {
            return Err(Fault::at(Token::TermsDupJudge, i));
        }
    }
    Ok(out)
}

pub fn guild_element(
    el: &k1::Value,
    i: usize,
    seller_key: &str,
    chain_id: u64,
    extra_required: &[&str],
    extra_optional: &[&str],
) -> Result<GuildEl, Fault> {
    let mut req: Vec<&str> = vec!["pointer", "amanahHash", "quoteHash"];
    req.extend_from_slice(extra_required);
    let opt: Vec<&str> = extra_optional.to_vec();
    element_shape(el, i, &req, &opt, Token::Guild, Token::GuildMissing, Token::GuildClosed)?;
    let p = form::as_str(el.get("pointer").expect("必有"))
        .and_then(form::pointer)
        .filter(|p| !form::is_zero_hex(&p.address) && p.address != seller_key && p.chain_id == chain_id)
        .ok_or_else(|| Fault::at(Token::GuildPointer, i))?;
    if !form::is_nonzero_hex32(el.get("amanahHash").expect("必有")) {
        return Err(Fault::at(Token::GuildAmanah, i));
    }
    if !form::is_nonzero_hex32(el.get("quoteHash").expect("必有")) {
        return Err(Fault::at(Token::GuildQuote, i));
    }
    let text = |k: &str| form::as_str(el.get(k).expect("必有")).expect("已过形").to_string();
    Ok(GuildEl { address: p.address, amanah_hash: text("amanahHash"), quote_hash: text("quoteHash") })
}

fn read_guilds(v: &k1::Value, seller_key: &str, chain_id: u64) -> Result<Vec<GuildEl>, Fault> {
    let Some(a) = form::as_arr(v) else {
        return Err(Fault::new(Token::TermsGuilds));
    };
    let mut out = Vec::with_capacity(a.len());
    for (i, el) in a.iter().enumerate() {
        out.push(guild_element(el, i, seller_key, chain_id, &[], &[])?);
    }
    for i in 0..out.len() {
        if out[..i].iter().any(|e| e.address == out[i].address) {
            return Err(Fault::at(Token::TermsDupGuild, i));
        }
    }
    Ok(out)
}
