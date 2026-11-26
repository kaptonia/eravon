use super::*;

pub(super) fn token_of(d: &Draft, v: &mut Verdict) -> Option<String> {
    let t = d.token.trim();
    if t.is_empty() {
        return None;
    }
    let got = crate::draft::token_address(t);
    if got.is_none() {
        v.miss("token", "E_GOODS_FORM", format!("结算币「{t}」折不成合约地址:请从本机登载的币种里重挑"));
    }
    got
}

pub const HOURS_MAX: u64 = 23;
pub const MINUTES_MAX: u64 = 59;
pub const DAYS_MAX: u64 = u32::MAX as u64 / 86_400;

pub fn secs_of(cell: &str, days: &str, hours: &str, minutes: &str, v: &mut Verdict) -> Option<u64> {
    let mut bad = false;
    let mut part = |c: String, s: &str, max: u64| -> u64 {
        let s = s.trim();
        if s.is_empty() {
            return 0;
        }
        match s.parse::<u64>() {
            Ok(n) if s.bytes().all(|b| b.is_ascii_digit()) && n <= max => n,
            Ok(_) if s.bytes().all(|b| b.is_ascii_digit()) => {
                v.miss(c, "E_GOODS_FORM", format!("最多 {max}"));
                bad = true;
                0
            }
            _ => {
                v.miss(c, "E_GOODS_FORM", "只收数字");
                bad = true;
                0
            }
        }
    };
    let d = part(cell.to_string(), days, DAYS_MAX);
    let h = part(format!("{cell}h"), hours, HOURS_MAX);
    let m = part(format!("{cell}m"), minutes, MINUTES_MAX);
    if bad {
        return None;
    }
    let secs = d * 86_400 + h * 3_600 + m * 60;
    if secs == 0 {
        v.miss(cell, "E_GOODS_EMPTY", "未填写");
        return None;
    }
    if secs > u64::from(u32::MAX) {
        v.miss(cell, "E_GOODS_FORM", "期限过长");
        return None;
    }
    Some(secs)
}

pub fn span(secs: u64) -> (u64, u64, u64) {
    (secs / 86_400, secs % 86_400 / 3_600, secs % 3_600 / 60)
}

pub(super) fn bps_of(p: &str) -> Option<u64> {
    let p = p.trim();
    let (i, f) = p.split_once('.').unwrap_or((p, ""));
    if i.is_empty() || !i.bytes().all(|c| c.is_ascii_digit()) || f.len() > 2 || !f.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let whole: u64 = i.parse().ok()?;
    let frac: u64 = format!("{f:0<2}").parse().ok()?;
    let bps = whole.checked_mul(100)?.checked_add(frac)?;
    if bps > 10_000 { None } else { Some(bps) }
}

pub(super) fn amount_of(price: &str, decimals: u64, v: &mut Verdict) -> Option<String> {
    let Some(d) = u8::try_from(decimals).ok().filter(|d| *d <= eravon_literals::units::DECIMALS_MAX) else {
        v.miss("price", "E_GOODS_DECIMALS", "无法读取币种精度");
        return None;
    };
    match eravon_literals::units::units_of(price, d) {
        Ok(raw) => Some(raw),
        Err(name) => {
            let say = match name {
                eravon_literals::units::AMOUNT_TOO_PRECISE => format!("该币种最多 {d} 位小数"),
                eravon_literals::units::AMOUNT_TOO_LARGE => "价格过大".to_string(),
                _ => "请输入有效价格".to_string(),
            };
            v.miss("price", name, say);
            None
        }
    }
}

pub fn local(d: &Draft) -> Verdict {
    let mut v = Verdict::default();
    verdict::presence(d, &mut v);
    if let Err((cell, say)) = crate::series::cells(d) {
        v.miss(cell, crate::series::E_SERIES, say);
    }
    let _ = token_of(d, &mut v);
    let _ = secs_of("t1", &d.t1, &d.t1h, &d.t1m, &mut v);
    let _ = secs_of("t2", &d.t2, &d.t2h, &d.t2m, &mut v);
    for (i, t) in d.ladder.iter().enumerate() {
        if bps_of(&t.pct).is_none() {
            v.miss(format!("ladder.{i}"), "E_GOODS_FORM", "请输入 0–100 之间的数,最多两位小数");
        }
    }
    v
}

pub fn judge(c: &Ctx<'_>, d: &Draft, nonce: u64) -> (Verdict, Resolved, Option<terms::Generated>, Option<u64>) {
    judge_with(c, d, None, nonce, &crate::company::of(c.data, c.me, &c.route, c.registries, c.now))
}

pub fn judge_with(
    c: &Ctx<'_>,
    d: &Draft,
    id: Option<&str>,
    nonce: u64,
    co: &Result<Option<crate::company::Company>, crate::company::Why>,
) -> (Verdict, Resolved, Option<terms::Generated>, Option<u64>) {
    let mut v = Verdict::default();
    verdict::presence(d, &mut v);
    let series = match crate::series::cells(d) {
        Ok(s) => s,
        Err((cell, say)) => {
            v.miss(cell, crate::series::E_SERIES, say);
            None
        }
    };
    let token = token_of(d, &mut v);
    let t1 = secs_of("t1", &d.t1, &d.t1h, &d.t1m, &mut v);
    let t2 = secs_of("t2", &d.t2, &d.t2h, &d.t2m, &mut v);
    let mut ladder: Vec<(String, u64)> = Vec::new();
    for (i, t) in d.ladder.iter().enumerate() {
        match bps_of(&t.pct) {
            Some(b) => ladder.push((t.name.clone(), b)),
            None => v.miss(format!("ladder.{i}"), "E_GOODS_FORM", "请输入 0–100 之间的数,最多两位小数"),
        }
    }
    let ways = match eravon_payways::ways::read(c.data, c.me) {
        Ok(w) => w,
        Err(e) => {
            v.miss("fund", e.name(), e.detail());
            eravon_payways::ways::view(&Default::default(), "")
        }
    };
    let (seller_key, payout, fund) = match co {
        Ok(Some(x)) => (x.address.clone(), x.payout.clone(), x.fund.clone()),
        Ok(None) => {
            let p = eravon_vault::store::read_identity(c.data, c.me).map(|t| t.payout).unwrap_or_default();
            if p.is_empty() {
                v.miss("payout", "E_GOODS_PAYOUT", "未设置收款地址");
            } else if p.eq_ignore_ascii_case(c.me) {
                v.miss("payout", E_GOODS_PAYOUT_KEY, "收款地址不能是卖家钥");
            }
            if ways.fund.is_empty() {
                v.miss("fund", E_GOODS_FUND, "未设置金库:在钱包页打开自带金库,或填金库地址");
            }
            (c.me.to_ascii_lowercase(), p, ways.fund.clone())
        }
        Err((n, detail)) => {
            v.miss("company", n.as_str(), detail.clone());
            (c.me.to_ascii_lowercase(), String::new(), String::new())
        }
    };

    let mut resolved = Resolved::default();
    let mut amount = None;
    let mut decimals: Option<u64> = None;
    let mut wire = None;
    if let Some(tok) = &token {
        match eravon_cands::wire::Wire::on(&c.route, c.registries, c.now)
            .and_then(|w| eravon_cands::registry::logs_of(&w, &d.cands).map(|l| (w, l)))
        {
            Err(a) => {
                v.miss("cands", a.name(), a.detail());
                if !d.price.trim().is_empty() {
                    v.miss("price", a.name(), "结算币的位数要读链才折得成最小单位");
                }
            }
            Ok((w, logs)) => {
                let table = match eravon_cands::table::list(c.data, c.me) {
                    Ok(t) => t,
                    Err(e) => {
                        v.miss("cands", e.name(), e.detail());
                        Vec::new()
                    }
                };
                resolved = resolve::resolve(&w, &logs, c.me, &table, &d.cands, tok);
                if !d.cands.is_empty() && resolved.judges.is_empty() && resolved.guilds.is_empty() {
                    v.miss("cands", "E_GOODS_NO_CANDIDATE", "所选候选均不可用");
                }
                if resolved.judges.len() + resolved.guilds.len() > eravon_literals::constants::CANDIDATE_CAP {
                    v.miss(
                        "cands",
                        "E_GOODS_CAP",
                        format!("候选最多 {} 位", eravon_literals::constants::CANDIDATE_CAP),
                    );
                }
                if !d.price.trim().is_empty() {
                    let rows = eravon_vault::tokens::list(c.data);
                    match eravon_standard::chain::amount_decimals(&w.t, &w.table, &rows, tok).map(u64::from) {
                        Some(dec) => {
                            decimals = Some(dec);
                            amount = amount_of(&d.price, dec, &mut v)
                        }
                        None => v.miss("price", "E_GOODS_DECIMALS", "无法读取币种精度"),
                    }
                }
                wire = Some(w);
            }
        }
    }
    if !v.formal() {
        return (v, resolved, None, decimals);
    }
    let (Some(tok), Some(t1), Some(t2), Some(amount)) = (token, t1, t2, amount) else {
        return (v, resolved, None, decimals);
    };
    let mut expiry = c.now + u64::from(eravon_literals::constants::LINK_VALIDITY_DAYS) * 86_400;
    let (mut open_window, mut buyer_may_open) = (ways.open_window, ways.buyer_may_open);
    if let Some(sr) = &series {
        let mates = crate::series::mates(c.data, c.me, &sr.id, id);
        let picked: Vec<String> = resolved.judges.iter().map(|j| j.judge.to_ascii_lowercase()).chain(resolved.guilds.iter().map(|g| g.pointer.to_ascii_lowercase())).collect();
        let cands: Vec<String> = if picked.len() == d.cands.len() { d.cands.iter().map(|x| x.to_ascii_lowercase()).collect() } else { picked };
        match crate::series::judge_stage(sr, &cands, &mates) {
            Err((cell, name, say)) => v.miss(cell, name, say),
            Ok(None) => {}
            Ok(Some(before)) => {
                buyer_may_open = false;
                let quote = resolved.judges.first().map(|j| j.quote.clone()).or_else(|| resolved.guilds.first().map(|g| g.quote.clone())).unwrap_or_default();
                match quote.strip_prefix("0x").and_then(|h| eravon_galeed::hexform::decode_digits(h.as_bytes())).and_then(|b| eravon_cands::lapse::of_bytes(&b).ok()) {
                    Some(clocks) => {
                        let (ow, ex) = crate::series::floors(before, &clocks);
                        open_window = open_window.max(ow);
                        expiry = expiry.max(ex);
                    }
                    None => v.miss("cands", crate::series::E_SERIES_PREV, "读不出候选的报价,推不出这一段的开案窗"),
                }
            }
        }
        if !v.formal() {
            return (v, resolved, None, decimals);
        }
    }
    let parts = terms::Parts {
        name: &d.name,
        description: &d.description,
        delivery_note: &d.delivery_note,
        goods_private: d.goods_private,
        delivery_private: d.delivery_private,
        seller_key: &seller_key,
        seller_payout: &payout.to_ascii_lowercase(),
        seller_fund: &fund.to_ascii_lowercase(),
        token: &tok,
        amount: &amount,
        t1_secs: t1,
        t2_secs: t2,
        open_window,
        buyer_may_open,
        ladder: &ladder,
        judges: &resolved.judges,
        guilds: &resolved.guilds,
        nonce,
        expiry,
        series: series.as_ref().map(|x| (x.id.as_str(), x.stage, x.stages)),
    };
    let made = terms::generate(&parts);
    law(&made.bytes, d, &mut v);
    if !v.formal() {
        return (v, resolved, None, decimals);
    }
    if let Some(w) = &wire {
        resolved.fees = resolve::fees_of(w, &made.bytes, &resolved, decimals);
    }
    (v, resolved, Some(made), decimals)
}

pub fn terms_fees(c: &Ctx<'_>, w: &eravon_cands::wire::Wire, bytes: &[u8]) -> Result<Vec<resolve::Fee>, (String, String)> {
    let t = eravon_terms::doc::terms(bytes).map_err(|f| (f.token.name().to_string(), "条款读不成".to_string()))?;
    let picks: Vec<String> = t.judges.iter().map(|j| j.judge.to_ascii_lowercase()).chain(t.guilds.iter().map(|g| g.address.to_ascii_lowercase())).collect();
    let logs = eravon_cands::registry::logs_of(w, &picks).map_err(|a| (a.name().to_string(), a.detail()))?;
    let row = |kind: &str, address: &String| eravon_store::record::Candidate { kind: kind.into(), address: address.clone(), label: String::new(), added_at: 0, seen: String::new() };
    let table: Vec<_> = picks.iter().take(t.judges.len()).map(|a| row("judge", a)).chain(picks.iter().skip(t.judges.len()).map(|a| row("guild", a))).collect();
    let r = resolve::resolve(w, &logs, c.me, &table, &picks, &t.token);
    let decimals = eravon_standard::chain::amount_decimals(&w.t, &w.table, &eravon_vault::tokens::list(c.data), &t.token).map(u64::from);
    let quotes: Vec<(String, String)> = r.judges.iter().map(|j| (j.judge.to_ascii_lowercase(), j.quote.clone())).chain(r.guilds.iter().map(|g| (g.pointer.to_ascii_lowercase(), g.quote.clone()))).collect();
    let mut each = Vec::with_capacity(picks.len());
    for (i, candidate) in picks.into_iter().enumerate() {
        let offer = eravon_terms::derive::offer(&t, i as u64).map_err(|f| (f.token.name().to_string(), "无法生成订单要约".to_string()))?;
        let quote = quotes.iter().find(|(a, _)| *a == candidate).map(|(_, q)| q.clone()).unwrap_or_default();
        let excluded = r.excluded.iter().find(|x| x.address.eq_ignore_ascii_case(&candidate)).map(|x| (x.name.clone(), x.detail.clone()));
        let ready = excluded
            .map_or(Ok(()), Err)
            .and_then(|()| eravon_standard::order::bytes_of("报价", &quote).map_err(|f| (f.name(), f.detail())))
            .and_then(|b| if eravon_terms::derive::keccak(&b) == offer.quote_hash { Ok(b) } else { Err((E_GOODS_QUOTE_MOVED.to_string(), "这位候选现读出的报价不是条款里的那一份(quoteHash 不合):报价已变".to_string())) });
        each.push((candidate, quote, offer, ready));
    }
    let (tt, tb) = (w.t.clone(), w.table.clone());
    let priced = eravon_chain::read::side_by_side(each.iter().map(|(_, _, offer, ready)| (offer.clone(), ready.clone())).collect::<Vec<_>>(), move |(offer, ready)| ready.and_then(|b| eravon_standard::precheck::quoted(&tt, &tb, &offer, &b).map_err(|f| (f.name(), f.detail()))));
    Ok(each.into_iter().zip(priced).map(|((candidate, quote, offer, _), priced)| resolve::Fee { candidate, quote, token: offer.token.clone(), decimals, amount: offer.amount, priced }).collect())
}

pub const E_GOODS_QUOTE_MOVED: &str = "E_GOODS_QUOTE_MOVED";

pub const E_GOODS_PAYOUT_KEY: &str = "E_GOODS_PAYOUT_KEY";
pub const E_GOODS_FUND: &str = "E_GOODS_FUND";
pub const E_GOODS_FUND_UNSET: &str = "E_GOODS_FUND_UNSET";

pub(super) fn law(bytes: &[u8], d: &Draft, v: &mut Verdict) {
    if let Err(f) = eravon_terms::doc::terms(bytes) {
        let name = f.token.name();
        v.miss(verdict::cell_of_law(name, d), name, "条款格式无效");
    }
}


#[cfg(test)]
mod window_tests {
    use super::*;

    fn secs(d: &str, h: &str, m: &str) -> (Option<u64>, Vec<(String, String)>) {
        let mut v = Verdict::default();
        let s = secs_of("t1", d, h, m, &mut v);
        (s, v.missing.into_iter().map(|x| (x.cell, x.name)).collect())
    }

    #[test]
    fn a_window_reads_days_hours_and_minutes() {
        assert_eq!(secs("7", "23", "59"), (Some(7 * 86_400 + 23 * 3_600 + 59 * 60), vec![]));
        assert_eq!(secs("", "", "30"), (Some(1_800), vec![]), "no days: the hours and minutes alone");
        assert_eq!(secs("", "", ""), (None, vec![("t1".into(), "E_GOODS_EMPTY".into())]), "zero is missing");
        assert_eq!(secs("0", "0", "0"), (None, vec![("t1".into(), "E_GOODS_EMPTY".into())]));
        assert_eq!(secs("1", "24", ""), (None, vec![("t1h".into(), "E_GOODS_FORM".into())]), "hours above 23");
        assert_eq!(secs("1", "", "60"), (None, vec![("t1m".into(), "E_GOODS_FORM".into())]), "minutes above 59");
        assert_eq!(secs("x", "", ""), (None, vec![("t1".into(), "E_GOODS_FORM".into())]), "not digits");
        assert_eq!(secs("-1", "", ""), (None, vec![("t1".into(), "E_GOODS_FORM".into())]));
        assert_eq!(secs(&DAYS_MAX.to_string(), "", ""), (Some(DAYS_MAX * 86_400), vec![]), "the most days");
        assert_eq!(secs(&(DAYS_MAX + 1).to_string(), "", ""), (None, vec![("t1".into(), "E_GOODS_FORM".into())]));
        assert_eq!(secs(&DAYS_MAX.to_string(), "23", "59").1, vec![("t1".to_string(), "E_GOODS_FORM".to_string())], "past uint32 seconds");
        assert_eq!(span(7 * 86_400 + 23 * 3_600 + 59 * 60), (7, 23, 59));
        assert_eq!(crate::terms::say_window(3 * 86_400), "3 天");
        assert_eq!(crate::terms::say_window(7 * 86_400 + 23 * 3_600 + 59 * 60), "7 天 23 小时 59 分钟");
    }
}
