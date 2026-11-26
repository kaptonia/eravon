use eravon_dispute::view::{self, CaseView};
use eravon_scan::scan::{self as a3, Side};
use eravon_store::record::Kind;
use eravon_store::seam::Box_;
use eravon_store::settings::TokenRow;
use eravon_vault::tokens;
use eravon_watch::order::{self, Order, State};

use crate::status::{self, Status};
use crate::{Why, Wire, a3w, a5w, why};

#[derive(Clone, Debug)]
pub struct Row {
    pub instance: String,
    pub case_id: String,
    pub record: String,
    pub number: String,
    pub buyer: String,
    pub label: String,
    pub product: String,
    pub amount: String,
    pub token: String,
    pub status: Result<Status, Why>,
    pub deadline: u64,
    pub placed: u64,
    pub block: Option<u64>,
    pub chain: Option<CaseView>,
    pub local: Option<State>,
    pub handed: bool,
    pub note: String,
    pub terms_hash: String,
    pub decimals: Option<u64>,
    pub past: bool,
    pub logic: String,
    pub series: Option<eravon_terms::doc::Series>,
}

impl Row {
    pub fn wire<'a>(&'a self, w: &Wire<'a>) -> Wire<'a> {
        Wire { logic: if self.logic.is_empty() { w.logic } else { &self.logic }, ..*w }
    }
}

#[derive(Clone, Debug)]
pub struct Table {
    pub rows: Vec<Row>,
    pub foreign: u64,
    pub agreement: String,
    pub unscanned: Option<Why>,
}

impl Table {
    pub fn complete(self) -> Result<Table, Why> {
        match self.unscanned {
            Some(w) => Err(w),
            None => Ok(self),
        }
    }
}

pub fn number(n: &a3::OrderNo) -> String {
    format!("{}-{:03}", n.month.replace('-', ""), n.seq)
}

struct Local {
    instance: String,
    case_id: String,
    record: String,
    created: u64,
    order: Option<Order>,
    state: Option<State>,
    decimals: Option<u64>,
    until: u64,
    handed_by: Option<String>,
    past: bool,
    series: Option<eravon_terms::doc::Series>,
}

pub const CONFIRM_HANDED: &str = "confirmHanded";

pub const CONFIRM_BY: &str = "confirmBy";

pub fn hand_holds(w: &Wire<'_>, by: &str, instance: &str) -> Result<bool, Why> {
    eravon_watch::order::still_acting(w, by, instance)
}


fn hand_of(r: &eravon_store::record::Record, owner: &str) -> Option<String> {
    r.field(CONFIRM_HANDED).is_some_and(|x| !x.is_empty()).then(|| r.field(CONFIRM_BY).filter(|k| !k.is_empty()).unwrap_or(owner).to_ascii_lowercase())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandOut {
    pub record: String,
    pub instance: String,
    pub chain: u64,
    pub by: String,
}

pub fn hand_outs(data: &str, owners: &[String]) -> Result<Vec<HandOut>, Why> {
    let s = eravon_vault::store::open_read(data).map_err(a5w)?;
    let mut out = Vec::new();
    for o in owners {
        let rows = match s.list(o, Box_::Packs) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(a5w(e)),
        };
        for r in rows.into_iter().filter(|r| r.kind == Kind::OrderFile) {
            let Some(by) = hand_of(&r, o) else { continue };
            let chain = s.part(o, Box_::Packs, &r.id, "pack.json").ok().and_then(|p| eravon_link::pack::pack(&p).ok()).map_or(0, |p| p.listing.chain_id);
            out.push(HandOut { record: r.id.clone(), instance: r.field("instance").unwrap_or_default().to_ascii_lowercase(), chain, by });
        }
    }
    Ok(out)
}

fn in_force(w: &Wire<'_>, l: Option<&Local>) -> bool {
    l.and_then(|l| l.handed_by.as_deref().map(|k| (k, l.instance.as_str()))).is_some_and(|(k, at)| hand_holds(w, k, at).unwrap_or(true))
}

pub fn handed_on(data: &str, owners: &[String], instance: &str, w: &Wire<'_>) -> Result<bool, Why> {
    for o in owners {
        if let Some(by) = locals(data, o, Side::Buyer)?.into_iter().find(|l| l.instance.eq_ignore_ascii_case(instance)).and_then(|l| l.handed_by) {
            return hand_holds(w, &by, instance);
        }
    }
    Ok(false)
}

fn locals(data: &str, me: &str, side: Side) -> Result<Vec<Local>, Why> {
    let s = eravon_vault::store::open_read(data).map_err(a5w)?;
    let (b, kind) = match side {
        Side::Seller => (Box_::Orders, Kind::Receipt),
        Side::Buyer => (Box_::Packs, Kind::OrderFile),
    };
    let rows = match s.list(me, b) {
        Ok(r) => r,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(Vec::new()),
        Err(e) => return Err(a5w(e)),
    };
    let mut out = Vec::new();
    let net = eravon_literals::constants::current_network().standard.chain_id;
    for r in rows.into_iter().filter(|r| r.kind == kind) {
        let order = match side {
            Side::Seller => Some(Order::of(&r)?),
            Side::Buyer => None,
        };
        let pack = match (&order, side) {
            (None, Side::Buyer) => s.part(me, b, &r.id, "pack.json").ok().and_then(|p| eravon_link::pack::pack(&p).ok()),
            _ => None,
        };
        let until = pack.as_ref().map(open_until).unwrap_or(0);
        let listing = match (&order, side) {
            (Some(o), _) => eravon_watch::order::part(data, me, &o.id, &eravon_watch::order::part_listing()).ok().and_then(|b| eravon_link::listing::listing(&b).ok()),
            (None, Side::Buyer) => pack.map(|p| p.listing),
            (None, Side::Seller) => None,
        };
        if listing.as_ref().is_some_and(|l| l.chain_id != net) {
            continue;
        }
        let past = !eravon_literals::constants::past_rows().is_empty()
            && match (&order, side) {
                (Some(o), _) => eravon_watch::poll::made_past(data, me, o),
                (None, Side::Buyer) => s
                    .part(me, b, &r.id, "pack.json")
                    .ok()
                    .and_then(|p| eravon_link::pack::pack(&p).ok())
                    .is_some_and(|p| eravon_watch::poll::made_under(&p.listing.logic, "").1),
                (None, Side::Seller) => false,
            };
        out.push(Local {
            instance: order.as_ref().map(|o| o.instance.clone()).or_else(|| r.field("instance").map(str::to_string)).unwrap_or_default().to_ascii_lowercase(),
            case_id: order.as_ref().map(|o| o.case_id.clone()).or_else(|| r.field("caseId").map(str::to_string)).unwrap_or_default(),
            record: r.id.clone(),
            created: r.created,
            order,
            state: r.field(KEPT_STATE).filter(|_| side == Side::Buyer).and_then(ended_of),
            decimals: r.field(eravon_watch::units::FIELD).and_then(|x| x.parse().ok()),
            until,
            handed_by: if side == Side::Buyer { hand_of(&r, me) } else { None },
            past,
            series: listing.and_then(|l| l.terms.series).map(|x| eravon_terms::doc::Series { id: x.id.to_ascii_lowercase(), ..x }),
        });
    }
    Ok(out)
}

pub fn shown(past: bool, switch: bool) -> bool {
    !past || switch
}

fn union(locals: Vec<Local>, cases: &[a3::Case]) -> Vec<(Option<Local>, Option<a3::Case>)> {
    let mut out: Vec<(Option<Local>, Option<a3::Case>)> = Vec::new();
    for l in locals {
        let c = cases.iter().find(|c| c.instance.eq_ignore_ascii_case(&l.instance)).cloned();
        out.push((Some(l), c));
    }
    for c in cases {
        if !out.iter().any(|(_, x)| x.as_ref().is_some_and(|x| x.instance.eq_ignore_ascii_case(&c.instance))) {
            out.push((None, Some(c.clone())));
        }
    }
    out
}


fn numbered(pairs: &[(Option<Local>, Option<a3::Case>)]) -> Vec<(a3::Case, u64)> {
    pairs.iter().filter_map(|(_, c)| c.clone()).map(|c| (c.clone(), 0)).collect()
}


pub fn table(data: &str, me: &str, side: Side, w: &Wire<'_>) -> Result<Table, Why> {
    let owners = match side {
        Side::Seller => eravon_vault::store::archive_of(data, me).map_err(|e| (e.name().to_string(), e.detail()))?,
        Side::Buyer => vec![me.to_string()],
    };
    table_of(data, &owners, &keys_of(data, me, side)?, side, w)
}

pub fn keys_of(data: &str, me: &str, side: Side) -> Result<Vec<String>, Why> {
    match side {
        Side::Seller => eravon_vault::keyset::seller(data, me).map_err(|e| (e.name().to_string(), e.detail())),
        Side::Buyer => Ok(vec![me.to_string()]),
    }
}

pub fn record_at(data: &str, owners: &[String], side: Side, instance: &str) -> Result<Option<String>, Why> {
    for o in owners {
        if let Some(l) = locals(data, o, side)?.into_iter().find(|l| l.instance.eq_ignore_ascii_case(instance)) {
            return Ok(Some(l.record));
        }
    }
    Ok(None)
}

pub fn table_of(data: &str, owners: &[String], keys: &[String], side: Side, w: &Wire<'_>) -> Result<Table, Why> {
    table_for(data, owners, keys, side, w, None)
}

pub fn table_one(data: &str, me: &str, side: Side, w: &Wire<'_>, instance: &str) -> Result<Table, Why> {
    let owners = match side {
        Side::Seller => eravon_vault::store::archive_of(data, me).map_err(|e| (e.name().to_string(), e.detail()))?,
        Side::Buyer => vec![me.to_string()],
    };
    table_for(data, &owners, &keys_of(data, me, side)?, side, w, Some(instance))
}

pub fn table_for(data: &str, owners: &[String], keys: &[String], side: Side, w: &Wire<'_>, only: Option<&str>) -> Result<Table, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let me = owners.first().map(String::as_str).unwrap_or_default();
    let mut cases: Vec<a3::Case> = Vec::new();
    let (mut foreign, mut agreement) = (0u64, String::new());
    let mut unscanned: Option<Why> = None;
    match a3::by_keys(w.t, w.table, w.logic, side, keys, 0, eravon_chain::seam::Upto::Latest, &[]) {
        Ok(s) => {
            foreign += s.foreign;
            agreement = s.agreement.to_string();
            for c in s.cases {
                if !cases.iter().any(|x| x.instance.eq_ignore_ascii_case(&c.instance)) {
                    cases.push(c);
                }
            }
        }
        Err(e) => unscanned = Some(a3w(e)),
    }
    let sweep = a3::Sweep { cases, foreign, agreement: if agreement == "single" { "single" } else { "agreed" } };
    let mut mine = Vec::new();
    for o in owners {
        mine.extend(locals(data, o, side)?);
    }
    let pairs = union(mine, &sweep.cases);
    let mut stamps: Vec<(u64, u64)> = Vec::new();
    let mut refused: Option<Why> = None;
    a3::ahead_stamps(w.t, w.table, &sweep.cases.iter().map(|c| c.block).collect::<Vec<_>>());
    for c in &sweep.cases {
        if !stamps.iter().any(|(b, _)| *b == c.block) {
            match a3::block_stamp(w.t, w.table, c.block) {
                Ok(t) => stamps.push((c.block, t)),
                Err(e) if e.name() == eravon_chain::fault::Fault::Unread(String::new()).name() => {
                    refused.get_or_insert(a3w(e));
                }
                Err(e) => return Err(a3w(e)),
            }
        }
    }
    if let Some(e) = refused {
        return Err(e);
    }
    let stamp_of = |b: u64| stamps.iter().find(|(x, _)| *x == b).map(|(_, s)| *s).unwrap_or(0);
    let batch = numbered(&pairs);
    let cases: Vec<a3::Case> = batch.iter().map(|(c, _)| c.clone()).collect();
    let times: Vec<u64> = batch.iter().map(|(c, t)| if *t != 0 { *t } else { stamp_of(c.block) }).collect();
    let numbers = if unscanned.is_some() { Vec::new() } else { a3::order_numbers(&cases, &times).map_err(a3w)? };
    let mut pairs = pairs;
    if let Some(one) = only {
        let is = |l: &Option<Local>, c: &Option<a3::Case>| c.as_ref().map(|c| c.instance.as_str()).or_else(|| l.as_ref().map(|l| l.instance.as_str())).is_some_and(|i| i.eq_ignore_ascii_case(one));
        pairs.retain(|(l, c)| is(l, c));
    }
    let labels = crate::label::all(data, me)?;
    let inputs: Vec<(Option<State>, bool, String)> = pairs.iter().map(|(l, _)| local_inputs(l.as_ref(), side, w)).collect();
    let reads_chain = |l: &Option<Local>, c: &Option<a3::Case>, local: Option<State>| -> bool {
        c.is_some() || matches!(local, Some(State::Deployed) | Some(State::Expired)) || (side == Side::Buyer && l.is_some()) || (unscanned.is_some() && l.is_some())
    };
    let opened_of = |l: &Option<Local>, c: &Option<a3::Case>| c.as_ref().map(|c| c.block).or_else(|| l.as_ref().and_then(|l| l.order.as_ref().map(|o| o.opened))).unwrap_or(0);
    let instance_of = |l: &Option<Local>, c: &Option<a3::Case>| c.as_ref().map(|c| c.instance.to_ascii_lowercase()).or_else(|| l.as_ref().map(|l| l.instance.clone())).unwrap_or_default();
    let case_id_of = |l: &Option<Local>, c: &Option<a3::Case>| c.as_ref().map(|c| c.case_id.clone()).or_else(|| l.as_ref().map(|l| l.case_id.clone())).unwrap_or_default();
    let wanted: Vec<(String, u64)> = pairs.iter().zip(&inputs).filter(|((l, c), i)| reads_chain(l, c, i.0)).map(|((l, c), _)| (instance_of(l, c), opened_of(l, c))).filter(|(i, _)| !i.is_empty()).collect();
    let mut events: Vec<(String, Vec<view::Ev>)> = match eravon_chain::recall::current().is_none().then(|| view::events_of_many(w, &wanted)) {
        Some(Ok(evs)) => wanted.iter().map(|(i, _)| i.clone()).zip(evs).collect(),
        _ => Vec::new(),
    };
    let kept = crate::ended::known();
    let mut ended = crate::ended::Record::new();
    let reading: Vec<(String, String)> = pairs
        .iter()
        .zip(&inputs)
        .filter(|((l, c), i)| reads_chain(l, c, i.0))
        .map(|((l, c), _)| (instance_of(l, c), case_id_of(l, c)))
        .filter(|(i, _)| !i.is_empty() && !crate::ended::serves(&kept, w.chain_id, i, events.iter().find(|(x, _)| x == i).map(|(_, e)| e.as_slice())))
        .collect();
    view::ahead_cases(w, &reading);
    let row_reads: Vec<(String, String, bool, u64, Option<Vec<view::Ev>>)> = pairs
        .iter()
        .zip(&inputs)
        .map(|((l, c), (local, _, _))| {
            let instance = instance_of(l, c);
            let opened = c.as_ref().map(|c| c.block).or_else(|| l.as_ref().and_then(|l| l.order.as_ref().map(|o| o.opened))).unwrap_or(0);
            let held = events.iter().position(|(i, _)| *i == instance).map(|at| events.swap_remove(at).1);
            (instance, case_id_of(l, c), reads_chain(l, c, *local), opened, held)
        })
        .collect();
    let ctx = std::sync::Arc::new((w.held(), kept));
    let read_row = move |(instance, case_id, read_chain, opened, held): (String, String, bool, u64, Option<Vec<view::Ev>>)| {
        let mut new = crate::ended::Record::new();
        let read = if read_chain { crate::ended::case_of(&ctx.0.wire(), &ctx.1, &mut new, &instance, &case_id, opened, held) } else { Ok(None) };
        (read, new)
    };
    let mut reads: Vec<(Result<Option<CaseView>, Why>, crate::ended::Record)> =
        if eravon_chain::recall::current().is_none() { eravon_chain::read::side_by_side(row_reads, read_row) } else { row_reads.into_iter().map(&read_row).collect() };
    let mut rows = Vec::new();
    for (((l, c), (local, handed, note)), (read, new)) in pairs.into_iter().zip(inputs).zip(reads.drain(..)) {
        ended.extend(new);
        let instance = instance_of(&l, &c);
        let case_id = case_id_of(&l, &c);
        let chain = read.as_ref().ok().cloned().flatten();
        let buyer = c.as_ref().map(|c| c.buyer.clone()).or_else(|| l.as_ref().and_then(|l| l.order.as_ref().map(|o| o.buyer_key.clone()))).unwrap_or_default();
        let (product, terms_amount) = l.as_ref().and_then(|l| l.order.as_ref()).map(|o| terms_facts(data, me, o)).unwrap_or_default();
        let amount = chain.as_ref().filter(|v| v.seats > 0).map(|v| v.amount.to_dec()).unwrap_or(terms_amount);
        let token = chain.as_ref().filter(|v| !v.token.is_empty()).map(|v| v.token.clone()).or_else(|| l.as_ref().and_then(|l| l.order.as_ref().map(|o| o.token.clone()))).unwrap_or_default();
        let status = match &read {
            Err(e) if e.0 == eravon_chain::fault::Fault::Unread(String::new()).name() && matches!(local, Some(State::Cancelled | State::Expired)) => status::judge_noted(local, None, handed, &note),
            Err(e) => Err(e.clone()),
            Ok(_) => status::judge_noted(local, chain.as_ref(), handed, &note),
        };
        let deadline = deadline_of(status.as_ref().ok(), chain.as_ref(), l.as_ref().and_then(|l| l.order.as_ref())).max(l.as_ref().filter(|_| matches!(status.as_ref().map(|s| s.word), Ok(status::Word::AwaitPay | status::Word::AwaitDeploy))).map_or(0, |l| l.until));
        let past = chain.as_ref().is_some_and(|v| v.past) || l.as_ref().is_some_and(|l| l.past);
        let chain_logic = chain.as_ref().map(|v| v.logic.clone()).unwrap_or_else(|| w.logic.to_string());
        let number = numbers.iter().find(|n| n.instance.eq_ignore_ascii_case(&instance) && c.is_some()).map(number).unwrap_or_default();
        let placed = c.as_ref().map(|c| stamp_of(c.block)).or_else(|| l.as_ref().map(|l| l.created)).unwrap_or(0);
        let label = labels.iter().find(|(b, _)| b.eq_ignore_ascii_case(&buyer)).map(|(_, n)| n.clone()).unwrap_or_default();
        let terms_hash = l
            .as_ref()
            .and_then(|l| l.order.as_ref())
            .and_then(|o| order::part(data, me, &o.id, &order::part_receipt()).ok())
            .and_then(|b| eravon_link::receipt::receipt(&b).ok())
            .map(|r| r.terms_hash)
            .unwrap_or_default();
        rows.push(Row {
            instance,
            case_id,
            record: l.as_ref().map(|l| l.record.clone()).unwrap_or_default(),
            number,
            buyer,
            label,
            product,
            amount,
            token,
            status,
            deadline,
            placed,
            block: c.as_ref().map(|c| c.block),
            chain,
            local,
            handed,
            note,
            terms_hash,
            decimals: l.as_ref().and_then(|l| l.order.as_ref().and_then(eravon_watch::units::of).or(l.decimals)),
            past,
            logic: chain_logic,
            series: l.as_ref().and_then(|l| l.series.clone()),
        });
    }
    crate::ended::add(ended);
    rows.retain(|r| shown(r.past, eravon_literals::constants::past_switch()));
    rows.sort_by(|a, b| b.placed.cmp(&a.placed).then(b.case_id.cmp(&a.case_id)));
    let rows = group(rows);
    Ok(Table { rows, foreign: sweep.foreign, agreement: sweep.agreement.to_string(), unscanned })
}

fn local_inputs(l: Option<&Local>, side: Side, w: &Wire<'_>) -> (Option<State>, bool, String) {
    let local = l.and_then(|l| l.order.as_ref().map(|o| o.state).or(l.state)).or_else(|| (side == Side::Buyer && l.is_some()).then_some(State::Ready));
    let handed = in_force(w, l);
    let note = l.and_then(|l| l.order.as_ref()).map(|o| o.note.clone()).unwrap_or_default();
    (local, handed, note)
}

pub fn reread(data: &str, owner: &str, side: Side, w: &Wire<'_>, row: &mut Row) -> Result<(), Why> {
    reread_with(data, owner, side, w, row, None)
}

pub fn reread_many(data: &str, side: Side, w: &Wire<'_>, rows: &mut [(String, &mut Row)]) -> Vec<Result<(), Why>> {
    let wanted: Vec<(String, u64)> = rows.iter().map(|(_, r)| (r.instance.clone(), r.block.unwrap_or(0))).collect();
    let mut events: Vec<Option<Vec<view::Ev>>> = match (rows.len() > 1 && eravon_chain::recall::current().is_none()).then(|| view::events_of_many(w, &wanted)) {
        Some(Ok(evs)) => evs.into_iter().map(Some).collect(),
        _ => vec![None; rows.len()],
    };
    view::ahead_cases(w, &rows.iter().map(|(_, r)| (r.instance.clone(), r.case_id.clone())).collect::<Vec<_>>());
    rows.iter_mut().zip(events.iter_mut()).map(|((owner, row), evs)| reread_with(data, owner, side, w, row, evs.take())).collect()
}

fn reread_with(data: &str, owner: &str, side: Side, w: &Wire<'_>, row: &mut Row, events: Option<Vec<view::Ev>>) -> Result<(), Why> {
    let locals = locals(data, owner, side)?;
    let l = locals.iter().find(|l| l.instance.eq_ignore_ascii_case(&row.instance));
    let chain = view::case_if_deployed_with(w, &row.instance, &row.case_id, row.block.unwrap_or(0), events)?;
    (row.local, row.handed, row.note) = local_inputs(l, side, w);
    if let Some(v) = chain.as_ref() {
        row.past |= v.past;
        row.logic = v.logic.clone();
    }
    row.chain = chain;
    row.status = status_of(row, row.chain.as_ref());
    Ok(())
}

impl Row {
    pub fn unrecorded(&self) -> bool {
        self.record.is_empty()
    }
}

pub const KEPT_STATE: &str = "state";

pub fn ended_of(word: &str) -> Option<State> {
    [State::Expired, State::Cancelled].into_iter().find(|s| s.word() == word)
}

pub fn open_until(p: &eravon_link::pack::Pack) -> u64 {
    p.listing.order.expiry.min(p.receipt.open_by)
}

pub fn keep_ended(data: &str, writer: &str, owner: &str, record: &str, state: State, now: u64) -> Result<(), eravon_store::fault::Fault> {
    let st = eravon_vault::store::open_write(data, writer, now)?;
    let old = match st.get(owner, Box_::Packs, record) {
        Ok(x) => x.fields,
        Err(eravon_store::fault::Fault::Missing(_)) => Vec::new(),
        Err(e) => return Err(e),
    };
    let mut fields: Vec<(String, String)> = old.into_iter().filter(|(k, _)| k != KEPT_STATE).collect();
    fields.push((KEPT_STATE.into(), state.word().into()));
    st.put(owner, Kind::OrderFile, record, &fields, &[], now).map(|_| ())
}

pub fn status_of(r: &Row, chain: Option<&CaseView>) -> Result<Status, Why> {
    status::judge_noted(r.local, chain, r.handed, &r.note)
}

pub const PART_GOODS_TEXT: &str = eravon_goods::goods::PART_GOODS_TEXT;

fn terms_facts(data: &str, me: &str, o: &Order) -> (String, String) {
    let Ok(b) = order::part(data, me, &o.id, &order::part_terms()) else { return Default::default() };
    let Ok(t) = eravon_terms::doc::terms(&b) else { return Default::default() };
    let held = order::part(data, me, &o.id, PART_GOODS_TEXT).ok().and_then(|x| String::from_utf8(x).ok());
    let name = match eravon_goods::terms::goods_of(&t.block, held.as_deref()) {
        Some(text) => eravon_goods::terms::name_of(&text),
        None => t.block.get("goodsHash").and_then(eravon_galeed::Value::as_str).map(|h| format!("私密货物 {}…", &h[..h.len().min(10)])).unwrap_or_default(),
    };
    (name, t.amount.to_dec())
}

fn deadline_of(s: Option<&Status>, v: Option<&CaseView>, o: Option<&Order>) -> u64 {
    use crate::status::Word;
    match (s.map(|s| s.word), v) {
        (Some(Word::AwaitRedraw), Some(v)) => v.redraw_deadline,
        (Some(Word::AwaitAccept | Word::AwaitDelivery | Word::AwaitConfirm | Word::ConfirmHanded | Word::Disputed), Some(v)) => v.window_end,
        (Some(Word::AwaitPay | Word::AwaitDeploy), _) => o.map(|o| if o.open_by == 0 { o.expiry } else { o.expiry.min(o.open_by) }).unwrap_or(0),
        _ => 0,
    }
}

pub fn done(r: &Row) -> bool {
    r.status.as_ref().map(|s| s.word.done()).unwrap_or(false)
}

pub fn done_in(rows: &[Row], r: &Row) -> bool {
    match &r.series {
        Some(s) => rows.iter().filter(|x| x.series.as_ref().is_some_and(|y| y.id == s.id)).all(done),
        None => done(r),
    }
}

pub fn group(rows: Vec<Row>) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::with_capacity(rows.len());
    let mut taken = vec![false; rows.len()];
    for i in 0..rows.len() {
        if taken[i] {
            continue;
        }
        match rows[i].series.as_ref().map(|s| s.id.clone()) {
            None => {
                taken[i] = true;
                out.push(rows[i].clone());
            }
            Some(id) => {
                let mut mates: Vec<usize> = (i..rows.len()).filter(|j| !taken[*j] && rows[*j].series.as_ref().is_some_and(|s| s.id == id)).collect();
                mates.sort_by_key(|j| rows[*j].series.as_ref().map(|s| s.stage).unwrap_or(0));
                for j in mates {
                    taken[j] = true;
                    out.push(rows[j].clone());
                }
            }
        }
    }
    out
}

pub fn search<'a>(rows: &'a [Row], q: &str, from: &str, to: &str) -> Result<Vec<&'a Row>, Why> {
    let (from, to) = (crate::day::bound(from)?, crate::day::bound(to)?);
    let q = q.trim().to_lowercase();
    Ok(rows
        .iter()
        .filter(|r| {
            q.is_empty()
                || keyword_fields(r).iter().any(|x| x.to_lowercase().contains(&q))
        })
        .filter(|r| in_range(r.placed, from.as_deref(), to.as_deref()))
        .collect())
}

fn keyword_fields(r: &Row) -> [&String; 4] {
    [&r.number, &r.buyer, &r.label, &r.product]
}


fn in_range(placed: u64, from: Option<&str>, to: Option<&str>) -> bool {
    crate::when::within(placed, from, to)
}


pub fn find<'a>(t: &'a Table, instance: &str) -> Result<&'a Row, Why> {
    t.rows.iter().find(|r| r.instance.eq_ignore_ascii_case(instance)).ok_or_else(|| why("E_ORDERS_NONE", format!("表上没有 {instance}")))
}

pub fn token_word(rows: &[TokenRow], addr: &str) -> String {
    tokens::name(rows, tokens::chain(), addr, &tokens::ZH)
}

pub fn short(addr: &str) -> String {
    tokens::short(addr)
}

pub fn taken(r: &Row) -> bool {
    !done(r) && r.chain.as_ref().is_some_and(|v| v.accepted != 0)
}

pub fn open(r: &Row) -> bool {
    !done(r)
}

pub fn name(r: &Row) -> String {
    if r.number.is_empty() { r.instance.clone() } else { r.number.clone() }
}

pub fn amount_in(rows: &[TokenRow], w: &tokens::Words, amount: &str, decimals: Option<u64>, token: &str, form: tokens::Form) -> String {
    let chain = tokens::chain();
    tokens::amount(amount, tokens::decimals_of(rows, chain, token, decimals), &tokens::name(rows, chain, token, w), form, w)
}


pub fn amount_word(rows: &[TokenRow], amount: &str, decimals: Option<u64>, token: &str, form: tokens::Form) -> String {
    amount_in(rows, &tokens::ZH, amount, decimals, token, form)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_end_this_seat_made_before_a_case_opens_is_kept_by_its_word_and_read_back() {
        for s in [State::Cancelled, State::Expired] {
            assert_eq!(ended_of(s.word()), Some(s));
        }
        assert_eq!(ended_of(State::Ready.word()), None, "only the two ends are kept");
        let (mut r, done) = (row("A", "0xa", 1), row("B", "0xb", 1));
        assert!(done.unrecorded(), "no local record");
        r.record = "rec-a".into();
        assert!(!r.unrecorded());
        assert_eq!(status::judge(Some(State::Cancelled), None).map(|s| s.word), Ok(status::Word::Cancelled), "the chain not read yet: the kept end says it");
    }

    #[test]
    fn a_past_case_is_listed_only_with_the_switch_on() {
        assert!(!shown(true, false));
        assert!(shown(true, true));
        assert!(shown(false, false) && shown(false, true));
    }

    fn row(number: &str, case_id: &str, placed: u64) -> Row {
        Row {
            instance: String::new(),
            case_id: case_id.into(),
            record: String::new(),
            number: number.into(),
            buyer: format!("0x{}", "be".repeat(20)),
            label: String::new(),
            product: "瓷碗".into(),
            amount: String::new(),
            token: String::new(),
            status: Err(why("E_ORDERS_NONE", "-")),
            deadline: 0,
            placed,
            block: None,
            chain: None,
            local: None,
            handed: false,
            note: String::new(),
            terms_hash: String::new(),
            decimals: None,
            past: false,
            logic: String::new(),
            series: None,
        }
    }

    #[test]
    fn a_series_is_one_sale_in_the_table() {
        let s = |stage: u64| Some(eravon_terms::doc::Series { id: format!("0x{}", "5e".repeat(32)), stage, stages: 3 });
        let mut a = row("A", "0xa", 50);
        let mut s1 = row("S1", "0x1", 10);
        let mut s3 = row("S3", "0x3", 40);
        let mut s2 = row("S2", "0x2", 20);
        s1.series = s(1);
        s2.series = s(2);
        s3.series = s(3);
        let b = row("B", "0xb", 30);
        let got: Vec<String> = group(vec![a.clone(), s3.clone(), b.clone(), s2.clone(), s1.clone()]).into_iter().map(|r| r.number).collect();
        assert_eq!(got, ["A", "S1", "S2", "S3", "B"]);
        let fin = |mut r: Row| {
            r.status = Ok(crate::status::Status { word: crate::status::Word::Completed, reason: String::new() });
            r
        };
        a = fin(a);
        let rows = vec![a.clone(), fin(s1.clone()), s2.clone(), s3.clone()];
        assert!(done_in(&rows, &rows[0]));
        assert!(!done_in(&rows, &rows[1]), "前段已结、后段在途:整个系列仍在进行中");
        let all = vec![fin(s1), fin(s2), fin(s3)];
        assert!(done_in(&all, &all[0]));
    }

    #[test]
    fn search_fields_and_timeless_rows() {
        let day = 1_780_000_000;
        let rows = [row("R-0526-002", &format!("0x{}001{}", "a".repeat(30), "c".repeat(31)), day), row("R-0526-001", &format!("0x{}", "d".repeat(64)), 0)];
        let hits = |q: &str, from: &str, to: &str| search(&rows, q, from, to).map(|v| v.iter().map(|r| r.number.clone()).collect::<Vec<_>>());
        assert_eq!(hits("001", "", ""), Ok(vec!["R-0526-001".to_string()]), "案号里的 001 不留下另一单");
        assert_eq!(hits("瓷碗", "", ""), Ok(vec!["R-0526-002".to_string(), "R-0526-001".to_string()]));
        assert_eq!(hits("BEBE", "", ""), Ok(vec!["R-0526-002".to_string(), "R-0526-001".to_string()]));
        assert_eq!(hits("", "", "2026-12-31"), Ok(vec!["R-0526-002".to_string()]), "只设止日:无时刻行不列");
        assert_eq!(hits("", "2026-01-01", ""), Ok(vec!["R-0526-002".to_string()]), "只设起日:无时刻行不列");
        assert_eq!(hits("", "", "").map(|v| v.len()), Ok(2), "两头都没设:无时刻行照列");
    }

    #[test]
    fn amount_words() {
        let rows = tokens::factory();
        let usdc = tokens::on_chain(&rows, tokens::chain()).into_iter().find(|r| r.label == "USDC").map(|r| r.address).expect("钉住那一行的结算币表里有 USDC");
        let full = tokens::Form::Full;
        assert_eq!(amount_word(&rows, "12500000", None, &usdc, full), "12.5 USDC", "登载了的取登载表的位数");
        assert_eq!(amount_word(&rows, "5", Some(6), &usdc, full), "0.000005 USDC");
        assert_eq!(amount_word(&rows, "1234000000", Some(6), &usdc, full), "1,234 USDC");
        let other = format!("0x{}", "4e".repeat(20));
        assert_eq!(amount_word(&rows, "7", Some(0), &other, full), "7 未登载 0x4e4e…4e4e");
        assert_eq!(amount_word(&rows, "7", None, &other, full), "7 最小单位 · 未登载 0x4e4e…4e4e");
        assert_eq!(amount_word(&rows, "123456789", Some(8), &other, tokens::Form::Short), "1.234567… 未登载 0x4e4e…4e4e");
    }
}
