use eravon_literals::constants as k7;
use eravon_terms as k2;

use crate::abi::{self, Val};
use crate::fault::{Fault, R};
use crate::order::{self, Order};
use crate::sol;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub to: String,
    pub data: Vec<u8>,
}

impl Call {
    pub fn hex(&self) -> String {
        order::hex_of(&self.data)
    }
}

pub(crate) fn with(f: sol::Fun, args: &[Val]) -> R<Vec<u8>> {
    let open = f.sig.find('(').ok_or_else(|| Fault::Abi(format!("{} 的签名不成形", f.name)))?;
    abi::fits(&f.sig[open..], args).map_err(|e| Fault::Abi(format!("{}:{}", f.name, e.detail())))?;
    let mut d = f.selector().to_vec();
    d.extend_from_slice(&abi::encode(args));
    Ok(d)
}

pub fn deployer() -> &'static str {
    k7::PUBLIC_DEPLOYER
}

pub fn deploy_logic(creation_code: &[u8]) -> R<Call> {
    if creation_code.is_empty() {
        return Err(Fault::Form("逻辑合约的初始码不许空".into()));
    }
    let salt = k2::derive::keccak(eravon_literals::literals::SALT_LOGIC.as_bytes());
    let mut data = salt.to_vec();
    data.extend_from_slice(creation_code);
    Ok(Call { to: deployer().to_string(), data })
}

pub fn deploy_verifier(creation_code: &[u8]) -> R<Call> {
    if creation_code.is_empty() {
        return Err(Fault::Form("验证器的初始码不许空".into()));
    }
    let salt = k2::derive::keccak(eravon_literals::literals::SALT_VERIFIER.as_bytes());
    let mut data = salt.to_vec();
    data.extend_from_slice(creation_code);
    Ok(Call { to: deployer().to_string(), data })
}

pub fn deploy_board(creation_code: &[u8]) -> R<Call> {
    if creation_code.is_empty() {
        return Err(Fault::Form("公告板的初始码不许空".into()));
    }
    let salt = k2::derive::keccak(eravon_literals::literals::SALT_BOARD.as_bytes());
    let mut data = salt.to_vec();
    data.extend_from_slice(creation_code);
    Ok(Call { to: deployer().to_string(), data })
}

pub fn deploy_instance(case_id: &[u8; 32], logic: &str) -> R<Call> {
    let mut data = case_id.to_vec();
    data.extend_from_slice(&k2::derive::initcode(logic));
    Ok(Call { to: deployer().to_string(), data })
}


pub fn expire_init(o: &Order, instance: &str) -> R<Call> {
    init(o, instance, &[], &[], &[], &[], &[])
}

fn shape_at(logic: &str) -> k7::Shape {
    k7::generation_of_logic(logic).map_or(k7::Shape::Current, |g| g.shape())
}

pub fn expire_init_at(o: &Order, instance: &str, logic: &str) -> R<Call> {
    match shape_at(logic) {
        k7::Shape::Current => expire_init(o, instance),
        k7::Shape::Trial => Ok(Call {
            to: instance.to_string(),
            data: with(
                sol::TRIAL_INIT,
                &[o.abi_of(k7::Shape::Trial)?, Val::Bytes(Vec::new()), Val::Bytes(Vec::new()), Val::Bytes(Vec::new()), Val::Arr(Vec::new()), Val::Arr(Vec::new())],
            )?,
        }),
    }
}

pub fn cancel_at(o: &Order, instance: &str, sig: &[u8], logic: &str) -> R<Call> {
    match shape_at(logic) {
        k7::Shape::Current => cancel(o, instance, sig),
        k7::Shape::Trial => {
            if !sig.is_empty() && sig.len() != 65 {
                return Err(Fault::Form(format!("撤单签名是六十五字节,给的是 {}", sig.len())));
            }
            Ok(Call { to: instance.to_string(), data: with(sol::TRIAL_CANCEL, &[o.abi_of(k7::Shape::Trial)?, Val::Bytes(sig.to_vec())])? })
        }
    }
}

pub fn init(
    o: &Order,
    instance: &str,
    quote: &[u8],
    sig_buyer: &[u8],
    sig_seller: &[u8],
    members: &[String],
    amanah_hashes: &[[u8; 32]],
) -> R<Call> {
    if members.len() != amanah_hashes.len() {
        return Err(Fault::Form(format!(
            "名单 {} 位而判法词 {} 枚,两表要等长",
            members.len(),
            amanah_hashes.len()
        )));
    }
    let mut ms = Vec::with_capacity(members.len());
    for m in members {
        ms.push(Val::Addr(order::addr_of("名单里的地址", m)?));
    }
    let data = with(
        sol::INIT,
        &[
            o.abi()?,
            Val::Bytes(quote.to_vec()),
            Val::Bytes(sig_buyer.to_vec()),
            Val::Bytes(sig_seller.to_vec()),
            Val::Arr(ms),
            Val::Arr(amanah_hashes.iter().map(|h| Val::B32(*h)).collect()),
        ],
    )?;
    Ok(Call { to: instance.to_string(), data })
}

pub fn erc20_transfer(token: &str, to: &str, amount: &[u8; 32]) -> R<Call> {
    let data = with(
        sol::ERC20_TRANSFER,
        &[Val::Addr(order::addr_of("收件地址", to)?), Val::B32(*amount)],
    )?;
    Ok(Call { to: order::addr_norm("结算币", token)?, data })
}

pub fn erc20_approve(token: &str, spender: &str, amount: &[u8; 32]) -> R<Call> {
    let data = with(sol::ERC20_APPROVE, &[Val::Addr(order::addr_of("被授权人", spender)?), Val::B32(*amount)])?;
    Ok(Call { to: order::addr_norm("结算币", token)?, data })
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permit {
    pub owner: String,
    pub spender: String,
    pub value: [u8; 32],
    pub deadline: u64,
    pub sig: Vec<u8>,
}

pub const PERMIT_TYPE: &str = "Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)";

pub fn permit_digest(domain: &[u8; 32], owner: &str, spender: &str, value: &[u8; 32], nonce: &[u8; 32], deadline: u64) -> R<[u8; 32]> {
    let mut body = k2::derive::keccak(PERMIT_TYPE.as_bytes()).to_vec();
    body.extend_from_slice(&abi::encode(&[Val::Addr(order::addr_of("授权人", owner)?), Val::Addr(order::addr_of("被授权人", spender)?), Val::B32(*value)]));
    body.extend_from_slice(&permit_nonce(nonce));
    body.extend_from_slice(&abi::encode(&[Val::u64(deadline)]));
    let mut pre = vec![0x19u8, 0x01];
    pre.extend_from_slice(domain);
    pre.extend_from_slice(&k2::derive::keccak(&body));
    Ok(k2::derive::keccak(&pre))
}

fn permit_nonce(nonce: &[u8; 32]) -> Vec<u8> {
    nonce.to_vec()
}


pub fn erc20_permit(token: &str, p: &Permit) -> R<Call> {
    if p.sig.len() != 65 {
        return Err(Fault::Form(format!("permit 签名要 65 字节,给的是 {}", p.sig.len())));
    }
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&p.sig[..32]);
    s.copy_from_slice(&p.sig[32..64]);
    let v = match p.sig[64] {
        0 | 1 => u64::from(p.sig[64]) + 27,
        x => u64::from(x),
    };
    let data = with(
        sol::ERC20_PERMIT,
        &[
            Val::Addr(order::addr_of("授权人", &p.owner)?),
            Val::Addr(order::addr_of("被授权人", &p.spender)?),
            Val::B32(p.value),
            Val::u64(p.deadline),
            Val::u64(v),
            Val::B32(r),
            Val::B32(s),
        ],
    )?;
    Ok(Call { to: order::addr_norm("结算币", token)?, data })
}

pub fn erc20_nonces(owner: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::ERC20_NONCES, &[Val::Addr(*owner)])
}

pub fn merged(steps: &[&Call]) -> R<Call> {
    if steps.len() < 2 {
        return Err(Fault::Form("合并一笔至少要两件".into()));
    }
    if let Some(c) = steps.iter().find(|c| crate::gate::has_party_door(c).unwrap_or(true)) {
        return Err(Fault::Precheck {
            gate: crate::precheck::GATE_CALLER,
            why: format!("发往 {} 的那一件有当事人门:只由那一方的现行钥直发,不经 Multicall3", c.to),
        });
    }
    let to = k7::MULTICALL3
        .get()
        .ok_or_else(|| Fault::Form("MULTICALL3 这一槽还没有值".into()))?;
    let mut items = Vec::with_capacity(steps.len());
    for c in steps {
        items.push(Val::Tuple(vec![
            Val::Addr(order::addr_of("收件地址", &c.to)?),
            Val::Bool(false),
            Val::Bytes(c.data.clone()),
        ]));
    }
    Ok(Call { to: (*to).to_string(), data: with(sol::AGGREGATE3, &[Val::Arr(items)])? })
}

pub fn is_init(data: &[u8]) -> bool {
    data.len() >= 4 && (data[..4] == sol::INIT.selector() || data[..4] == sol::TRIAL_INIT.selector())
}

pub fn cancel(o: &Order, instance: &str, sig: &[u8]) -> R<Call> {
    if !sig.is_empty() && sig.len() != 65 {
        return Err(Fault::Form(format!("撤单签名是六十五字节,给的是 {}", sig.len())));
    }
    Ok(Call { to: instance.to_string(), data: with(sol::CANCEL, &[o.abi()?, Val::Bytes(sig.to_vec())])? })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    ClaimDelivery([u8; 32]),
    Confirm,
    Dispute,
    Refund,
    Settle { buyer_bps: u16, expiry: [u8; 32], sig_buyer: Vec<u8>, sig_seller: Vec<u8> },
    Commit { case_id: [u8; 32], manifest: [u8; 32] },
    Advance { case_id: [u8; 32], seat: u64 },
    Redraw { members: Vec<String>, amanah_hashes: Vec<[u8; 32]> },
    Sweep { who: String, token: String },
}

impl Action {
    pub fn fun(&self) -> sol::Fun {
        match self {
            Action::ClaimDelivery(_) => sol::CLAIM_DELIVERY,
            Action::Confirm => sol::CONFIRM,
            Action::Dispute => sol::DISPUTE,
            Action::Refund => sol::REFUND,
            Action::Settle { .. } => sol::SETTLE,
            Action::Commit { .. } => sol::COMMIT,
            Action::Advance { .. } => sol::ADVANCE,
            Action::Redraw { .. } => sol::REDRAW,
            Action::Sweep { .. } => sol::SWEEP,
        }
    }

    pub fn args(&self) -> R<Vec<Val>> {
        self.args_inner(0)
    }


    fn args_inner(&self, seat_shift: u64) -> R<Vec<Val>> {
        Ok(match self {
            Action::Confirm | Action::Refund | Action::Dispute => Vec::new(),
            Action::ClaimDelivery(m) => {
                if *m == [0u8; 32] {
                    return Err(Fault::Form("交付清单摘要不许是零(合约拒零)".into()));
                }
                vec![Val::B32(*m)]
            }
            Action::Settle { buyer_bps, expiry, sig_buyer, sig_seller } => {
                if *buyer_bps > 10_000 {
                    return Err(Fault::Form(format!("买家份额至多 10000,给的是 {buyer_bps}")));
                }
                vec![
                    Val::u64(u64::from(*buyer_bps)),
                    Val::U256(*expiry),
                    Val::Bytes(sig_buyer.clone()),
                    Val::Bytes(sig_seller.clone()),
                ]
            }
            Action::Commit { case_id, manifest } => vec![Val::B32(*case_id), Val::B32(*manifest)],
            Action::Advance { case_id, seat } => vec![Val::B32(*case_id), Val::u64(seat.saturating_sub(seat_shift))],
            Action::Redraw { members, amanah_hashes } => {
                if members.len() != amanah_hashes.len() {
                    return Err(Fault::Form("名单与判法词两表要等长".into()));
                }
                let mut ms = Vec::with_capacity(members.len());
                for m in members {
                    ms.push(Val::Addr(order::addr_of("名单里的地址", m)?));
                }
                vec![
                    Val::Arr(ms),
                    Val::Arr(amanah_hashes.iter().map(|h| Val::B32(*h)).collect()),
                ]
            }
            Action::Sweep { who, token } => vec![
                Val::Addr(order::addr_of("取回人", who)?),
                Val::Addr(order::addr_of("币", token)?),
            ],
        })
    }

    pub fn call(&self, instance: &str) -> R<Call> {
        Ok(Call { to: instance.to_string(), data: with(self.fun(), &self.args()?)? })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BySig {
    ClaimDelivery { manifest: [u8; 32], sig: Vec<u8> },
    Confirm { sig: Vec<u8> },
    Dispute { sig: Vec<u8> },
    Refund { sig: Vec<u8> },
}

impl BySig {
    pub fn fun(&self) -> sol::Fun {
        match self {
            BySig::ClaimDelivery { .. } => sol::CLAIM_DELIVERY_BY_SIG,
            BySig::Confirm { .. } => sol::CONFIRM_BY_SIG,
            BySig::Dispute { .. } => sol::DISPUTE_BY_SIG,
            BySig::Refund { .. } => sol::REFUND_BY_SIG,
        }
    }

    pub fn sig(&self) -> &[u8] {
        match self {
            BySig::ClaimDelivery { sig, .. }
            | BySig::Confirm { sig }
            | BySig::Dispute { sig }
            | BySig::Refund { sig } => sig,
        }
    }

    pub fn call(&self, instance: &str) -> R<Call> {
        self.call_with(instance, self.sig().to_vec())
    }


    fn call_with(&self, instance: &str, sig: Vec<u8>) -> R<Call> {
        if sig.is_empty() {
            return Err(Fault::Precheck {
                gate: crate::precheck::GATE_SIG,
                why: "四类 …BySig 不认空槽:以调用同意的一方对这一动作直接调实例".into(),
            });
        }
        self.form_of(&sig)?;
        let args = match self {
            BySig::ClaimDelivery { manifest, .. } => {
                vec![Val::B32(*manifest), Val::Bytes(sig)]
            }
            BySig::Confirm { .. } | BySig::Refund { .. } | BySig::Dispute { .. } => vec![Val::Bytes(sig)],
        };
        Ok(Call { to: instance.to_string(), data: with(self.fun(), &args)? })
    }
}

impl BySig {
    fn form_of(&self, sig: &[u8]) -> R<()> {
        if sig.len() == 65 {
            return Ok(());
        }
        let who = if matches!(self, BySig::Confirm { .. } | BySig::Dispute { .. }) { "买家" } else { "卖家" };
        Err(Fault::Form(format!("{who}的签名是六十五字节,给的是 {}", sig.len())))
    }
}

pub fn view(f: sol::Fun) -> R<Vec<u8>> {
    with(f, &[])
}

pub fn seat_of(case_id: &[u8; 32], seat: u64) -> R<Vec<u8>> {
    with(sol::SEAT_OF, &[Val::B32(*case_id), Val::u64(seat)])
}

pub fn account_of(who: &[u8; 20], token: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::ACCOUNT_OF, &[Val::Addr(*who), Val::Addr(*token)])
}

pub fn book_count(root: &[u8; 20], key: &[u8; 32]) -> R<Vec<u8>> {
    with(sol::BOOK_COUNT, &[Val::Addr(*root), Val::B32(*key)])
}

pub fn book_panel_at(root: &[u8; 20], key: &[u8; 32], index: u64) -> R<Vec<u8>> {
    with(sol::BOOK_PANEL_AT, &[Val::Addr(*root), Val::B32(*key), Val::u64(index)])
}

pub fn succession_current(a: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::SUCCESSION_CURRENT, &[Val::Addr(*a)])
}

pub fn consent(logic: &str, party: &str, digest: &[u8; 32]) -> R<Call> {
    Ok(Call { to: order::addr_norm("逻辑合约", logic)?, data: with(sol::CONSENT, &[Val::Addr(order::addr_of("当事人", party)?), Val::B32(*digest)])? })
}

pub fn declare(to: &str) -> R<Call> {
    Ok(Call { to: eravon_literals::constants::SUCCESSION_BOOK.to_string(), data: with(sol::SUCCESSION_DECLARE, &[Val::Addr(order::addr_of("新钥", to)?)])? })
}

pub fn accede(holder: &str) -> R<Call> {
    Ok(Call { to: eravon_literals::constants::SUCCESSION_BOOK.to_string(), data: with(sol::SUCCESSION_ACCEDE, &[Val::Addr(order::addr_of("旧钥", holder)?)])? })
}

pub fn succession_pending(a: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::SUCCESSION_PENDING, &[Val::Addr(*a)])
}

pub fn succession_entered(a: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::SUCCESSION_ENTERED, &[Val::Addr(*a)])
}

pub fn succession_root(a: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::SUCCESSION_ROOT, &[Val::Addr(*a)])
}

pub fn erc20_balance_of(who: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::ERC20_BALANCE_OF, &[Val::Addr(*who)])
}

pub fn feed_decimals() -> R<Vec<u8>> {
    with(sol::FEED_DECIMALS, &[])
}

pub fn feed_latest_round() -> R<Vec<u8>> {
    with(sol::FEED_LATEST_ROUND, &[])
}

pub fn erc20_allowance(owner: &[u8; 20], spender: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::ERC20_ALLOWANCE, &[Val::Addr(*owner), Val::Addr(*spender)])
}

pub fn revoke(logic: &str, seller: &str, terms_hash: &[u8; 32]) -> R<Call> {
    Ok(Call {
        to: order::addr_norm("逻辑合约", logic)?,
        data: with(sol::REVOKE, &[Val::Addr(order::addr_of("卖家钥", seller)?), Val::B32(*terms_hash)])?,
    })
}

pub fn accept_funding(logic: &str, seller: &str) -> R<Call> {
    Ok(Call { to: order::addr_norm("逻辑合约", logic)?, data: with(sol::ACCEPT_FUNDING, &[Val::Addr(order::addr_of("卖家钥", seller)?)])? })
}

pub fn post(board: &str, key: &[u8; 32], document: &[u8]) -> R<Call> {
    Ok(Call { to: order::addr_norm("公告板", board)?, data: with(sol::POST, &[Val::B32(*key), Val::Bytes(document.to_vec())])? })
}

pub fn revoked(seller: &[u8; 20], terms_hash: &[u8; 32]) -> R<Vec<u8>> {
    with(sol::REVOKED, &[Val::Addr(*seller), Val::B32(*terms_hash)])
}

pub fn consented(who: &[u8; 20], digest: &[u8; 32]) -> R<Vec<u8>> {
    with(sol::CONSENTED, &[Val::Addr(*who), Val::B32(*digest)])
}

pub fn funds(seller: &[u8; 20], fund: &[u8; 20]) -> R<Vec<u8>> {
    with(sol::FUNDS, &[Val::Addr(*seller), Val::Addr(*fund)])
}

pub fn hash_offer(o: &Order) -> R<Vec<u8>> {
    with(sol::HASH_OFFER, &[o.offer_abi()?])
}

#[cfg(test)]
mod tests {
    use super::*;

    const INST: &str = "0x9c7e62ac34d50b936ad58dfd1a1a895eaf52098f";

    fn carries(c: &Call, sig: &[u8]) -> bool {
        c.data.windows(sig.len()).any(|w| w == sig)
    }

    #[test]
    fn the_65_byte_form_is_taken_for_every_party() {
        let sig = vec![9u8; 65];
        for b in [BySig::ClaimDelivery { manifest: [1u8; 32], sig: sig.clone() }, BySig::Confirm { sig: sig.clone() }, BySig::Dispute { sig: sig.clone() }, BySig::Refund { sig: sig.clone() }] {
            assert!(carries(&b.call(INST).expect("六十五字节照收"), &sig));
        }
    }

    #[test]
    fn no_signature_but_65_bytes_is_read() {
        for b in [BySig::Confirm { sig: vec![5u8; 192] }, BySig::Dispute { sig: vec![5u8; 100] }, BySig::Refund { sig: vec![5u8; 64] }] {
            assert!(matches!(b.call(INST), Err(Fault::Form(_))), "只认六十五字节");
        }
    }

    #[test]
    fn a_call_whose_arguments_do_not_fit_its_signature_is_refused_by_name() {
        match with(sol::REVOKE, &[Val::B32([1u8; 32])]) {
            Err(Fault::Abi(why)) => assert!(why.contains("revoke"), "{why}"),
            other => panic!("少一参该拒:{other:?}"),
        }
        match with(sol::CONSENT, &[Val::B32([1u8; 32])]) {
            Err(Fault::Abi(why)) => assert!(why.contains("consent"), "{why}"),
            other => panic!("少一参该拒:{other:?}"),
        }
        assert!(with(sol::REVOKE, &[Val::Addr([2u8; 20]), Val::B32([1u8; 32])]).is_ok());
    }

    #[test]
    fn the_call_form_is_a_direct_call_not_a_relayed_one() {
        for b in [BySig::Confirm { sig: Vec::new() }, BySig::Refund { sig: Vec::new() }] {
            match b.call(INST) {
                Err(Fault::Precheck { why, .. }) => assert!(why.contains("直接调实例"), "{why}"),
                other => panic!("空槽该拒:{other:?}"),
            }
        }
    }
}
