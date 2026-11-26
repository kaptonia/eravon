use eravon_literals::json::{In, Out};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    pub block: u64,
    pub at: u64,
    pub agreement: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Head {
    pub chain_time: u64,
    pub meta: Meta,
    pub read_wall: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Case {
    pub case_id: String,
    pub instance: String,
    pub seller: String,
    pub buyer: String,
    pub block: u64,
    pub tx: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub instance: String,
    pub code: bool,
    pub columns: Vec<String>,
    pub meta: Meta,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coin {
    pub token: String,
    pub have: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Balances {
    pub holder: String,
    pub native: Option<String>,
    pub coins: Vec<Coin>,
    pub meta: Meta,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Book {
    pub key: String,
    pub chain_id: u64,
    pub logic: String,
    pub endpoints: String,
    pub head: Option<Head>,
    pub cursor: Option<u64>,
    pub scanned: Vec<String>,
    pub cases: Vec<Case>,
    pub scan: Option<Meta>,
    pub foreign: u64,
    pub states: Vec<State>,
    pub balances: Option<Balances>,
    pub kept: std::sync::Arc<eravon_chain::recall::Store>,
    pub tokens: Vec<(String, String)>,
}

impl Book {
    pub fn fresh(key: &str, chain_id: u64, logic: &str) -> Book {
        Book { key: key.to_ascii_lowercase(), chain_id, logic: logic.to_ascii_lowercase(), ..Book::default() }
    }

    pub fn holds(&self, key: &str, chain_id: u64, logic: &str) -> bool {
        self.key.eq_ignore_ascii_case(key) && self.chain_id == chain_id && self.logic.eq_ignore_ascii_case(logic)
    }


    pub fn chain_now(&self, wall: u64) -> Option<u64> {
        self.head.as_ref().map(|h| h.chain_time.saturating_add(wall.saturating_sub(h.read_wall)))
    }

    pub fn covers(&self, keys: &[String]) -> bool {
        self.cursor.is_some() && keys.iter().all(|k| self.scanned.iter().any(|s| s.eq_ignore_ascii_case(k)))
    }

    pub fn void_provenance(&mut self, endpoints: &str) {
        if self.endpoints == endpoints {
            return;
        }
        self.endpoints = endpoints.to_string();
        let mut kept = (*self.kept).clone();
        kept.void();
        self.kept = std::sync::Arc::new(kept);
        let clear = |m: &mut Meta| m.agreement.clear();
        if let Some(h) = self.head.as_mut() {
            clear(&mut h.meta);
        }
        if let Some(m) = self.scan.as_mut() {
            clear(m);
        }
        for s in &mut self.states {
            clear(&mut s.meta);
        }
        if let Some(b) = self.balances.as_mut() {
            clear(&mut b.meta);
        }
    }

    pub fn read_at(&self) -> Option<u64> {
        let mut at = self.head.as_ref().map(|h| h.meta.at);
        for m in self.scan.iter().chain(self.balances.iter().map(|b| &b.meta)).chain(self.states.iter().map(|s| &s.meta)) {
            at = Some(at.map_or(m.at, |a| a.max(m.at)));
        }
        at
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let meta = |m: &Meta| Out::O(vec![("agreement", Out::S(m.agreement.clone())), ("at", Out::N(m.at)), ("block", Out::N(m.block))]);
        let opt = |s: &Option<String>| s.clone().map_or(Out::Null, Out::S);
        let mut m: Vec<(&'static str, Out)> = vec![
            ("cases", Out::A(self.cases.iter().map(|c| Out::O(vec![
                ("block", Out::N(c.block)),
                ("buyer", Out::S(c.buyer.clone())),
                ("caseId", Out::S(c.case_id.clone())),
                ("instance", Out::S(c.instance.clone())),
                ("seller", Out::S(c.seller.clone())),
                ("tx", Out::S(c.tx.clone())),
            ])).collect())),
            ("chainId", Out::N(self.chain_id)),
            ("endpoints", Out::S(self.endpoints.clone())),
            ("foreign", Out::N(self.foreign)),
            ("key", Out::S(self.key.clone())),
            ("logic", Out::S(self.logic.clone())),
            ("states", Out::A(self.states.iter().map(|s| Out::O(vec![
                ("code", Out::B(s.code)),
                ("columns", Out::A(s.columns.iter().map(|c| Out::S(c.clone())).collect())),
                ("instance", Out::S(s.instance.clone())),
                ("meta", meta(&s.meta)),
            ])).collect())),
        ];
        m.push(("cursor", self.cursor.map_or(Out::Null, Out::N)));
        m.push(("scanned", Out::A(self.scanned.iter().map(|k| Out::S(k.clone())).collect())));
        m.push(("head", self.head.as_ref().map_or(Out::Null, |h| Out::O(vec![("chainTime", Out::N(h.chain_time)), ("meta", meta(&h.meta)), ("readWall", Out::N(h.read_wall))]))));
        m.push(("scan", self.scan.as_ref().map_or(Out::Null, meta)));
        m.push((
            "balances",
            self.balances.as_ref().map_or(Out::Null, |b| {
                Out::O(vec![
                    ("coins", Out::A(b.coins.iter().map(|c| Out::O(vec![("have", opt(&c.have)), ("token", Out::S(c.token.clone()))])).collect())),
                    ("holder", Out::S(b.holder.clone())),
                    ("meta", meta(&b.meta)),
                    ("native", opt(&b.native)),
                ])
            }),
        ));
        m.push(("kept", self.kept.to_out()));
        m.push(("tokens", Out::A(self.tokens.iter().map(|(a, w)| Out::A(vec![Out::S(a.clone()), Out::S(w.clone())])).collect())));
        eravon_store::doc::seal(eravon_store::seam::V_READING, m)
    }

    pub fn of_bytes(b: &[u8]) -> Option<Book> {
        let v = eravon_store::doc::open(eravon_store::seam::V_READING, b).ok()?;
        let s = |v: &In, k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let n = |v: &In, k: &str| match v.get(k) {
            Some(In::Int(i)) if *i >= 0 => Some(*i as u64),
            _ => None,
        };
        let meta = |v: Option<&In>| -> Option<Meta> {
            let v = v?;
            Some(Meta { block: n(v, "block")?, at: n(v, "at")?, agreement: s(v, "agreement")? })
        };
        let arr = |v: &In, k: &str| match v.get(k) {
            Some(In::Arr(a)) => a.clone(),
            _ => Vec::new(),
        };
        let mut book = Book {
            key: s(&v, "key")?,
            chain_id: n(&v, "chainId")?,
            logic: s(&v, "logic")?,
            endpoints: s(&v, "endpoints")?,
            cursor: n(&v, "cursor"),
            foreign: n(&v, "foreign")?,
            scan: meta(v.get("scan")),
            ..Book::default()
        };
        if let Some(h) = v.get("head").filter(|h| !matches!(h, In::Null)) {
            if let Some(read_wall) = n(h, "readWall") {
                book.head = Some(Head { chain_time: n(h, "chainTime")?, meta: meta(h.get("meta"))?, read_wall });
            }
        }
        for k in arr(&v, "scanned") {
            book.scanned.push(k.as_str()?.to_ascii_lowercase());
        }
        for c in arr(&v, "cases") {
            book.cases.push(Case { case_id: s(&c, "caseId")?, instance: s(&c, "instance")?, seller: s(&c, "seller")?, buyer: s(&c, "buyer")?, block: n(&c, "block")?, tx: s(&c, "tx")? });
        }
        for st in arr(&v, "states") {
            let columns = arr(&st, "columns").iter().map(|x| x.as_str().map(str::to_string)).collect::<Option<Vec<String>>>()?;
            let code = matches!(st.get("code"), Some(In::Bool(true)));
            book.states.push(State { instance: s(&st, "instance")?, code, columns, meta: meta(st.get("meta"))? });
        }
        if let Some(b) = v.get("balances").filter(|b| !matches!(b, In::Null)) {
            let coins = arr(b, "coins").iter().map(|c| Some(Coin { token: s(c, "token")?, have: s(c, "have") })).collect::<Option<Vec<Coin>>>()?;
            book.balances = Some(Balances { holder: s(b, "holder")?, native: s(b, "native"), coins, meta: meta(b.get("meta"))? });
        }
        if let Some(k) = v.get("kept") {
            book.kept = std::sync::Arc::new(eravon_chain::recall::Store::of_in(k)?);
        }
        for t in arr(&v, "tokens") {
            let p = t.as_arr()?;
            book.tokens.push((p.first()?.as_str()?.to_string(), p.get(1)?.as_str()?.to_string()));
        }
        Some(book)
    }

    pub fn rows(&self, now: u64) -> Out {
        let meta = |m: &Meta| {
            Out::O(vec![
                ("agreement", Out::S(m.agreement.clone())),
                ("at", Out::N(m.at)),
                ("block", Out::N(m.block)),
                ("stale", Out::B(stale(m.at, now))),
            ])
        };
        let mut m = vec![
            ("cases", Out::N(self.cases.len() as u64)),
            ("chainId", Out::N(self.chain_id)),
            ("key", Out::S(self.key.clone())),
            ("logic", Out::S(self.logic.clone())),
            ("states", Out::A(self.states.iter().map(|s| Out::O(vec![("code", Out::B(s.code)), ("instance", Out::S(s.instance.clone())), ("meta", meta(&s.meta)), ("phase", Out::S(s.columns.get(eravon_standard::chain::status_col::PHASE).cloned().unwrap_or_default()))])).collect())),
        ];
        m.push(("cursor", self.cursor.map_or(Out::Null, Out::N)));
        m.push(("head", self.head.as_ref().map_or(Out::Null, |h| Out::O(vec![("chainTime", Out::N(h.chain_time)), ("meta", meta(&h.meta))]))));
        m.push(("scan", self.scan.as_ref().map_or(Out::Null, meta)));
        m.push((
            "balances",
            self.balances.as_ref().map_or(Out::Null, |b| {
                Out::O(vec![
                    ("coins", Out::A(b.coins.iter().map(|c| Out::O(vec![("have", c.have.clone().map_or(Out::Null, Out::S)), ("token", Out::S(c.token.clone()))])).collect())),
                    ("holder", Out::S(b.holder.clone())),
                    ("meta", meta(&b.meta)),
                    ("native", b.native.clone().map_or(Out::Null, Out::S)),
                ])
            }),
        ));
        m.push(("readAt", self.read_at().map_or(Out::Null, Out::N)));
        Out::O(m)
    }
}

pub fn stale(at: u64, now: u64) -> bool {
    now.saturating_sub(at) > eravon_literals::constants::READ_STALE_SECS
}


pub fn load(data: &str, key: &str) -> Option<Book> {
    if key.is_empty() {
        return None;
    }
    eravon_vault::reading::get(data, key).and_then(|b| Book::of_bytes(&b)).filter(|b| b.key.eq_ignore_ascii_case(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Book {
        let m = |b, at| Meta { block: b, at, agreement: "agreed".into() };
        Book {
            key: "0x00000000000000000000000000000000000000a1".into(),
            chain_id: 31337,
            logic: "0x00000000000000000000000000000000000000b2".into(),
            endpoints: "http://127.0.0.1:1\nhttp://127.0.0.1:2".into(),
            head: Some(Head { chain_time: 1_780_000_000, meta: m(9, 100), read_wall: 1_791_500_000 }),
            cursor: Some(9),
            scanned: vec!["0x00000000000000000000000000000000000000a1".into()],
            cases: vec![Case { case_id: "0x01".into(), instance: "0x00000000000000000000000000000000000000c3".into(), seller: "0xs".into(), buyer: "0xb".into(), block: 4, tx: "0xt".into() }],
            scan: Some(m(9, 100)),
            foreign: 1,
            states: vec![State { instance: "0x00000000000000000000000000000000000000c3".into(), code: true, columns: vec!["2".into(), "1".into()], meta: m(9, 100) }],
            balances: Some(Balances { holder: "0xa1".into(), native: Some("5".into()), coins: vec![Coin { token: "0xt1".into(), have: None }, Coin { token: "0xt2".into(), have: Some("7".into()) }], meta: m(9, 100) }),
            tokens: vec![("0xt1".into(), "这一地址上没有码".into())],
            kept: std::sync::Arc::new({
                let mut k = eravon_chain::recall::Store::fresh(31337);
                k.set_head(9, 1_780_000_000, eravon_chain::read::Agreement::Agreed, 2);
                k
            }),
        }
    }

    #[test]
    fn a_book_goes_to_bytes_and_back_unchanged() {
        let b = sample();
        assert_eq!(Book::of_bytes(&b.to_bytes()), Some(b));
        let empty = Book::fresh("0xA1", 1, "0xB2");
        assert_eq!(Book::of_bytes(&empty.to_bytes()), Some(empty));
        assert_eq!(Book::of_bytes(b"{}"), None);
    }

    #[test]
    fn the_cursor_holds_only_for_the_keys_its_scan_covered() {
        let b = sample();
        let me = "0x00000000000000000000000000000000000000A1".to_string();
        let company = "0x00000000000000000000000000000000000000d4".to_string();
        assert!(b.covers(std::slice::from_ref(&me)), "扫过的那一把钥:游标成立");
        assert!(!b.covers(&[me.clone(), company]), "后添的公司地址没扫过:游标对它不成立");
        assert!(!Book { cursor: None, ..sample() }.covers(&[me.clone()]), "没扫完一趟:没有游标");
        assert!(!Book { scanned: Vec::new(), ..sample() }.covers(&[me]), "旧册不记扫过哪几把钥:对哪一把都不成立");
    }

    #[test]
    fn another_endpoint_table_voids_provenance_and_keeps_the_values() {
        let mut b = sample();
        b.void_provenance("http://127.0.0.1:3");
        assert!(b.head.as_ref().unwrap().meta.agreement.is_empty() && b.states[0].meta.agreement.is_empty());
        assert_eq!(b.balances.as_ref().unwrap().native.as_deref(), Some("5"));
        assert_eq!(b.cursor, Some(9));
    }

    #[test]
    fn a_reading_older_than_the_stale_line_is_stale() {
        let line = eravon_literals::constants::READ_STALE_SECS;
        assert!(!stale(100, 100 + line));
        assert!(stale(100, 101 + line));
    }

    #[test]
    fn the_chain_now_counts_elapsed_seconds_on_one_clock() {
        let mut b = sample();
        b.head = Some(Head { chain_time: 1_780_000_000, meta: Meta { block: 9, at: 1_780_000_000, agreement: "single".into() }, read_wall: 1_791_500_000 });
        assert_eq!(b.chain_now(1_791_500_030), Some(1_780_000_030));
        let back = Book::of_bytes(&b.to_bytes()).expect("back");
        assert_eq!(back.chain_now(1_791_500_030), Some(1_780_000_030));
        let old = String::from_utf8(b.to_bytes()).expect("text").replace(",\"readWall\":1791500000", "");
        assert!(!old.contains("readWall"));
        assert_eq!(Book::of_bytes(old.as_bytes()).map(|x| x.head), Some(None), "a head without the machine's clock is read again");
    }
}
