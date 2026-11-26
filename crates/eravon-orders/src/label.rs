use eravon_store::record::Kind;
use eravon_store::seam::Box_;

use crate::{Why, a5w};

fn id_of(buyer: &str) -> String {
    format!("label-{}", buyer.trim_start_matches("0x").to_ascii_lowercase())
}

pub fn all(data: &str, me: &str) -> Result<Vec<(String, String)>, Why> {
    let s = eravon_vault::store::open_read(data).map_err(a5w)?;
    let mut out: Vec<(String, String)> = Vec::new();
    for owner in eravon_vault::store::archive_of(data, me).map_err(a5w)? {
        let rows = match s.list(&owner, Box_::Orders) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(a5w(e)),
        };
        for l in rows
            .into_iter()
            .filter(|r| r.kind == Kind::OrderLabel)
            .filter_map(|r| Some((r.field("buyer")?.to_ascii_lowercase(), r.field("name")?.to_string())))
        {
            if !out.iter().any(|x| x.0 == l.0) {
                out.push(l);
            }
        }
    }
    Ok(out)
}

pub fn set(data: &str, writer: &str, now: u64, me: &str, buyer: &str, name: &str) -> Result<(), Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let s = eravon_vault::store::open_write(data, writer, now).map_err(a5w)?;
    s.put(
        me,
        Kind::OrderLabel,
        &id_of(buyer),
        &[("buyer".into(), buyer.to_ascii_lowercase()), ("name".into(), name.trim().to_string())],
        &[],
        now,
    )
    .map(|_| ())
    .map_err(a5w)
}
