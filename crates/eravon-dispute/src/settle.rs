use eravon_galeed as k1;
use eravon_link as k3;
use eravon_literals::json::{Out, write as canon};
use eravon_literals::literals as lit;
use eravon_standard as a1;
use eravon_terms as k2;
use eravon_watch::order::{self, Order};
use eravon_watch::send::Post;

use crate::{Why, Wire, a1w, why};

#[derive(Clone, Debug)]
pub struct Offer {
    pub buyer_bps: u64,
    pub expiry: u64,
    pub sig: String,
    pub digest: [u8; 32],
    pub bytes: Vec<u8>,
}

pub fn document(
    chain_id: u64,
    instance: &str,
    buyer_bps: u64,
    expiry: u64,
    sig: &str,
) -> Result<Vec<u8>, Why> {
    let m: Vec<(&'static str, Out)> = vec![
        ("buyerBps", Out::N(buyer_bps)),
        ("chainId", Out::N(chain_id)),
        ("expiry", Out::N(expiry)),
        ("instance", Out::S(instance.to_string())),
        ("sig", Out::S(sig.to_string())),
        ("v", Out::S(lit::DOC_SETTLE.to_string())),
    ];
    let bytes = canon(&Out::O(m));
    k3::settle::settle(&bytes).map_err(|f| why(f.token.name(), "无法生成和解要约"))?;
    Ok(bytes)
}

pub fn expiry_in(now: u64, days: &str) -> Result<u64, Why> {
    let d: u64 = days
        .trim()
        .parse()
        .map_err(|_| why("E_DISPUTE_EXPIRY", format!("有效期无效:{days}")))?;
    d.checked_mul(86_400)
        .and_then(|s| now.checked_add(s))
        .ok_or_else(|| why("E_DISPUTE_EXPIRY", format!("有效期 {d} 天超出范围")))
}

pub fn offer(
    sign: &dyn Fn(&[u8; 32]) -> Result<String, Why>,
    chain_id: u64,
    instance: &str,
    buyer_bps: u64,
    expiry: u64,
) -> Result<Offer, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    if buyer_bps > 10_000 {
        return Err(why(
            "E_DISPUTE_BPS",
            format!("退款比例超出范围:{buyer_bps}"),
        ));
    }
    let act = k2::derive::Action::Settle { buyer_bps, expiry: k2::u256::U256::from_u64(expiry) };
    let digest = k2::derive::digest(chain_id, instance, &act.struct_hash());
    let sig = sign(&digest)?;
    let bytes = document(chain_id, instance, buyer_bps, expiry, &sig)?;
    Ok(Offer { buyer_bps, expiry, sig, digest, bytes })
}

pub fn post_call(case_id: &[u8; 32], bytes: &[u8]) -> Result<a1::call::Call, Why> {
    a1::call::post(eravon_literals::constants::BOARD, case_id, bytes).map_err(a1w)
}


pub fn posted(w: &Wire<'_>, tx: &str, case_id: &[u8; 32], bytes: &[u8]) -> Result<(), Why> {
    let seen = eravon_scan::board::by_tx(w.t, w.table, tx, &k1::hash::sha256(bytes)).map_err(|e| why(&e.name(), e.detail()))?;
    if seen.iter().any(|p| p.key == *case_id) {
        Ok(())
    } else {
        Err(why("E_DISPUTE_POST_UNSEEN", format!("交易 {tx} 的回执里读不到这份和解要约(公告板码、hash 与 caseId 三样)")))
    }
}

pub fn full_is_refund(w: &Wire<'_>, o: &Order, buyer_bps: u64) -> Result<(), Why> {
    if buyer_bps == 10_000 && crate::refund::card("", "", w, o).is_ok() {
        return Err(why("E_DISPUTE_FULL_REFUND", "买家全额的和解走退款(refund):判官费照退卖家"));
    }
    Ok(())
}

pub fn remember(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    o: &Order,
    of: &Offer,
) -> Result<(), Why> {
    let mut o2 = order::load(data, me, &o.id)?;
    o2.set_extra("settleBps", &of.buyer_bps.to_string());
    o2.set_extra("settleExpiry", &of.expiry.to_string());
    o2.set_extra("settleSig", &of.sig);
    order::save(data, writer, now, me, &o2, &[])
}

pub fn remembered(o: &Order) -> Option<(u64, u64, String)> {
    Some((
        o.extra("settleBps")?.parse().ok()?,
        o.extra("settleExpiry")?.parse().ok()?,
        o.extra("settleSig")?.to_string(),
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paste {
    Settle { buyer_bps: u64, expiry: u64 },
    Confirm,
}

impl Paste {
    fn act(self) -> k2::derive::Action {
        match self {
            Paste::Settle { buyer_bps, expiry } => k2::derive::Action::Settle { buyer_bps, expiry: k2::u256::U256::from_u64(expiry) },
            Paste::Confirm => k2::derive::Action::Confirm,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Paste::Settle { .. } => "Settle",
            Paste::Confirm => "Confirm",
        }
    }
}

fn signed(
    chain_id: u64,
    instance: &str,
    what: Paste,
    sig: &str,
    buyer: &str,
) -> Result<(), k2::token::Fault> {
    let act = what.act();
    let d = k2::derive::digest(chain_id, instance, &act.struct_hash());
    k2::sig::verify_action(
        &act,
        k2::derive::Party::Buyer,
        &d,
        &k1::Value::Str(sig.to_string()),
        "",
        buyer,
    )
    .map(|_| ())
}

pub fn takes_empty(what: Paste) -> bool {
    matches!(what, Paste::Settle { .. })
}

pub fn check(
    chain_id: u64,
    instance: &str,
    buyer: &str,
    what: Paste,
    sig: &str,
    others: &[String],
) -> Result<(), Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let buyer = buyer.to_ascii_lowercase();
    if sig.is_empty() && takes_empty(what) {
        return Ok(());
    }
    if !k1::hexform::is_hex65(sig) {
        return Err(why(
            "E_PASTE_FORM",
            format!(
                "买家签名要是 hex65(0x 加一百三十位十六进制),给的是 {} 个字",
                sig.len()
            ),
        ));
    }
    match signed(chain_id, instance, what, sig, &buyer) {
        Ok(()) => return Ok(()),
        Err(f) if f.token.name() != "E_SIG_SIGNER" => {
            return Err(why(
                "E_PASTE_FORM",
                format!("买家签名无效:{}", f.token.name()),
            ));
        }
        Err(_) => {}
    }
    for other in others.iter().filter(|x| !x.eq_ignore_ascii_case(instance)) {
        if signed(chain_id, other, what, sig, &buyer).is_ok() {
            return Err(why(
                "E_PASTE_DOMAIN",
                format!("签名不属于本订单({other} ≠ {instance})"),
            ));
        }
    }
    let flip = match what {
        Paste::Settle { .. } => Some(Paste::Confirm),
        Paste::Confirm => None,
    };
    if let Some(f) = flip
        && signed(chain_id, instance, f, sig, &buyer).is_ok()
    {
        return Err(why(
            "E_PASTE_ACTION",
            format!("这枚签名签的是 {},这里要的是 {}", f.name(), what.name()),
        ));
    }
    Err(why(
        "E_PASTE_SIGNER",
        format!(
            "该 {} 签名不是本单买家 {buyer} 所签",
            what.name()
        ),
    ))
}


pub fn others(data: &str, me: &str) -> Result<Vec<String>, Why> {
    order::all(data, me).map(|v| v.into_iter().map(|o| o.instance).collect()).map_err(|e| why(&e.0, e.1))
}

fn word_u64(n: u64) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[24..].copy_from_slice(&n.to_be_bytes());
    w
}

pub fn check_pasted(w: &Wire<'_>, instance: &str, what: Paste, sig: &str, others: &[String]) -> Result<(), Why> {
    let s = a1::chain::stage(w.t, w.table, instance).map_err(a1w)?.value;
    let now = a1::chain::acting_key(w.t, w.table, &s.buyer_key, s.past).map_err(a1w)?;
    check_under(w.chain_id, instance, &now, &s.buyer_key, what, sig, others)
}

pub fn check_under(
    chain_id: u64,
    instance: &str,
    now: &str,
    recorded: &str,
    what: Paste,
    sig: &str,
    others: &[String],
) -> Result<(), Why> {
    let first = check(chain_id, instance, now, what, sig, others);
    if let (Err(e), Paste::Settle { .. }) = (&first, what)
        && e.0 == "E_PASTE_SIGNER"
        && !recorded.eq_ignore_ascii_case(now)
    {
        match check(chain_id, instance, recorded, what, sig, others) {
            Err(e2) if e2.0 == "E_PASTE_SIGNER" => {}
            other => return other,
        }
    }
    first
}

pub fn parties(w: &Wire<'_>, instance: &str) -> Result<(String, String), Why> {
    opened(w, instance).map(|(_, s, b, _)| (s, b))
}

pub fn opened(w: &Wire<'_>, instance: &str) -> Result<(String, String, String, u64), Why> {
    let (rows, _) = eravon_chain::read::logs(w.t, w.table, instance, 0, eravon_chain::seam::Upto::Latest)
        .map_err(|e| why(e.name(), e.detail()))?;
    let t0 = a1::sol::OPENED.topic0();
    for row in &rows {
        let l = eravon_scan::log::Log::read(row).map_err(|e| why(&e.name(), e.detail()))?;
        if l.topics.first() == Some(&t0) && l.topics.len() == 4 {
            let a = |i: usize| l.topic_address(i).map_err(|e| why(&e.name(), e.detail()));
            let case = l.topic_hex(1).map_err(|e| why(&e.name(), e.detail()))?;
            return Ok((case, a(2)?, a(3)?, l.block));
        }
    }
    Err(why("E_DISPUTE_NOT_OPEN", format!("订单尚未开启({instance})")))
}

pub fn send_settle(
    data: &str,
    me: &str,
    w: &Wire<'_>,
    o: &Order,
    sig_buyer: &str,
    post: &mut dyn Post,
) -> Result<String, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let (bps, expiry, sig_seller) = remembered(o).ok_or_else(|| {
        why(
            "E_PASTE_NO_OFFER",
            "请先发出和解要约",
        )
    })?;
    check_pasted(
        w,
        &o.instance,
        Paste::Settle {
            buyer_bps: bps,
            expiry,
        },
        sig_buyer,
        &others(data, me)?,
    )?;
    send_settle_raw(w, &o.instance, bps, expiry, sig_buyer, &sig_seller, post)
}

pub fn send_settle_raw(
    w: &Wire<'_>,
    instance: &str,
    bps: u64,
    expiry: u64,
    sig_buyer: &str,
    sig_seller: &str,
    post: &mut dyn Post,
) -> Result<String, Why> {
    let bytes = |what: &str, s: &str| a1::order::bytes_of(what, s).map_err(a1w);
    let buyer_bps =
        u16::try_from(bps).map_err(|_| why("E_DISPUTE_BPS", format!("比例超出范围:{bps}")))?;
    let (sb, ss) = (bytes("买家签名", sig_buyer)?, bytes("卖家签名", sig_seller)?);
    a1::call::Action::Settle { buyer_bps, expiry: word_u64(expiry), sig_buyer: sb.clone(), sig_seller: ss.clone() }.call(instance).map_err(a1w)?;
    a1::chain::code_matches(w.t, w.table, instance, w.logic).map_err(a1w)?;
    let digest = k2::derive::digest(w.chain_id, instance, &k2::derive::Action::Settle { buyer_bps: bps, expiry: k2::u256::U256::from_u64(expiry) }.struct_hash());
    let from = post.from().ok_or_else(|| why("E_DISPUTE_LOCKED", "未解锁,不知发送钥"))?;
    let (sig_buyer, sig_seller) = a1::precheck::settle_slots(w.t, w.table, w.logic, instance, &digest, &sb, &ss, &from).map_err(a1w)?;
    let call = a1::call::Action::Settle { buyer_bps, expiry: word_u64(expiry), sig_buyer, sig_seller }.call(instance).map_err(a1w)?;
    post.post(&call)
}

pub fn send_confirm(
    data: &str,
    me: &str,
    w: &Wire<'_>,
    o: &Order,
    sig: &str,
    post: &mut dyn Post,
) -> Result<String, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    check_pasted(w, &o.instance, Paste::Confirm, sig, &others(data, me)?)?;
    confirm_raw(w, &o.instance, sig, post)
}

pub fn confirm_raw(
    w: &Wire<'_>,
    instance: &str,
    sig: &str,
    post: &mut dyn Post,
) -> Result<String, Why> {
    let call = a1::call::BySig::Confirm {
        sig: a1::order::bytes_of("买家签名", sig).map_err(a1w)?,
    }
    .call(instance)
    .map_err(a1w)?;
    a1::chain::code_matches(w.t, w.table, instance, w.logic).map_err(a1w)?;
    post.post(&call)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_paste_goes_on_under_a_settlement_and_is_refused_for_a_confirm() {
        let a = "0x00000000000000000000000000000000000000a1";
        let buyer = "0x00000000000000000000000000000000000000c1";
        let s = Paste::Settle { buyer_bps: 5000, expiry: 99 };
        assert!(takes_empty(s) && !takes_empty(Paste::Confirm));
        assert!(check(1, a, buyer, s, "", &[]).is_ok());
        assert_eq!(check(1, a, buyer, Paste::Confirm, "", &[]).unwrap_err().0, "E_PASTE_FORM");
    }

    #[test]
    fn paste_names_domain_action_and_signer_apart() {
        let key = [7u8; 32];
        let buyer = k2::sig::sign_secp256k1(&key, &[0u8; 32]).expect("钥").1;
        let a = "0x00000000000000000000000000000000000000a1";
        let b = "0x00000000000000000000000000000000000000b1";
        let sign = |inst: &str, act: k2::derive::Action| k2::sig::sign_secp256k1(&key, &k2::derive::digest(1, inst, &act.struct_hash())).expect("签").0;
        let s = Paste::Settle { buyer_bps: 5000, expiry: 99 };
        let ok = sign(a, k2::derive::Action::Settle { buyer_bps: 5000, expiry: k2::u256::U256::from_u64(99) });
        assert!(check(1, a, &buyer, s, &ok, &[]).is_ok());
        let other = sign(b, k2::derive::Action::Settle { buyer_bps: 5000, expiry: k2::u256::U256::from_u64(99) });
        assert_eq!(check(1, a, &buyer, s, &other, &[b.to_string()]).unwrap_err().0, "E_PASTE_DOMAIN");
        let conf = sign(a, k2::derive::Action::Confirm);
        assert_eq!(check(1, a, &buyer, s, &conf, &[]).unwrap_err().0, "E_PASTE_ACTION");
        assert_eq!(check(1, a, &buyer, s, &other, &[]).unwrap_err().0, "E_PASTE_SIGNER");
        assert_eq!(check(1, a, &buyer, s, "0x12", &[]).unwrap_err().0, "E_PASTE_FORM");
    }

    #[test]
    fn a_moved_record_takes_the_acting_key_and_a_former_key_only_toward_the_registry() {
        let (old_key, new_key, third_key) = ([7u8; 32], [8u8; 32], [9u8; 32]);
        let address = |k: &[u8; 32]| k2::sig::sign_secp256k1(k, &[0u8; 32]).expect("钥").1;
        let (old, new) = (address(&old_key), address(&new_key));
        let a = "0x00000000000000000000000000000000000000a1";
        let sign = |k: &[u8; 32], act: k2::derive::Action| k2::sig::sign_secp256k1(k, &k2::derive::digest(1, a, &act.struct_hash())).expect("签").0;
        let settle = |k: &[u8; 32]| sign(k, k2::derive::Action::Settle { buyer_bps: 5000, expiry: k2::u256::U256::from_u64(99) });
        let confirm = |k: &[u8; 32]| sign(k, k2::derive::Action::Confirm);
        let (s, c) = (Paste::Settle { buyer_bps: 5000, expiry: 99 }, Paste::Confirm);
        let under = |what: Paste, sig: &str| check_under(1, a, &new, &old, what, sig, &[]);
        assert!(under(s, &settle(&new_key)).is_ok() && under(c, &confirm(&new_key)).is_ok());
        assert!(under(s, &settle(&old_key)).is_ok(), "the registry decides a former key's Settle");
        assert_eq!(under(c, &confirm(&old_key)).unwrap_err().0, "E_PASTE_SIGNER");
        let e = under(s, &settle(&third_key)).unwrap_err();
        assert!(e.0 == "E_PASTE_SIGNER" && e.1.contains(&new.to_ascii_lowercase()), "{e:?}");
        assert_eq!(under(c, &confirm(&third_key)).unwrap_err().0, "E_PASTE_SIGNER");
        let b = "0x00000000000000000000000000000000000000b1";
        let other = k2::sig::sign_secp256k1(&old_key, &k2::derive::digest(1, b, &k2::derive::Action::Settle { buyer_bps: 5000, expiry: k2::u256::U256::from_u64(99) }.struct_hash())).expect("签").0;
        assert_eq!(check_under(1, a, &new, &old, s, &other, &[b.to_string()]).unwrap_err().0, "E_PASTE_DOMAIN");
        assert!(check_under(1, a, &old, &old, s, &settle(&old_key), &[]).is_ok());
        assert_eq!(check_under(1, a, &new, &new, s, &settle(&old_key), &[]).unwrap_err().0, "E_PASTE_SIGNER");
    }
}
