use std::time::Duration;

use eravon_chain::sending as a2send;
use eravon_literals::json::Out;
use eravon_standard as a1;

use crate::fault::{Fault, R};
use crate::home::Ctx;
use crate::shared::{self, Flags, Wire};

const WAIT_SECS: u64 = 60;

const TICK: Duration = Duration::from_millis(200);

pub struct Delivered {
    pub tx_hash: String,
    pub endpoint: eravon_chain::seam::NodeAt,
    pub receipt: Result<Option<a2send::Receipt>, Fault>,
    pub nonce: u64,
    pub gas_limit: u64,
    pub fee_source: &'static str,
    pub max_fee: String,
    pub priority_fee: String,
    pub closing: Option<String>,
}

impl Delivered {
    pub fn verdict(&self) -> Verdict {
        match &self.receipt {
            Ok(Some(r)) if r.status == 1 => Verdict::Mined,
            Ok(Some(_)) => Verdict::Reverted,
            Ok(None) => Verdict::Unknown("没等收据(--wait 0)".into()),
            Err(e) => Verdict::Unknown(e.detail()),
        }
    }

    pub fn outcome(&self) -> Result<String, (String, String)> {
        match self.verdict() {
            Verdict::Mined => Ok(self.tx_hash.clone()),
            Verdict::Reverted => Err(("E_V_REVERTED".into(), format!("{} 被链回滚了", self.tx_hash))),
            Verdict::Unknown(why) => Err(("E_V_UNCONFIRMED".into(), format!("{} 发出去了而说不出成败:{why};别再发这一笔,先在链上核实", self.tx_hash))),
        }
    }
}

pub enum Verdict {
    Mined,
    Reverted,
    Unknown(String),
}

pub fn series<'a>(calls: impl IntoIterator<Item = &'a a1::call::Call>, mut one: impl FnMut(&a1::call::Call) -> R<Delivered>) -> R<Vec<String>> {
    let calls: Vec<&a1::call::Call> = calls.into_iter().collect();
    let mut sent: Vec<(String, &'static str)> = Vec::new();
    for call in calls {
        let before = sent.len();
        let stop = match one(call) {
            Err(e) => e,
            Ok(d) => match d.verdict() {
                Verdict::Mined => {
                    sent.push((d.tx_hash.clone(), "mined"));
                    continue;
                }
                Verdict::Reverted => {
                    sent.push((d.tx_hash.clone(), "reverted"));
                    Fault::Reverted { tx_hash: d.tx_hash.clone() }
                }
                Verdict::Unknown(why) => {
                    sent.push((d.tx_hash.clone(), "unconfirmed"));
                    Fault::Unconfirmed { tx_hash: d.tx_hash.clone(), why }
                }
            },
        };
        return Err(if before == 0 { stop } else { Fault::Series { sent, stop: Box::new(stop) } });
    }
    Ok(sent.into_iter().map(|(tx, _)| tx).collect())
}

pub fn asked(f: &Flags) -> bool {
    f.one("--send") == Some("true")
}

#[derive(Clone, Debug, Default)]
pub struct Opts {
    pub gas_limit: Option<u64>,
    pub fees: Option<(String, String)>,
    pub wait: Option<u64>,
    pub nonce: Option<u64>,
}

impl Opts {
    pub fn of(f: &Flags) -> R<Opts> {
        let fees = match (f.opt("--max-fee"), f.opt("--priority-fee")) {
            (Some(m), Some(p)) => Some((m, p)),
            (None, None) => None,
            _ => return Err(Fault::Arg("--max-fee 与 --priority-fee 要两个一起给".into())),
        };
        Ok(Opts { gas_limit: f.opt_int("--gas-limit")?, fees, wait: f.opt_int("--wait")?, nonce: None })
    }
}

pub fn deliver(
    c: &Ctx,
    f: &Flags,
    w: &Wire,
    who: &str,
    call: &a1::call::Call,
    value: &str,
) -> R<Delivered> {
    let o = Opts::of(f)?;
    hand(c, f, w, who, call, value, &o, None)
}

pub fn deliver_landed(c: &Ctx, f: &Flags, w: &Wire, who: &str, call: &a1::call::Call, value: &str, land: Land<'_>) -> R<Delivered> {
    deliver_with(c, f, w, who, call, value, Some(land))
}

pub fn deliver_with(c: &Ctx, f: &Flags, w: &Wire, who: &str, call: &a1::call::Call, value: &str, land: Option<Land<'_>>) -> R<Delivered> {
    let o = Opts::of(f)?;
    hand(c, f, w, who, call, value, &o, land)
}

pub fn deliver_again(c: &Ctx, f: &Flags, w: &Wire, who: &str, call: &a1::call::Call, at: &Pending, land: Option<Land<'_>>) -> R<Delivered> {
    let o = replacement(w, at)?;
    hand(c, f, w, who, call, "0", &o, land)
}

#[allow(clippy::too_many_arguments)]
fn hand(c: &Ctx, _f: &Flags, w: &Wire, who: &str, call: &a1::call::Call, value: &str, o: &Opts, land: Option<Land<'_>>) -> R<Delivered> {
    eravon_vault::session::need_open(&*c.session(None)?).map_err(crate::home::down)?;
    let send = |key_of: &dyn Fn() -> R<[u8; 32]>, land: Option<Land<'_>>| match land {
        Some(l) => deliver_key_landed(key_of, o, w, who, call, value, l),
        None => direct(key_of, o, w, who, call, value),
    };
    send(&|| c.key(who, None), land)
}

pub type Land<'a> = &'a mut dyn FnMut(u64, &str, &eravon_watch::order::Broadcast) -> R<()>;

impl Delivered {
    pub fn broadcast(&self) -> eravon_watch::order::Broadcast {
        eravon_watch::order::Broadcast { nonce: self.nonce, max_fee: self.max_fee.clone(), priority_fee: self.priority_fee.clone(), gas_limit: self.gas_limit }
    }
}

pub fn pending_of(b: &eravon_watch::order::Broadcast) -> Pending {
    Pending { nonce: b.nonce, max_fee: b.max_fee.clone(), priority_fee: b.priority_fee.clone(), gas_limit: b.gas_limit }
}

pub fn deliver_key(
    key_of: &dyn Fn() -> R<[u8; 32]>,
    o: &Opts,
    w: &Wire,
    who: &str,
    call: &a1::call::Call,
    value: &str,
) -> R<Delivered> {
    direct(key_of, o, w, who, call, value)
}

pub fn direct(key_of: &dyn Fn() -> R<[u8; 32]>, o: &Opts, w: &Wire, who: &str, call: &a1::call::Call, value: &str) -> R<Delivered> {
    let machine = eravon_store::writer::machine_dir().map_err(|e| Fault::Io(format!("直发的记录没处放:{}", e.detail())))?;
    let dir = eravon_send::sent::place(&machine, w.chain_id, &account(who)?, &account(&call.to)?, &call.data, &word(value)?);
    direct_at(&dir, key_of, o, w, who, call, value)
}

fn account(a: &str) -> R<[u8; 20]> {
    eravon::hexfmt::decode(a.trim()).and_then(|b| b.try_into().ok()).ok_or_else(|| Fault::Arg(format!("账户要是 0x 加四十位十六进制:{a}")))
}

pub fn hash_of(tx: &str) -> R<[u8; 32]> {
    eravon::hexfmt::decode(tx.trim()).and_then(|b| b.try_into().ok()).ok_or_else(|| Fault::Arg(format!("交易号要是 0x 加六十四位十六进制:{tx}")))
}

pub fn record_fault(said: String) -> Fault {
    Fault::Io(format!("直发的记录读写不成:{said}"))
}

#[allow(clippy::too_many_arguments)]
fn direct_at(dir: &std::path::Path, key_of: &dyn Fn() -> R<[u8; 32]>, o: &Opts, w: &Wire, who: &str, call: &a1::call::Call, value: &str) -> R<Delivered> {
    use eravon_send::sent;
    let dir = dir.to_path_buf();
    let seal = std::rc::Rc::new(sent::Seal::of(&key_of()?, &dir));
    let open = if o.nonce.is_none() { sent::records(&dir, &seal).map_err(record_fault)? } else { Vec::new() };
    let (from, to) = (who.to_ascii_lowercase(), call.to.to_ascii_lowercase());
    let record = |dir: &std::path::Path| {
        let dir = dir.to_path_buf();
        let (from, to, seal) = (from.clone(), to.clone(), seal.clone());
        move |n: u64, tx: &str, b: &eravon_watch::order::Broadcast| {
            let c = sent::Carried { max_fee: b.max_fee.clone(), priority_fee: b.priority_fee.clone(), gas_limit: b.gas_limit, from: from.clone(), to: to.clone() };
            sent::land_carried(&dir, &seal, n, &hash_of(tx)?, &c).map_err(record_fault)
        }
    };
    let closed = |d: R<Delivered>| -> R<Delivered> {
        let mut x = d?;
        if matches!(x.receipt, Ok(Some(_))) {
            let landed = hash_of(&x.tx_hash).and_then(|h| {
                let others: Vec<(u64, [u8; 32])> = open.iter().filter(|r| r.tx != h && r.nonce == x.nonce).map(|r| (r.nonce, r.tx)).collect();
                sent::void(&dir, &seal, &others).map_err(record_fault)?;
                sent::done(&dir, &seal, &h).map_err(record_fault)
            });
            x.closing = landed.err().map(|e| format!("{}:{}", e.name(), e.detail()));
        }
        Ok(x)
    };
    let Some(last) = open.last().cloned() else {
        return closed(deliver_key_landed(key_of, o, w, who, call, value, &mut record(&dir)));
    };
    let answered = |r: &sent::Rec, receipt: Result<Option<a2send::Receipt>, Fault>| Delivered {
        tx_hash: r.tx_hex(),
        endpoint: eravon_chain::seam::NodeAt::new(String::new()),
        receipt,
        nonce: r.nonce,
        gas_limit: r.carried.gas_limit,
        fee_source: "record",
        max_fee: r.carried.max_fee.clone(),
        priority_fee: r.carried.priority_fee.clone(),
        closing: None,
    };
    eravon_chain::read::ahead_receipts(&w.t, &w.table, &open.iter().map(|r| r.tx_hex()).collect::<Vec<_>>());
    for r in &open {
        if let Some(got) = a2send::receipt(&w.t, &w.table, &r.tx_hex()).map_err(|f| unheard(&r.tx_hex(), f))? {
            return closed(Ok(answered(r, Ok(Some(got)))));
        }
    }
    for r in &open {
        let (held, _) = eravon_chain::read::transaction(&w.t, &w.table, &r.tx_hex()).map_err(|f| unheard(&r.tx_hex(), f))?;
        if matches!(held, eravon_literals::json::In::Null) {
            continue;
        }
        let wait = o.wait.unwrap_or(WAIT_SECS);
        let Some(url) = w.table.list.first().filter(|_| wait > 0) else { return Ok(answered(r, Ok(None))) };
        let at = eravon_chain::seam::NodeAt::new(url.clone());
        let mut d = answered(r, waited(w, &at, &r.tx_hex(), wait).map(Some));
        d.endpoint = at;
        return closed(Ok(d));
    }
    let at = eravon_watch::order::Broadcast { nonce: last.nonce, max_fee: last.carried.max_fee.clone(), priority_fee: last.carried.priority_fee.clone(), gas_limit: last.carried.gas_limit };
    let ww = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let all: Vec<(u64, [u8; 32])> = open.iter().map(|x| (x.nonce, x.tx)).collect();
    match eravon_watch::send::void_check(who, &ww, &last.tx_hex(), &at).map_err(|(n, d)| Fault::Down { layer: "S4", token: n, why: d })? {
        eravon_watch::send::Voided::Void => {
            sent::void(&dir, &seal, &all).map_err(record_fault)?;
            Err(Fault::SentVoid { tx_hash: last.tx_hex(), nonce: last.nonce })
        }
        eravon_watch::send::Voided::Held(d) => Err(Fault::Down { layer: "S4", token: eravon_watch::send::NOT_VOID.to_string(), why: d }),
        eravon_watch::send::Voided::Unused { .. } => {
            let again = Opts { nonce: Some(last.nonce), gas_limit: Some(last.carried.gas_limit), ..o.clone() };
            closed(deliver_key_landed(key_of, &again, w, who, call, value, &mut record(&dir)))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn deliver_key_landed(
    key_of: &dyn Fn() -> R<[u8; 32]>,
    o: &Opts,
    w: &Wire,
    who: &str,
    call: &a1::call::Call,
    value: &str,
    land: Land<'_>,
) -> R<Delivered> {
    let value_word = word(value)?;
    a1::gate::sender_current(&w.t, &w.table, who, call)?;
    let (head, _) = eravon_chain::read::height(&w.t, &w.table).map_err(Fault::from)?;
    let at = eravon_chain::seam::quantity(head);
    let ctx = std::sync::Arc::new((w.clone(), who.to_string(), call.clone(), value.to_string(), o.clone(), at));
    let (c1, c2, c3) = (ctx.clone(), ctx.clone(), ctx);
    let (gas_limit, (fees_got, balance)) = eravon_chain::read::both(
        move || match c1.4.gas_limit {
            Some(g) => Ok(g),
            None => limit_at(&c1.0, head, &c1.1, &c1.2, &c1.3),
        },
        move || eravon_chain::read::both(move || fees(&c2.4, &c2.0, head), move || eravon_chain::read::balance(&c3.0.t, &c3.0.table, &c3.1, &c3.5).map_err(Fault::from)),
    );
    let gas_limit = gas_limit?;
    let (max_fee, priority_fee, fee_source) = fees_got?;
    let (balance, _) = balance?;
    let need = most_spent(&max_fee, gas_limit, &value_word);
    if !covers(&balance, &need) {
        return Err(Fault::Precheck { gate: crate::fault::GATE_FEE.into(), why: fee_why(&balance, &need) });
    }

    let nonce = match o.nonce {
        Some(n) => n,
        None => a2send::pending_nonce(&w.t, &w.table, who)?.0,
    };

    let tx = a1::tx::assemble(
        w.chain_id,
        nonce,
        &priority_fee,
        &max_fee,
        gas_limit,
        &call.to,
        &value_word,
        &call.data,
    )?;

    let key = key_of()?;
    let sig_hex = shared::sign(&key, &tx.digest())?;
    let sig = a1::order::bytes_of("签名", &sig_hex)?;
    let raw = tx.raw(&sig)?;
    let tx_hash = eravon_chain::seam::hex_of(&tx.hash(&sig)?);
    let (max_fee, priority_fee) = (dec(&max_fee), dec(&priority_fee));

    let carried = eravon_watch::order::Broadcast { nonce, max_fee: max_fee.clone(), priority_fee: priority_fee.clone(), gas_limit };

    let sent = land_then_hand(land, &carried, w, &raw, &tx_hash)?;
    let receipt = settle(o.wait, w, &sent);
    Ok(Delivered {
        tx_hash: sent.tx_hash,
        endpoint: sent.endpoint,
        receipt,
        nonce,
        gas_limit,
        fee_source,
        max_fee,
        priority_fee,
        closing: None,
    })
}

fn land_then_hand(land: Land<'_>, carried: &eravon_watch::order::Broadcast, w: &Wire, raw: &[u8], tx_hash: &str) -> R<a2send::Sent> {
    land(carried.nonce, tx_hash, carried).map_err(|e| Fault::Io(format!("签好的 {tx_hash} 没记下,一个字节也没交出去:{}:{}", e.name(), e.detail())))?;
    a2send::send_everywhere(&w.t, &w.table, raw, tx_hash).map_err(|f| unheard(tx_hash, f))
}

fn unheard(tx_hash: &str, f: eravon_chain::fault::Fault) -> Fault {
    Fault::Unheard { tx_hash: tx_hash.to_string(), why: format!("{}:{}", f.name(), f.detail()) }
}


fn dec(w: &[u8; 32]) -> String {
    eravon_terms::u256::U256::from_be(w).to_dec()
}

pub const REPLACE_BUMP_PERCENT: u128 = eravon_chain::send::REPLACE_BUMP_PERCENT as u128;

pub fn replacing(old: (&str, &str), now: (&str, &str)) -> R<(String, String)> {
    let n = |x: &str| x.parse::<u64>().map_err(|_| Fault::Arg(format!("费率要是十进制整数:{x}")));
    let fees = |(cap, tip): (&str, &str)| -> R<eravon_chain::send::Fees> { Ok(eravon_chain::send::Fees { max_fee: n(cap)?, priority: n(tip)?, gas_limit: 0, from_chain: true }) };
    let r = eravon_chain::send::Fees::replacing(fees(old)?, fees(now)?);
    Ok((r.max_fee.to_string(), r.priority.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub nonce: u64,
    pub max_fee: String,
    pub priority_fee: String,
    pub gas_limit: u64,
}

pub fn bump(key_of: &dyn Fn() -> R<[u8; 32]>, w: &Wire, who: &str, call: &a1::call::Call, value: &str, at: &Pending) -> R<Delivered> {
    deliver_key(key_of, &replacement(w, at)?, w, who, call, value)
}

pub fn replacement(w: &Wire, at: &Pending) -> R<Opts> {
    let f = eravon_chain::gas::suggested_fees(&w.t, &w.table)?;
    let fees = replacing((&at.max_fee, &at.priority_fee), (&f.cap, &f.tip))?;
    Ok(Opts { gas_limit: Some(at.gas_limit), fees: Some(fees), wait: None, nonce: Some(at.nonce) })
}

pub fn pinned(o: &Opts, w: &Wire, who: &str, call: &a1::call::Call) -> R<(Opts, String)> {
    let (head, _) = eravon_chain::read::height(&w.t, &w.table).map_err(Fault::from)?;
    let gas_limit = match o.gas_limit {
        Some(g) => g,
        None => limit_at(w, head, who, call, "0")?,
    };
    let (max_fee, priority_fee, _) = fees(o, w, head)?;
    let most = most_spent(&max_fee, gas_limit, &[0u8; 32]);
    Ok((Opts { gas_limit: Some(gas_limit), fees: Some((dec(&max_fee), dec(&priority_fee))), ..o.clone() }, most))
}

fn most_spent(max_fee: &[u8; 32], gas_limit: u64, value: &[u8; 32]) -> String {
    let limbs = |w: &[u8; 32]| -> Vec<u128> { (0..8).map(|i| u32::from_be_bytes(w[28 - 4 * i..32 - 4 * i].try_into().unwrap_or([0; 4])) as u128).collect() };
    let mut acc = vec![0u128; 12];
    let (fee, val) = (limbs(max_fee), limbs(value));
    let gl = [(gas_limit & 0xffff_ffff) as u128, (gas_limit >> 32) as u128];
    for (i, f) in fee.iter().enumerate() {
        for (j, g) in gl.iter().enumerate() {
            acc[i + j] += f * g;
        }
    }
    for (i, v) in val.iter().enumerate() {
        acc[i] += v;
    }
    for i in 0..acc.len() - 1 {
        let carry = acc[i] >> 32;
        acc[i] &= 0xffff_ffff;
        acc[i + 1] += carry;
    }
    let mut digits: Vec<u8> = Vec::new();
    let mut n = acc;
    while n.iter().any(|x| *x != 0) {
        let mut rem = 0u128;
        for x in n.iter_mut().rev() {
            let cur = (rem << 32) | *x;
            *x = cur / 10;
            rem = cur % 10;
        }
        digits.push(b'0' + rem as u8);
    }
    if digits.is_empty() {
        digits.push(b'0');
    }
    digits.reverse();
    String::from_utf8(digits).unwrap_or_default()
}

const FEE_HAVE: &str = "余额 ";
const FEE_NEED: &str = " wei 付不起这一笔最多要花的 ";
const FEE_TAIL: &str = " wei(费率上限 × gas 上限 + 随附金额)";

fn fee_why(balance: &str, need: &str) -> String {
    format!("{FEE_HAVE}{balance}{FEE_NEED}{need}{FEE_TAIL}")
}

pub fn fee_gap(detail: &str) -> Option<String> {
    let (gate, why) = crate::fault::precheck_of(detail)?;
    if gate != crate::fault::GATE_FEE {
        return None;
    }
    let (have, rest) = why.strip_prefix(FEE_HAVE)?.split_once(FEE_NEED)?;
    let need = rest.strip_suffix(FEE_TAIL)?;
    let (have, need) = (have.parse::<u128>().ok()?, need.parse::<u128>().ok()?);
    Some(need.saturating_sub(have).to_string())
}

fn covers(balance: &str, need: &str) -> bool {
    let digits = |d: &str| !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit());
    let wide = |d: &str| d.parse::<u128>().unwrap_or(u128::MAX);
    let need = if digits(need) { wide(need) } else { u128::MAX };
    let have = if digits(balance) { wide(balance) } else { 0 };
    eravon_send::batch::funds_check(need, have).is_ok()
}

pub fn limit(w: &Wire, who: &str, call: &a1::call::Call, value: &str) -> R<u64> {
    let (head, _) = eravon_chain::read::height(&w.t, &w.table).map_err(Fault::from)?;
    limit_at(w, head, who, call, value)
}

fn limit_at(w: &Wire, head: u64, who: &str, call: &a1::call::Call, value: &str) -> R<u64> {
    let value_q = quantity(&word(value)?);
    let (estimated, _) = eravon_chain::gas::estimate_pinned(&w.t, &w.table, head, Some(who), &call.to, Some(&value_q), Some(&call.hex()))?;
    Ok(margin(estimated).saturating_add(push_room(call)).saturating_add(route_room(w, call)?))
}

fn route_room(w: &Wire, call: &a1::call::Call) -> R<u64> {
    if call.data.get(..4) != Some(&a1::sol::REDRAW.selector()[..]) {
        return Ok(0);
    }
    let st = a1::chain::status(&w.t, &w.table, &call.to)?.value;
    let router = st.get(a1::chain::status_col::ROUTER).ok_or_else(|| a1::fault::Fault::Abi("status 没有答 router".into()))?.addr()?;
    if router == [0u8; 20] {
        return Ok(0);
    }
    let read = a1::chain::call_read(&w.t, &w.table, &call.to, &a1::sol::ROUTE_GAS.selector(), &[a1::abi::Ty::Uint])?.value;
    let g = read.first().ok_or_else(|| a1::fault::Fault::Abi("routeGas 的答是空的".into()))?.small()?;
    route_gas_held(g)
}

fn route_gas_held(g: u64) -> R<u64> {
    let law = eravon_literals::constants::ROUTE_GAS;
    if g != law {
        return Err(Fault::Precheck { gate: crate::fault::GATE_ROUTE.into(), why: format!("实例的 routeGas() 答 {g},法定 {law};不是本标准的实例,一笔也没发") });
    }
    Ok(g + g / 63)
}

const PUSH_GAS: u64 = 1_000_000;

const PUSHES: [(&a1::sol::Fun, u64); 13] = [
    (&a1::sol::CONFIRM, 3),
    (&a1::sol::CONFIRM_BY_SIG, 3),
    (&a1::sol::SETTLE, 3),
    (&a1::sol::REFUND, 3),
    (&a1::sol::REFUND_BY_SIG, 3),
    (&a1::sol::ADVANCE, 3),
    (&a1::sol::REDRAW, 3),
    (&a1::sol::DISPUTE, 3),
    (&a1::sol::DISPUTE_BY_SIG, 3),
    (&a1::sol::SWEEP, 2),
    (&a1::sol::CANCEL, 1),
    (&a1::sol::INIT, 1),
    (&a1::sol::JUDGE_FACE[2], 1),
];

fn push_room(call: &a1::call::Call) -> u64 {
    let Some(sel) = call.data.get(..4) else { return 0 };
    let pushes = PUSHES.iter().find(|(f, _)| sel == f.selector()).map_or(0, |(_, n)| *n);
    pushes * (PUSH_GAS + PUSH_GAS / 63)
}

fn margin(estimated: u64) -> u64 {
    eravon_chain::send::limit_for(estimated)
}

fn settle(wait: Option<u64>, w: &Wire, sent: &a2send::Sent) -> R<Option<a2send::Receipt>> {
    let wait = wait.unwrap_or(WAIT_SECS);
    match wait {
        0 => Ok(None),
        n => waited(w, &sent.endpoint, &sent.tx_hash, n).map(Some),
    }
}

fn waited(w: &Wire, at: &eravon_chain::seam::NodeAt, tx_hash: &str, secs: u64) -> R<a2send::Receipt> {
    match a2send::await_receipt_within(&w.t, &w.table, at, tx_hash, Duration::from_secs(secs), TICK) {
        Ok(Some(r)) => Ok(r),
        Ok(None) => Err(Fault::TxNotYet { tx_hash: tx_hash.to_string(), waited: secs }),
        Err(f) => Err(unheard(tx_hash, f)),
    }
}


fn fees(o: &Opts, w: &Wire, head: u64) -> R<([u8; 32], [u8; 32], &'static str)> {
    match &o.fees {
        Some((m, p)) => Ok((word(m)?, word(p)?, "given")),
        None => {
            let f = eravon_chain::gas::suggested_fees_at(&w.t, &w.table, head)?;
            Ok((word(&f.cap)?, word(&f.tip)?, f.tip_from.word()))
        }
    }
}

pub fn quantity(w: &[u8; 32]) -> String {
    let mut body = String::with_capacity(64);
    for b in w {
        body.push_str(&format!("{b:02x}"));
    }
    let trimmed = body.trim_start_matches('0');
    format!("0x{}", if trimmed.is_empty() { "0" } else { trimmed })
}

pub fn word(s: &str) -> R<[u8; 32]> {
    match s.strip_prefix("0x") {
        Some(h) => {
            let b = a1::order::bytes_of("数", &format!("0x{h}"))?;
            if b.len() > 32 {
                return Err(Fault::Arg(format!("{s} 超过三十二字节")));
            }
            let mut w = [0u8; 32];
            w[32 - b.len()..].copy_from_slice(&b);
            Ok(w)
        }
        None => {
            let n: u128 =
                s.parse().map_err(|_| Fault::Arg(format!("要是十进制整数或 0x 十六进制:{s}")))?;
            let mut w = [0u8; 32];
            w[16..].copy_from_slice(&n.to_be_bytes());
            Ok(w)
        }
    }
}

impl Delivered {
    pub fn rows(&self) -> Vec<(&'static str, Out)> {
        let mut m = vec![
            ("endpoint", Out::S(self.endpoint.said())),
            ("feeSource", Out::S(self.fee_source.to_string())),
            ("gasLimit", Out::N(self.gas_limit)),
            ("nonce", Out::N(self.nonce)),
            ("sent", Out::B(true)),
            ("txHash", Out::S(self.tx_hash.clone())),
        ];
        if let Some(c) = &self.closing {
            m.push(("recordOpen", Out::S(c.clone())));
        }
        m.push((
            "receipt",
            match &self.receipt {
                Ok(None) => Out::O(vec![("waited", Out::B(false))]),
                Err(e) => Out::O(vec![
                    ("detail", Out::S(e.detail())),
                    ("token", Out::S(e.name().to_string())),
                    ("waited", Out::B(true)),
                ]),
                Ok(Some(r)) => Out::O(vec![
                    ("block", Out::N(r.block)),
                    ("gasUsed", Out::N(r.gas_used)),
                    ("status", Out::N(r.status)),
                    ("waited", Out::B(true)),
                ]),
            },
        ));
        m
    }
}

#[cfg(test)]
mod push_tests {
    use super::*;

    #[test]
    fn a_call_that_pushes_gets_room_for_each_push() {
        let call = |f: &a1::sol::Fun| a1::call::Call { to: String::new(), data: f.selector().to_vec() };
        let one = PUSH_GAS + PUSH_GAS / 63;
        assert_eq!(push_room(&call(&a1::sol::CONFIRM)), 3 * one);
        assert_eq!(push_room(&call(&a1::sol::ADVANCE)), 3 * one);
        assert_eq!(push_room(&call(&a1::sol::SWEEP)), 2 * one);
        assert_eq!(push_room(&call(&a1::sol::CANCEL)), one);
        assert_eq!(push_room(&call(&a1::sol::DISPUTE)), 3 * one);
        assert_eq!(push_room(&call(&a1::sol::DISPUTE_BY_SIG)), 3 * one);
        assert_eq!(push_room(&call(&a1::sol::CLAIM_DELIVERY)), 0);
        assert_eq!(push_room(&a1::call::Call { to: String::new(), data: vec![1] }), 0);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_fee_refusal_says_how_much_is_short() {
        let f = Fault::Precheck { gate: crate::fault::GATE_FEE.into(), why: super::fee_why("10", "25") };
        assert_eq!(super::fee_gap(&f.detail()).as_deref(), Some("15"));
        let g = Fault::Precheck { gate: "funds".into(), why: super::fee_why("10", "25") };
        assert_eq!(super::fee_gap(&g.detail()), None);
    }

    #[test]
    fn a_routed_draw_takes_the_law_route_gas_and_nothing_else() {
        let g = eravon_literals::constants::ROUTE_GAS;
        assert_eq!(route_gas_held(g).expect("法定"), g + g / 63);
        assert!(matches!(route_gas_held(g - 1), Err(Fault::Precheck { ref gate, .. }) if gate == "route"));
        assert!(matches!(route_gas_held(0), Err(Fault::Precheck { .. })));
    }

    use super::*;

    fn sent(tx: &str, status: Option<u64>) -> R<Delivered> {
        let receipt = match status {
            Some(s) => Ok(Some(a2send::Receipt { tx_hash: tx.into(), block: 1, status: s, gas_used: 1, contract: String::new() })),
            None => Err(Fault::Io("等收据超时".into())),
        };
        Ok(Delivered { tx_hash: tx.into(), endpoint: eravon_chain::seam::NodeAt::new(""), receipt, nonce: 0, gas_limit: 0, fee_source: "given", max_fee: "0".into(), priority_fee: "0".into(), closing: None })
    }

    #[test]
    fn a_series_stopped_part_way_names_what_is_on_chain_and_keeps_the_stopping_word() {
        let calls = [a1::call::Call { to: "0x01".into(), data: Vec::new() }, a1::call::Call { to: "0x02".into(), data: Vec::new() }, a1::call::Call { to: "0x03".into(), data: Vec::new() }];
        let run = |answers: Vec<R<Delivered>>| {
            let mut it = answers.into_iter();
            series(calls.iter(), |_| it.next().unwrap())
        };
        assert_eq!(run(vec![sent("0xa", Some(1)), sent("0xb", Some(1)), sent("0xc", Some(1))]), Ok(vec!["0xa".into(), "0xb".into(), "0xc".into()]));
        let e = run(vec![sent("0xa", Some(1)), sent("0xb", Some(0))]).unwrap_err();
        assert_eq!(e.name(), "E_V_REVERTED", "回滚照名,不改写成参数错");
        assert_eq!(e, Fault::Series { sent: vec![("0xa".into(), "mined"), ("0xb".into(), "reverted")], stop: Box::new(Fault::Reverted { tx_hash: "0xb".into() }) });
        let e = run(vec![sent("0xa", Some(1)), sent("0xb", None)]).unwrap_err();
        assert_eq!(e.name(), "E_V_UNCONFIRMED");
        let e = run(vec![sent("0xa", Some(1)), Err(Fault::Precheck { gate: "funds".into(), why: "不够".into() })]).unwrap_err();
        assert!(matches!(&e, Fault::Series { sent, .. } if sent == &vec![("0xa".to_string(), "mined")]), "第二笔发不出,第一笔照报:{e:?}");
        assert_eq!(run(vec![sent("0xa", Some(0))]).unwrap_err(), Fault::Reverted { tx_hash: "0xa".into() }, "第一笔就停,只是它自己的拒因");
    }

    #[test]
    fn a_replacement_rises_a_tenth_over_both_fields() {
        assert_eq!(replacing(("1000", "100"), ("0", "0")).unwrap(), ("1100".into(), "110".into()));
        assert_eq!(replacing(("1001", "101"), ("0", "0")).unwrap(), ("1102".into(), "112".into()), "向上取整");
        assert_eq!(replacing(("1", "0"), ("0", "0")).unwrap(), ("2".into(), "1".into()), "至少加一 wei");
        assert_eq!(replacing(("1000", "100"), ("5000", "300")).unwrap(), ("5000".into(), "300".into()), "现价更高取现价");
        assert_eq!(replacing(("1000", "1000"), ("0", "0")).unwrap(), ("1100".into(), "1100".into()));
        assert_eq!(replacing(("1000", "2000"), ("0", "0")).unwrap(), ("1100".into(), "1100".into()), "小费不过上限");
        assert!(replacing(("x", "1"), ("0", "0")).is_err());
    }

    #[test]
    fn the_gas_limit_is_one_and_a_half_times_the_estimate() {
        assert_eq!(margin(21_000), 31_500);
        assert_eq!(margin(3), 5);
        assert_eq!(margin(u64::MAX), eravon_chain::send::GAS_LIMIT);
    }

    #[test]
    fn the_balance_must_cover_the_most_the_transaction_can_spend() {
        let w = |n: u128| {
            let mut x = [0u8; 32];
            x[16..].copy_from_slice(&n.to_be_bytes());
            x
        };
        assert_eq!(most_spent(&w(3_000_000_000), 31_500, &w(0)), "94500000000000");
        assert_eq!(most_spent(&w(2), 3, &w(4)), "10");
        assert_eq!(most_spent(&[0xff; 32], u64::MAX, &[0xff; 32]).len(), 97, "不回绕");
        assert!(covers("10", "10") && covers("11", "10") && !covers("9", "10") && covers("100", "99") && !covers("0", "1"));
    }

    fn node() -> (String, std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>) {
        node_with(Vec::new())
    }

    fn node_with(over: Vec<(&'static str, &'static str)>) -> (String, std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>) {
        node_by(move |m, _| over.iter().find(|(k, _)| *k == m).map(|(_, v)| v.to_string()))
    }

    fn node_by(over: impl Fn(&str, usize) -> Option<String> + Send + Sync + 'static) -> (String, std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>) {
        use std::io::{Read, Write};
        let over = std::sync::Arc::new(over);
        let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = l.local_addr().expect("addr").port();
        let log = asked.clone();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let log = log.clone();
                let over = over.clone();
                std::thread::spawn(move || {
                    let mut s = s;
                    let mut seen = Vec::new();
                    let mut buf = [0u8; 4096];
                    let body = loop {
                        match s.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => seen.extend_from_slice(&buf[..n]),
                        }
                        let text = String::from_utf8_lossy(&seen).to_string();
                        if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                            let len = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                            if rest.len() >= len {
                                break rest[..len].to_string();
                            }
                        }
                    };
                    let id = body.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("1").to_string();
                    let method = body.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_string();
                    let params = body.split("\"params\":").nth(1).unwrap_or("").to_string();
                    let nth = {
                        let mut l = log.lock().unwrap();
                        let n = l.iter().filter(|(m, _)| *m == method).count();
                        l.push((method.clone(), params.clone()));
                        n
                    };
                    let ok = |v: &str| format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{v}}}");
                    let said = match (method.as_str(), over(&method, nth)) {
                        (_, Some(v)) => ok(&v),
                        ("eth_chainId", _) => ok("\"0x7a69\""),
                        ("eth_blockNumber", _) => ok("\"0x20\""),
                        ("eth_getTransactionCount", _) => ok("\"0x0\""),
                        ("eth_estimateGas", _) => ok("\"0x5208\""),
                        ("eth_getBlockByNumber", _) => ok("{\"number\":\"0x20\",\"baseFeePerGas\":\"0x1\"}"),
                        ("eth_feeHistory", _) => ok("{\"reward\":[[\"0x1\"]]}"),
                        ("eth_getBalance", _) if params.contains("\"0x20\"") => ok("\"0xffffffffffffffff\""),
                        ("eth_getBalance", _) => ok("\"0x0\""),
                        _ => format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32000,\"message\":\"insufficient funds for gas * price + value\"}}}}"),
                    };
                    let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{said}", said.len()).as_bytes());
                });
            }
        });
        (format!("http://127.0.0.1:{port}"), asked)
    }

    fn wire(at: &str) -> Wire {
        eravon_chain::patience::set_waits(Some(Duration::ZERO));
        let t = eravon_chain::transport::Auto::web_roots().expect("a transport").with_timeout(Duration::from_secs(5));
        Wire { t, table: eravon_chain::endpoints::Table::new(vec![at.to_string()]).expect("a table"), source: crate::home::Source::Inline, chain_id: 31337, logic: String::new(), registries: Vec::new() }
    }

    fn key() -> ([u8; 32], String) {
        let k = [0x11u8; 32];
        (k, shared::address_of(&k).expect("an address"))
    }

    fn call() -> a1::call::Call {
        a1::call::Call { to: "0x00000000000000000000000000000000000000d1".into(), data: vec![0xab] }
    }

    #[test]
    fn a_record_that_does_not_land_sends_not_a_byte() {
        let (at, asked) = node();
        let (k, who) = key();
        let o = Opts { wait: Some(0), ..Opts::default() };
        let got = deliver_key_landed(&|| Ok(k), &o, &wire(&at), &who, &call(), "0", &mut |_, _, _| Err(Fault::Io("盘满".into())));
        let e = got.err().expect("refused");
        assert_eq!(e.name(), "E_V_IO", "{e:?}");
        assert!(e.detail().contains("一个字节也没交出去") && e.detail().contains("盘满"), "{}", e.detail());
        assert!(asked.lock().unwrap().iter().all(|(m, _)| m != "eth_sendRawTransaction"), "nothing handed: {:?}", asked.lock().unwrap());
    }

    #[test]
    fn the_record_lands_before_the_bytes_are_handed() {
        let (at, asked) = node();
        let (k, who) = key();
        let o = Opts { wait: Some(0), ..Opts::default() };
        let mut when: Option<(usize, u64, String)> = None;
        let log = asked.clone();
        let got = deliver_key_landed(&|| Ok(k), &o, &wire(&at), &who, &call(), "0", &mut |n, tx, b| {
            when = Some((log.lock().unwrap().len(), n, tx.to_string()));
            assert_eq!(b.nonce, n);
            Ok(())
        });
        assert!(got.is_err(), "the node refuses the broadcast");
        let (landed_at, nonce, tx) = when.expect("landed");
        let asked = asked.lock().unwrap();
        let handed: Vec<usize> = asked.iter().enumerate().filter(|(_, (m, _))| m == "eth_sendRawTransaction").map(|(i, _)| i).collect();
        assert_eq!(handed.len(), 1, "{asked:?}");
        assert!(landed_at <= handed[0], "landed before the bytes went out: {landed_at} vs {handed:?}");
        assert_eq!(nonce, 0);
        assert!(tx.starts_with("0x") && tx.len() == 66, "{tx}");
    }

    #[test]
    fn fees_estimate_and_balance_are_asked_at_one_pinned_head() {
        let (at, asked) = node();
        let (k, who) = key();
        let o = Opts { wait: Some(0), ..Opts::default() };
        let got = deliver_key_landed(&|| Ok(k), &o, &wire(&at), &who, &call(), "0", &mut |_, _, _| Ok(()));
        let asked = asked.lock().unwrap();
        assert!(!matches!(&got, Err(Fault::Precheck { .. })), "the balance gate passed: {:?}", got.err());
        let at_of = |m: &str| asked.iter().filter(|(x, _)| x == m).map(|(_, p)| p.clone()).collect::<Vec<_>>();
        for m in ["eth_getBalance", "eth_estimateGas", "eth_getBlockByNumber", "eth_feeHistory"] {
            let ps = at_of(m);
            assert!(!ps.is_empty() && ps.iter().all(|p| p.contains("\"0x20\"") && !p.contains("latest")), "{m} at the pinned head: {ps:?}");
        }
        assert_eq!(at_of("eth_blockNumber").len(), 1, "the head is read once: {asked:?}");
        assert_eq!(at_of("eth_sendRawTransaction").len(), 1, "then handed once");
    }

    #[test]
    fn a_key_funded_for_a_call_is_sent_on_the_terms_it_was_funded_by() {
        let (at, asked) = node_by(|m, nth| match m {
            "eth_feeHistory" => Some(format!("{{\"reward\":[[\"0x{:x}\"]]}}", nth + 1)),
            "eth_getBalance" => Some("\"0x17124\"".to_string()),
            _ => None,
        });
        let (k, who) = key();
        let w = wire(&at);
        let o = Opts { wait: Some(0), ..Opts::default() };
        let (terms, most) = pinned(&o, &w, &who, &call()).expect("pinned");
        assert_eq!(most, "94500");
        assert_eq!((terms.gas_limit, terms.fees.clone(), terms.wait), (Some(31_500), Some(("3".to_string(), "1".to_string())), Some(0)));
        let afresh = deliver_key_landed(&|| Ok(k), &o, &w, &who, &call(), "0", &mut |_, _, _| Ok(()));
        assert!(matches!(&afresh, Err(Fault::Precheck { gate, .. }) if gate == "fee"), "read afresh, the cap rose past what was funded: {:?}", afresh.err());
        let before = asked.lock().unwrap().len();
        let on_terms = deliver_key_landed(&|| Ok(k), &terms, &w, &who, &call(), "0", &mut |_, _, b| {
            assert_eq!((b.max_fee.as_str(), b.priority_fee.as_str(), b.gas_limit), ("3", "1", 31_500));
            Ok(())
        });
        assert!(!matches!(&on_terms, Err(Fault::Precheck { .. })), "the balance gate asks what was funded: {:?}", on_terms.err());
        let asked = asked.lock().unwrap();
        let after: Vec<&str> = asked[before..].iter().map(|(m, _)| m.as_str()).collect();
        assert!(!after.iter().any(|m| ["eth_feeHistory", "eth_estimateGas", "eth_getBlockByNumber"].contains(m)), "the terms are not read again: {after:?}");
        assert_eq!(after.iter().filter(|m| **m == "eth_sendRawTransaction").count(), 1, "handed once: {after:?}");
    }

    #[test]
    fn a_direct_send_run_again_asks_its_record_first() {
        let (k, who) = key();
        let o = Opts { wait: Some(0), ..Opts::default() };
        let tx = [0xab; 32];
        let tx_hex = eravon::hexfmt::encode(&tx);
        let fresh = |tag: &str| {
            let d = std::env::temp_dir().join(format!("eravon-direct-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            let c = eravon_send::sent::Carried { max_fee: "3".into(), priority_fee: "1".into(), gas_limit: 21000, from: String::new(), to: String::new() };
            eravon_send::sent::land_carried(&d, &eravon_send::sent::Seal::of(&k, &d), 0, &tx, &c).unwrap();
            d
        };
        let sends = |a: &std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>| a.lock().unwrap().iter().filter(|(m, _)| m == "eth_sendRawTransaction").count();
        let d = fresh("reverted");
        let (at, asked) = node_with(vec![("eth_getTransactionReceipt", r#"{"blockNumber":"0x5","status":"0x0","gasUsed":"0x5208"}"#)]);
        let got = direct_at(&d, &|| Ok(k), &o, &wire(&at), &who, &call(), "0").expect("answered");
        assert!(matches!(&got.receipt, Ok(Some(r)) if r.status == 0) && got.tx_hash == tx_hex && sends(&asked) == 0, "said, nothing sent");
        assert!(eravon_send::sent::read(&d, &eravon_send::sent::Seal::of(&k, &d)).unwrap().is_empty(), "the reverted one done");
        let d = fresh("mined");
        let (at, asked) = node_with(vec![("eth_getTransactionReceipt", r#"{"blockNumber":"0x5","status":"0x1","gasUsed":"0x5208"}"#)]);
        let got = direct_at(&d, &|| Ok(k), &o, &wire(&at), &who, &call(), "0").expect("answered");
        assert!(matches!(&got.receipt, Ok(Some(r)) if r.status == 1 && r.block == 5) && got.tx_hash == tx_hex && sends(&asked) == 0);
        assert!(eravon_send::sent::read(&d, &eravon_send::sent::Seal::of(&k, &d)).unwrap().is_empty(), "answered: marked done, so the same command later is a send of its own");
        let d = fresh("held");
        let (at, asked) = node_with(vec![("eth_getTransactionReceipt", "null"), ("eth_getTransactionByHash", r#"{"hash":"0xab"}"#)]);
        let got = direct_at(&d, &|| Ok(k), &o, &wire(&at), &who, &call(), "0").expect("answered");
        assert!(matches!(got.receipt, Ok(None)) && got.tx_hash == tx_hex && sends(&asked) == 0);
        let d = fresh("again");
        let (at, asked) = node_with(vec![("eth_getTransactionReceipt", "null"), ("eth_getTransactionByHash", "null"), ("eth_getTransactionCount", "\"0x0\"")]);
        let _ = direct_at(&d, &|| Ok(k), &o, &wire(&at), &who, &call(), "0");
        assert_eq!(sends(&asked), 1, "signed again and handed");
        let recs = eravon_send::sent::read(&d, &eravon_send::sent::Seal::of(&k, &d)).unwrap();
        assert_eq!((recs.len(), recs[0], recs[1].0, recs[1].1 != tx), (2, (0, tx), 0, true), "both asked after, at the same nonce: {recs:?}");
        let d = fresh("void");
        let (at, asked) = node_with(vec![("eth_getTransactionReceipt", "null"), ("eth_getTransactionByHash", "null"), ("eth_getTransactionCount", "\"0x1\"")]);
        let e = direct_at(&d, &|| Ok(k), &o, &wire(&at), &who, &call(), "0").err().expect("refused");
        assert_eq!((e.name(), sends(&asked)), ("E_V_SENT_VOID", 0), "{e:?}");
        assert!(!eravon_literals::schema::UNANSWERED.contains(&e.name()), "a refusal, exit 1");
        assert!(eravon_send::sent::read(&d, &eravon_send::sent::Seal::of(&k, &d)).unwrap().is_empty(), "marked void: the next run sends afresh");
        for tag in ["reverted", "mined", "held", "again", "void"] {
            let _ = std::fs::remove_dir_all(std::env::temp_dir().join(format!("eravon-direct-{tag}-{}", std::process::id())));
        }
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    const PIN: &str = "48261537";
    const PHRASE: &str = "test test test test test test test test test test test junk";

    #[test]
    fn a_send_asks_whether_the_session_is_open_not_for_a_passcode_flag() {
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create(PIN, eravon_store::vault::Seat::Buyer, "主钥", Some(PHRASE), 1_790_000_000).expect("建得出库");
        v.write(&data).expect("落得下盘");
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let table = eravon_chain::endpoints::Table::new(vec!["http://127.0.0.1:1".to_string()]).expect("table");
        let _ = table.chain.set(31337);
        let w = Wire { t: eravon_chain::transport::Auto::web_roots().expect("roots"), table, source: crate::home::Source::Inline, chain_id: 31337, logic: String::new(), registries: Vec::new() };
        let f = crate::shared::Flags::read(&[]).expect("no flags");
        let call = a1::call::Call { to: "0x00000000000000000000000000000000000000bb".into(), data: Vec::new() };
        let who = "0x00000000000000000000000000000000000000aa";
        let open = crate::home::open_ctx(data.clone(), 1_790_000_000, "t".into(), eravon_store::vault::Seat::Buyer, Some(PIN));
        match deliver_with(&open, &f, &w, who, &call, "0", None) {
            Err(Fault::Arg(m)) => panic!("an open session asks for no passcode flag: {m}"),
            Err(e) => assert!(!matches!(&e, Fault::Down { token, .. } if token == "E_VAULT_LOCKED"), "open: it went on to the chain: {e:?}"),
            Ok(_) => panic!("the nodes are dead"),
        }
        let locked = crate::home::open_ctx(data, 1_790_000_000, "t".into(), eravon_store::vault::Seat::Buyer, None);
        match deliver_with(&locked, &f, &w, who, &call, "0", None) {
            Err(e) => assert!(matches!(&e, Fault::Down { token, .. } if token == "E_VAULT_LOCKED"), "locked: refused before the chain is asked: {e:?}"),
            Ok(_) => panic!("a locked session sends nothing"),
        }
    }
}
