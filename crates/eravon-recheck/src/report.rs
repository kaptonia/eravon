use eravon_literals::json::Out;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Pass,
    Mismatch,
    Unread,
}

impl State {
    pub const ALL: [State; 3] = [State::Pass, State::Mismatch, State::Unread];

    pub const fn word(self) -> &'static str {
        match self {
            State::Pass => "pass",
            State::Mismatch => "mismatch",
            State::Unread => "unread",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub item: Name,
    pub law: &'static str,
    pub row: Option<(usize, String)>,
    pub state: State,
    pub read: String,
    pub why: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Name {
    Posted,
    Code,
    RowTx,
    RowTimestamp,
    RowKind,
    OpenedBuyer,
    OpenedSeller,
    OpenedCaller,
    SettledBuyer,
    SettledSeller,
    SeatedEvent,
    SeatedSeatOf,
    Beyond,
    PanelFrozen,
    Terminal,
    Anchored,
    EntryJudge,
    EntryAmanah,
    EntryDecision,
    Ledger,
    Complete,
}

impl Name {
    pub const ALL: [Name; 21] = [
        Name::Posted,
        Name::Code,
        Name::RowTx,
        Name::RowTimestamp,
        Name::RowKind,
        Name::OpenedBuyer,
        Name::OpenedSeller,
        Name::OpenedCaller,
        Name::SettledBuyer,
        Name::SettledSeller,
        Name::SeatedEvent,
        Name::SeatedSeatOf,
        Name::Beyond,
        Name::PanelFrozen,
        Name::Terminal,
        Name::Anchored,
        Name::EntryJudge,
        Name::EntryAmanah,
        Name::EntryDecision,
        Name::Ledger,
        Name::Complete,
    ];

    pub const fn word(self) -> &'static str {
        match self {
            Name::Posted => "posted",
            Name::Code => "code",
            Name::RowTx => "row.tx",
            Name::RowTimestamp => "row.timestamp",
            Name::RowKind => "row.kind",
            Name::OpenedBuyer => "opened.buyer",
            Name::OpenedSeller => "opened.seller",
            Name::OpenedCaller => "opened.caller",
            Name::SettledBuyer => "settled.buyer",
            Name::SettledSeller => "settled.seller",
            Name::SeatedEvent => "seated.event",
            Name::SeatedSeatOf => "seated.seatOf",
            Name::Beyond => "beyond",
            Name::PanelFrozen => "panelFrozen",
            Name::Terminal => "terminal",
            Name::Anchored => "anchored",
            Name::EntryJudge => "entry.judge",
            Name::EntryAmanah => "entry.amanah",
            Name::EntryDecision => "entry.decision",
            Name::Ledger => "ledger",
            Name::Complete => "complete",
        }
    }
}

impl Item {
    pub fn pass(item: Name, law: &'static str, read: String) -> Item {
        Item { item, law, row: None, state: State::Pass, read, why: None }
    }
    pub fn mismatch(item: Name, law: &'static str, read: String) -> Item {
        Item { item, law, row: None, state: State::Mismatch, read, why: None }
    }
    pub fn unread(item: Name, law: &'static str, read: String, why: String) -> Item {
        Item { item, law, row: None, state: State::Unread, read, why: Some(why) }
    }
    pub fn at(mut self, i: usize, kind: &str) -> Item {
        self.row = Some((i, kind.to_string()));
        self
    }

    pub fn out(&self) -> Out {
        let mut m = vec![
            ("item", Out::S(self.item.word().to_string())),
            ("law", Out::S(self.law.to_string())),
            ("read", Out::S(self.read.clone())),
            ("state", Out::S(self.state.word().to_string())),
        ];
        if let Some((i, k)) = &self.row {
            m.push(("row", Out::N(*i as u64)));
            m.push(("kind", Out::S(k.clone())));
        }
        if let Some(w) = &self.why {
            m.push(("why", Out::S(w.clone())));
        }
        Out::O(m)
    }
}

pub fn closed() -> Out {
    let words = |v: Vec<&'static str>| Out::A(v.into_iter().map(|w| Out::S(w.to_string())).collect());
    Out::O(vec![("item", words(Name::ALL.map(Name::word).to_vec())), ("state", words(State::ALL.map(State::word).to_vec()))])
}

pub fn counts(items: &[Item]) -> Out {
    let n = |s: State| items.iter().filter(|x| x.state == s).count() as u64;
    Out::O(vec![
        ("mismatch", Out::N(n(State::Mismatch))),
        ("pass", Out::N(n(State::Pass))),
        ("unread", Out::N(n(State::Unread))),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_has_its_own_word() {
        let w = State::ALL.map(State::word);
        assert_eq!(w, ["pass", "mismatch", "unread"]);
    }

    #[test]
    fn the_lock_lists_the_same_words() {
        let items = Name::ALL.map(Name::word);
        let states = State::ALL.map(State::word);
        for v in [&items[..], &states[..]] {
            let mut seen = std::collections::BTreeSet::new();
            assert!(v.iter().all(|w| !w.is_empty() && seen.insert(*w)), "{v:?}");
        }
        let lock = include_bytes!("../../../tools/schemalock/lock/erk-schema.json");
        let doc = eravon_literals::json::read(lock).expect("the lock reads as JSON");
        let set = doc.get("closed").expect("the lock has a closed block");
        let got = |k: &str| -> Vec<String> { set.get(k).and_then(|x| x.as_arr()).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(str::to_string)).collect() };
        assert_eq!(got("item"), items.map(str::to_string));
        assert_eq!(got("state"), states.map(str::to_string));
    }

    #[test]
    fn an_unread_row_carries_its_reason() {
        let i = Item::unread(Name::OpenedBuyer, "§5.1", "x".into(), "端点不供调用追踪".into());
        let b = eravon_literals::json::write(&i.out());
        let s = String::from_utf8(b).unwrap_or_default();
        assert!(s.contains("\"why\":\"端点不供调用追踪\""), "{s}");
        assert!(s.contains("\"state\":\"unread\""), "{s}");
    }
}
