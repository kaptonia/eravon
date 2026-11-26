use super::*;

#[derive(Clone, Debug)]
pub struct Cands {
    pub tab: Side,
    pub rows: Result<Vec<Brief>, Said>,
    pub detail: Option<(Candidate, Result<Detail, Absent>, Option<String>)>,
    pub dtab: usize,
    pub said: Option<Said>,
    pub me: String,
    pub codes: Vec<(String, Result<bool, Said>)>,
    pub lapse: Option<String>,
    pub unread: bool,
}

fn codes_of(data: &str, addr: &str, d: &Result<Detail, Absent>) -> Vec<(String, Result<bool, Said>)> {
    let seen = eravon_cli::deskops::candsview::shown(d);
    match crate::house::orders::wire(data, addr) {
        Ok(w) => eravon_cli::deskops::candsview::codes(&w.t, &w.table, &seen).into_iter().map(|(a, r)| (a, r.map_err(|(n, d)| Said::bad(&n, d)))).collect(),
        Err(e) => seen.into_iter().map(|a| (a, Err(e.clone()))).collect(),
    }
}

fn signed_of(data: &str, addr: &str) -> Result<Vec<eravon_cands::view::Signed>, Said> {
    eravon_goods::goods::signed_words(data, addr).map_err(|e| Said::bad(e.name(), e.detail()))
}

fn list_of(data: &str, addr: &str, f: &Field, now: u64) -> Result<Vec<Brief>, Said> {
    let signed = signed_of(data, addr)?;
    eravon_cands::view::list(data, addr, &f.registries, now, seam::WRITER, &signed).map_err(|e| Said::bad(e.name(), e.detail()))
}

#[derive(Clone, Debug, Default)]
pub struct Field {
    pub registries: Vec<String>,
    pub home: String,
}

impl Cands {
    pub fn entered(slot: &mut Option<Cands>, addr: &str) {
        match slot.as_mut() {
            Some(c) if c.me.eq_ignore_ascii_case(addr) => {
                c.detail = None;
                c.codes = Vec::new();
                c.lapse = None;
            }
            _ => *slot = None,
        }
    }

    pub fn lapse_of(&self, token: &str) -> Result<(eravon_cands::lapse::Clocks, bool), String> {
        let Some((_, Ok(d), _)) = &self.detail else { return Err("先打开一位判官或一家行会".into()) };
        let (quotes, guild) = match d {
            Detail::Judge(j) => (&j.quotes, false),
            Detail::Guild(g) => (&g.quotes, true),
        };
        let row = quotes.iter().find(|q| token.is_empty() || q.token.eq_ignore_ascii_case(token)).ok_or_else(|| format!("没有 {token} 的报价"))?;
        let q = row.quote.as_ref().map_err(Clone::clone)?;
        eravon_cands::lapse::clocks(q).map(|c| (c, guild))
    }

    pub fn lapse_open(&mut self, token: &str) -> Result<(), String> {
        self.lapse_of(token)?;
        self.lapse = Some(token.to_string());
        Ok(())
    }

    pub fn open(data: &str, addr: &str, f: &Field, now: u64, tab: Side) -> Cands {
        Cands { tab, rows: list_of(data, addr, f, now), detail: None, dtab: 0, said: None, me: addr.to_string(), codes: Vec::new(), lapse: None, unread: false }
    }

    pub fn behind(live: Option<&Cands>, from: Option<&Cands>, got: &Cands) -> bool {
        from.is_none() && live.is_some_and(|l| !l.unread && l.me.eq_ignore_ascii_case(&got.me))
    }

    pub fn local(data: &str, addr: &str, tab: Side) -> Cands {
        let rows = eravon_cands::table::read(data, addr)
            .map(|rows| rows.into_iter().map(|r| Brief { row: r.row, state: Err(eravon_cands::fault::Absent::Unreachable(String::new())), warn: None, flaw: r.flaw }).collect())
            .map_err(|e| Said::bad(e.name(), e.detail()));
        Cands { tab, rows, detail: None, dtab: 0, said: None, me: addr.to_string(), codes: Vec::new(), lapse: None, unread: true }
    }

    fn reload(&mut self, data: &str, addr: &str, f: &Field, now: u64) {
        self.rows = list_of(data, addr, f, now);
        self.unread = false;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn plan(data: &str, addr: &str, what: &CandsPlan) -> Result<(), Said> {
        let s = |e: eravon_cands::fault::Fault| Said::bad(e.name(), e.detail());
        match what {
            CandsPlan::Add(side, input, label) => eravon_cli::deskops::cands::plan(data, addr, *side, input, label).map(|_| ()).map_err(s),
            CandsPlan::Label(v) => eravon_cands::table::need_label(v).map(|_| ()).map_err(s),
            CandsPlan::Remove => Ok(()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add(&mut self, data: &str, addr: &str, f: &Field, now: u64, side: Side, input: &str, label: &str, ok_word: &str, still: &dyn Fn() -> bool) {
        if let Err(e) = crate::house::asked(Cands::plan(data, addr, &CandsPlan::Add(side, input.to_string(), label.to_string()))) {
            self.said = Some(e);
            return;
        }
        match eravon_cli::deskops::cands::add(data, seam::WRITER, now, addr, side, input, label, &eravon_watch::wire_parts(data, addr), &f.registries, still) {
            Ok(_) => self.said = Some(Said::ok(ok_word)),
            Err(eravon_cli::deskops::cands::No::Stale) => self.said = Some(crate::house::stale()),
            Err(eravon_cli::deskops::cands::No::Cands(e)) => self.said = Some(Said::bad(e.name(), e.detail())),
        }
        self.tab = side;
        self.reload(data, addr, f, now);
    }

    pub fn open_one(&mut self, data: &str, addr: &str, f: &Field, now: u64, who: &str) {
        let signed = match signed_of(data, addr) {
            Ok(s) => s,
            Err(e) => {
                self.said = Some(e);
                return;
            }
        };
        match eravon_cands::view::detail(data, addr, who, &f.registries, now, seam::WRITER, &signed) {
            Ok(d) => {
                self.codes = codes_of(data, addr, &d.1);
                if let Ok(rows) = self.rows.as_mut() {
                    if let Some(b) = rows.iter_mut().find(|b| b.row.address == d.0.address) {
                        b.state = d.1.as_ref().map(eravon_cands::view::summary_of).map_err(Clone::clone);
                    }
                }
                self.detail = Some(d);
                self.dtab = 0;
                self.lapse = None;
            }
            Err(e) => self.said = Some(Said::bad(e.name(), e.detail())),
        }
    }

    pub fn peek(&mut self, data: &str, addr: &str, f: &Field, now: u64, who: &str) {
        let (row, got) = eravon_cands::view::peek(data, addr, who, &f.registries, now);
        self.codes = codes_of(data, addr, &got);
        self.detail = Some((row, got, None));
        self.dtab = 0;
        self.lapse = None;
    }

    pub fn rename(&mut self, data: &str, addr: &str, f: &Field, now: u64, who: &str, label: &str, ok_word: &str) {
        match eravon_cands::table::rename(data, seam::WRITER, now, addr, who, label) {
            Ok(()) => {
                self.said = Some(Said::ok(ok_word));
                if self.detail.as_ref().is_some_and(|(r, _, _)| r.address == who) {
                    self.open_one(data, addr, f, now, who);
                }
            }
            Err(e) => self.said = Some(Said::bad(e.name(), e.detail())),
        }
        self.reload(data, addr, f, now);
    }

    pub fn remove(&mut self, data: &str, addr: &str, f: &Field, now: u64, who: &str, ok_word: &str) {
        match eravon_cands::table::remove(data, seam::WRITER, now, addr, who) {
            Ok(()) => {
                self.said = Some(Said::ok(ok_word));
                self.detail = None;
                self.lapse = None;
            }
            Err(e) => self.said = Some(Said::bad(e.name(), e.detail())),
        }
        self.reload(data, addr, f, now);
    }

    pub fn rows(&self) -> Out {
        let mut m = vec![("tab", Out::S(self.tab.word().to_string()))];
        match &self.rows {
            Ok(list) if self.unread => {
                m.push(("unread", Out::B(true)));
                m.push(("rows", Out::A(list.iter().map(listed_out).collect())));
            }
            Ok(list) => m.push(("rows", Out::A(list.iter().map(brief_out).collect()))),
            Err(s) => m.push(("rows", s.out())),
        }
        if let Some((row, got, warn)) = &self.detail {
            let codes: Vec<(String, Result<bool, (String, String)>)> = self.codes.iter().map(|(a, r)| (a.clone(), r.clone().map_err(|s| (s.name, s.detail)))).collect();
            let mut d = eravon_cli::deskops::candsview::detail_out(row, got, warn.as_ref(), &codes);
            d.insert(2, ("tab", Out::N(self.dtab as u64)));
            if let Some(t) = &self.lapse {
                d.push(("lapseOf", Out::S(t.clone())));
                if let Ok((c, guild)) = self.lapse_of(t) {
                    d.push(("lapse", crate::house::case::lapse_out(&c, guild)));
                }
            }
            m.push(("detail", Out::O(d)));
        }
        if let Some(s) = &self.said {
            m.push(("said", s.out()));
        }
        Out::O(m)
    }
}

pub use eravon_cli::deskops::candsview::{brief_out, pill_of, Pill};

fn listed_out(b: &Brief) -> Out {
    let mut m = vec![
        ("addedAt", Out::N(b.row.added_at)),
        ("address", Out::S(b.row.address.clone())),
        ("kind", Out::S(b.row.kind.clone())),
        ("label", Out::S(b.row.label.clone())),
        ("seen", Out::S(b.row.seen.clone())),
    ];
    if let Some(f) = &b.flaw {
        m.push(("flaw", Out::S(f.clone())));
    }
    Out::O(m)
}

crate::house::merge::lands!(Cands {
    whole: [me],
    data: [rows, unread],
    page: [tab, dtab, lapse],
    keyed(|m: &Cands| m.detail.as_ref().map(|d| format!("{:?}", d.0))): [detail, codes],
    nested: [],
    said: [said],
});

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "48261537";
    const PHRASE: &str = "test test test test test test test test test test test junk";
    const NOW: u64 = 1_790_000_000;
    const WRITER: &str = "cands-test";

    #[test]
    fn the_list_off_the_disk_lands_before_the_chain_read_and_the_chain_read_takes_its_states() {
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create(PIN, eravon_store::vault::Seat::Seller, "主钥", Some(PHRASE), NOW).expect("a vault");
        v.write(&data).expect("written");
        let mut s = eravon_vault::session::Session::locked(&data);
        s.unlock(PIN).expect("opens");
        let me = eravon_vault::accounts::who(&data, eravon_store::vault::Seat::Seller).expect("a key").to_ascii_lowercase();
        let st = eravon_vault::store::open_write(&data, WRITER, NOW).expect("writable");
        st.init(&eravon_store::settings::Global::default()).expect("laid out");
        st.init_identity(&me, NOW).expect("registered");
        let judge = "0x00000000000000000000000000000000000000b1";
        eravon_cands::table::add(&data, WRITER, NOW, &me, Side::Judge, judge, "甲", "").expect("a row");
        let listed = Cands::local(&data, &me, Side::Judge);
        assert!(listed.unread);
        let rows = listed.rows.as_ref().expect("the table reads off the disk");
        assert_eq!(rows.iter().map(|b| b.row.address.as_str()).collect::<Vec<_>>(), [judge]);
        let mut slot = None;
        crate::house::merge::land(&mut slot, crate::house::merge::Pass::new(None, listed.clone(), true));
        assert!(slot.as_ref().is_some_and(|c: &Cands| c.unread && c.rows.as_ref().is_ok_and(|r| r.len() == 1)), "the page has its rows before the chain is read");
        let mut read = listed.clone();
        read.unread = false;
        read.rows = Ok(vec![Brief { row: rows[0].row.clone(), state: Err(eravon_cands::fault::Absent::NoDocument), warn: None, flaw: None }]);
        assert!(!Cands::behind(slot.as_ref(), None, &read));
        crate::house::merge::land(&mut slot, crate::house::merge::Pass::new(None, read.clone(), true));
        let page = slot.as_ref().expect("kept");
        assert!(!page.unread);
        assert!(matches!(page.rows.as_ref().map(|r| &r[0].state), Ok(Err(eravon_cands::fault::Absent::NoDocument))), "the chain read takes the states");
        assert!(Cands::behind(slot.as_ref(), None, &listed), "a list off the disk landing after the chain read is not taken");
        assert!(Cands::behind(slot.as_ref(), None, &read), "nor a read started with nothing in hand, landing after a list with states");
        assert!(!Cands::behind(slot.as_ref(), Some(&listed), &listed), "a pass started from the page in hand is");
        assert!(!Cands::behind(slot.as_ref(), None, &Cands { me: "0x00000000000000000000000000000000000000cc".into(), ..listed }), "another key's list is");
    }
}
