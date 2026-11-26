use crate::rpc::{Broke, Endpoint, Trouble};
use crate::tx;
use eravon::hexfmt;
use eravon::json::Value;

pub const PRIORITY_FEE: u64 = 1_000_000_000;
pub const MAX_FEE: u64 = 3_000_000_000;
pub const GAS_LIMIT: u64 = 16_777_216;

pub const PLAIN_GAS_MAX: u64 = u64::MAX / 3 * 2;

pub fn limit_for(estimate: u64) -> u64 {
    estimate.saturating_mul(3).div_ceil(2).min(GAS_LIMIT)
}

pub fn plain_limit_for(estimate: u64) -> u64 {
    u64::try_from((estimate as u128 * 3).div_ceil(2)).unwrap_or(u64::MAX)
}
#[derive(Clone, Debug, PartialEq)]
pub enum NoGas<E> {
    Network(E),
    Refused(E),
    NotText(Value),
    Unreadable(String),
    OverCap(u128),
}

pub fn estimate_gas<E>(
    head: impl FnOnce() -> Result<u64, E>,
    ask: impl FnOnce(&Value) -> Result<Value, E>,
    call: Value,
    is_network: impl Fn(&E) -> bool,
) -> Result<u64, NoGas<E>> {
    crate::seam();
    let v = head()
        .and_then(|height| ask(&Value::Arr(vec![call, Value::Str(format!("0x{height:x}"))])))
        .map_err(|e| if is_network(&e) { NoGas::Network(e) } else { NoGas::Refused(e) })?;
    let hex = match &v {
        Value::Str(s) => s.clone(),
        other => return Err(NoGas::NotText(other.clone())),
    };
    let n = quantity_128(&hex).ok_or_else(|| NoGas::Unreadable(hex.clone()))?;
    if n > GAS_LIMIT as u128 {
        return Err(NoGas::OverCap(n));
    }
    Ok(n as u64)
}

pub fn estimate_plain_gas<E>(
    head: impl FnOnce() -> Result<u64, E>,
    ask: impl FnOnce(&Value) -> Result<Value, E>,
    call: Value,
    is_network: impl Fn(&E) -> bool,
) -> Result<u64, NoGas<E>> {
    match estimate_gas(head, ask, call, is_network) {
        Err(NoGas::OverCap(n)) if n <= PLAIN_GAS_MAX as u128 => Ok(n as u64),
        other => other,
    }
}

fn quantity_128(x: &str) -> Option<u128> {
    let body = x.strip_prefix("0x")?;
    if body.is_empty() || body.len() > 32 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(body, 16).ok()
}

pub const REPLACE_BUMP_PERCENT: u64 = 10;

pub const TIP_BLOCKS: u64 = 20;
pub const TIP_PERCENTILE: u64 = 50;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fees {
    pub max_fee: u64,
    pub priority: u64,
    pub gas_limit: u64,
    pub from_chain: bool,
}

impl Fees {
    pub fn fallback() -> Fees {
        Fees { max_fee: MAX_FEE, priority: PRIORITY_FEE, gas_limit: GAS_LIMIT, from_chain: false }
    }

    pub fn of(base_fee: Option<u64>, tip: Option<u64>) -> Fees {
        let Some(base) = base_fee else { return Fees::fallback() };
        let priority = tip.map(|t| t.min(PRIORITY_FEE)).unwrap_or(PRIORITY_FEE);
        Fees { max_fee: base.saturating_mul(2).saturating_add(priority), priority, gas_limit: GAS_LIMIT, from_chain: true }
    }

    pub fn replacing(old: Fees, now: Fees) -> Fees {
        let up = |x: u64| x.saturating_mul(100 + REPLACE_BUMP_PERCENT).div_ceil(100).max(x.saturating_add(1));
        let max_fee = now.max_fee.max(up(old.max_fee));
        let priority = now.priority.max(up(old.priority)).min(max_fee);
        Fees { max_fee, priority, gas_limit: old.gas_limit, from_chain: now.from_chain }
    }

    pub fn with_estimate(self, estimate: u64) -> Fees {
        Fees { gas_limit: limit_for(estimate), ..self }
    }

    pub fn with_plain_estimate(self, estimate: u64) -> Fees {
        Fees { gas_limit: plain_limit_for(estimate), ..self }
    }

    pub fn cap_wei(self) -> u128 {
        self.max_fee as u128 * self.gas_limit as u128
    }
}

pub fn tip_params(block: u64) -> Value {
    Value::Arr(vec![
        Value::Str(format!("0x{TIP_BLOCKS:x}")),
        Value::Str(format!("0x{block:x}")),
        Value::Arr(vec![Value::Int(TIP_PERCENTILE)]),
    ])
}

pub fn tip_of(history: &Value) -> Option<u64> {
    let Some(Value::Arr(blocks)) = history.member("reward") else { return None };
    let mut tips: Vec<u64> = Vec::with_capacity(blocks.len());
    for b in blocks {
        let Value::Arr(cells) = b else { return None };
        let [Value::Str(one)] = cells.as_slice() else { return None };
        tips.push(quantity(one)?);
    }
    if tips.is_empty() {
        return None;
    }
    let paid: Vec<u64> = tips.iter().copied().filter(|t| *t != 0).collect();
    let mut over = if paid.is_empty() { tips } else { paid };
    over.sort_unstable();
    Some(over[(over.len() - 1) / 2])
}

pub fn base_fee_of(block: &Value) -> Option<u64> {
    let body = block.member("baseFeePerGas")?.as_str()?.strip_prefix("0x")?;
    if body.is_empty() || body.len() > 32 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(body, 16).ok().and_then(|n| u64::try_from(n).ok())
}

pub(crate) fn quantity(x: &str) -> Option<u64> {
    let body = x.strip_prefix("0x")?;
    if body.is_empty() || body.len() > 16 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(body, 16).ok()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Confirm {
    Included { status: u64, block_number: u64 },
    NotYet,
    Unreachable(String),
    Split { has: Vec<String>, not_yet: Vec<String> },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unheld {
    Void,
    Unused,
    Wait,
}

pub fn unheld(on_chain: Option<u64>, mine: u64, none_included: impl FnOnce() -> bool) -> Unheld {
    match on_chain {
        None => Unheld::Wait,
        Some(n) if n > mine => match none_included() {
            true => Unheld::Void,
            false => Unheld::Wait,
        },
        Some(_) => Unheld::Unused,
    }
}

pub fn submit(ep: &mut dyn Endpoint, raw: &[u8], hash: &[u8; 32]) -> Result<(), Trouble> {
    crate::seam();
    let sent = ep.call("eth_sendRawTransaction", &Value::Arr(vec![Value::Str(hexfmt::encode(raw))]))?;
    let echoed = sent.as_str().and_then(hexfmt::decode).unwrap_or_default();
    if echoed != hash {
        return Err(Trouble::Transport(Broke::EchoNotOurs));
    }
    Ok(())
}

pub const RECEIPT_BACKOFF: [std::time::Duration; 4] = [
    std::time::Duration::from_millis(250),
    std::time::Duration::from_millis(500),
    std::time::Duration::from_millis(1000),
    std::time::Duration::from_millis(2000),
];

pub fn confirm_each(eps: &mut [&mut dyn Endpoint], hash: &[u8; 32], wait: std::time::Duration, backoff: &[std::time::Duration]) -> Confirm {
    if eps.is_empty() {
        return Confirm::Unreachable(Broke::NoEndpoint.evidence());
    }
    let deadline = std::time::Instant::now() + wait;
    let q = Value::Arr(vec![Value::Str(hexfmt::encode(hash))]);
    let mut last: Option<String> = None;
    let mut ever_answered = false;
    let mut round = 0usize;
    let split_pauses = crate::judge::not_yet_pauses();
    let mut split_rounds = 0usize;
    let mut split: Option<(Vec<String>, Vec<String>)> = None;
    let mut seen = false;
    let mut base: Option<u64> = None;
    let mut watched = false;
    let mut ask_receipt = true;
    loop {
        let mut has: Vec<(String, (u64, u64))> = Vec::new();
        let mut has_names: Vec<String> = Vec::new();
        let mut not_yet: Vec<String> = Vec::new();
        let mut cut = false;
        let mut first_not_yet = false;
        for (at, ep) in eps.iter_mut().enumerate() {
            if !ask_receipt {
                break;
            }
            if at == 1 && !wait.is_zero() && !seen && has.is_empty() && not_yet.len() == 1 {
                break;
            }
            let asked = if wait.is_zero() {
                ep.call("eth_getTransactionReceipt", &q)
            } else {
                let left = deadline.saturating_duration_since(std::time::Instant::now());
                if left.is_zero() {
                    cut = true;
                    break;
                }
                let left = left.min(ONE_ASK);
                ep.call_within("eth_getTransactionReceipt", &q, left)
            };
            match asked {
                Ok(r) if !r.is_null() => match (tx::qty_u64(&r, "status"), tx::qty_u64(&r, "blockNumber")) {
                    (Some(status), Some(block_number)) => {
                        ever_answered = true;
                        if !seen {
                            round = 0;
                        }
                        seen = true;
                        has.push((ep.place(), (status, block_number)));
                        has_names.push(ep.name());
                    }
                    _ => {
                        ever_answered = true;
                        last = Some(Broke::Unread { method: "eth_getTransactionReceipt".into() }.evidence());
                    }
                },
                Ok(_) => {
                    ever_answered = true;
                    first_not_yet |= at == 0;
                    not_yet.push(ep.name());
                }
                Err(e) => last = Some(format!("{e:?}")),
            }
        }
        if !cut {
            split = None;
        }
        let none_had = has.is_empty();
        if !has.is_empty() && !cut {
            match taken(&has, not_yet.len()) {
                Taken::Counts((status, block_number)) => return Confirm::Included { status, block_number },
                Taken::Split if wait.is_zero() => match split_pauses.get(split_rounds) {
                    Some(pause) => {
                        split_rounds += 1;
                        if !pause.is_zero() {
                            std::thread::sleep(*pause);
                        }
                        continue;
                    }
                    None => return Confirm::Split { has: has_names, not_yet },
                },
                Taken::Split => split = Some((has_names, not_yet)),
                Taken::Differ => {}
            }
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            if let Some((has, not_yet)) = split {
                return Confirm::Split { has, not_yet };
            }
            return match ever_answered {
                true => Confirm::NotYet,
                false => Confirm::Unreachable(last.unwrap_or_else(|| Broke::NoEndpoint.evidence())),
            };
        }
        let pause = backoff.get(round.min(backoff.len().saturating_sub(1))).map_or(std::time::Duration::ZERO, |p| (*p).min(deadline - now));
        round += 1;
        if !watched && !wait.is_zero() && !seen && none_had && first_not_yet && !cut {
            watched = true;
            base = crate::action::head_at_broadcast(hash);
            ask_receipt = base.is_none();
        }
        if !ask_receipt {
            let head = crate::action::after_a_beat(pause, || head_within(&mut *eps[0], deadline));
            match (head, base) {
                (Some(h), Some(b)) if h <= b => continue,
                _ => {
                    base = None;
                    ask_receipt = true;
                    round = 0;
                }
            }
            continue;
        }
        if !pause.is_zero() {
            std::thread::sleep(pause);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taken {
    Counts((u64, u64)),
    Split,
    Differ,
}

pub fn taken(has: &[(String, (u64, u64))], not_yet: usize) -> Taken {
    let mut groups: Vec<((u64, u64), usize)> = Vec::new();
    for (_, r) in has {
        if !groups.iter().any(|(g, _)| g == r) {
            groups.push((*r, crate::endpoints::distinct_places(has.iter().filter(|(_, x)| x == r).map(|(n, _)| n.as_str()))));
        }
    }
    let two: Vec<(u64, u64)> = groups.iter().filter(|(_, n)| *n >= 2).map(|(g, _)| *g).collect();
    match (two.as_slice(), groups.as_slice()) {
        ([one], _) => Taken::Counts(*one),
        ([], [(one, n)]) if counts(*n, not_yet) => Taken::Counts(*one),
        ([], [_]) => Taken::Split,
        _ => Taken::Differ,
    }
}

pub fn counts(given: usize, not_yet: usize) -> bool {
    given >= 2 || (given == 1 && not_yet == 0)
}

fn head_within(ep: &mut dyn Endpoint, deadline: std::time::Instant) -> Option<u64> {
    let left = deadline.saturating_duration_since(std::time::Instant::now());
    if left.is_zero() {
        return None;
    }
    let left = left.min(ONE_ASK);
    ep.call_within("eth_blockNumber", &Value::Arr(Vec::new()), left).ok().as_ref().and_then(crate::action::height)
}

pub const ONE_ASK: std::time::Duration = std::time::Duration::from_secs(5);

#[cfg(test)]
mod fee_tests {
    use super::*;

    fn history(tips: &[u64]) -> Value {
        Value::Obj(vec![(
            "reward".into(),
            Value::Arr(tips.iter().map(|t| Value::Arr(vec![Value::Str(format!("0x{t:x}"))])).collect()),
        )])
    }

    #[test]
    fn the_fee_rule_reads_the_chain_and_holds_its_ceiling() {
        assert_eq!(tip_of(&history(&[5, 1, 4, 2, 3])), Some(3));
        assert_eq!(tip_of(&history(&[4, 1, 3, 2])), Some(2));
        assert_eq!(tip_of(&history(&[])), None);
        assert_eq!(tip_of(&Value::Null), None);
        assert_eq!(tip_of(&Value::Obj(vec![("reward".into(), Value::Arr(vec![Value::Arr(vec![Value::Str("0xzz".into())])]))])), None);
        assert_eq!(Fees::of(Some(10), Some(3)), Fees { max_fee: 23, priority: 3, gas_limit: GAS_LIMIT, from_chain: true });
        assert_eq!(Fees::of(Some(10), Some(PRIORITY_FEE * 3)).priority, PRIORITY_FEE);
        assert_eq!(Fees::of(Some(10), None).priority, PRIORITY_FEE);
        assert_eq!(Fees::of(Some(0), Some(0)), Fees { max_fee: 0, priority: 0, gas_limit: GAS_LIMIT, from_chain: true });
        assert_eq!(Fees::of(Some(u64::MAX), Some(1)).max_fee, u64::MAX);
        assert_eq!(Fees::of(None, Some(1)), Fees::fallback());
        assert!(!Fees::fallback().from_chain);
        let even = Fees::of(Some(PRIORITY_FEE), Some(PRIORITY_FEE * 5));
        assert_eq!((even.max_fee, even.priority), (MAX_FEE, PRIORITY_FEE));
        assert!(even.from_chain);
    }

    #[test]
    fn blocks_that_paid_no_priority_fee_are_not_counted() {
        assert_eq!(tip_of(&history(&[0, 5, 0, 1, 3])), Some(3), "three paid blocks: their median");
        assert_eq!(tip_of(&history(&[0, 4, 0, 2])), Some(2), "two paid blocks: the lower middle");
        assert_eq!(tip_of(&history(&[0, 0, 0, 7, 0])), Some(7), "one paid block among empty ones");
        assert_eq!(tip_of(&history(&[0, 0, 0])), Some(0), "every block empty: zero");
        assert_eq!(tip_of(&history(&[0])), Some(0));
        assert_eq!(tip_of(&history(&[])), None);
        let half_read = Value::Obj(vec![("reward".into(), Value::Arr(vec![Value::Arr(vec![Value::Str("0x0".into())]), Value::Arr(vec![Value::Str("0xzz".into())])]))]);
        assert_eq!(tip_of(&half_read), None, "an unreadable entry is unread, zeros or not");
        assert_eq!(Fees::of(Some(10), tip_of(&history(&[0, 0, 0]))).priority, 0, "all zero: no priority fee");
    }

    #[test]
    fn the_gas_limit_follows_the_estimate() {
        assert_eq!(limit_for(0), 0);
        assert_eq!(limit_for(1), 2);
        assert_eq!(limit_for(21_000), 31_500);
        assert_eq!(limit_for(25_001), 37_502);
        assert_eq!(limit_for(11_184_810), 16_777_215);
        assert_eq!(limit_for(11_184_811), 16_777_216);
        assert_eq!(limit_for(GAS_LIMIT), GAS_LIMIT);
        assert_eq!(limit_for(u64::MAX), GAS_LIMIT);
        let f = Fees::of(Some(10), Some(3)).with_estimate(30_000);
        assert_eq!((f.max_fee, f.priority, f.gas_limit), (23, 3, 45_000));
        assert_eq!(f.cap_wei(), 23 * 45_000);
        assert_eq!(Fees::fallback().gas_limit, GAS_LIMIT);
        assert_eq!(Fees::fallback().cap_wei(), MAX_FEE as u128 * GAS_LIMIT as u128);
    }

    #[test]
    fn a_base_fee_is_read_one_way() {
        let b = |x: &str| Value::Obj(vec![("baseFeePerGas".into(), Value::Str(x.into()))]);
        assert_eq!(base_fee_of(&b("0x3b9aca00")), Some(1_000_000_000));
        assert_eq!(base_fee_of(&b("0x00000000000000000000000000000001")), Some(1));
        assert_eq!(base_fee_of(&b("0xffffffffffffffff")), Some(u64::MAX));
        assert_eq!(base_fee_of(&b("0x8000000000000000")), Some(1 << 63));
        for bad in ["0x10000000000000000", "0x", "1", "0xzz", "0x000000000000000000000000000000001"] {
            assert_eq!(base_fee_of(&b(bad)), None, "{bad}");
        }
        assert_eq!(base_fee_of(&Value::Obj(Vec::new())), None);
        assert_eq!(Fees::of(base_fee_of(&b("0x8000000000000000")), Some(1)).max_fee, u64::MAX, "doubled saturates");
    }
}

#[cfg(test)]
mod wait_tests {
    use super::*;
    use crate::wire::{Body, W};

    struct Waited {
        name: &'static str,
        heads: Vec<Option<u64>>,
        receipts: Vec<Result<Option<u64>, ()>>,
        asked: Vec<&'static str>,
        at: Vec<std::time::Instant>,
    }

    impl Waited {
        fn new(name: &'static str, heads: Vec<Option<u64>>, receipts: Vec<Result<Option<u64>, ()>>) -> Waited {
            Waited { name, heads, receipts, asked: Vec::new(), at: Vec::new() }
        }
    }

    fn nth<T: Clone>(xs: &[T], n: usize) -> T {
        xs.get(n).or(xs.last()).cloned().expect("a scene")
    }

    impl Endpoint for Waited {
        fn call(&mut self, method: &str, _: &Value) -> Result<W, Trouble> {
            let refused = || Trouble::Node("{\"code\":-32000,\"message\":\"unavailable\"}".into());
            match method {
                "eth_blockNumber" => {
                    let n = self.asked.iter().filter(|m| **m == "head").count();
                    self.asked.push("head");
                    nth(&self.heads, n).map(|h| W::of(Body::Str(format!("0x{h:x}")))).ok_or_else(refused)
                }
                "eth_getTransactionReceipt" => {
                    let n = self.asked.iter().filter(|m| **m == "receipt").count();
                    self.asked.push("receipt");
                    self.at.push(std::time::Instant::now());
                    match nth(&self.receipts, n) {
                        Ok(Some(b)) => Ok(crate::wire::parse(format!("{{\"blockHash\":\"0x{}\",\"blockNumber\":\"0x{b:x}\",\"status\":\"0x1\"}}", "ab".repeat(32)).as_bytes()).expect("json")),
                        Ok(None) => Ok(W::of(Body::Null)),
                        Err(()) => Err(refused()),
                    }
                }
                _ => Ok(W::of(Body::Null)),
            }
        }
        fn name(&self) -> String {
            self.name.into()
        }
        fn place(&self) -> String {
            self.name.into()
        }
    }

    const LONG: std::time::Duration = std::time::Duration::from_secs(30);

    fn broadcast_at_0x10() -> crate::action::Opened {
        let a = crate::action::open(None);
        crate::action::note("held", "eth_blockNumber", &Value::Arr(Vec::new()), &W::of(Body::Str("0x10".into())));
        crate::action::sent("held", Some(&[1; 32]));
        a
    }

    #[test]
    fn a_wait_asks_the_head_until_it_moves_and_then_the_receipt() {
        let mut node = Waited::new("resumed", vec![Some(0x10)], vec![Ok(None), Ok(Some(0x10))]);
        let got = confirm_each(&mut [&mut node], &[1; 32], LONG, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x10 });
        assert_eq!(node.asked, ["receipt", "receipt"], "no head the broadcast went out on: the receipt each beat");
        let _action = broadcast_at_0x10();
        let mut node = Waited::new("taker", vec![Some(0x10), Some(0x10), Some(0x10), Some(0x11)], vec![Ok(None), Ok(Some(0x11))]);
        let got = confirm_each(&mut [&mut node], &[1; 32], LONG, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x11 });
        assert_eq!(node.asked, ["receipt", "head", "head", "head", "head", "receipt"], "the head until it moved, then the receipt");
        let mut node = Waited::new("taker", vec![None], vec![Ok(None), Ok(None), Ok(Some(0x10))]);
        let got = confirm_each(&mut [&mut node], &[1; 32], LONG, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x10 });
        assert_eq!(node.asked, ["receipt", "head", "receipt", "receipt"], "a head that does not read: the receipt each beat");
        let mut node = Waited::new("taker", vec![Some(0x10), Some(0x11)], vec![Ok(None), Ok(None), Ok(Some(0x12))]);
        let got = confirm_each(&mut [&mut node], &[1; 32], LONG, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x12 });
        assert_eq!(node.asked, ["receipt", "head", "head", "receipt", "receipt"], "past the head it started on, the receipt each beat, its head watched no more");
        let mut node = Waited::new("late", vec![Some(0x11), Some(0x12)], vec![Ok(None), Ok(Some(0x11))]);
        let got = confirm_each(&mut [&mut node], &[1; 32], LONG, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x11 });
        assert_eq!(node.asked, ["receipt", "head", "receipt"], "the head already past the broadcast's: the receipt next");
    }

    #[test]
    fn a_wait_on_the_lower_of_two_chains_watches_that_chains_head() {
        let none = Value::Arr(Vec::new());
        let raw = [0x02u8, 0xf8, 0x6b, 0x01];
        let tx = eravon::cryptox::keccak256(&raw);
        let broadcast = Value::Arr(vec![Value::Str(hexfmt::encode(&raw))]);
        let wait = std::time::Duration::from_secs(5);
        {
            let _action = crate::action::open(None);
            let mut high = crate::endpoints::remembering("high", Box::new(Waited::new("high", vec![Some(0x1000)], vec![Ok(None)])));
            let mut low = crate::endpoints::remembering("low", Box::new(Waited::new("low", vec![Some(0x10)], vec![Ok(None)])));
            high.call("eth_blockNumber", &none).expect("the higher chain's head");
            low.call("eth_blockNumber", &none).expect("the lower chain's head");
            let _ = low.call(crate::rpc::BROADCAST, &broadcast);
            assert_eq!(crate::action::head_at_broadcast(&tx), Some(0x10), "the lower chain's own head");
            let mut node = Waited::new("low", vec![Some(0x10), Some(0x11)], vec![Ok(None), Ok(Some(0x11))]);
            let started = std::time::Instant::now();
            let got = confirm_each(&mut [&mut node], &tx, wait, &[]);
            assert_eq!(got, Confirm::Included { status: 1, block_number: 0x11 }, "read, never \"not yet\" at the deadline");
            assert!(started.elapsed() < wait / 2, "in a moment: {:?}", started.elapsed());
            assert_eq!(node.asked, ["receipt", "head", "head", "receipt"], "that chain's head until it passed 0x10, then the receipt");
        }
        let _action = crate::action::open(None);
        let mut high = crate::endpoints::remembering("high", Box::new(Waited::new("high", vec![Some(0x1000)], vec![Ok(None)])));
        let mut a = crate::endpoints::remembering("low-a", Box::new(Waited::new("low-a", vec![Some(0x12)], vec![Ok(None)])));
        let mut b = crate::endpoints::remembering("low-b", Box::new(Waited::new("low-b", vec![Some(0x11)], vec![Ok(None)])));
        for n in [&mut high, &mut a, &mut b] {
            n.call("eth_blockNumber", &none).expect("a head");
        }
        let _ = a.call(crate::rpc::BROADCAST, &broadcast);
        let _ = b.call(crate::rpc::BROADCAST, &broadcast);
        assert_eq!(crate::action::head_at_broadcast(&tx), Some(0x12), "kept by the node that went first, unset by none");
        let mut node = Waited::new("low-a", vec![Some(0x12), Some(0x13)], vec![Ok(None), Ok(Some(0x13))]);
        let got = confirm_each(&mut [&mut node], &tx, wait, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x13 });
        assert_eq!(node.asked, ["receipt", "head", "head", "receipt"], "the head watched past 0x12, then the receipt");
    }

    #[test]
    fn at_the_deadline_not_yet_only_where_a_node_answered_the_receipt_question() {
        let beat = [std::time::Duration::from_millis(10)];
        let wait = std::time::Duration::from_millis(100);
        let _action = broadcast_at_0x10();
        let mut still = Waited::new("taker", vec![Some(0x10)], vec![Ok(None)]);
        assert_eq!(confirm_each(&mut [&mut still], &[1; 32], wait, &beat), Confirm::NotYet);
        assert_eq!(still.asked.iter().filter(|m| **m == "receipt").count(), 1, "the head stood still: the receipt asked once");
        let mut dark = Waited::new("taker", vec![Some(0x10)], vec![Err(())]);
        assert!(matches!(confirm_each(&mut [&mut dark], &[1; 32], wait, &beat), Confirm::Unreachable(_)));
        assert!(!dark.asked.contains(&"head"), "no head asked: {:?}", dark.asked);
    }

    #[test]
    fn once_one_node_gives_the_receipt_the_beats_start_again() {
        let long = std::time::Duration::from_millis(1500);
        let beats = [std::time::Duration::ZERO, std::time::Duration::ZERO, long, long];
        let mut first = Waited::new("first", vec![None], vec![Ok(None), Ok(None), Ok(Some(0x10))]);
        let mut behind = Waited::new("behind", vec![Some(0x10)], vec![Ok(None), Ok(Some(0x10))]);
        let got = confirm_each(&mut [&mut first, &mut behind], &[1; 32], LONG, &beats);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x10 });
        let gap = behind.at[1].duration_since(behind.at[0]);
        assert!(gap < long / 2, "the node behind asked again after the shortest beat: {gap:?}");
    }

    #[test]
    fn two_nodes_giving_the_same_receipt_count_and_the_one_behind_is_not_waited_for() {
        let mut a = Waited::new("a", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        let mut b = Waited::new("b", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        let mut behind = Waited::new("behind", vec![Some(0x0e)], vec![Ok(None)]);
        let got = confirm_each(&mut [&mut a, &mut b, &mut behind], &[1; 32], LONG, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x10 });
        assert_eq!(behind.asked.iter().filter(|m| **m == "receipt").count(), 1, "asked once, never waited for");
        let (mut a, mut b, mut behind) = (Waited::new("a", vec![], vec![Ok(Some(0x10))]), Waited::new("b", vec![], vec![Ok(Some(0x10))]), Waited::new("behind", vec![], vec![Ok(None)]));
        assert_eq!(confirm_each(&mut [&mut a, &mut b, &mut behind], &[1; 32], std::time::Duration::ZERO, &[]), Confirm::Included { status: 1, block_number: 0x10 }, "one look: two alike count");
    }

    #[test]
    fn one_nodes_receipt_alone_does_not_count_and_tables_of_two_and_one_are_as_before() {
        let wait = std::time::Duration::from_millis(60);
        let mut a = Waited::new("a", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        let mut b = Waited::new("b", vec![Some(0x0e)], vec![Ok(None)]);
        let mut c = Waited::new("c", vec![Some(0x0e)], vec![Ok(None)]);
        assert_eq!(confirm_each(&mut [&mut a, &mut b, &mut c], &[1; 32], wait, &[]), Confirm::Split { has: vec!["a".into()], not_yet: vec!["b".into(), "c".into()] });
        let mut a = Waited::new("a", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        let mut b = Waited::new("b", vec![Some(0x0e)], vec![Ok(None)]);
        assert_eq!(confirm_each(&mut [&mut a, &mut b], &[1; 32], wait, &[]), Confirm::Split { has: vec!["a".into()], not_yet: vec!["b".into()] }, "a table of two waits for both");
        let mut one = Waited::new("one", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        assert_eq!(confirm_each(&mut [&mut one], &[1; 32], wait, &[]), Confirm::Included { status: 1, block_number: 0x10 }, "a table of one: its node's word");
    }

    #[test]
    fn two_alike_receipts_count_beside_another_and_two_unlike_do_not() {
        let wait = std::time::Duration::from_millis(60);
        let mut a = Waited::new("a", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        let mut b = Waited::new("b", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        let mut other = Waited::new("other", vec![Some(0x12)], vec![Ok(Some(0x11))]);
        let got = confirm_each(&mut [&mut a, &mut b, &mut other], &[1; 32], wait, &[]);
        assert_eq!(got, Confirm::Included { status: 1, block_number: 0x10 }, "two alike count beside a third that differs");
        let mut a = Waited::new("a", vec![Some(0x12)], vec![Ok(Some(0x10))]);
        let mut other = Waited::new("other", vec![Some(0x12)], vec![Ok(Some(0x11))]);
        let got = confirm_each(&mut [&mut a, &mut other], &[1; 32], wait, &[]);
        assert!(!matches!(got, Confirm::Included { .. }), "a table of two whose receipts differ: {got:?}");
        assert_eq!(taken(&[("a".into(), (1, 0x10)), ("b".into(), (1, 0x11)), ("c".into(), (1, 0x10)), ("d".into(), (1, 0x11))], 0), Taken::Differ, "two against two");
        assert_eq!(taken(&[("a".into(), (1, 0x10))], 1), Taken::Split, "one beside a not yet: asked again");
        assert_eq!(taken(&[("a".into(), (1, 0x10))], 0), Taken::Counts((1, 0x10)), "one alone: counts");
    }

    #[test]
    fn one_place_written_twice_is_one_place_for_a_receipt() {
        let twice = |r: (u64, u64)| -> Vec<(String, (u64, u64))> { vec![("http://127.0.0.1:8545".into(), r), ("http://127.0.0.1:8545".into(), r)] };
        assert_eq!(taken(&twice((1, 0x10)), 1), Taken::Split, "one place twice beside a not yet");
        let mut three = twice((1, 0x10));
        three.push(("http://127.0.0.2:8545".into(), (1, 0x11)));
        assert_eq!(taken(&three, 0), Taken::Differ, "one place twice beside another receipt");
        three.push(("http://127.0.0.3:8545".into(), (1, 0x10)));
        assert_eq!(taken(&three, 0), Taken::Counts((1, 0x10)), "two places alike");
    }

}
