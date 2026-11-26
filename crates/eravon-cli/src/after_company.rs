use eravon_link as k3;
use eravon_standard as a1;
use eravon_terms::u256::U256;
use a1::company::Item;
use a1::proposal::{Original, Plan};

use crate::shared::Wire;

pub type Why = (String, String);

fn a1w(f: a1::fault::Fault) -> Why {
    (f.name(), f.detail())
}

fn watch_wire(w: &Wire) -> eravon_watch::Wire<'_> {
    eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries }
}

pub struct At<'a> {
    pub instance: &'a str,
    pub case_id: &'a str,
    pub token: &'a str,
    pub window_end: u64,
}

pub struct Evidence<'a> {
    pub manifest: [u8; 32],
    pub bytes: Option<&'a [u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Wallet,
    Personal,
}

impl Form {
    pub fn of(s: &str) -> Option<Form> {
        match s {
            "wallet" => Some(Form::Wallet),
            "personal" => Some(Form::Personal),
            _ => None,
        }
    }
}

fn instance_call(call: &a1::call::Call, fun: a1::sol::Fun) -> Result<(a1::proposal::PCall, Original), Why> {
    Ok((a1::proposal::of_call(call).map_err(a1w)?, instance_abi(&[fun])?))
}

pub fn refund_need(w: &Wire, instance: &str, case_id: &str) -> Result<(U256, U256, U256), Why> {
    fee_of(w, instance, case_id).map(|f| (f.lf, f.stray, f.need))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fee {
    pub lf: U256,
    pub stray: U256,
    pub need: U256,
    pub token: String,
}

fn fee_of(w: &Wire, instance: &str, case_id: &str) -> Result<Fee, Why> {
    let sw = watch_wire(w);
    let v = eravon_dispute::view::case(&sw, instance, case_id, 0)?;
    let st = a1::chain::status(&w.t, &w.table, instance).map_err(a1w)?.value;
    let stray = eravon_watch::poll::stray(&sw, instance, &v.token, &st, &v.amount)?;
    let need = if v.lf.cmp_u(&stray) == std::cmp::Ordering::Greater { v.lf.overflowing_sub(&stray).0 } else { U256::from_be(&[0u8; 32]) };
    Ok(Fee { lf: v.lf, stray, need, token: v.token })
}

pub fn refund_fee(w: &Wire, instance: &str, case_id: &str, payer: &str) -> Result<Fee, Why> {
    let f = fee_of(w, instance, case_id)?;
    covered(w, &f, payer)?;
    Ok(f)
}

fn covered(w: &Wire, f: &Fee, payer: &str) -> Result<(), Why> {
    if !f.need.is_zero() {
        let have = U256::from_be(&a1::chain::erc20_balance(&w.t, &w.table, &f.token, payer).map_err(a1w)?.value);
        if f.need.cmp_u(&have) == std::cmp::Ordering::Greater {
            let short = a1::precheck::word(&w.t, &w.table, &f.token, &f.need.overflowing_sub(&have).0.to_be());
            return Err(("E_P6_SHORT".into(), format!("诉讼费不足 {short},请转入 {payer}")));
        }
    }
    Ok(())
}

pub fn pull_fee(w: &Wire, instance: &str, case_id: &str, me: &str) -> Result<(Fee, String), Why> {
    let frozen = a1::chain::stage(&w.t, &w.table, instance).map_err(a1w)?.value.buyer_key;
    let f = fee_of(w, instance, case_id)?;
    if !f.need.is_zero() && !frozen.eq_ignore_ascii_case(me) {
        return Err(("E_P6_PULL_KEY".into(), format!("拉取只从冻结的 buyerKey {frozen} 拉,{me} 不是它;请先把诉讼费转到订单,再申请退款")));
    }
    covered(w, &f, &frozen)?;
    Ok((f, frozen))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owed {
    pub token: String,
    pub payout: String,
    pub amount: String,
    pub balance: String,
}

pub fn payout_of(chain: Option<&eravon_dispute::view::CaseView>, pack: Option<&[u8]>) -> Result<String, Why> {
    if let Some(p) = chain.map(|v| v.buyer_payout.clone()).filter(|p| !p.is_empty() && p.trim_start_matches("0x").chars().any(|x| x != '0')) {
        return Ok(p);
    }
    let pk = pack.and_then(|b| k3::pack::pack(b).ok()).ok_or_else(|| ("E_P6_NO_PAYOUT".to_string(), "无法读取退款地址".to_string()))?;
    Ok(pk.receipt.buyer_payout.to_ascii_lowercase())
}

pub fn owed(w: &Wire, instance: &str, order_token: &str, chain: Option<&eravon_dispute::view::CaseView>, pack: Option<&[u8]>, token: Option<&str>) -> Result<Owed, Why> {
    let token = match token {
        Some(t) => eravon_vault::payout::accept(t.trim()).map_err(|e| (e.name().to_string(), e.detail()))?,
        None => order_token.to_string(),
    };
    let payout = payout_of(chain, pack)?;
    let balance = U256::from_be(&a1::chain::erc20_balance(&w.t, &w.table, &token, instance).map_err(a1w)?.value);
    let case_token = chain.is_some_and(|v| v.phase != 0 && v.token.eq_ignore_ascii_case(&token));
    let amount = if !case_token {
        balance
    } else {
        let owed = U256::from_be(&a1::chain::owed(&w.t, &w.table, instance, &payout, &token).map_err(a1w)?.value);
        let stray = match chain {
            Some(v) if v.phase >= 4 => {
                let st = a1::chain::status(&w.t, &w.table, instance).map_err(a1w)?.value;
                eravon_watch::poll::stray(&watch_wire(w), instance, &token, &st, &v.amount)?
            }
            _ => U256::from_be(&[0u8; 32]),
        };
        owed.overflowing_add(&stray).0
    };
    Ok(Owed { token, payout, amount: amount.to_dec(), balance: balance.to_dec() })
}

pub fn sweep_due(w: &Wire, instance: &str, order_token: &str, chain: Option<&eravon_dispute::view::CaseView>, pack: Option<&[u8]>, token: Option<&str>) -> Result<Owed, Why> {
    let o = owed(w, instance, order_token, chain, pack, token)?;
    nothing(&o)?;
    Ok(o)
}

fn nothing(o: &Owed) -> Result<(), Why> {
    if o.amount == "0" {
        return Err(("E_P6_NOTHING".into(), format!("没有可取回的 {}", o.token)));
    }
    Ok(())
}


fn confirm_to<'a>(instance: &'a str, _logic: &'a str) -> &'a str {
    instance
}


pub fn confirm_plan(w: &Wire, at: &At<'_>) -> Result<Plan, Why> {
    a1::chain::code_matches(&w.t, &w.table, at.instance, &w.logic).map_err(a1w)?;
    let (pc, abi) = instance_call(&a1::call::Action::Confirm.call(confirm_to(at.instance, &w.logic)).map_err(a1w)?, a1::sol::CONFIRM)?;
    Ok(Plan { verb: "confirm", calls: vec![pc], originals: vec![abi], items: Vec::new(), face: Vec::new(), instance: at.instance.to_string() })
}

fn wallet_leg(form: Form, need: &U256) -> bool {
    form == Form::Wallet && !need.is_zero()
}


pub fn dispute_plan(w: &Wire, at: &At<'_>, form: Form, wallet: &str, first: Option<&Evidence<'_>>) -> Result<Plan, Why> {
    a1::chain::code_matches(&w.t, &w.table, at.instance, &w.logic).map_err(a1w)?;
    eravon_dispute::refund::dispute_open(&watch_wire(w), at.instance, at.case_id)?;
    let (_, _, need) = refund_need(w, at.instance, at.case_id)?;
    let owed = (!wallet_leg(form, &need) && !need.is_zero()).then(|| a1::precheck::word(&w.t, &w.table, at.token, &need.to_be()));
    let (calls, originals, face) = dispute_material(at, form, wallet, &need, owed.as_deref(), first)?;
    Ok(Plan { verb: "dispute", calls, originals, items: vec![Item::Deadline { name: "t2", who: String::new(), at: at.window_end }], face, instance: at.instance.to_string() })
}

fn dispute_material(at: &At<'_>, form: Form, wallet: &str, need: &U256, owed: Option<&str>, first: Option<&Evidence<'_>>) -> Result<(Vec<a1::proposal::PCall>, Vec<Original>, Vec<String>), Why> {
    let mut calls = Vec::new();
    let mut originals = Vec::new();
    let mut face = Vec::new();
    if wallet_leg(form, need) {
        let (pay, abi) = a1::proposal::wallet_pay(wallet, at.token, at.instance, &need.to_be()).map_err(a1w)?;
        calls.push(pay);
        originals.push(abi);
    } else if let Some(word) = owed {
        face.push(format!("个人垫付:执行之前先把诉讼费 {word} 转到实例 {};提案不动公司的钱。", at.instance));
    }
    let mut funs = vec![a1::sol::DISPUTE];
    calls.push(a1::proposal::of_call(&a1::call::Action::Dispute.call(at.instance).map_err(a1w)?).map_err(a1w)?);
    let mut evidence = None;
    if let Some(ev) = first {
        let case_id = a1::order::b32_of("案号", at.case_id).map_err(a1w)?;
        funs.push(a1::sol::COMMIT);
        calls.push(a1::proposal::of_call(&a1::call::Action::Commit { case_id, manifest: ev.manifest }.call(at.instance).map_err(a1w)?).map_err(a1w)?);
        evidence = ev.bytes.map(|b| Original { kind: a1::proposal::EVIDENCE_DOC_KIND.into(), name: a1::proposal::EVIDENCE_DOC_NAME.into(), bytes: b.to_vec() });
        face.push(a1::proposal::FACE_EVIDENCE.to_string());
    }
    originals.push(instance_abi(&funs)?);
    originals.extend(evidence);
    Ok((calls, originals, face))
}

fn instance_abi(funs: &[a1::sol::Fun]) -> Result<Original, Why> {
    a1::proposal::abi_doc(funs, a1::proposal::INSTANCE_ABI_NAME).map_err(a1w)
}

fn evidence_items(w: &Wire, instance: &str, case_id: &[u8; 32]) -> Result<Vec<Item>, Why> {
    Ok(vec![a1::company::evidence_item(&w.t, &w.table, instance, case_id).map_err(a1w)?])
}


pub fn commit_plan(w: &Wire, at: &At<'_>, ev: &Evidence<'_>) -> Result<Plan, Why> {
    a1::chain::code_matches(&w.t, &w.table, at.instance, &w.logic).map_err(a1w)?;
    let case_id = a1::order::b32_of("案号", at.case_id).map_err(a1w)?;
    let (pc, abi) = instance_call(&a1::call::Action::Commit { case_id, manifest: ev.manifest }.call(at.instance).map_err(a1w)?, a1::sol::COMMIT)?;
    let mut originals = vec![abi];
    if let Some(b) = ev.bytes {
        originals.push(Original { kind: a1::proposal::EVIDENCE_DOC_KIND.into(), name: a1::proposal::EVIDENCE_DOC_NAME.into(), bytes: b.to_vec() });
    }
    Ok(Plan { verb: "commit", calls: vec![pc], originals, items: evidence_items(w, at.instance, &case_id)?, face: vec![a1::proposal::FACE_EVIDENCE.to_string()], instance: at.instance.to_string() })
}

fn settle_extra(_w: &Wire, _company: &str, _digest: &[u8; 32]) -> Result<Vec<(a1::proposal::PCall, Vec<Original>)>, Why> {
    Ok(Vec::new())
}


pub fn settle_plan(w: &Wire, company: &str, instance: &str, st: &k3::settle::Settle, digest: &[u8; 32]) -> Result<Plan, Why> {
    a1::chain::code_matches(&w.t, &w.table, instance, &w.logic).map_err(a1w)?;
    let sig_seller = if st.sig.is_empty() { Vec::new() } else { a1::order::bytes_of("卖家签名", &st.sig).map_err(a1w)? };
    let expiry = U256::from_dec(&st.expiry.to_string()).unwrap_or_else(|| U256::from_be(&[0u8; 32])).to_be();
    let buyer_bps = u16::try_from(st.buyer_bps).map_err(|_| ("E_P6_SETTLE_FORM".to_string(), "买家份额超界".to_string()))?;
    let mut calls = Vec::new();
    let mut originals = Vec::new();
    for (pc, docs) in settle_extra(w, company, digest)? {
        calls.push(pc);
        originals.extend(docs);
    }
    let (pc, abi) = instance_call(&a1::call::Action::Settle { buyer_bps, expiry, sig_buyer: Vec::new(), sig_seller }.call(instance).map_err(a1w)?, a1::sol::SETTLE)?;
    calls.push(pc);
    originals.push(abi);
    Ok(Plan { verb: "settle", calls, originals, items: vec![Item::Deadline { name: "settleExpiry", who: String::new(), at: st.expiry }], face: Vec::new(), instance: instance.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_originals_of_a_proposal_share_a_name() {
        let (instance, token, wallet) = (format!("0x{}", "11".repeat(20)), format!("0x{}", "22".repeat(20)), format!("0x{}", "33".repeat(20)));
        let case_id = format!("0x{}", "44".repeat(32));
        let at = At { instance: &instance, case_id: &case_id, token: &token, window_end: 1 };
        let manifest = [7u8; 32];
        let bytes = b"{\"manifest\":1}".to_vec();
        let firsts = [None, Some(Evidence { manifest, bytes: Some(&bytes) }), Some(Evidence { manifest, bytes: None })];
        let owed = U256::from_dec("5").expect("five");
        let none = U256::from_dec("0").expect("zero");
        let to = a1::order::addr_of("实例", &instance).expect("an address");
        for (form, need, word) in [(Form::Wallet, &owed, None), (Form::Wallet, &none, None), (Form::Personal, &owed, Some("5"))] {
            for first in &firsts {
                let (calls, originals, _) = dispute_material(&at, form, &wallet, need, word, first.as_ref()).expect("the material");
                let mut names: Vec<&str> = originals.iter().map(|o| o.name.as_str()).collect();
                names.sort();
                let n = names.len();
                names.dedup();
                assert_eq!(names.len(), n, "{form:?} need {} first {}: {names:?}", need.to_dec(), first.is_some());
                let abi = originals.iter().find(|o| o.name == a1::proposal::INSTANCE_ABI_NAME).expect("the instance abi");
                let listed = String::from_utf8(abi.bytes.clone()).expect("text");
                for c in calls.iter().filter(|c| c.to == to) {
                    let f = [a1::sol::DISPUTE, a1::sol::COMMIT].into_iter().find(|f| c.data.starts_with(&f.selector())).expect("an instance function");
                    assert!(listed.contains(&format!("\"name\":\"{}\"", f.name)), "{} listed in the one abi: {listed}", f.name);
                }
                assert_eq!(calls.iter().filter(|c| c.to == to).count(), 1 + first.is_some() as usize);
            }
        }
        let mut own = [a1::proposal::INSTANCE_ABI_NAME, a1::proposal::LOGIC_ABI_NAME, a1::proposal::WALLET_ABI_NAME, a1::proposal::EVIDENCE_DOC_NAME, "settle-offer.json"];
        own.sort();
        assert!(own.windows(2).all(|w| w[0] != w[1]), "{own:?}");
    }
}
