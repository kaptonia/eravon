use eravon_standard as a1;

use crate::order::{self, Broadcast, Order, State};
use crate::poll::{self, Deal};
use crate::{Why, Wire, why};

fn a1w(e: a1::fault::Fault) -> Why {
    why(&e.name(), e.detail())
}

pub trait Post {
    fn from(&self) -> Option<String>;
    fn post(&mut self, call: &a1::call::Call) -> Result<String, Why>;
    fn broadcast(&self) -> Option<(String, Broadcast)>;
    fn resend(&mut self, call: &a1::call::Call, at: &Broadcast) -> Result<String, Why>;
    fn post_landed(&mut self, call: &a1::call::Call, land: Land<'_>) -> Result<String, Why> {
        let _ = land;
        self.post(call)
    }
    fn resend_landed(&mut self, call: &a1::call::Call, at: &Broadcast, land: Land<'_>) -> Result<String, Why> {
        let _ = land;
        self.resend(call, at)
    }
}

pub type Land<'a> = &'a mut dyn FnMut(&str, &Broadcast) -> Result<(), Why>;

struct Landing<'a> {
    data: &'a str,
    writer: &'a str,
    now: u64,
    me: &'a str,
    order: Order,
    what: &'static str,
    landed: Option<(String, Broadcast)>,
}

impl<'a> Landing<'a> {
    fn new(data: &'a str, writer: &'a str, now: u64, me: &'a str, o: &Order, what: &'static str) -> Landing<'a> {
        Landing { data, writer, now, me, order: o.clone(), what, landed: None }
    }

    fn land(&mut self, tx: &str, b: &Broadcast) -> Result<(), Why> {
        self.order.set_pending(self.what, tx, b);
        order::save(self.data, self.writer, self.now, self.me, &self.order, &[])?;
        self.landed = Some((tx.to_string(), b.clone()));
        Ok(())
    }


    fn keep(&self, o: &mut Order) -> bool {
        match &self.landed {
            Some((tx, b)) => {
                o.set_pending(self.what, tx, b);
                true
            }
            None => false,
        }
    }
}

enum Seen {
    Mined,
    Reverted,
    Waiting,
    Gone,
}

fn seen(w: &Wire<'_>, tx: &str) -> Result<Seen, Why> {
    if w.table.list.is_empty() {
        return Err(why(crate::E_NO_ENDPOINT, "未配置节点"));
    }
    if let Some(r) = eravon_chain::sending::receipt(w.t, w.table, tx).map_err(|e| why(e.name(), e.detail()))? {
        return Ok(if r.status == 1 { Seen::Mined } else { Seen::Reverted });
    }
    let (v, _) = eravon_chain::read::transaction(w.t, w.table, tx).map_err(|e| why(e.name(), e.detail()))?;
    Ok(if matches!(v, eravon_literals::json::In::Null) { Seen::Gone } else { Seen::Waiting })
}

pub fn after_send(tx: String, local: Result<(), Why>) -> Result<String, Why> {
    match local {
        Ok(()) => Ok(tx),
        Err((n, d)) => Err(("E_LOCAL_AFTER_SEND".into(), format!("{tx} 已上链,但本机记录保存失败({n}:{d}),请勿重复发送"))),
    }
}

const PENDING_DEPLOY: &str = "deploy";
const PENDING_INIT: &str = "init";
const PENDING_EXPIRE: &str = "expire";

fn held(o: &mut Order, n: &str, what: &str, p: &dyn Post) {
    if n == "E_V_UNCONFIRMED" {
        if let Some((tx, b)) = p.broadcast() {
            o.set_pending(what, &tx, &b);
        }
    }
}

fn missed(head: &str, n: &str, detail: &str, landed: bool) -> String {
    match n {
        "E_V_UNCONFIRMED" | "E_LOCAL_AFTER_SEND" => format!("{SENT_UNCONFIRMED}:{n}:{detail}"),
        _ if landed => format!("{SENT_UNKNOWN}:{n}:{detail}"),
        _ => format!("{head}:{n}:{detail}"),
    }
}

const SENT_UNCONFIRMED: &str = "已发出,等待确认";
const SENT_UNKNOWN: &str = "已签已记,交没交出去未知,下一轮先按哈希问";

pub fn missed_parts(note: &str) -> Option<(&str, &str)> {
    if note.starts_with(SENT_UNCONFIRMED) || note.starts_with(SENT_UNKNOWN) {
        return None;
    }
    let (_, rest) = note.split_once(':')?;
    let (name, detail) = rest.split_once(':').unwrap_or((rest, ""));
    (name.starts_with("E_") && name.len() > 2 && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')).then_some((name, detail))
}

fn act_refused(r: &poll::Reading) -> Option<(String, String)> {
    if poll::stage_waits(&r.prev, r.prev_lapsed).is_some() {
        return None;
    }
    match &r.act {
        Some(Err(f)) => Some((f.name.clone(), f.why.clone())),
        _ => None,
    }
}

pub const GUILD_NOTE: &str = "行会候选:名单与判法词没读成,S4 不自发;经命令行 deploy 递 --member 与 --amanah";

fn ready_panel(w: &Wire<'_>, d: &Deal) -> Result<Option<crate::pool::Panel>, Why> {
    if !d.guild {
        return Ok(None);
    }
    crate::pool::panel(w, &d.guild_addr, &d.o).map(Some)
}

pub const NO_KEY_NOTE: &str = "未发送:请先解锁";

pub const OPEN_OFF_NOTE: &str = "已到款,待开案";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opens {
    Yes,
    Off,
    Read,
}

impl Opens {
    pub fn auto(on: bool) -> Opens {
        if on { Opens::Yes } else { Opens::Off }
    }
}

pub fn draw(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, o: &mut Order, post: Option<&mut dyn Post>) -> Result<Option<(&'static str, String)>, Why> {
    let mut sent = None;
    if let Ok(d) = poll::deal(data, me, o, w)
        && d.guild
        && let Some(call) = crate::pool::draw_due(w, &o.instance)?
    {
        match post {
            None => o.note = NO_KEY_NOTE.to_string(),
            Some(p) => match p.post(&call) {
                Ok(tx) => {
                    o.note = format!("已抽签:{tx}");
                    sent = Some(("redraw", tx));
                }
                Err((n, detail)) => o.note = format!("抽签未发:{n}:{detail}"),
            },
        }
        order::save(data, writer, now, me, o, &[])?;
    }
    Ok(sent)
}

pub fn draw_round(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, post: &mut dyn Post) -> Result<Vec<(String, Result<String, Why>)>, Why> {
    let mut out = Vec::new();
    let all = order::all(data, me)?;
    let status = eravon_standard::call::view(eravon_standard::sol::STATUS).ok();
    let views: Vec<(String, Vec<u8>)> = all.iter().filter(|o| o.state == State::Deployed && poll::deal(data, me, o, w).is_ok_and(|d| d.guild)).filter_map(|o| status.clone().map(|s| (o.instance.clone(), s))).collect();
    eravon_standard::chain::ahead_views(w.t, w.table, &views);
    for mut o in all {
        if o.state != State::Deployed || poll::past_held(data, me, &o) {
            continue;
        }
        match draw(data, writer, now, me, w, &mut o, Some(&mut *post)) {
            Ok(Some((_, tx))) => out.push((o.id.clone(), Ok(tx))),
            Ok(None) => {}
            Err(e) => out.push((o.id.clone(), Err(e))),
        }
    }
    Ok(out)
}

pub struct Ticked {
    pub order: Order,
    pub sent: Option<(&'static str, String)>,
    pub refused: Option<(String, String)>,
}

fn deploy_call(w: &Wire<'_>, d: &Deal) -> Result<a1::call::Call, Why> {
    let _ = w;
    a1::call::deploy_instance(&d.derived.salt, &d.logic).map_err(a1w)
}

fn init_call(w: &Wire<'_>, d: &Deal, quote: &[u8], instance: &str, panel: Option<&crate::pool::Panel>) -> Result<a1::call::Call, Why> {
    let _ = w;
    let (members, hashes): (&[String], &[[u8; 32]]) = match panel {
        Some(p) => (&p.members, &p.amanah_hashes),
        None => (&[], &[]),
    };
    a1::call::init(&d.o, instance, quote, &d.sig_buyer, &d.sig_seller, members, hashes).map_err(a1w)
}


fn deployed_code(w: &Wire<'_>, instance: &str) -> Result<bool, Why> {
    let (code, _) = eravon_chain::read::code(w.t, w.table, instance, eravon_chain::seam::TAG_LATEST).map_err(|e| why(e.name(), e.detail()))?;
    Ok(!code.is_empty())
}

#[allow(clippy::too_many_arguments)]
fn deploy(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, d: &Deal, o: &mut Order, panel: Option<&crate::pool::Panel>, p: &mut dyn Post) -> Result<String, Why> {
    a1::precheck::verifier_there(w.t, w.table).map_err(a1w)?;
    let from = p.from();
    let (sig_seller, sig_buyer) = a1::precheck::slots(&d.o, w.chain_id, w.logic, &d.sig_seller, &d.sig_buyer, &mut |k: &str| a1::chain::current(w.t, w.table, k).map(|r| r.value)).map_err(a1w)?;
    let fitted = Deal { sig_seller, sig_buyer, ..d.clone() };
    let d = &fitted;
    let mut reg =
        |who: &str, dg: &[u8; 32]| a1::consent::registered(w.t, w.table, w.logic, who, dg);
    a1::precheck::consents(
        &d.o,
        w.chain_id,
        w.logic,
        &d.sig_seller,
        &d.sig_buyer,
        a1::consent::Sender {
            from: from.as_deref(),
            via_multicall: false,
        },
        &mut |k: &str| a1::chain::current(w.t, w.table, k).map(|r| r.value),
        &mut reg,
    )
    .map_err(a1w)?;
    a1::precheck::funded(w.t, w.table, &d.o, &o.instance).map_err(a1w)?;
    a1::precheck::in_window(poll::open_until(&d.o), now).map_err(a1w)?;
    let passed = crate::act::check(w, &d.o).map_err(|r| crate::act::why_of(&r))?;
    if !deployed_code(w, &o.instance)? {
        let mut l = Landing::new(data, writer, now, me, &*o, PENDING_DEPLOY);
        let sent = p.post_landed(&deploy_call(w, d)?, &mut |tx, b| l.land(tx, b));
        if let Err(e) = sent {
            l.keep(o);
            return Err(e);
        }
        o.clear_pending();
    }
    let call = init_call(w, d, &passed.quote, &o.instance, panel)?;
    let mut l = Landing::new(data, writer, now, me, &*o, PENDING_INIT);
    let sent = p.post_landed(&call, &mut |tx, b| l.land(tx, b));
    if sent.is_err() {
        l.keep(o);
    }
    sent
}

pub const EXPIRED_NOTE: &str = "开不了案,余额已退回买家";

fn expire(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, d: &Deal, o: &mut Order, p: &mut dyn Post) -> Result<String, Why> {
    if !deployed_code(w, &o.instance)? {
        let mut l = Landing::new(data, writer, now, me, &*o, PENDING_DEPLOY);
        let sent = p.post_landed(&deploy_call(w, d)?, &mut |tx, b| l.land(tx, b));
        if let Err(e) = sent {
            l.keep(o);
            return Err(e);
        }
        o.clear_pending();
    }
    let mut l = Landing::new(data, writer, now, me, &*o, PENDING_EXPIRE);
    let sent = p.post_landed(&expire_call(d, o)?, &mut |tx, b| l.land(tx, b));
    if sent.is_err() {
        l.keep(o);
    }
    sent
}

fn expire_call(d: &Deal, o: &Order) -> Result<a1::call::Call, Why> {
    a1::call::expire_init_at(&d.o, &o.instance, &d.logic).map_err(a1w)
}

fn pending_of(o: &Order) -> Result<(String, String, Broadcast), Why> {
    let none = || why("E_WATCH_NO_PENDING", "这一单没有记下 nonce 与费率的待定交易");
    let (what, tx) = o.pending().ok_or_else(none)?;
    let at = o.pending_at().ok_or_else(none)?;
    Ok((what, tx, at))
}

#[allow(clippy::too_many_arguments)]
pub fn bump(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, id: &str, p: &mut dyn Post) -> Result<Ticked, Why> {
    let mut o = order::load(data, me, id)?;
    not_past(data, me, &o)?;
    let (what, _, at) = pending_of(&o)?;
    let d = poll::deal(data, me, &o, w)?;
    let (call, what) = match what.as_str() {
        PENDING_EXPIRE => (expire_call(&d, &o)?, PENDING_EXPIRE),
        PENDING_INIT => {
            let passed = crate::act::check(w, &d.o).map_err(|r| crate::act::why_of(&r))?;
            let panel = ready_panel(w, &d)?;
            (init_call(w, &d, &passed.quote, &o.instance, panel.as_ref())?, PENDING_INIT)
        }
        _ => (deploy_call(w, &d)?, PENDING_DEPLOY),
    };
    let mut sent = None;
    let mut l = Landing::new(data, writer, now, me, &o, what);
    match replace(p, &call, &at, &mut |tx, b| l.land(tx, b)) {
        Ok(tx) => {
            o.clear_pending();
            match what {
                PENDING_EXPIRE => {
                    o.expire_tx = tx.clone();
                    sent = Some(("expire", tx));
                }
                PENDING_INIT => {
                    o.deployed(&tx);
                    sent = Some(("init", tx));
                }
                _ => {
                    o.note = format!("实例已部署({tx}),下一轮发 init");
                    sent = Some(("deploy", tx));
                }
            }
        }
        Err((n, detail)) => {
            let landed = l.keep(&mut o);
            held(&mut o, &n, what, p);
            o.note = missed("加价重发未成", &n, &detail, landed);
        }
    }
    let saved = order::save(data, writer, now, me, &o, &[]);
    match &sent {
        Some((_, tx)) => {
            after_send(tx.clone(), saved)?;
        }
        None => saved?,
    }
    Ok(Ticked { order: o, sent, refused: None })
}

fn replace(p: &mut dyn Post, call: &a1::call::Call, at: &Broadcast, land: Land<'_>) -> Result<String, Why> {
    p.resend_landed(call, at, land)
}


pub fn void(data: &str, writer: &str, now: u64, me: &str, from: &str, w: &Wire<'_>, id: &str) -> Result<Order, Why> {
    let mut o = order::load(data, me, id)?;
    not_past(data, me, &o)?;
    let (_, tx, at) = pending_of(&o)?;
    match void_ok(from, w, &tx, &at)? {
        Voided::Void => {}
        Voided::Unused { done } => return Err(why(NOT_VOID, &format!("链上已确认的 nonce 是 {done},还没越过这一笔的 {}:它仍可能上链", at.nonce))),
        Voided::Held(said) => return Err(why(NOT_VOID, &said)),
    }
    o.clear_pending();
    o.note = format!("已作废 {tx}:链上 nonce 已越过、无节点持有;可再发");
    order::save(data, writer, now, me, &o, &[])?;
    Ok(o)
}

pub const NOT_VOID: &str = "E_WATCH_NOT_VOID";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Voided {
    Void,
    Unused { done: u64 },
    Held(String),
}

pub fn void_check(from: &str, w: &Wire<'_>, tx: &str, at: &Broadcast) -> Result<Voided, Why> {
    void_ok(from, w, tx, at)
}

fn void_ok(from: &str, w: &Wire<'_>, tx: &str, at: &Broadcast) -> Result<Voided, Why> {
    let said = eravon_chain::read::void_read(w.t, w.table, from, tx, at.nonce).map_err(|e| why(e.name(), e.detail()))?;
    let (low, none, held) = void_judged(&said, at.nonce, tx);
    Ok(match eravon_chain::send::unheld(low, at.nonce, || none) {
        eravon_chain::send::Unheld::Void => Voided::Void,
        eravon_chain::send::Unheld::Unused => Voided::Unused { done: low.unwrap_or(0) },
        eravon_chain::send::Unheld::Wait => Voided::Held(held),
    })
}

fn void_judged(said: &[(eravon_chain::seam::NodeAt, Option<eravon_chain::read::VoidSaid>)], mine: u64, tx: &str) -> (Option<u64>, bool, String) {
    use eravon_chain::read::VoidSaid;
    let came: Vec<(&eravon_chain::seam::NodeAt, &VoidSaid)> = said.iter().filter_map(|(n, v)| v.as_ref().map(|v| (n, v))).collect();
    let nonce_of = |v: &VoidSaid| match v {
        VoidSaid::Nonce(n) | VoidSaid::Past(n, _) => Some(*n),
        VoidSaid::Unread(_) => None,
    };
    let two = |pick: &dyn Fn(&VoidSaid) -> bool| -> Option<u64> {
        let xs: Vec<&(&eravon_chain::seam::NodeAt, &VoidSaid)> = came.iter().filter(|(_, v)| pick(v)).collect();
        (eravon_chain::read::places(xs.iter().map(|(n, _)| *n)) >= 2).then(|| xs.iter().filter_map(|(_, v)| nonce_of(v)).min()).flatten()
    };
    if let Some((n, v)) = came.iter().find(|(_, v)| matches!(v, VoidSaid::Past(_, Some(false)))) {
        return (nonce_of(v), false, format!("{tx} 仍有节点持有({})", n.said()));
    }
    if let Some(low) = two(&|v| matches!(v, VoidSaid::Past(_, Some(true)))) {
        return (Some(low), true, String::new());
    }
    if let Some(low) = two(&|v| matches!(v, VoidSaid::Nonce(_))) {
        return (Some(low), false, String::new());
    }
    let words: Vec<String> = said
        .iter()
        .map(|(n, v)| match v {
            Some(VoidSaid::Past(_, Some(true))) => format!("{} 越过 nonce {mine} 说没有", n.said()),
            Some(VoidSaid::Past(_, Some(false))) => format!("{} 持有", n.said()),
            Some(VoidSaid::Past(_, None)) => format!("{} 没答是否持有", n.said()),
            Some(VoidSaid::Nonce(k)) => format!("{} 还没越过(nonce {k})", n.said()),
            Some(VoidSaid::Unread(f)) => format!("{} 没答:{}:{}", n.said(), f.name(), f.detail()),
            None => format!("{} 没答", n.said()),
        })
        .collect();
    let held = (None, false, format!("{tx} 是否仍有节点持有读不清:没有两处同答({})", words.join(";")));
    if eravon_chain::read::places(said.iter().map(|(n, _)| n)) >= 2 || came.len() < said.len() {
        return held;
    }
    let low = came.iter().filter_map(|(_, v)| nonce_of(v)).min();
    if came.iter().all(|(_, v)| matches!(v, VoidSaid::Past(_, Some(true)))) {
        return (low, true, String::new());
    }
    if came.iter().all(|(_, v)| matches!(v, VoidSaid::Nonce(_))) {
        return (low, false, String::new());
    }
    held
}


#[allow(clippy::too_many_arguments)]
pub fn tick(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    id: &str,
    opens: Opens,
    post: Option<&mut dyn Post>,
) -> Result<Ticked, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let mut o = order::load(data, me, id)?;
    if poll::past_held(data, me, &o) {
        return Ok(Ticked { order: o, sent: None, refused: None });
    }
    if crate::units::of(&o).is_none() {
        let th = order::part(data, me, &o.id, &order::part_receipt())
            .ok()
            .and_then(|b| eravon_link::receipt::receipt(&b).ok())
            .map(|r| r.terms_hash)
            .unwrap_or_default();
        if crate::units::fill(data, me, Some(w), &mut o, &th) {
            order::save(data, writer, now, me, &o, &[])?;
        }
    }
    if o.state == State::Deployed {
        if opens == Opens::Read {
            return Ok(Ticked { order: o, sent: None, refused: None });
        }
        let sent = draw(data, writer, now, me, w, &mut o, post)?;
        return Ok(Ticked { order: o, sent, refused: None });
    }
    if matches!(o.state, State::Cancelled) || (matches!(o.state, State::Expired | State::Revoked) && !o.expire_tx.is_empty()) {
        return Ok(Ticked { order: o, sent: None, refused: None });
    }
    if let Some((what, tx)) = o.pending() {
        match seen(w, &tx)? {
            Seen::Mined => {
                o.clear_pending();
                match what.as_str() {
                    PENDING_EXPIRE => o.expire_tx = tx,
                    PENDING_INIT => o.deploy_tx = tx,
                    _ => {}
                }
            }
            Seen::Reverted | Seen::Gone => o.clear_pending(),
            Seen::Waiting => {
                o.note = format!("已发出 {tx},等待上链;不重发");
                order::save(data, writer, now, me, &o, &[])?;
                return Ok(Ticked { order: o, sent: None, refused: None });
            }
        }
    }
    let d = poll::deal(data, me, &o, w)?;
    let r = poll::read(w, &o, &d, now)?;
    let (state, gap, note) = poll::decide(&o, &r)?;
    if state == State::Deployed {
        o.deployed("");
    } else {
        o.set_state(state);
    }
    o.balance = r.balance.to_dec();
    o.gap = gap;
    o.note = note;
    let mut sent = None;
    let mut refused: Option<(String, String)> = None;
    match state {
        State::Ready if d.past => {
            o.note = format!("未发送:{}", a1::fault::PAST_LISTING_SAYS);
            refused = Some((a1::fault::PAST_LISTING.to_string(), a1::fault::PAST_LISTING_SAYS.to_string()));
        }
        State::Ready if opens == Opens::Off => o.note = OPEN_OFF_NOTE.to_string(),
        State::Ready if opens == Opens::Read => {}
        State::Ready => match (post, ready_panel(w, &d)) {
            (_, Err((n, detail))) => {
                o.note = format!("未发送:{n}:{detail}");
                refused = Some((n, detail));
            }
            (None, Ok(_)) => {
                o.note = NO_KEY_NOTE.to_string();
                refused = Some(("E_VAULT_LOCKED".to_string(), NO_KEY_NOTE.to_string()));
            }
            (Some(p), Ok(panel)) => match deploy(data, writer, now, me, w, &d, &mut o, panel.as_ref(), p) {
                Ok(tx) => {
                    o.deployed(&tx);
                    sent = Some(("init", tx));
                }
                Err((n, detail)) => {
                    let landed = o.pending().is_some();
                    if !landed {
                        held(&mut o, &n, PENDING_INIT, p);
                    }
                    o.note = missed("未发送", &n, &detail, landed);
                    refused = missed_parts(&o.note).map(|(n, d)| (n.to_string(), d.to_string()));
                }
            },
        },
        State::Expired | State::Revoked if !r.balance.is_zero() && opens == Opens::Read => {}
        State::Expired | State::Revoked if !r.balance.is_zero() => match post {
            None => o.note = format!("有待退余额:{}", NO_KEY_NOTE),
            Some(p) => match expire(data, writer, now, me, w, &d, &mut o, p) {
                Ok(tx) => {
                    o.expire_tx = tx.clone();
                    o.note = EXPIRED_NOTE.to_string();
                    sent = Some(("expire", tx));
                }
                Err((n, detail)) => {
                    let landed = o.pending().is_some();
                    if !landed {
                        held(&mut o, &n, PENDING_EXPIRE, p);
                    }
                    o.note = missed("退回买家未发送", &n, &detail, landed);
                    refused = missed_parts(&o.note).map(|(n, d)| (n.to_string(), d.to_string()));
                }
            },
        },
        State::PriceStale => {
            o.notice = o.note.clone();
            o.notice_at = now;
            refused = act_refused(&r);
        }
        State::Blocked | State::DepositShort => refused = act_refused(&r),
        _ => {}
    }
    let saved = order::save(data, writer, now, me, &o, &[]);
    match &sent {
        Some((_, tx)) => {
            after_send(tx.clone(), saved)?;
        }
        None => saved?,
    }
    Ok(Ticked { order: o, sent, refused })
}

fn not_past(data: &str, me: &str, o: &Order) -> Result<(), Why> {
    if poll::made_past(data, me, o) {
        return Err(why(a1::fault::PAST_LISTING, a1::fault::PAST_LISTING_SAYS));
    }
    Ok(())
}

#[cfg(test)]
mod void_tests {
    use eravon_chain::read::VoidSaid;
    use eravon_chain::seam::NodeAt;

    fn verdict(said: Vec<(&str, Option<VoidSaid>)>) -> String {
        let said: Vec<(NodeAt, Option<VoidSaid>)> = said.into_iter().map(|(n, v)| (NodeAt::new(n), v)).collect();
        let (low, none, words) = super::void_judged(&said, 4, "0xab");
        match eravon_chain::send::unheld(low, 4, || none) {
            eravon_chain::send::Unheld::Void => "void".into(),
            eravon_chain::send::Unheld::Unused => format!("unused {}", low.unwrap_or(0)),
            eravon_chain::send::Unheld::Wait => format!("held {words}"),
        }
    }

    #[test]
    fn two_places_past_the_nonce_holding_none_make_it_void() {
        assert_eq!(verdict(vec![("http://a.test", Some(VoidSaid::Past(6, Some(true)))), ("http://b.test", Some(VoidSaid::Past(5, Some(true)))), ("http://c.test", None)]), "void");
    }

    #[test]
    fn one_place_saying_none_alone_is_not_void() {
        let v = verdict(vec![("http://a.test", Some(VoidSaid::Past(6, Some(true)))), ("http://b.test", Some(VoidSaid::Unread(eravon_chain::fault::Fault::Transport("没答".into())))), ("http://c.test", Some(VoidSaid::Past(6, None)))]);
        assert!(v.starts_with("held ") && v.contains("读不清"), "{v}");
        let one = verdict(vec![("http://a.test", Some(VoidSaid::Past(6, Some(true))))]);
        assert_eq!(one, "void", "a table of one node past the nonce saying none: every node answered, as before");
    }

    #[test]
    fn two_places_not_past_the_nonce_make_it_unused() {
        assert_eq!(verdict(vec![("http://a.test", Some(VoidSaid::Nonce(4))), ("http://b.test", Some(VoidSaid::Nonce(3))), ("http://c.test", Some(VoidSaid::Past(9, Some(true))))]), "unused 3");
    }

    #[test]
    fn one_place_holding_it_leaves_nothing_void() {
        let v = verdict(vec![("http://a.test", Some(VoidSaid::Past(6, Some(true)))), ("http://b.test", Some(VoidSaid::Past(6, Some(false)))), ("http://c.test", Some(VoidSaid::Past(6, Some(true))))]);
        assert!(v.starts_with("held ") && v.contains("b.test"), "{v}");
    }

    #[test]
    fn fewer_than_two_places_alike_judge_nothing() {
        let v = verdict(vec![("http://a.test", Some(VoidSaid::Past(6, Some(true)))), ("http://b.test", Some(VoidSaid::Nonce(3)))]);
        assert!(v.starts_with("held ") && v.contains("读不清") && v.contains("a.test") && v.contains("b.test"), "{v}");
        let v = verdict(vec![("http://a.test", Some(VoidSaid::Past(6, Some(true)))), ("http://b.test", Some(VoidSaid::Nonce(3))), ("http://c.test", Some(VoidSaid::Past(7, None)))]);
        assert!(v.starts_with("held ") && v.contains("c.test"), "{v}");
        let v = verdict(vec![("http://a.test", Some(VoidSaid::Nonce(3))), ("http://b.test", None)]);
        assert!(v.starts_with("held ") && v.contains("b.test 没答"), "a place not past it with the other silent is not unused: {v}");
        assert_eq!(verdict(vec![("http://a.test", Some(VoidSaid::Nonce(3)))]), "unused 3", "a table of one place: its word");
        let v = verdict(vec![("http://a.test", None)]);
        assert!(v.starts_with("held "), "a table of one place not answered: {v}");
    }
}

#[cfg(test)]
mod missed_tests {
    use super::{missed, missed_parts};

    #[test]
    fn a_note_of_a_send_not_made_reads_back_as_its_refusal() {
        let n = missed("未发送", "E_CHAIN_FUNDS", "funds:余额付不起这一笔的手续费", false);
        assert_eq!(missed_parts(&n), Some(("E_CHAIN_FUNDS", "funds:余额付不起这一笔的手续费")));
        let back = missed("退回买家未发送", "E_CHAIN_EXECUTION", "reverted", false);
        assert_eq!(missed_parts(&back), Some(("E_CHAIN_EXECUTION", "reverted")));
        assert_eq!(missed_parts(&missed("未发送", "E_V_UNCONFIRMED", "x", false)), None, "sent, waiting to be confirmed");
        assert_eq!(missed_parts(&missed("未发送", "E_CHAIN_FUNDS", "x", true)), None, "signed and kept: outcome unknown");
        assert_eq!(missed_parts(super::NO_KEY_NOTE), None);
        assert_eq!(missed_parts("未发送:E_A1_PAST_LISTING"), Some(("E_A1_PAST_LISTING", "")));
    }
}
