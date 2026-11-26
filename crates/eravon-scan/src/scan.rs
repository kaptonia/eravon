use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed as k1;
use eravon_standard as a1;
use eravon_terms as k2;

use crate::fault::{Fault, R};
use crate::log::{self, Log};

#[derive(Clone, Debug)]
pub struct Case {
    pub case_id: String,
    pub instance: String,
    pub seller: String,
    pub buyer: String,
    pub block: u64,
    pub tx: String,
}

#[derive(Clone, Debug)]
pub struct Sweep {
    pub cases: Vec<Case>,
    pub foreign: u64,
    pub agreement: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Seller,
    Buyer,
}

fn key_topics(side: Side, key_topic: &str) -> Vec<Option<String>> {
    keys_topics(side, &[key_topic.to_string()]).into_iter().map(|p| p.into_iter().next()).collect()
}


fn keys_topics(side: Side, key_topics: &[String]) -> Vec<Vec<String>> {
    let t0 = vec![opened_topic0()];
    match side {
        Side::Seller => vec![t0, Vec::new(), key_topics.to_vec()],
        Side::Buyer => vec![t0, Vec::new(), Vec::new(), key_topics.to_vec()],
    }
}


fn asked(keys: &[String]) -> &[String] {
    keys
}


pub fn key_filter(side: Side, key: &str) -> R<Vec<Option<String>>> {
    Ok(key_topics(side, &log::address_topic(key)?))
}

pub fn anchored_topic0() -> String {
    a1::order::hex_of(&a1::sol::VENUE_EVENTS[2].topic0())
}

fn opened_topic0() -> String {
    let ev = a1::sol::OPENED;
    a1::order::hex_of(&ev.topic0())
}

pub fn by_key<T: Transport>(
    t: &T,
    table: &Table,
    logic: &str,
    side: Side,
    key: &str,
    from: u64,
    to: a2::seam::Upto,
) -> R<Sweep> {
    by_keys(t, table, logic, side, &[key.to_string()], from, to, &[])
}

#[allow(clippy::too_many_arguments)]
pub fn by_keys<T: Transport>(
    t: &T,
    table: &Table,
    logic: &str,
    side: Side,
    keys: &[String],
    from: u64,
    to: a2::seam::Upto,
    known: &[String],
) -> R<Sweep> {
    crate::mark::mark(crate::mark::FEATURE);
    if keys.is_empty() {
        return Ok(Sweep { cases: Vec::new(), foreign: 0, agreement: "agreed" });
    }
    let book = crate::succession::logs(t, table)?;
    let mut all: Vec<String> = keys.iter().map(|k| k.to_ascii_lowercase()).collect();
    for k in keys {
        for r in eravon_galeed::book::record_keys(&book, k) {
            let r = r.to_ascii_lowercase();
            if !all.contains(&r) {
                all.push(r);
            }
        }
    }
    let keys = asked(&all);
    let topics = keys.iter().map(|k| log::address_topic(k)).collect::<R<Vec<String>>>()?;
    let (rows, reading) = a2::read::logs_any(t, table, None, &keys_topics(side, &topics), from, to)?;
    let single = reading.agreement == a2::read::Agreement::Single;
    let mut cases = Vec::new();
    let mut foreign = 0u64;
    let mut checked = Checked::default();
    let chain = table.chain.get().copied();
    let kept = if chain.is_some() { crate::codes::known() } else { crate::codes::Record::new() };
    let kept_ok = |a: &str| chain.is_some_and(|c| kept.contains(&(c, a.to_ascii_lowercase())));
    let emitters: Vec<String> = rows.iter().filter_map(|row| Log::read(row).ok()).map(|l| l.address).filter(|a| !known.iter().any(|k| k.eq_ignore_ascii_case(a)) && !kept_ok(a)).collect();
    a2::read::ahead_codes(t, table, &emitters, a2::seam::TAG_LATEST);
    let mut passed: Vec<(String, u64)> = Vec::new();
    for row in &rows {
        let l = Log::read(row)?;
        let seen = known.iter().any(|k| k.eq_ignore_ascii_case(&l.address)) || kept_ok(&l.address);
        let ok = if seen {
            true
        } else {
            match checked.gather(t, table, &l.address, logic)? {
                Some(x) => {
                    if x {
                        passed.push((l.address.to_ascii_lowercase(), l.block));
                    }
                    x
                }
                None => continue,
            }
        };
        if !ok {
            foreign += 1;
            continue;
        }
        if l.topics.len() != 4 {
            return Err(Fault::Form(format!("Opened 该有四枚 topic,这一条有 {}", l.topics.len())));
        }
        cases.push(Case {
            case_id: l.topic_hex(1)?,
            instance: l.address.clone(),
            seller: l.topic_address(2)?,
            buyer: l.topic_address(3)?,
            block: l.block,
            tx: l.tx.clone(),
        });
    }
    checked.finish()?;
    if let (Some(c), false) = (chain, passed.is_empty()) {
        if let Ok((head, _)) = a2::read::height(t, table) {
            let settled: Vec<String> = passed.iter().filter(|(_, b)| b.saturating_add(a2::settled::MARGIN) <= head).map(|(a, _)| a.clone()).collect();
            crate::codes::keep(c, &settled);
        }
    }
    in_order(&mut cases, |c| (c.block, c.case_id.clone()));
    Ok(Sweep { cases, foreign, agreement: if single { "single" } else { "agreed" } })
}

#[derive(Default)]
struct Checked {
    seen: std::collections::BTreeMap<String, bool>,
    unread: Option<Fault>,
}

impl Checked {
    fn emitter_ok<T: Transport>(&mut self, t: &T, table: &Table, addr: &str, logic: &str) -> R<bool> {
        let k = addr.to_ascii_lowercase();
        if let Some(v) = self.seen.get(&k) {
            return Ok(*v);
        }
        let v = emitter_ok(t, table, addr, logic)?;
        self.seen.insert(k, v);
        Ok(v)
    }

    fn gather<T: Transport>(&mut self, t: &T, table: &Table, addr: &str, logic: &str) -> R<Option<bool>> {
        match self.emitter_ok(t, table, addr, logic) {
            Ok(x) => Ok(Some(x)),
            Err(e) if is_unread(&e) => {
                self.unread.get_or_insert(e);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    fn finish(self) -> R<()> {
        self.unread.map_or(Ok(()), Err)
    }
}

fn is_unread(e: &Fault) -> bool {
    e.name() == a2::fault::Fault::Unread(String::new()).name()
}

fn emitter_ok<T: Transport>(t: &T, table: &Table, addr: &str, logic: &str) -> R<bool> {
    match a1::chain::code_generation(t, table, addr, logic) {
        Ok(_) => Ok(true),
        Err(a1::fault::Fault::CodeHash { .. }) => Ok(false),
        Err(e) => Err(e.into()),
    }
}


#[derive(Clone, Debug)]
pub struct Rebuilt {
    pub case_id: String,
    pub seller: String,
    pub buyer: String,
    pub token: String,
    pub amount: String,
    pub terms_hash: String,
    pub members: usize,
    pub path: &'static str,
    pub init: a1::order::Init,
    pub logic: String,
    pub past: bool,
}

#[derive(Clone, Debug)]
pub enum Rebuild {
    Done(Box<Rebuilt>),
    Untraced,
}

pub const SAY_UNTRACED: &str = "未能读取调用追踪";

fn check_rebuild(want: &str, got: &str) -> R<()> {
    if want.eq_ignore_ascii_case(got) {
        Ok(())
    } else {
        Err(Fault::Rebuild { want: want.to_string(), got: got.to_string() })
    }
}


pub fn rebuild<T: Transport>(
    t: &T,
    table: &Table,
    tx_hash: &str,
    chain_id: u64,
    current: &str,
    want_case_id: &str,
) -> R<Rebuild> {
    rebuild_among(t, table, tx_hash, chain_id, current, eravon_literals::constants::past_rows(), want_case_id)
}

pub fn rebuild_among<T: Transport>(
    t: &T,
    table: &Table,
    tx_hash: &str,
    chain_id: u64,
    current: &str,
    past: &[eravon_literals::constants::StandardTable],
    want_case_id: &str,
) -> R<Rebuild> {
    let rows: Vec<(String, bool)> = std::iter::once((current.to_string(), false))
        .chain(past.iter().filter(|r| r.chain_id == chain_id && !r.logic.eq_ignore_ascii_case(current)).map(|r| (r.logic.to_string(), true)))
        .collect();
    let (tx, _) = a2::read::transaction(t, table, tx_hash)?;
    let input = tx
        .get("input")
        .and_then(|x| x.as_str())
        .ok_or_else(|| Fault::Form("这一笔交易上没有 input".into()))?;
    let raw = a1::order::bytes_of("calldata", input).map_err(crate::fault::Fault::from)?;
    let to = tx.get("to").and_then(|x| x.as_str()).unwrap_or_default();
    let (target, bytes, path) = if a1::call::is_init(&raw) {
        (to.to_string(), raw, "top")
    } else {
        match traced_init(t, table, tx_hash, &rows, want_case_id)? {
            Some((at, b)) => (at, b, "trace"),
            None => return Ok(Rebuild::Untraced),
        }
    };
    let init = a1::order::decode_init(&bytes)?;
    let (logic, past, got) = row_of(&init, chain_id, &rows, &target)?;
    check_rebuild(want_case_id, &got)?;
    Ok(Rebuild::Done(Box::new(Rebuilt {
        case_id: got,
        seller: init.order.offer.seller_key.clone(),
        buyer: init.order.buyer_key.clone(),
        token: init.order.offer.token.clone(),
        amount: init.order.offer.amount.to_dec(),
        terms_hash: a1::order::hex_of(&init.order.offer.terms_hash),
        members: init.members.len(),
        path,
        init,
        logic,
        past,
    })))
}

fn row_of(init: &a1::order::Init, chain_id: u64, rows: &[(String, bool)], target: &str) -> R<(String, bool, String)> {
    let mut first: Option<(String, bool, String)> = None;
    for (logic, past) in rows {
        let d = init.order.derive(chain_id, logic)?;
        let got = a1::order::hex_of(&d.salt);
        if d.instance.eq_ignore_ascii_case(target) {
            return Ok((logic.clone(), *past, got));
        }
        first.get_or_insert((logic.clone(), *past, got));
    }
    first.ok_or_else(|| Fault::Form("没有可对的标准表行".into()))
}

fn traced_init<T: Transport>(t: &T, table: &Table, tx_hash: &str, rows: &[(String, bool)], case_id: &str) -> R<Option<(String, Vec<u8>)>> {
    let mut missed: Option<String> = None;
    for (logic, _) in rows {
        match inner_init(t, table, tx_hash, logic, case_id)? {
            Inner::Found(at, b) => return Ok(Some((at, b))),
            Inner::Untraced => return Ok(None),
            Inner::NotFound(instance) => {
                missed.get_or_insert(instance);
            }
        }
    }
    Err(Fault::Form(format!("这一笔里没有对实例 {} 的未回滚的 init 调用", missed.unwrap_or_default())))
}

enum Inner {
    Found(String, Vec<u8>),
    NotFound(String),
    Untraced,
}

fn inner_init<T: Transport>(
    t: &T,
    table: &Table,
    tx_hash: &str,
    logic: &str,
    case_id: &str,
) -> R<Inner> {
    let salt = a1::order::b32_of("案号", case_id)?;
    let code_hash = k2::derive::keccak(&k2::derive::initcode(logic));
    let instance = k2::derive::create2(eravon_literals::constants::PUBLIC_DEPLOYER, &salt, &code_hash);
    let trial = eravon_literals::constants::generation_of_logic(logic).is_some_and(|g| g.shape() == eravon_literals::constants::Shape::Trial);
    let sel = a1::order::hex_of(&if trial { a1::sol::TRIAL_INIT } else { a1::sol::INIT }.selector());
    match a2::trace::find(t, table, tx_hash, &instance, &sel)?.0 {
        a2::trace::Traced::Found { call, .. } => Ok(Inner::Found(instance, a1::order::bytes_of("内层 calldata", &call.input)?)),
        a2::trace::Traced::NotFound { .. } => Ok(Inner::NotFound(instance)),
        a2::trace::Traced::Untraced => Ok(Inner::Untraced),
    }
}


pub fn backfill(
    r: &Rebuilt,
    chain_id: u64,
    judges: &[String],
    guilds: &[String],
    seq: Option<u64>,
) -> R<Vec<u8>> {
    let o = &r.init.order;
    let zero = k2::derive::zero_address();
    let (candidate, is_guild) = if !o.offer.judge.eq_ignore_ascii_case(&zero) {
        match position(judges, &o.offer.judge) {
            Some(i) => (i, false),
            None => {
                return Err(Fault::Form(format!("载荷的判官候选里没有要约上的那一位 {}", o.offer.judge)));
            }
        }
    } else {
        match position(guilds, &o.offer.guild) {
            Some(i) => (judges.len() + i, true),
            None => {
                return Err(Fault::Form(format!("载荷的行会候选里没有要约上的那一家 {}", o.offer.guild)));
            }
        }
    };
    let digest = o.derive(chain_id, &r.logic)?.order_digest;
    let sig = verified_sig(&digest, &r.init.sig_buyer, &o.buyer_key);
    let mut m: Vec<(String, k1::Value)> = vec![
        ("buyerKey".into(), k1::Value::Str(o.buyer_key.to_ascii_lowercase())),
        ("buyerPayout".into(), k1::Value::Str(o.buyer_payout.to_ascii_lowercase())),
        ("candidate".into(), k1::Value::Int(candidate as u64)),
        ("openBy".into(), k1::Value::Int(o.open_by)),
        ("prev".into(), k1::Value::Str(o.prev.to_ascii_lowercase())),
        ("sig".into(), sig),
        ("termsHash".into(), k1::Value::Str(a1::order::hex_of(&o.offer.terms_hash))),
        ("v".into(), k1::Value::Str(eravon_literals::literals::DOC_RECEIPT.into())),
    ];
    if let Some(n) = seq.filter(|_| is_guild) {
        m.push(("seq".into(), k1::Value::Int(n)));
    }
    k1::canon::canonical(&k1::Value::Obj(m)).map_err(|t| Fault::Form(format!("回执写不成正典形({})", t.name())))
}

fn position(list: &[String], addr: &str) -> Option<usize> {
    list.iter().position(|x| x.eq_ignore_ascii_case(addr))
}


pub fn sig_value(b: &[u8]) -> R<k1::Value> {
    match b.len() {
        0 => Ok(k1::Value::Str(String::new())),
        65 => Ok(k1::Value::Str(a1::order::hex_of(b))),
        n => Err(Fault::Form(format!("签名槽只认空串或六十五字节,这一槽 {n} 字节"))),
    }
}

fn verified_sig(digest: &[u8; 32], slot: &[u8], key: &str) -> k1::Value {
    let hex = a1::order::hex_of(slot);
    if slot.len() == 65 && k2::sig::verify_secp256k1(digest, &hex, &key.to_ascii_lowercase()).is_ok() {
        k1::Value::Str(hex)
    } else {
        k1::Value::Str(String::new())
    }
}

#[derive(Clone, Debug)]
pub struct Moment {
    pub name: String,
    pub block: u64,
    pub log_index: u64,
    pub tx: String,
}

fn order_history(rows: &mut [Moment]) {
    rows.sort_by(|a, b| a.block.cmp(&b.block).then(a.log_index.cmp(&b.log_index)));
}


pub fn logic_records<T: Transport>(t: &T, table: &Table, logic: &str, keys: &[String], to: a2::seam::Upto) -> R<Vec<Log>> {
    let mut events: Vec<String> = [a1::sol::CONSENTED_EV, a1::sol::PULLED_EV, a1::sol::REVOKED_EV, a1::sol::FUND_BOUND_EV].iter().map(|e| a1::order::hex_of(&e.topic0())).collect();
    events.sort();
    let mut who = keys.iter().map(|k| log::address_topic(k)).collect::<R<Vec<String>>>()?;
    who.sort();
    who.dedup();
    let mut out: Vec<Log> = Vec::new();
    let at = logic.to_ascii_lowercase();
    {
        let (rows, _) = a2::read::logs_any(t, table, Some(&at), &[events.clone(), who.clone()], 0, to)?;
        for row in &rows {
            let l = Log::read(row)?;
            if l.address.eq_ignore_ascii_case(logic) && !out.iter().any(|x| (x.block, x.log_index, &x.tx) == (l.block, l.log_index, &l.tx)) {
                out.push(l);
            }
        }
    }
    out.sort_by(|a, b| a.block.cmp(&b.block).then(a.log_index.cmp(&b.log_index)));
    Ok(out)
}

pub fn picked<'a>(records: &'a [Log], event: &[u8; 32], rest: &[Option<String>]) -> Vec<&'a Log> {
    records
        .iter()
        .filter(|l| l.topics.first() == Some(event))
        .filter(|l| rest.iter().enumerate().all(|(i, want)| want.as_ref().is_none_or(|w| l.topics.get(i + 1).is_some_and(|got| a1::order::hex_of(got).eq_ignore_ascii_case(w)))))
        .collect()
}

pub fn history<T: Transport>(
    t: &T,
    table: &Table,
    chain_id: u64,
    logic: &str,
    instance: &str,
    from: u64,
    to: a2::seam::Upto,
) -> R<Vec<Moment>> {
    let (rows, _) = a2::read::logs(t, table, instance, from, to)?;
    let mut out = Vec::new();
    let mut opened: Option<(String, String, String, String)> = None;
    let mut settled: Option<String> = None;
    for row in &rows {
        let l = Log::read(row)?;
        let decoded = a1::event::decode(&l.topics, &l.data)?;
        if decoded.name == "Opened" && opened.is_none() {
            opened = Some((l.topic_hex(1)?, l.topic_address(2)?, l.topic_address(3)?, l.tx.clone()));
        }
        if decoded.name == "Settled" && settled.is_none() {
            settled = Some(l.tx.clone());
        }
        out.push(Moment {
            name: decoded.name.to_string(),
            block: l.block,
            log_index: l.log_index,
            tx: l.tx.clone(),
        });
    }
    if let Some((case_id, seller, buyer, tx)) = opened {
        let salt = a1::order::b32_of("案号", &case_id)?;
        let mut pairs = vec![(buyer.clone(), k2::derive::digest(chain_id, instance, &salt))];
        let mut made = logic.to_string();
        if let Ok(Rebuild::Done(r)) = rebuild(t, table, &tx, chain_id, logic, &case_id) {
            pairs.push((seller.clone(), r.init.order.offer.offer_digest(chain_id, &r.logic)));
            made = r.logic.clone();
        }
        if let Some(stx) = &settled {
            if let Some(d) = settle_digest(t, table, stx, chain_id, instance)? {
                pairs.push((buyer.clone(), d));
                pairs.push((seller.clone(), d));
            }
        }
        let records = logic_records(t, table, &made, &[seller.clone(), buyer.clone()], to)?;
        for (who, d) in pairs {
            for l in picked(&records, &a1::sol::CONSENTED_EV.topic0(), &[Some(log::address_topic(&who)?), Some(a1::order::hex_of(&d))]) {
                if !a1::consent::from_logic(&l.address, &made) {
                    continue;
                }
                out.push(Moment { name: "Consented".into(), block: l.block, log_index: l.log_index, tx: l.tx.clone() });
            }
        }
    }
    order_history(&mut out);
    Ok(out)
}

fn settle_digest<T: Transport>(t: &T, table: &Table, tx_hash: &str, chain_id: u64, instance: &str) -> R<Option<[u8; 32]>> {
    let (tx, _) = a2::read::transaction(t, table, tx_hash)?;
    let sel = a1::sol::SETTLE.selector();
    let to = tx.get("to").and_then(|x| x.as_str()).unwrap_or_default();
    let raw = a1::order::bytes_of("calldata", tx.get("input").and_then(|x| x.as_str()).unwrap_or("0x"))?;
    let input = if to.eq_ignore_ascii_case(instance) && raw.starts_with(&sel) {
        raw
    } else {
        match a2::trace::find(t, table, tx_hash, instance, &a1::order::hex_of(&sel))?.0 {
            a2::trace::Traced::Found { call, .. } => a1::order::bytes_of("内层 calldata", &call.input)?,
            a2::trace::Traced::NotFound { .. } | a2::trace::Traced::Untraced => return Ok(None),
        }
    };
    let v = a1::abi::decode(&a1::abi::args_of(a1::sol::SETTLE.sig)?, input.get(4..).unwrap_or_default())?;
    let h = k2::derive::Action::Settle { buyer_bps: v[0].small()?, expiry: k2::u256::U256::from_be(&v[1].word()?) }.struct_hash();
    Ok(Some(k2::derive::digest(chain_id, instance, &h)))
}

#[derive(Clone, Debug)]
pub struct OrderNo {
    pub case_id: String,
    pub instance: String,
    pub month: String,
    pub seq: u64,
}

fn in_order<T>(rows: &mut [T], key: impl Fn(&T) -> (u64, String)) {
    rows.sort_by_key(key);
}


pub fn order_numbers(cases: &[Case], stamps: &[u64]) -> R<Vec<OrderNo>> {
    if cases.len() != stamps.len() {
        return Err(Fault::Form("案与时刻两表要等长".into()));
    }
    let mut rows: Vec<(u64, String, u64, String)> = cases
        .iter()
        .zip(stamps)
        .map(|(c, s)| (c.block, c.case_id.clone(), *s, c.instance.clone()))
        .collect();
    in_order(&mut rows, |r| (r.0, r.1.clone()));
    let mut out = Vec::new();
    let mut seen: Vec<(String, u64)> = Vec::new();
    for (_, case_id, stamp, instance) in rows {
        let month = month_of(stamp);
        let seq = match seen.iter_mut().find(|(m, _)| *m == month) {
            Some((_, n)) => {
                *n += 1;
                *n
            }
            None => {
                seen.push((month.clone(), 1));
                1
            }
        };
        out.push(OrderNo { case_id, instance, month, seq });
    }
    Ok(out)
}

pub fn month_of(secs: u64) -> String {
    let mut days = secs / 86_400;
    let mut year = 1970u64;
    loop {
        let len = if leap(year) { 366 } else { 365 };
        if days < len {
            break;
        }
        days -= len;
        year += 1;
    }
    let lens = [31u64, if leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1usize;
    for l in lens {
        if days < l {
            break;
        }
        days -= l;
        month += 1;
    }
    format!("{year:04}-{month:02}")
}

fn leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn block_stamp<T: Transport>(t: &T, table: &Table, block: u64) -> R<u64> {
    block_stamp_at(t, table, block, None)
}

pub fn block_stamp_at<T: Transport>(t: &T, table: &Table, block: u64, hash: Option<&str>) -> R<u64> {
    let kept = |c: Option<&u64>| c.and_then(|c| crate::times::time_of(*c, block, hash));
    if let Some(at) = kept(table.chain.get()) {
        return Ok(at);
    }
    let (n, _) = a2::clock::now(t, table)?;
    if block > n.block {
        return Err(Fault::Form(format!("第 {block} 块还没出")));
    }
    if let Some(at) = kept(table.chain.get()) {
        return Ok(at);
    }
    let params = vec![eravon_literals::json::Out::S(a2::seam::quantity(block)), eravon_literals::json::Out::B(false)];
    let r = a2::read::read_agreed(t, table, a2::seam::M_GET_BLOCK, params.clone())?;
    let s = r
        .value
        .get("timestamp")
        .and_then(|x| x.as_str())
        .ok_or_else(|| Fault::Form("块上没有 timestamp".into()))?;
    let at = a2::seam::read_quantity(s)?;
    if let Some(c) = table.chain.get()
        && block.saturating_add(a2::settled::MARGIN) <= n.block
        && let Some(h) = a2::read::held_alike(table, a2::seam::M_GET_BLOCK, &params, "hash")
        && hash.is_none_or(|x| x.eq_ignore_ascii_case(&h))
    {
        crate::times::keep(*c, block, &h, at);
    }
    Ok(at)
}

pub fn ahead_stamps<T: Transport>(t: &T, table: &Table, blocks: &[u64]) {
    let chain = table.chain.get().copied();
    let kept = if chain.is_some() { crate::times::known() } else { crate::times::Record::new() };
    let left: Vec<u64> = blocks.iter().copied().filter(|b| chain.and_then(|c| crate::times::time_in(&kept, c, *b, None)).is_none()).collect();
    a2::read::ahead_blocks(t, table, &left);
}
