use eravon_galeed as k1;
use eravon_link as k3;
use eravon_standard as a1;
use eravon_terms as k2;
use k2::u256::U256;

use crate::order::{self, Order, State};
use crate::{Why, Wire, why};

pub const E_OPEN_OPENED: &str = "E_OPEN_OPENED";

fn a1w(e: a1::fault::Fault) -> Why {
    why(&e.name(), e.detail())
}

#[derive(Clone)]
pub struct Deal {
    pub o: a1::order::Order,
    pub derived: k2::derive::Order,
    pub sig_seller: Vec<u8>,
    pub sig_buyer: Vec<u8>,
    pub guild: bool,
    pub guild_addr: String,
    pub seq: Option<u64>,
    pub logic: String,
    pub past: bool,
    pub prev_until: Option<u64>,
}

pub fn past_held(data: &str, me: &str, o: &Order) -> bool {
    a1::chain::past_gate(made_past(data, me, o)).is_err()
}

pub fn generation_at(w: &Wire<'_>, data: &str, me: &str, instance: &str, o: Option<&Order>) -> Result<eravon_literals::constants::Generation, Why> {
    let listing = o.and_then(|o| listed_logic(data, me, o));
    a1::chain::case_generation(w.t, w.table, instance, w.logic, listing.as_deref()).map_err(a1w)
}

pub fn listed_logic(data: &str, me: &str, o: &Order) -> Option<String> {
    order::part(data, me, &o.id, &order::part_listing()).ok().and_then(|b| k3::listing::listing(&b).ok()).map(|p| p.logic)
}

pub fn made_past(data: &str, me: &str, o: &Order) -> bool {
    if eravon_literals::constants::past_rows().is_empty() {
        return false;
    }
    listed_logic(data, me, o).is_some_and(|l| made_under(&l, "").1)
}

fn hex_bytes(what: &str, s: &str) -> Result<Vec<u8>, Why> {
    if s.is_empty() || s == "0x" {
        return Ok(Vec::new());
    }
    a1::order::bytes_of(what, s).map_err(a1w)
}

pub fn sig_bytes(v: &k1::Value) -> Result<Vec<u8>, Why> {
    match v {
        k1::Value::Str(s) => hex_bytes("回执的 sig", s),
        _ => Err(why("E_WATCH_SIG", "回执的 sig 只收六十五字节的 hex 或空串")),
    }
}

pub fn deal(data: &str, me: &str, o: &Order, w: &Wire<'_>) -> Result<Deal, Why> {
    let listing = order::part(data, me, &o.id, &order::part_listing())?;
    let receipt = order::part(data, me, &o.id, &order::part_receipt())?;
    let terms = order::part(data, me, &o.id, &order::part_terms())?;
    let p = k3::listing::listing(&listing).map_err(|f| why(f.token.name(), "订单数据无效"))?;
    let r = k3::receipt::receipt(&receipt).map_err(|f| why(f.token.name(), "订单回执无效"))?;
    let c = o.candidate as usize;
    let (sig, guild_addr) = if c < p.judges.len() {
        (p.judge_sigs.get(c).cloned().unwrap_or_default(), String::new())
    } else {
        let g = c - p.judges.len();
        let el = p.guilds.get(g).ok_or_else(|| why("E_WATCH_CANDIDATE", format!("清单里没有第 {c} 位候选")))?;
        (p.guild_sigs.get(g).cloned().unwrap_or_default(), el.address.to_ascii_lowercase())
    };
    let ao = crate::paste::order_of(&terms, o.candidate, &r.buyer_key, &r.buyer_payout, r.open_by, &r.prev)?;
    let prev_until = (!ao.prev.eq_ignore_ascii_case(a1::order::NO_PREV))
        .then(|| order::all(data, me).ok())
        .flatten()
        .and_then(|v| v.into_iter().find(|x| x.instance.eq_ignore_ascii_case(&ao.prev)))
        .map(|x| x.expiry.min(x.open_by));
    let (logic, past) = made_under(&p.logic, w.logic);
    let derived = ao.derive(w.chain_id, &logic).map_err(a1w)?;
    Ok(Deal {
        o: ao,
        derived,
        sig_seller: hex_bytes("要约签名", &sig)?,
        sig_buyer: sig_bytes(&r.sig)?,
        guild: !guild_addr.is_empty(),
        guild_addr,
        seq: r.seq,
        logic,
        past,
        prev_until,
    })
}

pub fn made_under(listing_logic: &str, current: &str) -> (String, bool) {
    match eravon_literals::constants::generation_of_logic(listing_logic).filter(|g| g.is_past()) {
        Some(g) => (g.row().logic.to_string(), true),
        None => (current.to_string(), false),
    }
}

pub fn open_until(o: &a1::order::Order) -> u64 {
    o.offer.expiry.min(o.open_by)
}

pub struct Reading {
    pub balance: U256,
    pub phase: u64,
    pub seats: u64,
    pub expired: bool,
    pub revoked: bool,
    pub consent: Option<a1::consent::RegRead>,
    pub act: Option<Result<eravon_cands::act::Passed, eravon_cands::act::Refused>>,
    pub prev: Option<(String, Result<a1::chain::Stage, Why>)>,
    pub prev_lapsed: bool,
}

pub fn stage_waits(prev: &Option<(String, Result<a1::chain::Stage, Why>)>, lapsed: bool) -> Option<(State, String)> {
    let (at, got) = prev.as_ref()?;
    match got {
        Ok(s) if s.ended && s.buyer_bps == 0 => None,
        Ok(s) if !s.ended => Some((State::AwaitPrev, format!("待前段:前段 {at} 还没结束"))),
        Ok(s) => Some((State::Blocked, format!("前段 {at} 结束时买家得 {} 基点:这一段不会开案,买家可撤单", s.buyer_bps))),
        Err(_) if lapsed => Some((State::Blocked, format!("前段 {at} {STAGE_LAPSED}:这一段不会开案,买家可撤单"))),
        Err((n, d)) => Some((State::AwaitPrev, format!("待前段:前段 {at} 还读不成本标准已开的实例:{n}:{d}"))),
    }
}

pub const STAGE_LAPSED: &str = "过了开案期没有开案";


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stale {
    pub said: String,
    pub facts: Facts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Facts {
    Round { feed: String, answer: String, updated_at: u64, max_stale: u64 },
    Shape { feed: String },
}

impl Facts {
    pub fn line(&self) -> String {
        match self {
            Facts::Round { feed, answer, updated_at, max_stale } => format!("feed={feed} answer={answer} updatedAt={updated_at} maxStale={max_stale}"),
            Facts::Shape { feed } => format!("feed={feed} shape"),
        }
    }

    pub fn read(line: &str) -> Option<Facts> {
        let mut words = line.split(' ');
        let feed = words.next()?.strip_prefix("feed=")?.to_string();
        let rest: Vec<&str> = words.collect();
        match rest.as_slice() {
            ["shape"] => Some(Facts::Shape { feed }),
            [a, u, m] => Some(Facts::Round {
                feed,
                answer: a.strip_prefix("answer=")?.to_string(),
                updated_at: u.strip_prefix("updatedAt=")?.parse().ok()?,
                max_stale: m.strip_prefix("maxStale=")?.parse().ok()?,
            }),
            _ => None,
        }
    }

    pub fn feed(&self) -> &str {
        match self {
            Facts::Round { feed, .. } | Facts::Shape { feed } => feed,
        }
    }
}

pub fn phase(w: &Wire<'_>, instance: &str) -> Result<(u64, u64), Why> {
    let (code, _) = eravon_chain::read::code(w.t, w.table, instance, eravon_chain::seam::TAG_LATEST).map_err(|e| why(e.name(), e.detail()))?;
    if code.is_empty() {
        return Ok((0, 0));
    }
    a1::chain::code_generation(w.t, w.table, instance, w.logic).map_err(a1w)?;
    let s = a1::chain::status(w.t, w.table, instance).map_err(a1w)?.value;
    let col = |i: usize, what: &str| -> Result<u64, Why> { s.get(i).ok_or_else(|| why("E_WATCH_STATUS", format!("status() 没有答 {what}")))?.small().map_err(a1w) };
    Ok((col(a1::chain::status_col::PHASE, "phase")?, col(a1::chain::status_col::SEATS, "seats")?))
}

pub fn opened(w: &Wire<'_>, instance: &str) -> Result<bool, Why> {
    Ok(phase(w, instance)?.0 != 0)
}

pub fn expired(until: u64, now: u64) -> bool {
    a1::precheck::in_window(until, now).is_err()
}


fn consent_blocks(r: &a1::consent::RegRead) -> bool {
    r.state != a1::consent::Registration::Held
}


pub fn stray(w: &Wire<'_>, instance: &str, token: &str, st: &[a1::abi::Val], amount: &U256) -> Result<U256, Why> {
    use a1::chain::status_col as col;
    let word = |i: usize, what: &str| -> Result<U256, Why> {
        let v = st.get(i).ok_or_else(|| why("E_WATCH_STATUS", format!("status() 没有答 {what}")))?;
        Ok(U256::from_be(&v.word().map_err(a1w)?))
    };
    let phase: u64 = word(col::PHASE, "phase")?.to_dec().parse().unwrap_or(u64::MAX);
    let mut held = word(col::UNPAID, "unpaid")?;
    if (1..4).contains(&phase) {
        for x in [*amount, word(col::DEPOSIT, "deposit")?, word(col::LF_HELD, "lfHeld")?] {
            held = held.overflowing_add(&x).0;
        }
    }
    let bal = U256::from_be(&a1::chain::erc20_balance(w.t, w.table, token, instance).map_err(a1w)?.value);
    Ok(if bal.cmp_u(&held) == core::cmp::Ordering::Greater { bal.overflowing_sub(&held).0 } else { U256::from_be(&[0u8; 32]) })
}

pub fn ahead_rounds(w: &Wire<'_>, orders: &[(&Order, &Deal)]) {
    if orders.len() < 2 {
        return;
    }
    let mut instances: Vec<String> = orders.iter().map(|(o, _)| o.instance.clone()).collect();
    for (_, d) in orders {
        if !instances.iter().any(|i| i.eq_ignore_ascii_case(&d.logic)) {
            instances.push(d.logic.clone());
        }
    }
    eravon_chain::read::ahead_codes(w.t, w.table, &instances, eravon_chain::seam::TAG_LATEST);
    let mut views: Vec<(String, Vec<u8>)> = Vec::new();
    for (o, d) in orders {
        if let Ok(data) = a1::order::addr_of("谁", &o.instance).and_then(|a| a1::call::erc20_balance_of(&a)) {
            views.push((o.token.clone(), data));
        }
        if let Ok(data) = a1::call::view(a1::sol::STATUS) {
            views.push((o.instance.clone(), data));
        }
        if let Ok(data) = a1::order::addr_of("卖家钥", &d.o.offer.seller_key).and_then(|s| a1::call::revoked(&s, &d.o.offer.terms_hash)) {
            views.push((d.logic.clone(), data));
        }
        if o.verdict == "RECEIPT_BY_CALL" {
            if let Ok(data) = a1::order::addr_of("登记人", &o.buyer_key).and_then(|b| a1::call::consented(&b, &d.derived.order_digest)) {
                views.push((d.logic.clone(), data));
            }
        }
    }
    a1::chain::ahead_views(w.t, w.table, &views);
}

pub fn read(w: &Wire<'_>, o: &Order, d: &Deal, now: u64) -> Result<Reading, Why> {
    let bal = U256::from_be(&a1::chain::erc20_balance(w.t, w.table, &o.token, &o.instance).map_err(a1w)?.value);
    let (phase, seats) = phase(w, &o.instance)?;
    let expired = expired(open_until(&d.o), now);
    let revoked = a1::chain::revoked(w.t, w.table, &d.logic, &d.o.offer.seller_key, &d.o.offer.terms_hash).map_err(a1w)?.value;
    let consent = (o.verdict == "RECEIPT_BY_CALL").then(|| a1::consent::registered(w.t, w.table, &d.logic, &o.buyer_key, &d.derived.order_digest));
    let need = U256::from_dec(&o.need).unwrap_or(U256::MAX);
    let open = phase == 0 && !expired && !revoked && bal.cmp_u(&need) != core::cmp::Ordering::Less;
    let prev = (open && !d.o.prev.eq_ignore_ascii_case(a1::order::NO_PREV)).then(|| (d.o.prev.clone(), a1::chain::stage(w.t, w.table, &d.o.prev).map(|r| r.value).map_err(a1w)));
    let prev_lapsed = prev.is_some() && d.prev_until.is_some_and(|u| self::expired(u, now));
    let act = open.then(|| crate::act::check(w, &d.o));
    Ok(Reading { balance: bal, phase, seats, expired, revoked, consent, act, prev, prev_lapsed })
}

pub const REFUNDED: u64 = 4;

fn standing(phase: u64, seats: u64, revoked: bool, expired: bool) -> Option<State> {
    if phase == REFUNDED && seats == 0 {
        return Some(State::Cancelled);
    }
    if phase != 0 {
        return Some(State::Deployed);
    }
    if revoked {
        return Some(State::Revoked);
    }
    expired.then_some(State::Expired)
}

pub fn lapsed(w: &Wire<'_>, instance: &str, logic: &str, seller_key: &str, terms_hash: &str, until: u64, now: u64) -> Result<bool, Why> {
    let (phase, seats) = phase(w, instance)?;
    let expired = expired(until, now);
    let revoked = phase == 0 && !expired && a1::chain::revoked(w.t, w.table, logic, seller_key, &a1::order::b32_of("termsHash", terms_hash).map_err(a1w)?).map_err(a1w)?.value;
    Ok(matches!(standing(phase, seats, revoked, expired), Some(State::Cancelled | State::Revoked | State::Expired)))
}

pub fn decide(o: &Order, r: &Reading) -> Result<(State, String, String), Why> {
    let need = U256::from_dec(&o.need)
        .ok_or_else(|| why("E_WATCH_RECORD", format!("应到的数读不成:{}", o.need)))?;
    match standing(r.phase, r.seats, r.revoked, r.expired) {
        Some(State::Revoked) => return Ok((State::Revoked, String::new(), "卖家已下架这份条款:开不了案,买家可取回".into())),
        Some(s) => return Ok((s, String::new(), String::new())),
        None => {}
    }
    if r.balance.is_zero() {
        return Ok((State::Awaiting, String::new(), String::new()));
    }
    if r.balance.cmp_u(&need) == core::cmp::Ordering::Less {
        return Ok(short(&need, &r.balance));
    }
    if let Some(c) = &r.consent
        && consent_blocks(c)
    {
        return Ok((
            State::PendingConsent,
            String::new(),
            format!(
                "等待买家同意:{}",
                if c.why.is_empty() {
                    c.state.word()
                } else {
                    &c.why
                }
            ),
        ));
    }
    if let Some((state, why)) = stage_waits(&r.prev, r.prev_lapsed) {
        return Ok((state, String::new(), why));
    }
    match &r.act {
        Some(Err(f)) if [a1::precheck::GATE_BOND, a1::precheck::GATE_FUNDS, a1::precheck::GATE_ALLOWANCE].contains(&f.gate) => {
            Ok((State::DepositShort, String::new(), format!("卖家押金不足,开不了案:{}:{}", f.name, f.why)))
        }
        Some(Err(f)) if f.gate == a1::precheck::GATE_PRICE => Ok((State::PriceStale, String::new(), format!("{};恢复后再发", f.why))),
        Some(Err(f)) => Ok((State::Blocked, String::new(), format!("开案前核不过:{}:{}:{}", f.gate, f.name, f.why))),
        _ => Ok((State::Ready, String::new(), String::new())),
    }
}

fn short(need: &U256, have: &U256) -> (State, String, String) {
    let (gap, _) = need.overflowing_sub(have);
    (State::Short, gap.to_dec(), order::GAP_NOTE.to_string())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn o(need: &str, verdict: &str) -> Order {
        Order {
            id: "o1".into(),
            case_id: "0x".to_string() + &"11".repeat(32),
            instance: "0x".to_string() + &"22".repeat(20),
            buyer_key: "0x".to_string() + &"33".repeat(20),
            buyer_payout: String::new(),
            verdict: verdict.into(),
            token: String::new(),
            need: need.into(),
            open_by: 0,
            candidate: 0,
            expiry: 100,
            state: State::Awaiting,
            balance: String::new(),
            gap: String::new(),
            note: String::new(),
            notice: String::new(),
            notice_at: 0,
            deploy_tx: String::new(),
            expire_tx: String::new(),
            source: "paste".into(),
            opened: 0,
            extra: Vec::new(),
        }
    }

    fn r(
        balance: &str,
        consent: Option<a1::consent::Registration>,
        refused: Option<&'static str>,
    ) -> Reading {
        Reading {
            balance: U256::from_dec(balance).expect("十进制"),
            phase: 0,
            seats: 0,
            expired: false,
            revoked: false,
            consent: consent.map(|state| a1::consent::RegRead {
                state,
                endpoints: 2,
                why: String::new(),
            }),
            act: refused.map(|gate| Err(eravon_cands::act::Refused { gate, name: "E_X".into(), why: "旧".into() })),
            prev: None,
            prev_lapsed: false,
        }
    }

    #[test]
    fn an_orders_standing_before_opening_is_one_judgement() {
        assert_eq!(standing(REFUNDED, 0, false, false), Some(State::Cancelled));
        assert_eq!(standing(REFUNDED, 1, true, true), Some(State::Deployed), "an opened case stands, whatever its window");
        assert_eq!(standing(1, 1, false, true), Some(State::Deployed));
        assert_eq!(standing(0, 0, true, true), Some(State::Revoked));
        assert_eq!(standing(0, 0, false, true), Some(State::Expired));
        assert_eq!(standing(0, 0, false, false), None, "can still open");
    }

    #[test]
    fn a_later_stage_waits_for_the_stage_before() {
        let st = |ended, buyer_bps| a1::chain::Stage { seller_key: String::new(), buyer_key: String::new(), ended, buyer_bps, series_judge: String::new(), case_id: [0; 32], past: false, logic: String::new() };
        let at = |x: Result<a1::chain::Stage, Why>| Some(("0xprev".to_string(), x));
        assert_eq!(stage_waits(&None, false), None, "无前段不等");
        assert_eq!(stage_waits(&at(Ok(st(true, 0))), false), None, "前段结束、买家零份额:可开");
        assert!(stage_waits(&at(Ok(st(false, 0))), false).is_some_and(|(s, w)| s == State::AwaitPrev && w.contains("还没结束")));
        assert!(stage_waits(&at(Ok(st(true, 5000))), false).is_some_and(|(s, w)| s == State::Blocked && w.contains("5000")));
        assert!(stage_waits(&at(Err(why("E_A1_CODEHASH", "x"))), false).is_some_and(|(s, w)| s == State::AwaitPrev && w.contains("读不成")));
        assert!(stage_waits(&at(Err(why("E_A1_CODEHASH", "x"))), true).is_some_and(|(s, w)| s == State::Blocked && w.contains("开案期")));
        assert!(stage_waits(&at(Ok(st(false, 0))), true).is_some_and(|(s, _)| s == State::AwaitPrev), "an opened stage before still runs: waits");
        let mut r0 = r("100", None, None);
        r0.prev = at(Ok(st(false, 0)));
        assert_eq!(decide(&o("100", "RECEIPT_BOUND"), &r0).expect("答").0, State::AwaitPrev);
    }

    #[test]
    fn states_follow_the_design_order() {
        use a1::consent::Registration::{Held, NotHeld};
        assert_eq!(
            decide(&o("100", "RECEIPT_BOUND"), &r("0", None, None))
                .unwrap()
                .0,
            State::Awaiting
        );
        let (s, gap, note) = decide(&o("100", "RECEIPT_BOUND"), &r("40", None, None)).unwrap();
        assert_eq!(
            (s, gap.as_str(), note.as_str()),
            (State::Short, "60", order::GAP_NOTE)
        );
        assert_eq!(
            decide(&o("100", "RECEIPT_BY_CALL"), &r("100", Some(NotHeld), None))
                .unwrap()
                .0,
            State::PendingConsent
        );
        assert_eq!(
            decide(&o("100", "RECEIPT_BY_CALL"), &r("100", Some(Held), None))
                .unwrap()
                .0,
            State::Ready
        );
        assert_eq!(decide(&o("100", "RECEIPT_BOUND"), &r("100", None, Some(a1::precheck::GATE_PRICE))).unwrap().0, State::PriceStale);
        for g in [a1::precheck::GATE_BOND, a1::precheck::GATE_FUNDS, a1::precheck::GATE_ALLOWANCE] {
            assert_eq!(decide(&o("100", "RECEIPT_BOUND"), &r("100", None, Some(g))).unwrap().0, State::DepositShort, "{g}:押金不足标在订单上");
        }
        assert_eq!(decide(&o("100", "RECEIPT_BOUND"), &r("100", None, Some("judge"))).unwrap().0, State::Blocked);
        let mut x = r("100", None, None);
        x.expired = true;
        assert_eq!(
            decide(&o("100", "RECEIPT_BOUND"), &x).unwrap().0,
            State::Expired
        );
        x.revoked = true;
        assert_eq!(decide(&o("100", "RECEIPT_BOUND"), &x).unwrap().0, State::Revoked, "撤单先于过期:付了即给取回");
        x.phase = 2;
        x.seats = 1;
        assert_eq!(
            decide(&o("100", "RECEIPT_BOUND"), &x).unwrap().0,
            State::Deployed
        );
        x.phase = REFUNDED;
        x.seats = 0;
        assert_eq!(decide(&o("100", "RECEIPT_BOUND"), &x).unwrap().0, State::Cancelled, "Refunded(4) 读成已撤单");
    }

    #[test]
    fn state_words_round_trip() {
        for s in State::ALL {
            assert_eq!(State::of(s.word()), Some(s));
        }
    }

    #[test]
    fn a_stale_feeds_facts_read_back_from_their_line() {
        let feed = format!("0x{}", "ab".repeat(20));
        for f in [
            Facts::Round { feed: feed.clone(), answer: "-5".into(), updated_at: 1_700_000_000, max_stale: 3600 },
            Facts::Shape { feed: feed.clone() },
        ] {
            assert_eq!(Facts::read(&f.line()), Some(f.clone()), "{}", f.line());
            assert_eq!(f.feed(), feed);
        }
        for bad in ["", "feed=0x1", "feed=0x1 answer=1 updatedAt=x maxStale=1", "价源 0x1 陈旧"] {
            assert_eq!(Facts::read(bad), None, "{bad}");
        }
    }

}
