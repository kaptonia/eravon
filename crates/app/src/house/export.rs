use eravon_export::docs::Side;
use eravon_export::pick::Pick;
use eravon_literals::json::Out;
use eravon_recheck::report::Item;
use eravon_statement::Statement;

use crate::house::seller::Said;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Dossier,
    Statement,
}

impl Tab {
    pub const fn name(self) -> &'static str {
        match self {
            Tab::Dossier => "dossier",
            Tab::Statement => "statement",
        }
    }
    pub fn of(s: &str) -> Option<Tab> {
        match s {
            "dossier" => Some(Tab::Dossier),
            "statement" => Some(Tab::Statement),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub tab: Tab,
    pub files: Vec<String>,
    pub report: Option<Result<Vec<Item>, Said>>,
    pub selfcheck: Option<String>,
    pub shape: Option<eravon_cli::deskops::export::Shape>,
    pub statement: Option<Statement>,
    pub compliance: Option<Result<String, Said>>,
}

#[derive(Clone, Debug, Default)]
pub struct Export {
    pub tab: Tab,
    pub me: String,
    pub buyer: bool,
    pub owners: Vec<String>,
    pub keys: Vec<String>,
    pub picks: Option<Result<Vec<Pick>, Said>>,
    pub picker: bool,
    pub q: String,
    pub from: String,
    pub to: String,
    pub shown: Option<Result<Vec<usize>, Said>>,
    pub sel: Option<String>,
    pub dir: String,
    pub said: Option<Said>,
    pub done: Option<Result<Outcome, Said>>,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub tab: Tab,
    pub data: String,
    pub me: String,
    pub buyer: bool,
    pub owners: Vec<String>,
    pub record: String,
    pub dir: String,
    pub registries: Vec<String>,
    pub now: u64,
}

fn said((n, d): eravon_export::Why) -> Said {
    Said::bad(&n, d)
}

impl Export {
    pub fn open(s: &eravon_vault::session::Session, data: &str, me: &str, buyer: bool) -> Export {
        crate::house::mark::mark(eravon_export::mark::FEATURE);
        let mut e = Export { me: me.to_ascii_lowercase(), buyer, ..Export::default() };
        e.reload(s, data, me);
        e
    }

    pub fn reload(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str) {
        self.me = me.to_ascii_lowercase();
        self.dir = last_dir(data);
        self.picks = Some((|| {
            eravon_vault::session::need_open(s).map_err(|e| crate::house::said_of(&e))?;
            let w = crate::house::orders::wire(data, me)?;
            let (owners, keys) = if self.buyer {
                let (o, k) = crate::house::buyer_orders::key_set(s, me)?;
                (o, k)
            } else {
                let k = eravon_orders::table::keys_of(data, me, eravon_scan::scan::Side::Seller).map_err(said)?;
                (eravon_vault::store::archive_of(data, me).map_err(|e| said((e.name().to_string(), e.detail())))?, k)
            };
            self.owners = owners.clone();
            self.keys = keys.clone();
            let mut picks = eravon_export::pick::picks(data, &owners, &keys, self.side(), &crate::house::orders::sw(&w)).map_err(said)?;
            if self.buyer {
                for p in picks.iter_mut() {
                    let (product, seller) = crate::house::buyer_orders::product_and_seller(data, &owners, &p.row.instance);
                    if p.row.product.is_empty() {
                        p.row.product = product;
                    }
                    p.row.label = seller;
                }
            }
            Ok(picks)
        })());
        self.search();
        if let Some(sel) = self.sel.clone() {
            if self.pick(&sel).is_none() {
                self.sel = None;
            }
        }
    }

    fn side(&self) -> Side {
        if self.buyer { Side::Buyer } else { Side::Seller }
    }

    pub fn pick(&self, instance: &str) -> Option<&Pick> {
        self.picks.as_ref()?.as_ref().ok()?.iter().find(|p| p.row.instance.eq_ignore_ascii_case(instance))
    }

    pub fn search(&mut self) {
        self.shown = match &self.picks {
            Some(Ok(p)) => Some(eravon_export::pick::search(p, &self.q, &self.from, &self.to).map_err(said)),
            _ => None,
        };
    }

    pub fn choose(&mut self, instance: &str) -> Result<(), Said> {
        let p = self.pick(instance).ok_or_else(|| Said::bad("E_S7_NONE", crate::fill!(crate::house::i18n::Key::ExNotInTable; instance.to_string())))?;
        if !p.ready() {
            return Err(Said::bad("E_S7_MISSING", crate::fill!(crate::house::i18n::Key::ExLacks; missing_words(&p.missing))));
        }
        self.sel = Some(p.row.instance.to_ascii_lowercase());
        self.picker = false;
        self.done = None;
        Ok(())
    }

    pub fn set_dir(&mut self, data: &str, now: u64, dir: &str) -> Result<(), Said> {
        crate::house::settings::write_global(data, now, |g| g.export_dir = dir.to_string())?;
        self.dir = dir.to_string();
        Ok(())
    }

    pub fn plan(&self, data: &str, registries: &[String], now: u64) -> Result<Job, Said> {
        let sel = self.sel.as_deref().ok_or_else(|| Said::bad("E_S7_NO_ORDER", crate::house::i18n::t(crate::house::i18n::Key::ExPickFirst)))?;
        let p = self.pick(sel).ok_or_else(|| Said::bad("E_S7_NONE", crate::fill!(crate::house::i18n::Key::ExNotInTable; sel.to_string())))?;
        if !p.ready() {
            return Err(Said::bad("E_S7_MISSING", crate::fill!(crate::house::i18n::Key::ExLacks; missing_words(&p.missing))));
        }
        if self.dir.is_empty() {
            return Err(Said::bad("E_S7_NO_DIR", crate::house::i18n::t(crate::house::i18n::Key::ExDirFirst)));
        }
        Ok(Job {
            tab: self.tab,
            data: data.to_string(),
            me: self.me.clone(),
            buyer: self.buyer,
            owners: self.owners.clone(),
            record: p.row.record.clone(),
            dir: self.dir.clone(),
            registries: registries.to_vec(),
            now,
        })
    }

    pub fn rows(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        let mut m: Vec<(&'static str, Out)> = vec![
            ("dir", s(&self.dir)),
            ("from", s(&self.from)),
            ("picker", Out::B(self.picker)),
            ("q", s(&self.q)),
            ("seat", s(if self.buyer { "buyer" } else { "seller" })),
            ("tab", s(self.tab.name())),
            ("to", s(&self.to)),
        ];
        if let Some(x) = &self.sel {
            m.push(("sel", s(x)));
        }
        if let Some(x) = &self.said {
            m.push(("said", x.out()));
        }
        match &self.picks {
            Some(Ok(ps)) => {
                let shown: Vec<usize> = match &self.shown {
                    Some(Ok(v)) => v.clone(),
                    _ => (0..ps.len()).collect(),
                };
                m.push((
                    "picks",
                    Out::A(
                        shown
                            .iter()
                            .filter_map(|i| ps.get(*i))
                            .map(|p| {
                                Out::O(vec![
                                    ("buyer", s(&p.row.buyer)),
                                    ("caseId", s(&p.row.case_id)),
                                    ("instance", s(&p.row.instance)),
                                    ("missing", Out::A(p.missing.iter().map(|x| s(x)).collect())),
                                    ("number", s(&p.row.number)),
                                    ("placed", Out::N(p.row.placed)),
                                    ("product", s(&p.row.product)),
                                    ("ready", Out::B(p.ready())),
                                    ("record", s(&p.row.record)),
                                ])
                            })
                            .collect(),
                    ),
                ));
            }
            Some(Err(e)) => m.push(("picks", e.out())),
            None => {}
        }
        if let Some(Err(e)) = &self.shown {
            m.push(("search", e.out()));
        }
        match &self.done {
            Some(Ok(o)) => m.push(("done", outcome_rows(o))),
            Some(Err(e)) => m.push(("done", e.out())),
            None => {}
        }
        Out::O(m)
    }
}

fn last_dir(data: &str) -> String {
    eravon_vault::store::read_global(data).map(|g| g.export_dir).unwrap_or_default()
}


pub fn run(j: &Job, still: &dyn Fn() -> bool) -> Result<Outcome, Said> {
    use eravon_cli::deskops::export as x;
    let w = crate::house::orders::wire(&j.data, &j.me)?;
    let kind = match j.tab {
        Tab::Dossier => x::Kind::Dossier,
        Tab::Statement => x::Kind::Statement,
    };
    let job = x::Job { kind, data: j.data.clone(), writer: crate::house::seam::WRITER.to_string(), me: j.me.clone(), buyer: j.buyer, owners: j.owners.clone(), record: j.record.clone(), dir: j.dir.clone(), registries: j.registries.clone(), now: j.now };
    let said_no = |n: eravon_cli::deskops::No| match n.name.as_str() {
        x::R_MISSING => Said::bad(x::R_MISSING, crate::fill!(crate::house::i18n::Key::ExLacks; missing_words(&n.facts))),
        x::R_STALE => crate::house::stale(),
        _ => Said::bad(&n.name, n.detail()),
    };
    let o = eravon_cli::deskops::export::run(&job, &w, still).map_err(said_no)?;
    Ok(Outcome {
        tab: j.tab,
        files: o.files,
        report: o.report.map(|r| r.map_err(|n| Said::bad(&n.name, n.detail()))),
        selfcheck: o.selfcheck,
        shape: o.shape,
        statement: o.statement,
        compliance: o.compliance.map(|r| r.map_err(|n| Said::bad(&n.name, n.detail()))),
    })
}

pub fn outcome_rows(o: &Outcome) -> Out {
    use eravon_cli::deskops::{export as x, No};
    let no = |s: &Said| No::new(&s.name, vec![s.detail.clone()]);
    let raw = x::Outcome {
        kind: match o.tab {
            Tab::Dossier => x::Kind::Dossier,
            Tab::Statement => x::Kind::Statement,
        },
        files: o.files.clone(),
        report: o.report.as_ref().map(|r| r.clone().map_err(|e| no(&e))),
        selfcheck: o.selfcheck.clone(),
        shape: o.shape.clone(),
        statement: o.statement.clone(),
        compliance: o.compliance.as_ref().map(|r| r.clone().map_err(|e| no(&e))),
    };
    raw.out_by(&|n: &No| Said::bad(&n.name, n.detail()).out())
}

pub const NEEDS: [&str; 2] = ["E_S7_NO_ORDER", "E_S7_NO_DIR"];

fn missing_words(m: &[impl AsRef<str>]) -> String {
    let now = crate::house::i18n::current();
    m.iter().map(|x| crate::house::i18n::library(now, x.as_ref())).collect::<Vec<_>>().join(&crate::house::i18n::t(crate::house::i18n::Key::ListJoin))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_round_trip() {
        for t in [Tab::Dossier, Tab::Statement] {
            assert_eq!(Tab::of(t.name()), Some(t));
        }
    }

    #[test]
    fn plan_needs_an_order_and_a_dir() {
        let e = Export::default();
        assert_eq!(e.plan("/x", &[], 1).err().map(|s| s.name), Some("E_S7_NO_ORDER".to_string()));
    }
}

crate::house::merge::lands!(Export {
    whole: [me, buyer],
    data: [owners, keys, picks, shown, done, dir],
    page: [tab, picker, q, from, to, sel],
    keyed(|_: &Export| ()): [],
    nested: [],
    said: [said],
});
