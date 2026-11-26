use eravon_store::record::Kind;
use eravon_store::seam::Box_;

use crate::order::Order;
use crate::Wire;

pub const FIELD: &str = "decimals";

pub fn archived(data: &str, me: &str, terms_hash: &str) -> Option<u64> {
    let s = eravon_vault::store::open_read(data).ok()?;
    for owner in eravon_vault::store::archive_of(data, me).ok()? {
        let Ok(rows) = s.list(&owner, Box_::Products) else { continue };
        for r in rows.into_iter().filter(|r| r.kind == Kind::Payload) {
            let Some(d) = r.field(FIELD).and_then(|x| x.parse::<u64>().ok()) else { continue };
            for (name, _) in &r.parts {
                let Ok(b) = s.part(&owner, Box_::Products, &r.id, name) else { continue };
                if eravon_link::listing::listing(&b).is_ok_and(|p| p.terms_hash.eq_ignore_ascii_case(terms_hash)) {
                    return Some(d);
                }
            }
        }
    }
    None
}

pub fn of(o: &Order) -> Option<u64> {
    o.extra(FIELD).and_then(|x| x.parse().ok())
}

pub fn fill(data: &str, me: &str, w: Option<&Wire<'_>>, o: &mut Order, terms_hash: &str) -> bool {
    if of(o).is_some() {
        return false;
    }
    let got = archived(data, me, terms_hash).or_else(|| {
        let w = w?;
        let cw = eravon_cands::wire::Wire { t: w.t.clone(), table: w.table.clone(), registries: Vec::new(), now: 0 };
        eravon_cands::judge::decimals_of(&cw, &o.token)
    });
    match got {
        Some(d) => {
            o.set_extra(FIELD, &d.to_string());
            true
        }
        None => false,
    }
}
