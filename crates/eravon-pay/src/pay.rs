use eravon_link as k3;
use eravon_standard as a1;
use eravon_terms as k2;

use crate::check::{self, Chain};
use crate::fault::{Fault, R};

pub struct PayIn<'a> {
    pub c: &'a Chain,
    pub s: &'a eravon_vault::session::Session,
    pub link: &'a [u8],
    pub listing: &'a [u8],
    pub p: &'a k3::listing::Listing,
    pub candidate: usize,
    pub payout: &'a str,
    pub writer: &'a str,
    pub attachments: &'a [String],
    pub stop: &'a dyn Fn() -> bool,
}

#[derive(Clone, Debug)]
pub enum Stop {
    Short { address: String, gap: String, decimals: Option<u64> },
    Stale(Fault),
    Revoked(Fault),
    DepositShort(Fault),
    AwaitSeller { pay_tx: String },
    AwaitStage { pay_tx: String, prev: String },
    Opened { pay_tx: String, open_tx: String },
    Locked { pay_tx: String, funded: bool },
}

#[derive(Clone, Debug)]
pub struct Paid {
    pub buyer_key: String,
    pub bumped: bool,
    pub instance: String,
    pub case_id: String,
    pub sig: String,
    pub id: String,
    pub stop: Stop,
}

pub fn pay(i: &PayIn<'_>) -> R<Paid> {
    crate::mark::mark(crate::mark::FEATURE);
    let (c, p) = (i.c, i.p);
    let n = i.candidate;
    let me = c.buyer.clone();
    let terms = p.terms_text.as_bytes();
    let open_by = eravon_cli::shared::open_by_at(p.order.expiry, p.order.open_window, c.now)?;
    let prev = eravon_cli::buyer::series_prev(i.s.data(), &me, &me, p, &eravon_cli::buyer::live_on(&c.w, c.now))?;
    let pick = eravon_cli::buyer::key_pick(i.s, &me, &c.w, p, n, i.payout, open_by, Some(c.now), &prev)?;
    let open_by = pick.open_by;
    eravon_cli::buyer::sign_gate(&c.w, i.listing, p, &pick.address, n as u64)?;
    let o = eravon_cli::shared::order_of(terms, n as u64, &pick.address, i.payout, open_by, &prev)?;
    let sw = eravon_watch::Wire { t: &c.w.t, table: &c.w.table, chain_id: c.w.chain_id, logic: &c.w.logic, registries: &c.w.registries };
    let gate = eravon_cli::buyer::pay_gate(&sw, &o);
    let unsigned = |stop: Stop| -> R<Paid> {
        let d = o.derive(c.w.chain_id, &p.logic)?;
        Ok(Paid { buyer_key: pick.address.clone(), bumped: pick.bumped, instance: d.instance.clone(), case_id: eravon_cli::answer::hex(&d.salt), sig: String::new(), id: String::new(), stop })
    };
    match &gate {
        eravon_cli::buyer::Gate::Revoked(r) => return unsigned(Stop::Revoked(Fault::new(r.name.clone(), r.why.clone()))),
        eravon_cli::buyer::Gate::Stale(r) => return unsigned(Stop::Stale(Fault::new(r.name.clone(), r.why.clone()))),
        eravon_cli::buyer::Gate::Refused(r) => return Err(Fault::new(r.name.clone(), format!("{}:{}", r.gate, r.why))),
        _ => {}
    }
    let mut key = eravon_cli::buyer::signing_key(i.s, &me, &pick)?;
    let seq = eravon_cli::buyer::pool_seq_on(&c.w, p, n as u64)?;
    let signed = eravon_cli::buyer::sign_order(&eravon_cli::buyer::SignIn {
        key: &key,
        buyer: &pick.address,
        payout: i.payout,
        link: i.link,
        listing: i.listing,
        p,
        candidate: n as u64,
        seq,
        open_by,
        prev: &prev,
        attachments: i.attachments,
    });
    for b in key.iter_mut() {
        *b = 0;
    }
    let _ = std::hint::black_box(&key);
    let sd = signed?;
    let id = sd.case_id[2..18].to_string();
    let decimals = a1::chain::erc20_decimals(&c.w.t, &c.w.table, &p.token);
    let store = eravon_vault::store::open_write(i.s.data(), i.writer, c.now)?;
    eravon_cli::buyer::keep_order(&store, &me, &id, &sd, decimals, c.now)?;
    drop(store);
    let paid = |stop: Stop| Paid {
        buyer_key: pick.address.clone(),
        bumped: pick.bumped,
        instance: sd.instance.clone(),
        case_id: sd.case_id.clone(),
        sig: sd.sig.clone(),
        id: id.clone(),
        stop,
    };
    let waits = match gate {
        eravon_cli::buyer::Gate::DepositShort(r) => return Ok(paid(Stop::DepositShort(Fault::new(r.name, r.why)))),
        eravon_cli::buyer::Gate::AwaitStage(_) => true,
        _ => false,
    };
    let at_instance = k2::u256::U256::from_be(&a1::chain::erc20_balance(&c.w.t, &c.w.table, &p.token, &sd.instance)?.value);
    let owe = if at_instance.cmp_u(&p.order.amount) == std::cmp::Ordering::Less { p.order.amount.overflowing_sub(&at_instance).0 } else { k2::u256::U256::ZERO };
    let bal = check::balance(c, &p.token)?;
    if let Some(g) = check::gap(&owe, &bal.have) {
        return Ok(paid(Stop::Short { address: me, gap: g.to_dec(), decimals: bal.decimals }));
    }
    if (i.stop)() {
        return Ok(paid(Stop::Locked { pay_tx: String::new(), funded: owe.is_zero() }));
    }
    let pay_note = |tx: &str| if tx.is_empty() { "已足额转入".to_string() } else { tx.to_string() };
    let pay_tx = if owe.is_zero() {
        String::new()
    } else {
        let transfer = a1::call::erc20_transfer(&p.token, &sd.instance, &owe.to_be())?;
        send(i, &me, &transfer, &eravon_wallet::tx::Tag::moving(eravon_wallet::tx::Kind::Pay, &sd.instance, &p.token, &owe.to_dec()))?
    };
    if waits {
        return Ok(paid(Stop::AwaitStage { pay_tx, prev }));
    }
    if !p.order.buyer_may_open {
        return Ok(paid(Stop::AwaitSeller { pay_tx }));
    }
    if (i.stop)() {
        return Ok(paid(Stop::Locked { pay_tx, funded: true }));
    }
    let open_tx = match deploy(i, &pick.address, &sd.sig, open_by, &prev) {
        Ok(tx) => tx,
        Err(f) if f.token == eravon_cli::seller::E_OPEN_OPENED => String::new(),
        Err(f) => return Err(Fault::new(f.token, format!("已转账到 {}({}),开案那一笔没发:{}", sd.instance, pay_note(&pay_tx), f.detail))),
    };
    Ok(paid(Stop::Opened { pay_tx, open_tx }))
}

fn deploy(i: &PayIn<'_>, buyer_key: &str, sig_buyer: &str, open_by: u64, prev: &str) -> R<String> {
    let (c, p, n) = (i.c, i.p, i.candidate);
    let o = eravon_cli::shared::order_of(p.terms_text.as_bytes(), n as u64, buyer_key, i.payout, open_by, prev)?;
    let sig_seller = a1::order::bytes_of("要约签名", p.candidate_sig(n)).unwrap_or_default();
    let sig_buyer = a1::order::bytes_of("买家签名", sig_buyer)?;
    let guild = n.checked_sub(p.judges.len()).and_then(|g| p.guilds.get(g)).map(|g| g.address.to_ascii_lowercase());
    let seq = eravon_cli::buyer::pool_seq_on(&c.w, p, n as u64)?;
    let opened = eravon_cli::seller::open_calls(&eravon_cli::seller::OpenIn {
        w: &c.w,
        o: &o,
        sig_seller: &sig_seller,
        sig_buyer: &sig_buyer,
        from: buyer_key,
        guild: guild.as_deref().map(|g| (g, seq.unwrap_or(0))),
        now: c.now,
    })?;
    if let Some(dep) = &opened.deploy {
        send(i, &c.buyer, dep, &eravon_wallet::tx::Tag::of(eravon_wallet::tx::Kind::Open, &opened.instance))?;
    }
    let tag = eravon_wallet::tx::Tag::of(eravon_wallet::tx::Kind::Open, &opened.instance);
    send(i, &c.buyer, &opened.init, &tag)
}


fn send(i: &PayIn<'_>, who: &str, call: &a1::call::Call, tag: &eravon_wallet::tx::Tag) -> R<String> {
    let s = i.s;
    let k = eravon_wallet::tx::Keep { data: s.data(), writer: i.writer, owner: &i.c.buyer };
    let key = || s.own_key(&i.c.buyer, who).map_err(|e| eravon_cli::fault::Fault::Arg(format!("{}:{}", e.name(), e.detail())));
    eravon_wallet::tx::post(&k, &i.c.w, who, &key, &eravon_cli::send::Opts::default(), call, "0", tag).map_err(|f| Fault::new(f.token, f.detail))
}
