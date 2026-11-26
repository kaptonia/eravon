use super::No;

pub type Pick = eravon_dispute::seal::Pick;

pub const R_NO_JUDGE: &str = "E_JUDGE_NONE";
pub const R_NOT_KEPT: &str = "E_EVIDENCE_NOT_KEPT";

#[derive(Clone, Debug)]
pub struct Seated {
    pub address: String,
    pub name: Option<String>,
    pub home: Option<String>,
    pub note_md: Option<String>,
    pub frozen: String,
    pub frozen_version: Option<u64>,
    pub frozen_text: Option<String>,
    pub frozen_discarded: bool,
    pub pick: Pick,
    pub unread: Option<String>,
}

pub fn read(w: &eravon_cands::wire::Wire, judge: &str, amanah: &str) -> Result<Seated, No> {
    if judge.trim_start_matches("0x").chars().all(|c| c == '0') {
        return Err(No::new(R_NO_JUDGE, vec![judge.to_string()]));
    }
    let absent = |a: eravon_cands::fault::Absent| No::new(a.name(), vec![a.detail()]);
    let logs = eravon_cands::registry::logs_of(w, &[judge.to_ascii_lowercase()]).map_err(absent)?;
    let r = eravon_cands::judge::in_case(w, &logs, judge, amanah).map_err(absent)?;
    Ok(Seated {
        address: r.view.address.clone(),
        name: r.view.name.clone(),
        home: r.view.home.clone(),
        note_md: r.view.note_md.clone(),
        frozen: r.frozen.clone(),
        frozen_version: r.frozen_version,
        frozen_text: r.frozen_text.clone(),
        frozen_discarded: r.frozen_discarded,
        pick: eravon_dispute::seal::pick_from(&r.ledger),
        unread: r.ledger.as_ref().err().cloned(),
    })
}

pub fn kept(data: &str, owners: &[String], instance: &str, manifest: &[u8; 32]) -> Result<Vec<u8>, No> {
    use eravon_store::seam::Box_;
    let s = eravon_vault::store::open_read(data).map_err(No::store)?;
    for owner in owners {
        for b in [Box_::Packs, Box_::Orders] {
            let rows = match s.list(owner, b) {
                Ok(r) => r,
                Err(eravon_store::fault::Fault::Missing(_)) => continue,
                Err(e) => return Err(No::store(e)),
            };
            for r in rows.iter().filter(|r| r.field("instance").is_some_and(|i| i.eq_ignore_ascii_case(instance))) {
                for (name, _) in &r.parts {
                    let Ok(bytes) = s.part(owner, b, &r.id, name) else { continue };
                    if eravon_evidence::sheet::check(&bytes).is_ok() && eravon_evidence::sheet::manifest(&bytes).is_ok_and(|m| m == *manifest) {
                        return Ok(bytes);
                    }
                }
            }
        }
    }
    Err(No::new(R_NOT_KEPT, vec![eravon_standard::order::hex_of(manifest), instance.to_string()]))
}

pub fn pick_out(p: &Pick) -> eravon_literals::json::Out {
    use eravon_literals::json::Out;
    let mut r = vec![];
    match p {
        Pick::One(b) => {
            r.push(("recipient", Out::S(b.recipient().to_string())));
            r.push(("seq", Out::N(b.seq())));
            r.push(("state", Out::S(Pick::ONE.into())));
        }
        Pick::Candidates { reason, list, .. } => {
            r.push(("count", Out::N(list.len() as u64)));
            r.push(("reason", Out::S(reason.name().into())));
            r.push(("state", Out::S(Pick::CANDIDATES.into())));
        }
        Pick::None => r.push(("state", Out::S(Pick::NONE.into()))),
        Pick::Unread => r.push(("state", Out::S(Pick::UNREAD.into()))),
    }
    Out::O(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seal_with_no_judge_or_no_kept_manifest_is_refused_by_name() {
        let t = eravon_chain::transport::Auto::web_roots().expect("roots");
        let table = eravon_chain::endpoints::Table::new(vec!["http://127.0.0.1:1".to_string()]).expect("table");
        let route: eravon_goods::company::Route = Ok((t, table));
        let w = eravon_cands::wire::Wire::on(&route, &["0x00000000000000000000000000000000000000cc".to_string()], 1_790_000_000).expect("a wire");
        let none = read(&w, "0x0000000000000000000000000000000000000000", "").expect_err("no judge");
        assert_eq!(none.name, R_NO_JUDGE);
        let data = eravon_vault::testing::fresh();
        let me = "0x00000000000000000000000000000000000000aa".to_string();
        let got = kept(&data, &[me], "0x00000000000000000000000000000000000000bb", &[7u8; 32]).expect_err("nothing kept");
        assert_eq!(got.name, R_NOT_KEPT);
    }
}
