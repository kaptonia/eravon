use eravon_standard as a1;
use a1::company::Item;
use a1::proposal::Original;

pub type Why = (String, String);

#[derive(Clone, Debug)]
pub struct Company {
    pub address: String,
    pub payout: String,
    pub fund: String,
    pub bound: u64,
    pub agreement: &'static str,
    pub time_on: bool,
}

pub fn of(data: &str, me: &str, route: &Route, registries: &[String], now: u64) -> Result<Option<Company>, Why> {
    let ident = eravon_vault::store::read_identity(data, me).unwrap_or_default();
    if !ident.company_on() || ident.company.is_empty() {
        return Ok(None);
    }
    let _ = (registries, now);
    let (t, table) = route.clone()?;
    match a1::company::open(&t, &table, &ident.company) {
        Err(f) => Err((f.name(), f.detail())),
        Ok(Err(s)) => Err(("E_GOODS_COMPANY_SHUT".into(), format!("公司模式不可开:{} 处 {}({})", s.at, s.kind.word(), s.why))),
        Ok(Ok((v, _))) => {
            let address = ident.company.to_ascii_lowercase();
            Ok(Some(Company { payout: payout_of(&address, &v), fund: v.wallet.clone(), address, bound: v.calendar_bound, agreement: v.agreement, time_on: ident.time_check_on() }))
        }
    }
}

pub type Route = Result<(eravon_chain::transport::Auto, eravon_chain::endpoints::Table), Why>;

fn payout_of(_address: &str, v: &a1::company::Views) -> String {
    v.ledger.clone()
}


pub fn unsigned(signer: &crate::sign::Signer, terms_bytes: &[u8]) -> Result<Vec<String>, Why> {
    let _ = signer;
    let t = eravon_terms::doc::terms(terms_bytes).map_err(|f| (f.token.name().to_string(), "无法读取条款".to_string()))?;
    Ok(vec![String::new(); t.judges.len() + t.guilds.len()])
}


#[derive(Clone, Debug)]
pub struct Exported {
    pub proposal: Vec<u8>,
    pub docs: Vec<Original>,
    pub digests: Vec<String>,
    pub time: a1::company::Verdict,
    pub face: Vec<String>,
}

fn gate_on(on: bool) -> bool {
    on
}


pub const E_GOODS_GUILD_OPEN: &str = "E_GOODS_GUILD_OPEN";

pub fn export(co: &Company, route: &Route, terms_bytes: &[u8], listing: &[u8], now: u64, quotes: &[String]) -> Result<Exported, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let t = eravon_terms::doc::terms(terms_bytes).map_err(|f| (f.token.name().to_string(), "无法读取条款".to_string()))?;
    if !t.guilds.is_empty() && !t.buyer_may_open {
        return Err((E_GOODS_GUILD_OPEN.into(), "行会候选的单,公司不经提案开案:先在付款方式里打开「允许买家自付 gas 开案」".into()));
    }
    let mut items: Vec<Item> = vec![Item::Window { name: "t1", who: String::new(), secs: t.delivery_window }];
    items.push(a1::company::resign_lead_item());
    if !t.buyer_may_open {
        items.push(Item::Window { name: "openWindow", who: String::new(), secs: t.open_window });
    }
    let who: Vec<String> = t.judges.iter().map(|j| j.judge.clone()).chain(t.guilds.iter().map(|g| g.address.clone())).collect();
    for (addr, q) in who.iter().zip(quotes) {
        let q = eravon_cands::quote::decode(q).map_err(|why| ("E_GOODS_QUOTE".to_string(), format!("候选 {addr} 的报价读不出:{why}")))?;
        items.push(Item::Window { name: "t3", who: addr.clone(), secs: q.ruling_window });
    }
    let time = a1::company::time_check(co.bound, now, &items);
    if !time.enough && gate_on(co.time_on) {
        return Err(("E_GOODS_TIME_SHORT".into(), format!("期限不足(日历上界 {} 秒):{}", time.bound, time.say_short())));
    }
    let listing_doc = Original { kind: eravon_literals::literals::DOC_LISTING.into(), name: crate::goods::PART_LISTING.into(), bytes: listing.to_vec() };
    let total = (t.judges.len() + t.guilds.len()) as u64;
    let (post, board) = a1::proposal::board_post(&eravon_terms::doc::terms_hash(terms_bytes), listing).map_err(|f| (f.name(), f.detail()))?;
    let mut calls = vec![post];
    let mut originals = vec![board];
    let mut digests = Vec::new();
    for c in 0..total {
        let d = eravon_terms::derive::offer_digest_at(&t, c).map_err(|f| (f.token.name().to_string(), "无法生成订单要约".to_string()))?;
        let (pc, docs) = a1::proposal::consent(&t.logic, &t.seller_key, &d, listing_doc.clone()).map_err(|f| (f.name(), f.detail()))?;
        calls.push(pc);
        originals.extend(docs);
        digests.push(a1::order::hex_of(&d));
    }
    senders(route, &co.address, &calls)?;
    let mut face = vec![a1::proposal::face_consent(t.expiry), a1::proposal::FACE_OFFER.to_string(), FACE_POST.to_string()];
    if !time.enough {
        face.push(format!("时间核对关着,照导出;期限对日历上界 {} 秒不够:{}。", time.bound, time.say_short()));
    }
    let (p, docs) = a1::proposal::assemble(t.chain_id, calls, originals, vec!["eravon".into(), "offer".into()], face.join("\n"), &eravon_galeed::hash::sha256)
        .map_err(|f| (f.name(), f.detail()))?;
    let proposal = p.bytes().map_err(|f| (f.name(), f.detail()))?;
    Ok(Exported { proposal, docs, digests, time, face })
}

pub const FACE_POST: &str = "清单随本提案由本公司上公告板:上板与要约登记同成或同不成。";

pub fn revoke_export(route: &Route, chain_id: u64, logic: &str, company: &str, terms_hash: &[u8; 32]) -> Result<(Vec<u8>, Vec<Original>), Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let call = a1::call::revoke(logic, company, terms_hash).and_then(|c| a1::proposal::of_call(&c)).map_err(|f| (f.name(), f.detail()))?;
    senders(route, company, std::slice::from_ref(&call))?;
    let abi = a1::proposal::abi_doc(&[a1::sol::REVOKE], a1::proposal::LOGIC_ABI_NAME).map_err(|f| (f.name(), f.detail()))?;
    let (p, docs) = a1::proposal::assemble(chain_id, vec![call], vec![abi], vec!["eravon".into(), "revoke".into()], FACE_REVOKE.to_string(), &eravon_galeed::hash::sha256)
        .map_err(|f| (f.name(), f.detail()))?;
    Ok((p.bytes().map_err(|f| (f.name(), f.detail()))?, docs))
}

fn senders(route: &Route, company: &str, calls: &[a1::proposal::PCall]) -> Result<(), Why> {
    let (t, table) = route.clone()?;
    a1::gate::proposal_senders(&t, &table, company, calls).map_err(|f| (f.name(), f.detail()))
}

pub const FACE_REVOKE: &str = "执行即撤下这一版:之后的付款开不了案,买家可取回;已开的案照旧。";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reg {
    pub digest: String,
    pub state: String,
    pub why: String,
}

pub fn registered(route: &Route, company: &str, logic: &str, digests: &[String], now: u64) -> Result<(Vec<Reg>, bool), Why> {
    let _ = now;
    let (t, table) = route.clone()?;
    let mut rows = Vec::new();
    let parsed = digests.iter().map(|d| a1::order::b32_of("要约摘要", d).map_err(|f| (f.name(), f.detail()))).collect::<Result<Vec<_>, _>>()?;
    a1::consent::ahead_registered(&t, &table, logic, company, &parsed);
    for (d, digest) in digests.iter().zip(&parsed) {
        let r = a1::consent::registered(&t, &table, logic, company, digest);
        rows.push(Reg { digest: d.clone(), state: r.state.word().to_string(), why: r.why });
    }
    let clear = clears(&rows);
    Ok((rows, clear))
}

fn clears(rows: &[Reg]) -> bool {
    !rows.is_empty() && rows.iter().all(|r| r.state == "held")
}

