use eravon_link as k3;
use eravon_standard as a1;
use eravon_terms as k2;

use eravon_store::record::Kind;
use eravon_store::seam::Box_;

use crate::order::{self, Order, State};
use crate::{Why, why};

pub fn order_of(
    terms: &[u8],
    candidate: u64,
    buyer_key: &str,
    buyer_payout: &str,
    open_by: u64,
    prev: &str,
) -> Result<a1::order::Order, Why> {
    let t = k2::doc::terms(terms).map_err(|f| why(f.token.name(), "无法读取条款"))?;
    let offer = k2::derive::offer(&t, candidate)
        .map_err(|f| why(f.token.name(), "无法生成订单要约"))?;
    let a =
        |what: &str, s: &str| a1::order::addr_norm(what, s).map_err(|e| why(&e.name(), e.detail()));
    Ok(a1::order::Order {
        offer,
        buyer_key: a("买家签名钥", buyer_key)?,
        buyer_payout: a("买家退款地址", buyer_payout)?,
        open_by,
        prev: a("前段实例", prev)?,
    })
}

fn terms_ok(p: &k3::listing::Listing, terms: &[u8], derived: &str, bound: &str) -> Result<(), Why> {
    let got = a1::order::hex_of(&k2::doc::terms_hash(terms));
    if !got.eq_ignore_ascii_case(&p.terms_hash) {
        return Err(why("E_WATCH_TERMS", format!("条款文档的 sha256 是 {got},清单所指的是 {}", p.terms_hash)));
    }
    if !derived.eq_ignore_ascii_case(bound) {
        return Err(why("E_WATCH_BIND", format!("条款装出的实例 {derived} 与 K3 绑定的预算地址 {bound} 不同")));
    }
    Ok(())
}


pub const E_SERIES: &str = "E_WATCH_SERIES";

fn series_ok(data: &str, me: &str, p: &k3::listing::Listing, r: &k3::receipt::Receipt, w: Option<&crate::Wire<'_>>) -> Result<(), Why> {
    let zero = r.prev.eq_ignore_ascii_case(a1::order::NO_PREV);
    let Some(sr) = p.terms.series.as_ref().filter(|s| s.stage > 1) else {
        return if zero { Ok(()) } else { Err(why(E_SERIES, "prev 非零的回执只收作系列的后段:这份清单不是系列后段")) };
    };
    if zero {
        return Err(why(E_SERIES, format!("系列第 {} 段的回执要以 prev 指向第 {} 段的实例", sr.stage, sr.stage - 1)));
    }
    let before = order::all(data, me)?.into_iter().find(|o| o.instance.eq_ignore_ascii_case(&r.prev));
    let Some(before) = before else {
        return Err(why(E_SERIES, format!("prev {} 不是本机所持的前段订单:先收第 {} 段的回执", r.prev, sr.stage - 1)));
    };
    let bl = order::part(data, me, &before.id, &order::part_listing()).ok().and_then(|b| k3::listing::listing(&b).ok());
    let Some(bl) = bl else { return Err(why(E_SERIES, format!("前段订单 {} 的清单读不成", before.id))) };
    let agrees = bl.terms.series.as_ref().is_some_and(|b| b.id.eq_ignore_ascii_case(&sr.id) && b.stage + 1 == sr.stage && b.stages == sr.stages)
        && bl.chain_id == p.chain_id
        && bl.logic.eq_ignore_ascii_case(&p.logic)
        && bl.order.seller_key.eq_ignore_ascii_case(&p.order.seller_key);
    if !agrees {
        return Err(why(E_SERIES, format!("prev 所指的订单 {} 不是同一系列的第 {} 段(系列号、段数、链、逻辑与卖家钥要都相同)", before.id, sr.stage - 1)));
    }
    if !before.buyer_key.eq_ignore_ascii_case(&r.buyer_key) {
        return Err(why(E_SERIES, format!("系列各段要同一把买家钥:前段是 {},这一段是 {}", before.buyer_key, r.buyer_key)));
    }
    let w = w.ok_or_else(|| why(E_SERIES, "核后段的 openBy 要读链取前段候选的报价"))?;
    let d = crate::poll::deal(data, me, &before, w)?;
    let q = crate::act::quote(w, &eravon_cands::act::Candidate::of(&d.o), &a1::order::hex_of(&d.o.offer.quote_hash))?;
    let span = eravon_goods::series::span(bl.terms.delivery_window, bl.terms.confirm_window, q.offer_window, q.ruling_window, d.guild);
    let latest = before.open_by.saturating_add(span);
    if r.open_by <= latest {
        return Err(why(E_SERIES, format!("这一段的 openBy {} 不晚于前段最晚可结束之秒 {latest}:前段还在途时这一段就可能过期退款", r.open_by)));
    }
    Ok(())
}


#[allow(clippy::too_many_arguments)]
pub fn paste(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    payload: &[u8],
    receipt: &[u8],
    terms: &[u8],
    id: Option<&str>,
    w: Option<&crate::Wire<'_>>,
) -> Result<Order, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let p = k3::listing::listing(payload).map_err(|f| why(f.token.name(), "清单无效"))?;
    let r = k3::receipt::receipt_under(&p.logic, receipt).map_err(|f| why(f.token.name(), "回执无效"))?;
    let bound = k3::receipt::bind_receipt(&p, &r).map_err(|v| why(v.name, "回执与订单不匹配"))?;
    let parties = crate::scan::party_keys(data, me)?;
    if !parties.iter().any(|k| p.order.seller_key.eq_ignore_ascii_case(k)) {
        return Err(why(
            "E_WATCH_SELLER",
            format!("清单的卖家是 {},当前身份认的是 {}", p.order.seller_key, parties.join("、")),
        ));
    }
    let o = order_of(terms, r.candidate, &r.buyer_key, &r.buyer_payout, r.open_by, &r.prev)?;
    let d = o.derive(p.chain_id, &p.logic).map_err(|e| why(&e.name(), e.detail()))?;
    terms_ok(&p, terms, &d.instance, &bound.instance)?;
    series_ok(data, me, &p, &r, w)?;
    let case_id = a1::order::hex_of(&bound.salt);
    let id = id
        .map(str::to_string)
        .unwrap_or_else(|| case_id[2..18].to_string());
    let old = order::load(data, me, &id)
        .ok()
        .filter(|x| x.case_id.eq_ignore_ascii_case(&case_id));
    let mut out = Order {
        opened: 0,
        id: id.clone(),
        case_id,
        instance: bound.instance.clone(),
        buyer_key: r.buyer_key.to_ascii_lowercase(),
        buyer_payout: r.buyer_payout.to_ascii_lowercase(),
        verdict: bound.verdict().to_string(),
        token: o.offer.token.to_ascii_lowercase(),
        need: o.offer.amount.to_dec(),
        open_by: r.open_by,
        candidate: r.candidate,
        expiry: o.offer.expiry,
        state: State::Awaiting,
        balance: String::new(),
        gap: String::new(),
        note: String::new(),
        notice: String::new(),
        notice_at: 0,
        deploy_tx: String::new(),
        expire_tx: String::new(),
        source: "paste".into(),
        extra: Vec::new(),
    };
    if let Some(old) = old {
        out.state = old.state;
        out.balance = old.balance;
        out.gap = old.gap;
        out.note = old.note;
        out.notice = old.notice;
        out.notice_at = old.notice_at;
        out.deploy_tx = old.deploy_tx;
        out.expire_tx = old.expire_tx;
        out.extra = old.extra;
    }
    crate::units::fill(data, me, None, &mut out, &r.terms_hash);
    order::save(
        data,
        writer,
        now,
        me,
        &out,
        &[
            (order::part_receipt(), r.current_form()),
            (order::part_listing(), payload.to_vec()),
            (order::part_terms(), terms.to_vec()),
        ],
    )?;
    Ok(out)
}

pub fn archived(
    data: &str,
    me: &str,
    terms_hash: &str,
) -> Result<Option<(k3::listing::Listing, Vec<u8>, Vec<u8>)>, Why> {
    let s = eravon_vault::store::open_read(data).map_err(|e| why(e.name(), e.detail()))?;
    let mut payload = None;
    let mut terms = None;
    for owner in eravon_vault::store::archive_of(data, me).map_err(|e| why(e.name(), e.detail()))? {
        let rows = match s.list(&owner, Box_::Products) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(why(e.name(), e.detail())),
        };
        for r in &rows {
            for (name, _) in &r.parts {
                let Ok(b) = s.part(&owner, Box_::Products, &r.id, name) else {
                    continue;
                };
                match r.kind {
                    Kind::Payload if payload.is_none() => {
                        if let Ok(p) = k3::listing::listing(&b)
                            && p.terms_hash.eq_ignore_ascii_case(terms_hash)
                        {
                            payload = Some((p, b));
                        }
                    }
                    Kind::TermsDocument
                        if terms.is_none()
                            && a1::order::hex_of(&k2::doc::terms_hash(&b))
                                .eq_ignore_ascii_case(terms_hash) =>
                    {
                        terms = Some(b);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(match (payload, terms) {
        (Some((p, pb)), Some(t)) => Some((p, pb, t)),
        _ => None,
    })
}

pub fn paste_receipt(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    receipt: &[u8],
    id: Option<&str>,
    w: Option<&crate::Wire<'_>>,
) -> Result<Order, Why> {
    let r = k3::receipt::receipt(receipt)
        .or_else(|e| k3::receipt::receipt_trial(receipt).map_err(|_| e))
        .map_err(|f| why(f.token.name(), "回执无效"))?;
    let Some((_, pb, tb)) = archived(data, me, &r.terms_hash)? else {
        return Err(why(
            "E_WATCH_ARCHIVE",
            format!(
                "商品归档里没有 termsHash {} 那一版的清单与条款;命令行 seller-receipt-paste 可递 --listing 与 --terms",
                r.terms_hash
            ),
        ));
    };
    paste(data, writer, now, me, &pb, receipt, &tb, id, w)
}
