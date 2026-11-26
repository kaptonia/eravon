use eravon_dispute::view::CaseView;
use eravon_link as k3;
use eravon_standard as a1;

use eravon_literals::json::Out;
use eravon_orders::table::Row;
use eravon_store::record::Kind;
use eravon_store::seam::Box_;

use super::said::Said;
use crate::shared::Wire;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Key,
    ByCall,
    Company,
}

impl Access {
    pub fn of(s: &str) -> Access {
        match s {
            "byCall" => Access::ByCall,
            _ => Access::Key,
        }
    }

    pub const fn note(self) -> &'static str {
        match self {
            Access::Key => "",
            Access::ByCall => "此订单以调用同意",
            Access::Company => "公司订单:操作将导出提案",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Case {
    pub instance: String,
    pub case_id: String,
    pub buyer_key: String,
    pub token: String,
    pub seller: String,
    pub access: Access,
    pub pack: Option<Vec<u8>>,
    pub chain: Option<CaseView>,
    pub handed: bool,
}

impl Case {
    pub fn past(&self) -> bool {
        self.chain.as_ref().is_some_and(|v| v.past)
            || (!eravon_literals::constants::past_rows().is_empty()
                && self.pack.as_ref().and_then(|b| k3::pack::pack(b).ok()).is_some_and(|p| eravon_literals::constants::generation_of_logic(&p.listing.logic).is_some_and(|g| g.is_past())))
    }
}

pub const PAST_LACKS_ACTS: &[&str] = &["openCase"];

pub fn unopened(c: &Case) -> Option<a1::order::Order> {
    if c.chain.as_ref().is_some_and(|v| v.phase != 0) {
        return None;
    }
    let pk = c.pack.as_ref().and_then(|b| k3::pack::pack(b).ok())?;
    crate::shared::order_of(pk.listing.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev).ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    No,
    Lapsed,
    Revoked,
}

pub fn closed(w: &Wire, c: &Case, now: u64) -> Result<Option<Take>, eravon_standard::fault::Fault> {
    let Some(o) = unopened(c) else { return Ok(None) };
    if a1::precheck::in_window(o.offer.expiry.min(o.open_by), now).is_err() {
        return Ok(Some(Take::Lapsed));
    }
    let logic = c.pack.as_ref().and_then(|b| k3::pack::pack(b).ok()).map_or_else(|| w.logic.clone(), |p| p.listing.logic);
    Ok(a1::chain::revoked(&w.t, &w.table, &logic, &o.offer.seller_key, &o.offer.terms_hash)?.value.then_some(Take::Revoked))
}

pub fn take_from(how: Option<Take>, funded: bool) -> Take {
    match how {
        Some(t) if funded => t,
        _ => Take::No,
    }
}

pub fn wallet_uri(chain_id: u64, to: &str, data: &[u8], value_wei: &str) -> String {
    let word = |b: &[u8]| -> String { b.iter().map(|x| format!("{x:02x}")).collect() };
    let addr = |w: &[u8]| format!("0x{}", word(&w[12..32]));
    let uint = |w: &[u8]| eravon_terms::u256::U256::from_be(w.try_into().unwrap_or(&[0u8; 32])).to_dec();
    let to = to.to_ascii_lowercase();
    if data.len() == 4 + 64 {
        let (sel, a, b) = (&data[..4], &data[4..36], &data[36..68]);
        let name = match sel {
            [0xa9, 0x05, 0x9c, 0xbb] => Some("transfer"),
            [0x09, 0x5e, 0xa7, 0xb3] => Some("approve"),
            _ => None,
        };
        if let Some(n) = name {
            return format!("ethereum:{to}@{chain_id}/{n}?address={}&uint256={}", addr(a), uint(b));
        }
    }
    if data.is_empty() {
        return format!("ethereum:{to}@{chain_id}?value={value_wei}");
    }
    format!("ethereum:{to}@{chain_id}?value={value_wei}&data=0x{}", word(data))
}

pub fn company_mode(data: &str, addr: &str) -> Option<(String, bool)> {
    let ident = eravon_vault::store::read_identity(data, addr).ok()?;
    (ident.company_on() && !ident.company.is_empty()).then(|| (ident.company.to_ascii_lowercase(), ident.time_check_on()))
}

pub fn acts_for(company: &str, recorded: &str, acting: Option<String>) -> bool {
    acting.as_deref().unwrap_or(recorded).eq_ignore_ascii_case(company)
}

pub fn acting_on(w: &Wire, instance: &str, key: &str) -> Option<String> {
    let g = eravon_standard::chain::case_generation(&w.t, &w.table, instance, &w.logic, None).ok()?;
    eravon_standard::chain::acting_key(&w.t, &w.table, key, g.is_past()).ok()
}

pub const E_READONLY: &str = "E_P6_READONLY";

pub fn key_party(r: &BRow) -> Result<(), &'static str> {
    if r.access == Access::Key { Ok(()) } else { Err(r.access.note()) }
}

#[derive(Clone, Debug)]
pub struct BRow {
    pub row: Row,
    pub owner: String,
    pub record: String,
    pub access: Access,
    pub pack: Option<Vec<u8>>,
    pub seller: String,
    pub product: String,
    pub lf_gap: Option<Result<String, Said>>,
    pub budget: Option<Result<Budget, Said>>,
    pub receipt: Option<String>,
}

pub fn receipt_of(pack: Option<&[u8]>) -> Option<String> {
    let pk = eravon_link::pack::pack(pack?).ok()?;
    Some(eravon_link::transport::encode_receipt(&pk.receipt.bytes))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    pub have: String,
    pub need: String,
    pub owe: String,
    pub until: u64,
    pub lapsed: bool,
    pub revoked: bool,
    pub emptied: Option<Take>,
    pub buyer_may_open: bool,
    pub payout: String,
    pub receipt: String,
    pub pay_uri: String,
    pub deposit_short: bool,
}

impl Budget {
    pub fn paid(&self) -> bool {
        self.owe == "0"
    }
}

pub fn pre_open(r: &BRow) -> bool {
    use eravon_orders::status::Word;
    r.row.chain.as_ref().is_none_or(|v| v.phase == 0) && matches!(r.row.status.as_ref().map(|s| s.word), Ok(Word::AwaitDeploy | Word::AwaitPay | Word::Revoked))
}

pub fn budget_of(w: &Wire, r: &BRow, now: Option<u64>) -> Option<Result<Budget, Said>> {
    use eravon_terms::u256::U256;
    if !pre_open(r) {
        return None;
    }
    let pk = r.pack.as_ref().and_then(|b| eravon_link::pack::pack(b).ok())?;
    Some((|| {
        let have = U256::from_be(&eravon_standard::chain::erc20_balance(&w.t, &w.table, &r.row.token, &r.row.instance).map_err(|e| Said::bad(&e.name(), e.detail()))?.value);
        let need = pk.listing.order.amount;
        let owe = if have.cmp_u(&need) == std::cmp::Ordering::Less { need.overflowing_sub(&have).0 } else { U256::ZERO };
        let short = !owe.is_zero() && deposit_short(w, r);
        let pay_uri = if owe.is_zero() || short {
            String::new()
        } else {
            let call = eravon_standard::call::erc20_transfer(&r.row.token, &r.row.instance, &owe.to_be()).map_err(|e| Said::bad(&e.name(), e.detail()))?;
            wallet_uri(w.chain_id, &call.to, &call.data, "0")
        };
        let until = eravon_orders::table::open_until(&pk);
        let how = match now {
            Some(n) => closed(w, &r.case(), n).map_err(|f| Said::bad(&f.name(), f.detail()))?,
            None => None,
        };
        let take = take_from(how, !have.is_zero());
        let unlisted = matches!(r.row.status.as_ref().map(|s| s.word), Ok(eravon_orders::status::Word::Revoked));
        let revoked = take == Take::Revoked || (now.is_none() && unlisted && !have.is_zero());
        let emptied = if have.is_zero() { how.or_else(|| unlisted.then_some(Take::Revoked)) } else { None };
        Ok(Budget {
            pay_uri,
            have: have.to_dec(),
            need: need.to_dec(),
            owe: owe.to_dec(),
            until,
            lapsed: take == Take::Lapsed || revoked,
            revoked,
            emptied,
            buyer_may_open: pk.listing.order.buyer_may_open,
            payout: pk.receipt.buyer_payout.to_ascii_lowercase(),
            receipt: receipt_of(r.pack.as_deref()).unwrap_or_default(),
            deposit_short: short,
        })
    })())
}

pub fn deposit_short(w: &Wire, r: &BRow) -> bool {
    let Some(pk) = r.pack.as_ref().and_then(|b| eravon_link::pack::pack(b).ok()) else { return false };
    let Ok(o) = crate::shared::order_of(pk.listing.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev) else { return false };
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    matches!(crate::buyer::pay_gate(&sw, &o), crate::buyer::Gate::DepositShort(_))
}

impl BRow {
    pub fn case(&self) -> Case {
        Case {
            instance: self.row.instance.clone(),
            case_id: self.row.case_id.clone(),
            buyer_key: self.row.buyer.to_ascii_lowercase(),
            token: self.row.token.clone(),
            seller: self.seller.clone(),
            access: self.access,
            pack: self.pack.clone(),
            chain: self.row.chain.clone(),
            handed: self.row.handed,
        }
    }
}

pub fn key_set(s: &eravon_vault::session::Session, me: &str) -> Result<(Vec<String>, Vec<String>), Said> {
    eravon_vault::keyset::buyer(s, me).map(|b| (b.owners, b.keys)).map_err(|e| Said::bad(e.name(), e.detail()))
}

pub fn company_row(company: Option<&str>, buyer: &str, acting: impl FnOnce() -> Option<String>) -> bool {
    company.is_some_and(|c| acts_for(c, buyer, acting()))
}

pub fn column_holder(co: &crate::company::Company) -> &str {
    &co.views.wallet
}

fn lf_gap(w: &Wire, co: &crate::company::Company, row: &Row, holder: for<'c> fn(&'c crate::company::Company) -> &'c str) -> Result<String, Said> {
    use eravon_terms::u256::U256;
    let (_, _, need) = crate::after_company::refund_need(w, &row.instance, &row.case_id).map_err(|(n, d)| Said::bad(&n, d))?;
    let have = U256::from_be(&eravon_standard::chain::erc20_balance(&w.t, &w.table, &row.token, holder(co)).map_err(|e| Said::bad(&e.name(), e.detail()))?.value);
    Ok(if need.cmp_u(&have) == std::cmp::Ordering::Greater { need.overflowing_sub(&have).0.to_dec() } else { "0".to_string() })
}

pub fn company_of(w: &Wire, data: &str, me: &str, shut: &dyn Fn(String, String, String) -> String) -> Option<Result<crate::company::Company, Said>> {
    let (company, _) = company_mode(data, me)?;
    Some(match eravon_standard::company::open(&w.t, &w.table, &company) {
        Err(f) => Err(Said::bad(&f.name(), f.detail())),
        Ok(Err(sh)) => Err(Said::bad("E_P5_COMPANY_SHUT", shut(sh.at.to_string(), sh.kind.word().to_string(), sh.why.to_string()))),
        Ok(Ok((views, _))) => Ok(crate::company::Company { address: company, views }),
    })
}

fn local(data: &str, owners: &[String], instance: &str) -> Option<(String, String, Access, Option<Vec<u8>>, String)> {
    let st = eravon_vault::store::open_read(data).ok()?;
    for o in owners {
        let Ok(list) = st.list(o, Box_::Packs) else { continue };
        if let Some(r) = list.into_iter().find(|r| r.kind == Kind::OrderFile && r.field("instance").is_some_and(|x| x.eq_ignore_ascii_case(instance))) {
            let pack = st.part(o, Box_::Packs, &r.id, "pack.json").ok();
            let seller = st
                .part(o, Box_::Packs, &r.id, "rebuilt.json")
                .ok()
                .and_then(|b| eravon_literals::json::read(&b).ok().and_then(|v| v.get("seller").and_then(|x| x.as_str()).map(str::to_string)))
                .unwrap_or_default();
            return Some((o.clone(), r.id.clone(), Access::of(r.field("mode").unwrap_or("key")), pack, seller));
        }
    }
    None
}

fn fill_from_pack(row: &mut Row, pack: Option<&[u8]>) {
    if !row.token.is_empty() && !row.amount.is_empty() && !row.buyer.is_empty() {
        return;
    }
    let Some(pk) = pack.and_then(|b| eravon_link::pack::pack(b).ok()) else { return };
    if row.buyer.is_empty() {
        row.buyer = pk.receipt.buyer_key.to_ascii_lowercase();
    }
    if row.token.is_empty() {
        row.token = pk.listing.token.to_ascii_lowercase();
    }
    if row.amount.is_empty() {
        row.amount = pk.listing.order.amount.to_dec();
    }
}

pub fn product_and_seller(data: &str, owners: &[String], instance: &str, private: &str) -> (String, String) {
    let Some((_, _, _, pack, rebuilt_seller)) = local(data, owners, instance) else { return Default::default() };
    let (product, pack_seller) = facts(pack.as_deref(), private);
    (product, if pack_seller.is_empty() { rebuilt_seller } else { pack_seller })
}

pub fn product_held(pack: Option<&[u8]>) -> bool {
    let Some(pk) = pack.and_then(|b| eravon_link::pack::pack(b).ok()) else { return false };
    let held = pk.attachments.iter().map(|a| a.text_md.clone()).find(|t| eravon_goods::terms::goods_of(&pk.listing.terms.block, Some(t)).is_some());
    eravon_goods::terms::goods_of(&pk.listing.terms.block, held.as_deref()).is_some()
}

fn facts(pack: Option<&[u8]>, private: &str) -> (String, String) {
    let Some(pk) = pack.and_then(|b| eravon_link::pack::pack(b).ok()) else { return Default::default() };
    let held = pk.attachments.iter().map(|a| a.text_md.clone()).find(|t| eravon_goods::terms::goods_of(&pk.listing.terms.block, Some(t)).is_some());
    let title = match eravon_goods::terms::goods_of(&pk.listing.terms.block, held.as_deref()) {
        Some(text) => eravon_goods::terms::name_of(&text),
        None => private.to_string(),
    };
    (title, pk.listing.order.seller_key.clone())
}

pub struct Page<'a> {
    pub tokens: &'a [eravon_store::settings::TokenRow],
    pub private_goods: &'a str,
    pub company_shut: &'a dyn Fn(String, String, String) -> String,
    pub company_row: fn(Option<&str>, &str, &dyn Fn() -> Option<String>) -> bool,
    pub column_holder: for<'c> fn(&'c crate::company::Company) -> &'c str,
}

pub struct Table {
    pub rows: Vec<BRow>,
    pub unscanned: Option<Said>,
}

fn sw(w: &Wire) -> eravon_watch::Wire<'_> {
    eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries }
}

pub fn rows_of(s: &eravon_vault::session::Session, data: &str, me: &str, w: &Wire, p: &Page, owners_out: &mut Vec<String>) -> Result<Table, Said> {
    rows_for(s, data, me, w, p, owners_out, None)
}

pub fn row_one(s: &eravon_vault::session::Session, data: &str, me: &str, w: &Wire, p: &Page, owners_out: &mut Vec<String>, instance: &str) -> Result<Table, Said> {
    rows_for(s, data, me, w, p, owners_out, Some(instance))
}

fn rows_for(s: &eravon_vault::session::Session, data: &str, me: &str, w: &Wire, p: &Page, owners_out: &mut Vec<String>, only: Option<&str>) -> Result<Table, Said> {
    let (owners, keys) = key_set(s, me)?;
    *owners_out = owners.clone();
    let t = eravon_orders::table::table_for(data, &owners, &keys, eravon_scan::scan::Side::Buyer, &sw(w), only).map_err(|(n, d)| Said::bad(&n, d))?;
    let unscanned = t.unscanned.as_ref().map(|(n, d)| Said::bad(n, d.clone()));
    let co = company_of(w, data, me, p.company_shut);
    let co_addr = eravon_vault::store::parties_of(data, me).map_err(|e| Said::bad(e.name(), e.detail()))?.into_iter().nth(1);
    let now = std::cell::OnceCell::new();
    let rows: Vec<BRow> = t.rows
        .into_iter()
        .map(|mut row| {
            let l = local(data, &owners, &row.instance);
            let (owner, record, access, pack, rebuilt_seller) = l.unwrap_or((me.to_ascii_lowercase(), String::new(), Access::Key, None, String::new()));
            fill_from_pack(&mut row, pack.as_deref());
            if row.decimals.is_none() && eravon_vault::tokens::find(p.tokens, eravon_vault::tokens::chain(), &row.token).is_none() && !row.token.is_empty() {
                row.decimals = eravon_standard::chain::side_decimals(&w.t, &w.table, &row.token);
            }
            let (product, pack_seller) = facts(pack.as_deref(), p.private_goods);
            let seller = if pack_seller.is_empty() { rebuilt_seller } else { pack_seller };
            let ours = (p.company_row)(co_addr.as_deref(), &row.buyer, &|| acting_on(w, &row.instance, &row.buyer));
            let access = if ours { Access::Company } else { access };
            let opened = row.chain.as_ref().is_some_and(|v| v.phase != 0);
            let lf_gap = match (&co, ours && opened) {
                (Some(Ok(c)), true) => Some(lf_gap(w, c, &row, p.column_holder)),
                (Some(Err(s)), true) => Some(Err(s.clone())),
                _ => None,
            };
            let receipt = receipt_of(pack.as_deref());
            BRow { row, owner, record, access, pack, seller, product, lf_gap, budget: None, receipt }
        })
        .collect();
    let mut rows = rows;
    {
        let sw = sw(w);
        let mut again: Vec<(String, &mut eravon_orders::table::Row)> = rows.iter_mut().filter(|r| pre_open(r)).map(|r| (r.owner.clone(), &mut r.row)).collect();
        let _ = eravon_orders::table::reread_many(data, eravon_scan::scan::Side::Buyer, &sw, &mut again);
    }
    let at = rows.iter().any(pre_open).then(|| *now.get_or_init(|| eravon_standard::precheck::chain_now(&w.t, &w.table).ok())).flatten();
    eravon_chain::read::together(&w.t, &w.table, || {
        let pre: Vec<&BRow> = rows.iter().filter(|r| pre_open(r)).collect();
        a1::chain::ahead_erc20_balances(&w.t, &w.table, &pre.iter().map(|r| (r.row.token.clone(), r.row.instance.clone())).collect::<Vec<_>>());
        if let Some(now) = at {
            let views: Vec<(String, Vec<u8>)> = pre
                .iter()
                .filter_map(|r| {
                    let c = r.case();
                    let o = unopened(&c).filter(|o| a1::precheck::in_window(o.offer.expiry.min(o.open_by), now).is_ok())?;
                    let logic = c.pack.as_ref().and_then(|b| k3::pack::pack(b).ok()).map_or_else(|| w.logic.clone(), |p| p.listing.logic);
                    let k = a1::order::addr_of("卖家钥", &o.offer.seller_key).ok()?;
                    Some((logic, a1::call::revoked(&k, &o.offer.terms_hash).ok()?))
                })
                .collect();
            a1::chain::ahead_views(&w.t, &w.table, &views);
        }
    });
    let wire = std::sync::Arc::new(w.clone());
    let budgets = eravon_chain::read::side_by_side(rows.to_vec(), move |r| budget_of(&wire, &r, at));
    for (r, b) in rows.iter_mut().zip(budgets) {
        r.budget = b;
        worded(r);
    }
    Ok(Table { rows, unscanned })
}

pub fn shown_of<'a>(rows: &'a [BRow], done: Option<bool>, q: &str, from: &str, to: &str) -> (Vec<&'a BRow>, Option<Said>) {
    let all: Vec<Row> = rows.iter().map(|x| x.row.clone()).collect();
    let tab: Vec<&BRow> = rows.iter().filter(|x| done.is_none_or(|d| eravon_orders::table::done_in(&all, &x.row) == d)).collect();
    if q.trim().is_empty() && from.trim().is_empty() && to.trim().is_empty() {
        return (tab, None);
    }
    let searched: Vec<Row> = rows
        .iter()
        .map(|x| {
            let mut row = x.row.clone();
            if row.product.is_empty() {
                row.product = x.product.clone();
            }
            row.label = x.seller.clone();
            row
        })
        .collect();
    match eravon_orders::table::search(&searched, q, from, to) {
        Ok(h) => (tab.into_iter().filter(|x| h.iter().any(|i| i.instance.eq_ignore_ascii_case(&x.row.instance))).collect(), None),
        Err((n, d)) => (tab, Some(Said::bad(&n, d))),
    }
}

pub fn acts(r: &BRow, read_only: fn(&BRow) -> bool) -> Vec<&'static str> {
    acts_under(r, eravon_literals::constants::past_switch(), read_only)
}

pub fn acts_under(r: &BRow, switch: bool, read_only: fn(&BRow) -> bool) -> Vec<&'static str> {
    if r.row.past && !switch {
        return Vec::new();
    }
    shown_acts(r, read_only).iter().copied().filter(|a| !past_lacks(r, read_only).contains(a)).collect()
}

pub fn past_lacks(r: &BRow, read_only: fn(&BRow) -> bool) -> Vec<&'static str> {
    if !r.row.past {
        return Vec::new();
    }
    shown_acts(r, read_only).iter().copied().filter(|a| PAST_LACKS_ACTS.contains(a)).collect()
}

pub fn shown_acts(r: &BRow, read_only: fn(&BRow) -> bool) -> &'static [&'static str] {
    use eravon_orders::status::Word;
    if read_only(r) {
        return &[];
    }
    match r.row.status.as_ref().map(|s| s.word) {
        Ok(Word::AwaitDeploy) | Ok(Word::AwaitPay) => match &r.budget {
            Some(Ok(b)) => budget_acts(b),
            _ => &["expire"],
        },
        Ok(Word::Revoked) if r.row.chain.as_ref().is_none_or(|v| v.phase == 0) => match &r.budget {
            Some(Ok(b)) if b.emptied.is_some() => &[],
            _ => &["expire"],
        },
        Ok(Word::AwaitAccept) | Ok(Word::AwaitRedraw) | Ok(Word::AwaitDelivery) => &["advance"],
        Ok(Word::AwaitConfirm) if r.access == Access::Company => &["confirm", "dispute", "advance"],
        Ok(Word::AwaitConfirm) => &["confirm", "confirmHand", "dispute", "advance"],
        Ok(Word::ConfirmHanded) => &["confirm", "advance"],
        Ok(Word::Disputed) => &["commit", "settle", "advance"],
        Ok(w) if w.done() => &["sweep"],
        _ => &[],
    }
}

pub fn budget_acts(b: &Budget) -> &'static [&'static str] {
    if b.emptied.is_some() {
        &[]
    } else if b.lapsed {
        &["expire"]
    } else if b.paid() && b.buyer_may_open {
        &["openCase", "cancel"]
    } else {
        &["cancel"]
    }
}

pub fn read_only(r: &BRow) -> bool {
    !matches!(r.access, Access::Key | Access::Company) || r.row.unrecorded()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pre {
    RevokedOpen,
    Ended(eravon_orders::status::Word),
    LapsedOpen,
    AwaitOpen,
    AwaitPay,
}

impl Pre {
    pub fn word(self) -> &'static str {
        match self {
            Pre::RevokedOpen => "revokedOpen",
            Pre::Ended(w) => w.word(),
            Pre::LapsedOpen => "lapsedOpen",
            Pre::AwaitOpen => "awaitOpen",
            Pre::AwaitPay => eravon_orders::status::Word::AwaitPay.word(),
        }
    }
}

pub fn pre_state(r: &BRow) -> Option<Pre> {
    match r.budget.as_ref()? {
        Ok(b) if b.revoked => Some(Pre::RevokedOpen),
        Ok(b) if b.emptied.is_some() => Some(Pre::Ended(ended_word(b))),
        Ok(b) if b.lapsed => Some(Pre::LapsedOpen),
        Ok(b) if b.paid() => Some(Pre::AwaitOpen),
        Ok(_) => Some(Pre::AwaitPay),
        Err(_) => None,
    }
}

pub fn worded(r: &mut BRow) {
    if let Some(Pre::Ended(word)) = pre_state(r) {
        r.row.status = Ok(eravon_orders::status::Status { word, reason: String::new() });
    }
}

pub fn ended_word(b: &Budget) -> eravon_orders::status::Word {
    use eravon_orders::status::Word;
    if b.emptied == Some(Take::Revoked) { Word::Revoked } else { Word::Expired }
}

pub fn row_out(r: &BRow, tokens: &[eravon_store::settings::TokenRow], read_only: fn(&BRow) -> bool) -> Out {
    let (word, say) = match &r.row.status {
        Ok(s) => (s.word.word().to_string(), s.word.say().to_string()),
        Err((n, d)) => (n.clone(), d.clone()),
    };
    let mut v = vec![
        ("access", Out::S(match r.access {
            Access::Key => "key",
            Access::ByCall => "byCall",
            Access::Company => "company",
        }
        .into())),
        ("acts", Out::A(acts(r, read_only).iter().map(|a| Out::S((*a).into())).collect())),
        ("amount", Out::S(r.row.amount.clone())),
        ("amountWord", Out::S(eravon_orders::table::amount_in(tokens, &eravon_vault::tokens::ZH, &r.row.amount, r.row.decimals, &r.row.token, eravon_vault::tokens::Form::Short))),
        ("buyer", Out::S(r.row.buyer.clone())),
        ("caseId", Out::S(r.row.case_id.clone())),
        ("ended", crate::deskops::orderpage::ended_out(&r.row)),
        ("instance", Out::S(r.row.instance.clone())),
        ("number", Out::S(r.row.number.clone())),
        ("past", Out::B(r.row.past)),
        ("placed", Out::N(r.row.placed)),
        ("preState", Out::S(pre_state(r).map_or("", Pre::word).to_string())),
        ("productHeld", Out::B(product_held(r.pack.as_deref()))),
        ("product", Out::S(r.product.clone())),
        ("record", Out::S(r.record.clone())),
        ("say", Out::S(say)),
        ("series", crate::deskops::orderpage::series_out(&r.row.series)),
        ("state", Out::S(word)),
    ];
    match &r.lf_gap {
        Some(Ok(g)) => v.push(("lfGap", Out::S(g.clone()))),
        Some(Err(s)) => v.push(("lfGap", s.out())),
        None => {}
    }
    match &r.budget {
        Some(Ok(b)) => v.push((
            "budget",
            Out::O(vec![
                ("buyerMayOpen", Out::B(b.buyer_may_open)),
                ("depositShort", Out::B(b.deposit_short)),
                ("have", Out::S(b.have.clone())),
                ("lapsed", Out::B(b.lapsed)),
                ("revoked", Out::B(b.revoked)),
                ("need", Out::S(b.need.clone())),
                ("owe", Out::S(b.owe.clone())),
                ("payUri", Out::S(b.pay_uri.clone())),
                ("until", Out::N(b.until)),
            ]),
        )),
        Some(Err(s)) => v.push(("budget", s.out())),
        None => {}
    }
    v.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(v)
}

impl Page<'_> {
    pub fn company_row_of(company: Option<&str>, buyer: &str, acting: &dyn Fn() -> Option<String>) -> bool {
        company_row(company, buyer, acting)
    }
}

#[cfg(test)]
mod page_tests {
    use super::*;
    use eravon_orders::status::{Status, Word};

    fn row(word: Word, instance: &str, series: Option<eravon_terms::doc::Series>) -> BRow {
        let row = Row {
            instance: instance.into(),
            case_id: String::new(),
            record: "kept".into(),
            number: String::new(),
            buyer: String::new(),
            label: String::new(),
            product: String::new(),
            amount: "10".into(),
            token: String::new(),
            status: Ok(Status { word, reason: String::new() }),
            deadline: 0,
            placed: 0,
            block: None,
            chain: None,
            local: None,
            handed: false,
            note: String::new(),
            terms_hash: String::new(),
            decimals: None,
            past: false,
            logic: String::new(),
            series,
        };
        BRow { row, owner: String::new(), record: String::new(), access: Access::Key, pack: None, seller: String::new(), product: String::new(), lf_gap: None, budget: None, receipt: None }
    }

    fn budget(owe: &str, lapsed: bool, revoked: bool) -> Budget {
        Budget { have: "0".into(), need: "10".into(), owe: owe.into(), until: 0, lapsed, revoked, emptied: None, buyer_may_open: true, payout: String::new(), receipt: String::new(), pay_uri: String::new(), deposit_short: false }
    }

    #[test]
    fn a_budget_taken_back_reads_expired_and_done_whatever_the_local_record() {
        let mut r = row(Word::AwaitDeploy, "0xa1", None);
        r.budget = Some(Ok(Budget { emptied: Some(Take::Lapsed), lapsed: true, ..budget("0", true, false) }));
        worded(&mut r);
        assert_eq!(r.row.status.as_ref().map(|s| s.word).ok(), Some(Word::Expired));
        assert!(eravon_orders::table::done_in(std::slice::from_ref(&r.row), &r.row), "进「已完成」");
        let mut u = row(Word::AwaitDeploy, "0xa2", None);
        u.budget = Some(Err(Said::bad("E_X", "读不成".to_string())));
        worded(&mut u);
        assert_eq!(u.row.status.as_ref().map(|s| s.word).ok(), Some(Word::AwaitDeploy), "预算读不成照原状态词");
    }

    fn key(o: &Out, k: &str) -> Option<Out> {
        match o {
            Out::O(m) => m.iter().find(|(x, _)| *x == k).map(|(_, v)| v.clone()),
            _ => None,
        }
    }

    fn said(o: Option<Out>) -> Option<String> {
        o.map(|x| String::from_utf8(eravon_literals::json::write(&x)).expect("utf8"))
    }

    #[test]
    fn a_row_says_its_instance_series_actions_and_head_word() {
        let series = eravon_terms::doc::Series { id: format!("0x{}", "11".repeat(32)), stage: 2, stages: 3 };
        let mut r = row(Word::AwaitPay, "0x00000000000000000000000000000000000000d4", Some(series));
        r.budget = Some(Ok(budget("0", true, true)));
        let out = row_out(&r, &[], read_only);
        assert_eq!(said(key(&out, "instance")), said(Some(Out::S("0x00000000000000000000000000000000000000d4".into()))));
        assert!(matches!(key(&out, "series"), Some(Out::O(_))));
        assert_eq!(said(key(&out, "preState")), said(Some(Out::S("revokedOpen".into()))));
        assert_eq!(said(key(&out, "acts")), said(Some(Out::A(vec![Out::S("expire".into())]))), "已下架,可取回:只留取回");
        let mut paid = row(Word::AwaitPay, "0x00000000000000000000000000000000000000d5", None);
        paid.budget = Some(Ok(budget("0", false, false)));
        assert_eq!(pre_state(&paid).map(Pre::word), Some("awaitOpen"));
        paid.budget = Some(Ok(budget("3", false, false)));
        assert_eq!(pre_state(&paid).map(Pre::word), Some("awaitPay"));
    }

    #[test]
    fn the_tabs_split_the_rows_as_the_page_does() {
        let rows = vec![row(Word::Expired, "0x00000000000000000000000000000000000000e1", None), row(Word::AwaitPay, "0x00000000000000000000000000000000000000e2", None)];
        let done: Vec<&str> = shown_of(&rows, Some(true), "", "", "").0.iter().map(|r| r.row.instance.as_str()).collect();
        let active: Vec<&str> = shown_of(&rows, Some(false), "", "", "").0.iter().map(|r| r.row.instance.as_str()).collect();
        assert_eq!(done, vec!["0x00000000000000000000000000000000000000e1"]);
        assert_eq!(active, vec!["0x00000000000000000000000000000000000000e2"]);
        assert_eq!(shown_of(&rows, None, "", "", "").0.len(), 2);
        assert!(shown_of(&rows, None, "", "2026/01/02", "").1.is_some(), "日期写错即说拒");
    }
}

pub type FeeLine = fn(&Row, bool, String) -> String;

pub fn fee_line(row: &Row, earns: bool, s2: String) -> String {
    match &row.chain {
        Some(v) if !earns && !v.events.iter().any(|e| e.name == "Closed") => "0".to_string(),
        _ => s2,
    }
}

pub fn earns(w: &Wire, r: &BRow) -> Result<bool, Said> {
    let Some(v) = &r.row.chain else { return Ok(false) };
    let now = eravon_standard::precheck::chain_now(&w.t, &w.table).map_err(|e| Said::bad(&e.name(), e.detail()))?;
    let seat = v.seat_state.map(|state| eravon_standard::precheck::Seat { state, accepted: v.accepted != 0, window_end: v.window_end });
    Ok(eravon_standard::precheck::premium_to_judge(eravon_standard::precheck::Close::Settle, v.phase, seat, now))
}

pub fn fee_rows(amount: String, judge_fee: Out, split: Option<&(String, String, String, String)>, mine: Result<Option<(String, usize)>, Said>) -> Vec<(&'static str, Out)> {
    let mut v = vec![("amount", Out::S(amount)), ("judgeFee", judge_fee)];
    if let Some((b, se, j, bb)) = split {
        let s = |x: &String| Out::S(x.clone());
        v.push(("split", Out::O(vec![("bondToBuyer", s(bb)), ("buyer", s(b)), ("judge", s(j)), ("seller", s(se))])));
    }
    match mine {
        Ok(Some((wei, n))) => {
            v.push(("mineFee", Out::S(wei)));
            v.push(("mineTxs", Out::N(n as u64)));
        }
        Ok(None) => {}
        Err(e) => v.push(("mineFee", e.out())),
    }
    v
}

pub fn delivery_files(d: &eravon_orders::detail::Delivery) -> Out {
    if !d.held {
        return Out::Null;
    }
    d.files.as_ref().map_or_else(|(n, x)| Said::bad(n, x.clone()).out(), |f| Out::N(f.len() as u64))
}

pub fn detail_out(data: &str, r: &BRow, w: &Wire, owners: &[String], fee_line: FeeLine) -> Out {
    let ov = eravon_orders::detail::overview(data, &r.owner, &r.row, None);
    let terms = match r.pack.as_deref().and_then(|b| k3::pack::pack(b).ok()) {
        Some(_) => "saved",
        None => "missing",
    };
    let ctx = std::sync::Arc::new((w.clone(), r.row.clone(), data.to_string(), r.owner.clone()));
    let read_record = {
        let c = ctx.clone();
        move || eravon_orders::detail::record(&sw(&c.0), &c.1, None)
    };
    let read_delivery = {
        let c = ctx.clone();
        move || eravon_orders::detail::delivery(&c.2, &c.3, &sw(&c.0), &c.1, None)
    };
    let read_dispute = {
        let c = ctx.clone();
        move || eravon_orders::detail::dispute(&c.2, &c.3, &sw(&c.0), &c.1, None)
    };
    let (record, (delivery, dispute)) = if eravon_chain::recall::current().is_none() {
        eravon_chain::read::both(read_record, move || eravon_chain::read::both(read_delivery, read_dispute))
    } else {
        (read_record(), (read_delivery(), read_dispute()))
    };
    let record = record
        .map(|v| Out::A(v.iter().map(|m| Out::O(vec![("at", Out::N(m.at)), ("name", Out::S(m.name.clone())), ("say", Out::S(m.say.clone())), ("tx", Out::S(m.tx.clone()))])).collect()))
        .unwrap_or_else(|(n, d)| Said::bad(&n, d).out());
    let fee = eravon_orders::detail::fee(&r.row);
    let judge_fee = match earns(w, r) {
        Ok(e) => Out::S(fee_line(&r.row, e, fee.judge_fee.clone())),
        Err(s) => s.out(),
    };
    let judge = if ov.judge.addr.is_empty() {
        r.pack.as_deref().and_then(|b| k3::pack::pack(b).ok()).and_then(|pk| eravon_orders::detail::chosen(&pk.listing, pk.receipt.candidate)).unwrap_or_default()
    } else {
        ov.judge.addr.clone()
    };
    Out::O(vec![
        ("delivery", Out::O(vec![("done", Out::B(delivery.done)), ("files", delivery_files(&delivery)), ("note", Out::S(delivery.note))])),
        (
            "dispute",
            match dispute {
                Some(d) => Out::O(vec![("deadline", Out::N(d.deadline)), ("evidence", Out::N(d.evidence.len() as u64)), ("lf", Out::S(d.lf))]),
                None => Out::S(String::new()),
            },
        ),
        ("fee", Out::O(fee_rows(fee.amount, judge_fee, fee.split.as_ref(), crate::deskops::receipts::order_fee(data, owners, &r.row.instance)))),
        ("overview", Out::O(vec![("judge", Out::S(judge)), ("judgeState", Out::S(ov.judge.state.into())), ("terms", Out::S(terms.into()))])),
        ("record", record),
    ])
}

#[cfg(test)]
mod fee_tab {
    #[test]
    fn the_fee_rows_carry_the_split_once_the_case_closed() {
        use eravon_literals::json::Out;
        let split = ("70".to_string(), "30".to_string(), "5".to_string(), "0".to_string());
        let with = String::from_utf8(eravon_literals::json::write(&Out::O(super::fee_rows("100".into(), Out::S("5".into()), Some(&split), Ok(None))))).unwrap_or_default();
        assert!(with.contains(r#""split":{"bondToBuyer":"0","buyer":"70","judge":"5","seller":"30"}"#), "{with}");
        let before = String::from_utf8(eravon_literals::json::write(&Out::O(super::fee_rows("100".into(), Out::S("5".into()), None, Ok(None))))).unwrap_or_default();
        assert!(!before.contains("split"), "{before}");
    }
}

#[cfg(test)]
mod delivery_tab {
    use eravon_literals::json::Out;

    fn delivered(held: bool, files: Vec<(String, u64, String)>) -> eravon_orders::detail::Delivery {
        eravon_orders::detail::Delivery { done: true, at: 1, note: String::new(), files: Ok(files), held, tx: "0x01".into(), deadline: 0, hand_over: None }
    }

    #[test]
    fn items_not_held_here_are_not_counted_as_none() {
        assert!(matches!(super::delivery_files(&delivered(false, Vec::new())), Out::Null), "没有交付记录:件数为 null");
        let one = vec![("a.txt".to_string(), 8, "0x00".to_string())];
        assert!(matches!(super::delivery_files(&delivered(true, one)), Out::N(1)), "有记录:照数件数");
        assert!(matches!(super::delivery_files(&delivered(true, Vec::new())), Out::N(0)), "记录里零件:是 0");
    }
}
