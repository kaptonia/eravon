use eravon_store::record::Kind as K;
use eravon_store::seam::Box_;
use eravon_terms::u256::U256;

use super::said::Said;

pub const CHAIN_ID: &str = "chainId";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tx {
    pub id: String,
    pub hash: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub order: String,
    pub token: String,
    pub amount: String,
    pub block: String,
    pub status: String,
    pub gas_used: String,
    pub gas_price: String,
    pub fee: String,
    pub at: u64,
}

impl Tx {
    pub fn settled(&self) -> bool {
        !self.status.is_empty() && !self.fee.is_empty()
    }
}

pub fn list(data: &str, owner: &str) -> Result<Vec<Tx>, Said> {
    let st = eravon_vault::store::open_read(data).map_err(|f| Said::bad(&f.name(), f.detail()))?;
    let rows = match st.list(owner, Box_::Orders) {
        Ok(r) => r,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(Vec::new()),
        Err(e) => return Err(Said::bad(e.name(), e.detail())),
    };
    let here = eravon_literals::constants::current_network().standard.chain_id.to_string();
    let mut mine = Vec::new();
    for r in rows.into_iter().filter(|r| r.kind == K::TxReceipt) {
        match r.field(CHAIN_ID) {
            Some(c) if c == here => mine.push(r),
            Some(_) => {}
            None => return Err(Said::bad("E_P7_RECORD", format!("交易记录 {} 没有网络号,读不成", r.id))),
        }
    }
    let mut out: Vec<Tx> = mine
        .into_iter()
        .map(|r| {
            let g = |k: &str| r.field(k).unwrap_or_default().to_string();
            Tx {
                id: r.id.clone(),
                hash: g("hash"),
                from: g("from"),
                to: g("to"),
                kind: g("kind"),
                order: g("order"),
                token: g("token"),
                amount: g("amount"),
                block: g("block"),
                status: g("status"),
                gas_used: g("gasUsed"),
                gas_price: g("gasPrice"),
                fee: g("fee"),
                at: r.field("at").and_then(|x| x.parse().ok()).unwrap_or(r.created),
            }
        })
        .collect();
    out.sort_by(|a, b| b.at.cmp(&a.at).then(b.hash.cmp(&a.hash)));
    Ok(out)
}

pub fn fee_total(txs: &[Tx], instance: &str) -> Option<(String, usize)> {
    let mine: Vec<&Tx> = txs.iter().filter(|t| !t.fee.is_empty() && t.order.eq_ignore_ascii_case(instance)).collect();
    if mine.is_empty() {
        return None;
    }
    let mut sum = U256::ZERO;
    for t in &mine {
        sum = sum.overflowing_add(&U256::from_dec(&t.fee).unwrap_or(U256::ZERO)).0;
    }
    Some((sum.to_dec(), mine.len()))
}

pub fn order_fee(data: &str, owners: &[String], instance: &str) -> Result<Option<(String, usize)>, Said> {
    let mut all = Vec::new();
    for o in owners {
        all.extend(list(data, o)?);
    }
    Ok(fee_total(&all, instance))
}
