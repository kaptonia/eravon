use super::No;
use crate::shared::Wire;
use eravon_export::docs::{self, Got, Side};
use eravon_export::pick::Pick;
use eravon_recheck::report::Item;
use eravon_statement::Statement;

pub const R_MISSING: &str = "E_S7_MISSING";
pub const R_NONE: &str = "E_S7_NONE";
pub const R_STALE: &str = "stale";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Dossier,
    Statement,
}

impl Kind {
    pub const fn name(self) -> &'static str {
        match self {
            Kind::Dossier => "dossier",
            Kind::Statement => "statement",
        }
    }

    pub fn of(s: &str) -> Option<Kind> {
        match s {
            "dossier" => Some(Kind::Dossier),
            "statement" => Some(Kind::Statement),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Job {
    pub kind: Kind,
    pub data: String,
    pub writer: String,
    pub me: String,
    pub buyer: bool,
    pub owners: Vec<String>,
    pub record: String,
    pub dir: String,
    pub registries: Vec<String>,
    pub now: u64,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub kind: Kind,
    pub files: Vec<String>,
    pub report: Option<Result<Vec<Item>, No>>,
    pub selfcheck: Option<String>,
    pub shape: Option<Shape>,
    pub statement: Option<Statement>,
    pub compliance: Option<Result<String, No>>,
}

impl Outcome {
    pub fn out_by(&self, no: &dyn Fn(&No) -> eravon_literals::json::Out) -> eravon_literals::json::Out {
        use eravon_literals::json::Out;
        let s = |x: &str| Out::S(x.to_string());
        let mut m: Vec<(&'static str, Out)> = vec![("files", Out::A(self.files.iter().map(|f| s(f)).collect())), ("tab", s(self.kind.name()))];
        match &self.report {
            Some(Ok(items)) => {
                m.push(("report", Out::A(items.iter().map(Item::out).collect())));
                m.push(("counts", eravon_recheck::report::counts(items)));
            }
            Some(Err(e)) => m.push(("report", no(e))),
            None => {}
        }
        if let Some(x) = &self.selfcheck {
            m.push(("selfcheck", s(x)));
        }
        if let Some(sh) = &self.shape {
            m.push(("shape", sh.out()));
        }
        if let Some(st) = &self.statement {
            m.push(("statement", s(&String::from_utf8_lossy(&eravon_statement::statement_bytes(st)))));
        }
        match &self.compliance {
            Some(Ok(p)) => m.push(("compliance", s(p))),
            Some(Err(e)) => m.push(("compliance", no(e))),
            None => {}
        }
        Out::O(m)
    }

    pub fn out(&self) -> eravon_literals::json::Out {
        use eravon_literals::json::Out;
        self.out_by(&|n: &No| Out::O(vec![("detail", Out::S(n.detail())), ("name", Out::S(n.name.clone()))]))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape {
    pub rows: usize,
    pub kinds: Vec<(&'static str, usize)>,
    pub terminal: Option<String>,
    pub entry: bool,
    pub compliance: bool,
    pub posted: (String, String),
    pub attachments: usize,
}

impl Shape {
    fn of(a: &eravon_export::assemble::Assembled) -> Shape {
        Shape { rows: a.rows, kinds: a.kinds.clone(), terminal: a.terminal.clone(), entry: a.entry, compliance: a.compliance, posted: a.posted.clone(), attachments: a.attachments }
    }

    pub fn out(&self) -> eravon_literals::json::Out {
        use eravon_literals::json::Out;
        Out::O(vec![
            ("attachments", Out::N(self.attachments as u64)),
            ("compliance", Out::B(self.compliance)),
            ("entry", Out::B(self.entry)),
            ("kinds", Out::A(self.kinds.iter().map(|(k, n)| Out::O(vec![("count", Out::N(*n as u64)), ("kind", Out::S((*k).to_string()))])).collect())),
            ("posted", Out::O(vec![("hash", Out::S(self.posted.1.clone())), ("tx", Out::S(self.posted.0.clone()))])),
            ("rows", Out::N(self.rows as u64)),
            ("terminal", Out::S(self.terminal.clone().unwrap_or_default())),
        ])
    }
}

fn why((n, d): eravon_export::Why) -> No {
    No::new(&n, vec![d])
}

pub fn owners_keys(s: &eravon_vault::session::Session, data: &str, me: &str, buyer: bool) -> Result<(Vec<String>, Vec<String>), No> {
    if buyer {
        let b = eravon_vault::keyset::buyer(s, me).map_err(|e| No::new(e.name(), vec![e.detail()]))?;
        Ok((b.owners, b.keys))
    } else {
        let k = eravon_orders::table::keys_of(data, me, eravon_scan::scan::Side::Seller).map_err(why)?;
        let owners = eravon_vault::store::archive_of(data, me).map_err(|e| No::new(e.name(), vec![e.detail()]))?;
        Ok((owners, k))
    }
}

pub fn pick(data: &str, owners: &[String], keys: &[String], buyer: bool, w: &Wire, instance: &str) -> Result<String, No> {
    let side = if buyer { Side::Buyer } else { Side::Seller };
    let a3side = if buyer { eravon_scan::scan::Side::Buyer } else { eravon_scan::scan::Side::Seller };
    let (record, missing) = match eravon_orders::table::record_at(data, owners, a3side, instance).map_err(why)? {
        Some(record) => {
            let missing = match docs::docs(data, owners, side, &record).map_err(why)? {
                Got::Docs(_) => Vec::new(),
                Got::Missing(m) => m,
            };
            (record, missing)
        }
        None => {
            let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
            let p: Pick = eravon_export::pick::picks(data, owners, keys, side, &sw).map_err(why)?.into_iter().find(|p| p.row.instance.eq_ignore_ascii_case(instance)).ok_or_else(|| No::new(R_NONE, vec![instance.to_string()]))?;
            (p.row.record, p.missing)
        }
    };
    if !missing.is_empty() {
        return Err(No::new(R_MISSING, missing.iter().map(|x| x.to_string()).collect()));
    }
    Ok(record)
}

pub fn run(j: &Job, w: &Wire, still: &dyn Fn() -> bool) -> Result<Outcome, No> {
    let side = if j.buyer { Side::Buyer } else { Side::Seller };
    let d = match docs::docs(&j.data, &j.owners, side, &j.record).map_err(why)? {
        Got::Docs(d) => d,
        Got::Missing(m) => return Err(No::new(R_MISSING, m.iter().map(|x| x.to_string()).collect())),
    };
    let ask = local_ask(j);
    let a = eravon_export::assemble::assemble(&w.t, &w.table, &d, &ask).map_err(why)?;
    if !still() {
        return Err(No::new(R_STALE, Vec::new()));
    }
    match j.kind {
        Kind::Dossier => {
            let name = eravon_export::seam::dossier_name(&a.case_id, j.now);
            let opts = eravon_recheck::check::Opts { events_from: eravon_chain::read::instance_from(ask.opened), ..eravon_export::export::opts_for(&j.registries) };
            let done = eravon_export::export::export(&w.t, &w.table, &a, &j.dir, &name, &opts).map_err(why)?;
            keep(j, &a);
            Ok(Outcome {
                kind: j.kind,
                files: vec![done.dir.clone()],
                report: Some(done.report.map_err(why)),
                selfcheck: Some(String::from_utf8_lossy(&done.selfcheck).into_owned()),
                shape: Some(Shape::of(&a)),
                statement: None,
                compliance: None,
            })
        }
        Kind::Statement => {
            let receipt = a.files.iter().find(|(n, _)| n == eravon_export::seam::FILE_RECEIPT).map(|(_, b)| b.clone()).unwrap_or_default();
            let st = eravon_statement::statement(&w.t, &w.table, &a.index, &d.listing, &receipt).map_err(why)?;
            let at = eravon_statement::write(&j.dir, &eravon_export::seam::statement_name(&a.case_id, j.now), &eravon_statement::statement_bytes(&st)).map_err(why)?;
            let compliance = match eravon_statement::compliance_bytes(&a.index) {
                Ok(b) => Some(eravon_statement::write(&j.dir, &eravon_export::seam::compliance_name(&a.case_id, j.now), &b).map_err(why)),
                Err(e) => Some(Err(why(e))),
            };
            let mut files = vec![at];
            if let Some(Ok(p)) = &compliance {
                files.push(p.clone());
            }
            if a.compliance {
                keep(j, &a);
            }
            Ok(Outcome { kind: j.kind, files, report: None, selfcheck: None, shape: None, statement: Some(st), compliance })
        }
    }
}

fn local_ask(j: &Job) -> eravon_export::assemble::Ask {
    let mut ask = eravon_export::assemble::Ask { serial: serial_of(j), ..Default::default() };
    if !j.buyer {
        if let Ok(o) = eravon_watch::order::load(&j.data, &j.me, &j.record) {
            ask.opened = o.opened;
        }
    }
    ask
}

fn serial_of(j: &Job) -> u64 {
    let Ok(s) = eravon_vault::store::open_read(&j.data) else { return 1 };
    let owners = eravon_vault::store::archive_of(&j.data, &j.me).unwrap_or_else(|_| vec![j.me.to_ascii_lowercase()]);
    let kept = owners.iter().filter_map(|o| s.list(o, eravon_store::seam::Box_::Cases).ok()).flatten().filter(|x| x.kind == eravon_store::record::Kind::Dossier).count();
    kept as u64 + 1
}

fn keep(j: &Job, a: &eravon_export::assemble::Assembled) {
    let Ok(s) = eravon_vault::store::open_write(&j.data, &j.writer, j.now) else { return };
    let id = format!("d-{}-{}", &a.case_id.trim_start_matches("0x")[..16.min(a.case_id.len().saturating_sub(2))], j.now);
    let _ = s.put(&j.me, eravon_store::record::Kind::Dossier, &id, &[("caseId".to_string(), a.case_id.clone())], &[(eravon_literals::literals::DOSSIER_INDEX_FILE.to_string(), a.index.clone())], j.now);
}
