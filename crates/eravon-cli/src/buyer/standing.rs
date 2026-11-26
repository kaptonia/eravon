use super::*;

pub type Why = (String, String);

pub struct Standing {
    pub book: Result<(), Why>,
    pub place: Result<(), Why>,
    pub revoked: Result<(), Why>,
    pub may_open: Vec<(usize, bool, String)>,
}

pub fn standing(w: &eravon_watch::Wire<'_>, listing: &[u8], p: &k3::listing::Listing, buyer: &str) -> Standing {
    let a1w = |f: a1::fault::Fault| (f.name().to_string(), f.detail());
    let book = eravon_scan::records::book(w.t, w.table)
        .map_err(|f| (f.name().to_string(), f.detail()))
        .and_then(|_| a1::chain::current(w.t, w.table, &p.order.seller_key).map(|_| ()).map_err(a1w));
    let revoked = a1::order::b32_of("termsHash", &p.terms_hash)
        .and_then(|th| a1::chain::revoked(w.t, w.table, &p.logic, &p.order.seller_key, &th))
        .map_err(a1w)
        .and_then(|r| match r.value {
            true => Err((E_REVOKED.to_string(), format!("卖家已在逻辑合约上撤下文书 {}", p.terms_hash))),
            false => Ok(()),
        });
    Standing { book, place: place(w, listing), revoked, may_open: may_open(w, p, buyer) }
}

impl Standing {
    pub fn out(&self) -> Out {
        let step = |r: &Result<(), Why>| match r {
            Ok(()) => Out::O(vec![("ok", Out::B(true))]),
            Err((n, d)) => Out::O(vec![("detail", Out::S(d.clone())), ("name", Out::S(n.clone())), ("ok", Out::B(false))]),
        };
        Out::O(vec![
            ("book", step(&self.book)),
            ("mayOpen", Out::A(self.may_open.iter().map(|(i, ok, why)| Out::O(vec![("candidate", Out::N(*i as u64)), ("ok", Out::B(*ok)), ("why", Out::S(why.clone()))])).collect())),
            ("place", step(&self.place)),
            ("revoked", step(&self.revoked)),
        ])
    }
}

pub fn stands(w: &eravon_watch::Wire<'_>, listing: &[u8], p: &k3::listing::Listing, buyer: &str, n: u64) -> R<()> {
    let s = standing(w, listing, p, buyer);
    let down = |(token, why): Why| Fault::Down { layer: "P3", token, why };
    s.book.map_err(down)?;
    s.place.map_err(down)?;
    s.revoked.map_err(down)?;
    match s.may_open.into_iter().find(|(i, _, _)| *i as u64 == n) {
        Some((_, false, why)) => Err(down((E_OFFER_RETIRED.into(), why))),
        _ => Ok(()),
    }
}

pub fn sign_gate(w: &crate::shared::Wire, listing: &[u8], p: &k3::listing::Listing, buyer: &str, n: u64) -> R<()> {
    super::actions::must_pass(&super::candidate_gate(Some(w), p), n)?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    stands(&sw, listing, p, buyer, n)
}

pub fn listing_of_pack(pack: &[u8]) -> R<(Vec<u8>, k3::listing::Listing, u64)> {
    let pk = k3::pack::pack(pack).map_err(Fault::from)?;
    let v = eravon_galeed::canon::accept(pack).map_err(|t| Fault::Arg(format!("订单文件读不成:{t:?}")))?;
    let bytes = v.get("listing").and_then(|l| eravon_galeed::canon::canonical(l).ok()).ok_or_else(|| Fault::Arg("订单文件里没有清单".into()))?;
    Ok((bytes, pk.listing, pk.receipt.candidate))
}

pub const E_OFFER_RETIRED: &str = "E_P3_OFFER_RETIRED";

pub const E_REVOKED: &str = "E_A1_REVOKED";

pub fn place(w: &eravon_watch::Wire<'_>, listing: &[u8]) -> Result<(), Why> {
    let refuse = |token: String, why: &str| (token, why.to_string());
    match eravon_scan::records::pay_check(w.t, w.table, w.chain_id, listing).map_err(|e| (e.name().to_string(), e.detail()))? {
        Ok(eravon_link::identity::Pay::Payable) => Ok(()),
        Ok(v) => Err(refuse(v.word().to_string(), match v {
            eravon_link::identity::Pay::Unplaced => "卖家没有自己把这份清单登上公告板",
            _ => "这份清单是卖家钥交出记录之后才登的板",
        })),
        Err(v) => Err(refuse(v.name.to_string(), "清单不合 eravon.link/1 §2.5")),
    }
}


pub fn may_open(w: &eravon_watch::Wire<'_>, p: &k3::listing::Listing, buyer: &str) -> Vec<(usize, bool, String)> {
    let n = p.candidate_count();
    if !p.order.buyer_may_open {
        return (0..n).map(|i| (i, true, String::new())).collect();
    }
    let now = match a1::chain::current(w.t, w.table, &p.order.seller_key) {
        Ok(r) => r.value,
        Err(f) => return (0..n).map(|i| (i, false, format!("{} · {}", f.name(), f.detail()))).collect(),
    };
    let terms = p.terms_text.as_bytes();
    (0..n)
        .map(|i| {
            let o = match shared::order_of(terms, i as u64, buyer, buyer, 0, a1::order::NO_PREV) {
                Ok(o) => o,
                Err(f) => return (i, false, format!("{} · {}", f.name(), f.detail())),
            };
            let digest = o.offer.offer_digest(w.chain_id, &p.logic);
            let sig = a1::order::bytes_of("要约签名", p.candidate_sig(i)).unwrap_or_default();
            if !a1::consent::fit(&now, &digest, &sig).is_empty() {
                return (i, true, String::new());
            }
            let r = a1::consent::registered(w.t, w.table, &p.logic, &p.order.seller_key, &digest);
            match r.state {
                a1::consent::Registration::Held => (i, true, String::new()),
                a1::consent::Registration::NotHeld => (i, false, "E_P3_OFFER_RETIRED · 要约是卖家换下的旧钥签的,登记簿也没有它:开不了案".to_string()),
                a1::consent::Registration::Unverified => (i, false, format!("E_P3_OFFER_RETIRED · 要约是卖家换下的旧钥签的,登记簿上有没有它核不实:{}", r.why)),
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub enum Gate {
    Go,
    AwaitStage(eravon_cands::act::Refused),
    Revoked(eravon_cands::act::Refused),
    DepositShort(eravon_cands::act::Refused),
    Stale(eravon_cands::act::Refused),
    Refused(eravon_cands::act::Refused),
}

impl Gate {
    pub fn signs(&self) -> bool {
        matches!(self, Gate::Go | Gate::AwaitStage(_) | Gate::DepositShort(_))
    }

    pub fn pays(&self) -> bool {
        matches!(self, Gate::Go | Gate::AwaitStage(_))
    }

    pub fn refusal(&self) -> Option<&eravon_cands::act::Refused> {
        match self {
            Gate::Go => None,
            Gate::AwaitStage(r) | Gate::Revoked(r) | Gate::DepositShort(r) | Gate::Stale(r) | Gate::Refused(r) => Some(r),
        }
    }
}

pub fn must_pay(w: &eravon_watch::Wire<'_>, o: &a1::order::Order) -> R<()> {
    let gate = pay_gate(w, o);
    match gate.refusal() {
        Some(r) if !gate.pays() => Err(Fault::Down { layer: "V2", token: r.name.clone(), why: format!("{}:{}", r.gate, r.why) }),
        _ => Ok(()),
    }
}

pub fn pay_gate(w: &eravon_watch::Wire<'_>, o: &a1::order::Order) -> Gate {
    if let Err(f) = a1::precheck::not_revoked(w.t, w.table, o, w.logic) {
        return Gate::Revoked(eravon_cands::act::Refused { gate: a1::precheck::GATE_REVOKED, name: f.name(), why: f.detail() });
    }
    match eravon_watch::act::check(w, o) {
        Ok(_) => Gate::Go,
        Err(r) if stage_gate(&r) => Gate::AwaitStage(r),
        Err(r) if r.gate == a1::precheck::GATE_REVOKED => Gate::Revoked(r),
        Err(r) if deposit_gate(&r) => Gate::DepositShort(r),
        Err(r) if r.gate == a1::precheck::GATE_PRICE => Gate::Stale(r),
        Err(r) => Gate::Refused(r),
    }
}
