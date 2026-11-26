use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRow {
    pub version: u64,
    pub link: String,
    pub expiry: u64,
    pub posted_tx: String,
    pub posted_hash: String,
}

#[derive(Clone, Debug)]
pub struct View {
    pub row: Row,
    pub draft: Draft,
    pub links: Vec<LinkRow>,
    pub revoked: Option<String>,
    pub link_state: String,
    pub pending: Vec<String>,
    pub company: Option<(String, String, Vec<String>)>,
    pub editing: bool,
    pub fees_signed: Vec<crate::resolve::Fee>,
    pub fees_new: Vec<crate::resolve::Fee>,
}

impl View {
    pub fn awaiting_company(&self) -> bool {
        self.company.as_ref().is_some_and(|(state, _, _)| state == "awaiting")
    }

    pub fn current_link(&self) -> Option<&LinkRow> {
        if !self.pending.is_empty() {
            return None;
        }
        self.links.first().filter(|l| l.version == self.row.version)
    }
}

pub fn view(data: &str, me: &str, id: &str, now: u64) -> Result<View, Fault> {
    here_or_refused(data, me, id)?;
    let row = list(data, me, "")?.into_iter().find(|r| r.id == id).ok_or_else(|| Fault::Missing(id.to_string()))?;
    let d = load(data, me, id)?;
    let s = eravon_vault::store::open_read(data)?;
    let owner = owner_in(&s, data, me, id)?;
    let me = owner.as_str();
    let rec = s.get(me, Box_::Products, id)?;
    let mut links = Vec::new();
    if row.listed {
        let prefix = format!("listing-{id}-v");
        for r in s.list(me, Box_::Products)?.into_iter().filter(|r| r.kind == Kind::Payload && r.id.starts_with(&prefix)) {
            let expiry = r.field(field::EXPIRY).and_then(|x| x.parse().ok()).unwrap_or(0);
            let (Some(link), Some(tx)) = (r.field(field::LINK), r.field(field::POSTED_TX)) else { continue };
            if expiry >= now {
                links.push(LinkRow {
                    version: r.field(field::VERSION).and_then(|x| x.parse().ok()).unwrap_or(0),
                    link: link.to_string(),
                    expiry,
                    posted_tx: tx.to_string(),
                    posted_hash: r.field(field::POSTED_HASH).unwrap_or_default().to_string(),
                });
            }
        }
        links.sort_by(|a, b| b.version.cmp(&a.version));
    }
    Ok(View {
        row,
        draft: d,
        links,
        link_state: rec.field(field::LINK_STATE).unwrap_or_default().to_string(),
        revoked: rec.field(field::REVOKED).map(str::to_string),
        editing: rec.field(field::EDITING).is_some(),
        fees_signed: rec.field(field::FEES_SIGNED).and_then(crate::resolve::fees_of_text).unwrap_or_default(),
        fees_new: rec.field(field::FEES_NEW).and_then(crate::resolve::fees_of_text).unwrap_or_default(),
        pending: rec.field(field::PENDING).unwrap_or_default().split(',').filter(|x| !x.is_empty()).map(str::to_string).collect(),
        company: rec.field(field::COMPANY).map(|st| {
            (
                st.to_string(),
                rec.field(field::COMPANY_ADDRESS).unwrap_or_default().to_string(),
                rec.field(field::COMPANY_DIGESTS).unwrap_or_default().split(',').filter(|x| !x.is_empty()).map(str::to_string).collect(),
            )
        }),
    })
}

pub fn company_check(c: &Ctx<'_>, id: &str) -> Result<(Vec<crate::company::Reg>, bool), Fault> {
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let owner = owner_in(&s, c.data, c.me, id)?;
    let rec = s.get(&owner, Box_::Products, id)?;
    let Some(state) = rec.field(field::COMPANY) else {
        return Err(Fault::Missing(format!("{id} 不是公司模式版本")));
    };
    let digests: Vec<String> = rec.field(field::COMPANY_DIGESTS).unwrap_or_default().split(',').filter(|x| !x.is_empty()).map(str::to_string).collect();
    let company = rec.field(field::COMPANY_ADDRESS).unwrap_or_default().to_string();
    let tid = rec.field(field::TERMS_ID).ok_or_else(|| Fault::Missing(format!("{id} 没有条款")))?.to_string();
    let terms = s.part(&owner, Box_::Products, &tid, PART_TERMS)?;
    let t = eravon_terms::doc::terms(&terms).map_err(|f| Fault::Company(format!("{}:无法读取条款", f.token.name())))?;
    let (rows, held) = crate::company::registered(&c.route, &company, &t.logic, &digests, c.now)
        .map_err(|(n, d)| Fault::Company(format!("{n}:{d}")))?;
    let posted = company_posted(&s, c, &owner, id, &rec, &company, &terms)?;
    let rec = s.get(&owner, Box_::Products, id)?;
    if rec.field(field::REVOKING).is_some() && rec.field(field::REVOKED).is_none() {
        let (tr, table) = c.route.clone().map_err(|(name, detail)| Fault::Chain { name, detail })?;
        let on = eravon_standard::chain::revoked(&tr, &table, &t.logic, &company, &t.terms_hash).map_err(|f| Fault::Chain { name: f.name(), detail: f.detail() })?;
        if on.value {
            let mut fields: Vec<(String, String)> = rec.fields.iter().filter(|(k, _)| ![field::LISTED, field::REVOKED].contains(&k.as_str())).cloned().collect();
            fields.push((field::LISTED.to_string(), "false".into()));
            fields.push((field::REVOKED.to_string(), REVOKED_BY_COMPANY.into()));
            s.put(&owner, Kind::Product, id, &fields, &[], c.now)?;
        }
    }
    let clear = held && posted;
    let rec = s.get(&owner, Box_::Products, id)?;
    if clear && state != "registered" {
        let mut fields: Vec<(String, String)> = rec.fields.iter().filter(|(k, _)| k != field::COMPANY).cloned().collect();
        fields.push((field::COMPANY.to_string(), "registered".into()));
        s.put(&owner, Kind::Product, id, &fields, &[], c.now)?;
    }
    Ok((rows, clear))
}

pub fn company_version(data: &str, me: &str, id: &str) -> bool {
    let Ok(s) = eravon_vault::store::open_read(data) else { return false };
    owner_in(&s, data, me, id).is_ok_and(|o| s.get(&o, Box_::Products, id).is_ok_and(|r| r.field(field::COMPANY).is_some()))
}

pub(super) fn owner_in(s: &eravon_vault::store::Store, data: &str, me: &str, id: &str) -> Result<String, Fault> {
    Ok(eravon_vault::store::archive_of(data, me)?.into_iter().find(|o| s.get(o, Box_::Products, id).is_ok()).unwrap_or_else(|| me.to_string()))
}

pub const REVOKED_BY_COMPANY: &str = "company";

fn company_posted(s: &eravon_vault::store::Store, c: &Ctx<'_>, owner: &str, id: &str, rec: &eravon_store::record::Record, company: &str, terms: &[u8]) -> Result<bool, Fault> {
    let version: u64 = rec.field(field::VERSION).and_then(|x| x.parse().ok()).unwrap_or(0);
    let lid = listing_id(id, version);
    let lrec = match s.get(owner, Box_::Products, &lid) {
        Ok(x) => x,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    if lrec.field(field::POSTED_TX).is_some() && lrec.field(field::LINK).is_some() {
        return Ok(true);
    }
    let listing = s.part(owner, Box_::Products, &lid, PART_LISTING)?;
    let key = eravon_terms::doc::terms_hash(terms);
    let (t, table) = c.route.clone().map_err(|(name, detail)| Fault::Chain { name, detail })?;
    let posts = eravon_scan::board::by_key(&t, &table, &key, 0, eravon_chain::seam::Upto::Latest).map_err(|e| Fault::Chain { name: e.name(), detail: e.detail() })?;
    let Some(p) = posts.iter().find(|p| p.poster.eq_ignore_ascii_case(company) && p.document == listing) else { return Ok(false) };
    let link = crate::link::link_of(eravon_literals::constants::active_standard().chain_id, &p.tx, &listing).map_err(|(name, detail)| Fault::Chain { name, detail })?;
    let expiry: u64 = lrec.field(field::EXPIRY).and_then(|x| x.parse().ok()).unwrap_or(0);
    let mut lf: Vec<(String, String)> = lrec.fields.iter().filter(|(k, _)| ![field::POSTED_TX, field::POSTED_HASH, field::LINK].contains(&k.as_str())).cloned().collect();
    lf.push((field::POSTED_TX.to_string(), p.tx.clone()));
    lf.push((field::POSTED_HASH.to_string(), eravon_galeed::hash::hex_encode(&p.hash)));
    lf.push((field::LINK.to_string(), link.clone()));
    s.put(owner, Kind::Payload, &lid, &lf, &[], c.now)?;
    let mut fields: Vec<(String, String)> = rec.fields.iter().filter(|(k, _)| ![field::LINK, field::LINK_EXPIRY, field::LINK_STATE, field::PENDING].contains(&k.as_str())).cloned().collect();
    fields.push((field::LINK.to_string(), link));
    fields.push((field::LINK_EXPIRY.to_string(), expiry.to_string()));
    let left: Vec<&str> = rec.field(field::PENDING).unwrap_or_default().split(',').filter(|x| !x.is_empty() && *x != super::POST).collect();
    if !left.is_empty() {
        fields.push((field::PENDING.to_string(), left.join(",")));
    }
    s.put(owner, Kind::Product, id, &fields, &[], c.now)?;
    Ok(true)
}

pub fn proposal_dir(dir: &str, id: &str, version: u64) -> String {
    std::path::Path::new(dir).join(format!("eravon-offer-{id}-v{version}")).display().to_string()
}

pub fn revoke_dir(dir: &str, id: &str, version: u64) -> String {
    std::path::Path::new(dir).join(format!("eravon-revoke-{id}-v{version}")).display().to_string()
}

pub(super) fn write_proposal(out: &str, proposal: &[u8], docs: &[eravon_standard::proposal::Original]) -> Result<String, Fault> {
    let dps = docs
        .iter()
        .map(|d| eravon_literals::seam::named_under(std::path::Path::new(out), &d.name).map(|x| x.display().to_string()).map_err(Fault::Export))
        .collect::<Result<Vec<_>, Fault>>()?;
    std::fs::create_dir_all(out).map_err(|e| Fault::Export(format!("无法创建 {out}:{e}")))?;
    let path = std::path::Path::new(out).join(PART_PROPOSAL).display().to_string();
    std::fs::write(&path, proposal).map_err(|e| Fault::Export(format!("无法写入 {path}:{e}")))?;
    for (d, dp) in docs.iter().zip(&dps) {
        std::fs::write(dp, &d.bytes).map_err(|e| Fault::Export(format!("无法写入 {dp}:{e}")))?;
    }
    Ok(path)
}

pub fn proposal_time(data: &str, me: &str, id: &str) -> Result<Option<(u64, String)>, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let owner = owner_in(&s, data, me, id)?;
    let me = owner.as_str();
    let rec = s.get(me, Box_::Products, id)?;
    Ok(rec.field(field::COMPANY_TIME).and_then(|v| v.split_once('|')).and_then(|(b, sh)| Some((b.parse().ok()?, sh.to_string()))))
}

pub fn export_proposal(data: &str, me: &str, id: &str, dir: &str) -> Result<String, Fault> {
    here_or_refused(data, me, id)?;
    let s = eravon_vault::store::open_read(data)?;
    let owner = owner_in(&s, data, me, id)?;
    let me = owner.as_str();
    let rec = s.get(me, Box_::Products, id)?;
    if rec.field(field::COMPANY).is_none() {
        return Err(Fault::Missing(format!("{id} 没有公司提案")));
    }
    let version: u64 = rec.field(field::VERSION).and_then(|x| x.parse().ok()).unwrap_or(0);
    let lid = listing_id(id, version);
    let proposal = match s.part(me, Box_::Products, &lid, PART_PROPOSAL) {
        Ok(b) => b,
        Err(eravon_store::fault::Fault::Missing(_)) => {
            let why = match rec.field(field::COMPANY_REFUSED).and_then(|x| x.split_once('|')) {
                Some((n, d)) => format!("{n}:{d}"),
                None => "E_GOODS_COMPANY".to_string(),
            };
            return Err(Fault::Company(format!("{why}:这一版保存时没生成公司提案;改了设置或期限之后再保存一版,新版带提案")));
        }
        Err(e) => return Err(e.into()),
    };
    let p = eravon_standard::proposal::read(&proposal).map_err(|f| Fault::Company(format!("{}:{}", f.name(), f.detail())))?;
    let out = proposal_dir(dir, id, version);
    let dps = p
        .documents
        .iter()
        .map(|d| eravon_literals::seam::named_under(std::path::Path::new(&out), &d.name).map(|x| x.display().to_string()).map_err(Fault::Export))
        .collect::<Result<Vec<_>, Fault>>()?;
    std::fs::create_dir_all(&out).map_err(|e| Fault::Export(format!("无法创建 {out}:{e}")))?;
    let path = std::path::Path::new(&out).join(PART_PROPOSAL).display().to_string();
    std::fs::write(&path, &proposal).map_err(|e| Fault::Export(format!("无法写入 {path}:{e}")))?;
    for (d, dp) in p.documents.iter().zip(&dps) {
        let b = s.part(me, Box_::Products, &lid, &d.name)?;
        std::fs::write(dp, &b).map_err(|e| Fault::Export(format!("无法写入 {dp}:{e}")))?;
    }
    Ok(path)
}

pub(super) fn gate(_v: &Verdict) -> Result<(), Fault> {
    Ok(())
}


pub fn terms_hashes(data: &str, me: &str, id: &str) -> Result<Vec<String>, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let owner = owner_in(&s, data, me, id)?;
    let me = owner.as_str();
    let prefix = format!("terms-{id}-v");
    let rows = match s.list(me, Box_::Products) {
        Ok(r) => r,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(rows
        .into_iter()
        .filter(|r| r.kind == Kind::TermsDocument && r.id.starts_with(&prefix))
        .filter_map(|r| r.field(field::TERMS_HASH).map(str::to_string))
        .collect())
}

pub fn load(data: &str, me: &str, id: &str) -> Result<Draft, Fault> {
    let b = live_bytes(data, me, id)?;
    Draft::open(&b).ok_or_else(|| Fault::Form(id.to_string()))
}

pub fn live_bytes(data: &str, me: &str, id: &str) -> Result<Vec<u8>, Fault> {
    here_or_refused(data, me, id)?;
    let s = eravon_vault::store::open_read(data)?;
    Ok(s.part(&owner_in(&s, data, me, id)?, Box_::Products, id, PART_DRAFT)?)
}

pub fn load_edit(data: &str, me: &str, id: &str) -> Result<Draft, Fault> {
    here_or_refused(data, me, id)?;
    let s = eravon_vault::store::open_read(data)?;
    let b = edit_bytes(&s, &owner_in(&s, data, me, id)?, id)?;
    Draft::open(&b).ok_or_else(|| Fault::Form(id.to_string()))
}

pub(super) fn edit_bytes(s: &eravon_vault::store::Store, me: &str, id: &str) -> Result<Vec<u8>, Fault> {
    let rec = s.get(me, Box_::Products, id)?;
    let part = if rec.field(field::EDITING).is_some() { edit_part() } else { PART_DRAFT };
    Ok(s.part(me, Box_::Products, id, part)?)
}

pub(super) fn edit_part() -> &'static str {
    PART_EDIT
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: String,
    pub token: String,
    pub draft: bool,
    pub cands: Vec<String>,
    pub version: u64,
    pub updated: u64,
    pub listed: bool,
    pub pending: Vec<String>,
    pub series: Option<crate::series::Series>,
    pub chain: Option<u64>,
}

pub fn chain_of(s: &eravon_vault::store::Store, owner: &str, r: &eravon_store::record::Record) -> Option<u64> {
    if let Some(c) = r.field(field::CHAIN).and_then(|x| x.parse().ok()) {
        return Some(c);
    }
    let version: u64 = r.field(field::VERSION).and_then(|x| x.parse().ok()).unwrap_or(0);
    if version == 0 {
        return None;
    }
    let b = s.part(owner, Box_::Products, &listing_id(&r.id, version), PART_LISTING).ok()?;
    match eravon_literals::json::read(&b).ok()?.get("chainId") {
        Some(eravon_literals::json::In::Int(n)) => u64::try_from(*n).ok(),
        _ => None,
    }
}

pub fn chain_here(data: &str, me: &str) -> Result<u64, Fault> {
    match eravon_literals::constants::run_chain() {
        Some(c) => Ok(c),
        None => Ok(eravon_vault::store::network(data, me).map_err(|e| Fault::Store { name: "E_GOODS_CHAIN_UNREAD", detail: format!("{}:{}", e.name(), e.detail()) })?),
    }
}

pub(super) fn on_this_chain(s: &eravon_vault::store::Store, data: &str, me: &str, owner: &str, r: &eravon_store::record::Record) -> Result<(), Fault> {
    let Some(chain) = chain_of(s, owner, r) else { return Ok(()) };
    let here = chain_here(data, me)?;
    if chain == here {
        return Ok(());
    }
    Err(Fault::OtherChain { id: r.id.clone(), chain, here })
}

pub(super) fn here_or_refused(data: &str, me: &str, id: &str) -> Result<(), Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let owner = owner_in(&s, data, me, id)?;
    match s.get(&owner, Box_::Products, id) {
        Ok(r) => on_this_chain(&s, data, me, &owner, &r),
        Err(_) => Ok(()),
    }
}

pub fn records(data: &str, me: &str) -> Result<Vec<(String, eravon_store::record::Record)>, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let mut recs: Vec<(String, eravon_store::record::Record)> = Vec::new();
    for owner in eravon_vault::store::archive_of(data, me)? {
        let got = match s.list(&owner, Box_::Products) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        for r in got {
            if !recs.iter().any(|(_, x)| x.id == r.id) {
                recs.push((owner.clone(), r));
            }
        }
    }
    eravon_store::record::newest_first_by(&mut recs, |(_, r)| r.updated, |(_, r)| r.id.as_bytes());
    Ok(recs)
}

pub fn listed_order(recs: &mut [(String, eravon_store::record::Record)]) {
    eravon_store::record::newest_first_by(recs, |(_, r)| r.created, |(_, r)| r.id.as_bytes());
}

pub fn records_here(data: &str, me: &str) -> Result<Vec<(String, eravon_store::record::Record)>, Fault> {
    let here = chain_here(data, me)?;
    let mut recs = records(data, me)?;
    listed_order(&mut recs);
    let s = eravon_vault::store::open_read(data)?;
    Ok(recs.into_iter().filter(|(owner, r)| chain_of(&s, owner, r).is_none_or(|c| c == here)).collect())
}

pub fn record(data: &str, me: &str, id: &str) -> Result<eravon_store::record::Record, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let owner = owner_in(&s, data, me, id)?;
    let r = s.get(&owner, Box_::Products, id)?;
    on_this_chain(&s, data, me, &owner, &r)?;
    Ok(r)
}

pub fn terms_of(data: &str, me: &str, id: &str) -> Result<Option<Vec<u8>>, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let owner = owner_in(&s, data, me, id)?;
    let rec = s.get(&owner, Box_::Products, id)?;
    let Some(tid) = rec.field(field::TERMS_ID) else { return Ok(None) };
    Ok(Some(s.part(&owner, Box_::Products, tid, PART_TERMS)?))
}

pub fn signed_words(data: &str, me: &str) -> Result<Vec<eravon_cands::view::Signed>, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let mut out = Vec::new();
    for (owner, r) in records(data, me)?.into_iter().filter(|(_, r)| r.kind == Kind::Product) {
        if !on_sale(&r) {
            continue;
        }
        let Some(tid) = r.field(field::TERMS_ID) else { continue };
        let Ok(bytes) = s.part(&owner, Box_::Products, tid, PART_TERMS) else { continue };
        let Ok(t) = eravon_terms::doc::terms(&bytes) else { continue };
        let name = s.part(&owner, Box_::Products, &r.id, PART_DRAFT).ok().and_then(|b| Draft::open(&b)).map(|d| d.name).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| r.id.clone());
        let words = t.judges.iter().map(|j| (j.judge.clone(), j.amanah_hash.clone(), String::new())).chain(t.guilds.iter().map(|g| (g.address.clone(), g.amanah_hash.clone(), g.quote_hash.clone())));
        out.extend(words.map(|(address, word, quote)| eravon_cands::view::Signed { address, word, item: name.clone(), quote, token: t.token.to_ascii_lowercase() }));
    }
    Ok(out)
}

fn on_sale(r: &eravon_store::record::Record) -> bool {
    r.field(field::DRAFT) == Some("false") && r.field(field::LISTED) != Some("false") && r.field(field::REVOKED).is_none()
}

pub fn list(data: &str, me: &str, query: &str) -> Result<Vec<Row>, Fault> {
    let here = chain_here(data, me)?;
    let mut rows = list_every_chain(data, me, query)?;
    rows.retain(|r| r.chain.is_none_or(|c| c == here));
    Ok(rows)
}

pub fn list_every_chain(data: &str, me: &str, query: &str) -> Result<Vec<Row>, Fault> {
    let q = query.trim().to_lowercase();
    let mut recs = records(data, me)?;
    listed_order(&mut recs);
    let s = eravon_vault::store::open_read(data)?;
    let table = names_in(&cand_table(&s, me)?);
    let name_of = |p: &str| name_of(&table, p).unwrap_or_else(|| short_of(p));
    let mut out = Vec::new();
    for (owner, r) in recs.into_iter().filter(|(_, r)| r.kind == Kind::Product) {
        let b = match s.part(&owner, Box_::Products, &r.id, PART_DRAFT) {
            Ok(b) => b,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        let d = Draft::open(&b).ok_or_else(|| Fault::Form(r.id.clone()))?;
        out.push(Row {
            id: r.id.clone(),
            name: d.name.clone(),
            description: d.description.clone(),
            price: d.price.clone(),
            token: d.token.clone(),
            draft: r.field(field::DRAFT) != Some("false"),
            cands: d.cands.iter().map(|p| name_of(p)).collect(),
            version: r.field(field::VERSION).and_then(|x| x.parse().ok()).unwrap_or(0),
            updated: r.updated,
            listed: r.field(field::DRAFT) == Some("false") && r.field(field::LISTED) != Some("false"),
            pending: r.field(field::PENDING).unwrap_or_default().split(',').filter(|x| !x.is_empty()).map(str::to_string).collect(),
            series: crate::series::cells(&d).ok().flatten(),
            chain: chain_of(&s, &owner, &r),
        });
    }
    if !q.is_empty() {
        out.retain(|r| r.name.to_lowercase().contains(&q) || r.description.to_lowercase().contains(&q));
    }
    Ok(out)
}

fn cand_table(s: &eravon_vault::store::Store, me: &str) -> Result<Vec<(String, String, String, u64)>, Fault> {
    match s.candidates(me) {
        Ok(t) => Ok(t),
        Err(eravon_store::fault::Fault::Missing(_)) => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

pub fn name_of(names: &[(String, String)], p: &str) -> Option<String> {
    let p = p.to_ascii_lowercase();
    names.iter().find(|(a, _)| p.ends_with(&a.to_ascii_lowercase())).map(|(_, n)| n.clone())
}

fn names_in(table: &[(String, String, String, u64)]) -> Vec<(String, String)> {
    table.iter().filter(|(_, _, l, _)| !l.trim().is_empty()).map(|(_, a, l, _)| (a.clone(), l.trim().to_string())).collect()
}

pub fn names(data: &str, me: &str) -> Result<Vec<(String, String)>, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    Ok(names_in(&cand_table(&s, me)?))
}

fn short_of(p: &str) -> String {
    p.rsplit(':').next().map(|a| {
        let c: Vec<char> = a.chars().collect();
        if c.len() > 10 { format!("{}…{}", c[..6].iter().collect::<String>(), c[c.len() - 4..].iter().collect::<String>()) } else { a.to_string() }
    }).unwrap_or_default()
}

pub fn stored(data: &str, me: &str, id: Option<&str>) -> Option<Draft> {
    match id {
        Some(i) => load_edit(data, me, i).ok(),
        None => Draft::open(&Draft::default().seal()),
    }
}

pub fn dirty(stored: Option<&Draft>, d: &Draft) -> bool {
    stored.is_none_or(|x| x.seal() != d.seal())
}


pub fn using(data: &str, token: &str) -> Result<Vec<(String, String)>, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let mut out = Vec::new();
    for me in s.identities()? {
        let recs = match s.list(&me, Box_::Products) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        for r in recs.into_iter().filter(|r| r.kind == Kind::Product) {
            let parts: &[&str] = if r.field(field::EDITING).is_some() { &[PART_DRAFT, PART_EDIT] } else { &[PART_DRAFT] };
            for part in parts {
                let b = match s.part(&me, Box_::Products, &r.id, part) {
                    Ok(b) => b,
                    Err(eravon_store::fault::Fault::Missing(_)) => continue,
                    Err(e) => return Err(e.into()),
                };
                let d = Draft::open(&b).ok_or_else(|| Fault::Form(r.id.clone()))?;
                if crate::draft::token_address(&d.token).is_some_and(|a| a.eq_ignore_ascii_case(token.trim())) {
                    out.push((me.clone(), d.name.clone()));
                    break;
                }
            }
        }
    }
    Ok(out)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kept {
    pub goods_text: Option<String>,
    pub delivery_text: Option<String>,
    pub posted_tx: Option<String>,
}

pub fn kept(data: &str, me: &str, terms_hash: &str, goods_hash: &str, delivery_hash: &str, listing_sha: &str) -> Result<Kept, Fault> {
    let s = eravon_vault::store::open_read(data)?;
    let same = |a: &str, b: &str| !a.is_empty() && a.trim_start_matches("0x").eq_ignore_ascii_case(b.trim_start_matches("0x"));
    let holds = |rows: &[eravon_store::record::Record]| rows.iter().any(|r| r.kind == Kind::TermsDocument && r.field(field::TERMS_HASH).is_some_and(|h| same(h, terms_hash)));
    let mut held = None;
    for owner in eravon_vault::store::archive_of(data, me)? {
        match s.list(&owner, Box_::Products) {
            Ok(r) if holds(&r) => {
                held = Some((owner, r));
                break;
            }
            Ok(_) | Err(eravon_store::fault::Fault::Missing(_)) => {}
            Err(e) => return Err(e.into()),
        }
    }
    let Some((me, rows)) = held else { return Ok(Kept::default()) };
    let me = me.as_str();
    let mut out = Kept::default();
    if let Some(r) = rows.iter().find(|r| r.kind == Kind::TermsDocument && r.field(field::TERMS_HASH).is_some_and(|h| same(h, terms_hash))) {
        for (want, part, slot) in [(goods_hash, PART_GOODS_TEXT, &mut out.goods_text), (delivery_hash, PART_DELIVERY_TEXT, &mut out.delivery_text)] {
            if want.is_empty() || !r.parts.iter().any(|(n, _)| n == part) {
                continue;
            }
            let text = String::from_utf8_lossy(&s.part(me, Box_::Products, &r.id, part)?).into_owned();
            if same(&terms::segment_hash(&text), want) {
                *slot = Some(text);
            }
        }
    }
    out.posted_tx = rows
        .iter()
        .find(|r| r.kind == Kind::Payload && r.field(field::POSTED_HASH).is_some_and(|h| same(h, listing_sha)))
        .and_then(|r| r.field(field::POSTED_TX))
        .map(str::to_string);
    Ok(out)
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn a_candidate_is_named_by_the_stored_table() {
        let a = format!("0x{}", "ab".repeat(20));
        let table = vec![("row-a".to_string(), a.clone(), " 甲 ".to_string(), 1u64), ("row-b".to_string(), format!("0x{}", "cd".repeat(20)), "  ".to_string(), 2)];
        let names = names_in(&table);
        assert_eq!(names, vec![(a.clone(), "甲".to_string())], "没写名字的那一行不算有名");
        assert_eq!(name_of(&names, &a.to_ascii_uppercase().replacen("0X", "0x", 1)).as_deref(), Some("甲"));
        assert_eq!(name_of(&names, &format!("galeed:guild:1:{a}")).as_deref(), Some("甲"), "指针按它末尾的地址认");
        assert_eq!(name_of(&names, &format!("0x{}", "cd".repeat(20))), None);
    }
}

#[cfg(test)]
mod on_sale_tests {
    use super::{field, on_sale};
    use eravon_store::record::{Kind, Record};

    fn item(fields: &[(&str, &str)]) -> Record {
        Record { id: "g1".into(), kind: Kind::Product, created: 1, updated: 1, writer: "w".into(), fields: fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), parts: Vec::new() }
    }

    #[test]
    fn a_draft_a_delisted_or_a_revoked_item_is_not_on_sale() {
        assert!(on_sale(&item(&[(field::DRAFT, "false")])), "published and listed");
        assert!(on_sale(&item(&[(field::DRAFT, "false"), (field::LISTED, "true")])), "published and listed, said so");
        assert!(!on_sale(&item(&[])), "a draft never published");
        assert!(!on_sale(&item(&[(field::DRAFT, "true")])), "a draft");
        assert!(!on_sale(&item(&[(field::DRAFT, "false"), (field::LISTED, "false")])), "taken off the shelf");
        assert!(!on_sale(&item(&[(field::DRAFT, "false"), (field::REVOKED, "1")])), "taken down");
    }
}

#[cfg(test)]
mod dirty_tests {
    use super::{dirty, Draft};

    #[test]
    fn the_draft_is_compared_with_the_copy_the_page_holds() {
        let held = Draft::open(&Draft::default().seal()).expect("an empty draft reads");
        let mut d = held.clone();
        assert!(!dirty(Some(&held), &d));
        d.name = "改了名".into();
        assert!(dirty(Some(&held), &d));
        assert!(dirty(None, &held), "a copy that did not read");
    }
}

#[cfg(test)]
mod order_tests {
    #[test]
    fn the_list_stands_in_the_order_items_were_made_whatever_was_saved_since() {
        let rec = |id: &str, created: u64, updated: u64| ("0xa".to_string(), eravon_store::record::Record { id: id.into(), kind: eravon_store::record::Kind::Product, created, updated, writer: String::new(), fields: Vec::new(), parts: Vec::new() });
        let mut recs = vec![rec("old", 1, 90), rec("new", 5, 6), rec("mid", 3, 3)];
        super::listed_order(&mut recs);
        assert_eq!(recs.iter().map(|(_, r)| r.id.as_str()).collect::<Vec<_>>(), vec!["new", "mid", "old"], "a background write to the oldest leaves it last");
    }
}
