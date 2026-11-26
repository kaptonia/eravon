use eravon_dispute::view::{CaseView, Ev};
use eravon_scan::scan as a3;
use eravon_watch::order::{self, Order};

use crate::status::Word;
use crate::table::Row;
use crate::{Why, Wire, a3w};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terms {
    Ok(String),
    Missing,
    Unreadable(String, String),
    Mismatch(String, String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judge {
    pub addr: String,
    pub state: &'static str,
}

#[derive(Clone, Debug)]
pub struct Overview {
    pub judge: Judge,
    pub placed: String,
    pub terms: Terms,
}

#[derive(Clone, Debug)]
pub struct Delivery {
    pub done: bool,
    pub at: u64,
    pub note: String,
    pub files: Result<Vec<(String, u64, String)>, Why>,
    pub held: bool,
    pub tx: String,
    pub deadline: u64,
    pub hand_over: Option<Result<eravon_deliver::deliver::HandOver, Why>>,
}

#[derive(Clone, Debug)]
pub struct Evidence {
    pub by: String,
    pub ours: bool,
    pub at: u64,
    pub manifest: String,
    pub tx: String,
    pub files: Result<Vec<(String, u64)>, Why>,
    pub source: String,
}

#[derive(Clone, Debug)]
pub struct Dispute {
    pub at: u64,
    pub deadline: u64,
    pub lf: String,
    pub evidence: Vec<Evidence>,
}

#[derive(Clone, Debug)]
pub struct Moment {
    pub at: u64,
    pub say: String,
    pub name: String,
    pub tx: String,
}

#[derive(Clone, Debug, Default)]
pub struct Fee {
    pub amount: String,
    pub judge_fee: String,
    pub deposit: String,
    pub lf: (String, String),
    pub split: Option<(String, String, String, String)>,
}

pub fn say(name: &str) -> &'static str {
    match name {
        "Opened" => "开案(付款生效)",
        "Priced" => "定价",
        "PanelFrozen" => "锁定行会名单",
        "Seated" => "邀请判官",
        "Pulled" => "拉取卖家押金",
        "DrawArmed" => "开始抽签",
        "DeliveryClaimed" => "交付",
        "Confirmed" => "买家确认,放款",
        "Disputed" => "买家申请退款",
        "Settled" => "和解",
        "Ruled" => "裁决",
        "Resigned" => "判官辞任",
        "Lapsed" => "判官逾期结清",
        "Refunded" => "退款",
        "Closed" => "分账",
        "Swept" => "推送付款",
        "Committed" => "提交证据",
        "Anchored" => "锚定记录",
        "Consented" => "登记同意",
        _ => "其他事件",
    }
}

pub fn chain_time(w: &Wire<'_>, block: u64) -> Result<u64, Why> {
    a3::block_stamp(w.t, w.table, block).map_err(a3w)
}

pub fn ahead_stamps(w: &Wire<'_>, blocks: impl IntoIterator<Item = u64>) {
    a3::ahead_stamps(w.t, w.table, &blocks.into_iter().filter(|b| *b != 0).collect::<Vec<_>>());
}

fn stamp(w: &Wire<'_>, block: u64) -> u64 {
    chain_time(w, block).unwrap_or(0)
}

fn find<'a>(v: &'a CaseView, name: &str) -> Option<&'a Ev> {
    v.events.iter().rev().find(|e| e.name == name)
}

fn terms_view(bytes: Vec<u8>, want: &str) -> Terms {
    let got = eravon_galeed::hash::hex_encode(&eravon_terms::doc::terms_hash(&bytes));
    if got.eq_ignore_ascii_case(want) {
        Terms::Ok(String::from_utf8_lossy(&bytes).to_string())
    } else {
        Terms::Mismatch(want.to_string(), got)
    }
}


fn terms_unread((name, detail): crate::Why) -> Terms {
    if name == "E_STORE_MISSING" { Terms::Missing } else { Terms::Unreadable(name, detail) }
}

pub fn overview(data: &str, me: &str, r: &Row, o: Option<&Order>) -> Overview {
    let terms = o
        .and_then(|o| {
            let rc = match order::part(data, me, &o.id, &order::part_receipt()) {
                Ok(b) => b,
                Err(e) => return Some(terms_unread(e)),
            };
            let want = eravon_link::receipt::receipt(&rc).ok()?.terms_hash;
            Some(match order::part(data, me, &o.id, &order::part_terms()) {
                Ok(b) => terms_view(b, &want),
                Err(e) => terms_unread(e),
            })
        })
        .unwrap_or(Terms::Missing);
    let judge = match &r.chain {
        Some(v) if v.seats > 0 => Judge {
            addr: v.judge.clone(),
            state: if v.accepted != 0 {
                "已接单"
            } else if matches!(r.status.as_ref().map(|s| s.word), Ok(Word::AwaitAccept)) {
                "待接单"
            } else {
                "未接单"
            },
        },
        _ => Judge { addr: String::new(), state: "" },
    };
    Overview { judge, placed: crate::day::of(r.placed), terms }
}

pub fn delivery(data: &str, me: &str, w: &Wire<'_>, r: &Row, o: Option<&Order>) -> Delivery {
    let claimed = r.chain.as_ref().and_then(|v| find(v, "DeliveryClaimed")).cloned();
    let rec = o.and_then(|o| eravon_deliver::deliver::delivery(data, me, o));
    let done = claimed.is_some() || rec.is_some();
    let tx = claimed.as_ref().map(|e| e.tx.clone()).or(rec.as_ref().map(|d| d.tx.clone())).unwrap_or_default();
    let hand_over = (o.is_some() && !tx.is_empty() && matches!(r.status.as_ref().map(|s| s.word), Ok(Word::AwaitConfirm | Word::ConfirmHanded)))
        .then(|| eravon_deliver::deliver::hand_over(w, &tx));
    Delivery {
        done,
        at: claimed.as_ref().map(|e| stamp(w, e.block)).or(rec.as_ref().map(|d| d.at)).unwrap_or(0),
        note: rec.as_ref().map(|d| d.note.clone()).unwrap_or_default(),
        files: rec.as_ref().map_or(Ok(Vec::new()), |d| d.files.clone()),
        held: rec.is_some(),
        tx,
        deadline: match (&r.status, &r.chain) {
            (Ok(s), Some(v)) if !done && s.word == Word::AwaitDelivery => v.window_end,
            _ => 0,
        },
        hand_over,
    }
}

pub fn dispute(data: &str, me: &str, w: &Wire<'_>, r: &Row, o: Option<&Order>) -> Option<Dispute> {
    dispute_keyed(data, me, &[me.to_string()], w, r, o)
}

pub fn dispute_keyed(data: &str, me: &str, keys: &[String], w: &Wire<'_>, r: &Row, o: Option<&Order>) -> Option<Dispute> {
    let v = r.chain.as_ref()?;
    let d = find(v, "Disputed")?;
    let mine = |by: &str| keys.iter().any(|k| k.eq_ignore_ascii_case(by));
    let ours = o.map(|o| eravon_dispute::evidence::ours(data, me, o)).unwrap_or_default();
    ahead_stamps(w, std::iter::once(d.block).chain(v.events.iter().filter(|e| e.name == "Committed").map(|e| e.block)));
    let evidence = v
        .events
        .iter()
        .filter(|e| e.name == "Committed")
        .map(|e| {
            let by = e.args.get(1).cloned().unwrap_or_default();
            let manifest = e.args.get(2).cloned().unwrap_or_default();
            let own = ours.iter().find(|x| x.manifest.eq_ignore_ascii_case(&manifest) && mine(&by));
            Evidence {
                ours: mine(&by),
                by,
                at: stamp(w, e.block),
                manifest,
                tx: e.tx.clone(),
                files: own.map_or(Ok(Vec::new()), |x| x.files.clone()),
                source: own.map(|x| x.source.clone()).unwrap_or_default(),
            }
        })
        .collect();
    Some(Dispute {
        at: stamp(w, d.block),
        deadline: if v.phase == 3 { v.window_end } else { 0 },
        lf: if v.phase == 3 { v.lf_held.to_dec() } else { v.lf.to_dec() },
        evidence,
    })
}

fn moments(w: &Wire<'_>, r: &Row, o: Option<&Order>) -> Result<Vec<a3::Moment>, Why> {
    if r.block.is_none() && r.chain.is_none() {
        return Ok(Vec::new());
    }
    let opened = r.block.or_else(|| o.map(|o| o.opened)).unwrap_or(0);
    a3::history(w.t, w.table, w.chain_id, r.wire(w).logic, &r.instance, eravon_chain::read::instance_from(opened), eravon_chain::seam::Upto::Latest).map_err(a3w)
}


pub fn record(w: &Wire<'_>, r: &Row, o: Option<&Order>) -> Result<Vec<Moment>, Why> {
    let moments = moments(w, r, o)?;
    ahead_stamps(w, moments.iter().map(|m| m.block));
    moments
        .into_iter()
        .map(|m| Ok(Moment { at: if m.block == 0 { 0 } else { chain_time(w, m.block)? }, say: say(&m.name).into(), name: m.name, tx: m.tx }))
        .collect()
}

fn split(v: &CaseView, _premium: u64) -> Option<(String, String, String, String)> {
    let c = find(v, "Closed")?;
    let a = |i: usize| c.args.get(i).cloned().unwrap_or_default();
    Some((a(1), a(2), a(3), a(4)))
}


pub fn chosen(listing: &eravon_link::listing::Listing, candidate: u64) -> Option<String> {
    let n = usize::try_from(candidate).ok()?;
    listing.judges.get(n).map(|j| j.judge.clone()).or_else(|| n.checked_sub(listing.judges.len()).and_then(|g| listing.guilds.get(g)).map(|g| g.address.clone()))
}

pub fn fee(r: &Row) -> Fee {
    let Some(v) = &r.chain else {
        return Fee { amount: r.amount.clone(), ..Fee::default() };
    };
    let sp = split(v, 0);
    let judge_fee = match &sp {
        Some((_, _, j, _)) => j.clone(),
        None => v.premium.to_dec(),
    };
    Fee {
        amount: r.amount.clone(),
        judge_fee,
        deposit: v.deposit.to_dec(),
        lf: (v.lf.to_dec(), v.lf_held.to_dec()),
        split: sp,
    }
}

pub fn order_of(data: &str, me: &str, r: &Row) -> Option<Order> {
    (!r.record.is_empty()).then(|| order::load(data, me, &r.record).ok()).flatten()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_missing_part_is_missing() {
        assert_eq!(terms_unread(("E_STORE_MISSING".into(), "not there".into())), Terms::Missing);
        assert_eq!(
            terms_unread(("E_STORE_SEALED".into(), "unopenable p".into())),
            Terms::Unreadable("E_STORE_SEALED".into(), "unopenable p".into())
        );
    }
}
