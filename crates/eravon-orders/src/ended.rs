use std::collections::BTreeMap;

use eravon_dispute::view::{CaseView, Ev};
use eravon_galeed as k1;
use eravon_galeed::Value;
use eravon_terms::u256::U256;

pub const FILE: &str = "ended-cases.json";

pub const FORM: &str = "eravon.ended-cases/1";

pub const TERMINALS: std::ops::RangeInclusive<u64> = 4..=9;

pub const CLOSED: &str = "Closed";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct End {
    pub tx: String,
    pub block: u64,
    pub log_index: u64,
}

#[derive(Clone, Debug)]
pub struct Kept {
    pub end: End,
    pub view: CaseView,
}

pub type Record = BTreeMap<(u64, String), Kept>;

fn closed(events: &[Ev]) -> Option<End> {
    events.iter().rev().find(|e| e.name == CLOSED).map(|e| End { tx: e.tx.to_ascii_lowercase(), block: e.block, log_index: e.log_index })
}

pub fn ended(v: &CaseView) -> Option<End> {
    TERMINALS.contains(&v.phase).then(|| closed(&v.events)).flatten()
}

pub fn still(k: &Kept, events: &[Ev]) -> Option<CaseView> {
    (closed(events).as_ref() == Some(&k.end)).then(|| CaseView { events: events.to_vec(), ..k.view.clone() })
}

pub fn serves(kept: &Record, chain_id: u64, instance: &str, events: Option<&[Ev]>) -> bool {
    match (kept.get(&(chain_id, instance.to_ascii_lowercase())), events) {
        (Some(k), Some(evs)) => still(k, evs).is_some(),
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn case_of(w: &crate::Wire<'_>, kept: &Record, new: &mut Record, instance: &str, case_id: &str, opened: u64, events: Option<Vec<Ev>>) -> Result<Option<CaseView>, crate::Why> {
    let key = (w.chain_id, instance.to_ascii_lowercase());
    if let (Some(k), Some(evs)) = (kept.get(&key), events.as_deref()) {
        if let Some(v) = still(k, evs) {
            return Ok(Some(v));
        }
    }
    let read = eravon_dispute::view::case_if_deployed_with(w, instance, case_id, opened, events)?;
    if let Some(v) = &read {
        if let Some(end) = ended(v) {
            if kept.get(&key).is_none_or(|k| k.end != end) {
                new.insert(key, Kept { end, view: CaseView { events: Vec::new(), ..v.clone() } });
            }
        }
    }
    Ok(read)
}

pub fn known() -> Record {
    eravon_scan::judged::kept(FILE).map(|b| read(&b)).unwrap_or_default()
}

pub fn add(new: Record) {
    if new.is_empty() {
        return;
    }
    eravon_scan::judged::keep(FILE, |now| {
        let mut all = now.map(|b| read(&b)).unwrap_or_default();
        all.extend(new);
        write(&all)
    });
}

fn big(v: &U256) -> Value {
    Value::Str(v.to_dec())
}

pub fn write(all: &Record) -> Option<Vec<u8>> {
    let rows = all
        .iter()
        .map(|((chain, instance), k)| {
            let v = &k.view;
            let mut m = vec![
                ("accepted".to_string(), Value::Int(v.accepted)),
                ("amount".to_string(), big(&v.amount)),
                ("buyerPayout".to_string(), Value::Str(v.buyer_payout.clone())),
                ("chainId".to_string(), Value::Int(*chain)),
                ("closedBlock".to_string(), Value::Int(k.end.block)),
                ("closedLogIndex".to_string(), Value::Int(k.end.log_index)),
                ("closedTx".to_string(), Value::Str(k.end.tx.clone())),
                ("deposit".to_string(), big(&v.deposit)),
                ("instance".to_string(), Value::Str(instance.clone())),
                ("judge".to_string(), Value::Str(v.judge.clone())),
                ("lf".to_string(), big(&v.lf)),
                ("lfHeld".to_string(), big(&v.lf_held)),
                ("logic".to_string(), Value::Str(v.logic.clone())),
                ("past".to_string(), Value::Bool(v.past)),
                ("pendingRedraw".to_string(), Value::Bool(v.pending_redraw)),
                ("phase".to_string(), Value::Int(v.phase)),
                ("premium".to_string(), big(&v.premium)),
                ("redrawDeadline".to_string(), Value::Int(v.redraw_deadline)),
                ("seats".to_string(), Value::Int(v.seats)),
                ("sellerFund".to_string(), Value::Str(v.seller_fund.clone())),
                ("sellerPayout".to_string(), Value::Str(v.seller_payout.clone())),
                ("token".to_string(), Value::Str(v.token.clone())),
                ("windowEnd".to_string(), Value::Int(v.window_end)),
            ];
            if let Some(s) = v.seat_state {
                m.push(("seatState".to_string(), Value::Int(s)));
            }
            m.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Obj(m)
        })
        .collect();
    k1::canon::canonical(&Value::Obj(vec![("cases".to_string(), Value::Arr(rows)), ("form".to_string(), Value::Str(FORM.to_string()))])).ok()
}

pub fn read(bytes: &[u8]) -> Record {
    let mut out = Record::new();
    let Ok(v) = k1::canon::read(bytes) else { return out };
    if v.get("form").and_then(Value::as_str) != Some(FORM) {
        return out;
    }
    let rows = v.get("cases").and_then(Value::as_arr).cloned().unwrap_or_default();
    for row in rows {
        let int = |k: &str| match row.get(k) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        };
        let s = |k: &str| row.get(k).and_then(Value::as_str).map(str::to_string);
        let b = |k: &str| match row.get(k) {
            Some(Value::Bool(x)) => Some(*x),
            _ => None,
        };
        let n = |k: &str| s(k).and_then(|x| U256::from_dec(&x));
        let got = (|| {
            let view = CaseView {
                phase: int("phase")?,
                seats: int("seats")?,
                premium: n("premium")?,
                deposit: n("deposit")?,
                lf_held: n("lfHeld")?,
                pending_redraw: b("pendingRedraw")?,
                redraw_deadline: int("redrawDeadline")?,
                judge: s("judge")?,
                accepted: int("accepted")?,
                window_end: int("windowEnd")?,
                seat_state: int("seatState"),
                amount: n("amount")?,
                token: s("token")?,
                buyer_payout: s("buyerPayout")?,
                seller_fund: s("sellerFund")?,
                seller_payout: s("sellerPayout")?,
                lf: n("lf")?,
                events: Vec::new(),
                past: b("past")?,
                logic: s("logic")?,
            };
            let end = End { tx: s("closedTx")?, block: int("closedBlock")?, log_index: int("closedLogIndex")? };
            TERMINALS.contains(&view.phase).then_some(((int("chainId")?, s("instance")?.to_ascii_lowercase()), Kept { end, view }))
        })();
        if let Some((k, v)) = got {
            out.insert(k, v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(name: &str, block: u64, tx: &str) -> Ev {
        Ev { name: name.into(), block, log_index: 2, tx: tx.into(), args: Vec::new() }
    }

    fn view(phase: u64, events: Vec<Ev>) -> CaseView {
        let n = |x: u64| U256::from_dec(&x.to_string()).unwrap();
        CaseView {
            phase,
            seats: 1,
            premium: n(0),
            deposit: n(0),
            lf_held: n(0),
            pending_redraw: false,
            redraw_deadline: 0,
            judge: "0x11".into(),
            accepted: 7,
            window_end: 1_800_000_000,
            seat_state: Some(2),
            amount: n(123_456_789),
            token: "0x22".into(),
            buyer_payout: "0x33".into(),
            seller_fund: "0x44".into(),
            seller_payout: "0x55".into(),
            lf: n(9),
            events,
            past: false,
            logic: "0x66".into(),
        }
    }

    #[test]
    fn only_a_terminal_phase_with_its_closed_counts_as_ended() {
        let closed = || vec![ev("Opened", 10, "0xa"), ev(CLOSED, 20, "0xB")];
        for p in 0..=3 {
            assert_eq!(ended(&view(p, closed())), None, "live phase {p}");
        }
        assert_eq!(ended(&view(0, vec![ev("Refunded", 15, "0xc")])), None, "expiry branch");
        for p in 4..=9 {
            assert_eq!(ended(&view(p, closed())), Some(End { tx: "0xb".into(), block: 20, log_index: 2 }), "terminal {p}");
        }
        assert_eq!(ended(&view(5, vec![ev("Confirmed", 20, "0xb")])), None, "no Closed read");
        assert_eq!(ended(&view(10, closed())), None, "outside the set");
    }

    #[test]
    fn a_kept_reading_stands_only_on_its_own_closed() {
        let k = Kept { end: End { tx: "0xb".into(), block: 20, log_index: 2 }, view: view(5, Vec::new()) };
        let now = vec![ev(CLOSED, 20, "0xB"), ev("Swept", 30, "0xd")];
        let v = still(&k, &now).expect("stands");
        assert_eq!((v.phase, v.events.len()), (5, 2));
        assert!(still(&k, &[ev(CLOSED, 21, "0xb")]).is_none(), "moved block");
        assert!(still(&k, &[ev(CLOSED, 20, "0xe")]).is_none(), "another transaction");
        assert!(still(&k, &[ev("Opened", 10, "0xa")]).is_none(), "gone");
    }

    #[test]
    fn the_record_reads_back_what_it_wrote() {
        let mut all = Record::new();
        all.insert((1, "0xab".into()), Kept { end: End { tx: "0xb".into(), block: 20, log_index: 2 }, view: view(8, Vec::new()) });
        let mut no_seat = view(4, Vec::new());
        no_seat.seat_state = None;
        all.insert((11155111, "0xcd".into()), Kept { end: End { tx: "0xc".into(), block: 21, log_index: 0 }, view: no_seat });
        let back = read(&write(&all).expect("writes"));
        assert_eq!(back.len(), 2);
        let a = &back[&(1, "0xab".to_string())];
        assert_eq!((a.view.phase, a.view.amount.to_dec(), a.view.seat_state, a.view.judge.as_str(), a.end.block), (8, "123456789".to_string(), Some(2), "0x11", 20));
        assert_eq!(back[&(11155111, "0xcd".to_string())].view.seat_state, None);
        assert!(read(b"{\"cases\":[],\"form\":\"other/1\"}").is_empty());
    }

    #[test]
    fn a_row_kept_as_ended_asks_the_chain_nothing() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let t = eravon_chain::transport::Auto::web_roots().expect("roots");
        let table = eravon_chain::endpoints::Table::new(vec!["http://127.0.0.1:1".to_string()]).expect("table");
        let _ = table.chain.set(1);
        let w = crate::Wire { t: &t, table: &table, chain_id: 1, logic: "0x66", registries: &[] };
        let mut kept = Record::new();
        kept.insert((1, "0xab".into()), Kept { end: End { tx: "0xb".into(), block: 20, log_index: 2 }, view: view(5, Vec::new()) });
        let now = vec![ev(CLOSED, 20, "0xb")];
        let mut new = Record::new();
        let got = case_of(&w, &kept, &mut new, "0xAB", "0x01", 10, Some(now.clone())).expect("asks nothing");
        assert_eq!(got.map(|v| v.phase), Some(5));
        assert!(new.is_empty());
        assert!(case_of(&w, &Record::new(), &mut new, "0xab", "0x01", 10, Some(now)).is_err(), "not kept: asked");
        assert!(case_of(&w, &kept, &mut new, "0xab", "0x01", 10, Some(vec![ev("Opened", 10, "0xa")])).is_err(), "Closed gone: asked");
        assert!(case_of(&w, &kept, &mut new, "0xab", "0x01", 10, None).is_err(), "events not read: asked");
    }
}
