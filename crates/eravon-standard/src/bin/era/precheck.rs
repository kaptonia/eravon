use eravon_literals::misuse::Misuse;
use super::*;

pub(super) fn precheck(f: &Flags) -> Result<Out, Misuse> {
    let what = need(f, "--what")?;
    ans((|| {
        match what.as_str() {
            "sig" => {
                let path = flag(f, "--order")?;
                let (chain, logic) = {
                    let (c, l, _) = a1::chain::standard_table();
                    let c = match f.one("--chain") {
                        None => c,
                        Some(s) => s.parse::<u64>().map_err(|_| Fault::Form("--chain 不是数".into()))?,
                    };
                    (c, f.one("--logic").unwrap_or(l).to_string())
                };
                let o = read_order(&path)?;
                a1::precheck::signatures(
                    &o,
                    chain,
                    &logic,
                    &flag(f, "--sig-seller")?,
                    &flag(f, "--sig-buyer")?,
                )?;
                Ok(ok(vec![("gate", Out::S("sig".into()))]))
            }
            "consent" => {
                let path = flag(f, "--order")?;
                let (chain, logic) = {
                    let (c, l, _) = a1::chain::standard_table();
                    let c = match f.one("--chain") {
                        None => c,
                        Some(s) => s.parse::<u64>().map_err(|_| Fault::Form("--chain 不是数".into()))?,
                    };
                    (c, f.one("--logic").unwrap_or(l).to_string())
                };
                let via_multicall = match f.one("--via").unwrap_or("direct") {
                    "direct" => false,
                    "merged" => true,
                    other => return Err(Fault::Form(format!("--via 只收 direct 与 merged,给的是 {other:?}"))),
                };
                let o = read_order(&path)?;
                let ss = order::bytes_of("卖家签名", &flag(f, "--sig-seller")?)?;
                let sb = order::bytes_of("买家签名", &flag(f, "--sig-buyer")?)?;
                let urls: Vec<String> = f.many("--endpoint").iter().map(|x| (*x).to_string()).collect();
                let w = wire(f)?;
                let table = if urls.is_empty() { None } else { Some(Table::new(urls)?) };
                let mut registry = |who: &str, d: &[u8; 32]| match &table {
                    Some(t) => a1::consent::registered(&w, t, &logic, who, d),
                    None => a1::consent::RegRead {
                        state: a1::consent::Registration::Unverified,
                        endpoints: 0,
                        why: "没给端点,登记簿读不了".into(),
                    },
                };
                let sender = a1::consent::Sender { from: f.one("--sender"), via_multicall };
                let mut current = |k: &str| match &table {
                    Some(t) => a1::chain::current(&w, t, k).map(|r| r.value),
                    None => Ok(k.to_ascii_lowercase()),
                };
                let (s, b) = a1::precheck::consents(&o, chain, &logic, &ss, &sb, sender, &mut current, &mut registry)?;
                Ok(ok(vec![
                    ("buyer", Out::S(b.word().into())),
                    ("gate", Out::S(a1::precheck::GATE_SIG.into())),
                    ("seller", Out::S(s.word().into())),
                ]))
            }
            "sender" => {
                let urls: Vec<String> = f.many("--endpoint").iter().map(|x| (*x).to_string()).collect();
                let w = wire(f)?;
                let t = Table::new(urls)?;
                let call = a1::call::Call { to: order::addr_norm("收件地址", &flag(f, "--to")?)?, data: order::bytes_of("calldata", &flag(f, "--data")?)? };
                a1::gate::sender_current(&w, &t, &flag(f, "--sender")?, &call)?;
                Ok(ok(vec![("gate", Out::S(a1::precheck::GATE_CURRENT.into()))]))
            }
            "window" => {
                a1::precheck::in_window(numf(f, "--window-end")?, numf(f, "--now")?)?;
                Ok(ok(vec![("gate", Out::S("window".into()))]))
            }
            "price" => {
                let t = Table::new(f.many("--endpoint").iter().map(|x| (*x).to_string()).collect())?;
                let w = wire(f)?;
                let (now, source) = match f.one("--now") {
                    Some(_) => (numf(f, "--now")?, "given"),
                    None => (a1::precheck::chain_now(&w, &t)?, "chain"),
                };
                let max_stale = numf(f, "--max-stale")?;
                let r = a1::feed::gate(&w, &t, &flag(f, "--feed")?, now, max_stale)?;
                Ok(ok(vec![
                    ("age", Out::N(now.saturating_sub(r.updated_at))),
                    ("answer", Out::S(r.answer.clone())),
                    ("decimals", Out::N(r.decimals)),
                    ("gate", Out::S("price".into())),
                    ("maxStale", Out::N(max_stale)),
                    ("now", Out::N(now)),
                    ("nowSource", Out::S(source.into())),
                    ("updatedAt", Out::N(r.updated_at)),
                ]))
            }
            "verifier" => {
                let t = Table::new(f.many("--endpoint").iter().map(|x| (*x).to_string()).collect())?;
                let w = wire(f)?;
                let (got, _want) = a1::precheck::verifier_there(&w, &t)?;
                Ok(ok(vec![
                    ("codeHash", Out::S(got)),
                    ("gate", Out::S(a1::precheck::GATE_VERIFIER.into())),
                    ("verifier", Out::S(k7::VERIFIER.to_string())),
                ]))
            }
            "deposit" => {
                let t = Table::new(f.many("--endpoint").iter().map(|x| (*x).to_string()).collect())?;
                let w = wire(f)?;
                let (chain, logic) = {
                    let (c, l, _) = a1::chain::standard_table();
                    let c = match f.one("--chain") {
                        None => c,
                        Some(s) => s.parse::<u64>().map_err(|_| Fault::Form("--chain 不是数".into()))?,
                    };
                    (c, f.one("--logic").unwrap_or(l).to_string())
                };
                a2::read::chain_guard(&w, &t, chain)?;
                let o = read_order(&flag(f, "--order")?)?;
                let quote = order::bytes_of("报价", &flag(f, "--quote")?)?;
                let p = a1::precheck::priced(&w, &t, &o, &quote)?;
                a1::precheck::deposit(&w, &t, &o, &logic, &p)?;
                a1::precheck::not_revoked(&w, &t, &o, &logic)?;
                Ok(ok(vec![
                    ("deposit", Out::S(p.deposit.to_dec())),
                    ("lf", Out::S(p.lf.to_dec())),
                    ("premium", Out::S(p.premium.to_dec())),
                ]))
            }
            "funds" => {
                let t = Table::new(f.many("--endpoint").iter().map(|x| (*x).to_string()).collect())?;
                let w = wire(f)?;
                let path = flag(f, "--order")?;
                let (chain, logic) = {
                    let (c, l, _) = a1::chain::standard_table();
                    let c = match f.one("--chain") {
                        None => c,
                        Some(s) => s.parse::<u64>().map_err(|_| Fault::Form("--chain 不是数".into()))?,
                    };
                    (c, f.one("--logic").unwrap_or(l).to_string())
                };
                a2::read::chain_guard(&w, &t, chain)?;
                let o = read_order(&path)?;
                let d = o.derive(chain, &logic)?;
                let got = a1::precheck::funded(&w, &t, &o, &d.instance)?;
                let decimals = if f.one("--units-text") == Some("true") { a1::chain::erc20_decimals(&w, &t, &o.offer.token).map(|d| d as u8) } else { None };
                let mut m = units_rows(f, "balanceText", "balanceDecimals", &got, decimals);
                m.extend(vec![("balance", Out::S(order::hex_of(&got))), ("gate", Out::S("funds".into()))]);
                Ok(ok(m))
            }
            "allowance" => {
                let t = Table::new(f.many("--endpoint").iter().map(|x| (*x).to_string()).collect())?;
                let w = wire(f)?;
                let (chain, _) = {
                    let (c, l, _) = a1::chain::standard_table();
                    let c = match f.one("--chain") {
                        None => c,
                        Some(s) => s.parse::<u64>().map_err(|_| Fault::Form("--chain 不是数".into()))?,
                    };
                    (c, l)
                };
                a2::read::chain_guard(&w, &t, chain)?;
                let token = flag(f, "--token")?;
                let decimals = || a1::chain::erc20_decimals(&w, &t, &token).map(|d| d as u8);
                let need_w = match amount_arg(f, "--need", "--need-text", decimals) {
                    Ok(given) => word_of("--need", &given?)?,
                    Err(_) => return Err(Fault::Form("缺 --need".into())),
                };
                let got = a1::precheck::allowed(
                    &w,
                    &t,
                    &flag(f, "--token")?,
                    &flag(f, "--owner")?,
                    &flag(f, "--instance")?,
                    &need_w,
                )?;
                let mut m = units_rows(f, "allowanceText", "allowanceDecimals", &got, if f.one("--units-text") == Some("true") { decimals() } else { None });
                m.extend(vec![("allowance", Out::S(order::hex_of(&got))), ("gate", Out::S("allowance".into()))]);
                Ok(ok(m))
            }
            "late" => {
                let accepted = numf(f, "--accepted")? != 0;
                let window_end = numf(f, "--window-end")?;
                let now = numf(f, "--now")?;
                let phase = numf(f, "--phase")?;
                let state = match f.one("--seat-state") {
                    Some(x) => x.parse::<u64>().map_err(|_| Fault::Form("--seat-state 不是数".into()))?,
                    None => 0,
                };
                let seat = Some(a1::precheck::Seat { state, accepted, window_end });
                let p = |c| Out::B(a1::precheck::premium_to_judge(c, phase, seat, now));
                use a1::precheck::Close;
                Ok(ok(vec![
                    ("exit", Out::S(a1::precheck::seat_exit(accepted, window_end, now).word().into())),
                    (
                        "premiumToJudge",
                        Out::O(vec![("confirm", p(Close::Confirm)), ("refund", p(Close::Refund)), ("rule", p(Close::Rule)), ("settle", p(Close::Settle))]),
                    ),
                ]))
            }
            other => Err(Fault::Form(format!("四扇门之外的词:{other:?}"))),
        }
    })())
}

pub(super) fn word_of(k: &'static str, v: &str) -> a1::fault::R<[u8; 32]> {
    if v.starts_with("0x") {
        return order::b32_of(k, &format!("0x{:0>64}", v.trim_start_matches("0x")));
    }
    k2::u256::U256::from_dec(v).map(|x| x.to_be()).ok_or_else(|| Fault::Form(format!("{k} 要是十进制整数或 0x 形")))
}

pub(super) fn approve(f: &Flags) -> Result<Out, Misuse> {
    let given = amount_arg(f, "--amount", "--amount-text", || chain_decimals(f, f.one("--token")))?;
    ans((|| {
        let amount = word_of("--amount", &given?)?;
        let c = a1::call::erc20_approve(&flag(f, "--token")?, &flag(f, "--spender")?, &amount)?;
        Ok(ok(vec![("call", call_out(&c))]))
    })())
}

pub(super) fn permit(f: &Flags) -> Result<Out, Misuse> {
    let given = amount_arg(f, "--amount", "--amount-text", || chain_decimals(f, f.one("--token")))?;
    ans((|| {
        let (owner, spender) = (flag(f, "--owner")?, flag(f, "--spender")?);
        let value = word_of("--amount", &given?)?;
        let nonce = word_of("--nonce", &flag(f, "--nonce")?)?;
        let domain = order::b32_of("--domain", &flag(f, "--domain")?)?;
        let deadline = numf(f, "--deadline")?;
        let d = a1::call::permit_digest(&domain, &owner, &spender, &value, &nonce, deadline)?;
        let mut rows = vec![("digest", Out::S(order::hex_of(&d)))];
        if let Some(sig) = f.one("--sig") {
            let c = a1::call::erc20_permit(&flag(f, "--token")?, &a1::call::Permit { owner, spender, value, deadline, sig: order::bytes_of("--sig", sig)? })?;
            rows.push(("call", call_out(&c)));
        }
        Ok(ok(rows))
    })())
}

pub(super) fn digest(f: &Flags) -> Result<Out, Misuse> {
    let what = need(f, "--do")?;
    if what == "offer" {
        return offer_digest(f);
    }
    let instance = need(f, "--instance")?;
    let chain: u64 = match f.one("--chain") {
        None => a1::chain::standard_table().0,
        Some(x) => x.parse().map_err(|_| Misuse::not_int("--chain", x))?,
    };
    ans((|| {
        let act = match what.as_str() {
            "claim-delivery" => {
                k2::derive::Action::ClaimDelivery { manifest: flag(f, "--manifest")? }
            }
            "confirm" => k2::derive::Action::Confirm,
            "dispute" => k2::derive::Action::Dispute,
            "cancel" => k2::derive::Action::Cancel,
            "refund" => k2::derive::Action::Refund,
            "settle" => k2::derive::Action::Settle {
                buyer_bps: numf(f, "--buyer-bps")?,
                expiry: k2::u256::U256::from_u64(numf(f, "--settle-expiry")?),
            },
            other => {
                return Err(Fault::Form(format!("六条动作消息与 offer 之外的词:{other:?}")));
            }
        };
        let d = k2::derive::digest(chain, &instance, &act.struct_hash());
        Ok(ok(vec![
            ("digest", Out::S(order::hex_of(&d))),
            ("message", Out::S(act.name().to_string())),
        ]))
    })())
}

pub(super) fn offer_digest(f: &Flags) -> Result<Out, Misuse> {
    let bytes = read_bounded(&need(f, "--terms")?)?;
    let c = num(f, "--candidate")?;
    let expiry = match f.one("--expiry") {
        None => None,
        Some(_) => Some(num(f, "--expiry")?),
    };
    ans((|| {
        let law = |e: k2::token::Fault| Fault::Law { token: e.token.name().to_string(), detail: format!("{e:?}") };
        let mut t = k2::doc::terms(&bytes).map_err(law)?;
        if let Some(x) = expiry {
            t.expiry = x;
        }
        let d = k2::derive::offer_digest_at(&t, c).map_err(law)?;
        Ok(ok(vec![
            ("digest", Out::S(order::hex_of(&d))),
            ("message", Out::S("Offer".to_string())),
        ]))
    })())
}

pub(super) fn decode_log(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    ans((|| {
        let mut topics = Vec::new();
        for t in f.many("--topic") {
            topics.push(order::b32_of("topic", t)?);
        }
        let log = a1::event::decode(&topics, &order::bytes_of("data", &data)?)?;
        Ok(ok(vec![("args", words(&log.args)), ("event", Out::S(log.name.to_string()))]))
    })())
}

pub(super) fn decode_init(f: &Flags) -> Result<Out, Misuse> {
    let data = need(f, "--data")?;
    ans((|| {
        let raw = order::bytes_of("calldata", &data)?;
        let v = a1::order::decode_init(&raw)?;
        let o = &v.order.offer;
        Ok(ok(vec![
            ("amount", Out::S(o.amount.to_dec())),
            ("buyerKey", Out::S(v.order.buyer_key.clone())),
            ("buyerPayout", Out::S(v.order.buyer_payout.clone())),
            ("guild", Out::S(o.guild.clone())),
            ("judge", Out::S(o.judge.clone())),
            ("members", Out::N(v.members.len() as u64)),
            ("offerHash", Out::S(order::hex_of(&o.struct_hash()))),
            ("quoteBytes", Out::N(v.quote.len() as u64)),
            ("sellerKey", Out::S(o.seller_key.clone())),
            ("termsHash", Out::S(order::hex_of(&o.terms_hash))),
            ("token", Out::S(o.token.clone())),
        ]))
    })())
}
