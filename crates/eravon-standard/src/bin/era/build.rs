use eravon_literals::misuse::Misuse;
use super::*;

pub(super) fn read_order(path: &str) -> a1::fault::R<Order> {
    let bytes = std::fs::read(path).map_err(|e| Fault::Form(format!("读不到订单档 {path}:{e}")))?;
    order::read_json(&bytes)
}

pub(super) fn chain_and_logic(f: &Flags) -> Result<(u64, String), Misuse> {
    if let Some(c) = f.one("--chain").and_then(|s| s.parse::<u64>().ok()).filter(|c| eravon_literals::constants::network_of(*c).is_some()) {
        eravon_literals::constants::select_chain(c);
    }
    let (chain, logic, _) = a1::chain::standard_table();
    let c = match f.one("--chain") {
        None => chain,
        Some(s) => s.parse().map_err(|_| Misuse::not_int("--chain", s))?,
    };
    let l = f.one("--logic").unwrap_or(logic).to_string();
    Ok((c, l))
}

pub(super) fn order_arg(f: &Flags) -> Result<a1::fault::R<(Order, u64, String)>, Misuse> {
    let (chain, logic) = chain_and_logic(f)?;
    match (f.one("--order"), f.one("--terms")) {
        (Some(path), None) => {
            let path = path.to_string();
            Ok(read_order(&path).map(|o| (o, chain, logic)))
        }
        (None, Some(tpath)) => {
            let bytes = read_bounded(tpath)?;
            let c = num(f, "--candidate")?;
            let bk = need(f, "--buyer-key")?;
            let bp = need(f, "--buyer-payout")?;
            let ob = num(f, "--open-by")?;
            Ok((|| {
                let law = |e: k2::token::Fault| Fault::Law { token: e.token.name().to_string(), detail: format!("{e:?}") };
                let t = k2::doc::terms(&bytes).map_err(law)?;
                let offer = k2::derive::offer(&t, c).map_err(law)?;
                let chain = if f.one("--chain").is_some() { chain } else { t.chain_id };
                let logic = if f.one("--logic").is_some() { logic } else { t.logic.clone() };
                let o = Order {
                    offer,
                    buyer_key: order::addr_norm("buyerKey", &bk)?,
                    buyer_payout: order::addr_norm("buyerPayout", &bp)?,
                    open_by: ob,
                    prev: order::NO_PREV.to_string(),
                };
                Ok((o, chain, logic))
            })())
        }
        (Some(_), Some(_)) => Err(Misuse::both(&["--order", "--terms"], eravon_literals::misuse::ONE_FORM)),
        (None, None) => Err(Misuse::both(&["--order", "--terms"], eravon_literals::misuse::said("缺订单:给 --order,或给 --terms 连同 --candidate --buyer-key --buyer-payout --open-by", "no order: give --order, or --terms with --candidate --buyer-key --buyer-payout --open-by"))),
    }
}

pub(super) fn address(f: &Flags) -> Result<Out, Misuse> {
    let path = need(f, "--order")?;
    let (chain, logic) = chain_and_logic(f)?;
    ans((|| {
        let o = read_order(&path)?;
        let d = o.derive(chain, &logic)?;
        Ok(ok(vec![
            ("caseId", Out::S(order::hex_of(&d.salt))),
            ("chainId", Out::N(chain)),
            ("instance", Out::S(d.instance.clone())),
            ("logic", Out::S(logic.clone())),
            ("offerHash", Out::S(order::hex_of(&o.offer.struct_hash()))),
            ("orderDigest", Out::S(order::hex_of(&d.order_digest))),
            (
                "sellerDigest",
                Out::S(order::hex_of(&o.offer.offer_digest(chain, &logic))),
            ),
        ]))
    })())
}

pub(super) fn deploy_logic(f: &Flags) -> Result<Out, Misuse> {
    let path = need(f, "--initcode")?;
    let raw = read_bounded(&path)?;
    ans((|| {
        let text = String::from_utf8_lossy(&raw).trim().to_string();
        let code = order::bytes_of("初始码", &text)?;
        Ok(ok(vec![("call", call_out(&a1::call::deploy_logic(&code)?))]))
    })())
}

pub(super) fn deploy_verifier(f: &Flags) -> Result<Out, Misuse> {
    let path = need(f, "--initcode")?;
    let raw = read_bounded(&path)?;
    ans((|| {
        let text = String::from_utf8_lossy(&raw).trim().to_string();
        let code = order::bytes_of("初始码", &text)?;
        let call = a1::call::deploy_verifier(&code)?;
        Ok(ok(vec![
            ("call", call_out(&call)),
            ("verifier", Out::S(k7::VERIFIER.to_string())),
        ]))
    })())
}

pub(super) fn deploy_board(f: &Flags) -> Result<Out, Misuse> {
    let path = need(f, "--initcode")?;
    let raw = read_bounded(&path)?;
    ans((|| {
        let text = String::from_utf8_lossy(&raw).trim().to_string();
        let call = a1::call::deploy_board(&order::bytes_of("初始码", &text)?)?;
        Ok(ok(vec![("board", Out::S(k7::BOARD.to_string())), ("call", call_out(&call))]))
    })())
}

pub(super) fn deploy_instance(f: &Flags) -> Result<Out, Misuse> {
    let path = need(f, "--order")?;
    let (chain, logic) = chain_and_logic(f)?;
    ans((|| {
        let o = read_order(&path)?;
        let d = o.derive(chain, &logic)?;
        Ok(ok(vec![
            ("call", call_out(&a1::call::deploy_instance(&d.salt, &logic)?)),
            ("instance", Out::S(d.instance.clone())),
        ]))
    })())
}

pub(super) struct InitParts {
    call: a1::call::Call,
    instance: String,
}

pub(super) fn build_init(f: &Flags) -> a1::fault::R<InitParts> {
    let quote = match f.one("--quote") {
        None => Vec::new(),
        Some(h) => order::bytes_of("报价", h)?,
    };
    let (o, chain, logic) = order_arg(f)
        .map_err(|_| Fault::Form("订单要 --order 或条款那一形(--terms 与四枚)择一".into()))??;
    let d = o.derive(chain, &logic)?;
    let (sb, ss) = (
        order::bytes_of("买家签名", f.one("--sig-buyer").unwrap_or("0x"))?,
        order::bytes_of("卖家签名", f.one("--sig-seller").unwrap_or("0x"))?,
    );
    let mut members = Vec::new();
    for m in f.many("--member") {
        members.push(m.to_string());
    }
    let mut words = Vec::new();
    for w in f.many("--amanah") {
        words.push(order::b32_of("判法词", w)?);
    }
    let call = a1::call::init(&o, &d.instance, &quote, &sb, &ss, &members, &words)?;
    Ok(InitParts { call, instance: d.instance })
}

pub(super) fn build_expire(f: &Flags) -> a1::fault::R<InitParts> {
    let (o, chain, logic) = order_arg(f)
        .map_err(|_| Fault::Form("订单要 --order 或条款那一形(--terms 与四枚)择一".into()))??;
    let d = o.derive(chain, &logic)?;
    let call = a1::call::expire_init(&o, &d.instance)?;
    Ok(InitParts { call, instance: d.instance })
}

pub(super) fn init(f: &Flags) -> Result<Out, Misuse> {
    ans((|| {
        let p = build_init(f)?;
        Ok(ok(vec![("call", call_out(&p.call)), ("instance", Out::S(p.instance.clone()))]))
    })())
}

pub(super) fn expire(f: &Flags) -> Result<Out, Misuse> {
    let now = num(f, "--now")?;
    ans((|| {
        let path = f.one("--order").ok_or_else(|| Fault::Form("缺 --order".into()))?;
        let o = read_order(path)?;
        if now <= o.offer.expiry && now <= o.open_by {
            return Err(Fault::Precheck {
                gate: a1::precheck::GATE_WINDOW,
                why: format!("要约到 {}、openBy 到 {} 为止,现在是 {now};过期分支还不该发", o.offer.expiry, o.open_by),
            });
        }
        let p = build_expire(f)?;
        Ok(ok(vec![
            ("call", call_out(&p.call)),
            ("expiry", Out::N(o.offer.expiry)),
            ("instance", Out::S(p.instance.clone())),
        ]))
    })())
}


pub(super) fn merged(f: &Flags) -> Result<Out, Misuse> {
    let got = order_arg(f)?;
    let vpath = f.one("--verifier-initcode").map(str::to_string);
    let vraw = match &vpath {
        Some(p) => Some(read_bounded(p)?),
        None => None,
    };
    let bpath = f.one("--board-initcode").map(str::to_string);
    let braw = match &bpath {
        Some(p) => Some(read_bounded(p)?),
        None => None,
    };
    ans((|| {
        let (o, chain, logic) = got?;
        let d = o.derive(chain, &logic)?;
        let dep = a1::call::deploy_instance(&d.salt, &logic)?;
        let p = build_init(f)?;
        let ver = match &vraw {
            Some(raw) => {
                let text = String::from_utf8_lossy(raw).trim().to_string();
                Some(a1::call::deploy_verifier(&order::bytes_of("初始码", &text)?)?)
            }
            None => None,
        };
        let board = match &braw {
            Some(raw) => {
                let text = String::from_utf8_lossy(raw).trim().to_string();
                Some(a1::call::deploy_board(&order::bytes_of("初始码", &text)?)?)
            }
            None => None,
        };
        let mut steps: Vec<&a1::call::Call> = Vec::new();
        if let Some(v) = &ver {
            steps.push(v);
        }
        if let Some(b) = &board {
            steps.push(b);
        }
        steps.push(&dep);
        let deploy = if steps.len() == 1 { dep.clone() } else { a1::call::merged(&steps)? };
        Ok(ok(vec![
            ("call", call_out(&deploy)),
            ("init", call_out(&p.call)),
            ("instance", Out::S(d.instance.clone())),
            ("steps", Out::A(steps.iter().map(|c| call_out(c)).collect())),
        ]))
    })())
}

pub(super) fn action(f: &Flags) -> Result<Out, Misuse> {
    let instance = need(f, "--instance")?;
    let what = need(f, "--do")?;
    ans((|| {
        let a = match what.as_str() {
            "claim-delivery" => {
                Action::ClaimDelivery(order::b32_of("清单摘要", &flag(f, "--manifest")?)?)
            }
            "confirm" => Action::Confirm,
            "dispute" => Action::Dispute,
            "refund" => Action::Refund,
            "settle" => Action::Settle {
                buyer_bps: u16::try_from(numf(f, "--buyer-bps")?)
                    .map_err(|_| Fault::Form("买家份额超出 uint16".into()))?,
                expiry: {
                    let mut w = [0u8; 32];
                    w[24..].copy_from_slice(&numf(f, "--settle-expiry")?.to_be_bytes());
                    w
                },
                sig_buyer: order::bytes_of("买家签名", &flag(f, "--sig-buyer")?)?,
                sig_seller: order::bytes_of("卖家签名", &flag(f, "--sig-seller")?)?,
            },
            "commit" => Action::Commit {
                case_id: order::b32_of("案号", &flag(f, "--case-id")?)?,
                manifest: order::b32_of("清单摘要", &flag(f, "--manifest")?)?,
            },
            "advance" => Action::Advance {
                case_id: order::b32_of("案号", &flag(f, "--case-id")?)?,
                seat: numf(f, "--seat")?,
            },
            "redraw" => Action::Redraw {
                members: f.many("--member").iter().map(|x| (*x).to_string()).collect(),
                amanah_hashes: {
                    let mut v = Vec::new();
                    for w in f.many("--amanah") {
                        v.push(order::b32_of("判法词", w)?);
                    }
                    v
                },
            },
            "sweep" => Action::Sweep {
                who: flag(f, "--who")?,
                token: flag(f, "--token")?,
            },
            other => {
                return Err(Fault::Form(format!(
                    "十项当事人动作之外的词:{other:?}(判官那三枚在这里没有门)"
                )));
            }
        };
        if let Some(end) = f.one("--window-end") {
            let end: u64 = end.parse().map_err(|_| Fault::Form("--window-end 不是数".into()))?;
            let now: u64 = f
                .one("--now")
                .ok_or_else(|| Fault::Form("给了 --window-end 就要给 --now".into()))?
                .parse()
                .map_err(|_| Fault::Form("--now 不是数".into()))?;
            a1::precheck::in_window(end, now)?;
        }
        Ok(ok(vec![("call", call_out(&a.call(&instance)?)), ("verb", Out::S(what.clone()))]))
    })())
}

pub(super) fn amount_arg(f: &Flags, raw: &'static str, text: &'static str, decimals: impl FnOnce() -> Option<u8>) -> Result<a1::fault::R<String>, Misuse> {
    if let Some(v) = f.one(raw) {
        return Ok(Ok(v.to_string()));
    }
    let t = f.one(text).ok_or_else(|| Misuse::both(&[raw, text], eravon_literals::misuse::said("缺这一个金额:给其中一种写法", "this amount is missing: give one of its two forms")))?;
    Ok(match decimals() {
        None => Err(Fault::Law { token: "token-decimals-unread".into(), detail: format!("{text} 要按币的小数位折成最小单位,小数位读不成(给 --endpoint 读链,或改用 {raw})") }),
        Some(d) => eravon_literals::units::units_of(t, d).map_err(|n| Fault::Law { token: n.into(), detail: format!("{text} {t:?} 折不成最小单位(至多 {d} 位小数)") }),
    })
}

pub(super) fn chain_decimals(f: &Flags, token: Option<&str>) -> Option<u8> {
    let token = token?;
    let t = Table::new(f.many("--endpoint").iter().map(|x| (*x).to_string()).collect()).ok()?;
    let w = super::read::wire(f).ok()?;
    a1::chain::erc20_decimals(&w, &t, token).map(|d| d as u8)
}

pub(super) fn units_rows(f: &Flags, text: &'static str, dec: &'static str, word: &[u8; 32], decimals: Option<u8>) -> Vec<(&'static str, Out)> {
    if f.one("--units-text") != Some("true") {
        return Vec::new();
    }
    match decimals {
        Some(d) => vec![(text, Out::S(eravon_literals::units::units_text(&k2::u256::U256::from_be(word).to_dec(), d, false))), (dec, Out::N(u64::from(d)))],
        None => vec![(text, Out::Null), (dec, Out::Null)],
    }
}

pub(super) fn flag(f: &Flags, k: &'static str) -> a1::fault::R<String> {
    f.one(k).map(str::to_string).ok_or_else(|| Fault::Form(format!("缺 {k}")))
}

pub(super) fn numf(f: &Flags, k: &'static str) -> a1::fault::R<u64> {
    flag(f, k)?.parse().map_err(|_| Fault::Form(format!("{k} 不是非负整数")))
}


pub(super) fn bysig(f: &Flags) -> Result<Out, Misuse> {
    let instance = need(f, "--instance")?;
    let what = need(f, "--do")?;
    ans((|| {
        let sig = order::bytes_of("签名", &flag(f, "--sig")?)?;
        let b = match what.as_str() {
            "claim-delivery" => BySig::ClaimDelivery {
                manifest: order::b32_of("清单摘要", &flag(f, "--manifest")?)?,
                sig,
            },
            "confirm" => BySig::Confirm { sig },
            "dispute" => BySig::Dispute { sig },
            "refund" => BySig::Refund { sig },
            other => {
                return Err(Fault::Form(format!("四个 …BySig 之外的词:{other:?}")));
            }
        };
        Ok(ok(vec![("call", call_out(&b.call(&instance)?)), ("verb", Out::S(what.clone()))]))
    })())
}

pub(super) fn cancel(f: &Flags) -> Result<Out, Misuse> {
    let got = order_arg(f)?;
    ans((|| {
        let (o, chain, logic) = got?;
        let d = o.derive(chain, &logic)?;
        let sig = order::bytes_of("买家签名", f.one("--sig-buyer").unwrap_or("0x"))?;
        let call = a1::call::cancel(&o, &d.instance, &sig)?;
        Ok(ok(vec![("call", call_out(&call)), ("instance", Out::S(d.instance.clone()))]))
    })())
}

pub(super) fn revoke(f: &Flags) -> Result<Out, Misuse> {
    let (_, logic) = chain_and_logic(f)?;
    let seller = need(f, "--seller")?;
    let th = need(f, "--terms-hash")?;
    ans((|| {
        let call = a1::call::revoke(&logic, &seller, &order::b32_of("termsHash", &th)?)?;
        Ok(ok(vec![("call", call_out(&call))]))
    })())
}

pub(super) fn accept_funding(f: &Flags) -> Result<Out, Misuse> {
    let (_, logic) = chain_and_logic(f)?;
    let seller = need(f, "--seller")?;
    ans((|| {
        let call = a1::call::accept_funding(&logic, &seller)?;
        Ok(ok(vec![("call", call_out(&call))]))
    })())
}

pub(super) fn post(f: &Flags) -> Result<Out, Misuse> {
    let key = need(f, "--key")?;
    let path = need(f, "--document")?;
    let doc = read_bounded(&path)?;
    ans((|| {
        let board = f.one("--board").map(str::to_string).unwrap_or_else(|| k7::BOARD.to_string());
        let call = a1::call::post(&board, &order::b32_of("key", &key)?, &doc)?;
        let hash = eravon_terms::doc::terms_hash(&doc);
        Ok(ok(vec![("call", call_out(&call)), ("hash", Out::S(order::hex_of(&hash)))]))
    })())
}
