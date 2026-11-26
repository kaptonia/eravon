use eravon_home::notes::{self, Goods, Note};
use eravon_literals::json::Out;
use eravon_orders::table::{self, Row};

use crate::house::seam;
use crate::house::seller::Said;

fn said((n, d): eravon_home::Why) -> Said {
    Said::bad(&n, d)
}

#[derive(Clone, Debug)]
pub struct Home {
    pub me: String,
    pub now: u64,
    pub cards: eravon_home::cards::Cards,
    pub goods: Result<u64, Said>,
    pub cands: Result<u64, Said>,
    pub orders: Vec<Row>,
    pub cand_rows: Vec<(String, String, String)>,
    pub activity: Result<Vec<eravon_home::activity::Act>, Said>,
    pub notes: Result<Vec<Note>, Said>,
    pub marks: Vec<(String, bool, bool)>,
    pub badge: u64,
    pub system: Vec<Note>,
    pub toast: Option<eravon_home::toast::Said>,
    pub copied: Option<String>,
    pub notify_failed: Vec<String>,
}

pub const ORDER_ROWS: usize = 6;
pub const ACTIVITY_ROWS: usize = 10;

impl Home {
    pub fn open(data: &str, addr: &str) -> Home {
        let parts = eravon_watch::wire_parts(data, addr);
        let (chain_id, logic) = eravon_watch::standard();
        let (goods, goods_notes) = goods_of(data, addr);
        let (cands, cand_rows) = cands_of(data, addr);
        let (t, table) = match parts {
            Ok(x) => x,
            Err(e) => {
                let err = e.clone();
                let mut h = Home::empty(addr, goods, cands, cand_rows, &err);
                h.toast = (err.0 != eravon_watch::E_NO_ENDPOINT).then(|| eravon_home::toast::of(&err, &[]));
                return h;
            }
        };
        let regs = crate::house::orders::injected();
        let w = eravon_watch::Wire { t: &t, table: &table, chain_id, logic, registries: &regs };
        let now = eravon_home::chain_now(&w);
        let tbl = match &now {
            Ok(_) => eravon_orders::table::table(data, addr, eravon_scan::scan::Side::Seller, &w).and_then(eravon_orders::table::Table::complete),
            Err(e) => Err(e.clone()),
        };
        let chain_t = now.as_ref().map(|x| x.0).unwrap_or(0);
        let goods_notes = goods_notes(chain_t);
        let cards = eravon_home::cards::cards(&w, data, addr, &tbl, chain_t);
        let activity = eravon_home::activity::recent(&w, data, addr, &tbl, ACTIVITY_ROWS).map_err(said);
        let ns = notes::notes(&w, data, addr, &tbl, &goods_notes).map_err(said);
        let mut local_err = None;
        let marks = notes::marks(data, addr).map_err(|e| local_err = Some(e)).ok();
        let (badge, system) = match (&ns, &marks) {
            (Ok(v), Some(m)) => (notes::badge(v, m), notes::due(v, chain_t, m).into_iter().cloned().collect::<Vec<Note>>()),
            (Ok(v), None) => (v.len() as u64, Vec::new()),
            (Err(_), _) => (0, Vec::new()),
        };
        let marks = match notes::marks(data, addr) {
            Ok(m) => m,
            Err(e) => {
                local_err = Some(e);
                marks.unwrap_or_default()
            }
        };
        let toast = match tbl.as_ref().err() {
            Some(e) => Some(read_failed(e, &table)),
            None => local_err.as_ref().map(eravon_home::toast::local),
        };
        let orders = tbl.as_ref().map(|t| t.rows.iter().filter(|r| !table::done(r)).take(ORDER_ROWS).cloned().collect()).unwrap_or_default();
        Home { me: addr.to_string(), now: chain_t, cards, goods, cands, orders, cand_rows, activity, notes: ns, marks, badge, system, toast, copied: None, notify_failed: Vec::new() }
    }

    fn empty(addr: &str, goods: Result<u64, Said>, cands: Result<u64, Said>, cand_rows: Vec<(String, String, String)>, e: &eravon_home::Why) -> Home {
        Home {
            me: addr.to_string(),
            now: 0,
            cards: eravon_home::cards::Cards { active: Err(e.clone()), income: Err(e.clone()), month: String::new() },
            goods,
            cands,
            orders: Vec::new(),
            cand_rows,
            activity: Err(said(e.clone())),
            notes: Err(said(e.clone())),
            marks: Vec::new(),
            badge: 0,
            system: Vec::new(),
            toast: None,
            copied: None,
            notify_failed: Vec::new(),
        }
    }

    pub fn fresh_for(&self, addr: &str) -> bool {
        self.me.eq_ignore_ascii_case(addr)
    }

    pub fn handle(&mut self, data: &str, addr: &str, now: u64, key: &str) -> Result<(), Said> {
        notes::handle(data, seam::WRITER, now, addr, key).map_err(said)?;
        self.marks = notes::marks(data, addr).map_err(said)?;
        if let Ok(v) = &self.notes {
            self.badge = notes::badge(v, &self.marks);
        }
        Ok(())
    }

    pub fn item(&self, key: &str, item: &str) -> Option<String> {
        let v = self.notes.as_ref().ok()?;
        v.iter().find(|n| n.key == key)?.items.iter().find(|(k, _)| k == item).map(|(_, x)| x.clone())
    }

    pub fn rows(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        let mut m: Vec<(&'static str, Out)> = vec![("me", s(&self.me)), ("month", s(&self.cards.month)), ("now", Out::N(self.now))];
        match &self.cards.active {
            Ok(n) => m.push(("active", Out::N(*n))),
            Err(e) => m.push(("active", said(e.clone()).out())),
        }
        match &self.cards.income {
            Ok(v) => m.push((
                "income",
                Out::A(
                    v.iter()
                        .map(|x| {
                            Out::O(vec![
                                ("count", Out::N(x.count)),
                                ("token", s(&x.token)),
                                ("total", s(&x.total)),
                                ("word", s(&crate::house::coin::word(&x.total, x.decimals, &x.token, crate::house::coin::Form::Short))),
                            ])
                        })
                        .collect(),
                ),
            )),
            Err(e) => m.push(("income", said(e.clone()).out())),
        }
        for (k, v) in [("goods", &self.goods), ("cands", &self.cands)] {
            match v {
                Ok(n) => m.push((k, Out::N(*n))),
                Err(e) => m.push((k, e.out())),
            }
        }
        m.push(("orders", Out::A(self.orders.iter().map(|r| Out::O(vec![("instance", s(&r.instance)), ("number", s(&r.number))])).collect())));
        m.push(("candRows", Out::N(self.cand_rows.len() as u64)));
        match &self.activity {
            Ok(v) => m.push((
                "activity",
                Out::A(v.iter().map(act_out).collect()),
            )),
            Err(e) => m.push(("activity", e.out())),
        }
        match &self.notes {
            Ok(v) => m.push((
                "notes",
                Out::A(
                    v.iter()
                        .map(|n| {
                            let handled = self.marks.iter().any(|(k, h, _)| *k == n.key && *h);
                            Out::O(vec![
                                ("deadline", Out::N(n.deadline)),
                                ("handled", Out::B(handled)),
                                ("items", Out::O(n.items.iter().map(|(k, v)| (item_key(k), s(v))).collect())),
                                ("key", s(&n.key)),
                                ("kind", s(n.kind.word())),
                                ("say", s(&n.say)),
                                ("subject", s(&n.subject)),
                            ])
                        })
                        .collect(),
                ),
            )),
            Err(e) => m.push(("notes", e.out())),
        }
        m.push(("badge", Out::N(self.badge)));
        m.push(("system", Out::A(self.system.iter().map(|n| Out::O(vec![("key", s(&n.key)), ("say", s(&n.say))])).collect())));
        if let Some(t) = &self.toast {
            m.push(("toast", Out::O(vec![("detail", s(t.detail.as_deref().unwrap_or(""))), ("head", s(&t.head))])));
        }
        if let Some(c) = &self.copied {
            m.push(("copied", s(c)));
        }
        Out::O(m)
    }
}

fn item_key(k: &str) -> &'static str {
    match k {
        "grantee" => "grantee",
        "work" => "work",
        "terms" => "terms",
        _ => "other",
    }
}

#[allow(clippy::type_complexity)]
fn goods_of(data: &str, addr: &str) -> (Result<u64, Said>, Box<dyn Fn(u64) -> Vec<Goods>>) {
    match eravon_goods::goods::list(data, addr, "") {
        Ok(rows) => {
            let n = rows.iter().filter(|r| !r.draft && r.listed).count() as u64;
            let (d, a) = (data.to_string(), addr.to_string());
            let make = move |now: u64| -> Vec<Goods> {
                let lead = u64::from(eravon_literals::constants::RESIGN_LEAD_DAYS) * 86_400;
                let mut out = Vec::new();
                for r in rows.iter().filter(|r| !r.draft && r.listed) {
                    let expiry = eravon_goods::goods::view(&d, &a, &r.id, now).ok().and_then(|v| v.links.first().map(|l| l.expiry)).unwrap_or(0);
                    for _why in &r.pending {
                        out.push(Goods { id: r.id.clone(), name: r.name.clone(), why: "resign".into(), expiry });
                    }
                    if now != 0 && expiry != 0 && expiry >= now && expiry - now <= lead {
                        out.push(Goods { id: r.id.clone(), name: r.name.clone(), why: "expiring".into(), expiry });
                    }
                }
                out
            };
            (Ok(n), Box::new(make))
        }
        Err(e) => (Err(Said::bad(e.name(), e.detail())), Box::new(|_| Vec::new())),
    }
}

fn cands_of(data: &str, addr: &str) -> (Result<u64, Said>, Vec<(String, String, String)>) {
    match eravon_cands::table::read(data, addr) {
        Ok(rows) => {
            let ok: Vec<(String, String, String)> = rows.iter().filter(|r| r.flaw.is_none()).map(|r| (r.row.label.clone(), r.row.kind.clone(), r.row.address.clone())).collect();
            (Ok(ok.len() as u64), ok)
        }
        Err(e) => (Err(Said::bad(e.name(), e.detail())), Vec::new()),
    }
}

pub fn delivered(data: &str, owner: &str, now: u64, keys: &[String]) -> Result<(), crate::house::seller::Said> {
    eravon_home::notes::sent(data, crate::house::seam::WRITER, now, owner, keys).map_err(said)
}

pub(crate) fn act_out(a: &eravon_home::activity::Act) -> Out {
    let s = |x: &str| Out::S(x.to_string());
    match &a.unread {
        None => Out::O(vec![("at", s(&eravon_orders::day::at(a.at))), ("order", s(&a.order)), ("say", s(&a.say)), ("tx", s(&a.tx))]),
        Some((n, d)) => Out::O(vec![("order", s(&a.order)), ("unread", Out::O(vec![("detail", s(d)), ("name", s(n))]))]),
    }
}

pub fn read_failed(e: &eravon_home::Why, table: &eravon_chain::endpoints::Table) -> eravon_home::toast::Said {
    let said: Vec<String> = table.nodes().iter().map(|n| n.said()).collect();
    eravon_home::toast::of(e, &said)
}

crate::house::merge::lands!(Home {
    whole: [me],
    data: [now, cards, goods, cands, orders, cand_rows, activity, notes, marks, badge, system, toast, copied, notify_failed],
    page: [],
    keyed(|_: &Home| ()): [],
    nested: [],
    said: [],
});
