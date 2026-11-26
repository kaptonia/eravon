use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_literals::constants as k7;

use crate::abi::{self, Val};
use crate::call::Call;
use crate::chain;
use crate::fault::{Fault, R};
use crate::order;
use crate::precheck::GATE_CURRENT;
use crate::sol;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gate {
    Open,
    Keys(Vec<String>),
    Raw(Vec<String>),
    Case(Roles),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Roles {
    pub seller: bool,
    pub buyer: bool,
    pub judges: bool,
}

fn is(data: &[u8], f: &sol::Fun) -> bool {
    data.len() >= 4 && data[..4] == f.selector()
}

fn args_of(f: &sol::Fun, data: &[u8]) -> R<Vec<Val>> {
    let open = f.sig.find('(').ok_or_else(|| Fault::Abi(format!("{} 的签名不成形", f.name)))?;
    abi::decode(&abi::tuple_types(&f.sig[open..])?, data.get(4..).unwrap_or_default())
}

fn addr_arg(v: &[Val], i: usize) -> R<String> {
    Ok(order::hex_of(&v.get(i).ok_or_else(|| Fault::Abi(format!("缺第 {i} 个参数")))?.addr()?))
}

pub fn gate_of(call: &Call) -> R<Gate> {
    let d = &call.data;
    if call.to.eq_ignore_ascii_case(k7::PUBLIC_DEPLOYER) || d.len() < 4 {
        return Ok(Gate::Open);
    }
    if is(d, &sol::INIT) {
        let i = order::decode_init(d)?;
        let o = &i.order;
        let expiry_form = i.quote.is_empty() && i.sig_buyer.is_empty() && i.sig_seller.is_empty();
        let mut keys = vec![o.offer.seller_key.clone()];
        if expiry_form || o.offer.buyer_may_open {
            keys.push(o.buyer_key.clone());
        }
        return Ok(Gate::Keys(keys));
    }
    if is(d, &sol::CANCEL) {
        let v = args_of(&sol::CANCEL, d)?;
        let t = v.first().ok_or_else(|| Fault::Abi("cancel 缺条款".into()))?.items()?;
        use sol::src_abi as k;
        let offer = t.get(k::ERAVON_T_ORDER_TERMS_COL_OFFER).ok_or_else(|| Fault::Abi("cancel 缺要约".into()))?.items()?;
        let seller = order::hex_of(&offer.get(k::ERAVON_T_OFFER_COL_SELLER_KEY).ok_or_else(|| Fault::Abi("cancel 的要约缺卖家钥".into()))?.addr()?);
        let buyer = order::hex_of(&t.get(k::ERAVON_T_ORDER_TERMS_COL_BUYER_KEY).ok_or_else(|| Fault::Abi("cancel 缺买家钥".into()))?.addr()?);
        let signed = !v.get(1).ok_or_else(|| Fault::Abi("cancel 缺签名".into()))?.bytes()?.is_empty();
        return Ok(Gate::Keys(if signed { vec![seller, buyer] } else { vec![buyer] }));
    }
    if is(d, &sol::REVOKE) || is(d, &sol::CONSENT) {
        let f = if is(d, &sol::REVOKE) { sol::REVOKE } else { sol::CONSENT };
        return Ok(Gate::Keys(vec![addr_arg(&args_of(&f, d)?, 0)?]));
    }
    if is(d, &sol::JUDGE_FACE[1]) {
        return Ok(Gate::Keys(vec![addr_arg(&args_of(&sol::JUDGE_FACE[1], d)?, 0)?]));
    }
    if is(d, &sol::JUDGE_FACE[2]) {
        return Ok(Gate::Keys(vec![addr_arg(&args_of(&sol::JUDGE_FACE[2], d)?, 1)?]));
    }
    if is(d, &sol::TRIAL_INIT) {
        let i = order::decode_init(d)?;
        let expiry_form = i.quote.is_empty() && i.sig_buyer.is_empty() && i.sig_seller.is_empty();
        return Ok(if expiry_form || i.order.offer.buyer_may_open { Gate::Open } else { Gate::Raw(vec![i.order.offer.seller_key.clone()]) });
    }
    if is(d, &sol::TRIAL_CANCEL) {
        let v = args_of(&sol::TRIAL_CANCEL, d)?;
        let t = v.first().ok_or_else(|| Fault::Abi("cancel 缺条款".into()))?.items()?;
        use sol::src_abi as k;
        let buyer = order::hex_of(&t.get(k::ERAVON_T_ORDER_TERMS_COL_BUYER_KEY).ok_or_else(|| Fault::Abi("cancel 缺买家钥".into()))?.addr()?);
        let signed = !v.get(1).ok_or_else(|| Fault::Abi("cancel 缺签名".into()))?.bytes()?.is_empty();
        return Ok(if signed { Gate::Open } else { Gate::Raw(vec![buyer]) });
    }
    let r = |seller, buyer, judges| Ok(Gate::Case(Roles { seller, buyer, judges }));
    if is(d, &sol::CLAIM_DELIVERY) || is(d, &sol::REFUND) {
        return r(true, false, false);
    }
    if is(d, &sol::CONFIRM) || is(d, &sol::DISPUTE) {
        return r(false, true, false);
    }
    if [sol::CLAIM_DELIVERY_BY_SIG, sol::CONFIRM_BY_SIG, sol::DISPUTE_BY_SIG, sol::REFUND_BY_SIG, sol::SETTLE].iter().any(|f| is(d, f)) {
        return r(true, true, false);
    }
    if is(d, &sol::SWEEP) {
        return r(true, true, true);
    }
    Ok(Gate::Open)
}

fn past_open(d: &[u8]) -> bool {
    [sol::CLAIM_DELIVERY_BY_SIG, sol::CONFIRM_BY_SIG, sol::DISPUTE_BY_SIG, sol::REFUND_BY_SIG, sol::SETTLE, sol::SWEEP].iter().any(|f| is(d, f))
}

pub fn has_party_door(call: &Call) -> R<bool> {
    if gate_of(call)? != Gate::Open {
        return Ok(true);
    }
    let deployment = call.to.eq_ignore_ascii_case(k7::PUBLIC_DEPLOYER);
    Ok(!deployment && [sol::TRIAL_INIT, sol::TRIAL_CANCEL, sol::TRIAL_REVOKE, sol::TRIAL_CONSENT].iter().any(|f| is(&call.data, f)))
}

fn case_keys<T: Transport>(t: &T, table: &Table, at: &str, roles: Roles) -> R<(Vec<String>, bool)> {
    let s = chain::stage(t, table, at)?.value;
    let mut keys = Vec::new();
    if roles.seller {
        keys.push(s.seller_key.clone());
    }
    if roles.buyer {
        keys.push(s.buyer_key.clone());
    }
    if roles.judges {
        let st = chain::status(t, table, at)?.value;
        let seats = st.get(chain::status_col::SEATS).ok_or_else(|| Fault::Abi(format!("status 的答缺第 {} 栏", chain::status_col::SEATS)))?.small()?;
        chain::ahead_seats(t, table, &(0..seats).map(|seat| (at.to_string(), s.case_id, seat)).collect::<Vec<_>>());
        for seat in 0..seats {
            let v = chain::seat_of(t, table, at, &s.case_id, seat)?.value;
            keys.push(order::hex_of(&v.get(chain::seat_col::JUDGE).ok_or_else(|| Fault::Abi("seatOf 的答缺判官".into()))?.addr()?));
        }
    }
    Ok((keys, s.past))
}


pub fn sender_current<T: Transport>(t: &T, table: &Table, sender: &str, call: &Call) -> R<()> {
    let (keys, raw) = match gate_of(call)? {
        Gate::Open => return Ok(()),
        Gate::Keys(k) => (k, false),
        Gate::Raw(k) => (k, true),
        Gate::Case(roles) => {
            if past_open(&call.data) && chain::code_generation(t, table, &call.to, k7::active_standard().logic)?.value.is_past() {
                return Ok(());
            }
            case_keys(t, table, &call.to, roles)?
        }
    };
    if !raw {
        chain::ahead_current(t, table, &keys);
    }
    let mut seen = Vec::with_capacity(keys.len());
    for k in &keys {
        let now = chain::acting_key(t, table, k, raw)?;
        if now.eq_ignore_ascii_case(sender) {
            return Ok(());
        }
        seen.push(format!("{k} → {now}"));
    }
    let name = sol::ACTIONS.iter().chain(sol::REGISTRY_ACTIONS.iter()).chain(sol::EXPORTED.iter()).chain(sol::JUDGE_FACE.iter())
        .chain([sol::INIT, sol::CANCEL].iter())
        .find(|f| is(&call.data, f))
        .map_or("这一笔", |f| f.name);
    Err(Fault::Precheck {
        gate: GATE_CURRENT,
        why: format!("NotCurrent:{name} 只收 {} 的现行钥,发送钥 {sender} 不是;一笔也没发", seen.join("、")),
    })
}

pub fn proposal_senders<T: Transport>(t: &T, table: &Table, company: &str, calls: &[crate::proposal::PCall]) -> R<()> {
    for c in calls {
        sender_current(t, table, company, &Call { to: order::hex_of(&c.to), data: c.data.clone() })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eravon_terms as k2;

    fn a(c: char) -> String {
        format!("0x{}", c.to_string().repeat(40))
    }

    fn sample(buyer_may_open: bool) -> order::Order {
        order::Order {
            offer: k2::derive::Offer {
                seller_key: a('1'),
                judge: a('2'),
                guild: a('0'),
                seller_payout: a('3'),
                seller_fund: a('4'),
                token: a('5'),
                amount: k2::u256::U256::from_dec("1000").expect("十进制"),
                delivery_window: 1,
                confirm_window: 1,
                open_window: 1,
                buyer_may_open,
                ladder: vec![k2::doc::Rung { buyer_bps: 0, seller_bps: 10_000, fee_bearer: 0 }],
                terms_hash: [0x11; 32],
                amanah_hash: [0x22; 32],
                quote_hash: [0x33; 32],
                nonce: 1,
                expiry: 1,
            },
            buyer_key: a('6'),
            buyer_payout: a('7'),
            open_by: 1,
            prev: order::NO_PREV.to_string(),
        }
    }

    #[test]
    fn each_door_names_its_keys() {
        use crate::call::{self, Action, BySig};
        let (s, b) = (a('1'), a('6'));
        let inst = a('9');
        let init = |o: &order::Order, q: &[u8], sb: &[u8], ss: &[u8]| call::init(o, &inst, q, sb, ss, &[], &[]).unwrap();
        assert_eq!(gate_of(&init(&sample(false), &[1], &[0; 65], &[0; 65])).unwrap(), Gate::Keys(vec![s.clone()]), "buyerMayOpen 假:只卖家");
        assert_eq!(gate_of(&init(&sample(true), &[1], &[0; 65], &[0; 65])).unwrap(), Gate::Keys(vec![s.clone(), b.clone()]), "buyerMayOpen 真:两方");
        assert_eq!(gate_of(&call::expire_init(&sample(false), &inst).unwrap()).unwrap(), Gate::Keys(vec![s.clone(), b.clone()]), "过期支路:任一方");
        assert_eq!(gate_of(&call::cancel(&sample(false), &inst, &[]).unwrap()).unwrap(), Gate::Keys(vec![b.clone()]), "空签撤单:只买家");
        assert_eq!(gate_of(&call::cancel(&sample(false), &inst, &[0; 65]).unwrap()).unwrap(), Gate::Keys(vec![s.clone(), b.clone()]), "带签撤单:任一方");
        let roles = |x: Action| gate_of(&x.call(&inst).unwrap()).unwrap();
        assert_eq!(roles(Action::Refund), Gate::Case(Roles { seller: true, ..Roles::default() }));
        assert_eq!(roles(Action::ClaimDelivery([1; 32])), Gate::Case(Roles { seller: true, ..Roles::default() }));
        assert_eq!(roles(Action::Confirm), Gate::Case(Roles { buyer: true, ..Roles::default() }));
        assert_eq!(roles(Action::Dispute), Gate::Case(Roles { buyer: true, ..Roles::default() }));
        assert_eq!(roles(Action::Settle { buyer_bps: 0, expiry: [0; 32], sig_buyer: vec![], sig_seller: vec![] }), Gate::Case(Roles { seller: true, buyer: true, ..Roles::default() }));
        assert_eq!(roles(Action::Redraw { members: vec![], amanah_hashes: vec![] }), Gate::Open, "抽签对任何发送者开(§7)");
        assert_eq!(roles(Action::Sweep { who: s.clone(), token: b.clone() }), Gate::Case(Roles { seller: true, buyer: true, judges: true }));
        assert_eq!(roles(Action::Advance { case_id: [1; 32], seat: 0 }), Gate::Open, "到期戳谁都能发");
        assert_eq!(roles(Action::Commit { case_id: [1; 32], manifest: [2; 32] }), Gate::Open);
        let by = BySig::Confirm { sig: vec![0; 65] }.call(&inst).unwrap();
        assert_eq!(gate_of(&by).unwrap(), Gate::Case(Roles { seller: true, buyer: true, ..Roles::default() }));
        let con = Call { to: a('8'), data: call::with(sol::CONSENT, &[Val::Addr(order::addr_of("x", &b).unwrap()), Val::B32([3; 32])]).unwrap() };
        assert_eq!(gate_of(&con).unwrap(), Gate::Keys(vec![b.clone()]));
        assert_eq!(gate_of(&call::revoke(&a('8'), &s, &[3; 32]).unwrap()).unwrap(), Gate::Keys(vec![s.clone()]));
        let w = Call { to: inst.clone(), data: call::with(sol::JUDGE_FACE[2], &[Val::Addr([5; 20]), Val::Addr([2; 20]), Val::Addr([3; 20])]).unwrap() };
        assert_eq!(gate_of(&w).unwrap(), Gate::Keys(vec![order::hex_of(&[2; 20])]), "withdraw 认第二参的判官");
        let dep = Call { to: k7::PUBLIC_DEPLOYER.into(), data: call::cancel(&sample(false), &inst, &[]).unwrap().data };
        assert_eq!(gate_of(&dep).unwrap(), Gate::Open, "部署器那一笔是部署");
        assert_eq!(gate_of(&call::erc20_transfer(&a('5'), &inst, &[0; 32]).unwrap()).unwrap(), Gate::Open);
    }

    #[test]
    fn the_trial_generations_doors_are_its_own() {
        use crate::call;
        let (s, b) = (a('1'), a('6'));
        let inst = a('9');
        let trial = |o: &order::Order, quote: &[u8], sig: &[u8]| Call {
            to: inst.clone(),
            data: call::with(sol::TRIAL_INIT, &[o.abi_of(k7::Shape::Trial).unwrap(), Val::Bytes(quote.to_vec()), Val::Bytes(sig.to_vec()), Val::Bytes(sig.to_vec()), Val::Arr(Vec::new()), Val::Arr(Vec::new())]).unwrap(),
        };
        assert_eq!(gate_of(&trial(&sample(false), &[1], &[0; 65])).unwrap(), Gate::Raw(vec![s.clone()]), "buyerMayOpen 假:只卖家原钥");
        assert_eq!(gate_of(&trial(&sample(true), &[1], &[0; 65])).unwrap(), Gate::Open, "buyerMayOpen 真:往代谁都能开");
        assert_eq!(gate_of(&trial(&sample(false), &[], &[])).unwrap(), Gate::Open, "往代过期支路:谁都能发");
        let cancel = |sig: &[u8]| Call { to: inst.clone(), data: call::with(sol::TRIAL_CANCEL, &[sample(false).abi_of(k7::Shape::Trial).unwrap(), Val::Bytes(sig.to_vec())]).unwrap() };
        assert_eq!(gate_of(&cancel(&[])).unwrap(), Gate::Raw(vec![b.clone()]), "空签撤单:只买家原钥");
        assert_eq!(gate_of(&cancel(&[0; 65])).unwrap(), Gate::Open, "带签撤单:往代谁都能递");
        for f in [sol::CLAIM_DELIVERY_BY_SIG, sol::CONFIRM_BY_SIG, sol::DISPUTE_BY_SIG, sol::REFUND_BY_SIG, sol::SETTLE, sol::SWEEP] {
            assert!(past_open(&f.selector()), "{} 在往代不问发送者", f.name);
        }
        for f in [sol::CLAIM_DELIVERY, sol::REFUND, sol::CONFIRM, sol::DISPUTE] {
            assert!(!past_open(&f.selector()), "{} 在往代照比原钥", f.name);
        }
    }
}
