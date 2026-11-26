use super::*;

pub(super) fn s4((token, why): eravon_watch::Why) -> Fault {
    Fault::Down { layer: "S4", token, why }
}

pub(super) fn s5_down((token, why): eravon_watch::Why) -> Fault {
    Fault::Down { layer: "S5", token, why }
}

pub(super) struct CliPost<'a> {
    c: &'a Ctx,
    f: &'a Flags,
    w: &'a shared::Wire,
    who: &'a str,
    rows: Vec<Out>,
    sent: Option<(String, eravon_watch::order::Broadcast)>,
}

impl eravon_watch::send::Post for CliPost<'_> {
    fn from(&self) -> Option<String> {
        Some(self.who.to_string())
    }
    fn post(&mut self, call: &a1::call::Call) -> Result<String, eravon_watch::Why> {
        self.send(call, None, None)
    }
    fn resend(&mut self, call: &a1::call::Call, at: &eravon_watch::order::Broadcast) -> Result<String, eravon_watch::Why> {
        self.send(call, Some(at), None)
    }
    fn post_landed(&mut self, call: &a1::call::Call, land: eravon_watch::send::Land<'_>) -> Result<String, eravon_watch::Why> {
        self.send(call, None, Some(land))
    }
    fn resend_landed(&mut self, call: &a1::call::Call, at: &eravon_watch::order::Broadcast, land: eravon_watch::send::Land<'_>) -> Result<String, eravon_watch::Why> {
        self.send(call, Some(at), Some(land))
    }
    fn broadcast(&self) -> Option<(String, eravon_watch::order::Broadcast)> {
        self.sent.clone()
    }
}

pub(super) fn s6((token, why): eravon_watch::Why) -> Fault {
    Fault::Down { layer: "S6", token, why }
}

pub(super) struct ActPost<'a> {
    c: &'a Ctx,
    f: &'a Flags,
    w: &'a shared::Wire,
    who: &'a str,
    call: Option<a1::call::Call>,
    rows: Vec<(&'static str, Out)>,
    tx: String,
    carried: Option<eravon_watch::order::Broadcast>,
}

impl<'a> ActPost<'a> {
    pub(super) fn new(c: &'a Ctx, f: &'a Flags, w: &'a shared::Wire, who: &'a str) -> ActPost<'a> {
        ActPost { c, f, w, who, call: None, rows: Vec::new(), tx: String::new(), carried: None }
    }

    pub(super) fn finish(self, layer: &'static str, r: Result<String, eravon_watch::Why>) -> R<Vec<(&'static str, Out)>> {
        match r {
            Ok(_) => {
                let mut rows = self.call.as_ref().map(|c| shared::tx_rows(c, "0")).unwrap_or_default();
                rows.extend(self.rows);
                Ok(rows)
            }
            Err((n, _)) if n == NOT_SENT => {
                let mut rows = self.call.as_ref().map(|c| shared::tx_rows(c, "0")).unwrap_or_default();
                rows.push(("sent", Out::B(false)));
                rows.push(("why", Out::S("没给 --send true,故只把这一笔的字节交出来".into())));
                Ok(rows)
            }
            Err((n, _)) if n == "E_V_REVERTED" => Err(Fault::Reverted { tx_hash: self.tx }),
            Err((token, why)) => Err(Fault::Down { layer, token, why }),
        }
    }
}

impl eravon_watch::send::Post for ActPost<'_> {
    fn from(&self) -> Option<String> {
        Some(self.who.to_string())
    }
    fn post(&mut self, call: &a1::call::Call) -> Result<String, eravon_watch::Why> {
        self.send(call, None, None)
    }
    fn resend(&mut self, call: &a1::call::Call, at: &eravon_watch::order::Broadcast) -> Result<String, eravon_watch::Why> {
        self.send(call, Some(at), None)
    }
    fn post_landed(&mut self, call: &a1::call::Call, land: eravon_watch::send::Land<'_>) -> Result<String, eravon_watch::Why> {
        self.send(call, None, Some(land))
    }
    fn resend_landed(&mut self, call: &a1::call::Call, at: &eravon_watch::order::Broadcast, land: eravon_watch::send::Land<'_>) -> Result<String, eravon_watch::Why> {
        self.send(call, Some(at), Some(land))
    }
    fn broadcast(&self) -> Option<(String, eravon_watch::order::Broadcast)> {
        self.carried.clone().filter(|_| !self.tx.is_empty()).map(|b| (self.tx.clone(), b))
    }
}

pub(super) fn order_at(c: &Ctx, who: &str, instance: &str, id: Option<String>, case_id: &str) -> R<eravon_watch::order::Order> {
    if !eravon_chain::seam::address_ok(instance) {
        return Err(Fault::Arg(format!("--instance 要 0x 加四十位十六进制,给的是 {instance:?}")));
    }
    if let Some(o) = eravon_watch::order::all(&c.data, who).map_err(s4)?.into_iter().find(|o| o.instance.eq_ignore_ascii_case(instance)) {
        return Ok(o);
    }
    let seed = case_id.get(2..18).filter(|_| case_id.len() >= 18 && case_id.is_ascii()).unwrap_or(&instance[2..18]);
    Ok(eravon_watch::order::Order {
        opened: 0,
        id: id.unwrap_or_else(|| seed.to_string()),
        case_id: skeleton_case(case_id, instance),
        instance: instance.to_ascii_lowercase(),
        buyer_key: String::new(),
        buyer_payout: String::new(),
        verdict: String::new(),
        token: String::new(),
        need: String::new(),
        open_by: 0,
        candidate: 0,
        expiry: 0,
        state: eravon_watch::order::State::Deployed,
        balance: String::new(),
        gap: String::new(),
        note: String::new(),
        notice: String::new(),
        notice_at: 0,
        deploy_tx: String::new(),
        expire_tx: String::new(),
        source: String::new(),
        extra: Vec::new(),
    })
}

pub(super) fn skeleton_case(case_id: &str, _instance: &str) -> String {
    case_id.to_string()
}

pub(super) fn order_rows(o: &eravon_watch::order::Order) -> Vec<(&'static str, Out)> {
    vec![
        ("balance", Out::S(o.balance.clone())),
        ("caseId", Out::S(o.case_id.clone())),
        ("deployTx", Out::S(o.deploy_tx.clone())),
        ("expireTx", Out::S(o.expire_tx.clone())),
        ("gap", Out::S(o.gap.clone())),
        ("gapNote", Out::S(if o.gap.is_empty() { String::new() } else { eravon_watch::order::GAP_NOTE.to_string() })),
        ("id", Out::S(o.id.clone())),
        ("instance", Out::S(o.instance.clone())),
        ("need", Out::S(o.need.clone())),
        ("note", Out::S(o.note.clone())),
        ("notice", Out::S(o.notice.clone())),
        ("say", Out::S(o.state.say().to_string())),
        ("state", Out::S(o.state.word().to_string())),
        ("verdict", Out::S(o.verdict.clone())),
    ]
}

pub(super) fn watch_order(c: &Ctx, f: &Flags, id: &str) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let mut post = CliPost { c, f, w: &w, who: &who, rows: Vec::new(), sent: None };
    let t = eravon_watch::send::tick(
        &c.data,
        &c.writer,
        c.now,
        &who,
        &sw,
        id,
        if crate::send::asked(f) { eravon_watch::send::Opens::Yes } else { eravon_watch::send::Opens::Read },
        if crate::send::asked(f) { Some(&mut post as &mut dyn eravon_watch::send::Post) } else { None },
    )
    .map_err(s4)?;
    let mut rows = order_rows(&t.order);
    rows.push(("sent", Out::S(t.sent.as_ref().map(|s| s.0.to_string()).unwrap_or_default())));
    rows.push(("txs", Out::A(post.rows)));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn watch_scan(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let from = f.int_or("--from-block", 0).map_err(|_| Fault::Arg("--from-block 要是整数".into()))?;
    let mut post = CliPost { c, f, w: &w, who: &who, rows: Vec::new(), sent: None };
    let s = eravon_watch::scan::startup(
        &c.data,
        &c.writer,
        c.now,
        &who,
        &sw,
        from,
        &[],
        if crate::send::asked(f) { eravon_watch::send::Opens::Yes } else { eravon_watch::send::Opens::Read },
        if crate::send::asked(f) { Some(&mut post as &mut dyn eravon_watch::send::Post) } else { None },
    )
    .map_err(s4)?;
    let mut rows = vec![
        ("enrolled", strs(&s.enrolled)),
        ("foreign", Out::N(s.foreign)),
        ("marked", strs(&s.marked)),
        (
            "failed",
            Out::A(s.failed.iter().map(|(id, (n, d))| Out::O(vec![("detail", Out::S(d.clone())), ("id", Out::S(id.clone())), ("token", Out::S(n.clone()))])).collect()),
        ),
        (
            "ticked",
            Out::A(s.ticked.iter().map(|(id, st)| Out::O(vec![("id", Out::S(id.clone())), ("state", Out::S(st.word().to_string()))])).collect()),
        ),
        (
            "sends",
            Out::A(
                s.sent
                    .iter()
                    .map(|(id, what, tx)| Out::O(vec![("id", Out::S(id.clone())), ("tx", Out::S(tx.clone())), ("what", Out::S((*what).to_string()))]))
                    .collect(),
            ),
        ),
        ("txs", Out::A(post.rows)),
    ];
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn price_rows(c: &Ctx, f: &Flags, w: &shared::Wire) -> R<Vec<(&'static str, Out)>> {
    let Some(at) = f.opt("--price-feed") else {
        return Ok(vec![
            ("priceFeed", Out::S("absent".into())),
            ("priceFeedDoor", Out::S("给 --price-feed 即现读一轮(Chainlink 形)".into())),
            ("priceFeedOwner", Out::S("调用者给价源地址;此处只读一轮,不判新旧".into())),
        ]);
    };
    let r = match a1::feed::observe(&w.t, &w.table, &at)? {
        a1::feed::Seen::Round(r) => r,
        a1::feed::Seen::Anomaly(why) => {
            return Err(a1::fault::Fault::Precheck { gate: a1::precheck::GATE_PRICE, why }.into());
        }
    };
    let age = c.now.saturating_sub(r.updated_at);
    Ok(vec![
        ("priceAge", Out::N(age)),
        ("priceAnswer", Out::S(r.answer.clone())),
        ("priceDecimals", Out::N(r.decimals)),
        ("priceUpdatedAt", Out::N(r.updated_at)),
        ("priceFeed", Out::S(at)),
        ("priceFeedDoor", Out::S(r.agreement.to_string())),
        ("priceFeedOwner", Out::S("调用者给价源地址;此处只读一轮,不判新旧".into())),
    ])
}

pub(super) fn watch(c: &Ctx, f: &Flags) -> R<Out> {
    if let Some(id) = f.opt("--id") {
        return watch_order(c, f, &id);
    }
    if f.flag("--scan") {
        return watch_scan(c, f);
    }
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let terms = home::read_bounded(&f.opt("--terms").ok_or_else(|| Fault::Arg("要 --terms".into()))?)?;
    let o = shared::order_of(
        &terms,
        f.int_or("--candidate", 0).map_err(|_| Fault::Arg("--candidate 要是整数".into()))?,
        &f.opt("--buyer-key").ok_or_else(|| Fault::Arg("要 --buyer-key".into()))?,
        &f.opt("--buyer-payout").ok_or_else(|| Fault::Arg("要 --buyer-payout".into()))?,
        shared::open_by_flag(f)?,
        a1::order::NO_PREV,
    )?;
    let (case_id, instance, _) = shared::derive_rows(&o, w.chain_id, &w.logic)?;
    let got = a1::chain::erc20_balance(&w.t, &w.table, &o.offer.token, &instance)?;
    let want = o.offer.amount.to_dec();
    let have = k2::u256::U256::from_be(&got.value).to_dec();
    let funded = a1::precheck::funded(&w.t, &w.table, &o, &instance).is_ok();
    let mut rows = vec![
        ("agreement", Out::S(got.agreement.to_string())),
        ("balance", Out::S(have)),
        ("caseId", Out::S(case_id)),
        ("funded", Out::B(funded)),
        ("instance", Out::S(instance)),
        ("need", Out::S(want)),
    ];
    rows.extend(price_rows(c, f, &w)?);
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub struct Held {
    pub o: a1::order::Order,
    pub sig_seller: Vec<u8>,
    pub sig_buyer: Vec<u8>,
    pub guild: Option<(String, u64)>,
    pub logic: String,
    pub pack: Vec<u8>,
    pub owner: String,
}

impl Held {
    pub fn writable(&self) -> R<()> {
        Ok(a1::chain::listing_writable(&self.logic)?)
    }
}

pub fn held(c: &Ctx, f: &Flags, w: &shared::Wire, who: &str) -> R<Held> {
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id(订单记录)".into()))?;
    match c.seat {
        Seat::Seller => {
            let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
            let o = eravon_watch::order::load(&c.data, who, &id).map_err(s4)?;
            let d = eravon_watch::poll::deal(&c.data, who, &o, &sw).map_err(s4)?;
            Ok(Held { o: d.o, sig_seller: d.sig_seller, sig_buyer: d.sig_buyer, guild: d.guild.then(|| (d.guild_addr.clone(), d.seq.unwrap_or(0))), logic: d.logic, pack: Vec::new(), owner: who.to_string() })
        }
        Seat::Buyer => {
            let r = c.read()?;
            let owners = eravon_vault::store::archive_of(&c.data, who).map_err(|e| Fault::State(e.detail()))?;
            let (owner, b) = match owners.iter().find_map(|o| r.part(o, Box_::Packs, &id, "pack.json").ok().map(|b| (o.clone(), b))) {
                Some(x) => x,
                None => (who.to_string(), r.part(who, Box_::Packs, &id, "pack.json")?),
            };
            let pk = k3::pack::pack(&b).map_err(Fault::from)?;
            let (p, r) = (&pk.listing, &pk.receipt);
            let o = shared::order_of(p.terms_text.as_bytes(), r.candidate, &r.buyer_key, &r.buyer_payout, r.open_by, &r.prev)?;
            let n = r.candidate as usize;
            let sig_seller = a1::order::bytes_of("要约签名", p.candidate_sig(n)).unwrap_or_default();
            let sig_buyer = eravon_watch::poll::sig_bytes(&r.sig).map_err(s4)?;
            let guild = n.checked_sub(p.judges.len()).and_then(|g| p.guilds.get(g)).map(|g| (g.address.to_ascii_lowercase(), r.seq.unwrap_or(0)));
            Ok(Held { o, sig_seller, sig_buyer, guild, logic: p.logic.clone(), pack: b, owner })
        }
    }
}

pub fn deploy(c: &Ctx, f: &Flags) -> R<Out> {
    if c.seat == Seat::Seller {
        let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id(订单记录)".into()))?;
        let who = c.who(f.one("--address"))?;
        if eravon_watch::poll::made_past(&c.data, &who, &eravon_watch::order::load(&c.data, &who, &id).map_err(s4)?) {
            return Err(a1::fault::Fault::PastListing { logic: String::new() }.into());
        }
        if let Some(path) = exporting(f) {
            let who = c.who(f.one("--address"))?;
            let o = eravon_watch::order::load(&c.data, &who, &id).map_err(s4)?;
            return company_export(c, f, &who, &path, |w, co| {
                let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
                let (p, buyer) = eravon_watch::company::open(&sw, &c.data, &who, &o, &co.address, c.now).map_err(s4)?;
                let mut e = crate::company::Export::of(p);
                e.deal = crate::company::Deal::Unopened(buyer);
                Ok(e)
            });
        }
        return watch_order(c, f, &id);
    }
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let h = crate::buyer::new_order_held(c, f, &w, &who)?;
    let p = open_calls(&OpenIn { w: &w, o: &h.o, sig_seller: &h.sig_seller, sig_buyer: &h.sig_buyer, from: &who, guild: h.guild.as_ref().map(|(g, s)| (g.as_str(), *s)), now: c.now })?;
    let mut rows = vec![("caseId", Out::S(hex(&p.case_id))), ("instance", Out::S(p.instance.clone()))];
    rows.extend(two_calls(c, f, &w, &who, p.deploy.as_ref(), &p.init)?);
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(crate) fn two_calls(c: &Ctx, f: &Flags, w: &shared::Wire, who: &str, first: Option<&a1::call::Call>, then: &a1::call::Call) -> R<Vec<(&'static str, Out)>> {
    let mut rows = vec![("initTx", Out::O(vec![("data", Out::S(then.hex())), ("to", Out::S(then.to.clone()))]))];
    match first {
        None => rows.extend(shared::deliver_rows(c, f, w, who, then, "0")?),
        Some(dep) => {
            let (one, second) = shared::deliver_two(c, f, w, who, dep, then)?;
            rows.extend(one);
            if let Some(second) = second {
                rows.push(("init", Out::O(second)));
            }
        }
    }
    Ok(rows)
}

pub fn expire_calls(w: &shared::Wire, o: &a1::order::Order, logic: &str, now: u64) -> R<(k2::derive::Order, Option<a1::call::Call>, a1::call::Call)> {
    let until = o.offer.expiry.min(o.open_by);
    let revoked = a1::chain::revoked(&w.t, &w.table, logic, &o.offer.seller_key, &o.offer.terms_hash)?.value;
    if !revoked && a1::precheck::in_window(until, now).is_ok() {
        return Err(Fault::Precheck {
            gate: a1::precheck::GATE_WINDOW.to_string(),
            why: format!("这一单到 {until} 前还能开案(min(expiry, openBy)),也没下架;现在是 {now}"),
        });
    }
    let d = o.derive(w.chain_id, logic)?;
    let (code, _) = eravon_chain::read::code(&w.t, &w.table, &d.instance, eravon_chain::seam::TAG_LATEST)?;
    let deploy_call = if code.is_empty() { Some(a1::call::deploy_instance(&d.salt, logic)?) } else { None };
    let init_call = a1::call::expire_init_at(o, &d.instance, logic)?;
    Ok((d, deploy_call, init_call))
}

pub fn expire(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let h = held(c, f, &w, &who)?;
    h.writable()?;
    let (d, deploy_call, init_call) = expire_calls(&w, &h.o, &h.logic, c.now)?;
    let mut rows = vec![
        ("caseId", Out::S(hex(&d.salt))),
        ("expiry", Out::N(h.o.offer.expiry)),
        ("instance", Out::S(d.instance.clone())),
        ("openBy", Out::N(h.o.open_by)),
    ];
    rows.extend(two_calls(c, f, &w, &who, deploy_call.as_ref(), &init_call)?);
    rows.extend(kept_end(c, f, &h, eravon_watch::order::State::Expired));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub fn kept_end(c: &Ctx, f: &Flags, h: &Held, state: eravon_watch::order::State) -> Vec<(&'static str, Out)> {
    let Some(id) = f.opt("--id").filter(|_| c.seat == Seat::Buyer) else { return Vec::new() };
    match eravon_orders::table::keep_ended(&c.data, &c.writer, &h.owner, &id, state, c.now) {
        Ok(()) => Vec::new(),
        Err(e) => vec![("notKept", Out::S(format!("{}:{}", e.name(), e.detail())))],
    }
}

pub struct OpenIn<'a> {
    pub w: &'a shared::Wire,
    pub o: &'a a1::order::Order,
    pub sig_seller: &'a [u8],
    pub sig_buyer: &'a [u8],
    pub from: &'a str,
    pub guild: Option<(&'a str, u64)>,
    pub now: u64,
}

pub struct Opened {
    pub case_id: [u8; 32],
    pub instance: String,
    pub deploy: Option<a1::call::Call>,
    pub init: a1::call::Call,
    pub passed: eravon_cands::act::Passed,
}

pub fn open_calls(i: &OpenIn<'_>) -> R<Opened> {
    let w = i.w;
    let d = i.o.derive(w.chain_id, &w.logic)?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (code, _) = eravon_chain::read::code(&w.t, &w.table, &d.instance, eravon_chain::seam::TAG_LATEST)?;
    if !code.is_empty() {
        let (phase, _) = eravon_watch::poll::phase(&sw, &d.instance).map_err(|(token, why)| Fault::Down { layer: "S4", token, why })?;
        if phase != 0 {
            return Err(Fault::Down { layer: "S4", token: E_OPEN_OPENED.into(), why: format!("{} 的案已开(阶段 {phase}),不必再开", d.instance) });
        }
    }
    let (sig_seller, sig_buyer) = eravon_watch::act::opening_slots(&sw, i.o, i.sig_seller, i.sig_buyer)?;
    let (sig_seller, sig_buyer) = (sig_seller.as_slice(), sig_buyer.as_slice());
    gates(w, i.o, &d.instance, sig_seller, sig_buyer, i.now, i.from)?;
    a1::precheck::verifier_there(&w.t, &w.table)?;
    let passed = eravon_watch::act::check(&sw, i.o).map_err(act_refused)?;
    let (members, hashes) = match i.guild {
        None => (Vec::new(), Vec::new()),
        Some((g, _)) => {
            let p = eravon_watch::pool::panel(&sw, g, i.o).map_err(s4)?;
            (p.members, p.amanah_hashes)
        }
    };
    let init = a1::call::init(i.o, &d.instance, &passed.quote, sig_buyer, sig_seller, &members, &hashes)?;
    let deploy = if code.is_empty() { Some(a1::call::deploy_instance(&d.salt, &w.logic)?) } else { None };
    Ok(Opened { case_id: d.salt, instance: d.instance.clone(), deploy, init, passed })
}

pub fn act_refused(r: eravon_cands::act::Refused) -> Fault {
    Fault::Down { layer: "S4", token: r.name, why: format!("{}:{}", r.gate, r.why) }
}

pub fn act_gate_of(f: &Fault) -> Option<&str> {
    match f {
        Fault::Down { layer: "S4", why, .. } => why.split_once(':').map(|(g, _)| g),
        _ => None,
    }
}

pub const E_OPEN_OPENED: &str = eravon_watch::poll::E_OPEN_OPENED;

#[allow(clippy::too_many_arguments)]
pub(super) fn gates(w: &shared::Wire, o: &a1::order::Order, instance: &str, sig_seller: &[u8], sig_buyer: &[u8], now: u64, from: &str) -> R<()> {
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    Ok(eravon_watch::act::opening(&sw, o, instance, sig_seller, sig_buyer, now, from)?)
}


fn one_send(c: &Ctx, f: &Flags, w: &crate::shared::Wire, who: &str, call: &a1::call::Call, at: Option<&eravon_watch::order::Broadcast>, land: Option<eravon_watch::send::Land<'_>>) -> (Option<(String, eravon_watch::order::Broadcast)>, R<crate::send::Delivered>) {
    let mut landed = None;
    let got = match land {
        Some(l) => {
            let mut keep = |_: u64, tx: &str, b: &eravon_watch::order::Broadcast| -> R<()> {
                l(tx, b).map_err(|(token, why)| Fault::Down { layer: "S4", token, why })?;
                landed = Some((tx.to_string(), b.clone()));
                Ok(())
            };
            match at {
                Some(at) => crate::send::deliver_again(c, f, w, who, call, &crate::send::pending_of(at), Some(&mut keep)),
                None => crate::send::deliver_with(c, f, w, who, call, "0", Some(&mut keep)),
            }
        }
        None => match at {
            Some(at) => crate::send::deliver_again(c, f, w, who, call, &crate::send::pending_of(at), None),
            None => crate::send::deliver_with(c, f, w, who, call, "0", None),
        },
    };
    let handed = landed.or_else(|| got.as_ref().ok().map(|d| (d.tx_hash.clone(), d.broadcast())));
    (handed, got)
}

impl CliPost<'_> {
    fn send(&mut self, call: &a1::call::Call, at: Option<&eravon_watch::order::Broadcast>, land: Option<eravon_watch::send::Land<'_>>) -> Result<String, eravon_watch::Why> {
        self.sent = None;
        let (handed, got) = one_send(self.c, self.f, self.w, self.who, call, at, land);
        self.sent = handed;
        let d = got.map_err(|e| (e.name().to_string(), e.detail()))?;
        self.rows.push(Out::O(d.rows()));
        d.outcome()
    }
}

impl ActPost<'_> {
    fn send(&mut self, call: &a1::call::Call, at: Option<&eravon_watch::order::Broadcast>, land: Option<eravon_watch::send::Land<'_>>) -> Result<String, eravon_watch::Why> {
        self.call = Some(call.clone());
        self.tx.clear();
        self.carried = None;
        if !crate::send::asked(self.f) {
            return Err((NOT_SENT.into(), "没给 --send true".into()));
        }
        let (handed, got) = one_send(self.c, self.f, self.w, self.who, call, at, land);
        if let Some((tx, b)) = handed {
            (self.tx, self.carried) = (tx, Some(b));
        }
        let d = got.map_err(|e| (e.name().to_string(), e.detail()))?;
        self.rows = d.rows();
        d.outcome()
    }
}
