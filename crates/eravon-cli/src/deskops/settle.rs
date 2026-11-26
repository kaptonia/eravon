use eravon_link as k3;
use eravon_standard as a1;
use eravon_terms as k2;
use eravon_terms::u256::U256;

use super::No;
use crate::fault::{Fault, R};
use crate::shared::Wire;

pub const E_SETTLE_FORM: &str = "E_P6_SETTLE_FORM";

pub fn offer_by_link(w: &Wire, link: &[u8]) -> R<k3::settle::Settle> {
    let doc = crate::buyer::posted(w, link)?;
    match k3::link::resolve(link, &doc).map_err(Fault::from)? {
        k3::link::Resolved::Settle(st) => Ok(st),
        k3::link::Resolved::Listing(_) => Err(Fault::Down { layer: "P6", token: E_SETTLE_FORM.into(), why: "这条链接指向清单,不是和解要约".into() }),
    }
}

pub struct Settlement<'a> {
    pub instance: &'a str,
    pub buyer_key: &'a str,
    pub offer: &'a k3::settle::Settle,
    pub digest: &'a [u8; 32],
}

pub fn submit<T, E: From<No> + From<crate::fault::Fault> + From<a1::fault::Fault>>(
    w: &Wire,
    s: &Settlement<'_>,
    me: &str,
    own: bool,
    by_call: bool,
    key_for: &dyn Fn(&str) -> Result<[u8; 32], E>,
    post: impl FnOnce(&str, &a1::call::Call) -> Result<T, E>,
) -> Result<T, E> {
    if by_call && !own {
        return Err(No::new("E_P6_BY_CALL_KEY", vec![format!("{} 不是当前钥,请使用签名确认", s.buyer_key)]).into());
    }
    a1::chain::code_matches(&w.t, &w.table, s.instance, &w.logic)?;
    let sig_buyer = if by_call {
        Vec::new()
    } else {
        let mut k = key_for(s.buyer_key)?;
        let sig = crate::shared::sign(&k, s.digest);
        for b in k.iter_mut() {
            *b = 0;
        }
        let _ = std::hint::black_box(&k);
        a1::order::bytes_of("买家签名", &sig?)?
    };
    let sig_seller = if s.offer.sig.is_empty() { Vec::new() } else { a1::order::bytes_of("卖家签名", &s.offer.sig)? };
    let from = if by_call { s.buyer_key } else { me };
    let (sig_buyer, sig_seller) = a1::precheck::settle_slots(&w.t, &w.table, &w.logic, s.instance, s.digest, &sig_buyer, &sig_seller, from)?;
    let expiry = U256::from_dec(&s.offer.expiry.to_string()).unwrap_or_else(|| U256::from_be(&[0u8; 32])).to_be();
    let buyer_bps = u16::try_from(s.offer.buyer_bps).map_err(|_| No::new(E_SETTLE_FORM, vec!["买家份额超出范围".into()]))?;
    let call = a1::call::Action::Settle { buyer_bps, expiry, sig_buyer, sig_seller }.call(s.instance)?;
    post(from, &call)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Consent {
    pub digest: [u8; 32],
    pub seller_by_call: bool,
}

pub fn consent(w: &Wire, instance: &str, pack: Option<&[u8]>, seller: &str, st: &k3::settle::Settle) -> Result<Consent, No> {
    let a1n = |f: a1::fault::Fault| No::new(&f.name(), vec![f.detail()]);
    let (b, seller, logic) = match pack.map(k3::pack::pack) {
        Some(Ok(pk)) => (k3::settle::bind_settle(&pk, st), pk.listing.order.seller_key.clone(), Some(pk.listing.logic.clone())),
        _ => {
            let seller = if seller.is_empty() {
                let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
                eravon_dispute::settle::opened(&dw, instance).map_err(|(n, d)| No::new(&n, vec![d]))?.1
            } else {
                seller.to_string()
            };
            (k3::settle::bind_settle_on(w.chain_id, instance, &seller, st), seller, None)
        }
    };
    let b = b.map_err(|v| No::new(v.name, vec!["和解要约与订单不匹配".into()]))?;
    let now = a1::precheck::chain_now(&w.t, &w.table).map_err(a1n)?;
    if a1::precheck::in_window(st.expiry, now).is_err() {
        return Err(No::new("E_P6_SETTLE_EXPIRED", vec![format!("和解要约已于 {} 过期(现在 {now})", st.expiry)]));
    }
    let seller_by_call = matches!(b.form, k2::sig::Form::Call);
    if seller_by_call {
        let logic = a1::chain::case_generation(&w.t, &w.table, instance, &w.logic, logic.as_deref()).map_err(a1n)?.row().logic.to_string();
        gate(w, &logic, &seller, &b.digest)?;
    }
    Ok(Consent { digest: b.digest, seller_by_call })
}

fn gate(w: &Wire, logic: &str, seller: &str, digest: &[u8; 32]) -> Result<(), No> {
    let (g, r) = a1::consent::settle_gate(&w.t, &w.table, logic, seller, digest);
    match g {
        a1::consent::SettleGate::Pass => Ok(()),
        _ => Err(No::new("E_P6_SETTLE_GATE", vec![format!("{}({})", g.say(), r.why)])),
    }
}

