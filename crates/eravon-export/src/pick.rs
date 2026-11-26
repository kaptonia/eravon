use eravon_orders::table::{self, Row};
use eravon_scan::scan::Side as A3Side;

use crate::Why;
use crate::docs::{self, Got, Side};

#[derive(Clone, Debug)]
pub struct Pick {
    pub row: Row,
    pub missing: Vec<&'static str>,
}

impl Pick {
    pub fn ready(&self) -> bool {
        self.missing.is_empty()
    }
}

pub fn picks(data: &str, owners: &[String], keys: &[String], side: Side, w: &eravon_watch::Wire<'_>) -> Result<Vec<Pick>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let a3side = match side {
        Side::Seller => A3Side::Seller,
        Side::Buyer => A3Side::Buyer,
    };
    let t = table::table_of(data, owners, keys, a3side, w)?.complete()?;
    let mut out = Vec::with_capacity(t.rows.len());
    for row in t.rows {
        let missing = match docs::docs(data, owners, side, &row.record)? {
            Got::Docs(_) => Vec::new(),
            Got::Missing(m) => m,
        };
        out.push(Pick { row, missing });
    }
    Ok(out)
}

pub fn search(picks: &[Pick], q: &str, from: &str, to: &str) -> Result<Vec<usize>, Why> {
    let rows: Vec<Row> = picks.iter().map(|p| p.row.clone()).collect();
    let kept = table::search(&rows, q, from, to)?;
    Ok(picks
        .iter()
        .enumerate()
        .filter(|(_, p)| kept.iter().any(|r| r.instance.eq_ignore_ascii_case(&p.row.instance)))
        .map(|(i, _)| i)
        .collect())
}
