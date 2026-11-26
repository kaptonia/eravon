use eravon_chain as a2;
use eravon_link as k3;
use eravon_literals::json::Out;
use eravon_scan::scan as a3;
use eravon_store::record::Kind;
use eravon_store::seam::Box_;
use eravon_vault::session::Session;

use crate::fault::{Fault, R};

pub const SAY_BY_CALL: &str = "此订单以调用同意";
pub const SAY_COMPANY: &str = "公司订单:操作将导出提案";
pub const SAY_FOREIGN_KEY: &str = "这份订单的买家钥不在本机";
pub const SAY_TERMS_MISSING: &str = "条款未保存";
pub const SAY_NOT_OPENED: &str = "订单尚未开启";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Key,
    ByCall,
    Company,
}

impl Mode {
    pub const fn name(self) -> &'static str {
        match self {
            Mode::Key => "key",
            Mode::ByCall => "byCall",
            Mode::Company => "company",
        }
    }

    pub const fn note(self) -> &'static str {
        match self {
            Mode::Key => "",
            Mode::ByCall => SAY_BY_CALL,
            Mode::Company => SAY_COMPANY,
        }
    }

    const fn stored(self) -> &'static str {
        match self {
            Mode::Company => "byCall",
            m => m.name(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub case_id: String,
    pub instance: String,
    pub tx: String,
    pub terms_hash: String,
    pub amount: String,
    pub token: String,
    pub seller: String,
    pub why: Option<Fault>,
    pub terms_saved: bool,
    pub decimals: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    pub address: String,
    pub label: String,
    pub mode: Mode,
    pub order: Order,
}

pub fn run(s: &Session, me: &str, bytes: &[u8], writer: &str, now: u64) -> R<Done> {
    crate::mark::mark(crate::mark::FEATURE);
    eravon_vault::session::need_open(s)?;
    let pk = k3::pack::pack(bytes)?;
    let buyer = pk.receipt.buyer_key.to_ascii_lowercase();
    let mode = mode_of(&pk, s, me, &buyer)?;
    let mut order = found(s.data(), me, &buyer, &pk.case_id, &pk.instance, pk.chain_id, &pk.listing.logic);
    let owner = if mode == Mode::Key { buyer.clone() } else { me.to_ascii_lowercase() };
    let company = (mode == Mode::Company).then_some(buyer.as_str());
    keep(s.data(), writer, now, &owner, mode, company, &mut order, Some(bytes))?;
    Ok(Done { address: owner, label: label_of(&pk.case_id), mode, order })
}

fn mode_of(pk: &k3::pack::Pack, s: &Session, me: &str, buyer: &str) -> R<Mode> {
    if held(s, me, buyer)? {
        return Ok(Mode::Key);
    }
    if by_call(pk) {
        return Ok(if ours(s.data(), me, buyer)? { Mode::Company } else { Mode::ByCall });
    }
    Err(Fault::new("E_P4_FOREIGN_KEY", SAY_FOREIGN_KEY))
}

fn held(s: &Session, me: &str, buyer: &str) -> R<bool> {
    Ok(eravon_vault::keyset::buyer(s, me)?.mine.iter().any(|k| k.eq_ignore_ascii_case(buyer)))
}


fn ours(data: &str, me: &str, buyer: &str) -> R<bool> {
    Ok(eravon_vault::store::seen_of(data, me)?.iter().skip(1).any(|k| k.eq_ignore_ascii_case(buyer)))
}


fn by_call(pk: &k3::pack::Pack) -> bool {
    pk.bound.by_call()
}


struct Wire {
    t: a2::transport::Auto,
    table: a2::endpoints::Table,
}

fn wire(data: &str, me: &str) -> R<Wire> {
    let nodes = eravon_vault::store::endpoints(data, me)?;
    if nodes.is_empty() {
        return Err(Fault::new("E_P4_NO_ENDPOINT", "未配置节点"));
    }
    Ok(Wire { t: a2::transport::Auto::web_roots()?, table: a2::endpoints::Table::new(nodes.clone())? })
}

fn blank() -> Order {
    Order {
        case_id: String::new(),
        instance: String::new(),
        tx: String::new(),
        terms_hash: String::new(),
        amount: String::new(),
        token: String::new(),
        seller: String::new(),
        why: None,
        terms_saved: false,
        decimals: None,
    }
}

fn decimals_of(w: &Wire, token: &str) -> Option<u64> {
    if token.is_empty() {
        return None;
    }
    eravon_standard::chain::erc20_decimals(&w.t, &w.table, token)
}

fn found(data: &str, me: &str, buyer: &str, case_id: &str, instance: &str, chain_id: u64, logic: &str) -> Order {
    let mut o = Order { case_id: case_id.to_string(), instance: instance.to_ascii_lowercase(), ..blank() };
    let got = (|| {
        let w = wire(data, me)?;
        let sweep = a3::by_key(&w.t, &w.table, logic, a3::Side::Buyer, buyer, 0, a2::seam::Upto::Latest)?;
        let c = sweep.cases.iter().find(|c| c.case_id.eq_ignore_ascii_case(case_id)).ok_or_else(|| Fault::new("E_P4_NOT_OPENED", SAY_NOT_OPENED))?;
        let r = rebuilt(&w, &c.tx, chain_id, logic, case_id)?;
        let d = r.as_ref().and_then(|x| decimals_of(&w, &x.token));
        Ok((c.tx.clone(), r, d))
    })();
    match got {
        Ok((tx, r, d)) => {
            o.tx = tx;
            fill(&mut o, r);
            o.decimals = d;
        }
        Err(f) => o.why = Some(f),
    }
    o
}

fn rebuilt(w: &Wire, tx: &str, chain_id: u64, logic: &str, case_id: &str) -> R<Option<Box<a3::Rebuilt>>> {
    match a3::rebuild(&w.t, &w.table, tx, chain_id, logic, case_id)? {
        a3::Rebuild::Done(r) => Ok(Some(r)),
        a3::Rebuild::Untraced => Err(Fault::new("E_P4_UNTRACED", a3::SAY_UNTRACED)),
    }
}


fn fill(o: &mut Order, r: Option<Box<a3::Rebuilt>>) {
    if let Some(r) = r {
        o.terms_hash = r.terms_hash.clone();
        o.amount = r.amount.clone();
        o.token = r.token.clone();
        o.seller = r.seller.clone();
    }
}

fn archive_id(case_id: &str, _now: u64) -> String {
    record_id(case_id)
}


pub const LABEL_PREFIX: &str = "订单";

pub fn label_of(case_id: &str) -> String {
    let h = case_id.trim_start_matches("0x");
    format!("{LABEL_PREFIX} {}", &h[..h.len().min(8)])
}

pub fn record_id(case_id: &str) -> String {
    let h = case_id.trim_start_matches("0x");
    h[..h.len().min(16)].to_string()
}

#[allow(clippy::too_many_arguments)]
fn keep(data: &str, writer: &str, now: u64, owner: &str, mode: Mode, company: Option<&str>, o: &mut Order, pack: Option<&[u8]>) -> R<()> {
    let st = eravon_vault::store::open_write(data, writer, now)?;
    st.init_identity(owner, now)?;
    let id = archive_id(&o.case_id, now);
    let had = match st.get(owner, Box_::Packs, &id) {
        Ok(r) => r.parts.iter().any(|(n, _)| n == "pack.json"),
        Err(eravon_store::fault::Fault::Missing(_)) => false,
        Err(e) => return Err(e.into()),
    };
    o.terms_saved = pack.is_some() || had;
    let old = match st.get(owner, Box_::Packs, &id) {
        Ok(r) => r.fields,
        Err(eravon_store::fault::Fault::Missing(_)) => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    let mut fields = vec![
        ("caseId".to_string(), o.case_id.clone()),
        ("instance".to_string(), o.instance.clone()),
        ("mode".to_string(), mode.stored().to_string()),
        ("termsSaved".to_string(), if o.terms_saved { "yes" } else { "no" }.to_string()),
    ];
    if !o.tx.is_empty() {
        fields.push(("tx".to_string(), o.tx.clone()));
    }
    if let Some(c) = company {
        fields.push(("company".to_string(), c.to_string()));
    }
    if let Some(d) = o.decimals {
        fields.push((eravon_watch::units::FIELD.to_string(), d.to_string()));
    }
    let note = if !mode.note().is_empty() { mode.note() } else if !o.terms_saved { SAY_TERMS_MISSING } else { "" };
    if !note.is_empty() {
        fields.push(("note".to_string(), note.to_string()));
    }
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    if let Some(b) = pack {
        parts.push(("pack.json".into(), b.to_vec()));
    }
    if !o.terms_hash.is_empty() {
        let v = Out::O(vec![
            ("amount", Out::S(o.amount.clone())),
            ("caseId", Out::S(o.case_id.clone())),
            ("seller", Out::S(o.seller.clone())),
            ("termsHash", Out::S(o.terms_hash.clone())),
            ("token", Out::S(o.token.clone())),
            ("tx", Out::S(o.tx.clone())),
        ]);
        parts.push(("rebuilt.json".into(), eravon_literals::json::write(&v)));
    }
    let mut merged: Vec<(String, String)> = old.into_iter().filter(|(k, _)| !fields.iter().any(|(n, _)| n == k)).collect();
    merged.extend(fields);
    st.put(owner, Kind::OrderFile, &id, &merged, &parts, now)?;
    Ok(())
}
