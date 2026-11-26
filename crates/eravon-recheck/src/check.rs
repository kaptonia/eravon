use std::collections::BTreeSet;

use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed as k1;
use eravon_literals::literals as lit;
use eravon_scan as a3;
use eravon_standard as a1;
use eravon_terms as k2;

use crate::dossier::{Case, Row, Terminal};
use crate::fault::{Fault, R};
use crate::judge::Audit;
use crate::read::{self, Caller, Event, Tx};
use crate::report::{Item, Name, State};

const L_POSTED: &str = "dossier/1 §5.1 公告板 Posted";
const L_CODE: &str = "dossier/1 §5.1 实例码";
const L_ROW: &str = "dossier/1 §5.1 交易行与见证表";
const L_OPENED: &str = "dossier/1 §5.1 Opened 两方同意";
const L_CALLER: &str = "dossier/1 §5.1 buyerMayOpen 为假时 init 的调用者";
const L_SETTLED: &str = "dossier/1 §5.1 Settled 两方同意";
const L_SEAT: &str = "dossier/1 §5.1 Seated 与 seatOf";
const L_BEYOND: &str = "dossier/1 §5.1 链上有而卷宗无";
const L_PANEL: &str = "dossier/1 §5.1 PanelFrozen";
const L_TERMINAL: &str = "dossier/1 §5.1 终端与 Closed";
const L_ANCHORED: &str = "dossier/1 §5.1 Anchored 携带形";
const L_LINEAGE: &str = "dossier/1 §5.1 判官世系";
const L_WORD: &str = "dossier/1 §5.1 判法词";
const L_DECISION: &str = "dossier/1 §5.1 Ruled 档与辞任";
const L_LEDGER: &str = "dossier/1 §5.1 判官账本的根与基底";
const L_COMPLETE: &str = "dossier/1 §6 完整性";

#[derive(Clone, Debug, Default)]
pub struct Opts {
    pub registries: Vec<String>,
    pub events_from: u64,
}

struct Seen {
    evs: Vec<Event>,
    opened: Option<Event>,
    init: Option<Call>,
    settle: Option<Settle>,
    post: Option<a3::board::Post>,
    book: k1::book::Logs,
    generation: eravon_literals::constants::Generation,
}

impl Seen {
    fn shape(&self) -> eravon_literals::constants::Shape {
        self.generation.shape()
    }

    fn fun(&self, current: a1::sol::Fun) -> a1::sol::Fun {
        if self.shape() != eravon_literals::constants::Shape::Trial {
            return current;
        }
        match current.name {
            "init" => a1::sol::TRIAL_INIT,
            "cancel" => a1::sol::TRIAL_CANCEL,
            _ => current,
        }
    }
}

fn current_at(s: &Seen, a: &str, at: (u64, u64)) -> String {
    if s.generation.is_past() {
        return a.to_ascii_lowercase();
    }
    a3::succession::current_at(&s.book, a, at)
}

fn pos(tx: &Tx) -> (u64, u64) {
    (tx.block, tx.logs.iter().map(|l| l.log_index).min().unwrap_or(0))
}

fn root_at(s: &Seen, a: &str, at: (u64, u64)) -> String {
    s.book
        .accessions
        .iter()
        .filter(|x| x.emitter.eq_ignore_ascii_case(a) && (x.block_number, x.log_index) < at)
        .map(|x| x.root.to_ascii_lowercase())
        .next()
        .unwrap_or_else(|| a.to_ascii_lowercase())
}

pub fn run<T: Transport>(t: &T, table: &Table, c: &Case, o: &Opts) -> R<Vec<Item>> {
    crate::mark::mark(crate::mark::FEATURE);
    a2::read::chain_guard(t, table, c.chain_id)?;
    let (head, _) = a2::read::height(t, table)?;
    let (s, posted) = seen(t, table, c, o.events_from)?;
    let s = std::sync::Arc::new(s);
    enum Part {
        Code,
        Row(usize),
        Parties,
        Settled,
        Seated,
        Panel,
        Audit,
    }
    enum Got {
        Items(Vec<Item>),
        Audit(Box<crate::judge::Audit>),
    }
    let mut parts = vec![Part::Code];
    parts.extend((0..c.rows.len()).map(Part::Row));
    parts.extend([Part::Parties, Part::Settled, Part::Seated]);
    if c.pool() && s.opened.is_some() {
        parts.push(Part::Panel);
    }
    if c.entry.is_some() {
        parts.push(Part::Audit);
    }
    let lost = || Err(Fault::Form("复核的一路读没有做完(线程停了)".into()));
    let ctx = std::sync::Arc::new((t.clone(), table.clone(), c.clone(), s.clone(), o.registries.clone()));
    let got: Vec<R<Got>> = a2::endpoints::until_enough(
        parts,
        move |p| -> R<Got> {
            let (t, table, c, s, registries) = &*ctx;
            Ok(match p {
                Part::Code => Got::Items(vec![code(t, table, c)?]),
                Part::Row(i) => Got::Items(row_items(t, table, c, s, i, &c.rows[i])?),
                Part::Parties => Got::Items(opened_parties(t, table, c, s)?),
                Part::Settled => Got::Items(settled(t, table, c, s)?),
                Part::Seated => Got::Items(seated(t, table, c, s)?),
                Part::Panel => Got::Items(vec![panel(t, table, c, s)?]),
                Part::Audit => Got::Audit(Box::new(crate::judge::audit(t, table, c, registries, head)?)),
            })
        },
        lost,
        a2::endpoints::every,
    )
    .into_iter()
    .map(|g| g.unwrap_or_else(lost))
    .collect();
    let mut items = vec![posted];
    let mut audit = None;
    for g in got {
        match g? {
            Got::Items(x) => items.extend(x),
            Got::Audit(a) => audit = Some(a),
        }
    }
    if let Some(term) = &c.terminal {
        items.push(terminal(term, &s.evs)?);
    }
    let anchored_tx = anchored(c, &s, &s.evs).map(|e| e.log.tx.to_ascii_lowercase());
    if let Some(audit) = audit {
        items.extend(entry_items(c, &s, &audit, &o.registries)?);
    }
    items.extend(beyond(c, &s, anchored_tx.as_deref())?);
    items.push(complete(t, table, c, &s)?);
    Ok(items)
}

fn seen<T: Transport>(t: &T, table: &Table, c: &Case, from: u64) -> R<(Seen, Item)> {
    let evs = read::instance_events(t, table, &c.instance, from)?;
    let opened = evs.iter().find(|e| e.ev.name == "Opened").cloned();
    let generation = generation_of(t, table, c)?;
    let book = a3::succession::logs(t, table)?;
    let mut s = Seen { evs, opened, init: None, settle: None, post: None, book, generation };
    if let Some(e) = s.opened.clone() {
        s.init = Some(call_of(t, table, c, &e.log.tx, &s.fun(a1::sol::INIT), (e.log.block, e.log.log_index))?);
    }
    s.settle = settle_call(t, table, c, &s.evs)?;
    let (posted, post) = posted(t, table, c)?;
    s.post = post;
    Ok((s, posted))
}

fn generation_of<T: Transport>(t: &T, table: &Table, c: &Case) -> R<eravon_literals::constants::Generation> {
    match a1::chain::case_generation(t, table, &c.instance, &c.logic, Some(&c.logic)) {
        Err(eravon_standard::fault::Fault::CodeHash { .. }) => Ok(eravon_literals::constants::generation_of_logic(&c.logic).unwrap_or(eravon_literals::constants::Generation::Current)),
        g => g.map_err(Into::into),
    }
}

pub fn rows_of<T: Transport>(t: &T, table: &Table, c: &Case, more: &[String], from: u64) -> R<Vec<Row>> {
    crate::mark::mark(crate::mark::FEATURE);
    let (s, _) = seen(t, table, c, from)?;
    let mut txs: Vec<String> = owed(t, table, c, &s)?.into_iter().collect();
    for m in more {
        let m = m.to_ascii_lowercase();
        if !txs.contains(&m) {
            txs.push(m);
        }
    }
    let one = |h: &String| format!("[\"{h}\"]");
    a2::read::ask_ahead(t, table, a2::seam::M_GET_TX, &txs.iter().map(one).collect::<Vec<_>>());
    a2::read::ask_ahead(t, table, a2::seam::M_GET_RECEIPT, &txs.iter().map(one).collect::<Vec<_>>());
    if a2::read::holds_ahead(table) {
        let blocks: Vec<u64> = txs.iter().filter_map(|h| read::tx(t, table, h).ok().flatten()).map(|tx| tx.block).collect();
        a3::scan::ahead_stamps(t, table, &blocks);
        a2::read::ahead_codes_by(t, table, &blocks.iter().filter(|b| **b > 0).map(|b| (c.instance.clone(), b - 1)).collect::<Vec<_>>());
    }
    a2::trace::ahead(t, table, &txs);
    let mut out = Vec::new();
    let mut stamps: Vec<(u64, u64)> = Vec::new();
    for h in &txs {
        let Some(tx) = read::tx(t, table, h)? else { continue };
        let timestamp = match stamps.iter().find(|(b, _)| *b == tx.block) {
            Some((_, ts)) => *ts,
            None => {
                let ts = a3::scan::block_stamp(t, table, tx.block)?;
                stamps.push((tx.block, ts));
                ts
            }
        };
        for kind in lit::TRANSACTION_KINDS {
            let row = Row { kind: kind.to_string(), tx: h.clone(), block: tx.block, timestamp };
            if witness(t, table, c, &s, &row, &tx)?.0 == State::Pass {
                out.push(row);
            }
        }
    }
    out.sort_by_key(|r| r.block);
    Ok(out)
}

fn hex(b: &[u8]) -> String {
    a1::order::hex_of(b)
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn b32(what: &str, s: &str) -> R<[u8; 32]> {
    Ok(a1::order::b32_of(what, s)?)
}

fn topic(addr: &str) -> R<String> {
    Ok(a3::log::address_topic(addr)?)
}

fn keys_of(c: &Case) -> Vec<String> {
    vec![c.listing.order.seller_key.clone(), c.receipt.buyer_key.clone()]
}

fn posted<T: Transport>(t: &T, table: &Table, c: &Case) -> R<(Item, Option<a3::board::Post>)> {
    let hash = b32("posted.hash", &c.posted.hash)?;
    let key = b32("terms.hash", &c.listing.terms_hash)?;
    let posts = a3::board::by_tx(t, table, &c.posted.tx, &hash)?;
    let hit = posts.iter().find(|p| p.key == key && p.document == c.listing_bytes).cloned();
    Ok(match hit {
        Some(p) => (Item::pass(Name::Posted, L_POSTED, format!("交易 {} 在第 {} 块:公告板发了 Posted,key 是 terms.hash,文书即清单档", c.posted.tx, p.block)), Some(p)),
        None => {
            let seen: Vec<String> = posts.iter().map(|p| format!("(key {}, {} 字节)", hex(&p.key), p.document.len())).collect();
            (Item::mismatch(Name::Posted, L_POSTED, format!("交易 {} 里公告板码发的、hash 为 {} 的 Posted:[{}];没有一条 key 是 {} 且文书即清单档", c.posted.tx, c.posted.hash, seen.join(", "), c.listing.terms_hash)), None)
        }
    })
}

fn code<T: Transport>(t: &T, table: &Table, c: &Case) -> R<Item> {
    let code = read::code_at(t, table, &c.instance, None)?;
    let carries = c.rows.iter().any(|r| r.kind == "deploy") || c.terminal.is_some();
    let h = hex(&k2::derive::keccak(&code));
    if !carries {
        let said = if code.is_empty() { format!("实例 {} 上没有码:已注资未部署,非故障", c.instance) } else { format!("实例 {} 上的码 keccak256 是 {h}", c.instance) };
        return Ok(Item::pass(Name::Code, L_CODE, said));
    }
    if code.is_empty() {
        return Ok(Item::mismatch(Name::Code, L_CODE, format!("实例 {} 上没有码,而卷宗带 deploy 行或终端", c.instance)));
    }
    Ok(if same(&h, &c.proxy_code_hash) {
        Item::pass(Name::Code, L_CODE, format!("实例上的码 keccak256 是 {h},等于 proxyCodeHash"))
    } else {
        Item::mismatch(Name::Code, L_CODE, format!("实例上的码 keccak256 是 {h},proxyCodeHash 是 {}", c.proxy_code_hash))
    })
}

fn row_items<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen, i: usize, row: &Row) -> R<Vec<Item>> {
    let tx = read::tx(t, table, &row.tx)?;
    let mut out = Vec::new();
    out.push(match &tx {
        None => Item::mismatch(Name::RowTx, L_ROW, format!("链上没有交易 {}", row.tx)),
        Some(x) if x.block != row.block => Item::mismatch(Name::RowTx, L_ROW, format!("交易 {} 在第 {} 块,卷宗写第 {} 块", row.tx, x.block, row.block)),
        Some(x) => Item::pass(Name::RowTx, L_ROW, format!("交易 {} 在第 {} 块", row.tx, x.block)),
    }
    .at(i, &row.kind));
    out.push(match a3::scan::block_stamp(t, table, row.block) {
        Ok(ts) if ts == row.timestamp => Item::pass(Name::RowTimestamp, L_ROW, format!("第 {} 块的块头时刻是 {ts}", row.block)),
        Ok(ts) => Item::mismatch(Name::RowTimestamp, L_ROW, format!("第 {} 块的块头时刻是 {ts},卷宗写 {}", row.block, row.timestamp)),
        Err(a3::fault::Fault::Form(s)) => Item::mismatch(Name::RowTimestamp, L_ROW, s),
        Err(e) => return Err(e.into()),
    }
    .at(i, &row.kind));
    let (state, said, why) = match &tx {
        None => (State::Mismatch, format!("链上没有交易 {},无从见证 {}", row.tx, row.kind), None),
        Some(x) => table_says(t, table, c, s, row, x)?,
    };
    out.push(Item { item: Name::RowKind, law: L_ROW, row: Some((i, row.kind.clone())), state, read: said, why });
    Ok(out)
}

type Said = (State, String, Option<String>);

fn ok(s: String) -> Said {
    (State::Pass, s, None)
}

fn no(s: String) -> Said {
    (State::Mismatch, s, None)
}

fn not_recomputed(s: String, why: &str) -> Said {
    (State::Unread, s, Some(why.to_string()))
}

fn untraced(read: String, why: &str) -> Said {
    (State::Unread, read, Some(why.to_string()))
}


pub const WHY_UNTRACED: &str = "端点不供这一笔的调用追踪";
pub const WHY_OTHER_DIGEST: &str = "Consented 的摘要不在 §5.1 的三对之内(其余摘要,非故障)";
pub const WHY_NO_QUOTE: &str = "定 P 与 lf 的报价字节未读到(init 调用未读)";
pub const WHY_BEYOND: &str = "链上有而卷宗之外,非故障:§6 的完整性读它";

fn table_says<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen, row: &Row, tx: &Tx) -> R<Said> {
    witness(t, table, c, s, row, tx)
}


fn emitted(evs: &[Event]) -> String {
    let names: Vec<&str> = evs.iter().map(|e| e.ev.name).collect();
    format!("这一笔对实例发了 [{}]", names.join(", "))
}

fn cause_of(e: &Event) -> R<u64> {
    Ok(e.ev.at(0)?.small()?)
}

fn witness<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen, row: &Row, tx: &Tx) -> R<Said> {
    let evs = read::events(&tx.logs, &c.instance);
    let has = |name: &str| evs.iter().any(|e| e.ev.name == name);
    let by_event = |name: &'static str| -> Said { if has(name) { ok(format!("这一笔对实例发了 {name}")) } else { no(emitted(&evs)) } };
    let refunded = |want: u64| -> R<bool> {
        for e in evs.iter().filter(|e| e.ev.name == "Refunded") {
            if cause_of(e)? == want {
                return Ok(true);
            }
        }
        Ok(false)
    };
    Ok(match row.kind.as_str() {
        "post" => match &s.post {
            Some(p) if same(&p.tx, &row.tx) => ok(format!("这一笔就是 posted.tx,公告板发了那一条 Posted(第 {} 块)", p.block)),
            Some(_) => no(format!("posted.tx 是 {},不是这一笔", c.posted.tx)),
            None => no(format!("posted.tx {} 里没有合格的 Posted", c.posted.tx)),
        },
        "fund" => fund(c, tx)?,
        "deploy" => deploy(t, table, c, row, tx)?,
        "init" => init_row(c, s, tx, &evs)?,
        "accept" => accept(t, table, c, s, tx)?,
        "delivery" => by_event("DeliveryClaimed"),
        "confirm" => by_event("Confirmed"),
        "dispute" => by_event("Disputed"),
        "commit" => by_event("Committed"),
        "settle" => by_event("Settled"),
        "rule" => by_event("Ruled"),
        "resign" => resign_row(&evs)?,
        "refund" => {
            if refunded(lit::CAUSE_SELLER_REFUND.into())? {
                ok("这一笔对实例发了 Refunded,因 3".into())
            } else {
                no(emitted(&evs))
            }
        }
        "expire" => {
            if refunded(lit::CAUSE_EXPIRY.into())? {
                expire_row(t, table, c, row)?
            } else {
                no(emitted(&evs))
            }
        }
        "cancel" => cancel_row(t, table, c, s, tx, &evs)?,
        "revoke" => revoke_row(c, tx)?,
        "advance" => kept(t, table, c, tx, &a1::sol::ADVANCE, None)?,
        "redraw" => redraw_row(t, table, c, tx, &evs)?,
        "setPayout" => kept(t, table, c, tx, &a1::sol::JUDGE_FACE[1], Some(&current_judges(c, s, tx)?))?,
        "withdraw" => kept(t, table, c, tx, &a1::sol::JUDGE_FACE[2], Some(&current_judges(c, s, tx)?))?,
        "sweep" => {
            if has("Swept") {
                ok("这一笔对实例发了 Swept".into())
            } else {
                kept(t, table, c, tx, &a1::sol::SWEEP, None)?
            }
        }
        "consent" => consent_row(c, &s.settle, tx)?,
        "fundBind" => fund_bind_row(c, s, tx)?,
        other => no(format!("{other} 不在见证表上")),
    })
}

fn fund(c: &Case, tx: &Tx) -> R<Said> {
    let token = c.listing.token.as_str();
    let t0 = a1::sol::ERC20_TRANSFER_EV.topic0();
    for l in tx.logs.iter().filter(|l| same(&l.address, token) && l.topics.first() == Some(&t0) && l.topics.len() == 3) {
        if same(&l.topic_address(2)?, &c.instance) {
            let word: &[u8; 32] = l.data.get(..32).and_then(|w| w.try_into().ok()).ok_or_else(|| Fault::Form("Transfer 的 data 该有一枚字".into()))?;
            let n = k2::u256::U256::from_be(word).to_dec();
            return Ok(ok(format!("这一笔回执里 {token} 向实例转了 {}", fund_word(c, &n))));
        }
    }
    Ok(no(format!("这一笔回执里没有 {token} 付给实例的 Transfer")))
}

fn fund_word(c: &Case, raw: &str) -> String {
    use eravon_literals::units;
    let factory = eravon_literals::constants::network_of(c.chain_id).and_then(|n| n.tokens.iter().find(|x| same(x.address, &c.listing.token)));
    match factory {
        Some(t) => units::amount(raw, Some(t.decimals), t.symbol, units::Form::Full, &units::ZH),
        None => units::amount(raw, None, &units::short(&c.listing.token), units::Form::Full, &units::ZH),
    }
}

pub fn carries_creation(c: &Case, input: &[u8]) -> bool {
    let call = a1::call::deploy_instance(&c.bound.salt, &c.listing.logic).map(|x| x.data).unwrap_or_default();
    !call.is_empty() && input.windows(call.len()).any(|w| w == call.as_slice())
}

fn deploy<T: Transport>(t: &T, table: &Table, c: &Case, row: &Row, tx: &Tx) -> R<Said> {
    if read::code_before(t, table, &c.instance, row.block) == read::Before::Code {
        return Ok(no(format!("第 {} 块之前实例上已经有码:这一笔不是头一次留码的那一笔", row.block)));
    }
    let initcode = k2::derive::initcode(&c.logic);
    Ok(match read::traced(t, table, tx, &c.instance, initcode.get(..4).unwrap_or_default())? {
        Caller::Own { from, .. } | Caller::Inner { from, .. } => ok(format!("调用追踪里有在实例上留码的那一次创建(创建者 {from})")),
        Caller::Missing => no("调用追踪里没有在实例上留码的那一次创建".into()),
        Caller::Untraced => untraced("那一次创建由部署器做,不是交易自己那一次调用".into(), WHY_UNTRACED),
    })
}

fn deposit_of(c: &Case, s: &Seen, quote: &[u8], evs: &[Event]) -> R<Result<k2::u256::U256, String>> {
    let qh = hex(&k2::derive::keccak(quote));
    if !same(&qh, &c.seat.quote_hash) {
        return Ok(Err(format!("init 递的报价字节 keccak256 是 {qh},席位的 quoteHash 是 {}", c.seat.quote_hash)));
    }
    let q = match k2::quote::decode_under(s.shape(), quote) {
        Ok(q) => q,
        Err(f) => return Ok(Err(format!("报价解不出:{:?}", f.token))),
    };
    let amount = c.listing.order.amount;
    let fee = match k2::fee::fee(&q, &amount) {
        Ok(f) => f,
        Err(f) => return Ok(Err(format!("按报价算费不成:{:?}", f.token))),
    };
    let lf = match fee.lf {
        Some(lf) => lf,
        None => match evs.iter().find(|e| e.ev.name == "Priced") {
            Some(e) => k2::u256::U256::from_be(&e.ev.at(1)?.word()?),
            None => return Ok(Err("报价按价源定价,这一笔没有 Priced 可读 lf".into())),
        },
    };
    let (sum, over) = fee.premium.overflowing_add(&lf);
    if over {
        return Ok(Err("P + lf 越过 2^256 − 1".into()));
    }
    Ok(Ok(sum))
}

fn init_row(c: &Case, s: &Seen, tx: &Tx, evs: &[Event]) -> R<Said> {
    if !evs.iter().any(|e| e.ev.name == "Opened") {
        return Ok(no(emitted(evs)));
    }
    if c.pool() && !evs.iter().any(|e| e.ev.name == "DrawArmed") {
        return Ok(no(format!("行会轨:{};没有 DrawArmed", emitted(evs))));
    }
    let quote = s.init.as_ref().filter(|x| same(&x.tx, &tx.hash)).and_then(|x| x.init.as_ref()).map(|i| i.quote.clone());
    let Some(quote) = quote else {
        return Ok(not_recomputed("这一笔对实例发了 Opened;Pulled 那一比未做".into(), WHY_NO_QUOTE));
    };
    let need = match deposit_of(c, s, &quote, evs)? {
        Ok(n) => n,
        Err(why) => return Ok(no(format!("这一笔对实例发了 Opened;{why}"))),
    };
    if need.is_zero() {
        return Ok(ok("这一笔对实例发了 Opened;P + lf 为零,无 Pulled".into()));
    }
    let t0 = a1::sol::PULLED_EV.topic0();
    for l in tx.logs.iter().filter(|l| a1::consent::from_logic(&l.address, &c.logic) && l.topics.first() == Some(&t0)) {
        let e = a1::event::decode(&l.topics, &l.data)?;
        let seller = hex(&e.at(0)?.addr()?);
        let fund = hex(&e.at(1)?.addr()?);
        let token = hex(&e.at(2)?.addr()?);
        let instance = hex(&e.at(3)?.addr()?);
        let amount = k2::u256::U256::from_be(&e.at(4)?.word()?);
        if same(&seller, &c.listing.order.seller_key) && same(&fund, &c.listing.order.seller_fund) && same(&token, &c.listing.token) && same(&instance, &c.instance) && amount == need {
            return Ok(ok(format!("这一笔对实例发了 Opened;逻辑合约发了 Pulled,押金 {}", need.to_dec())));
        }
    }
    Ok(no(format!("这一笔对实例发了 Opened;逻辑合约没有发 Pulled({}, {}, {}, 实例, {})", c.listing.order.seller_key, c.listing.order.seller_fund, c.listing.token, need.to_dec())))
}

fn seats<'a>(c: &Case, evs: &'a [Event]) -> R<Vec<(u64, String, &'a Event)>> {
    let case = b32("案号", &c.case_id)?;
    let mut out = Vec::new();
    for e in evs.iter().filter(|e| e.ev.name == "Seated") {
        if e.ev.at(1)?.word()? == case {
            out.push((e.ev.at(2)?.small()?, hex(&e.ev.at(0)?.addr()?), e));
        }
    }
    Ok(out)
}

fn current_judges(c: &Case, s: &Seen, tx: &Tx) -> R<Vec<String>> {
    Ok(seats(c, &s.evs)?.into_iter().map(|(_, j, _)| current_at(s, &j, pos(tx))).collect())
}

fn accept<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen, tx: &Tx) -> R<Said> {
    let f = &a1::sol::JUDGE_FACE[0];
    Ok(match read::call_in(t, table, tx, &c.instance, &f.selector())? {
        Caller::Own { from, input } | Caller::Inner { from, input } => {
            let v = a1::abi::decode(&a1::abi::args_of(f.sig)?, input.get(4..).unwrap_or_default())?;
            let n = v[1].small()?;
            let who: Vec<String> = seats(c, &s.evs)?.into_iter().filter(|(k, _, _)| *k == n).map(|(_, j, _)| current_at(s, &j, pos(tx))).collect();
            if who.iter().any(|j| same(j, &from)) {
                ok(format!("这一笔对实例调了 accept(第 {n} 席,调用者 {from} 即该席判官那一刻的现行钥)"))
            } else {
                no(format!("这一笔对实例的 accept(第 {n} 席)由 {from} 调,该席判官那一刻的现行钥是 [{}]", who.join(", ")))
            }
        }
        Caller::Missing => no("这一笔里没有对实例未失败的 accept 调用".into()),
        Caller::Untraced => untraced("这一笔对实例的 accept 不是交易自己那一次调用".into(), WHY_UNTRACED),
    })
}

fn kept<T: Transport>(t: &T, table: &Table, c: &Case, tx: &Tx, f: &a1::sol::Fun, callers: Option<&[String]>) -> R<Said> {
    let name = f.name;
    Ok(match read::call_in(t, table, tx, &c.instance, &f.selector())? {
        Caller::Own { from, .. } | Caller::Inner { from, .. } => match callers {
            Some(js) if !js.iter().any(|j| same(j, &from)) => no(format!("这一笔对实例的 {name} 由 {from} 调,本案各席判官那一刻的现行钥是 [{}]", js.join(", "))),
            _ => ok(format!("这一笔对实例调了 {name}(调用者 {from})")),
        },
        Caller::Missing => no(format!("这一笔里没有对实例未失败的 {name} 调用")),
        Caller::Untraced => untraced(format!("这一笔对实例的 {name} 不是交易自己那一次调用"), WHY_UNTRACED),
    })
}

fn resign_row(evs: &[Event]) -> R<Said> {
    for e in evs.iter().filter(|e| e.ev.name == "Resigned") {
        let form = e.ev.at(1)?.small()?;
        if form <= 2 {
            return Ok(if evs.iter().any(|a| a.ev.name == "Anchored") {
                ok(format!("这一笔对实例发了 Resigned(形 {form})与它的 Anchored"))
            } else {
                no(format!("这一笔发了 Resigned(形 {form}),没有它的 Anchored"))
            });
        }
    }
    Ok(no(format!("{};没有形 0、1、2 的 Resigned", emitted(evs))))
}

fn expire_row<T: Transport>(t: &T, table: &Table, c: &Case, row: &Row) -> R<Said> {
    let mut which = Vec::new();
    if row.timestamp > c.listing.order.expiry {
        which.push(format!("要约 expiry {} 已过", c.listing.order.expiry));
    }
    if row.timestamp > c.receipt.open_by {
        which.push(format!("openBy {} 已过", c.receipt.open_by));
    }
    let topics = [Some(topic(&c.listing.order.seller_key)?), Some(c.listing.terms_hash.to_ascii_lowercase())];
    if read::logic_logs(t, table, &c.logic, &keys_of(c), &a1::sol::REVOKED_EV.topic0(), &topics)?.iter().any(|l| l.block <= row.block) {
        which.push("卖家已撤单(Revoked)".into());
    }
    Ok(if which.is_empty() {
        no(format!("这一笔对实例发了 Refunded,因 2;而块时刻 {} 未过要约 expiry 与 openBy,也没有不晚于这一块的撤单", row.timestamp))
    } else {
        ok(format!("这一笔对实例发了 Refunded,因 2:{}", which.join("、")))
    })
}

fn cancel_row<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen, tx: &Tx, evs: &[Event]) -> R<Said> {
    let mut four = false;
    for e in evs.iter().filter(|e| e.ev.name == "Refunded") {
        four |= cause_of(e)? == u64::from(lit::CAUSE_BUYER_CANCEL);
    }
    let mut closed = false;
    for e in evs.iter().filter(|e| e.ev.name == "Closed") {
        closed |= e.ev.at(0)?.small()? == 0;
    }
    if !four || !closed {
        return Ok(no(format!("{};撤单要 Refunded(4) 与终端 Refunded 的 Closed", emitted(evs))));
    }
    let buyer = current_at(s, &c.receipt.buyer_key, pos(tx));
    let cancel = s.fun(a1::sol::CANCEL);
    Ok(match read::call_in(t, table, tx, &c.instance, &cancel.selector())? {
        Caller::Own { from, input } | Caller::Inner { from, input } => {
            if same(&from, &buyer) {
                ok(format!("cancel 的调用者 {from} 就是 buyerKey 那一刻的现行钥"))
            } else {
                let v = a1::abi::decode(&a1::abi::args_of(cancel.sig)?, input.get(4..).unwrap_or_default())?;
                let slot = v[1].bytes()?;
                let d = k2::derive::digest(c.chain_id, &c.instance, &k2::derive::Action::Cancel.struct_hash());
                match a3::scan::sig_value(slot).ok().map(|sv| k2::sig::verify_buyer(&d, &sv, &buyer)) {
                    Some(Ok(_)) => ok(format!("cancel 由 {from} 递交,槽里是 buyerKey 那一刻的现行钥 {buyer} 对 Cancel() 的签名")),
                    _ => no(format!("cancel 的调用者 {from} 不是 buyerKey 那一刻的现行钥 {buyer},槽里也不是它对 Cancel() 的签名")),
                }
            }
        }
        Caller::Missing => no("这一笔里没有对实例未失败的 cancel 调用".into()),
        Caller::Untraced => untraced("这一笔对实例的 cancel 不是交易自己那一次调用".into(), WHY_UNTRACED),
    })
}

fn revoke_row(c: &Case, tx: &Tx) -> R<Said> {
    let t0 = a1::sol::REVOKED_EV.topic0();
    let th = b32("terms.hash", &c.listing.terms_hash)?;
    for l in tx.logs.iter().filter(|l| a1::consent::from_logic(&l.address, &c.logic) && l.topics.first() == Some(&t0) && l.topics.len() == 3) {
        if same(&l.topic_address(1)?, &c.listing.order.seller_key) && l.topics[2] == th {
            return Ok(ok(format!("逻辑合约发了 Revoked({}, {})", c.listing.order.seller_key, c.listing.terms_hash)));
        }
    }
    Ok(no(format!("这一笔里逻辑合约没有发 Revoked({}, {})", c.listing.order.seller_key, c.listing.terms_hash)))
}

fn redraw_row<T: Transport>(t: &T, table: &Table, c: &Case, tx: &Tx, evs: &[Event]) -> R<Said> {
    let call = kept(t, table, c, tx, &a1::sol::REDRAW, None)?;
    if call.0 != State::Pass {
        return Ok(call);
    }
    if let Some(e) = evs.iter().find(|e| e.ev.name == "Seated") {
        return Ok(ok(format!("这一笔对实例调了 redraw,发了 Seated(序 {})", e.ev.at(2)?.small()?)));
    }
    let mut form3 = false;
    for e in evs.iter().filter(|e| e.ev.name == "Resigned") {
        form3 |= e.ev.at(1)?.small()? == 3;
    }
    let mut closed = false;
    for e in evs.iter().filter(|e| e.ev.name == "Closed") {
        closed |= e.ev.at(0)?.small()? == 4;
    }
    Ok(if form3 && closed { ok("这一笔对实例调了 redraw:种子块哈希已不可得,Resigned(形 3)与终端 Resigned 的 Closed".into()) } else { no(format!("这一笔调了 redraw,{}", emitted(evs))) })
}

fn fund_bind_row(c: &Case, s: &Seen, tx: &Tx) -> R<Said> {
    let t0 = a1::sol::FUND_BOUND_EV.topic0();
    for l in tx.logs.iter().filter(|l| a1::consent::from_logic(&l.address, &c.logic) && l.topics.first() == Some(&t0) && l.topics.len() == 3) {
        if same(&l.topic_address(1)?, &c.listing.order.seller_key) && same(&l.topic_address(2)?, &c.listing.order.seller_fund) {
            return Ok(match &s.opened {
                Some(o) if (l.block, l.log_index) >= (o.log.block, o.log.log_index) => no(format!("FundBound 在第 {} 块第 {} 条,不早于 Opened(第 {} 块第 {} 条)", l.block, l.log_index, o.log.block, o.log.log_index)),
                _ => ok(format!("逻辑合约发了 FundBound({}, {})", c.listing.order.seller_key, c.listing.order.seller_fund)),
            });
        }
    }
    Ok(no(format!("这一笔里逻辑合约没有发 FundBound({}, {})", c.listing.order.seller_key, c.listing.order.seller_fund)))
}

fn pairs(c: &Case, settle: &Option<Settle>) -> (Vec<[u8; 32]>, [u8; 32], Option<[u8; 32]>) {
    let offers = (0..c.listing.candidate_count()).map(|i| c.listing.candidate_offer(i).offer_digest(c.chain_id, &c.logic)).collect();
    let sd = settle.as_ref().and_then(|s| s.args.as_ref()).map(|a| a.digest);
    (offers, c.bound.order_digest, sd)
}

fn consent_row(c: &Case, settle: &Option<Settle>, tx: &Tx) -> R<Said> {
    let buyer = c.receipt.buyer_key.as_str();
    let seller = c.listing.order.seller_key.as_str();
    let ours: Vec<(String, [u8; 32])> = read::consents_in(tx, &c.logic)?.into_iter().filter(|(w, _)| same(w, buyer) || same(w, seller)).collect();
    if ours.is_empty() {
        return Ok(no("这一笔里逻辑合约没有发买家钥或卖家钥的 Consented".into()));
    }
    let (offers, order, sd) = pairs(c, settle);
    let mut crossed = Vec::new();
    for (who, d) in &ours {
        let is_offer = offers.contains(d);
        let is_order = order == *d;
        if same(who, seller) && is_offer {
            return Ok(ok(format!("Consented({who}, {}):卖家对候选的要约摘要", hex(d))));
        }
        if same(who, buyer) && is_order {
            return Ok(ok(format!("Consented({who}, {}):买家对订单摘要", hex(d))));
        }
        if sd.is_some_and(|x| x == *d) {
            return Ok(ok(format!("Consented({who}, {}):对发出 Settled 的那一次 settle 的和解摘要", hex(d))));
        }
        if is_offer || is_order {
            crossed.push(format!("({who}, {})", hex(d)));
        }
    }
    if !crossed.is_empty() {
        return Ok(no(format!("Consented {}:摘要属另一把钥的那一对,不在任何一对之下", crossed.join(", "))));
    }
    let shown: Vec<String> = ours.iter().map(|(w, d)| format!("({w}, {})", hex(d))).collect();
    let said = format!("Consented {}", shown.join(", "));
    Ok(match settle {
        Some(Settle { args: None, untraced: true, .. }) => untraced(said, WHY_UNTRACED),
        _ => (State::Unread, said, Some(WHY_OTHER_DIGEST.to_string())),
    })
}

#[derive(Clone, Debug)]
struct Call {
    tx: String,
    at: (u64, u64),
    caller: Caller,
    init: Option<a1::order::Init>,
}

fn call_of<T: Transport>(t: &T, table: &Table, c: &Case, tx_hash: &str, f: &a1::sol::Fun, at: (u64, u64)) -> R<Call> {
    let tx = read::tx(t, table, tx_hash)?.ok_or_else(|| Fault::Form(format!("发出事件的交易 {tx_hash} 读不到")))?;
    let caller = read::call_in(t, table, &tx, &c.instance, &f.selector())?;
    let init = match &caller {
        Caller::Own { input, .. } | Caller::Inner { input, .. } if f.name == "init" => a1::order::decode_init(input).ok(),
        _ => None,
    };
    Ok(Call { tx: tx.hash, at, caller, init })
}

#[derive(Clone, Debug)]
struct SettleArgs {
    sig_b: Vec<u8>,
    sig_s: Vec<u8>,
    digest: [u8; 32],
}

#[derive(Clone, Debug)]
struct Settle {
    at: (u64, u64),
    caller: Caller,
    args: Option<SettleArgs>,
    untraced: bool,
}

fn settle_call<T: Transport>(t: &T, table: &Table, c: &Case, evs: &[Event]) -> R<Option<Settle>> {
    let Some(e) = evs.iter().find(|e| e.ev.name == "Settled") else { return Ok(None) };
    let call = call_of(t, table, c, &e.log.tx, &a1::sol::SETTLE, (e.log.block, e.log.log_index))?;
    let args = match &call.caller {
        Caller::Own { input, .. } | Caller::Inner { input, .. } => {
            let v = a1::abi::decode(&a1::abi::args_of(a1::sol::SETTLE.sig)?, input.get(4..).unwrap_or_default())?;
            let h = k2::derive::Action::Settle { buyer_bps: v[0].small()?, expiry: k2::u256::U256::from_be(&v[1].word()?) }.struct_hash();
            let digest = k2::derive::digest(c.chain_id, &c.instance, &h);
            Some(SettleArgs { sig_b: v[2].bytes()?.to_vec(), sig_s: v[3].bytes()?.to_vec(), digest })
        }
        _ => None,
    };
    let untraced = call.caller == Caller::Untraced;
    Ok(Some(Settle { at: call.at, caller: call.caller, args, untraced }))
}

#[allow(clippy::too_many_arguments)]
fn by_call<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen, item: Name, law: &'static str, what: &str, caller: &Caller, party: &str, d: &[u8; 32], at: (u64, u64)) -> R<Item> {
    if let Caller::Own { from, .. } | Caller::Inner { from, .. } = caller {
        let now = current_at(s, party, at);
        if same(from, &now) {
            let how = if matches!(caller, Caller::Own { .. }) { "交易自己那一次调用" } else { "内层调用,按调用追踪取" };
            return Ok(Item::pass(item, law, format!("{what} 的调用者 {from} 就是 {party} 那一刻的现行钥({how})")));
        }
    }
    let prior: Vec<read::Consent> = read::consented(t, table, &c.logic, &keys_of(c), party, d)?.into_iter().filter(|x| (x.block, x.log_index) < at).collect();
    if let Some(p) = prior.first() {
        return Ok(Item::pass(item, law, format!("逻辑合约在第 {} 块第 {} 条发了 Consented({party}, {}),早于 {what}", p.block, p.log_index, hex(d))));
    }
    Ok(match caller {
        Caller::Own { from, .. } | Caller::Inner { from, .. } => Item::mismatch(item, law, format!("{what} 的调用者是 {from},不是 {party} 那一刻的现行钥;逻辑合约在它之前也没有发 Consented({party}, {})", hex(d))),
        Caller::Missing => Item::mismatch(item, law, format!("交易里找不到发出它的那一次 {what};逻辑合约在它之前也没有发 Consented({party}, {})", hex(d))),
        Caller::Untraced => {
            let (state, read, why) = untraced(format!("{what} 不是交易自己那一次调用,调用者未读;逻辑合约在它之前没有发 Consented({party}, {})", hex(d)), WHY_UNTRACED);
            Item { item, law, row: None, state, read, why }
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn by_sig(s: &Seen, item: Name, law: &'static str, what: &str, slot: &[u8], party: &str, d: &[u8; 32], buyer: bool, at: (u64, u64)) -> Item {
    let now = current_at(s, party, at);
    let verified = if buyer {
        a3::scan::sig_value(slot).ok().is_some_and(|v| k2::sig::verify_buyer(d, &v, &now).is_ok())
    } else {
        k2::sig::verify_seller(d, &hex(slot), &now).is_ok()
    };
    if verified {
        Item::pass(item, law, format!("{what} 的槽里是 {party} 那一刻的现行钥 {now} 对 {} 的签名", hex(d)))
    } else {
        Item::mismatch(item, law, format!("{what} 的槽带 {} 字节,验不成 {party} 那一刻的现行钥 {now} 对 {} 的签名", slot.len(), hex(d)))
    }
}

fn opened_parties<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen) -> R<Vec<Item>> {
    let (Some(op), Some(call)) = (&s.opened, &s.init) else { return Ok(Vec::new()) };
    let at = (op.log.block, op.log.log_index);
    let seller_d = c.listing.candidate_offer(c.receipt.candidate as usize).offer_digest(c.chain_id, &c.logic);
    let parties = [
        (Name::OpenedBuyer, c.receipt.buyer_key.as_str(), c.bound.order_digest, c.buyer_consent == k2::sig::Form::Call),
        (Name::OpenedSeller, c.listing.order.seller_key.as_str(), seller_d, c.seller_consent == k2::sig::Form::Call),
    ];
    let mut out = Vec::new();
    for (item, party, d, by_call_form) in parties {
        let is_buyer = item == Name::OpenedBuyer;
        match &call.init {
            Some(init) => {
                let slot = if is_buyer { &init.sig_buyer } else { &init.sig_seller };
                out.push(if slot.is_empty() { by_call(t, table, c, s, item, L_OPENED, "init", &call.caller, party, &d, at)? } else { by_sig(s, item, L_OPENED, "init", slot, party, &d, is_buyer, at) });
            }
            None if by_call_form => out.push(by_call(t, table, c, s, item, L_OPENED, "init", &call.caller, party, &d, at)?),
            None => {}
        }
    }
    if !c.listing.order.buyer_may_open {
        let seller_key = current_at(s, &c.listing.order.seller_key, at);
        out.push(match &call.caller {
            Caller::Own { from, .. } | Caller::Inner { from, .. } if same(from, &seller_key) => Item::pass(Name::OpenedCaller, L_CALLER, format!("init 的调用者 {from} 就是 sellerKey 那一刻的现行钥")),
            Caller::Own { from, .. } | Caller::Inner { from, .. } => Item::mismatch(Name::OpenedCaller, L_CALLER, format!("buyerMayOpen 为假,init 的调用者却是 {from},不是 sellerKey 那一刻的现行钥 {seller_key}")),
            Caller::Missing => Item::mismatch(Name::OpenedCaller, L_CALLER, "交易里找不到发出 Opened 的那一次 init".into()),
            Caller::Untraced => {
                let (state, read, why) = untraced("init 不是交易自己那一次调用,调用者未读".into(), WHY_UNTRACED);
                Item { item: Name::OpenedCaller, law: L_CALLER, row: None, state, read, why }
            }
        });
    }
    Ok(out)
}

fn settled<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen) -> R<Vec<Item>> {
    let Some(st) = &s.settle else { return Ok(Vec::new()) };
    let buyer = c.receipt.buyer_key.as_str();
    let seller = c.listing.order.seller_key.as_str();
    let Some(a) = &st.args else {
        let mut out = Vec::new();
        for item in [Name::SettledBuyer, Name::SettledSeller] {
            out.push(if st.untraced {
                let (state, read, why) = untraced("发出 Settled 的那一次 settle 不是交易自己那一次调用,调用者与参数未读,不造摘要".into(), WHY_UNTRACED);
                Item { item, law: L_SETTLED, row: None, state, read, why }
            } else {
                Item::mismatch(item, L_SETTLED, "交易里找不到发出 Settled 的那一次 settle 调用".into())
            });
        }
        return Ok(out);
    };
    let d = a.digest;
    let mut out = Vec::new();
    for (item, party, slot, is_buyer) in [(Name::SettledBuyer, buyer, &a.sig_b, true), (Name::SettledSeller, seller, &a.sig_s, false)] {
        out.push(if slot.is_empty() { by_call(t, table, c, s, item, L_SETTLED, "settle", &st.caller, party, &d, st.at)? } else { by_sig(s, item, L_SETTLED, "settle", slot, party, &d, is_buyer, st.at) });
    }
    Ok(out)
}

fn seat_matches(judge: &str, want_judge: &str) -> bool {
    same(judge, want_judge)
}


fn seated<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen) -> R<Vec<Item>> {
    let (Some(_), Some(want)) = (&s.opened, c.seat.judge.as_deref()) else { return Ok(Vec::new()) };
    let seen = seats(c, &s.evs)?;
    let hit = seen.iter().any(|(n, j, _)| *n == c.seat.seat && seat_matches(j, want));
    let shown: Vec<String> = seen.iter().map(|(n, j, _)| format!("(序 {n}, {j})")).collect();
    let ev = if hit {
        Item::pass(Name::SeatedEvent, L_SEAT, format!("实例为第 {} 席发了 Seated,判官 {want}", c.seat.seat))
    } else {
        Item::mismatch(Name::SeatedEvent, L_SEAT, format!("卷宗写第 {} 席判官 {want};实例发的 Seated:[{}]", c.seat.seat, shown.join(", ")))
    };
    let case = b32("案号", &c.case_id)?;
    let of = match a1::chain::seat_of(t, table, &c.instance, &case, c.seat.seat) {
        Err(a1::fault::Fault::Chain { token, detail }) if eravon_chain::fault::reverted_word(&token, &detail) => {
            Item::mismatch(Name::SeatedSeatOf, L_SEAT, format!("seatOf({}, {}) 读不出:{detail}", c.case_id, c.seat.seat))
        }
        Err(e) => return Err(e.into()),
        Ok(r) => {
            let v = r.value;
            use a1::chain::seat_col as k;
            let judge = hex(&v.get(k::JUDGE).ok_or_else(|| Fault::Form("seatOf 没有判官那一栏".into()))?.addr()?);
            let amanah = hex(&v.get(k::AMANAH_HASH).ok_or_else(|| Fault::Form("seatOf 没有 amanahHash".into()))?.word()?);
            let quote = hex(&v.get(k::QUOTE_HASH).ok_or_else(|| Fault::Form("seatOf 没有 quoteHash".into()))?.word()?);
            let mut off = Vec::new();
            if !same(&judge, want) {
                off.push(format!("判官 {judge}"));
            }
            if !same(&amanah, &c.seat.amanah_hash) {
                off.push(format!("amanahHash {amanah}"));
            }
            if !same(&quote, &c.seat.quote_hash) {
                off.push(format!("quoteHash {quote}"));
            }
            if off.is_empty() {
                Item::pass(Name::SeatedSeatOf, L_SEAT, format!("seatOf 答判官 {judge}、amanahHash {amanah}、quoteHash {quote}"))
            } else {
                Item::mismatch(Name::SeatedSeatOf, L_SEAT, format!("seatOf 答的与卷宗的席位不合:{}", off.join("、")))
            }
        }
    };
    Ok(vec![ev, of])
}

fn anchored<'a>(c: &Case, s: &Seen, evs: &'a [Event]) -> Option<&'a Event> {
    let entry = c.entry.as_ref()?;
    let id = a1::order::b32_of("entryId", c.seat.entry_id.as_deref()?).ok()?;
    let judge = c.seat.judge.as_deref().unwrap_or("");
    evs.iter().filter(|e| e.ev.name == "Anchored").find(|e| {
        let now = current_at(s, judge, (e.log.block, e.log.log_index));
        let sender_ok = e.log.topic_address(1).is_ok_and(|x| same(&x, &now));
        let word = e.log.topics.get(2).copied().unwrap_or_default();
        sender_ok && anchor_matches(&word, &e.log.data, &id, &entry.bytes)
    })
}

fn beyond(c: &Case, s: &Seen, anchored_tx: Option<&str>) -> R<Vec<Item>> {
    let mut found: Vec<(&Event, String)> = Vec::new();
    for (n, j, e) in seats(c, &s.evs)? {
        if c.seat.judge.is_none() || n > c.seat.seat {
            found.push((e, format!("Seated(序 {n}, 判官 {j})")));
        }
    }
    if c.seat.entry_id.is_none() {
        if let Some(j) = c.seat.judge.as_deref() {
            for e in s.evs.iter().filter(|e| e.ev.name == "Anchored" && e.log.topic_address(1).is_ok_and(|x| same(&x, &current_at(s, j, (e.log.block, e.log.log_index))))) {
                found.push((e, format!("判官 {j} 的 Anchored")));
            }
        }
    }
    if c.terminal.is_none() {
        for e in s.evs.iter().filter(|e| e.ev.name == "Closed") {
            found.push((e, "Closed".into()));
        }
    }
    let mut ours: BTreeSet<String> = c.rows.iter().map(|r| r.tx.to_ascii_lowercase()).collect();
    ours.insert(c.posted.tx.to_ascii_lowercase());
    ours.extend(anchored_tx.map(str::to_string));
    Ok(found
        .into_iter()
        .map(|(e, what)| {
            let at = format!("{what}:交易 {} 第 {} 块", e.log.tx, e.log.block);
            if ours.contains(&e.log.tx.to_ascii_lowercase()) {
                Item::mismatch(Name::Beyond, L_BEYOND, format!("{at},这一笔是卷宗里的一笔,卷宗却没写它"))
            } else {
                Item::unread(Name::Beyond, L_BEYOND, at, WHY_BEYOND.into())
            }
        })
        .collect())
}

fn panel<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen) -> R<Item> {
    let g = c.receipt.candidate as usize - c.listing.judges.len();
    let guild = c.listing.guilds.get(g).map(|x| x.address.clone()).ok_or_else(|| Fault::Form("回执选的行会候选在清单里没有".into()))?;
    let seq = *c.listing.guild_seqs.get(g).ok_or_else(|| Fault::Form("载荷的行会元素没有 seq".into()))?;
    let want = c.listing.guild_panels.get(g).cloned().unwrap_or_default();
    let Some(op) = &s.opened else { return Ok(Item::mismatch(Name::PanelFrozen, L_PANEL, "实例没有开案".into())) };
    if s.generation.is_past() {
        return Ok(Item::unread(Name::PanelFrozen, L_PANEL, format!("实例是往代 {} 的", s.generation.row().logic), "往代的名单簿每个行会一本、冻结时不记根,与现行名单簿无从比".into()));
    }
    let at = (op.log.block, op.log.log_index);
    let root = root_at(s, &guild, at);
    let binds = match a1::chain::book_panel_at(t, table, eravon_literals::constants::PANEL, &root, &b32("proxyCodeHash", &c.proxy_code_hash)?, seq) {
        Ok(r) => {
            let expires = r.value.get(a1::chain::panel_col::EXPIRES).map(|v| v.small()).transpose()?.unwrap_or(0);
            let recorded = r.value.get(a1::chain::panel_col::RECORDED).map(|v| v.small()).transpose()?.unwrap_or(u64::MAX);
            recorded < op.log.block && expires >= a3::scan::block_stamp(t, table, op.log.block)?
        }
        Err(e) if a1::chain::reverted(&e) => false,
        Err(e) => return Err(e.into()),
    };
    let mut seen = Vec::new();
    for e in s.evs.iter().filter(|e| e.ev.name == "PanelFrozen") {
        let addr = hex(&e.ev.at(0)?.addr()?);
        let froze = hex(&e.ev.at(1)?.addr()?);
        let index = e.ev.at(2)?.small()?;
        let ph = hex(&e.ev.at(3)?.word()?);
        if same(&addr, &guild) && same(&froze, &root) && (!binds || index >= seq) && (index != seq || same(&ph, &want)) {
            return Ok(Item::pass(Name::PanelFrozen, L_PANEL, format!("PanelFrozen:行会 {addr}、根 {froze}、名单序 {index}(候选的 seq {seq})、panelHash {ph}")));
        }
        seen.push(format!("(行会 {addr}, 根 {froze}, 序 {index}, panelHash {ph})"));
    }
    Ok(Item::mismatch(Name::PanelFrozen, L_PANEL, format!("候选是行会 {guild}(开案时的根 {root})、seq {seq}、panelHash {want};实例发的 PanelFrozen:[{}]", seen.join(", "))))
}

fn amounts_match(got: &[String], want: &[String; 4]) -> bool {
    got == want
}


fn terminal(term: &Terminal, evs: &[Event]) -> R<Item> {
    let Some(e) = evs.iter().find(|e| e.ev.name == "Closed") else {
        return Ok(Item::mismatch(Name::Terminal, L_TERMINAL, format!("卷宗写终端 {},实例没有发 Closed", term.name)));
    };
    let b = e.ev.at(0)?.small()?;
    let name = usize::try_from(b).ok().and_then(|i| lit::TERMINAL_NAMES.get(i)).copied().unwrap_or("?");
    let mut got = Vec::new();
    for i in 1..5 {
        got.push(k2::u256::U256::from_be(&e.ev.at(i)?.word()?).to_dec());
    }
    let said = format!("Closed:终端字节 {b}({name}),四数 {}", got.join(" / "));
    Ok(if name == term.name && amounts_match(&got, &term.amounts) {
        Item::pass(Name::Terminal, L_TERMINAL, said)
    } else {
        Item::mismatch(Name::Terminal, L_TERMINAL, format!("{said};卷宗写 {}、{}", term.name, term.amounts.join(" / ")))
    })
}

fn anchor_matches(word: &[u8; 32], data: &[u8], entry_id: &[u8; 32], entry: &[u8]) -> bool {
    word == entry_id && k1::carried::anchor_bytes(data, word).is_some_and(|b| b == entry)
}


fn str_of<'a>(v: &'a k1::Value, k: &str) -> &'a str {
    v.get(k).and_then(k1::Value::as_str).unwrap_or_default()
}

fn entry_items(c: &Case, s: &Seen, audit: &Audit, registries: &[String]) -> R<Vec<Item>> {
    let (Some(entry), Some(want_id)) = (&c.entry, &c.seat.entry_id) else { return Ok(Vec::new()) };
    let judge = c.seat.judge.as_deref().unwrap_or("");
    let mut out = Vec::new();
    out.push(match anchored(c, s, &s.evs) {
        Some(e) => Item::pass(Name::Anchored, L_ANCHORED, format!("第 {} 块第 {} 条 Anchored:发送者 {} 是席位判官 {judge} 在那一笔位置的现行钥,词 {want_id},data 解出的字节等于条目档", e.log.block, e.log.log_index, e.log.topic_address(1).unwrap_or_default())),
        None => {
            let seen: Vec<String> = s.evs.iter().filter(|e| e.ev.name == "Anchored").map(|e| format!("({}, {})", e.log.topic_address(1).unwrap_or_default(), e.log.topic_hex(2).unwrap_or_default())).collect();
            Item::mismatch(Name::Anchored, L_ANCHORED, format!("实例没有发发送者是席位判官 {judge} 在那一笔位置的现行钥、词 {want_id}、data 即条目档的 Anchored;它发的:[{}]", seen.join(", ")))
        }
    });
    let regs = if registries.is_empty() { "(未声明)".to_string() } else { registries.join(", ") };
    let (root, how, l) = match audit {
        Audit::Read { root, how, ledger } => (root.clone(), how.clone(), ledger.as_ref()),
        Audit::Unread(why) => {
            out.push(Item::unread(Name::Ledger, L_LEDGER, format!("登记簿 {regs}"), format!("审计未重算:{why}")));
            out.push(Item::unread(Name::EntryJudge, L_LINEAGE, "需要审计出的账本".into(), format!("审计未重算:{why}")));
            out.push(Item::unread(Name::EntryAmanah, L_WORD, "需要审计出的账本".into(), format!("审计未重算:{why}")));
            out.extend(decision(c, entry, &s.evs)?);
            return Ok(out);
        }
    };
    let basis = audit.basis_text();
    let mut narrow = if l.narrow.is_empty() { String::new() } else { format!(";读窄处:{}", l.narrow.join(";")) };
    if !l.unchecked.is_empty() {
        narrow.push_str(&format!(";{} 条锚{}", l.unchecked.len(), eravon_scan::ledger::UNCHECKED));
    }
    out.push(Item::pass(Name::Ledger, L_LEDGER, format!("根 {root}({how}),登记簿 {regs},基底 {basis},标签 {}{narrow}", l.label())));
    let ej = str_of(&entry.value, "judge").to_string();
    out.push(if same(&ej, judge) {
        Item::pass(Name::EntryJudge, L_LINEAGE, format!("条目的 judge 就是席位判官 {judge}"))
    } else if l.lineage.iter().any(|k| same(k, &ej)) {
        Item::pass(Name::EntryJudge, L_LINEAGE, format!("条目的 judge {ej} 在以 {root} 为根的世系里(世系 {} 把钥)", l.lineage.len()))
    } else if let Some(why) = partial(registries, l) {
        Item::unread(Name::EntryJudge, L_LINEAGE, format!("条目的 judge {ej};以 {root} 为根读到的世系 {} 把钥", l.lineage.len()), format!("审计只读到账本的一部分:{why}"))
    } else {
        let keys: Vec<&str> = l.lineage.iter().map(String::as_str).collect();
        Item::mismatch(Name::EntryJudge, L_LINEAGE, format!("条目的 judge {ej} 不在以 {root} 为根的世系 [{}] 里", keys.join(", ")))
    });
    let named = entry.value.get("body").map(|b| str_of(b, "amanah")).unwrap_or_default().to_string();
    out.push(word(c, l, &named, &basis));
    out.extend(decision(c, entry, &s.evs)?);
    Ok(out)
}

fn partial(registries: &[String], l: &eravon_scan::ledger::Ledger) -> Option<String> {
    let mut why = Vec::new();
    if registries.is_empty() {
        why.push("未声明登记簿,登记簿上的锚不在审计的基里".to_string());
    }
    if !l.narrow.is_empty() {
        why.push(format!("读窄处:{}", l.narrow.join(";")));
    }
    if !l.unchecked.is_empty() {
        why.push(format!("{} 条锚{}", l.unchecked.len(), eravon_scan::ledger::UNCHECKED));
    }
    (!why.is_empty()).then(|| why.join(";"))
}

fn word(c: &Case, l: &eravon_scan::ledger::Ledger, named: &str, basis: &str) -> Item {
    if !c.pool() && named != c.seat.amanah_hash {
        return Item::mismatch(Name::EntryAmanah, L_WORD, format!("具名轨:条目的 amanah 是 {named},席位的 amanahHash 是 {}", c.seat.amanah_hash));
    }
    let held = l.entry(named).filter(|e| str_of(&e.value, "entryType") == k1::body::AMANAH);
    if held.is_none() {
        if let Some(d) = l.discarded_entry(named) {
            return Item::mismatch(Name::EntryAmanah, L_WORD, format!("条目点名的 amanah 条目 {named} {}(交易 {},块 {})", eravon_scan::ledger::DISCARDED_SAYS, d.tx, d.block));
        }
    }
    let Some(h) = held else {
        let why = if l.unchecked.is_empty() { format!("审计在基底 {basis} 之下没有它") } else { format!("审计在基底 {basis} 之下没有它;另有 {} 条锚{}", l.unchecked.len(), eravon_scan::ledger::UNCHECKED) };
        return Item::unread(Name::EntryAmanah, L_WORD, format!("条目点名的 amanah 条目 {named}"), why);
    };
    if !c.pool() {
        return Item::pass(Name::EntryAmanah, L_WORD, format!("具名轨:条目的 amanah 即席位的 amanahHash {named},那一条 amanah 条目在审计出的账本里"));
    }
    let text = h.value.get("body").map(|b| str_of(b, "text_md")).unwrap_or_default();
    let sha = hex(&k1::hash::sha256(text.as_bytes()));
    if same(&sha, &c.seat.amanah_hash) {
        Item::pass(Name::EntryAmanah, L_WORD, format!("池轨:amanah 条目 {named} 的 text_md 的 sha256 是 {sha},等于席位的 amanahHash"))
    } else {
        Item::mismatch(Name::EntryAmanah, L_WORD, format!("池轨:amanah 条目 {named} 的 text_md 的 sha256 是 {sha},席位的 amanahHash 是 {}", c.seat.amanah_hash))
    }
}


fn decision(c: &Case, entry: &crate::dossier::Entry, evs: &[Event]) -> R<Vec<Item>> {
    let Some(term) = &c.terminal else { return Ok(Vec::new()) };
    Ok(match term.name.as_str() {
        "Ruled" => {
            let said = entry.value.get("body").and_then(|b| b.get("decision")).and_then(|d| d.get("rung")).and_then(k2::form::as_int);
            let Some(e) = evs.iter().find(|e| e.ev.name == "Ruled") else {
                return Ok(vec![Item::mismatch(Name::EntryDecision, L_DECISION, "终端是 Ruled,实例没有发 Ruled".into())]);
            };
            let r = e.ev.at(0)?.small()?;
            vec![match said {
                Some(n) if n == r => Item::pass(Name::EntryDecision, L_DECISION, format!("条目的 decision.rung {n} 等于 Ruled 的 rung")),
                Some(n) => Item::mismatch(Name::EntryDecision, L_DECISION, format!("条目的 decision.rung 是 {n},Ruled 的 rung 是 {r}")),
                None => Item::mismatch(Name::EntryDecision, L_DECISION, format!("条目没有 decision.rung;Ruled 的 rung 是 {r}")),
            }]
        }
        "Resigned" => {
            let ty = str_of(&entry.value, "entryType");
            vec![if ty == k1::body::RESIGNATION {
                Item::pass(Name::EntryDecision, L_DECISION, "终端是 Resigned,条目是辞任".into())
            } else {
                Item::mismatch(Name::EntryDecision, L_DECISION, format!("终端是 Resigned,条目的 entryType 是 {ty}"))
            }]
        }
        _ => Vec::new(),
    })
}

fn complete<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen) -> R<Item> {
    let owed = owed(t, table, c, s)?;
    let rows: BTreeSet<String> = c.rows.iter().map(|r| r.tx.to_ascii_lowercase()).collect();
    let missing: Vec<&str> = owed.iter().filter(|x| !rows.contains(*x)).map(String::as_str).collect();
    if !missing.is_empty() {
        return Ok(Item::mismatch(Name::Complete, L_COMPLETE, format!("链上该成行的 {} 笔里,这 {} 笔不是任何一行的 tx:[{}]", owed.len(), missing.len(), missing.join(", "))));
    }
    if matches!(&s.settle, Some(Settle { args: None, .. })) {
        return Ok(Item::unread(Name::Complete, L_COMPLETE, format!("链上该成行的 {} 笔都在卷宗行里", owed.len()), "settle 调用未读:两把钥下其余摘要的 Consented 未计".into()));
    }
    Ok(Item::pass(Name::Complete, L_COMPLETE, format!("链上该成行的 {} 笔都是卷宗某一行的 tx", owed.len())))
}

fn owed<T: Transport>(t: &T, table: &Table, c: &Case, s: &Seen) -> R<BTreeSet<String>> {
    let mut owed: BTreeSet<String> = s.evs.iter().map(|e| e.log.tx.to_ascii_lowercase()).collect();
    let transfer = [Some(hex(&a1::sol::ERC20_TRANSFER_EV.topic0())), None, Some(topic(&c.instance)?)];
    owed.extend(read::logs_of(t, table, &c.listing.token, &transfer, 0)?.into_iter().map(|l| l.tx.to_ascii_lowercase()));
    owed.insert(c.posted.tx.to_ascii_lowercase());
    let buyer = c.receipt.buyer_key.as_str();
    let seller = c.listing.order.seller_key.as_str();
    let (_, order, sd) = pairs(c, &s.settle);
    let chosen = c.listing.candidate_offer(c.receipt.candidate as usize).offer_digest(c.chain_id, &c.logic);
    let mut wanted = vec![(seller.to_string(), chosen), (buyer.to_string(), order)];
    if let Some(d) = sd {
        wanted.push((seller.to_string(), d));
        wanted.push((buyer.to_string(), d));
    }
    for (who, d) in &wanted {
        owed.extend(read::consented(t, table, &c.logic, &keys_of(c), who, d)?.into_iter().map(|x| x.tx.to_ascii_lowercase()));
    }
    let pulled = [Some(topic(seller)?), Some(topic(&c.listing.order.seller_fund)?)];
    for l in read::logic_logs(t, table, &c.logic, &keys_of(c), &a1::sol::PULLED_EV.topic0(), &pulled)? {
        let names_us = a1::event::decode(&l.topics, &l.data).ok().and_then(|e| e.at(3).ok().and_then(|v| v.addr().ok())).is_some_and(|a| same(&hex(&a), &c.instance));
        if names_us {
            owed.insert(l.tx.to_ascii_lowercase());
        }
    }
    let mut expiry_branch = None;
    for e in s.evs.iter().filter(|e| e.ev.name == "Refunded") {
        if cause_of(e)? == u64::from(lit::CAUSE_EXPIRY) {
            expiry_branch = Some(e.log.block);
        }
    }
    if let Some(b) = expiry_branch {
        let topics = [Some(topic(seller)?), Some(c.listing.terms_hash.to_ascii_lowercase())];
        owed.extend(read::logic_logs(t, table, &c.logic, &keys_of(c), &a1::sol::REVOKED_EV.topic0(), &topics)?.into_iter().filter(|l| l.block <= b).map(|l| l.tx.to_ascii_lowercase()));
    }
    Ok(owed)
}
