use eravon_link as k3;
use eravon_literals::json::Out;
use eravon_standard as a1;
use eravon_store::seam::Box_;
use eravon_terms as k2;

use crate::check::{self, Chain};
use crate::fault::{Fault, R};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Registered,
    Direct,
}

impl Form {
    pub const fn name(self) -> &'static str {
        match self {
            Form::Registered => "registered",
            Form::Direct => "direct",
        }
    }

    pub fn of(s: &str) -> Option<Form> {
        match s {
            "registered" => Some(Form::Registered),
            "direct" => Some(Form::Direct),
            _ => None,
        }
    }
}

pub fn default_form(q: &k2::quote::Quote, p: &k3::listing::Listing, candidate: usize) -> Form {
    if direct_allowed(p, candidate) && check::feeds_of(q).is_empty() { Form::Direct } else { Form::Registered }
}


pub fn direct_allowed(p: &k3::listing::Listing, candidate: usize) -> bool {
    p.order.buyer_may_open && candidate < p.judges.len()
}

fn payout_of(co: &eravon_cli::company::Company) -> String {
    co.views.wallet.clone()
}


fn time_gate(on: bool) -> bool {
    on
}


fn opening(deploy: Option<&a1::call::Call>, init: &a1::call::Call) -> R<Vec<a1::call::Call>> {
    Ok(deploy.into_iter().chain(std::iter::once(init)).cloned().collect())
}


pub struct CoIn<'a> {
    pub c: &'a Chain,
    pub link: &'a [u8],
    pub listing: &'a [u8],
    pub p: &'a k3::listing::Listing,
    pub candidate: usize,
    pub form: Form,
    pub co: &'a eravon_cli::company::Company,
    pub time_on: bool,
    pub path: &'a str,
    pub confirmed: bool,
    pub attachments: &'a [String],
}

pub struct Exported {
    pub path: String,
    pub rows: Vec<(&'static str, Out)>,
    pub instance: String,
    pub case_id: String,
    pub form: Form,
    pub receipt: Vec<u8>,
    pub pack: Vec<u8>,
    pub id: String,
}

pub fn export(i: &CoIn<'_>, writer: &str) -> R<Exported> {
    crate::mark::mark(crate::mark::FEATURE);
    let (c, p, n) = (i.c, i.p, i.candidate);
    let seq = eravon_cli::buyer::pool_seq_on(&c.w, p, n as u64)?;
    let payout = payout_of(i.co);
    let (e_at, open_by) = eravon_cli::shared::open_by_by_call(p.order.expiry, p.order.open_window, c.now, i.co.views.calendar_bound)?;
    let sw = eravon_watch::Wire { t: &c.w.t, table: &c.w.table, chain_id: c.w.chain_id, logic: &c.w.logic, registries: &c.w.registries };
    let prev = eravon_cli::buyer::series_prev(&c.data, &c.buyer, &i.co.address, p, &eravon_cli::buyer::live_on(&c.w, c.now))?;
    let d = eravon_cli::buyer::co_draft(&eravon_cli::buyer::CoOrder {
        prev: &prev,
        w: &sw,
        p,
        link: i.link,
        listing: i.listing,
        candidate: n as u64,
        seq,
        buyer: &i.co.address,
        payout: &payout,
        open_by,
        chain: c.w.chain_id,
        attachments: i.attachments,
    })
    .map_err(|f| match f {
        eravon_cli::fault::Fault::Down { token, why, .. } => Fault::new(token, why),
        other => Fault::from(other),
    })?;
    let mut e = match i.form {
        Form::Registered => eravon_cli::buyer::co_registered(&d, p, &i.co.views.wallet)?,
        Form::Direct if !direct_allowed(p, n) => {
            return Err(Fault::new("E_P3_COMPANY_FORM", "直接式只在条款许买家开案、且是指名判官时可用;改登记式"));
        }
        Form::Direct => direct(i, &d)?,
    };
    e.items.push(a1::company::Item::Deadline { name: "openBy", who: String::new(), at: e_at });
    if let Some(say) = short_say(c, p, n, &i.co.views.wallet)? {
        e.face.push(say);
    }
    let rows = eravon_cli::company::write(c.now, &c.w, i.co, i.path, time_gate(i.time_on), i.confirmed, e).map_err(|f| match eravon_cli::company::time_asked(&f) {
        Some(say) => Fault::new(eravon_cli::company::TIME_ASK, say),
        None => Fault::from(f),
    })?;
    let id = d.case_id[2..18].to_string();
    let store = eravon_vault::store::open_write(&c.data, writer, c.now)?;
    eravon_cli::buyer::co_keep(&store, &c.buyer, &id, &d, &i.co.address, i.form.name(), p.terms_text.as_bytes(), c.now)?;
    Ok(Exported { path: i.path.to_string(), rows, instance: d.instance, case_id: d.case_id, form: i.form, receipt: d.receipt, pack: d.pack, id })
}

fn direct(i: &CoIn<'_>, d: &eravon_cli::buyer::CoDraft) -> R<eravon_cli::company::Export> {
    let (c, p, n) = (i.c, i.p, i.candidate);
    let (pay, wallet_abi) = a1::proposal::wallet_pay(&i.co.views.wallet, &p.token, &d.instance, &p.order.amount.to_be())?;
    let quote = check::act(c, &d.order).map_err(|r| Fault::new(r.name, format!("{}:{}", r.gate, r.why)))?.quote;
    let salt = a1::order::b32_of("案号", &d.case_id)?;
    let ss = a1::order::bytes_of("卖家签名", p.candidate_sig(n))?;
    let init = a1::call::init(&d.order, &d.instance, &quote, &[], &ss, &[], &[])?;
    let deploy = if a1::chain::has_code(&c.w.t, &c.w.table, &d.instance)? { None } else { Some(a1::call::deploy_instance(&salt, &p.logic)?) };
    let mut calls = vec![pay];
    for x in opening(deploy.as_ref(), &init)? {
        calls.push(a1::proposal::of_call(&x)?);
    }
    let mut originals = vec![wallet_abi, a1::proposal::abi_doc(&[a1::sol::INIT], a1::proposal::INSTANCE_ABI_NAME)?];
    originals.push(a1::proposal::Original { kind: eravon_literals::literals::DOC_RECEIPT.into(), name: "receipt.json".into(), bytes: d.receipt.clone() });
    originals.extend(d.docs.iter().cloned());
    Ok(eravon_cli::company::Export {
        verb: "order-direct",
        calls,
        originals,
        items: d.items.clone(),
        face: vec![format!("预算地址 {}:付款、部署(实例还没有码时)与 init 在同一份提案里全成或全不成;部署与 init 都由公司直接调。", d.instance)],
        deal: eravon_cli::company::Deal::Unopened(d.order.offer.seller_key.clone()),
    })
}

fn short_say(c: &Chain, p: &k3::listing::Listing, n: usize, wallet: &str) -> R<Option<String>> {
    let have = k2::u256::U256::from_be(&a1::chain::erc20_balance(&c.w.t, &c.w.table, &p.token, wallet)?.value);
    let lf = check::quote_of(c, p, n).ok().and_then(|q| k2::fee::fee(&q, &p.order.amount).ok()).and_then(|f| f.lf);
    let (need, _) = match &lf {
        Some(l) => p.order.amount.overflowing_add(l),
        None => (p.order.amount, false),
    };
    Ok(check::gap(&need, &have).map(|g| {
        let what = if lf.is_some() { "本单的价加争议要垫的诉讼费 LF" } else { "本单的价" };
        format!("公司钱包 {wallet} 余额不足{what},差 {}:先注资再买。", a1::precheck::word(&c.w.t, &c.w.table, &p.token, &g.to_be()))
    }))
}

pub fn open_plan(c: &Chain, s: &eravon_vault::session::Session, id: &str) -> R<(a1::proposal::Plan, String)> {
    crate::mark::mark(crate::mark::FEATURE);
    let store = eravon_vault::store::open_read(s.data())?;
    let pk = k3::pack::pack(&store.part(&c.buyer, Box_::Packs, id, "pack.json")?).map_err(|e| Fault::new(e.token.name(), "无法读取订单文件"))?;
    let p = &pk.listing;
    a1::chain::listing_payable(&p.logic).map_err(|f| Fault::new(f.name(), f.detail()))?;
    let n = pk.receipt.candidate as usize;
    let o = eravon_cli::shared::order_of(p.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev)?;
    let sig_seller = a1::order::bytes_of("要约签名", p.candidate_sig(n)).unwrap_or_default();
    Ok(eravon_cli::buyer::open_plan(&c.w, &o, &sig_seller, n >= p.judges.len(), c.now, &eravon_cli::buyer::seat_party(s.data(), &c.buyer)?)?)
}
