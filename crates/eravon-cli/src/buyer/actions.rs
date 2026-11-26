use super::*;

pub(super) fn act(c: &Ctx, f: &Flags, a: a1::call::Action) -> R<Out> {
    act_with(c, f, a, None)
}

pub(super) fn act_with(c: &Ctx, f: &Flags, a: a1::call::Action, evidence: Option<&[u8]>) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    a1::chain::code_matches(&w.t, &w.table, &instance, &w.logic)?;
    let call = a.call(&instance)?;
    if let Some(path) = exporting(f) {
        return company_export(c, f, &who, &path, |w, co| {
            let (case_id, token, window_end) = co_case(w, &instance)?;
            let at = crate::after_company::At { instance: &instance, case_id: &case_id, token: &token, window_end };
            let p = match &a {
                a1::call::Action::Confirm => crate::after_company::confirm_plan(w, &at),
                a1::call::Action::Dispute => crate::after_company::dispute_plan(w, &at, crate::after_company::Form::Personal, &co.views.wallet, None),
                a1::call::Action::Commit { manifest, .. } => crate::after_company::commit_plan(w, &at, &crate::after_company::Evidence { manifest: *manifest, bytes: evidence }),
                other => return Err(Fault::Arg(format!("{} 不出公司提案", other.fun().name))),
            }
            .map_err(p6)?;
            Ok(export_of(p))
        });
    }
    let mut rows = vec![
        ("action", Out::S(a.fun().name.to_string())),
        ("instance", Out::S(instance)),
        ("needsSeller", Out::B(false)),
    ];
    rows.extend(shared::deliver_rows(c, f, &w, &who, &call, "0")?);
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn confirm(c: &Ctx, f: &Flags) -> R<Out> {
    let mut answer = act(c, f, a1::call::Action::Confirm)?;
    if let (Some(pass), None) = (f.one("--passcode"), exporting(f)) {
        let who = c.who(f.one("--address"))?;
        let key = c.key(&who, Some(pass))?;
        let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
        let chain_id = home::chain_id(f.opt_int("--chain")?);
        let d = bysig_digest(chain_id, &instance, &k2::derive::Action::Confirm);
        let sig = shared::sign(&key, &d)?;
        let bs = a1::call::BySig::Confirm {
            sig: a1::order::bytes_of("sig", &sig)?,
        };
        let call = bs.call(&instance)?;
        let w = shared::wire(f, c, Some(&who))?;
        let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        eravon_dispute::settle::check_pasted(&dw, &instance, eravon_dispute::settle::Paste::Confirm, &sig, &[]).map_err(p6)?;
        let recorded = record_confirm(c, &who, &instance, &sig)?;
        if let Out::O(m) = &mut answer {
            m.push((
                "bySig",
                Out::O(vec![
                    ("data", Out::S(call.hex())),
                    ("digest", Out::S(hex(&d))),
                    ("recorded", Out::B(recorded)),
                    ("sig", Out::S(sig)),
                    ("to", Out::S(call.to.clone())),
                ]),
            ));
        }
    }
    Ok(answer)
}

pub(super) fn bysig_digest(chain_id: u64, instance: &str, act: &k2::derive::Action) -> [u8; 32] {
    k2::derive::digest(chain_id, instance, &act.struct_hash())
}


#[derive(Clone, Debug)]
pub struct Pull {
    pub way: &'static str,
    pub calls: Vec<(&'static str, a1::call::Call)>,
}

#[allow(clippy::too_many_arguments)]
pub fn pull_calls(w: &shared::Wire, key: &[u8; 32], buyer: &str, instance: &str, token: &str, need: &k2::u256::U256, now: u64) -> R<Pull> {
    let dispute = a1::call::Action::Dispute.call(instance)?;
    if need.is_zero() {
        return Ok(Pull { way: "none", calls: vec![("dispute", dispute)] });
    }
    let allowed = k2::u256::U256::from_be(&a1::chain::erc20_allowance(&w.t, &w.table, token, buyer, instance)?.value);
    if allowed.cmp_u(need) != std::cmp::Ordering::Less {
        return Ok(Pull { way: "approve", calls: vec![("dispute", dispute)] });
    }
    if let Some((domain, nonce)) = a1::chain::permit_facts(&w.t, &w.table, token, buyer)? {
        let deadline = now.saturating_add(PERMIT_TTL);
        let pd = a1::call::permit_digest(&domain, buyer, instance, &need.to_be(), &nonce, deadline)?;
        let psig = a1::order::bytes_of("permit 签名", &shared::sign(key, &pd)?)?;
        let permit = a1::call::erc20_permit(token, &a1::call::Permit { owner: buyer.to_string(), spender: instance.to_string(), value: need.to_be(), deadline, sig: psig })?;
        match a1::chain::call_raw(&w.t, &w.table, &permit.to, &permit.data) {
            Ok(_) => return Ok(Pull { way: "permit", calls: vec![("permit", permit), ("dispute", dispute)] }),
            Err(e) if a1::chain::reverted(&e) => {}
            Err(e) => return Err(e.into()),
        }
    }
    let mut calls = Vec::new();
    if !allowed.is_zero() {
        calls.push(("reset", a1::call::erc20_approve(token, instance, &[0u8; 32])?));
    }
    calls.push(("approve", a1::call::erc20_approve(token, instance, &need.to_be())?));
    calls.push(("dispute", dispute));
    Ok(Pull { way: "approve", calls })
}

pub(super) fn dispute_pull(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    a1::chain::code_matches(&w.t, &w.table, &instance, &w.logic)?;
    dispute_gate(c, &who, &w, &instance)?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (case_id, _, _, _) = eravon_dispute::settle::opened(&dw, &instance).map_err(p6)?;
    let (fee, frozen) = crate::after_company::pull_fee(&w, &instance, &case_id, &who).map_err(p6)?;
    let (token, need) = (fee.token.clone(), fee.need);
    if let Some(given) = f.one("--token") {
        if !given.eq_ignore_ascii_case(&token) {
            return Err(p6(("E_P6_FEE_GIVEN".into(), format!("--token {given} 不是这一单的结算币 {token}"))));
        }
    }
    if let Some(given) = shared::amount_flag(f, "--amount", "--amount-text", || shared::decimals_for(&c.data, Some(&w), &token))? {
        if k2::u256::U256::from_dec(&given).is_none_or(|g| g.cmp_u(&need) != std::cmp::Ordering::Equal) {
            return Err(p6(("E_P6_FEE_GIVEN".into(), format!("--amount {given} 不是还差的诉讼费 {}", need.to_dec()))));
        }
    }
    let now = a1::precheck::chain_now(&w.t, &w.table)?;
    let mut key = c.key(&who, f.one("--passcode"))?;
    let got = pull_calls(&w, &key, &frozen, &instance, &token, &need, now);
    for b in key.iter_mut() {
        *b = 0;
    }
    let _ = std::hint::black_box(&key);
    let p = got?;
    let mut rows = vec![
        ("action", Out::S("dispute".into())),
        ("instance", Out::S(instance.clone())),
        ("needsSeller", Out::B(false)),
        ("pull", Out::O(vec![
            ("calls", Out::A(p.calls.iter().map(|(k, c)| Out::O(vec![("data", Out::S(c.hex())), ("kind", Out::S((*k).into())), ("to", Out::S(c.to.clone()))])).collect())),
            ("way", Out::S(p.way.into())),
        ])),
    ];
    let mut sent: Vec<Out> = Vec::new();
    if crate::send::asked(f) {
        let txs = crate::send::series(p.calls.iter().map(|(_, call)| call), |call| crate::send::deliver(c, f, &w, &who, call, "0"))?;
        sent = txs.into_iter().map(Out::S).collect();
    }
    rows.push(("txs", Out::A(sent)));
    rows.extend(w.rows());
    Ok(ok(rows))
}

fn dispute_gate(c: &Ctx, who: &str, w: &shared::Wire, instance: &str) -> R<()> {
    let owners = super::case::owners(c, who)?;
    let ow = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    refund_after(eravon_orders::table::handed_on(&c.data, &owners, instance, &ow).map_err(p6)?).map_err(p6)?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (case_id, _, _, _) = eravon_dispute::settle::opened(&dw, instance).map_err(p6)?;
    eravon_dispute::refund::dispute_open(&dw, instance, &case_id).map_err(p6)
}

pub fn refund_after(handed: bool) -> Result<(), (String, String)> {
    eravon_orders::status::refund_open(handed)
}

pub(super) fn dispute(c: &Ctx, f: &Flags) -> R<Out> {
    if pull_route(f)? {
        return dispute_pull(c, f);
    }
    if let Some(path) = exporting(f) {
        match f.one("--advance").unwrap_or("wallet") {
            "personal" => {}
            "wallet" => return dispute_wallet_export(c, f, &path),
            other => return Err(Fault::Arg(format!("--advance 只收 wallet 与 personal,给的是 {other:?}"))),
        }
    }
    let mut answer = if exporting(f).is_some() {
        act(c, f, a1::call::Action::Dispute)?
    } else {
        dispute_send(c, f)?
    };
    if let (Some(pass), None) = (f.one("--passcode"), exporting(f)) {
        let who = c.who(f.one("--address"))?;
        let key = c.key(&who, Some(pass))?;
        let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
        let chain_id = home::chain_id(f.opt_int("--chain")?);
        let d = bysig_digest(chain_id, &instance, &k2::derive::Action::Dispute);
        let sig = shared::sign(&key, &d)?;
        let bs = a1::call::BySig::Dispute { sig: a1::order::bytes_of("sig", &sig)? };
        let call = bs.call(&instance)?;
        if let Out::O(m) = &mut answer {
            m.push((
                "bySig",
                Out::O(vec![
                    ("data", Out::S(call.hex())),
                    ("digest", Out::S(hex(&d))),
                    ("sig", Out::S(sig)),
                    ("to", Out::S(call.to.clone())),
                ]),
            ));
        }
    }
    Ok(answer)
}

fn dispute_send(c: &Ctx, f: &Flags) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    a1::chain::code_matches(&w.t, &w.table, &instance, &w.logic)?;
    dispute_gate(c, &who, &w, &instance)?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (case_id, _, _, _) = eravon_dispute::settle::opened(&dw, &instance).map_err(p6)?;
    let fee = crate::after_company::refund_fee(&w, &instance, &case_id, &who).map_err(p6)?;
    let (lf, stray, need) = (fee.lf, fee.stray, fee.need);
    let dispute = a1::call::Action::Dispute.call(&instance)?;
    let mut rows = vec![
        ("action", Out::S("dispute".into())),
        ("fee", Out::O(vec![("lf", Out::S(lf.to_dec())), ("need", Out::S(need.to_dec())), ("stray", Out::S(stray.to_dec()))])),
        ("instance", Out::S(instance.clone())),
        ("needsSeller", Out::B(false)),
    ];
    if need.is_zero() {
        rows.extend(shared::deliver_rows(c, f, &w, &who, &dispute, "0")?);
    } else {
        let pay = a1::call::erc20_transfer(&fee.token, &instance, &need.to_be())?;
        let (first, second) = shared::deliver_two(c, f, &w, &who, &pay, &dispute)?;
        rows.push(("feeTx", Out::O(first)));
        if let Some(second) = second {
            rows.push(("dispute", Out::O(second)));
        }
        rows.extend(shared::tx_rows(&dispute, "0"));
    }
    rows.extend(w.rows());
    Ok(ok(rows))
}

fn settle_offer(c: &Ctx, f: &Flags, who: &str) -> R<k3::settle::Settle> {
    let text = f.opt("--settle").ok_or_else(|| Fault::Arg("要 --settle".into()))?;
    let unread = match home::read_bounded(&text) {
        Ok(b) => return k3::settle::settle(&b).map_err(Fault::from),
        Err(e) => e,
    };
    let link = match k3::transport::decode_link(text.trim().as_bytes()).map_err(Fault::from) {
        Ok((l, _)) => l,
        Err(Fault::Down { layer, token, .. }) => {
            return Err(Fault::Down { layer, token, why: format!("--settle 既不是读得到的档({}),也不是链接文字(eravon://open?e=…)", unread.detail()) });
        }
        Err(other) => return Err(other),
    };
    crate::deskops::settle::offer_by_link(&shared::wire(f, c, Some(who))?, &link)
}

pub(super) fn settle_sign(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let st = settle_offer(c, f, &who)?;
    super::case::past_at(c, &who, &st.instance)?;
    let d = settle_consent(c, f, &who, &st)?.digest;
    if let Some(path) = exporting(f) {
        return company_export(c, f, &who, &path, |w, co| {
            Ok(export_of(crate::after_company::settle_plan(w, &co.address, &st.instance, &st, &d).map_err(p6)?))
        });
    }
    let key = c.key(&who, f.one("--passcode"))?;
    let sig = shared::sign(&key, &d)?;
    Ok(ok(vec![
        ("buyerBps", Out::N(st.buyer_bps)),
        ("digest", Out::S(hex(&d))),
        ("expiry", Out::N(st.expiry)),
        ("instance", Out::S(st.instance.clone())),
        ("sigBuyer", Out::S(sig)),
        ("sigSeller", Out::S(st.sig.clone())),
    ]))
}

pub(super) fn settle(c: &Ctx, f: &Flags) -> R<Out> {
    let buyer_key = c.who(f.one("--address"))?;
    let me = c.who(None)?;
    let st = settle_offer(c, f, &buyer_key)?;
    let d = settle_consent(c, f, &buyer_key, &st)?.digest;
    let w = shared::wire(f, c, Some(&me))?;
    let by_call = f.flag("--by-call");
    let s = crate::deskops::settle::Settlement { instance: &st.instance, buyer_key: &buyer_key, offer: &st, digest: &d };
    let own = buyer_key.eq_ignore_ascii_case(&me);
    let pass = f.one("--passcode");
    let mut rows = crate::deskops::settle::submit(&w, &s, &me, own, by_call, &|k: &str| c.key(k, pass), |from, call| shared::deliver_rows(c, f, &w, from, call, "0"))?;
    rows.splice(0..0, [("action", Out::S("settle".into())), ("instance", Out::S(st.instance.clone())), ("needsSeller", Out::B(false))]);
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn commit(c: &Ctx, f: &Flags) -> R<Out> {
    let case_id = a1::order::b32_of("--case", &f.opt("--case").ok_or_else(|| Fault::Arg("要 --case".into()))?)?;
    if f.opt("--manifest-dir").is_some() || f.opt("--kit").is_some() {
        return commit_built(c, f, case_id);
    }
    let (manifest, bytes) = shared::evidence_arg(f)?;
    act_with(c, f, a1::call::Action::Commit { case_id, manifest }, bytes.as_deref())
}

fn commit_built(c: &Ctx, f: &Flags, case_id: [u8; 32]) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (_, _, buyer_key, _) = eravon_dispute::settle::opened(&dw, &instance).map_err(p6)?;
    let case = a1::order::hex_of(&case_id);
    let ev = shared::evidence_built(c, f, &who, &buyer_key, &instance, &case, &w)?.ok_or_else(|| Fault::Arg("要 --manifest-dir 或 --kit".into()))?;
    let Out::O(mut m) = act_with(c, f, a1::call::Action::Commit { case_id, manifest: ev.manifest }, Some(&ev.bytes))? else { unreachable!("an answer is an object") };
    let tx = m.iter().find_map(|(k, v)| match v {
        Out::S(s) if *k == "txHash" => Some(s.clone()),
        _ => None,
    });
    if let (Some(tx), None) = (tx, exporting(f)) {
        let name = format!("evidence-{}.json", &tx.trim_start_matches("0x")[..8.min(tx.len().saturating_sub(2))]);
        let kept = super::case::keep_evidence(c, &who, &instance, &case, &name, ev.bytes.clone());
        eravon_watch::send::after_send(tx, kept.map_err(|e| (e.name().to_string(), e.detail()))).map_err(p6)?;
    }
    m.extend(shared::evidence_rows(&ev));
    Ok(Out::O(m))
}

pub(super) fn advance(c: &Ctx, f: &Flags) -> R<Out> {
    let case_id = a1::order::b32_of("--case", &f.opt("--case").ok_or_else(|| Fault::Arg("要 --case".into()))?)?;
    let seat = f.int("--seat").map_err(|_| Fault::Arg("要 --seat".into()))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    shared::advance_gate(f, c, &instance, &case_id, seat)?;
    act(c, f, a1::call::Action::Advance { case_id, seat })
}

pub(super) fn redraw(c: &Ctx, f: &Flags) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let (members, amanah_hashes) = shared::redraw_lists(f, &w, &instance)?;
    act(c, f, a1::call::Action::Redraw { members, amanah_hashes })
}

pub(super) fn sweep(c: &Ctx, f: &Flags) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let pack = super::case::pack_of(c, &who, &instance)?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let chain = match eravon_dispute::settle::opened(&dw, &instance) {
        Ok((case_id, _, _, at)) => Some(eravon_dispute::view::case(&dw, &instance, &case_id, at).map_err(p6)?),
        Err((n, _)) if n == "E_DISPUTE_NOT_OPEN" => None,
        Err(e) => return Err(p6(e)),
    };
    let order_token = chain.as_ref().map(|v| v.token.clone()).or_else(|| pack.as_deref().and_then(|b| k3::pack::pack(b).ok()).map(|p| p.listing.token)).unwrap_or_default();
    let o = crate::after_company::sweep_due(&w, &instance, &order_token, chain.as_ref(), pack.as_deref(), f.one("--token")).map_err(p6)?;
    if let Some(given) = f.one("--who") {
        if !given.eq_ignore_ascii_case(&o.payout) {
            return Err(p6(("E_P6_SWEEP_WHO".into(), format!("--who {given} 不是这一单的退款地址 {}", o.payout))));
        }
    }
    act(c, f, a1::call::Action::Sweep { who: o.payout, token: o.token })
}

fn p7((token, why): (String, String)) -> Fault {
    Fault::Down { layer: "P7", token, why }
}

pub(crate) fn withdraw_to(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let to = f.opt("--to").ok_or_else(|| Fault::Arg("要 --to".into()))?;
    let token = f.opt("--token").ok_or_else(|| Fault::Arg("要 --token".into()))?;
    let to = shared::send_plan(Some(&token), None, &to).map_err(p7)?;
    let have = a1::chain::erc20_balance(&w.t, &w.table, &token, &who)?;
    let amount = match shared::amount_flag(f, "--amount", "--amount-text", || shared::decimals_for(&c.data, Some(&w), &token))? {
        Some(a) => a,
        None => k2::u256::U256::from_be(&have.value).to_dec(),
    };
    let want = k2::u256::U256::from_dec(&amount)
        .ok_or_else(|| Fault::Arg(format!("--amount 要是十进制整数:{amount}")))?;
    shared::send_amount(&want, &amount).map_err(p7)?;
    let at_hand = a1::precheck::funds(&w.t, &w.table, &token, &who, &want.to_be())?;
    let call = erc20_transfer(&token, &to, &amount)?;
    let balance = k2::u256::U256::from_be(&at_hand).to_dec();
    let d = if shared::units_asked(f) { shared::decimals_for(&c.data, Some(&w), &token) } else { None };
    let mut rows = shared::units_rows(f, "amount", &amount, d);
    rows.extend(shared::units_rows(f, "balance", &balance, d));
    rows.extend(vec![("amount", Out::S(amount)), ("balance", Out::S(balance)), ("to", Out::S(to)), ("token", Out::S(token))]);
    rows.extend(shared::deliver_rows(c, f, &w, &who, &call, "0")?);
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn wallet_balance(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let mut rows = vec![("address", Out::S(who.clone()))];
    if let Some(token) = f.one("--token") {
        let b = a1::chain::erc20_balance(&w.t, &w.table, token, &who)?;
        rows.push(("agreement", Out::S(b.agreement.to_string())));
        rows.push(("balance", Out::S(k2::u256::U256::from_be(&b.value).to_dec())));
        rows.push(("token", Out::S(token.to_string())));
    } else {
        let (bal, r) = eravon_chain::read::balance(&w.t, &w.table, &who, "latest")?;
        rows.push(("agreement", Out::S(r.agreement.word().to_string())));
        rows.push(("native", Out::S(bal)));
    }
    if f.one("--address").is_none() {
        let keys = eravon_vault::store::keys_of(&c.data, &who).unwrap_or_default();
        let mut formers = Vec::new();
        for k in keys.into_iter().filter(|k| !k.eq_ignore_ascii_case(&who)) {
            let held = match f.one("--token") {
                Some(token) => a1::chain::erc20_balance(&w.t, &w.table, token, &k).map(|b| k2::u256::U256::from_be(&b.value).to_dec()).map_err(Fault::from),
                None => eravon_chain::read::balance(&w.t, &w.table, &k, "latest").map(|(b, _)| b).map_err(Fault::from),
            };
            if !shared::former_movable(&w, &who, f.one("--token"), &held) {
                continue;
            }
            match held {
                Ok(b) => formers.push(Out::O(vec![("address", Out::S(k)), ("balance", Out::S(b))])),
                Err(e) => formers.push(Out::O(vec![("address", Out::S(k)), ("why", Out::S(format!("{}:{}", e.name(), e.detail())))])),
            }
        }
        rows.push(("formers", Out::A(formers)));
    }
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn wallet_send(c: &Ctx, f: &Flags) -> R<Out> {
    withdraw_to(c, f)
}

pub(crate) fn wallet_former_move(c: &Ctx, f: &Flags) -> R<Out> {
    use crate::former;
    let who = c.who(f.one("--address"))?.to_ascii_lowercase();
    let from = f.opt("--former").ok_or_else(|| Fault::Arg("要 --former".into()))?;
    let from = h2::payout::accept(from.trim()).map_err(home::down)?;
    let keys = h2::store::keys_of(&c.data, &who)?;
    if from.eq_ignore_ascii_case(&who) || !keys.iter().any(|k| k.eq_ignore_ascii_case(&from)) {
        return Err(home::down(h2::vfault::Fault::NoIdentity(format!("{from} 不是这一席 {who} 的前任钥"))));
    }
    let w = shared::wire(f, c, Some(&who))?;
    let mut coins = Vec::new();
    if let Some(t) = f.one("--token") {
        let t = h2::payout::accept(t.trim()).map_err(home::down)?;
        let held = k2::u256::U256::from_be(&a1::chain::erc20_balance(&w.t, &w.table, &t, &from)?.value).to_dec();
        if shared::former_movable(&w, &who, Some(&t), &Ok::<_, Fault>(held.clone())) {
            coins.push((t, held));
        }
    }
    let (native, _) = eravon_chain::read::balance(&w.t, &w.table, &from, "latest")?;
    if coins.is_empty() && !shared::former_movable(&w, &who, None, &Ok::<_, Fault>(native.clone())) {
        return Err(former::p7(former::AMOUNT, format!("{from} 上没有可转到 {who} 的")));
    }
    let eth = native != "0";
    let leg = |token: &str, amount: &str, tx: Option<&str>| {
        let mut v = vec![("amount", Out::S(amount.to_string())), ("token", Out::S(token.to_string()))];
        if let Some(t) = tx {
            v.push(("tx", Out::S(t.to_string())));
        }
        Out::O(v)
    };
    let moves = coins.iter().map(|(t, a)| leg(t, a, None)).chain(eth.then(|| leg("", &native, None))).collect();
    let mut rows = vec![("address", Out::S(who.clone())), ("from", Out::S(from.clone())), ("moves", Out::A(moves))];
    if !crate::send::asked(f) {
        rows.push(("sent", Out::B(false)));
        rows.push(("why", Out::S("没给 --send true,故只说这一手要转的".into())));
        rows.extend(w.rows());
        return Ok(ok(rows));
    }
    h2::session::need_open(&*c.session(None)?).map_err(home::down)?;
    let base = crate::send::Opts::of(f)?;
    let mut post = |p: &former::Post<'_>| -> R<String> {
        let d = crate::send::direct(&|| c.key(p.from, None), p.opts, &w, p.from, p.call, p.value)?;
        match d.verdict() {
            crate::send::Verdict::Mined => Ok(d.tx_hash),
            crate::send::Verdict::Reverted => Err(Fault::Reverted { tx_hash: d.tx_hash }),
            crate::send::Verdict::Unknown(why) => Err(match d.receipt {
                Err(e) => e,
                Ok(_) => Fault::Unconfirmed { tx_hash: d.tx_hash, why },
            }),
        }
    };
    let tokens: Vec<String> = coins.into_iter().map(|(t, _)| t).collect();
    let m = former::run(&w, &from, &who, &tokens, eth, &base, &mut post, &|| true);
    if let Some(former::Stop::Token(e) | former::Stop::Eth(e)) = m.stop {
        return Err(if m.sent.is_empty() { e } else { Fault::Series { sent: m.sent.into_iter().map(|t| (t, "mined")).collect(), stop: Box::new(e) } });
    }
    let legs = |v: &[former::Leg]| Out::A(v.iter().map(|l| leg(&l.token, &l.amount, Some(&l.tx))).collect());
    rows.push(("moved", legs(&m.moved)));
    rows.push(("paid", legs(&m.paid)));
    rows.push(("sent", Out::B(true)));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn wallet_tx(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let hash = f.opt("--tx").ok_or_else(|| Fault::Arg("要 --tx".into()))?;
    let (tx, r) = eravon_chain::read::transaction(&w.t, &w.table, &hash)?;
    let get = |k: &str| tx.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let mut rows = vec![
        ("agreement", Out::S(r.agreement.word().to_string())),
        ("blockNumber", Out::S(get("blockNumber"))),
        ("from", Out::S(get("from"))),
        ("hash", Out::S(hash)),
        ("to", Out::S(get("to"))),
        ("value", Out::S(get("value"))),
    ];
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn expire(c: &Ctx, f: &Flags) -> R<Out> {
    if let Some(path) = exporting(f) {
        let who = c.who(f.one("--address"))?;
        return company_export(c, f, &who, &path, |w, _| {
            let h = seller::held(c, f, w, &who)?;
            expire_export(w, &h.o, &h.logic, c.now)
        });
    }
    seller::expire(c, f)
}

pub(super) fn gate_of(c: &Ctx, f: &Flags, p: &k3::listing::Listing) -> Vec<eravon_roster::gate::Admission> {
    if p.by_call.is_empty() {
        return candidate_gate(None, p);
    }
    let who = c.who(f.one("--address")).ok();
    let wire = shared::wire(f, c, who.as_deref()).ok();
    candidate_gate(wire.as_ref(), p)
}

pub fn candidate_gate(wire: Option<&shared::Wire>, p: &k3::listing::Listing) -> Vec<eravon_roster::gate::Admission> {
    use eravon_roster::gate::{self, Registration};
    let n = p.candidate_count();
    if p.by_call.is_empty() {
        return gate::gate(n, &[], &mut |_, _| Registration::Unverified);
    }
    let std_ok = p.logic.eq_ignore_ascii_case(eravon_literals::constants::active_standard().logic);
    let wire = if std_ok { wire } else { None };
    if let Some(w) = wire {
        let digests: Vec<[u8; 32]> = p.by_call.iter().filter_map(|(_, d)| a1::order::b32_of("要约摘要", d).ok()).collect();
        a1::consent::ahead_registered(&w.t, &w.table, &p.logic, &p.order.seller_key, &digests);
    }
    let mut read = |_: usize, d: &str| -> Registration {
        let Some(w) = wire else { return Registration::Unverified };
        let Ok(digest) = a1::order::b32_of("要约摘要", d) else { return Registration::Unverified };
        match a1::consent::registered(&w.t, &w.table, &p.logic, &p.order.seller_key, &digest).state {
            a1::consent::Registration::Held => Registration::Held,
            a1::consent::Registration::NotHeld => Registration::NotHeld,
            a1::consent::Registration::Unverified => Registration::Unverified,
        }
    };
    gate::gate(n, &p.by_call, &mut read)
}

pub(super) fn gate_rows(p: &k3::listing::Listing, g: &[eravon_roster::gate::Admission], page: Option<&[usize]>) -> Vec<(&'static str, Out)> {
    vec![
        (
            "gate",
            Out::A(
                g.iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let digest =
                            p.by_call.iter().find(|(c, _)| *c == i).map(|(_, d)| d.clone()).unwrap_or_default();
                        Out::O(vec![
                            ("candidate", Out::N(i as u64)),
                            ("digest", Out::S(digest)),
                            ("note", Out::S(a.note().to_string())),
                            ("state", Out::S(a.word().to_string())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("choosable", Out::A(choosable_of(g, page).into_iter().map(|n| Out::N(n as u64)).collect())),
    ]
}

pub(super) fn choosable_of(g: &[eravon_roster::gate::Admission], page: Option<&[usize]>) -> Vec<usize> {
    match page {
        Some(ns) => ns.to_vec(),
        None => g.iter().enumerate().filter(|(_, a)| a.choosable()).map(|(i, _)| i).collect(),
    }
}

#[cfg(test)]
mod choosable {
    use eravon_roster::gate::Admission;

    #[test]
    fn the_list_is_the_pages_where_there_is_a_page() {
        let gate = [Admission::Admitted, Admission::Signed, Admission::Hidden("x")];
        assert_eq!(super::choosable_of(&gate, Some(&[1])), vec![1], "停接新案那一位不入表");
        assert_eq!(gate[0].word(), "admitted", "候选门那一判照旧");
        assert_eq!(super::choosable_of(&gate, Some(&[])), Vec::<usize>::new(), "页整页拒即空表");
        assert_eq!(super::choosable_of(&gate, None), vec![0, 1], "不带身份照候选门列");
    }
}

pub(super) fn must_pass(g: &[eravon_roster::gate::Admission], n: u64) -> R<()> {
    match g.get(n as usize) {
        Some(a) if !a.choosable() => Err(Fault::State(format!("候选 {n} 以调用同意而未放行:{}", a.note()))),
        _ => Ok(()),
    }
}


pub(super) fn settle_consent(c: &Ctx, f: &Flags, who: &str, st: &k3::settle::Settle) -> R<crate::deskops::settle::Consent> {
    let pack = super::case::pack_of(c, who, &st.instance)?;
    let w = shared::wire(f, c, Some(who))?;
    crate::deskops::settle::consent(&w, &st.instance, pack.as_deref(), "", st).map_err(Fault::from)
}
