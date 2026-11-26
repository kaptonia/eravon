use eravon_literals::literals as lit;
use eravon_store::record::{Kind, Record};
use eravon_store::seam::Box_;

use crate::{Why, why};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Awaiting,
    Short,
    Ready,
    PendingConsent,
    PriceStale,
    DepositShort,
    Blocked,
    AwaitPrev,
    Revoked,
    Cancelled,
    Deployed,
    Expired,
}

impl State {
    pub const ALL: [State; 12] = [
        State::Awaiting,
        State::Short,
        State::Ready,
        State::PendingConsent,
        State::PriceStale,
        State::DepositShort,
        State::Blocked,
        State::AwaitPrev,
        State::Deployed,
        State::Expired,
        State::Revoked,
        State::Cancelled,
    ];

    pub const fn word(self) -> &'static str {
        match self {
            State::Awaiting => "awaiting",
            State::Short => "short",
            State::Ready => "ready",
            State::PendingConsent => "pending-consent",
            State::PriceStale => "price-stale",
            State::DepositShort => "deposit-short",
            State::Blocked => "blocked",
            State::AwaitPrev => "await-prev",
            State::Deployed => "deployed",
            State::Expired => "expired",
            State::Revoked => "revoked",
            State::Cancelled => "cancelled",
        }
    }

    pub fn of(w: &str) -> Option<State> {
        State::ALL.into_iter().find(|s| s.word() == w)
    }

    pub const fn say(self) -> &'static str {
        match self {
            State::Awaiting => "待到账",
            State::Short => "差额",
            State::Ready => "待部署",
            State::PendingConsent => "待部署 · 等买家同意",
            State::PriceStale => "价源陈旧",
            State::DepositShort => "卖家押金不足",
            State::Blocked => "开案前核不过",
            State::AwaitPrev => "待前段",
            State::Deployed => "已部署",
            State::Expired => "已过期",
            State::Revoked => "已下架",
            State::Cancelled => "已撤单",
        }
    }

    pub const fn open(self) -> bool {
        !matches!(self, State::Deployed | State::Expired | State::Revoked | State::Cancelled)
    }
}

pub const GAP_NOTE: &str = "只在初始化前补足有意义";

#[derive(Clone, Debug)]
pub struct Order {
    pub id: String,
    pub case_id: String,
    pub instance: String,
    pub buyer_key: String,
    pub buyer_payout: String,
    pub verdict: String,
    pub token: String,
    pub need: String,
    pub open_by: u64,
    pub candidate: u64,
    pub expiry: u64,
    pub state: State,
    pub balance: String,
    pub gap: String,
    pub note: String,
    pub notice: String,
    pub notice_at: u64,
    pub deploy_tx: String,
    pub expire_tx: String,
    pub source: String,
    pub opened: u64,
    pub extra: Vec<(String, String)>,
}

const OWN: [&str; 20] = [
    "balance",
    "buyerKey",
    "buyerPayout",
    "candidate",
    "caseId",
    "deployTx",
    "expireTx",
    "expiry",
    "gap",
    "instance",
    "need",
    "note",
    "notice",
    "noticeAt",
    "openBy",
    "openedBlock",
    "source",
    "state",
    "token",
    "verdict",
];

pub fn part_receipt() -> String {
    lit::DOC_RECEIPT.replace('/', "-")
}
pub fn part_listing() -> String {
    lit::DOC_LISTING.replace('/', "-")
}
pub fn part_terms() -> String {
    lit::DOC_TERMS.replace('/', "-")
}

pub const STALE_FACTS: &str = "staleFacts";

fn leaves_stale(s: State) -> bool {
    s != State::PriceStale
}


impl Order {
    pub fn fields(&self) -> Vec<(String, String)> {
        let mut f: Vec<(String, String)> = vec![
            ("balance".into(), self.balance.clone()),
            ("buyerKey".into(), self.buyer_key.clone()),
            ("buyerPayout".into(), self.buyer_payout.clone()),
            ("candidate".into(), self.candidate.to_string()),
            ("caseId".into(), self.case_id.clone()),
            ("deployTx".into(), self.deploy_tx.clone()),
            ("expireTx".into(), self.expire_tx.clone()),
            ("expiry".into(), self.expiry.to_string()),
            ("gap".into(), self.gap.clone()),
            ("instance".into(), self.instance.clone()),
            ("openBy".into(), self.open_by.to_string()),
            ("need".into(), self.need.clone()),
            ("note".into(), self.note.clone()),
            ("notice".into(), self.notice.clone()),
            ("noticeAt".into(), self.notice_at.to_string()),
            ("openedBlock".into(), self.opened.to_string()),
            ("source".into(), self.source.clone()),
            ("state".into(), self.state.word().to_string()),
            ("token".into(), self.token.clone()),
            ("verdict".into(), self.verdict.clone()),
        ];
        f.extend(self.extra.iter().cloned());
        f
    }

    pub fn of(r: &Record) -> Result<Order, Why> {
        let get = |k: &str| {
            r.fields
                .iter()
                .find(|(a, _)| a == k)
                .map(|(_, v)| v.clone())
        };
        let need = |k: &str| {
            get(k)
                .filter(|v| !v.is_empty())
                .ok_or_else(|| why("E_WATCH_RECORD", format!("订单记录 {} 缺 {k}", r.id)))
        };
        let int = |k: &str| -> Result<u64, Why> {
            match get(k) {
                None => Ok(0),
                Some(v) if v.is_empty() => Ok(0),
                Some(v) => v.parse().map_err(|_| {
                    why(
                        "E_WATCH_RECORD",
                        format!("订单记录 {} 的 {k} 不是整数:{v}", r.id),
                    )
                }),
            }
        };
        let state = match get("state") {
            None => State::Awaiting,
            Some(w) => State::of(&w).ok_or_else(|| {
                why(
                    "E_WATCH_RECORD",
                    format!("订单记录 {} 的状态词认不出:{w}", r.id),
                )
            })?,
        };
        Ok(Order {
            id: r.id.clone(),
            case_id: need("caseId")?,
            instance: need("instance")?,
            buyer_key: need("buyerKey")?,
            buyer_payout: get("buyerPayout").unwrap_or_default(),
            verdict: get("verdict").unwrap_or_default(),
            token: get("token").unwrap_or_default(),
            need: get("need").unwrap_or_default(),
            open_by: int("openBy")?,
            candidate: int("candidate")?,
            expiry: int("expiry")?,
            state,
            balance: get("balance").unwrap_or_default(),
            gap: get("gap").unwrap_or_default(),
            note: get("note").unwrap_or_default(),
            notice: get("notice").unwrap_or_default(),
            notice_at: int("noticeAt")?,
            deploy_tx: get("deployTx").unwrap_or_default(),
            expire_tx: get("expireTx").unwrap_or_default(),
            source: get("source").unwrap_or_default(),
            opened: int("openedBlock")?,
            extra: r
                .fields
                .iter()
                .filter(|(k, _)| !OWN.contains(&k.as_str()))
                .cloned()
                .collect(),
        })
    }

    pub fn extra(&self, k: &str) -> Option<&str> {
        self.extra
            .iter()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.as_str())
    }

    pub fn set_extra(&mut self, k: &str, v: &str) {
        match self.extra.iter_mut().find(|(a, _)| a == k) {
            Some(x) => x.1 = v.to_string(),
            None => self.extra.push((k.to_string(), v.to_string())),
        }
    }

    pub fn set_state(&mut self, s: State) {
        self.state = s;
        if leaves_stale(s) {
            self.notice.clear();
            self.notice_at = 0;
            self.extra.retain(|(k, _)| k != STALE_FACTS);
        }
    }

    pub fn deployed(&mut self, tx: &str) {
        self.set_state(State::Deployed);
        if !tx.is_empty() {
            self.deploy_tx = tx.to_string();
        }
        self.note.clear();
        self.clear_pending();
    }

    pub fn pending(&self) -> Option<(String, String)> {
        let tx = self.extra(PENDING_TX).filter(|x| !x.is_empty())?;
        Some((self.extra(PENDING_WHAT).unwrap_or_default().to_string(), tx.to_string()))
    }


    pub fn set_pending(&mut self, what: &str, tx: &str, b: &Broadcast) {
        self.set_extra(PENDING_WHAT, what);
        self.set_extra(PENDING_TX, tx);
        self.set_extra(PENDING_NONCE, &b.nonce.to_string());
        self.set_extra(PENDING_FEES, &format!("{} {} {}", b.max_fee, b.priority_fee, b.gas_limit));
    }

    pub fn pending_at(&self) -> Option<Broadcast> {
        let nonce = self.pending_nonce()?;
        let fees = self.extra(PENDING_FEES)?;
        let mut it = fees.split(' ');
        let (max_fee, priority_fee, gas) = (it.next()?.to_string(), it.next()?.to_string(), it.next()?.parse().ok()?);
        Some(Broadcast { nonce, max_fee, priority_fee, gas_limit: gas })
    }

    pub fn pending_nonce(&self) -> Option<u64> {
        self.extra(PENDING_NONCE).and_then(|n| n.parse().ok())
    }

    pub fn clear_pending(&mut self) {
        self.extra.retain(|(k, _)| k != PENDING_TX && k != PENDING_WHAT && k != PENDING_NONCE && k != PENDING_FEES);
    }
}

const PENDING_TX: &str = "pendingTx";
const PENDING_WHAT: &str = "pendingWhat";
const PENDING_NONCE: &str = "pendingNonce";
const PENDING_FEES: &str = "pendingFees";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Broadcast {
    pub nonce: u64,
    pub max_fee: String,
    pub priority_fee: String,
    pub gas_limit: u64,
}

fn store_why(e: eravon_store::fault::Fault) -> Why {
    why(e.name(), e.detail())
}

pub fn still_current(w: &crate::Wire<'_>, key: &str) -> Result<bool, Why> {
    let cur = eravon_standard::chain::current(w.t, w.table, key).map_err(|e| why(&e.name(), e.detail()))?;
    Ok(cur.value.eq_ignore_ascii_case(key))
}

pub fn still_acting(w: &crate::Wire<'_>, key: &str, instance: &str) -> Result<bool, Why> {
    let g = eravon_standard::chain::case_generation(w.t, w.table, instance, w.logic, None).map_err(|e| why(&e.name(), e.detail()))?;
    let acting = eravon_standard::chain::acting_key(w.t, w.table, key, g.is_past()).map_err(|e| why(&e.name(), e.detail()))?;
    Ok(acting.eq_ignore_ascii_case(key))
}

pub fn owner_of(data: &str, me: &str, id: &str) -> String {
    let Ok(s) = eravon_vault::store::open_read(data) else { return me.to_string() };
    for o in eravon_vault::store::archive_of(data, me).unwrap_or_else(|_| vec![me.to_string()]) {
        if s.get(&o, Box_::Orders, id).is_ok() {
            return o;
        }
    }
    me.to_string()
}

pub fn load(data: &str, me: &str, id: &str) -> Result<Order, Why> {
    let s = eravon_vault::store::open_read(data).map_err(store_why)?;
    let r = s.get(&owner_of(data, me, id), Box_::Orders, id).map_err(store_why)?;
    if r.kind != Kind::Receipt {
        return Err(why(
            "E_WATCH_RECORD",
            format!("{id} 不是订单记录,是 {}", r.kind.word()),
        ));
    }
    Order::of(&r)
}

pub fn all(data: &str, me: &str) -> Result<Vec<Order>, Why> {
    let s = eravon_vault::store::open_read(data).map_err(store_why)?;
    let mut out = Vec::new();
    for owner in eravon_vault::store::archive_of(data, me).map_err(store_why)? {
        let rows = match s.list(&owner, Box_::Orders) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(store_why(e)),
        };
        for r in rows.iter().filter(|r| r.kind == Kind::Receipt) {
            out.push(Order::of(r)?);
        }
    }
    Ok(out)
}

pub fn part(data: &str, me: &str, id: &str, name: &str) -> Result<Vec<u8>, Why> {
    let s = eravon_vault::store::open_read(data).map_err(store_why)?;
    s.part(&owner_of(data, me, id), Box_::Orders, id, name).map_err(store_why)
}

pub fn save(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    o: &Order,
    parts: &[(String, Vec<u8>)],
) -> Result<(), Why> {
    let owner = owner_of(data, me, &o.id);
    let s = eravon_vault::store::open_write(data, writer, now).map_err(store_why)?;
    s.put(&owner, Kind::Receipt, &o.id, &o.fields(), parts, now)
        .map_err(store_why)?;
    Ok(())
}
