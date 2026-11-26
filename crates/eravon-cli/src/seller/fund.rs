use super::*;

fn s1((token, why): (String, String)) -> Fault {
    Fault::Down { layer: "S1", token, why }
}

fn fund_of(c: &Ctx, f: &Flags, who: &str) -> R<String> {
    if let Some(a) = f.opt("--fund") {
        return eravon_payways::ways::fund(&a).map_err(|e| Fault::Down { layer: "S10", token: e.name().to_string(), why: e.detail() });
    }
    let w = eravon_payways::ways::read(&c.data, who).map_err(|e| Fault::Down { layer: "S10", token: e.name().to_string(), why: e.detail() })?;
    if w.fund.is_empty() {
        return Err(Fault::Down { layer: "S1", token: eravon_goods::goods::E_GOODS_FUND_UNSET.into(), why: "未设置金库:payways-set --fund-own true 用卖家钥当金库,或 --fund 填金库地址".into() });
    }
    Ok(w.fund)
}

fn state_rows(st: &eravon_goods::fund::State) -> Vec<(&'static str, Out)> {
    vec![
        ("agreement", Out::S(st.agreement.to_string())),
        ("allowance", Out::S(st.allowance.to_dec())),
        ("available", Out::S(st.available.to_dec())),
        ("balance", Out::S(st.balance.to_dec())),
        ("bound", Out::B(st.bound)),
        ("fund", Out::S(st.fund.clone())),
    ]
}

pub(super) fn fund_check(c: &Ctx, f: &Flags) -> R<Out> {
    let me = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&me))?;
    let token = f.opt("--token").ok_or_else(|| Fault::Arg("要 --token(结算币)".into()))?.to_ascii_lowercase();
    let (who, fund) = match f.opt("--company") {
        Some(_) => {
            let co = crate::company::company(c, f, &w, &me)?;
            (co.address.clone(), co.views.wallet.clone())
        }
        None => (me.clone(), fund_of(c, f, &me)?),
    };
    let st = eravon_goods::fund::state(&w.t, &w.table, &w.logic, &who, &fund, &token).map_err(s1)?;
    let mut rows = state_rows(&st);
    rows.push(("logic", Out::S(w.logic.clone())));
    rows.push(("token", Out::S(token)));
    let mut hints: Vec<Out> = Vec::new();
    if f.opt("--id").is_some() {
        let h = held(c, f, &w, &me)?;
        let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        match eravon_watch::act::check(&sw, &h.o) {
            Ok(p) => {
                rows.push(("deposit", Out::S(p.priced.deposit.to_dec())));
                rows.push(("orders", Out::S(st.orders(&p.priced.deposit).map(|n| n.to_dec()).unwrap_or_default())));
                if let Some(x) = eravon_goods::fund::large_deposit(&p.priced.deposit, &h.o.offer.amount, h.o.offer.buyer_may_open) {
                    hints.push(Out::O(vec![("say", Out::S(x.say())), ("word", Out::S(x.word().to_string()))]));
                }
            }
            Err(r) => rows.push(("act", Out::O(vec![("gate", Out::S(r.gate.to_string())), ("token", Out::S(r.name.clone())), ("why", Out::S(r.why.clone()))]))),
        }
    }
    let mut said: Vec<String> = Vec::new();
    for r in eravon_goods::goods::list(&c.data, &me, "").map_err(goods_down)?.into_iter().filter(|r| !r.draft) {
        let Ok(Some(b)) = eravon_goods::goods::terms_of(&c.data, &me, &r.id) else { continue };
        let Ok(t) = k2::doc::terms(&b) else { continue };
        if let Some(x) = eravon_goods::fund::new_standard(&t.logic) {
            if !said.contains(&x.say()) {
                said.push(x.say());
                hints.push(Out::O(vec![("say", Out::S(x.say())), ("word", Out::S(x.word().to_string()))]));
            }
        }
    }
    rows.push(("hints", Out::A(hints)));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn fund_setup(c: &Ctx, f: &Flags) -> R<Out> {
    if let Some(path) = exporting(f) {
        return fund_setup_company(c, f, &path);
    }
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let token = f.opt("--token").ok_or_else(|| Fault::Arg("要 --token(结算币)".into()))?.to_ascii_lowercase();
    let fund = fund_of(c, f, &who)?;
    let amount = f.opt("--amount").ok_or_else(|| Fault::Arg("要 --amount(给逻辑合约的押金额度,最小单位)".into()))?;
    let amount = k2::u256::U256::from_dec(&amount).ok_or_else(|| Fault::Arg(format!("--amount 要是十进制整数、不过 2^256 − 1:{amount}")))?;
    let st = eravon_goods::fund::state(&w.t, &w.table, &w.logic, &who, &fund, &token).map_err(s1)?;
    let calls = eravon_goods::fund::setup(&w.logic, &token, &who, &fund, &amount, &st.allowance, st.bound, &fund).map_err(s1)?;
    let sellers = eravon_vault::keyset::seller(&c.data, &who).map_err(home::down)?;
    if calls.iter().any(|c| c.data.starts_with(&a1::sol::ACCEPT_FUNDING.selector())) {
        eravon_goods::fund::binding_from(&w.logic, &sellers, &fund, &who).map_err(s1)?;
    }
    let mut rows = state_rows(&st);
    rows.push((
        "calls",
        Out::A(calls.iter().map(|x| Out::O(vec![("data", Out::S(x.hex())), ("from", Out::S(fund.clone())), ("to", Out::S(x.to.clone()))])).collect()),
    ));
    let own = fund.eq_ignore_ascii_case(&who);
    if crate::send::asked(f) && !calls.is_empty() {
        if !own && sellers.iter().any(|k| k.eq_ignore_ascii_case(&fund)) {
            return Err(Fault::Down { layer: "S1", token: eravon_goods::fund::E_GOODS_FUND_SELLER.into(), why: "金库是本席另一把卖家钥:先改金库".into() });
        }
        let txs = crate::send::series(calls.iter(), |call| crate::send::deliver(c, f, &w, &fund, call, "0"))?;
        rows.push(("txs", Out::A(txs.into_iter().map(Out::S).collect())));
    } else {
        rows.push(("sent", Out::B(false)));
        rows.push(("why", Out::S(if calls.is_empty() { "额度与登记已就位,无需再发".into() } else { if own { "金库就是卖家钥:给 --send true 由这把钥发".into() } else { "这几笔由金库钱包自己发:把 calls 交给金库钱包,或金库是本库的钥时给 --send true".into() } })));
    }
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn wallet_send(c: &Ctx, f: &Flags) -> R<Out> {
    let to = f.opt("--to").ok_or_else(|| Fault::Arg("要 --to".into()))?;
    let again = f.opt("--to-again").ok_or_else(|| Fault::Arg("要 --to-again(再填一遍转入地址)".into()))?;
    if !again.trim().eq_ignore_ascii_case(to.trim()) {
        return Err(Fault::Down { layer: "P7", token: E_P7_AGAIN.into(), why: "两遍地址不一样".into() });
    }
    crate::buyer::withdraw_to(c, f)
}

pub const E_P7_AGAIN: &str = "E_P7_AGAIN";

fn fund_setup_company(c: &Ctx, f: &Flags, path: &str) -> R<Out> {
    let me = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&me))?;
    let co = crate::company::company(c, f, &w, &me)?;
    let token = f.opt("--token").ok_or_else(|| Fault::Arg("要 --token(结算币)".into()))?.to_ascii_lowercase();
    let amount = f.opt("--amount").ok_or_else(|| Fault::Arg("要 --amount(给逻辑合约的押金额度,最小单位)".into()))?;
    let amount = k2::u256::U256::from_dec(&amount).ok_or_else(|| Fault::Arg(format!("--amount 要是十进制整数、不过 2^256 − 1:{amount}")))?;
    let gate = time_gate(crate::company::time_switch(c, f, &me)?);
    let set = company_setup(&w, &co, &token, &amount)?;
    let mut rows = state_rows(&set.state);
    rows.push(("allow", Out::O(crate::company::export(c, &w, &co, path, gate, set.allow)?)));
    if let Some(bind) = set.bind {
        let at = bind_path(path)?;
        rows.push(("fund", Out::O(crate::company::export(c, &w, &co, &at, gate, bind)?)));
    }
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub fn bind_path(path: &str) -> R<String> {
    let name = std::path::Path::new(path).file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
    crate::seam::proposal_doc_path(path, &format!("fund-{name}")).map_err(Fault::Arg)
}

pub struct CompanySetup {
    pub state: eravon_goods::fund::State,
    pub allow: crate::company::Export,
    pub bind: Option<crate::company::Export>,
}

pub fn company_setup(w: &shared::Wire, co: &crate::company::Company, token: &str, amount: &k2::u256::U256) -> R<CompanySetup> {
    let token = token.to_ascii_lowercase();
    let wallet = co.views.wallet.clone();
    let state = eravon_goods::fund::state(&w.t, &w.table, &w.logic, &co.address, &wallet, &token).map_err(s1)?;
    let mut allow = Vec::new();
    let mut originals = Vec::new();
    if !state.allowance.is_zero() && state.allowance != *amount {
        let (zero, abi) = a1::proposal::wallet_allow(&wallet, &token, &w.logic, &[0u8; 32])?;
        allow.push(zero);
        originals.push(abi);
    }
    let (n, abi) = a1::proposal::wallet_allow(&wallet, &token, &w.logic, &amount.to_be())?;
    allow.push(n);
    originals.push(abi);
    let face = vec![format!("公司钱包 {wallet} 给逻辑合约 {} 的押金额度设为 {}(币种 {token})", w.logic, amount.to_dec())];
    let allow = crate::company::Export { verb: "fund-allow", calls: allow, originals, items: Vec::new(), face, deal: crate::company::Deal::None };
    let bind = if state.bound {
        None
    } else {
        let (bind, abi) = a1::proposal::wallet_fund(&wallet, &w.logic)?;
        let face = vec![format!("公司钱包 {wallet} 在逻辑合约 {} 上登记为本公司的押金地址(acceptFunding)", w.logic)];
        Some(crate::company::Export { verb: "fund-bind", calls: vec![bind], originals: vec![abi], items: Vec::new(), face, deal: crate::company::Deal::None })
    };
    Ok(CompanySetup { state, allow, bind })
}
