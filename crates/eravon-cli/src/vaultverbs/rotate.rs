use eravon_literals::constants as k7;
use eravon_standard as a1;
use eravon_store::seam::Box_;
use eravon_store::vault::Seat;
use eravon_terms as k2;

use crate::fault::{Fault, R};
use crate::shared::Wire;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seg {
    Confirmed,
    Claim(String),
    Open,
    Moved(String),
    Unread(String),
}

impl Seg {
    pub fn word(&self) -> &'static str {
        match self {
            Seg::Confirmed => "confirmed",
            Seg::Claim(_) => "awaitingAccession",
            Seg::Open => "attempt",
            Seg::Moved(_) => "moved",
            Seg::Unread(_) => "unread",
        }
    }
}

#[derive(Clone, Debug)]
pub struct OnChain {
    pub seat: Seat,
    pub chain: u64,
    pub name: &'static str,
    pub seg: Seg,
}

#[derive(Clone, Debug, Default)]
pub struct Checklist {
    pub chains: Vec<OnChain>,
    pub txs: u64,
    pub txs_most: u64,
    pub owed: Vec<eravon_store::settings::HandOn>,
    pub owed_other: usize,
    pub cases: Vec<String>,
    pub cases_unread: Vec<(u64, String, String, Seat)>,
    pub orders: Vec<(String, u64, [u8; 32], Seat)>,
    pub handed: Vec<(String, u64, String, Seat)>,
    pub offers: Vec<(String, u64, [u8; 32], Seat)>,
    pub terms: Vec<(String, u64, [u8; 32], String, Seat)>,
    pub fund: Option<String>,
    pub deposits: Vec<String>,
    pub refunds: Vec<(String, Seat)>,
    pub series_short: Vec<(String, u64, u64)>,
    pub unswept: Vec<u64>,
    pub stranded: Vec<(String, u64, Seat)>,
    pub refused: Vec<(Seat, String, String)>,
}

impl Checklist {
    fn join(&mut self, l: Checklist) {
        self.chains.extend(l.chains);
        self.txs += l.txs;
        self.txs_most += l.txs_most;
        for c in l.cases {
            if !self.cases.iter().any(|x| x.eq_ignore_ascii_case(&c)) {
                self.cases.push(c);
            }
        }
        self.cases_unread.extend(l.cases_unread);
        self.orders.extend(l.orders);
        self.handed.extend(l.handed);
        self.offers.extend(l.offers);
        self.terms.extend(l.terms);
        self.fund = self.fund.take().or(l.fund);
        self.deposits.extend(l.deposits);
        self.refunds.extend(l.refunds);
        self.series_short.extend(l.series_short);
        for c in l.unswept {
            if !self.unswept.contains(&c) {
                self.unswept.push(c);
            }
        }
        self.stranded.extend(l.stranded);
        self.refused.extend(l.refused);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Reach {
    pub roots: Option<String>,
    pub proxy: Option<String>,
    pub given: Option<(u64, eravon_chain::endpoints::Table, crate::home::Source)>,
    pub opts: crate::send::Opts,
}

impl Reach {
    pub fn of(f: &crate::shared::Flags) -> R<Reach> {
        let (inline, file) = (f.many("--endpoint"), f.one("--table"));
        let given = if inline.is_empty() && file.is_none() {
            None
        } else {
            let chain = crate::home::chain_id(f.opt_int("--chain")?);
            if k7::network_of(chain).is_none() {
                return Err(Fault::Arg(format!("--chain {chain} 不是已知的网")));
            }
            let (table, source) = crate::home::table(&inline, file, None)?;
            Some((chain, table, source))
        };
        Ok(Reach { roots: f.one("--roots").map(str::to_string), proxy: f.one("--proxy").map(str::to_string), given, opts: crate::send::Opts::of(f)? })
    }
}

pub struct In<'a> {
    pub data: &'a str,
    pub s: &'a eravon_vault::session::Session,
    pub reach: &'a Reach,
    pub seat: Seat,
    pub old: &'a str,
    pub now: u64,
    pub let_expire: &'a [String],
    pub revoke: &'a [String],
    pub coins: Option<&'a [String]>,
}

pub const E_ROTATE_MOVED: &str = "E_V_ROTATE_MOVED";
pub const E_ROTATE_ENTERED: &str = "E_V_ROTATE_ENTERED";
pub const E_ROTATE_NOT_LANDED: &str = "E_V_ROTATE_NOT_LANDED";
pub const E_ROTATE_NO_CHAIN: &str = "E_V_ROTATE_NO_CHAIN";
pub const E_ROTATE_RECORD: &str = "E_V_ROTATE_RECORD";
pub const E_ROTATE_DECIDED: &str = "E_V_ROTATE_DECIDED";
pub const E_ROTATE_HELD: &str = "E_V_ROTATE_HELD";

pub const E_ROTATE_OFF_NETWORK: &str = "E_V_ROTATE_OFF_NETWORK";

pub fn home_chain(data: &str, key: &str) -> Option<u64> {
    eravon_vault::store::network(data, key).ok()
}


fn nets_of(data: &str, key: &str) -> Vec<&'static k7::Network> {
    let home = home_chain(data, key);
    k7::NETWORKS.iter().filter(|n| Some(n.standard.chain_id) == home).collect()
}

fn nets(i: &In<'_>) -> Vec<&'static k7::Network> {
    nets_of(i.data, &nodes_of(i))
}

fn passed_by(data: &str, key: &str) -> bool {
    let laid = matches!(eravon_vault::store::home_settings(data, key), Ok(Some(_)));
    !laid || home_chain(data, key).is_none_or(|c| eravon_vault::store::endpoints_on(data, key, c).map_or(true, |n| n.is_empty()))
}


fn on_home(i: &In<'_>) -> R<()> {
    let Some((c, ..)) = &i.reach.given else { return Ok(()) };
    let home = home_chain(i.data, &nodes_of(i));
    if Some(*c) == home {
        return Ok(());
    }
    Err(named(E_ROTATE_OFF_NETWORK, format!("--chain {c} 不是这一席的家所在的网({}):换钥只做家选定的那条网", home.map_or_else(String::new, |h| h.to_string()))))
}

pub fn held_on(i: &In<'_>) -> Vec<String> {
    seller_of(i).map(|s| coins_on_chain(i, &s)).unwrap_or_default()
}

pub fn seller_of(i: &In<'_>) -> Option<String> {
    match i.seat {
        Seat::Seller => Some(i.old.to_ascii_lowercase()),
        Seat::Buyer => eravon_vault::accounts::other_seat(i.data, i.old).map(|k| k.to_ascii_lowercase()),
    }
}

fn coins_on_chain(i: &In<'_>, seller: &str) -> Vec<String> {
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let rows = eravon_vault::tokens::current();
    let mut orders: Vec<String> = eravon_watch::order::all(i.data, seller).map(|v| v.into_iter().map(|o| o.token.to_ascii_lowercase()).collect()).unwrap_or_default();
    orders.retain(|t| eravon_chain::seam::address_ok(t));
    let mut out: Vec<String> = Vec::new();
    for n in nets_of(i.data, seller) {
        let chain = n.standard.chain_id;
        let Ok(Some(w)) = wire_on(i.reach, i.data, seller, chain) else { continue };
        let mut tokens: Vec<String> = eravon_vault::tokens::on_chain(&rows, chain).into_iter().map(|r| r.address.to_ascii_lowercase()).collect();
        for t in &orders {
            if !tokens.contains(t) {
                tokens.push(t.clone());
            }
        }
        a1::chain::ahead_erc20_balances(&w.t, &w.table, &tokens.iter().map(|t| (t.clone(), seller.to_string())).collect::<Vec<_>>());
        for t in tokens {
            let held = a1::chain::erc20_balance(&w.t, &w.table, &t, seller).is_ok_and(|b| b.value != [0u8; 32]);
            if held && !out.contains(&t) {
                out.push(t);
            }
        }
    }
    restore(home);
    out
}

fn held_gate(i: &In<'_>) -> R<()> {
    let Some(seller) = seller_of(i) else { return Ok(()) };
    let held = match i.coins {
        Some(c) => c.to_vec(),
        None => coins_on_chain(i, &seller),
    };
    if held.is_empty() {
        return Ok(());
    }
    Err(named(E_ROTATE_HELD, format!("卖家钥 {seller} 上还有代币:{}", held.join("、"))))
}

pub(super) fn named(token: &str, why: impl Into<String>) -> Fault {
    Fault::Down { layer: "rotate", token: token.to_string(), why: why.into() }
}

pub fn wire_on(r: &Reach, data: &str, addr: &str, chain: u64) -> R<Option<Wire>> {
    let (table, source) = match &r.given {
        Some((c, t, src)) if *c == chain => (t.clone(), *src),
        _ => {
            let nodes = eravon_vault::store::endpoints_on(data, addr, chain).map_err(|e| Fault::State(e.detail()))?;
            if nodes.is_empty() {
                return Ok(None);
            }
            (eravon_chain::endpoints::Table::new(nodes)?, crate::home::Source::Identity)
        }
    };
    let n = k7::network_of(chain).ok_or_else(|| Fault::State(format!("链 {chain} 不是已知的网")))?;
    k7::select_chain(chain);
    let t = crate::home::transport(r.roots.as_deref(), r.proxy.as_deref(), source == crate::home::Source::Identity)?;
    Ok(Some(Wire { t, table, source, chain_id: chain, logic: n.standard.logic.to_string(), registries: Vec::new() }))
}

pub fn seg_of(w: &Wire, old: &str, new: Option<&str>) -> Seg {
    a1::chain::ahead_current_and_pending(&w.t, &w.table, old);
    let cur = match a1::chain::current(&w.t, &w.table, old) {
        Ok(r) => r.value,
        Err(f) => return Seg::Unread(format!("{}:{}", f.name(), f.detail())),
    };
    if let Some(n) = new.filter(|n| n.eq_ignore_ascii_case(&cur)) {
        let _ = n;
        return Seg::Confirmed;
    }
    if !cur.eq_ignore_ascii_case(old) {
        return Seg::Moved(cur);
    }
    match a1::chain::pending(&w.t, &w.table, old) {
        Ok(r) if r.value != a1::order::hex_of(&[0u8; 20]) => Seg::Claim(r.value),
        Ok(_) => Seg::Open,
        Err(f) => Seg::Unread(format!("{}:{}", f.name(), f.detail())),
    }
}

pub fn pending_new(i: &In<'_>) -> Option<String> {
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let next = eravon_vault::accounts::next_of(i.data, i.old);
    let mut found = None;
    for n in nets(i) {
        let Ok(Some(w)) = wire_on(i.reach, i.data, &nodes_of(i), n.standard.chain_id) else { continue };
        if let Seg::Claim(p) = seg_of(&w, i.old, None) {
            if next.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(&p)) || eravon_vault::accounts::find_in(i.data, &p).is_ok_and(|s| s == i.seat) {
                found = Some(p);
                break;
            }
        }
    }
    restore(home);
    found
}

fn restore(home: Option<u64>) {
    if let Some(c) = home {
        k7::select_chain(c);
    }
}

pub fn checklist(i: &In<'_>, new: Option<&str>) -> R<Checklist> {
    let mut out = seat_list(i, new)?;
    let others: Vec<(Seat, String, Option<String>)> = match new {
        Some(k) => seat_pairs(i.data, i.old, k)?.into_iter().filter(|(s, ..)| *s != i.seat).map(|(s, o, n)| (s, o, Some(n))).collect(),
        None => eravon_vault::accounts::other_seat(i.data, i.old).map(|o| (other_of(i.seat), o, None)).into_iter().collect(),
    };
    for (seat, old, newk) in others {
        if passed_by(i.data, &old) {
            continue;
        }
        let j = In { data: i.data, s: i.s, reach: i.reach, seat, old: &old, now: i.now, let_expire: i.let_expire, revoke: i.revoke, coins: i.coins };
        match seat_list(&j, newk.as_deref()) {
            Ok(l) => out.join(l),
            Err(Fault::Down { token, .. }) if token == E_ROTATE_NO_CHAIN => {}
            Err(Fault::Down { token, why, .. }) => out.refused.push((seat, token, why)),
            Err(f) => out.refused.push((seat, f.name().to_string(), f.detail())),
        }
    }
    Ok(out)
}

fn seat_list(i: &In<'_>, new: Option<&str>) -> R<Checklist> {
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let mut out = Checklist::default();
    let mut scanned: Vec<String> = Vec::new();
    let nets = nets(i);
    if nets.is_empty() {
        return Err(named(E_ROTATE_NO_CHAIN, "没有一条链读得出换钥簿:先配好节点"));
    }
    for n in nets {
        let chain = n.standard.chain_id;
        let w = match wire_on(i.reach, i.data, &nodes_of(i), chain) {
            Ok(Some(w)) => w,
            Ok(None) => {
                out.chains.push(OnChain { seat: i.seat, chain, name: n.name, seg: Seg::Unread("没有这条链的节点".into()) });
                continue;
            }
            Err(f) => {
                out.chains.push(OnChain { seat: i.seat, chain, name: n.name, seg: Seg::Unread(format!("{}:{}", f.name(), f.detail())) });
                continue;
            }
        };
        let seg = seg_of(&w, i.old, new);
        if matches!(i.seat, Seat::Seller) {
            match in_flight(&w, i.data, i.old) {
                Ok(c) => scanned.extend(c),
                Err(f) => out.cases_unread.push((chain, f.name().to_string(), f.detail(), i.seat)),
            }
        }
        if matches!(seg, Seg::Open | Seg::Claim(_)) {
            out.txs += if matches!(seg, Seg::Open) { 2 } else { 1 };
            out.txs_most += if matches!(seg, Seg::Open) { 4 } else { 3 };
        }
        if let (Seg::Confirmed, Some(k)) = (&seg, new) {
            if !swept_on(i.data, k, i.old, chain) {
                out.unswept.push(chain);
                out.txs_most += 1;
            }
        }
        out.chains.push(OnChain { seat: i.seat, chain, name: n.name, seg });
    }
    restore(home);
    match i.seat {
        Seat::Buyer => buyer_items(i, &mut out)?,
        Seat::Seller => seller_items(i, &mut out)?,
    }
    for c in scanned {
        if !out.cases.iter().any(|x| x.eq_ignore_ascii_case(&c)) {
            out.cases.push(c);
        }
    }
    if let Some(k) = new {
        let record = record_of(i.data, k)?;
        let decided = |kinds: &[&str], id: &str, c: u64| record.iter().any(|h| h.from.eq_ignore_ascii_case(i.old) && h.chain == c && h.id == id && kinds.contains(&h.kind.as_str()));
        use eravon_store::settings::hand_on::{OFFER, ORDER, TERMS};
        out.orders.retain(|(id, c, ..)| !decided(&[ORDER], id, *c));
        out.offers.retain(|(id, c, ..)| !decided(&[OFFER, TERMS], id, *c));
        out.terms.retain(|(id, c, ..)| !decided(&[OFFER, TERMS], id, *c));
        out.owed = owed_from(record, i.old);
        out.owed_other = other_owed(i.data, k)?;
    }
    let chosen = |v: &[String], id: &str| v.iter().any(|x| x == id);
    let registered = out.orders.iter().filter(|(id, ..)| !chosen(i.let_expire, id)).count() + out.offers.iter().filter(|(id, ..)| !chosen(i.revoke, id)).count();
    let revokes = out.terms.iter().filter(|(id, ..)| chosen(i.revoke, id)).count();
    let calls = (registered + revokes + out.owed.len()) as u64;
    out.txs += calls;
    out.txs_most += 2 * calls + if out.fund.is_some() { 2 } else { 0 };
    if out.chains.iter().filter(|c| c.seat == i.seat).all(|c| matches!(c.seg, Seg::Unread(_))) {
        return Err(named(E_ROTATE_NO_CHAIN, "没有一条链读得出换钥簿:先配好节点"));
    }
    if let Some((c, k)) = out.chains.iter().filter(|c| c.seat == i.seat).find_map(|c| match &c.seg {
        Seg::Moved(k) => Some((c.chain, k.clone())),
        _ => None,
    }) {
        return Err(named(E_ROTATE_MOVED, format!("链 {c} 上 {} 的记录已交给 {k},它不再是现行钥", i.old)));
    }
    out.stranded = stranded_of(i).into_iter().map(|(k, c)| (k, c, i.seat)).collect();
    Ok(out)
}

fn other_of(s: Seat) -> Seat {
    match s {
        Seat::Seller => Seat::Buyer,
        Seat::Buyer => Seat::Seller,
    }
}

fn in_flight(w: &Wire, data: &str, key: &str) -> R<Vec<String>> {
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let t = eravon_orders::table::table(data, key, eravon_scan::scan::Side::Seller, &sw)
        .and_then(eravon_orders::table::Table::complete)
        .map_err(|(token, why)| Fault::Down { layer: "S2", token, why: format!("链 {} 上本席的订单没读全,在途的案列不全:{why}", w.chain_id) })?;
    Ok(t.rows.iter().filter(|r| !eravon_orders::table::done(r)).map(|r| r.instance.clone()).collect())
}

fn buyer_items(i: &In<'_>, out: &mut Checklist) -> R<()> {
    let s = eravon_vault::store::open_read(i.data)?;
    let rows = match s.list(i.old, Box_::Packs) {
        Ok(r) => r,
        Err(eravon_store::fault::Fault::Missing(_)) => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut items = Vec::new();
    let home = home_chain(i.data, &nodes_of(i));
    for r in rows.into_iter().filter(|r| r.kind == eravon_store::record::Kind::OrderFile) {
        let Ok(b) = s.part(i.old, Box_::Packs, &r.id, "pack.json") else { continue };
        let Ok(pk) = eravon_link::pack::pack(&b) else { continue };
        if Some(pk.listing.chain_id) != home || !pk.receipt.buyer_key.eq_ignore_ascii_case(i.old) || a1::chain::listing_payable(&pk.listing.logic).is_err() {
            continue;
        }
        let o = crate::shared::order_of(pk.listing.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev)?;
        let d = o.derive(pk.listing.chain_id, &pk.listing.logic)?;
        items.push((r.id.clone(), pk.listing.chain_id, d, pk.listing.token.clone()));
    }
    ahead_chains(i, &items.iter().map(|(_, c, d, tok)| (*c, d.instance.clone(), Some(tok.clone()))).collect::<Vec<_>>());
    for (id, chain, d, token) in items {
        match paid_unopened(i, chain, &d.instance, &token) {
            Some(true) => out.orders.push((id, chain, d.order_digest, i.seat)),
            Some(false) => out.cases.push(d.instance.clone()),
            None => {}
        }
    }
    let owners = eravon_vault::keyset::buyer(i.s, i.old).map_err(|e| Fault::State(e.detail()))?.owners;
    let handed: Vec<_> = eravon_orders::table::hand_outs(i.data, &owners).map_err(|(n, d)| named(&n, d))?.into_iter().filter(|h| h.by.eq_ignore_ascii_case(i.old) && Some(h.chain) == home).collect();
    ahead_chains(i, &handed.iter().map(|h| (h.chain, h.instance.clone(), None)).collect::<Vec<_>>());
    for h in handed {
        if !case_ended(i, h.chain, &h.instance) {
            out.handed.push((h.record, h.chain, h.instance, i.seat));
        }
    }
    Ok(())
}

fn ahead_chains(i: &In<'_>, items: &[(u64, String, Option<String>)]) {
    let mut chains: Vec<u64> = items.iter().map(|x| x.0).collect();
    chains.sort_unstable();
    chains.dedup();
    for c in chains {
        let mine: Vec<&(u64, String, Option<String>)> = items.iter().filter(|x| x.0 == c).collect();
        if mine.len() < 2 {
            continue;
        }
        let home = eravon_vault::store::network(i.data, i.old).ok();
        if let Ok(Some(w)) = wire_on(i.reach, i.data, &nodes_of(i), c) {
            a1::chain::ahead_stages(&w.t, &w.table, &mine.iter().map(|x| x.1.clone()).collect::<Vec<_>>());
            a1::chain::ahead_erc20_balances(&w.t, &w.table, &mine.iter().filter_map(|x| Some((x.2.clone()?, x.1.clone()))).collect::<Vec<_>>());
        }
        restore(home);
    }
}

fn case_ended(i: &In<'_>, chain: u64, instance: &str) -> bool {
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let ended = wire_on(i.reach, i.data, &nodes_of(i), chain).ok().flatten().and_then(|w| a1::chain::stage(&w.t, &w.table, instance).ok()).is_some_and(|st| st.value.ended);
    restore(home);
    ended
}

fn paid_unopened(i: &In<'_>, chain: u64, instance: &str, token: &str) -> Option<bool> {
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let w = wire_on(i.reach, i.data, &nodes_of(i), chain).ok()??;
    let got = (|| {
        if a1::chain::has_code(&w.t, &w.table, instance).ok()? {
            let st = a1::chain::stage(&w.t, &w.table, instance).ok()?.value;
            if st.case_id != [0u8; 32] {
                return (!st.ended).then_some(false);
            }
        }
        let bal = a1::chain::erc20_balance(&w.t, &w.table, token, instance).ok()?.value;
        (bal != [0u8; 32]).then_some(true)
    })();
    restore(home);
    got
}

fn seller_items(i: &In<'_>, out: &mut Checklist) -> R<()> {
    out.fund = eravon_payways::ways::read(i.data, i.old).ok().map(|w| w.fund).filter(|f| !f.is_empty() && !f.eq_ignore_ascii_case(i.old));
    let rows = eravon_goods::goods::list_every_chain(i.data, i.old, "").map_err(|e| Fault::State(e.detail()))?;
    let mut series: Vec<(String, u64, u64)> = Vec::new();
    let home = home_chain(i.data, &nodes_of(i));
    for r in rows.iter().filter(|r| r.listed && !r.draft) {
        let Ok(Some(bytes)) = eravon_goods::goods::terms_of(i.data, i.old, &r.id) else { continue };
        let Ok(t) = k2::doc::terms(&bytes) else { continue };
        if Some(t.chain_id) != home || !t.seller_key.eq_ignore_ascii_case(i.old) {
            continue;
        }
        if t.seller_fund.eq_ignore_ascii_case(i.old) {
            out.deposits.push(r.id.clone());
        }
        if let Some(sr) = &t.series {
            match series.iter_mut().find(|x| x.0.eq_ignore_ascii_case(&sr.id)) {
                Some(x) => x.1 += 1,
                None => series.push((sr.id.to_ascii_lowercase(), 1, sr.stages)),
            }
        }
        if t.buyer_may_open {
            for c in 0..(t.judges.len() + t.guilds.len()) as u64 {
                out.offers.push((r.id.clone(), t.chain_id, k2::derive::offer_digest_at(&t, c)?, i.seat));
            }
            out.terms.push((r.id.clone(), t.chain_id, k2::doc::terms_hash(&bytes), t.logic.clone(), i.seat));
        }
    }
    out.series_short = series.into_iter().filter(|(_, have, all)| have < all).collect();
    for o in eravon_watch::order::all(i.data, i.old).map_err(|(n, d)| named(&n, d))? {
        if !matches!(o.state, eravon_watch::order::State::Cancelled | eravon_watch::order::State::Expired | eravon_watch::order::State::Revoked) {
            out.cases.push(o.instance.clone());
        }
        use eravon_watch::order::State as S;
        if matches!(o.state, S::Ready | S::PendingConsent | S::PriceStale | S::DepositShort | S::Blocked | S::AwaitPrev) && funded_by(i.data, i.old, &o.id) {
            out.refunds.push((o.id.clone(), i.seat));
        }
    }
    Ok(())
}

fn funded_by(data: &str, key: &str, id: &str) -> bool {
    eravon_watch::order::part(data, key, id, &eravon_watch::order::part_terms()).ok().and_then(|b| k2::doc::terms(&b).ok()).is_some_and(|t| t.seller_fund.eq_ignore_ascii_case(key))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    pub what: String,
    pub chain: Option<u64>,
    pub name: String,
    pub detail: String,
}

impl Left {
    fn of(what: &str, chain: Option<u64>, f: &Fault) -> Left {
        Left { what: what.to_string(), chain, name: f.name().to_string(), detail: f.detail() }
    }

    pub fn why(&self) -> String {
        format!("{}:{}", self.name, self.detail)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Done {
    pub new: String,
    pub finished: Option<String>,
    pub phrase: Option<String>,
    pub chains: Vec<OnChain>,
    pub sent: Vec<(&'static str, u64, String)>,
    pub left: Vec<Left>,
    pub let_expire: Vec<String>,
    pub funds: Vec<(u64, String, a1::call::Call)>,
    pub swept: Vec<Swept>,
    pub owed: Vec<eravon_store::settings::HandOn>,
    pub owed_other: usize,
    pub unswept: Vec<u64>,
    pub backup_again: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Swept {
    pub chain: u64,
    pub from: String,
    pub how: Sweep,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sweep {
    Moved { tx: String, amount: String },
    Short { balance: String, fee: String },
}

impl Sweep {
    pub fn word(&self) -> &'static str {
        match self {
            Sweep::Moved { .. } => "moved",
            Sweep::Short { .. } => "short",
        }
    }
}

fn swept_entry(chain: u64, former: &str) -> String {
    format!("{chain}:{}", former.to_ascii_lowercase())
}

pub fn swept_on(data: &str, new: &str, former: &str, chain: u64) -> bool {
    eravon_vault::store::read_identity(data, new).is_ok_and(|t| t.swept.contains(&swept_entry(chain, former)))
}

fn sweep(w: &Wire, i: &In<'_>, (old, old_key): (&str, &[u8; 32]), new: &str, done: &mut Done) {
    if swept_on(i.data, new, old, w.chain_id) {
        return;
    }
    let got = (|| -> R<Sweep> {
        let balance = k2::u256::U256::from_dec(&eravon_chain::read::balance(&w.t, &w.table, old, "latest")?.0).ok_or_else(|| Fault::State(format!("{old} 的余额读不成整数")))?;
        let to = a1::call::Call { to: new.to_string(), data: Vec::new() };
        let (terms, fee) = crate::send::pinned(&i.reach.opts, w, old, &to)?;
        let fee_n = k2::u256::U256::from_dec(&fee).ok_or_else(|| Fault::State(format!("转账费 {fee:?} 读不成整数")))?;
        if balance.cmp_u(&fee_n) != std::cmp::Ordering::Greater {
            return Ok(Sweep::Short { balance: balance.to_dec(), fee });
        }
        let amount = balance.overflowing_sub(&fee_n).0.to_dec();
        let tx = send(w, &terms, old, old_key, &to, &amount)?;
        Ok(Sweep::Moved { tx, amount })
    })();
    match got {
        Ok(how) => {
            if let Sweep::Moved { tx, .. } = &how {
                done.sent.push(("sweep", w.chain_id, tx.clone()));
            }
            let settled = eravon_vault::store::change_identity(i.data, new, |t| {
                let e = swept_entry(w.chain_id, old);
                if !t.swept.contains(&e) {
                    t.swept.push(e);
                }
            });
            if let Err(e) = settled {
                done.left.push(Left::of(old, Some(w.chain_id), &Fault::State(format!("余额已归新钥,本机记录没写成:{}", e.detail()))));
            }
            done.swept.push(Swept { chain: w.chain_id, from: old.to_ascii_lowercase(), how });
        }
        Err(f) => done.left.push(Left::of(old, Some(w.chain_id), &f)),
    }
}


fn make_new(i: &In<'_>, _writer: &str) -> R<(String, Option<String>, Vec<Left>)> {
    let (new, phrase) = eravon_vault::accounts::next_words(i.s, i.old).map_err(|e| Fault::State(e.detail()))?;
    Ok((new, phrase, Vec::new()))
}

fn add_new(i: &In<'_>, writer: &str) -> R<(String, String, Vec<Left>)> {
    let st = eravon_vault::store::open_write(i.data, writer, i.now)?;
    let (new, phrase) = eravon_vault::accounts::new_words_as(i.s, i.seat, "换钥新钥", false, i.now).map_err(|e| Fault::State(e.detail()))?;
    eravon_vault::accounts::switch(i.data, i.seat, i.old).map_err(|e| Fault::State(e.detail()))?;
    let b = eravon_vault::onboard::born_added(&st, i.data, &new, eravon_literals::constants::current_network().standard.chain_id, i.now);
    b.own.map_err(|e| Fault::State(e.detail()))?;
    let mut left: Vec<Left> = b.others.into_iter().filter_map(|o| o.err()).map(|e| Left { what: new.clone(), chain: None, name: e.name().to_string(), detail: e.detail() }).collect();
    let own = i.old.to_ascii_lowercase();
    for (_, old, newk) in seat_pairs(i.data, i.old, &new)? {
        let got = carry_nodes(i.data, &old, &newk, old == own);
        match got {
            Err(f) if old == own => return Err(f),
            Err(f) => left.push(Left::of(&newk, None, &f)),
            Ok(()) => {}
        }
    }
    Ok((new, phrase, left))
}

fn carry_nodes(data: &str, old: &str, new: &str, own: bool) -> R<()> {
    let read = |a: &str| eravon_vault::store::home_settings(data, a).map_err(|e| Fault::State(e.detail()));
    let from = read(old)?;
    if !own && (from.is_none() || read(new)?.is_none()) {
        return Ok(());
    }
    let from = from.unwrap_or_default();
    eravon_vault::store::change_home(data, new, |h| {
        h.endpoints = from.endpoints;
        h.rest = from.rest;
        h.rest_on = from.rest_on;
        h.chain_id = from.chain_id;
    })
    .map_err(|e| Fault::State(e.detail()))
}

pub fn signed_to(key: &[u8; 32], old: &str) -> R<String> {
    let challenge = k2::derive::keccak(format!("{}{}", k7::SUCCESSION_BOOK.to_ascii_lowercase(), old.to_ascii_lowercase()).as_bytes());
    let sig = crate::shared::sign(key, &challenge)?;
    let raw = a1::order::bytes_of("签名", &sig)?;
    let (r, s, v): ([u8; 32], [u8; 32], u8) = (raw[..32].try_into().expect("32"), raw[32..64].try_into().expect("32"), raw[64]);
    let a = eravon_galeed::sign::recover(&challenge, &r, &s, v.saturating_sub(27)).ok_or_else(|| Fault::State("新钥的签名恢复不出地址".into()))?;
    Ok(a1::order::hex_of(&a))
}


#[cfg(test)]
mod tests {
    use eravon_store::vault::Seat;

    #[test]
    fn a_chain_that_stops_does_not_keep_the_chains_after_it() {
        let at = |chain| super::OnChain { seat: Seat::Seller, chain, name: "", seg: super::Seg::Open };
        let chains = [at(1), at(11155111), at(17000)];
        let mut stepped = Vec::new();
        let stops = super::each_chain(&chains, |c| {
            stepped.push(c.chain);
            if c.chain == 1 { Err(super::named(super::E_ROTATE_NOT_LANDED, "主网上旧钥付不起费")) } else { Ok(()) }
        });
        assert_eq!(stepped, vec![1, 11155111, 17000]);
        assert_eq!(stops.len(), 1);
        assert_eq!(stops[0].0, Some(1));
        assert!(stops[0].1.detail().ends_with("主网上旧钥付不起费"), "{}", stops[0].1.detail());
    }

    #[test]
    fn a_stop_keeps_its_name_apart_and_the_answer_joins_them() {
        let f = crate::fault::Fault::State("没有一条链读得出换钥簿".into());
        let l = super::Left::of("0x01", Some(1), &f);
        assert_eq!((l.name.as_str(), l.detail.as_str()), ("E_V_STATE", "没有一条链读得出换钥簿"));
        assert_eq!(l.why(), "E_V_STATE:没有一条链读得出换钥簿");
    }

    fn flags(v: &[&str]) -> crate::shared::Flags {
        crate::shared::Flags::read(&v.iter().map(|x| x.to_string()).collect::<Vec<_>>()).expect("形对")
    }

    #[test]
    fn the_rotation_takes_its_nodes_and_send_options_from_the_flags() {
        let r = super::Reach::of(&flags(&["--endpoint", "http://127.0.0.1:8545", "--chain", "11155111", "--max-fee", "5", "--priority-fee", "1", "--gas-limit", "90000", "--wait", "0", "--proxy", "none"])).expect("读得成");
        let (chain, table, _) = r.given.as_ref().expect("给了节点");
        assert_eq!(*chain, 11_155_111);
        assert_eq!(table.list.len(), 1);
        assert_eq!(r.opts.fees, Some(("5".to_string(), "1".to_string())));
        assert_eq!((r.opts.gas_limit, r.opts.wait), (Some(90_000), Some(0)));
        assert_eq!(r.proxy.as_deref(), Some("none"));
        assert!(matches!(super::Reach::of(&flags(&["--endpoint", "http://127.0.0.1:8545", "--chain", "5"])), Err(crate::fault::Fault::Arg(_))), "an unknown network");
        let none = super::Reach::of(&flags(&[])).expect("读得成");
        assert!(none.given.is_none() && none.proxy.is_none() && none.opts.fees.is_none());
    }

    #[test]
    fn the_declared_key_is_the_one_that_signed() {
        let key = [7u8; 32];
        let other = [9u8; 32];
        let old = "0x1111111111111111111111111111111111111111";
        let me = crate::shared::address_of(&key).unwrap();
        assert!(super::signed_to(&key, old).unwrap().eq_ignore_ascii_case(&me));
        assert!(!super::signed_to(&other, old).unwrap().eq_ignore_ascii_case(&me));
    }

    const PIN: &str = "48261537";
    const PHRASE: &str = "test test test test test test test test test test test junk";
    const NOW: u64 = 1_790_000_000;
    const WRITER: &str = "rotate-test";

    fn opened() -> (String, eravon_vault::session::Session) {
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create(PIN, Seat::Seller, "主钥", Some(PHRASE), NOW).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut s = eravon_vault::session::Session::locked(&data);
        s.unlock(PIN).expect("开得了");
        (data, s)
    }

    fn keys(data: &str, open: &eravon_vault::session::Session) -> (String, String, String) {
        let s = eravon_vault::store::open_write(data, WRITER, NOW).expect("可写");
        s.init(&eravon_store::settings::Global::default()).expect("铺得下");
        let mut out = vec![eravon_vault::accounts::who(data, Seat::Seller).expect("首枚钥").to_ascii_lowercase()];
        for label in ["二", "三"] {
            out.push(eravon_vault::accounts::new_words(open, Seat::Seller, label, NOW).expect("添一把").0.to_ascii_lowercase());
        }
        for k in &out {
            match s.init_identity(k, NOW) {
                Ok(()) | Err(eravon_store::fault::Fault::Exists(_)) => {}
                Err(e) => panic!("登记身份 {k}: {e:?}"),
            }
        }
        (out[0].clone(), out[1].clone(), out[2].clone())
    }

    fn both_seats() -> (String, eravon_vault::session::Session, String, String) {
        use eravon_vault::onboard;
        let data = eravon_vault::testing::fresh();
        let seller = onboard::land(&data, onboard::Mode::First, Seat::Seller, "主钥", false, onboard::Source::Words(PHRASE), PIN, None, None, NOW).expect("首启").address;
        let mut s = eravon_vault::session::Session::locked(&data);
        s.unlock(PIN).expect("开得了");
        let (row, _) = eravon_vault::records::seat_of(&seller).expect("身份");
        let buyer = row.address(eravon_vault::vault::role_of(Seat::Buyer)).expect("买家席地址").hex();
        onboard::seat(&data, Seat::Buyer, &buyer, eravon_literals::constants::default_network().standard.chain_id, NOW).expect("买家席就绪");
        for (k, node, chain) in [(&seller, "http://127.0.0.1:18501", 31_337), (&buyer, "http://127.0.0.1:18502", 11_155_111)] {
            eravon_vault::store::change_home(&data, k, |h| {
                h.endpoints = vec![node.to_string()];
                h.chain_id = Some(chain);
            })
            .expect("节点落得下");
        }
        (data, s, seller.to_ascii_lowercase(), buyer.to_ascii_lowercase())
    }

    #[test]
    fn a_new_key_is_born_on_each_seat_with_that_seats_old_nodes() {
        let (data, s, seller, buyer) = both_seats();
        let reach = super::Reach::default();
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let done = super::fresh(&i, WRITER).expect("换得了一把新钥");
        assert!(done.left.is_empty(), "{:?}", done.left);
        let pairs = super::seat_pairs(&data, &seller, &done.new).expect("两席成对");
        assert_eq!(pairs.len(), 2, "{pairs:?}");
        for (seat, old, new) in pairs {
            let (o, n) = (eravon_vault::store::home_settings(&data, &old).unwrap().expect("旧钥的家"), eravon_vault::store::home_settings(&data, &new).unwrap().expect("新钥生出了家"));
            assert_eq!((&n.endpoints, n.chain_id), (&o.endpoints, o.chain_id), "{seat:?} 席的新钥照旧钥那一席的节点读");
        }
        assert_ne!(buyer, done.new);
    }

    #[test]
    fn a_new_key_is_not_current_until_it_took_the_record_over() {
        let (data, s, seller, _) = both_seats();
        let reach = super::Reach::default();
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let rows = eravon_vault::vault::read(&data).expect("身份表").identities.len();
        let (new, phrase, _) = super::make_new(&i, WRITER).expect("造得出新钥");
        assert_ne!(new.to_ascii_lowercase(), seller);
        let now = eravon_vault::accounts::active(&data, Seat::Seller).expect("读得出当前钥").expect("有当前钥");
        assert_eq!(now.to_ascii_lowercase(), seller, "the old key stays current");
        assert_eq!(eravon_vault::vault::read(&data).expect("身份表").identities.len(), rows, "no row made");
        assert_eq!(eravon_vault::accounts::next_of(&data, &seller).as_deref(), Some(new.to_ascii_lowercase().as_str()), "the key made ready next in the seat's own row");
        let signer = |k: &str| crate::shared::address_of(&s.key_for(k).expect("钥在库里")).expect("地址").to_ascii_lowercase();
        assert_eq!((signer(&seller), signer(&new)), (seller.clone(), new.to_ascii_lowercase()), "the declaration's old key and the accession's new key");
        let (again, phrase_again, _) = super::make_new(&i, WRITER).expect("再造一次");
        assert_eq!((again.to_ascii_lowercase(), phrase_again), (new.to_ascii_lowercase(), phrase));
    }

    #[test]
    fn a_rotation_hands_over_in_the_identitys_own_row() {
        let (data, s, seller, buyer) = both_seats();
        let reach = super::Reach::default();
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let before = eravon_vault::vault::read(&data).expect("身份表").identities.len();
        let home = eravon_vault::records::seat_of(&seller).expect("身份").0.home(eravon_vault::roles::Role::Author).expect("卖家席的家");
        let (new, phrase, _) = super::make_new(&i, WRITER).expect("造得出新钥");
        let phrase = phrase.expect("a row of recovery words gets new words");
        let pairs = super::seat_pairs(&data, &seller, &new).expect("两席成对");
        assert_eq!(pairs.len(), 2, "both seats hand on together: {pairs:?}");
        let new_buyer = pairs.iter().find(|(s, ..)| *s == Seat::Buyer).map(|(_, o, n)| (o.clone(), n.clone())).expect("买家席");
        assert_eq!(new_buyer.0, buyer);
        eravon_vault::accounts::hand_over(&data, &new).expect("接任");
        assert_eq!(eravon_vault::accounts::active(&data, Seat::Seller).expect("读得出").as_deref(), Some(new.to_ascii_lowercase().as_str()));
        assert_eq!(eravon_vault::vault::read(&data).expect("身份表").identities.len(), before, "the menu's rows as they were");
        let (row, _) = eravon_vault::records::seat_of(&new).expect("新钥认得出那一行");
        assert_eq!(row.home(eravon_vault::roles::Role::Author), Some(home), "the same home");
        assert_eq!(eravon_vault::records::seat_of(&seller).expect("旧钥仍认得出那一行").0.id, row.id, "the old key names the same row");
        assert_eq!(eravon_vault::records::seat_of(&new_buyer.1).expect("买家席的新钥").0.id, row.id);
        let signer = |k: &str| crate::shared::address_of(&s.key_for(k).expect("钥在库里")).expect("地址").to_ascii_lowercase();
        assert_eq!((signer(&seller), signer(&new)), (seller.clone(), new.to_ascii_lowercase()), "after office each key still signs as itself (the old key's sweep, the new key's writes)");
        assert_eq!(super::seat_pairs(&data, &seller, &new).expect("接任之后").len(), 2, "after office: each seat's key before and in office");
        let carried = eravon_vault::store::read_identity(&data, &seller).unwrap_or_default();
        eravon_vault::store::change_identity(&data, &new, |t| t.take_over(&seller, &carried)).expect("接任记录");
        assert_eq!(eravon_vault::store::archive_of(&data, &new).expect("归档"), vec![new.to_ascii_lowercase()], "one home read once");
        assert!(eravon_vault::store::keys_of(&data, &new).expect("各钥").contains(&seller), "the old key is still this seat's");
        let b = eravon_vault::keyset::buyer(&s, &new_buyer.1).expect("买家钥集");
        assert_eq!(b.owners, vec![new_buyer.1.to_ascii_lowercase()], "the buyer's home read once, by its key in office");
        assert!(b.mine.contains(&buyer) && b.keys.contains(&buyer), "the buyer key before is the seat's own and scanned: {b:?}");
        assert_eq!(s.phrase_for(&new).expect("助记词"), phrase, "the identity's words are the new ones");
        assert!(eravon_vault::vault::recover(&data, PHRASE, "73915264", "73915264").is_err(), "the old words open the vault no more");
        eravon_vault::vault::recover(&data, &phrase, "73915264", "73915264").expect("the new words open it");
    }

    #[test]
    fn each_key_of_a_row_is_taken_from_its_own_slot_and_a_slot_that_is_not_its_key_is_refused() {
        let (data, s, seller, _) = both_seats();
        let reach = super::Reach::default();
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let signer = |k: &str| crate::shared::address_of(&s.key_for(k).expect("钥在库里")).expect("地址").to_ascii_lowercase();
        let (next, _, _) = super::make_new(&i, WRITER).expect("造得出新钥");
        let next = next.to_ascii_lowercase();
        assert_eq!(signer(&next), next, "the key made ready next is its own key, not the key in office");
        eravon_vault::accounts::hand_over(&data, &next).expect("接任");
        assert_eq!(signer(&seller), seller, "the key before is its own key, not the key in office");
        assert_eq!(signer(&next), next);
        let at = eravon_vault::key::Address::parse(&seller).expect("地址");
        eravon_vault::keybox::put(&eravon_vault::places::key_slot(&at), &[7u8; 32]).expect("槽换成另一把钥");
        let e = s.key_for(&seller).expect_err("槽里的钥与地址对不上");
        assert_eq!(e.name(), "E_VAULT_KEYFILE_BROKEN", "{}", e.detail());
    }

    #[test]
    fn a_rotation_finishes_only_what_this_vault_holds_and_reads_by_the_current_key() {
        let (data, s, seller, _) = both_seats();
        let reach = super::Reach::default();
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        assert!(super::held(&i, &seller), "the seat's key is in the vault");
        assert!(!super::held(&i, &format!("0x{}", "f1".repeat(20))), "a key the vault never held");
        assert_eq!(super::nodes_of(&i).to_ascii_lowercase(), seller, "before the seat moves, the old key's nodes");
        let (new, _, _) = super::make_new(&i, WRITER).expect("造得出新钥");
        eravon_vault::accounts::hand_over(&data, &new).expect("新钥接任");
        assert_eq!(super::nodes_of(&i).to_ascii_lowercase(), new.to_ascii_lowercase(), "once the seat moved, the new key's nodes, whichever key is handed on");
    }

    #[test]
    fn each_seat_and_chain_is_listed_once_and_each_item_keeps_its_seat() {
        let row = |seat: Seat, chain: u64| super::OnChain { seat, chain, name: "", seg: super::Seg::Open };
        let one = |seat: Seat| super::Checklist {
            chains: vec![row(seat, 1), row(seat, 11_155_111)],
            txs: 4,
            txs_most: 8,
            offers: if seat == Seat::Seller { vec![("p1".to_string(), 1, [7u8; 32], seat)] } else { Vec::new() },
            ..super::Checklist::default()
        };
        let mut l = one(Seat::Buyer);
        l.join(one(Seat::Seller));
        let mut keys: Vec<(u8, u64)> = l.chains.iter().map(|c| (c.seat as u8, c.chain)).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!((l.chains.len(), keys.len()), (4, 4), "each (seat, chain) once: {:?}", l.chains);
        assert_eq!((l.txs, l.txs_most), (8, 16), "the seats' sum");
        assert_eq!(l.offers.iter().map(|o| o.3).collect::<Vec<_>>(), vec![Seat::Seller], "the seller seat's offer, said as the seller seat's");
    }

    #[test]
    fn each_chain_stands_on_its_own_receipts() {
        const NODE: &str = "http://127.0.0.1:8545";
        let _outer = eravon_chain::action::open(None);
        eravon_chain::action::landed(NODE, 7);
        let c = |chain: u64| super::OnChain { seat: Seat::Seller, chain, name: "", seg: super::Seg::Open };
        let mut floors = Vec::new();
        let stops = super::each_chain(&[c(1), c(11_155_111)], |x| {
            floors.push(eravon_chain::action::floor_among([NODE]));
            eravon_chain::action::landed(NODE, if x.chain == 1 { 21_000_000 } else { 7_000_000 });
            Ok(())
        });
        assert!(stops.is_empty());
        assert_eq!(floors, vec![None, None], "each chain starts with no receipt read");
        assert_eq!(eravon_chain::action::floor_among([NODE]), Some(7), "the action around them comes back as it was");
    }

    #[test]
    fn nodes_given_through_the_door_for_another_chain_keep_the_homes_own_network() {
        let (data, s, _seller, buyer) = both_seats();
        let here = eravon_literals::constants::current_network().standard.chain_id;
        let other = if here == 1 { 11_155_111 } else { 1 };
        eravon_vault::store::change_home(&data, &buyer, |h| {
            h.endpoints = vec![format!("{here}=http://127.0.0.1:18502")];
            h.chain_id = Some(here);
        })
        .expect("节点落得下");
        let args = [("writer", WRITER.to_string()), ("now", NOW.to_string()), ("chain", other.to_string()), ("nodes", "http://127.0.0.1:18503".to_string())];
        let req = crate::door::Request { verb: "buyer-settings-endpoints".into(), home: data.clone(), args: args.iter().map(|(k, v)| (k.to_string(), v.clone())).collect() };
        let body = crate::deskwire::run_for_desktop(&req, crate::deskwire::Lent { session: s.fork().expect("借得出") }).expect("门答得出");
        assert!(body.contains("\"ok\":true"), "{body}");
        let h = eravon_vault::store::home_settings(&data, &buyer).unwrap().expect("买家的家");
        assert_eq!(eravon_vault::store::urls_on(&h.endpoints, here), vec!["http://127.0.0.1:18502".to_string()], "the home's own network keeps its node: {:?}", h.endpoints);
        assert_eq!(eravon_vault::store::urls_on(&h.endpoints, other), vec!["http://127.0.0.1:18503".to_string()], "the node typed for chain {other} is written under it: {:?}", h.endpoints);
        assert_eq!(eravon_literals::constants::current_network().standard.chain_id, here, "the call put no network in effect");
    }

    #[test]
    fn the_candidate_table_goes_with_the_record_where_the_new_key_has_none() {
        let (data, open) = opened();
        let (old, new, third) = keys(&data, &open);
        let (old, new, third) = (old.as_str(), new.as_str(), third.as_str());
        let judge = "0x00000000000000000000000000000000000000d1";
        eravon_cands::table::add(&data, WRITER, NOW, old, eravon_cands::table::Side::Judge, judge, "甲", "w1").expect("旧钥的候选");
        assert!(eravon_cands::table::carry(&data, WRITER, NOW, old, new).expect("搬得了"));
        let rows = eravon_cands::table::list(&data, new).expect("新钥的表");
        assert_eq!(rows.iter().map(|r| (r.address.as_str(), r.label.as_str(), r.seen.as_str())).collect::<Vec<_>>(), vec![(judge, "甲", "w1")]);
        assert!(!eravon_cands::table::carry(&data, WRITER, NOW, old, new).expect("答得出"), "新钥已有表:不搬");
        eravon_cands::table::add(&data, WRITER, NOW, third, eravon_cands::table::Side::Judge, "0x00000000000000000000000000000000000000d2", "乙", "").expect("第三把自己的候选");
        assert!(!eravon_cands::table::carry(&data, WRITER, NOW, old, third).expect("答得出"));
        assert_eq!(eravon_cands::table::list(&data, third).expect("表").len(), 1, "自己那一张照留");
    }

    #[test]
    fn an_item_kept_under_a_former_key_is_listed_and_opens_for_the_new_key() {
        let (data, open) = opened();
        let (old, new, _) = keys(&data, &open);
        let (old, new) = (old.as_str(), new.as_str());
        let s = eravon_vault::store::open_write(&data, WRITER, NOW).expect("可写");
        for (k, id, name, at) in [(old, "g-old", "旧钥那一件", NOW), (new, "g-new", "新钥这一件", NOW + 5)] {
            let d = eravon_goods::draft::Draft { name: name.to_string(), ..eravon_goods::draft::Draft::default() };
            s.put(k, eravon_store::record::Kind::Product, id, &[], &[(eravon_goods::goods::PART_DRAFT.to_string(), d.seal())], at).expect("落档");
        }
        drop(s);
        let read = |me: &str| eravon_goods::goods::list(&data, me, "").expect("列得出").into_iter().map(|r| r.id).collect::<Vec<_>>();
        assert_eq!(read(new), vec!["g-new".to_string()], "记录没交过:只列自己的");
        let carried = eravon_vault::store::read_identity(&data, old).unwrap_or_default();
        eravon_vault::store::change_identity(&data, new, |t| t.take_over(old, &carried)).expect("接任");
        assert_eq!(read(new), vec!["g-new".to_string(), "g-old".to_string()], "交过之后两把钥的都列");
        let v = eravon_goods::goods::view(&data, new, "g-old", NOW).expect("旧钥那一件在新钥下开得了");
        assert_eq!(v.draft.name, "旧钥那一件");
        assert_eq!(eravon_goods::goods::load_edit(&data, new, "g-old").expect("编辑页读得到").name, "旧钥那一件");
        let recs: Vec<String> = eravon_goods::goods::records(&data, new).expect("记录列得出").into_iter().map(|(_, r)| r.id).collect();
        assert_eq!(recs, vec!["g-new".to_string(), "g-old".to_string()], "product-list 缺省列得出旧钥那一件");
        assert_eq!(eravon_goods::goods::record(&data, new, "g-old").expect("product-show 读得到").id, "g-old");
        assert!(matches!(eravon_goods::goods::record(&data, new, "g-none"), Err(eravon_goods::goods::Fault::Missing(_))), "谁都没有的照旧以缺失拒");
    }

    #[test]
    fn an_item_kept_under_a_former_key_is_read_and_written_in_its_cell() {
        use eravon_goods::goods as g1;
        let (data, open) = opened();
        let (old, new, _) = keys(&data, &open);
        let (old, new) = (old.as_str(), new.as_str());
        let s = eravon_vault::store::open_write(&data, WRITER, NOW).expect("可写");
        let d = eravon_goods::draft::Draft { name: "旧钥那一件".to_string(), ..eravon_goods::draft::Draft::default() };
        for (id, draft) in [("g-live", "false"), ("g-draft", "true")] {
            let fields = [("draft".to_string(), draft.to_string()), ("version".to_string(), "1".to_string()), ("listed".to_string(), "true".to_string())];
            s.put(old, eravon_store::record::Kind::Product, id, &fields, &[(g1::PART_DRAFT.to_string(), d.seal())], NOW).expect("落档");
        }
        drop(s);
        let carried = eravon_vault::store::read_identity(&data, old).unwrap_or_default();
        eravon_vault::store::change_identity(&data, new, |t| t.take_over(old, &carried)).expect("接任");
        let signer = eravon_goods::sign::Signer::of(None, new);
        let c = g1::Ctx { data: &data, writer: WRITER, now: NOW + 9, me: new, registries: &[], signer: &signer, route: Err(("E_TEST_OFFLINE".into(), "不连网".into())) };
        g1::set_listed(&c, "g-live", false).expect("隐下在那件所在的那一格");
        let edited = eravon_goods::draft::Draft { name: "改过".to_string(), ..d };
        let sv = g1::save(&c, Some("g-live"), &edited).expect("存得了");
        assert!(sv.draft, "缺项存成草稿:{sv:?}");
        g1::delete(&c, "g-draft").expect("删稿删那一格的");
        let r = eravon_vault::store::open_read(&data).expect("可读");
        let live = r.get(old, eravon_store::seam::Box_::Products, "g-live").expect("旧钥那一格仍在");
        assert_eq!((live.field("listed"), live.field("draft"), live.field("editing")), (Some("false"), Some("false"), Some("draft")), "隐下与草稿都落在旧钥那一格,在售版照旧");
        assert!(r.get(new, eravon_store::seam::Box_::Products, "g-live").is_err(), "新钥名下不另存一份");
        assert!(r.get(old, eravon_store::seam::Box_::Products, "g-draft").is_err() && r.get(new, eravon_store::seam::Box_::Products, "g-draft").is_err(), "旧钥那一格的稿删了");
        let rows = g1::list(&data, new, "").expect("列得出");
        assert_eq!(rows.iter().map(|x| (x.id.as_str(), x.draft, x.listed)).collect::<Vec<_>>(), vec![("g-live", false, false)], "列表读在售那一版");
    }

    #[test]
    fn an_edit_of_an_item_under_a_former_key_carries_on_its_version() {
        let (data, open) = opened();
        let (old, new, _) = keys(&data, &open);
        let (old, new) = (old.as_str(), new.as_str());
        let s = eravon_vault::store::open_write(&data, WRITER, NOW).expect("可写");
        let d = eravon_goods::draft::Draft { name: "旧钥那一件".to_string(), ..eravon_goods::draft::Draft::default() };
        s.put(old, eravon_store::record::Kind::Product, "g-old", &[("version".to_string(), "3".to_string()), ("draft".to_string(), "true".to_string())], &[(eravon_goods::goods::PART_DRAFT.to_string(), d.seal())], NOW).expect("落档");
        drop(s);
        let carried = eravon_vault::store::read_identity(&data, old).unwrap_or_default();
        eravon_vault::store::change_identity(&data, new, |t| t.take_over(old, &carried)).expect("接任");
        let signer = eravon_goods::sign::Signer::of(None, new);
        let c = eravon_goods::goods::Ctx { data: &data, writer: WRITER, now: NOW + 9, me: new, registries: &[], signer: &signer, route: Err(("E_TEST_OFFLINE".into(), "不连网".into())) };
        let edited = eravon_goods::draft::Draft { name: "改过".to_string(), ..d };
        let sv = eravon_goods::goods::save(&c, Some("g-old"), &edited).expect("存得了");
        assert_eq!(sv.version, 3, "草稿存版本照旧钥那一份续:{sv:?}");
        let got = eravon_goods::goods::list(&data, new, "").expect("列得出");
        let row = got.iter().find(|r| r.id == "g-old").expect("还在表上");
        assert_eq!((row.name.as_str(), row.version), ("改过", 3), "{got:?}");
    }

    #[test]
    fn what_a_rotation_decided_and_did_not_land_is_finished_by_running_it_again() {
        use eravon_store::settings::{hand_on, HandOn};
        let (data, s, seller, buyer) = both_seats();
        let reach = super::Reach::default();
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let (new, _, _) = super::make_new(&i, WRITER).expect("造得出新钥");
        let new = new.to_ascii_lowercase();
        eravon_vault::accounts::hand_over(&data, &new).expect("新钥接任");
        let home = super::home_chain(&data, &new).expect("家的网");
        let item = |landed: bool| HandOn { chain: home, from: seller.clone(), kind: hand_on::OFFER.into(), id: "p1".into(), digest: format!("0x{}", "ab".repeat(32)), logic: String::new(), how: hand_on::REGISTER.into(), landed };
        eravon_vault::store::change_identity(&data, &new, |t| {
            t.former = vec![seller.clone()];
            t.hand_on = vec![item(false)];
        })
        .expect("记录落得下");
        let j = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &new, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        assert_eq!(super::unfinished(&j).expect("读得出").as_deref(), Some(seller.as_str()), "an item not landed: unfinished, from the former key");
        assert_eq!(super::owed_from(super::record_of(&data, &new).expect("读得出"), &seller), vec![item(false)], "what is owed, as decided");
        assert_eq!(super::other_owed(&data, &new).expect("读得出"), 0);
        eravon_vault::store::change_identity(&data, &new, |t| t.hand_on = vec![item(true)]).expect("记录落得下");
        assert_eq!(super::unfinished(&j).expect("读得出"), None, "every item landed: nothing left");
        let away = eravon_literals::constants::NETWORKS.iter().map(|n| n.standard.chain_id).find(|c| *c != home).expect("另一网");
        eravon_vault::store::change_identity(&data, &new, |t| t.hand_on = vec![HandOn { chain: away, ..item(false) }]).expect("记录落得下");
        assert_eq!(super::unfinished(&j).expect("读得出"), None, "an item decided on another network than the home's leaves nothing unfinished");
        assert!(super::owed_from(super::record_of(&data, &new).expect("读得出"), &seller).is_empty(), "nor is it owed");
        eravon_vault::store::change_identity(&data, &new, |t| t.hand_on = vec![item(true)]).expect("记录落得下");
        let other = super::other_seat_key(&data, &new).expect("另一席");
        assert_ne!(other, new);
        let theirs = super::home_chain(&data, &other).expect("另一席的网");
        eravon_vault::store::change_identity(&data, &other, |t| t.hand_on = vec![HandOn { chain: theirs, from: buyer.clone(), kind: hand_on::ORDER.into(), ..item(false) }]).expect("记录落得下");
        assert_eq!(super::unfinished(&j).expect("读得出").as_deref(), Some(seller.as_str()), "the other seat owes: unfinished");
        assert_eq!(super::other_owed(&data, &new).expect("读得出"), 1);
        let held = vec!["0x00000000000000000000000000000000000000aa".to_string()];
        let k = super::In { coins: Some(&held), ..super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &new, now: NOW, let_expire: &[], revoke: &[], coins: None } };
        let went = super::continue_on(&k, WRITER);
        assert!(!matches!(&went, Err(crate::fault::Fault::Down { token, .. }) if token == super::E_ROTATE_HELD), "a rotation left part way goes on past the gate: {went:?}");
        eravon_vault::store::change_identity(&data, &new, |t| t.hand_on = vec![HandOn { kind: "bogus".into(), ..item(false) }]).expect("记录落得下");
        let f = super::unfinished(&j).expect_err("读不成");
        assert!(matches!(&f, crate::fault::Fault::Down { token, .. } if token == super::E_ROTATE_RECORD), "{f:?}");
        assert!(super::listed(&j).is_err(), "the list is refused too");
    }

    #[test]
    fn a_rotation_walks_the_homes_network_alone_and_refuses_nodes_given_for_another() {
        let (data, s, seller, _) = both_seats();
        let home = super::home_chain(&data, &seller).expect("家的网");
        assert_eq!(super::nets_of(&data, &seller).iter().map(|n| n.standard.chain_id).collect::<Vec<_>>(), vec![home], "the home's network alone, whatever the node table holds");
        let away = eravon_literals::constants::NETWORKS.iter().map(|n| n.standard.chain_id).find(|c| *c != home).expect("另一网");
        let table = eravon_chain::endpoints::Table::new(vec!["http://127.0.0.1:18599".to_string()]).expect("节点表");
        let reach = super::Reach { given: Some((away, table, crate::home::Source::Inline)), ..super::Reach::default() };
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let before = eravon_vault::store::keys_of(&data, &seller).unwrap_or_default().len();
        let f = super::continue_on(&i, WRITER).expect_err("另一网的节点");
        assert!(matches!(&f, crate::fault::Fault::Down { token, .. } if token == super::E_ROTATE_OFF_NETWORK), "{f:?}");
        assert!(matches!(super::listed(&i), Err(crate::fault::Fault::Down { token, .. }) if token == super::E_ROTATE_OFF_NETWORK), "the list too");
        assert_eq!(eravon_vault::store::keys_of(&data, &seller).unwrap_or_default().len(), before, "nothing made");
    }

    #[test]
    fn a_seller_key_holding_tokens_refuses_the_rotation_and_makes_nothing() {
        let (data, s, seller, buyer) = both_seats();
        let reach = super::Reach::default();
        let held = vec!["0x00000000000000000000000000000000000000aa".to_string()];
        let keys = || eravon_vault::store::keys_of(&data, &seller).unwrap_or_default().len();
        let before = keys();
        for (seat, old) in [(Seat::Seller, &seller), (Seat::Buyer, &buyer)] {
            let i = super::In { data: &data, s: &s, reach: &reach, seat, old, now: NOW, let_expire: &[], revoke: &[], coins: Some(&held) };
            let f = super::continue_on(&i, WRITER).expect_err("钥上有币");
            assert!(matches!(&f, crate::fault::Fault::Down { token, .. } if token == super::E_ROTATE_HELD), "{seat:?}: {f:?}");
            assert_eq!(keys(), before, "{seat:?}: nothing made");
        }
        let i = super::In { data: &data, s: &s, reach: &reach, seat: Seat::Seller, old: &seller, now: NOW, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let f = super::continue_on(&i, WRITER).expect_err("读不到链");
        assert!(!matches!(&f, crate::fault::Fault::Down { token, .. } if token == super::E_ROTATE_HELD), "nothing held, nothing read: past the gate: {f:?}");
    }
}

fn send(w: &Wire, o: &crate::send::Opts, from: &str, key: &[u8; 32], call: &a1::call::Call, value: &str) -> R<String> {
    let k = *key;
    let key_of = move || -> R<[u8; 32]> { Ok(k) };
    let d = crate::send::deliver_key(&key_of, o, w, from, call, value)?;
    match d.receipt {
        Ok(Some(r)) if r.status == 1 => Ok(d.tx_hash),
        Ok(Some(_)) => Err(Fault::Reverted { tx_hash: d.tx_hash }),
        Ok(None) => Err(Fault::Unconfirmed { tx_hash: d.tx_hash, why: "没有等到回执".into() }),
        Err(f) => Err(f),
    }
}

fn paid_by_old(w: &Wire, o: &crate::send::Opts, (old, old_key): (&str, &[u8; 32]), (new, new_key): (&str, &[u8; 32]), call: &a1::call::Call, sent: &mut Vec<(&'static str, u64, String)>) -> R<String> {
    let (terms, most) = crate::send::pinned(o, w, new, call)?;
    let need = k2::u256::U256::from_dec(&most).ok_or_else(|| Fault::State(format!("最多要花的费 {most:?} 读不成整数")))?;
    let have = k2::u256::U256::from_dec(&eravon_chain::read::balance(&w.t, &w.table, new, "latest")?.0).unwrap_or(k2::u256::U256::ZERO);
    if need.cmp_u(&have) == std::cmp::Ordering::Greater {
        let lack = need.overflowing_sub(&have).0;
        let to = a1::call::Call { to: new.to_string(), data: Vec::new() };
        sent.push(("fee", w.chain_id, send(w, o, old, old_key, &to, &lack.to_dec())?));
    }
    send(w, &terms, new, new_key, call, "0")
}

pub fn seat_pairs(data: &str, old: &str, new: &str) -> R<Vec<(Seat, String, String)>> {
    let pairs = eravon_vault::accounts::rotation_pairs(data, old, new).map_err(|e| Fault::State(e.detail()))?;
    Ok(pairs.into_iter().map(|(s, a, b)| (s, a.to_ascii_lowercase(), b.to_ascii_lowercase())).collect())
}

pub fn continue_on(i: &In<'_>, writer: &str) -> R<Done> {
    on_home(i)?;
    if eravon_vault::accounts::settle_words(i.data, i.old).map_err(|e| Fault::State(e.detail()))? {
        let former = eravon_vault::store::keys_of(i.data, i.old).ok().and_then(|k| k.into_iter().skip(1).last());
        return Ok(Done { new: i.old.to_ascii_lowercase(), finished: former, ..Done::default() });
    }
    if let Some(former) = eravon_vault::accounts::waiting_other(i.data, i.old) {
        let j = In { data: i.data, s: i.s, reach: i.reach, seat: i.seat, old: &former, now: i.now, let_expire: i.let_expire, revoke: i.revoke, coins: i.coins };
        let mut done = hand_on(&j, i.old.to_ascii_lowercase(), None, Vec::new(), writer)?;
        done.finished = Some(former.clone());
        return Ok(done);
    }
    if let Some(former) = unfinished(i)? {
        let j = In { data: i.data, s: i.s, reach: i.reach, seat: i.seat, old: &former, now: i.now, let_expire: i.let_expire, revoke: i.revoke, coins: i.coins };
        let mut done = hand_on(&j, i.old.to_ascii_lowercase(), None, Vec::new(), writer)?;
        done.finished = Some(former.clone());
        return Ok(done);
    }
    held_gate(i)?;
    let (new, phrase) = resume_new(i, writer)?;
    let (phrase, left) = phrase.map_or((None, Vec::new()), |(p, l)| (p, l));
    hand_on(i, new, phrase, left, writer)
}

fn resume_new(i: &In<'_>, writer: &str) -> R<(String, Option<(Option<String>, Vec<Left>)>)> {
    match pending_new(i) {
        Some(p) => Ok((p, None)),
        None => {
            seat_list(i, None)?;
            let (a, p, left) = make_new(i, writer)?;
            Ok((a, Some((p, left))))
        }
    }
}


pub fn part_way(data: &str, me: &str, from: &str, l: &Checklist) -> R<bool> {
    let left = eravon_vault::accounts::rotation_left(data, me).map_err(|e| Fault::Down { layer: "rotate", token: e.name().to_string(), why: e.detail() })?;
    Ok(left || !from.eq_ignore_ascii_case(me) || l.chains.iter().any(|c| matches!(c.seg, Seg::Claim(_))) || !l.unswept.is_empty() || !l.owed.is_empty() || l.owed_other > 0)
}

pub fn listed(i: &In<'_>) -> R<(String, Option<String>, Checklist)> {
    on_home(i)?;
    let found = match eravon_vault::accounts::waiting_other(i.data, i.old) {
        Some(f) => Some(f),
        None => unfinished(i)?,
    };
    if let Some(former) = found {
        let j = In { data: i.data, s: i.s, reach: i.reach, seat: i.seat, old: &former, now: i.now, let_expire: i.let_expire, revoke: i.revoke, coins: i.coins };
        let l = checklist(&j, Some(i.old))?;
        return Ok((former, Some(i.old.to_ascii_lowercase()), l));
    }
    let new = pending_new(i);
    let l = checklist(i, new.as_deref())?;
    Ok((i.old.to_ascii_lowercase(), new, l))
}

fn unfinished(i: &In<'_>) -> R<Option<String>> {
    let formers = eravon_vault::store::keys_of(i.data, i.old).map_err(|e| named(E_ROTATE_RECORD, format!("{} 的设置读不成,说不出换钥做完没有:{}:{}", i.old, e.name(), e.detail())))?;
    let direct = formers.last().cloned().filter(|f| formers.len() > 1 && held(i, f));
    if let Some(f) = &direct {
        let mine = record_of(i.data, i.old)?.iter().any(|h| h.from.eq_ignore_ascii_case(f) && !h.landed);
        if mine || other_owed(i.data, i.old)? > 0 {
            return Ok(Some(f.clone()));
        }
    }
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let mut found = None;
    let nodes = nodes_of(i);
    for f in formers.iter().skip(1) {
        if !held(i, f) {
            continue;
        }
        let (mut moved, mut left) = (true, false);
        for n in nets(i) {
            let Ok(Some(w)) = wire_on(i.reach, i.data, &nodes, n.standard.chain_id) else { continue };
            match seg_of(&w, f, Some(i.old)) {
                Seg::Confirmed => {
                    moved = true;
                    if direct.as_ref() == Some(f) && !swept_on(i.data, i.old, f, n.standard.chain_id) {
                        left = true;
                    }
                }
                Seg::Open => left = true,
                Seg::Claim(p) if p.eq_ignore_ascii_case(i.old) => left = true,
                _ => {}
            }
        }
        if moved && left {
            found = Some(f.clone());
            break;
        }
    }
    restore(home);
    Ok(found)
}

fn record_of(data: &str, key: &str) -> R<Vec<eravon_store::settings::HandOn>> {
    let home = home_chain(data, key);
    match eravon_vault::store::read_identity(data, key) {
        Ok(t) => Ok(t.hand_on.into_iter().filter(|h| Some(h.chain) == home).collect()),
        Err(eravon_store::fault::Fault::Missing(_)) => Ok(Vec::new()),
        Err(e) => Err(named(E_ROTATE_RECORD, format!("{key} 换钥留下的记录读不成,说不出换钥做完没有:{}:{}", e.name(), e.detail()))),
    }
}

fn owed_from(record: Vec<eravon_store::settings::HandOn>, from: &str) -> Vec<eravon_store::settings::HandOn> {
    record.into_iter().filter(|h| h.from.eq_ignore_ascii_case(from) && !h.landed).collect()
}

fn other_seat_key(data: &str, key: &str) -> Option<String> {
    eravon_vault::accounts::other_seat(data, key).map(|k| k.to_ascii_lowercase())
}

fn other_owed(data: &str, key: &str) -> R<usize> {
    let Some(other) = other_seat_key(data, key) else { return Ok(0) };
    Ok(record_of(data, &other)?.iter().filter(|h| !h.landed).count())
}

fn held(i: &In<'_>, key: &str) -> bool {
    match i.s.key_for(key) {
        Ok(mut k) => {
            k.fill(0);
            let _ = std::hint::black_box(&k);
            true
        }
        Err(_) => false,
    }
}

fn nodes_of(i: &In<'_>) -> String {
    eravon_vault::accounts::active(i.data, i.seat).ok().flatten().unwrap_or_else(|| i.old.to_string())
}

fn stranded_of(i: &In<'_>) -> Vec<(String, u64)> {
    let Ok(formers) = eravon_vault::store::keys_of(i.data, i.old) else { return Vec::new() };
    let Some(f) = formers.last().filter(|f| formers.len() > 1 && !held(i, f)) else { return Vec::new() };
    let nodes = nodes_of(i);
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let mut out = Vec::new();
    for n in nets(i) {
        let chain = n.standard.chain_id;
        let Ok(Some(w)) = wire_on(i.reach, i.data, &nodes, chain) else { continue };
        if matches!(seg_of(&w, f, Some(i.old)), Seg::Confirmed) && !swept_on(i.data, i.old, f, chain) {
            out.push((f.clone(), chain));
        }
    }
    restore(home);
    out
}

fn hand_on(i: &In<'_>, new: String, phrase: Option<String>, left: Vec<Left>, writer: &str) -> R<Done> {
    let mut done = Done { new: new.clone(), phrase, left, ..Done::default() };
    let pairs = seat_pairs(i.data, i.old, &new)?;
    let mut first: Option<Fault> = None;
    let (mut landed, mut unlanded) = (false, false);
    let mut mine = 0..0;
    let mut wait: Vec<Seat> = Vec::new();
    for (seat, old, newk) in &pairs {
        if *seat != i.seat && passed_by(i.data, old) {
            continue;
        }
        let j = In { data: i.data, s: i.s, reach: i.reach, seat: *seat, old, now: i.now, let_expire: i.let_expire, revoke: i.revoke, coins: i.coins };
        let from = done.chains.len();
        let (took, stop) = continue_key(&j, newk, writer, &mut done);
        if *seat == i.seat {
            mine = from..done.chains.len();
        }
        landed |= took == Took::Landed;
        unlanded |= took == Took::Unlanded;
        let no_chain = matches!(&stop, Some(Fault::Down { token, .. }) if token == E_ROTATE_NO_CHAIN);
        if took == Took::Not && !no_chain && eravon_vault::accounts::next_of(i.data, old).is_some_and(|n| n.eq_ignore_ascii_case(newk)) {
            wait.push(*seat);
        }
        match stop {
            None => {}
            Some(Fault::Down { token, .. }) if *seat != i.seat && token == E_ROTATE_NO_CHAIN => {}
            Some(f) => {
                first.get_or_insert(f);
            }
        }
    }
    if landed && !unlanded {
        let took = if eravon_vault::accounts::same_row(i.data, i.old, &new) { eravon_vault::accounts::hand_over_but(i.data, &new, &wait) } else { eravon_vault::accounts::switch(i.data, i.seat, &new).map(|_| ()) };
        if let Err(e) = took {
            let f = Fault::Down { layer: "rotate", token: e.name().to_string(), why: e.detail() };
            done.left.push(Left::of(&new, None, &f));
            first.get_or_insert(f);
        }
    }
    if let Some((_, old, newk)) = pairs.iter().find(|(seat, ..)| *seat == i.seat) {
        match record_of(i.data, newk) {
            Ok(r) => done.owed = owed_from(r, old),
            Err(f) => done.left.push(Left::of(old, None, &f)),
        }
        match other_owed(i.data, newk) {
            Ok(n) => done.owed_other = n,
            Err(f) => done.left.push(Left::of(newk, None, &f)),
        }
        let mut moved: Vec<u64> = done.chains[mine.clone()].iter().filter(|c| c.seg == Seg::Confirmed).map(|c| c.chain).collect();
        moved.sort_unstable();
        moved.dedup();
        done.unswept = moved.into_iter().filter(|c| !swept_on(i.data, newk, old, *c)).collect();
    }
    done.backup_again = eravon_vault::accounts::backup_again(i.data, &new);
    match first {
        Some(_) if !done.sent.is_empty() => Ok(done),
        Some(f) => Err(f),
        None => Ok(done),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Took {
    Not,
    Landed,
    Unlanded,
}

fn continue_key(i: &In<'_>, new: &str, writer: &str, done: &mut Done) -> (Took, Option<Fault>) {
    let from = done.chains.len();
    let mut stops = continue_chains(i, new, writer, done);
    for (chain, f) in &stops {
        done.left.push(Left::of(i.old, *chain, f));
    }
    let mut took = Took::Not;
    if done.chains[from..].iter().any(|c| c.seg == Seg::Confirmed) {
        let mut carried = eravon_vault::store::read_identity(i.data, i.old).unwrap_or_default();
        eravon_payways::ways::carry(&mut carried, i.old);
        match eravon_vault::store::change_identity(i.data, new, |t| t.take_over(i.old, &carried)) {
            Ok(()) => {
                took = Took::Landed;
                if let Err(f) = eravon_cands::table::carry(i.data, writer, i.now, i.old, new) {
                    done.left.push(Left { what: i.old.to_string(), chain: None, name: f.name().to_string(), detail: f.detail() });
                }
            }
            Err(e) => {
                took = Took::Unlanded;
                let f = Fault::State(format!("新钥 {new} 接任 {} 的记录没有写成:{}", i.old, e.detail()));
                done.left.push(Left::of(i.old, None, &f));
                stops.push((None, f));
            }
        }
    }
    (took, stops.into_iter().next().map(|(_, f)| f))
}

fn continue_chains(i: &In<'_>, new: &str, writer: &str, done: &mut Done) -> Vec<(Option<u64>, Fault)> {
    let home = eravon_vault::store::network(i.data, i.old).ok();
    let got = (|| -> Result<Vec<(Option<u64>, Fault)>, (Option<u64>, Fault)> {
        let list = seat_list(i, Some(new)).map_err(|f| (None, f))?;
        let old_key = i.s.key_for(i.old).map_err(|e| (None, Fault::State(e.detail())))?;
        let new_key = i.s.key_for(new).map_err(|e| (None, Fault::State(e.detail())))?;
        let to = signed_to(&new_key, i.old).map_err(|f| (None, f))?;
        Ok(each_chain(&list.chains, |c| {
            let mut at = c.clone();
            let step = (|| -> R<()> {
                let Some(w) = wire_on(i.reach, i.data, &nodes_of(i), c.chain)? else { return Ok(()) };
                at.seg = seg_of(&w, i.old, Some(new));
                if let Seg::Moved(k) = &at.seg {
                    return Err(named(E_ROTATE_MOVED, format!("链 {} 上 {} 的记录已交给 {k},它不再是现行钥", c.chain, i.old)));
                }
                if matches!(at.seg, Seg::Open | Seg::Claim(_)) && !matches!(&at.seg, Seg::Claim(p) if p.eq_ignore_ascii_case(new)) {
                    if a1::chain::entered(&w.t, &w.table, new)?.value {
                        return Err(named(E_ROTATE_ENTERED, format!("新钥 {new} 在链 {} 的换钥簿上已经有记录,不能被声明", c.chain)));
                    }
                    done.sent.push(("declare", c.chain, send(&w, &i.reach.opts, i.old, &old_key, &a1::call::declare(&to)?, "0")?));
                }
                at.seg = seg_of(&w, i.old, Some(new));
                if matches!(at.seg, Seg::Claim(_)) {
                    let accede = a1::call::accede(i.old)?;
                    let tx = paid_by_old(&w, &i.reach.opts, (i.old, &old_key), (new, &new_key), &accede, &mut done.sent)?;
                    done.sent.push(("accede", c.chain, tx));
                    at.seg = seg_of(&w, i.old, Some(new));
                    if at.seg != Seg::Confirmed {
                        return Err(named(E_ROTATE_NOT_LANDED, format!("链 {} 上接任之后 current({}) 不是新钥", c.chain, i.old)));
                    }
                }
                if at.seg == Seg::Confirmed {
                    register(&w, i, &list, (new, &new_key), &old_key, writer, done);
                    fund_new(&w, i, new, done);
                    sweep(&w, i, (i.old, &old_key), new, done);
                }
                Ok(())
            })();
            done.chains.push(at);
            step
        }))
    })();
    restore(home);
    got.unwrap_or_else(|stop| vec![stop])
}

fn each_chain(chains: &[OnChain], mut step: impl FnMut(&OnChain) -> R<()>) -> Vec<(Option<u64>, Fault)> {
    chains
        .iter()
        .filter_map(|c| {
            let _own = eravon_chain::action::open(eravon_chain::action::keeper());
            step(c).err().map(|f| (Some(c.chain), f))
        })
        .collect()
}

fn fund_new(w: &Wire, i: &In<'_>, new: &str, done: &mut Done) {
    if i.seat != Seat::Seller {
        return;
    }
    let Ok(ways) = eravon_payways::ways::read(i.data, i.old) else { return };
    if ways.own {
        return;
    }
    let fund = ways.fund;
    if fund.is_empty() || fund.eq_ignore_ascii_case(new) {
        return;
    }
    let got = (|| -> R<Option<String>> {
        let old_bound = fund.eq_ignore_ascii_case(i.old) || a1::chain::funds(&w.t, &w.table, &w.logic, i.old, &fund)?.value;
        if !old_bound || a1::chain::funds(&w.t, &w.table, &w.logic, new, &fund)?.value {
            return Ok(None);
        }
        let call = a1::call::accept_funding(&w.logic, new)?;
        let Ok(fund_key) = i.s.key_for(&fund) else {
            done.funds.push((w.chain_id, fund.to_ascii_lowercase(), call));
            return Ok(None);
        };
        if fund.eq_ignore_ascii_case(i.old) {
            return Ok(Some(send(w, &i.reach.opts, &fund, &fund_key, &call, "0")?));
        }
        let old_key = i.s.key_for(i.old).map_err(|e| Fault::State(e.detail()))?;
        Ok(Some(paid_by_old(w, &i.reach.opts, (i.old, &old_key), (&fund, &fund_key), &call, &mut done.sent)?))
    })();
    match got {
        Ok(Some(tx)) => done.sent.push(("acceptFunding", w.chain_id, tx)),
        Ok(None) => {}
        Err(f) => done.left.push(Left::of(&fund, Some(w.chain_id), &f)),
    }
}

fn plan_on(w: &Wire, i: &In<'_>, list: &Checklist, new: &str, done: &mut Done) -> Option<Vec<eravon_store::settings::HandOn>> {
    use eravon_store::settings::{hand_on, HandOn};
    let mine = |h: &HandOn| h.chain == w.chain_id && h.from.eq_ignore_ascii_case(i.old);
    let record = match record_of(i.data, new) {
        Ok(r) => r,
        Err(f) => {
            done.left.push(Left::of(i.old, Some(w.chain_id), &f));
            return None;
        }
    };
    let kept: Vec<HandOn> = record.into_iter().filter(mine).collect();
    let decided = |kind: &str, id: &str| kept.iter().any(|h| h.kind == kind && h.id == id);
    let chosen = |v: &[String], id: &str| v.iter().any(|x| x == id);
    for h in &kept {
        let asked = match h.kind.as_str() {
            hand_on::ORDER if chosen(i.let_expire, &h.id) => hand_on::EXPIRE,
            hand_on::OFFER | hand_on::TERMS if chosen(i.revoke, &h.id) => hand_on::REVOKE,
            _ => continue,
        };
        if h.how != asked && !done.left.iter().any(|l| l.what == h.id && l.name == E_ROTATE_DECIDED) {
            done.left.push(Left { what: h.id.clone(), chain: Some(h.chain), name: E_ROTATE_DECIDED.to_string(), detail: format!("上一趟已定为 {},照记录办", h.how) });
        }
    }
    let from = i.old.to_ascii_lowercase();
    let item = |kind: &str, id: &str, digest: &[u8; 32], logic: &str, how: &str| HandOn { chain: w.chain_id, from: from.clone(), kind: kind.into(), id: id.into(), digest: a1::order::hex_of(digest), logic: logic.to_ascii_lowercase(), how: how.into(), landed: how == hand_on::EXPIRE };
    let mut plan: Vec<HandOn> = Vec::new();
    for (id, _, d, _) in list.orders.iter().filter(|(id, c, ..)| *c == w.chain_id && !decided(hand_on::ORDER, id)) {
        plan.push(item(hand_on::ORDER, id, d, "", if chosen(i.let_expire, id) { hand_on::EXPIRE } else { hand_on::REGISTER }));
    }
    let product_decided = |id: &str| decided(hand_on::TERMS, id) || decided(hand_on::OFFER, id);
    for (id, _, h, logic, _) in list.terms.iter().filter(|(id, c, ..)| *c == w.chain_id && chosen(i.revoke, id) && !product_decided(id)) {
        plan.push(item(hand_on::TERMS, id, h, logic, hand_on::REVOKE));
    }
    for (id, _, d, _) in list.offers.iter().filter(|(id, c, ..)| *c == w.chain_id && !chosen(i.revoke, id) && !product_decided(id)) {
        plan.push(item(hand_on::OFFER, id, d, "", hand_on::REGISTER));
    }
    if plan.is_empty() {
        return Some(kept);
    }
    match eravon_vault::store::change_identity(i.data, new, |t| t.hand_on.extend(plan.iter().cloned())) {
        Ok(()) => Some(kept.into_iter().chain(plan).collect()),
        Err(e) => {
            done.left.push(Left::of(i.old, Some(w.chain_id), &named(E_ROTATE_RECORD, format!("换钥要登记的单没能记进本机记录,一笔也没发:{}", e.detail()))));
            None
        }
    }
}

fn landed(i: &In<'_>, new: &str, h: &eravon_store::settings::HandOn, tx: &str, done: &mut Done) {
    let same = |x: &eravon_store::settings::HandOn| x.chain == h.chain && x.from == h.from && x.kind == h.kind && x.id == h.id && x.digest == h.digest;
    let marked = eravon_vault::store::change_identity(i.data, new, |t| {
        for x in t.hand_on.iter_mut().filter(|x| same(x)) {
            x.landed = true;
        }
    });
    if let Err(e) = marked {
        done.left.push(Left::of(&h.id, Some(h.chain), &named(E_ROTATE_RECORD, format!("链上已办({tx}),本机记录没记成:{}", e.detail()))));
    }
}

fn register(w: &Wire, i: &In<'_>, list: &Checklist, (new, new_key): (&str, &[u8; 32]), old_key: &[u8; 32], writer: &str, done: &mut Done) {
    use eravon_store::settings::hand_on;
    let Some(plan) = plan_on(w, i, list, new, done) else { return };
    for h in &plan {
        if h.how == hand_on::EXPIRE && !done.let_expire.contains(&h.id) {
            done.let_expire.push(h.id.clone());
        }
    }
    for h in plan.iter().filter(|h| !h.landed) {
        let Ok(digest) = a1::order::b32_of("digest", &h.digest) else {
            done.left.push(Left::of(&h.id, Some(w.chain_id), &named(E_ROTATE_RECORD, format!("本机记录里的摘要读不成:{}", h.digest))));
            continue;
        };
        let got = (|| -> R<String> {
            if h.how == hand_on::REVOKE {
                if a1::chain::revoked(&w.t, &w.table, &h.logic, i.old, &digest)?.value {
                    return Ok(String::new());
                }
                let call = a1::call::revoke(&h.logic, i.old, &digest)?;
                return paid_by_old(w, &i.reach.opts, (i.old, old_key), (new, new_key), &call, &mut done.sent);
            }
            if a1::consent::registered(&w.t, &w.table, &w.logic, i.old, &digest).state == a1::consent::Registration::Held {
                return Ok(String::new());
            }
            let call = a1::call::consent(&w.logic, i.old, &digest)?;
            paid_by_old(w, &i.reach.opts, (i.old, old_key), (new, new_key), &call, &mut done.sent)
        })();
        match got {
            Ok(tx) => {
                if !tx.is_empty() {
                    if h.how == hand_on::REVOKE {
                        if let Err(e) = eravon_goods::goods::revoked_recorded(i.data, i.old, writer, i.now, &h.id, &tx) {
                            done.left.push(Left::of(&h.id, Some(w.chain_id), &Fault::State(format!("链上已下架({tx}),本机记录没写成:{}", e.detail()))));
                        }
                        done.sent.push(("revoke", w.chain_id, tx.clone()));
                    } else {
                        done.sent.push(("consent", w.chain_id, tx.clone()));
                    }
                }
                landed(i, new, h, &tx, done);
            }
            Err(f) => done.left.push(Left::of(&h.id, Some(w.chain_id), &f)),
        }
    }
}

pub fn fresh(i: &In<'_>, writer: &str) -> R<Done> {
    let (new, phrase, left) = add_new(i, writer)?;
    eravon_vault::accounts::switch(i.data, i.seat, &new).map_err(|e| Fault::State(e.detail()))?;
    Ok(Done { new, phrase: Some(phrase), left, ..Done::default() })
}

pub fn company_plans(old: &str, new: &str) -> R<(a1::proposal::Plan, a1::proposal::Plan)> {
    let book_abi = || a1::proposal::abi_doc(&[a1::sol::SUCCESSION_DECLARE, a1::sol::SUCCESSION_ACCEDE], "succession-book.abi.json");
    let one = |verb: &'static str, call: a1::call::Call| -> R<a1::proposal::Plan> {
        Ok(a1::proposal::Plan { verb, calls: vec![a1::proposal::of_call(&call)?], originals: vec![book_abi()?], items: Vec::new(), face: Vec::new(), instance: k7::SUCCESSION_BOOK.to_ascii_lowercase() })
    };
    Ok((one("declare", a1::call::declare(new)?)?, one("accede", a1::call::accede(old)?)?))
}
