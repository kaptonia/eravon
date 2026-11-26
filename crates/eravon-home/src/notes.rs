use eravon_orders::status::Word;
use eravon_orders::table::{self, Table};
use eravon_store::record::Kind as K;
use eravon_store::seam::Box_;
use eravon_watch::order::State;

use crate::{Why, Wire, why};
use eravon_vault::lang::{fill1, fill2, filln, t, Key};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Deposit,
    Deliver,
    Disputed,
    Redraw,
    Arrived,
    Stale,
    Todo,
    Link,
    Backup,
    Pay,
    Deploy,
    Confirm,
    Expire,
    Owed,
    Offer,
    Unread,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Deposit => "deposit",
            Kind::Deliver => "deliver",
            Kind::Disputed => "disputed",
            Kind::Redraw => "redraw",
            Kind::Arrived => "arrived",
            Kind::Stale => "stale",
            Kind::Todo => "todo",
            Kind::Link => "link",
            Kind::Backup => "backup",
            Kind::Pay => "pay",
            Kind::Deploy => "deploy",
            Kind::Confirm => "confirm",
            Kind::Expire => "expire",
            Kind::Owed => "owed",
            Kind::Offer => "offer",
            Kind::Unread => "unread",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub kind: Kind,
    pub key: String,
    pub subject: String,
    pub instance: String,
    pub say: String,
    pub deadline: u64,
    pub items: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goods {
    pub id: String,
    pub name: String,
    pub why: String,
    pub expiry: u64,
}

fn key(kind: Kind, subject: &str, deadline: u64) -> String {
    format!("{}-{}-{deadline}", kind.word(), subject.to_ascii_lowercase())
}

fn stale_said(name: &str, o: &eravon_watch::order::Order) -> String {
    use eravon_watch::poll::Facts;
    let Some(f) = o.extra(eravon_watch::order::STALE_FACTS).and_then(Facts::read) else { return format!("{name} {}", o.notice) };
    let why = match &f {
        Facts::Round { answer, updated_at, max_stale, .. } => {
            let age = o.notice_at.saturating_sub(*updated_at).to_string();
            filln(Key::HomeStaleRound, &[answer, &eravon_orders::when::short(*updated_at), &age, &max_stale.to_string()])
        }
        Facts::Shape { .. } => t(Key::HomeStaleShape).to_string(),
    };
    filln(Key::HomeNoteStale, &[name, f.feed(), &why])
}

pub fn note(kind: Kind, subject: &str, instance: &str, say: String, deadline: u64) -> Note {
    Note { kind, key: key(kind, if instance.is_empty() { subject } else { instance }, deadline), subject: subject.to_string(), instance: instance.to_string(), say, deadline, items: Vec::new() }
}

fn gather(_w: &Wire<'_>, data: &str, me: &str, t: &Table, goods: &[Goods]) -> Result<Vec<Note>, Why> {
    let mut out = Vec::new();
    for r in &t.rows {
        let name = table::name(r);
        let word = r.status.as_ref().ok().map(|s| s.word);
        let day = |t: u64| eravon_orders::when::day(t);
        match word {
            Some(Word::AwaitDelivery) => out.push(note(Kind::Deliver, &name, &r.instance, fill2(Key::HomeNoteDeliver, &name, &day(r.deadline)), r.deadline)),
            Some(Word::Disputed) => disputed(&mut out, &name, r),
            Some(Word::AwaitRedraw) => out.push(note(Kind::Redraw, &name, &r.instance, fill2(Key::HomeNoteRedraw, &name, &day(r.deadline)), r.deadline)),
            _ => {}
        }
        if let Some(o) = eravon_orders::detail::order_of(data, me, r) {
            match o.state {
                State::Ready => out.push(note(Kind::Arrived, &name, &r.instance, fill1(Key::HomeNoteArrived, &name), 0)),
                State::PendingConsent => out.push(note(Kind::Arrived, &name, &r.instance, fill1(Key::HomeNoteArrivedConsent, &name), 0)),
                State::DepositShort => out.push(note(Kind::Deposit, &name, &r.instance, format!("{name} · {}", o.note), 0)),
                _ => {}
            }
            if !o.notice.is_empty() {
                out.push(note(Kind::Stale, &name, &r.instance, stale_said(&name, &o), 0));
            }
            if let Some(td) = eravon_deliver::todo::stored(&o) {
                let mut n = note(Kind::Todo, &name, &r.instance, fill1(Key::HomeNoteTodo, &name), 0);
                n.items = vec![("grantee".into(), td.grantee), ("work".into(), td.work), ("terms".into(), td.terms_hash)];
                out.push(n);
            }
        }
    }
    for g in goods {
        let say = match g.why.as_str() {
            "expiring" => fill2(Key::HomeNoteLinkExpiring, &g.name, &eravon_orders::when::day(g.expiry)),
            _ => fill1(Key::HomeNoteLinkResign, &g.name),
        };
        out.push(note(Kind::Link, &g.id, "", say, g.expiry));
    }
    if let Some(n) = backup_lag(data)? {
        out.push(n);
    }
    Ok(out)
}

pub fn backup_lag(data: &str) -> Result<Option<Note>, Why> {
    let fam = |f: eravon_vault::fault::Fault| a5w(eravon_vault::records::store_fault(f));
    eravon_vault::records::bind(data).map_err(fam)?;
    let m = eravon_vault::machine::read().map_err(fam)?;
    if let Some(t) = m.backup_failed {
        let say = fill1(Key::HomeNoteBackupFailed, &eravon_orders::when::minute(t));
        let mut out = note(Kind::Backup, BACKUP_SUBJECT, "", say, 0);
        out.key = key(Kind::Backup, BACKUP_SUBJECT, t);
        return Ok(Some(out));
    }
    let now = eravon_vault::backup::measured(m.backup.as_ref()).map_err(fam)?;
    let n = eravon_vault::machine::backup_behind(m.backup.as_ref(), Some(now)).unwrap_or(now);
    let say = match (&m.backup, n) {
        (Some(_), 0) => return Ok(None),
        (Some(b), n) => fill2(Key::HomeNoteBackupSince, &eravon_orders::when::minute(b.at), &n.to_string()),
        (None, 0) => t(Key::HomeNoteBackupNone).to_string(),
        (None, n) => fill1(Key::HomeNoteBackupNever, &n.to_string()),
    };
    let at = m.backup.as_ref().map_or(0, |b| b.at);
    let mut out = note(Kind::Backup, BACKUP_SUBJECT, "", say, 0);
    out.key = key(Kind::Backup, BACKUP_SUBJECT, at);
    Ok(Some(out))
}


pub const BACKUP_SUBJECT: &str = "backup";

fn disputed(out: &mut Vec<Note>, name: &str, r: &table::Row) {
    out.push(note(Kind::Disputed, name, &r.instance, fill2(Key::HomeNoteDisputed, name, &eravon_orders::when::day(r.deadline)), r.deadline));
}


pub fn dedupe(v: Vec<Note>) -> Vec<Note> {
    let mut out: Vec<Note> = Vec::new();
    for n in v {
        if !out.iter().any(|x| x.key == n.key) {
            out.push(n);
        }
    }
    out
}


pub fn notes(w: &Wire<'_>, data: &str, me: &str, t: &Result<Table, Why>, goods: &[Goods]) -> Result<Vec<Note>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let t = t.as_ref().map_err(Clone::clone)?;
    Ok(dedupe(gather(w, data, me, t, goods)?))
}

fn record_id(key: &str) -> String {
    note_id(key)
}


fn note_id(key: &str) -> String {
    let h = eravon_terms::derive::keccak(key.as_bytes());
    let mut s = String::from("note-");
    for b in &h[..16] {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn a5w(e: eravon_store::fault::Fault) -> Why {
    why(e.name(), e.detail())
}

pub fn marks(data: &str, me: &str) -> Result<Vec<(String, bool, bool)>, Why> {
    let s = eravon_vault::store::open_read(data).map_err(a5w)?;
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    for owner in eravon_vault::store::archive_of(data, me).map_err(a5w)? {
        let rows = match s.list(&owner, Box_::Orders) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(a5w(e)),
        };
        for m in rows
            .into_iter()
            .filter(|r| r.kind == K::NotificationRead)
            .filter_map(|r| Some((r.field("key")?.to_string(), r.field("handled") == Some("1"), r.field("sent") == Some("1"))))
        {
            if !out.iter().any(|x| x.0 == m.0) {
                out.push(m);
            }
        }
    }
    Ok(out)
}

fn put(data: &str, writer: &str, now: u64, me: &str, key: &str, handled: bool, sent: bool) -> Result<(), Why> {
    let s = eravon_vault::store::open_write(data, writer, now).map_err(a5w)?;
    let b = |x: bool| if x { "1".to_string() } else { "0".to_string() };
    s.put(me, K::NotificationRead, &record_id(key), &[("handled".into(), b(handled)), ("key".into(), key.to_string()), ("sent".into(), b(sent))], &[], now)
        .map(|_| ())
        .map_err(a5w)
}

pub fn handle(data: &str, writer: &str, now: u64, me: &str, key: &str) -> Result<(), Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let sent = marks(data, me)?.into_iter().any(|(k, _, s)| k == key && s);
    put(data, writer, now, me, key, true, sent)
}

pub fn badge(notes: &[Note], marks: &[(String, bool, bool)]) -> u64 {
    notes.iter().filter(|n| !marks.iter().any(|(k, h, _)| *k == n.key && *h)).count() as u64
}


pub fn orders_to_handle(notes: &[Note], marks: &[(String, bool, bool)]) -> u64 {
    let mut orders: Vec<&str> = Vec::new();
    for n in notes.iter().filter(|n| !n.instance.is_empty() && !marks.iter().any(|(k, h, _)| *k == n.key && *h)) {
        if !orders.iter().any(|o| o.eq_ignore_ascii_case(&n.instance)) {
            orders.push(&n.instance);
        }
    }
    orders.len() as u64
}

pub const LEAD: u64 = 86_400;

pub fn due<'a>(notes: &'a [Note], now: u64, marks: &[(String, bool, bool)]) -> Vec<&'a Note> {
    notes
        .iter()
        .filter(|n| n.deadline != 0 && now <= n.deadline && n.deadline - now <= LEAD)
        .filter(|n| !marks.iter().any(|(k, _, s)| *k == n.key && *s))
        .collect()
}


pub fn sent(data: &str, writer: &str, now: u64, me: &str, keys: &[String]) -> Result<(), Why> {
    if keys.is_empty() {
        return Ok(());
    }
    let m = marks(data, me)?;
    for k in keys {
        let handled = m.iter().any(|(x, h, _)| x == k && *h);
        put(data, writer, now, me, k, handled, true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_ids_fit_the_store_name_gate() {
        let long = key(Kind::Disputed, &format!("0x{}", "ab".repeat(20)), u64::MAX);
        let goods = key(Kind::Link, &"g".repeat(200), 1_790_919_534);
        for k in [long, goods] {
            let id = record_id(&k);
            assert!(eravon_store::seam::name_ok(&id), "{id}");
            assert_eq!(id.len(), 37);
        }
        assert_ne!(record_id("a"), record_id("b"));
    }

    #[test]
    fn to_handle_counts_orders_with_an_unhandled_alert() {
        let (a, b) = (format!("0x{}", "aa".repeat(20)), format!("0x{}", "bb".repeat(20)));
        let notes = vec![
            note(Kind::Deliver, "1", &a, String::new(), 10),
            note(Kind::Deposit, "1", &a, String::new(), 20),
            note(Kind::Disputed, "2", &b, String::new(), 30),
            note(Kind::Backup, "backup", "", String::new(), 0),
        ];
        assert_eq!(orders_to_handle(&notes, &[]), 2, "two orders; the backup alert is no order");
        let handled = vec![(notes[2].key.clone(), true, false)];
        assert_eq!(orders_to_handle(&notes, &handled), 1, "a handled alert is not to handle");
        assert_eq!(orders_to_handle(&notes[3..], &[]), 0, "a backup behind alone is nothing to handle");
    }

    #[test]
    fn a_stale_feeds_alert_is_said_in_the_language_spoken() {
        let feed = format!("0x{}", "ab".repeat(20));
        let mut o = eravon_watch::order::Order {
            id: "1".into(),
            case_id: String::new(),
            instance: String::new(),
            buyer_key: String::new(),
            buyer_payout: String::new(),
            verdict: String::new(),
            token: String::new(),
            need: String::new(),
            open_by: 0,
            candidate: 0,
            expiry: 0,
            state: State::PriceStale,
            balance: String::new(),
            gap: String::new(),
            note: String::new(),
            notice: format!("价源 {feed} 陈旧:旧了;恢复后再发"),
            notice_at: 1_700_003_700,
            deploy_tx: String::new(),
            expire_tx: String::new(),
            source: "paste".into(),
            opened: 0,
            extra: Vec::new(),
        };
        assert_eq!(stale_said("单 1", &o), format!("单 1 {}", o.notice), "no facts: as written");
        let round = eravon_watch::poll::Facts::Round { feed: feed.clone(), answer: "100".into(), updated_at: 1_700_000_000, max_stale: 3600 };
        for (facts, want) in [(round.line(), "3700"), (eravon_watch::poll::Facts::Shape { feed: feed.clone() }.line(), "not a price round")] {
            o.set_extra(eravon_watch::order::STALE_FACTS, &facts);
            eravon_vault::lang::set(eravon_vault::lang::Lang::En);
            let en = stale_said("Order 1", &o);
            eravon_vault::lang::set(eravon_vault::lang::Lang::Zh);
            let zh = stale_said("单 1", &o);
            assert!(en.contains(&feed) && en.contains(want) && !en.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{en}");
            assert!(zh.contains(&feed) && zh.contains("陈旧"), "{zh}");
        }
    }

}
