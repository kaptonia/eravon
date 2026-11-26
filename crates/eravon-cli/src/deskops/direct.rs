use super::No;
use crate::shared::Wire;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Direct {
    pub dir: String,
    pub from: String,
    pub to: String,
    pub txs: Vec<String>,
    pub at: eravon_watch::order::Broadcast,
}

impl Direct {
    pub fn last(&self) -> &str {
        self.txs.last().map(String::as_str).unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checked {
    Done { tx: String, block: u64, status: u64 },
    Void(String),
    Pending(String),
}

pub fn open(me: &str, secret: &[u8; 32]) -> Result<Vec<Direct>, No> {
    let machine = eravon_store::writer::machine_dir().map_err(No::store)?;
    let open = eravon_send::sent::open(&machine, secret).map_err(|e| io(crate::send::record_fault(e)))?;
    Ok(open
        .into_iter()
        .filter_map(|(dir, recs)| {
            let last = recs.last()?.clone();
            let c = &last.carried;
            (c.from.is_empty() || c.from.eq_ignore_ascii_case(me)).then(|| Direct {
                dir: dir.display().to_string(),
                from: c.from.clone(),
                to: c.to.clone(),
                txs: recs.iter().map(|r| r.tx_hex()).collect(),
                at: eravon_watch::order::Broadcast { nonce: last.nonce, max_fee: c.max_fee.clone(), priority_fee: c.priority_fee.clone(), gas_limit: c.gas_limit },
            })
        })
        .collect())
}

pub fn find(me: &str, secret: &[u8; 32], tx: &str) -> Result<Direct, No> {
    open(me, secret)?.into_iter().find(|d| d.txs.iter().any(|t| t.eq_ignore_ascii_case(tx))).ok_or_else(|| No::new("E_V_MISSING", vec![tx.to_string()]))
}

fn sw(w: &Wire) -> eravon_watch::Wire<'_> {
    eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries }
}

fn void(dir: &std::path::Path, d: &Direct, secret: &[u8; 32]) -> Result<(), No> {
    let txs = d.txs.iter().map(|t| crate::send::hash_of(t).map(|h| (d.at.nonce, h))).collect::<Result<Vec<_>, _>>().map_err(io)?;
    eravon_send::sent::void(dir, &eravon_send::sent::Seal::of(secret, dir), &txs).map_err(|e| io(crate::send::record_fault(e)))
}

fn io(e: crate::fault::Fault) -> No {
    No::new(e.name(), vec![e.detail()])
}

pub fn check(w: &Wire, d: &Direct, secret: &[u8; 32]) -> Result<Checked, No> {
    let dir = std::path::Path::new(&d.dir);
    eravon_chain::read::ahead_receipts(&w.t, &w.table, &d.txs);
    for tx in &d.txs {
        if let Some(r) = eravon_chain::sending::receipt(&w.t, &w.table, tx).map_err(|e| No::new(e.name(), vec![e.detail()]))? {
            eravon_send::sent::done(dir, &eravon_send::sent::Seal::of(secret, dir), &crate::send::hash_of(tx).map_err(io)?).map_err(|e| io(crate::send::record_fault(e)))?;
            return Ok(Checked::Done { tx: tx.clone(), block: r.block, status: r.status });
        }
    }
    if d.from.is_empty() {
        return Ok(Checked::Pending(d.last().to_string()));
    }
    match eravon_watch::send::void_check(&d.from, &sw(w), d.last(), &d.at).map_err(|(n, why)| No::new(&n, vec![why]))? {
        eravon_watch::send::Voided::Void => {
            void(dir, d, secret)?;
            Ok(Checked::Void(d.last().to_string()))
        }
        _ => Ok(Checked::Pending(d.last().to_string())),
    }
}

pub fn give_up(w: &Wire, d: &Direct, secret: &[u8; 32]) -> Result<(), No> {
    if d.from.is_empty() {
        return Err(No::new(eravon_watch::send::NOT_VOID, vec![d.last().to_string()]));
    }
    match eravon_watch::send::void_check(&d.from, &sw(w), d.last(), &d.at).map_err(|(n, why)| No::new(&n, vec![why]))? {
        eravon_watch::send::Voided::Void => void(std::path::Path::new(&d.dir), d, secret),
        eravon_watch::send::Voided::Unused { done } => Err(No::new(eravon_watch::send::NOT_VOID, vec![d.last().to_string(), format!("{done} \u{2264} {}", d.at.nonce)])),
        eravon_watch::send::Voided::Held(said) => Err(No::new(eravon_watch::send::NOT_VOID, vec![d.last().to_string(), said])),
    }
}
