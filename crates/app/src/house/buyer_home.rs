use eravon_home::notes::{self, Kind, Note};
use eravon_literals::json::Out;
use eravon_orders::status::Word;
use eravon_orders::table;

use crate::house::buyer_orders::BRow;
use crate::house::seam;
use crate::house::seller::Said;

fn said((n, d): eravon_home::Why) -> Said {
    Said::bad(&n, d)
}

pub const ORDER_ROWS: usize = 6;
pub const ACTIVITY_ROWS: usize = 10;

#[derive(Clone, Debug, Default)]
pub struct BHome {
    pub me: String,
    pub anchor: String,
    pub stopped: bool,
    pub now: u64,
    pub active: Option<Result<u64, Said>>,
    pub done: Option<Result<u64, Said>>,
    pub balance: Option<Result<eravon_wallet::balance::Balances, Said>>,
    pub orders: Vec<BRow>,
    pub activity: Option<Result<Vec<eravon_home::activity::Act>, Said>>,
    pub notes: Option<Result<Vec<Note>, Said>>,
    pub marks: Vec<(String, bool, bool)>,
    pub badge: u64,
    pub system: Vec<Note>,
    pub toast: Option<Said>,
    pub copied: Option<String>,
    pub notify_failed: Vec<String>,
}

fn active_of(rows: &[BRow]) -> u64 {
    rows.iter().filter(|r| !table::done(&r.row)).count() as u64
}


fn confirm(out: &mut Vec<Note>, name: &str, r: &BRow) {
    for end in confirm_ends(r) {
        out.push(notes::note(Kind::Confirm, name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNoteConfirm; name.to_string(), eravon_orders::when::day(end).to_string()), end));
    }
}


fn confirm_ends(r: &BRow) -> Vec<u64> {
    vec![r.row.deadline]
}


pub fn resumes(asked: bool, had: bool) -> bool {
    asked || had
}


fn name_of(r: &BRow) -> String {
    if r.row.number.is_empty() && !r.product.is_empty() { r.product.clone() } else { table::name(&r.row) }
}

impl BHome {
    pub fn open(s: &eravon_vault::session::Session, data: &str, me: &str) -> BHome {
        crate::house::mark::mark(crate::house::mark::FEATURE_P8);
        if !s.is_open() || me.is_empty() {
            return BHome { me: me.to_ascii_lowercase(), stopped: true, ..BHome::default() };
        }
        let mut h = BHome { me: me.to_ascii_lowercase(), ..BHome::default() };
        let bo = crate::house::buyer_orders::BOrders::load(s, data, me);
        h.anchor = bo.owners.first().cloned().unwrap_or_else(|| me.to_ascii_lowercase());
        let anchor = h.anchor.clone();
        let rows: Result<Vec<BRow>, Said> = match &bo.unscanned {
            Some(u) => Err(u.clone()),
            None => bo.rows.clone().unwrap_or_else(|| Err(Said::bad("E_APP_SCRIPT", crate::house::i18n::t(crate::house::i18n::Key::BhTableUnread)))),
        };
        let wire = match crate::house::orders::wire(data, me) {
            Ok(w) => w,
            Err(e) => {
                h.toast = (e.name != eravon_watch::E_NO_ENDPOINT).then(|| e.clone());
                h.active = Some(Err(e.clone()));
                h.done = Some(Err(e.clone()));
                h.balance = Some(Err(e.clone()));
                h.activity = Some(Err(e.clone()));
                h.notes = Some(Err(e));
                return h;
            }
        };
        let sw = crate::house::orders::sw(&wire);
        let now = eravon_home::chain_now(&sw).map(|x| x.0).map_err(said);
        h.now = now.clone().unwrap_or(0);
        let tokens = rows.as_ref().map(|v| crate::house::wallet::tokens_in(v.iter().map(|r| r.row.token.as_str()))).unwrap_or_default();
        let bal = eravon_wallet::balance::read(&wire, data, me, &tokens);
        let rows = rows.map(|mut v| {
            for r in v.iter_mut().filter(|r| r.row.decimals.is_none()) {
                r.row.decimals = bal.coins.iter().find(|c| c.token.eq_ignore_ascii_case(&r.row.token)).and_then(|c| c.decimals);
            }
            v
        });
        h.balance = Some(Ok(bal));
        match &rows {
            Ok(v) => {
                h.active = Some(Ok(active_of(v)));
                h.done = Some(Ok(v.iter().filter(|r| table::done(&r.row)).count() as u64));
                h.orders = v.iter().filter(|r| !table::done(&r.row)).take(ORDER_ROWS).cloned().collect();
            }
            Err(e) => {
                h.active = Some(Err(e.clone()));
                h.done = Some(Err(e.clone()));
                h.toast = Some(e.clone());
            }
        }
        h.activity = Some(activity(&sw, data, &bo.owners, &rows));
        let ns = rows.as_ref().map_err(Clone::clone).and_then(|v| now.clone().and_then(|t| gather(&wire, data, v, t)));
        let mut local_err: Option<Said> = None;
        let marks = notes::marks(data, &anchor).map_err(|e| local_err = Some(said(e))).ok();
        let (badge, system) = match (&ns, &marks) {
            (Ok(v), Some(m)) => (notes::badge(v, m), notes::due(v, h.now, m).into_iter().cloned().collect::<Vec<Note>>()),
            (Ok(v), None) => (v.len() as u64, Vec::new()),
            (Err(_), _) => (0, Vec::new()),
        };
        h.marks = match notes::marks(data, &anchor) {
            Ok(m) => m,
            Err(e) => {
                local_err = Some(said(e));
                marks.unwrap_or_default()
            }
        };
        h.badge = badge;
        h.system = system;
        if h.toast.is_none() {
            h.toast = ns.as_ref().err().cloned().or(local_err);
        }
        h.notes = Some(ns);
        h
    }

    pub fn fresh_for(&self, addr: &str) -> bool {
        self.me.eq_ignore_ascii_case(addr)
    }

    pub fn handle(&mut self, data: &str, now: u64, key: &str) -> Result<(), Said> {
        notes::handle(data, seam::WRITER, now, &self.anchor, key).map_err(said)?;
        self.marks = notes::marks(data, &self.anchor).map_err(said)?;
        if let Some(Ok(v)) = &self.notes {
            self.badge = notes::badge(v, &self.marks);
        }
        Ok(())
    }

    pub fn item(&self, key: &str, item: &str) -> Option<String> {
        let v = self.notes.as_ref()?.as_ref().ok()?;
        let n = v.iter().find(|n| n.key == key)?;
        match item {
            "instance" => Some(n.instance.clone()).filter(|x| !x.is_empty()),
            _ => n.items.iter().find(|(k, _)| k == item).map(|(_, x)| x.clone()),
        }
    }

    pub fn rows(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        let mut m: Vec<(&'static str, Out)> = vec![("me", s(&self.me)), ("now", Out::N(self.now)), ("stopped", Out::B(self.stopped))];
        for (k, v) in [("active", &self.active), ("done", &self.done)] {
            match v {
                Some(Ok(n)) => m.push((k, Out::N(*n))),
                Some(Err(e)) => m.push((k, e.out())),
                None => {}
            }
        }
        match &self.balance {
            Some(Ok(b)) => {
                let one = |r: &Result<String, eravon_wallet::fault::Fault>| match r {
                    Ok(v) => Out::S(v.clone()),
                    Err(f) => Said::bad(&f.token, f.detail.clone()).out(),
                };
                let held: Vec<Out> = b
                    .coins
                    .iter()
                    .filter(|c| c.have.as_ref().is_ok_and(|x| x != "0"))
                    .map(|c| Out::O(vec![("have", one(&c.have)), ("token", s(&c.token)), ("word", s(&crate::house::coin::word(c.have.as_deref().unwrap_or("0"), c.decimals, &c.token, crate::house::coin::Form::Short)))]))
                    .collect();
                let unread: Vec<Out> = b.coins.iter().filter(|c| c.have.is_err()).map(|c| s(&c.token)).collect();
                m.push(("balance", Out::O(vec![("coins", Out::A(held)), ("native", one(&b.native)), ("unread", Out::A(unread))])));
            }
            Some(Err(e)) => m.push(("balance", e.out())),
            None => {}
        }
        m.push(("orders", Out::A(self.orders.iter().map(|r| Out::O(vec![("instance", s(&r.row.instance)), ("name", s(&name_of(r)))])).collect())));
        match &self.activity {
            Some(Ok(v)) => m.push((
                "activity",
                Out::A(v.iter().map(crate::house::home::act_out).collect()),
            )),
            Some(Err(e)) => m.push(("activity", e.out())),
            None => {}
        }
        match &self.notes {
            Some(Ok(v)) => m.push((
                "notes",
                Out::A(
                    v.iter()
                        .map(|n| {
                            let handled = self.marks.iter().any(|(k, h, _)| *k == n.key && *h);
                            Out::O(vec![
                                ("deadline", Out::N(n.deadline)),
                                ("handled", Out::B(handled)),
                                ("instance", s(&n.instance)),
                                ("key", s(&n.key)),
                                ("kind", s(n.kind.word())),
                                ("say", s(&n.say)),
                                ("subject", s(&n.subject)),
                            ])
                        })
                        .collect(),
                ),
            )),
            Some(Err(e)) => m.push(("notes", e.out())),
            None => {}
        }
        m.push(("badge", Out::N(self.badge)));
        m.push(("system", Out::A(self.system.iter().map(|n| Out::O(vec![("key", s(&n.key)), ("say", s(&n.say))])).collect())));
        if let Some(t) = &self.toast {
            m.push(("toast", t.out()));
        }
        if let Some(c) = &self.copied {
            m.push(("copied", s(c)));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}

fn activity(w: &eravon_watch::Wire<'_>, data: &str, owners: &[String], rows: &Result<Vec<BRow>, Said>) -> Result<Vec<eravon_home::activity::Act>, Said> {
    let rows = rows.as_ref().map_err(Clone::clone)?;
    let t = table::Table { rows: rows.iter().map(|r| r.row.clone()).collect(), foreign: 0, agreement: "agreed".into(), unscanned: None };
    let mut all = eravon_home::activity::recent(w, data, owners.first().map(String::as_str).unwrap_or_default(), &Ok(t), usize::MAX).map_err(said)?;
    for o in owners {
        for tx in eravon_wallet::tx::list(data, o).map_err(|f| Said::bad(&f.token, f.detail))? {
            if tx.hash.is_empty() || all.iter().any(|a| a.tx.eq_ignore_ascii_case(&tx.hash)) {
                continue;
            }
            let order = rows.iter().find(|r| r.row.instance.eq_ignore_ascii_case(&tx.order)).map(name_of).unwrap_or_else(|| tx.order.clone());
            let say = eravon_wallet::tx::Kind::of(&tx.kind).map(|k| k.say().to_string()).unwrap_or_else(|| tx.kind.clone());
            all.push(eravon_home::activity::Act { at: tx.at, order, say, tx: tx.hash.clone(), unread: None });
        }
    }
    all.sort_by(eravon_home::activity::Act::order_of);
    let unread = all.iter().filter(|a| a.unread.is_some()).count();
    all.truncate(ACTIVITY_ROWS + unread);
    Ok(all)
}

fn gather(w: &eravon_cli::shared::Wire, data: &str, rows: &[BRow], now: u64) -> Result<Vec<Note>, Said> {
    let mut out = Vec::new();
    let a1s = |e: eravon_standard::fault::Fault| Said::bad(&e.name(), e.detail());
    for r in rows {
        let name = name_of(r);
        let day = eravon_orders::when::day;
        match r.row.status.as_ref().ok().map(|s| s.word) {
            Some(Word::AwaitConfirm) => confirm(&mut out, &name, r),
            Some(Word::Disputed) => {
                out.push(notes::note(Kind::Disputed, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNoteDisputed; name, day(r.row.deadline).to_string()), r.row.deadline));
                if let Some(o) = crate::house::buyer_orders::offer_text(data, r).and_then(|t| offer_expiry(&t)).filter(|x| *x > now) {
                    out.push(notes::note(Kind::Offer, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNoteOffer; name, day(o).to_string()), o));
                }
            }
            Some(Word::AwaitRedraw) => out.push(notes::note(Kind::Redraw, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNoteRedraw; name, day(r.row.deadline).to_string()), r.row.deadline)),
            Some(Word::AwaitDeploy | Word::AwaitPay) => out.extend(row_notes(&name, &r.row.instance, (|| {
                let mut v = Vec::new();
                let at = eravon_standard::chain::erc20_balance(&w.t, &w.table, &r.row.token, &r.row.instance).map_err(a1s)?;
                let have = eravon_terms::u256::U256::from_be(&at.value);
                let want = eravon_terms::u256::U256::from_dec(&r.row.amount).ok_or_else(|| Said::bad("E_APP_AMOUNT", crate::fill!(crate::house::i18n::Key::BhAmountBad; name, format!("{:?}", r.row.amount))))?;
                let closed = eravon_after::act::closed(w, &r.case(), now).map_err(|f| Said::bad(&f.token, f.detail))?;
                if closed.is_none() {
                    if have.cmp_u(&want) != std::cmp::Ordering::Less {
                        v.push(notes::note(Kind::Deploy, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNotePaid; name), 0));
                    } else if crate::house::buyer_orders::deposit_short(w, r) {
                        v.push(notes::note(Kind::Pay, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNoteDepositShort; name), 0));
                    } else {
                        let gap = want.overflowing_sub(&have).0.to_dec();
                        v.push(notes::note(Kind::Pay, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNotePay; name, crate::house::coin::word(&gap, r.row.decimals, &r.row.token, crate::house::coin::Form::Short)), 0));
                    }
                }
                if r.access == eravon_after::act::Access::Key && eravon_after::act::take_from(closed, !have.is_zero()) != eravon_after::act::Take::No {
                    v.push(notes::note(Kind::Expire, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNoteExpired; name), 0));
                }
                Ok(v)
            })())),
            Some(x) if x.done() => out.extend(row_notes(&name, &r.row.instance, (|| {
                let o = eravon_after::act::owed(w, &r.case(), None).map_err(|f| Said::bad(&f.token, f.detail))?;
                Ok(if o.amount != "0" {
                    vec![notes::note(Kind::Owed, &name, &r.row.instance, crate::fill!(crate::house::i18n::Key::BhNoteOwed; name, crate::house::coin::word(&o.amount, r.row.decimals, &r.row.token, crate::house::coin::Form::Short)), 0)]
                } else {
                    Vec::new()
                })
            })())),
            _ => {}
        }
    }
    if let Some(n) = notes::backup_lag(data).map_err(said)? {
        out.push(n);
    }
    Ok(notes::dedupe(out))
}

fn row_notes(name: &str, instance: &str, r: Result<Vec<Note>, Said>) -> Vec<Note> {
    match r {
        Ok(v) => v,
        Err(e) => vec![notes::note(Kind::Unread, name, instance, crate::fill!(crate::house::i18n::Key::BhNoteUnread; name.to_string(), e.name, e.detail), 0)],
    }
}

fn offer_expiry(_text: &str) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_unreadable_row_is_one_note_and_the_rest_stay() {
        let ok = row_notes("甲", "0xaa", Ok(vec![notes::note(Kind::Owed, "甲", "0xaa", "甲 待取回 1".into(), 0)]));
        let bad = row_notes("乙", "0xbb", Err(Said::bad("E_CHAIN_ENDPOINT", "端点读不成")));
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].kind, Kind::Owed);
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].kind, Kind::Unread);
        assert!(bad[0].say.contains("E_CHAIN_ENDPOINT") && bad[0].say.contains("乙"));
        assert_ne!(ok[0].key, bad[0].key);
    }
}

crate::house::merge::lands!(BHome {
    whole: [me],
    data: [anchor, stopped, now, active, done, balance, orders, activity, notes, marks, badge, system, toast, copied, notify_failed],
    page: [],
    keyed(|_: &BHome| ()): [],
    nested: [],
    said: [],
});
