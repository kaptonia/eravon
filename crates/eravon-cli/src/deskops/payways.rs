use super::No;
use eravon_payways::ways::{self, Edit, Ways};

pub struct Saved {
    pub ways: Ways,
}

pub fn save(data: &str, writer: &str, now: u64, me: &str, e: &Edit) -> Result<Saved, No> {
    let w = ways::save(data, writer, now, me, e).map_err(|f| No::new(f.name(), vec![f.detail()]))?;
    if e.fund.is_some() || e.own.is_some() {
        let _ = eravon_goods::goods::note_fund(data, writer, me, now);
    }
    Ok(Saved { ways: w })
}
