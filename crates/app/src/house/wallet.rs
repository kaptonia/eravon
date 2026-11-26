use eravon_literals::json::Out;
use eravon_wallet::balance::Balances;
use eravon_wallet::send::Amount;
use eravon_wallet::tx::Tx;

use crate::house::seller::Said;

#[derive(Clone, Debug, Default)]
pub struct Wallet {
    pub me: String,
    pub balances: Option<Result<Balances, Said>>,
    pub txs: Option<Result<Vec<Tx>, Said>>,
    pub filled: usize,
    pub qr: Option<(usize, Vec<bool>)>,
    pub open_tx: Option<String>,
    pub sheet: bool,
    pub said: Option<Said>,
    pub fill_said: Option<Said>,
    pub copied: Option<String>,
    pub all: Option<(Option<String>, Result<String, Said>)>,
    pub vault: Option<Result<Vault, Said>>,
    pub vault_set: Option<VaultSet>,
    pub orders_in: Vec<String>,
    pub formers: Vec<eravon_wallet::balance::Balances>,
}

pub fn order_tokens(s: &eravon_vault::session::Session, data: &str, me: &str, seller: bool) -> Vec<String> {
    if seller {
        let o = crate::house::orders::Orders::table_only(data, me);
        tokens_in(o.table.iter().flatten().flatten().map(|r| r.token.as_str()))
    } else {
        let b = crate::house::buyer_orders::BOrders::load(s, data, me);
        tokens_in(b.rows.iter().flatten().flatten().map(|r| r.row.token.as_str()))
    }
}

pub fn tokens_in<'a>(tokens: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tokens.map(str::to_ascii_lowercase).filter(|t| !t.is_empty()) {
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

pub const VAULT_SHEET: &str = "wl-vault";

#[derive(Clone, Debug)]
pub struct VaultSet {
    pub token: String,
    pub amount: String,
    pub calls: Result<Setup, Said>,
}

#[derive(Clone, Debug)]
pub enum Setup {
    Calls(Vec<eravon_standard::call::Call>),
    Proposals(Vec<(String, Vec<String>)>),
    Sent(Vec<String>),
}

pub const VAULT_SEND: &str = "wl-vault-send";

pub const E_WL_AGAIN: &str = eravon_cli::seller::E_P7_AGAIN;

#[derive(Clone, Debug, Default)]
pub struct Vault {
    pub fund: String,
    pub own: bool,
    pub company: String,
    pub logic: String,
    pub rows: Vec<(String, Result<eravon_goods::fund::State, Said>)>,
    pub hints: Vec<eravon_goods::fund::Hint>,
    pub waiting: Vec<Waiting>,
    pub decimals: std::collections::BTreeMap<String, Option<u64>>,
}

#[derive(Clone, Debug)]
pub struct Waiting {
    pub id: String,
    pub token: String,
    pub deposit: Result<eravon_terms::u256::U256, Said>,
}

impl Vault {
    pub fn stands(&self, token: &str, available: &eravon_terms::u256::U256) -> (usize, usize) {
        let on: Vec<&Waiting> = self.waiting.iter().filter(|w| w.token.eq_ignore_ascii_case(token)).collect();
        let ds: Vec<eravon_terms::u256::U256> = on.iter().map_while(|w| w.deposit.as_ref().ok().copied()).collect();
        (eravon_goods::fund::stands(available, &ds), on.len())
    }
}

fn waits(s: eravon_watch::order::State) -> bool {
    use eravon_watch::order::State as S;
    !matches!(s, S::Deployed | S::Expired | S::Revoked | S::Cancelled)
}

fn bad(f: eravon_wallet::fault::Fault) -> Said {
    Said::bad(&f.token, f.detail)
}

pub(crate) fn own_vault(data: &str, me: &str) -> bool {
    crate::house::orders::company_mode(data, me).is_none() && eravon_payways::ways::read(data, me).is_ok_and(|w| w.fund.eq_ignore_ascii_case(me))
}

#[allow(clippy::too_many_arguments)]
fn sign_own(s: &eravon_vault::session::Session, data: &str, me: &str, w: &eravon_cli::shared::Wire, token: &str, units: &eravon_terms::u256::U256, calls: &[eravon_standard::call::Call], still: &dyn Fn() -> bool) -> Result<Vec<String>, Said> {
    let key = || s.own_key(me, me).map_err(|e| eravon_cli::fault::Fault::Arg(format!("{}:{}", e.name(), e.detail())));
    let k = eravon_wallet::tx::Keep { data, writer: crate::house::seam::WRITER, owner: me };
    let last = calls.len().saturating_sub(1);
    let mut done: Vec<String> = Vec::new();
    for (i, call) in calls.iter().enumerate() {
        if !still() {
            return Err(Said::bad("E_APP_STALE", crate::house::i18n::t(crate::house::i18n::Key::ErrESTORESTALE)));
        }
        let amount = if i == last { units.to_dec() } else { "0".to_string() };
        let tag = eravon_wallet::tx::Tag::moving(eravon_wallet::tx::Kind::Approve, "", token, &amount);
        match eravon_wallet::tx::post(&k, w, me, &key, &eravon_cli::send::Opts::default(), call, "0", &tag) {
            Ok(tx) => done.push(tx),
            Err(f) => {
                let mut e = fee_said(f);
                if !done.is_empty() {
                    e.detail = format!("{};前面 {} 笔已上链:{}", e.detail, done.len(), done.join("、"));
                }
                return Err(e);
            }
        }
    }
    Ok(done)
}

fn fee_said(f: eravon_wallet::fault::Fault) -> Said {
    match eravon_cli::send::fee_gap(&f.detail) {
        Some(gap) => {
            let eth = crate::house::coin::eth(crate::lang::lang(), &gap, crate::house::coin::Form::Full);
            Said::bad(&f.token, crate::house::i18n::filln(crate::house::i18n::Key::WlFeeShort, &[&eth]))
        }
        None => bad(f),
    }
}

fn doc_logic(data: &str, me: &str, id: &str) -> Option<String> {
    let b = eravon_goods::goods::terms_of(data, me, id).ok()??;
    eravon_terms::doc::terms(&b).ok().map(|t| t.logic)
}

impl Wallet {
    pub fn open(s: &eravon_vault::session::Session, data: &str, me: &str, seller: bool) -> Wallet {
        crate::house::mark::mark(eravon_wallet::mark::FEATURE);
        let mut w = Wallet { me: me.to_ascii_lowercase(), qr: eravon_wallet::balance::receive_qr(me), ..Wallet::default() };
        w.reread(s, data, me, seller);
        w
    }

    pub fn reread(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, seller: bool) {
        self.orders_in = order_tokens(s, data, me, seller);
        self.reload(s, data, me);
    }

    pub fn reload(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str) {
        if !self.me.eq_ignore_ascii_case(me) {
            self.me = me.to_ascii_lowercase();
            self.qr = eravon_wallet::balance::receive_qr(me);
            self.open_tx = None;
            self.copied = None;
            self.orders_in.clear();
        }
        if let Err(e) = eravon_vault::session::need_open(s) {
            let x = crate::house::said_of(&e);
            self.balances = Some(Err(x.clone()));
            self.txs = Some(Err(x));
            return;
        }
        match crate::house::orders::wire(data, me) {
            Ok(wire) => {
                let k = eravon_wallet::tx::Keep { data, writer: crate::house::seam::WRITER, owner: me };
                (self.filled, self.fill_said) = match eravon_wallet::tx::fill(&k, &wire) {
                    Ok((n, refused)) => (n, refused.map(bad)),
                    Err(f) => (0, Some(bad(f))),
                };
                self.balances = Some(Ok(eravon_wallet::balance::read(&wire, data, me, &self.orders_in)));
                self.formers = eravon_wallet::balance::formers(&wire, data, me, &self.orders_in);
            }
            Err(e) => self.balances = Some(Err(e)),
        }
        self.txs = Some(eravon_wallet::tx::list(data, me).map_err(bad));
    }

    pub fn send_plan(&self, me: &str, token: Option<&str>, amount: &Amount, to: &str, again: Option<&str>) -> Result<(), Said> {
        if !self.me.eq_ignore_ascii_case(me) {
            return Err(Said::bad("E_APP_KEY_CHANGED", crate::fill!(crate::house::i18n::Key::WlKeyChanged; me.to_string(), self.me)));
        }
        if again.is_some_and(|a| !a.trim().eq_ignore_ascii_case(to.trim())) {
            return Err(Said::bad(E_WL_AGAIN, crate::house::i18n::t(crate::house::i18n::Key::ErrEP7AGAIN)));
        }
        eravon_wallet::send::plan(token, amount, to).map_err(bad)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn read_vault(&mut self, data: &str, me: &str) {
        let got = (|| -> Result<Vault, Said> {
            let w = crate::house::orders::wire(data, me)?;
            let (seller, fund, company, own) = match crate::house::orders::company_mode(data, me) {
                Some((c, _)) => {
                    let co = crate::house::orders::open_company(&w, &c)?;
                    (co.address.clone(), co.views.wallet.clone(), co.address, false)
                }
                None => {
                    let ways = eravon_payways::ways::read(data, me).map_err(|e| Said::bad(e.name(), e.detail()))?;
                    (me.to_string(), ways.fund, String::new(), ways.own)
                }
            };
            let mut v = Vault { fund: fund.clone(), own, company, logic: w.logic.clone(), ..Vault::default() };
            let rows = eravon_goods::goods::list(data, me, "").map_err(|e| Said::bad(e.name(), e.detail()))?;
            let mut tokens: Vec<String> = Vec::new();
            for r in rows.iter().filter(|r| !r.draft) {
                if let Some(t) = crate::house::coin::cell(&r.token).map(|a| a.to_ascii_lowercase()) {
                    if !tokens.contains(&t) {
                        tokens.push(t);
                    }
                }
                if let Some(h) = doc_logic(data, me, &r.id).and_then(|l| eravon_goods::fund::new_standard(&l)) {
                    if !v.hints.contains(&h) {
                        v.hints.push(h);
                    }
                }
            }
            if !fund.is_empty() {
                for t in tokens {
                    let st = eravon_goods::fund::state(&w.t, &w.table, &w.logic, &seller, &fund, &t).map_err(|(n, d)| Said::bad(&n, d));
                    v.decimals.insert(t.clone(), crate::house::coin::decimals_read(&w, &t));
                    v.rows.push((t, st));
                }
                let sw = crate::house::orders::sw(&w);
                let mut orders = eravon_watch::order::all(data, me).map_err(|(n, d)| Said::bad(&n, d))?;
                orders.retain(|o| waits(o.state));
                orders.reverse();
                for o in orders {
                    let deposit = eravon_watch::poll::deal(data, me, &o, &sw).map_err(|(n, d)| Said::bad(&n, d)).and_then(|d| {
                        let p = eravon_watch::act::priced(&sw, &d.o).map_err(|r| Said::bad(&r.name, format!("{}:{}", r.gate, r.why)))?;
                        if let Some(h) = eravon_goods::fund::large_deposit(&p.deposit, &d.o.offer.amount, d.o.offer.buyer_may_open) {
                            if !v.hints.contains(&h) {
                                v.hints.push(h);
                            }
                        }
                        Ok(p.deposit)
                    });
                    v.waiting.push(Waiting { id: o.id.clone(), token: o.token.to_ascii_lowercase(), deposit });
                }
            }
            Ok(v)
        })();
        self.vault = Some(got);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn vault_setup(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, dir: &str, now: u64, token: &str, amount: &str, still: &dyn Fn() -> bool) {
        let calls = (|| -> Result<Setup, Said> {
            let units = |w: &eravon_cli::shared::Wire| -> Result<eravon_terms::u256::U256, Said> {
                let rows = eravon_vault::tokens::current();
                let d = eravon_standard::chain::amount_decimals(&w.t, &w.table, &rows, token).ok_or_else(|| Said::bad("E_P7_DECIMALS", format!("读不到 {token} 的位数")))?;
                eravon_wallet::send::amount_of(amount, d).map_err(|e| Said::bad(&e.token, e.detail))
            };
            if let Some((c, time_on)) = crate::house::orders::company_mode(data, me) {
                if dir.is_empty() {
                    return Err(Said::bad("E_ORDERS_EXPORT_DIR", crate::house::i18n::t(crate::house::i18n::Key::OrdersExportDirPick)));
                }
                let w = crate::house::orders::wire(data, me)?;
                let units = units(&w)?;
                let co = crate::house::orders::open_company(&w, &c)?;
                let set = eravon_cli::seller::company_setup(&w, &co, token, &units).map_err(|f| Said::bad(f.name(), f.detail()))?;
                let at = std::path::Path::new(&crate::house::seam::order_proposal_dir(dir, &token.to_ascii_lowercase(), set.allow.verb)).join("proposal.json").display().to_string();
                let mut out = Vec::new();
                let mut write = |path: &str, e: eravon_cli::company::Export| -> Result<(), Said> {
                    let rows = eravon_cli::company::write(now, &w, &co, path, time_on, true, e).map_err(|f| Said::bad(f.name(), f.detail()))?;
                    out.push((path.to_string(), eravon_cli::company::face_of(&rows)));
                    Ok(())
                };
                write(&at, set.allow)?;
                if let Some(bind) = set.bind {
                    write(&eravon_cli::seller::bind_path(&at).map_err(|f| Said::bad(f.name(), f.detail()))?, bind)?;
                }
                return Ok(Setup::Proposals(out));
            }
            let fund = eravon_payways::ways::read(data, me).map_err(|e| Said::bad(e.name(), e.detail()))?.fund;
            if fund.is_empty() {
                return Err(Said::bad(eravon_goods::goods::E_GOODS_FUND_UNSET, crate::house::i18n::t(crate::house::i18n::Key::WlVaultUnset)));
            }
            let w = crate::house::orders::wire(data, me)?;
            let units = units(&w)?;
            let st = eravon_goods::fund::state(&w.t, &w.table, &w.logic, me, &fund, token).map_err(|(n, d)| Said::bad(&n, d))?;
            let calls = eravon_goods::fund::setup(&w.logic, token, me, &fund, &units, &st.allowance, st.bound, &fund).map_err(|(n, d)| Said::bad(&n, d))?;
            if !own_vault(data, me) || calls.is_empty() {
                return Ok(Setup::Calls(calls));
            }
            sign_own(s, data, me, &w, token, &units, &calls, still).map(Setup::Sent)
        })();
        let sent = matches!(calls, Ok(Setup::Sent(_)));
        if sent {
            let coin = crate::house::coin::name(crate::lang::lang(), token);
            self.said = Some(Said::ok(crate::house::i18n::filln(crate::house::i18n::Key::WlAllowanceNow, &[amount.trim(), &coin])));
        } else if let (Err(e), true) = (&calls, own_vault(data, me)) {
            self.said = Some(e.clone());
        }
        self.vault_set = Some(VaultSet { token: token.to_ascii_lowercase(), amount: amount.to_string(), calls });
    }

    pub fn read_all(&mut self, data: &str, me: &str, token: Option<&str>, to: &str) {
        let got = crate::house::orders::wire(data, me).and_then(|w| eravon_wallet::send::sendable(&w, me, token, to).map(|v| v.to_dec()).map_err(bad));
        self.all = Some((token.map(str::to_string), got));
    }

    pub fn decimals_of(&self, token: &str) -> Option<u64> {
        self.balances.as_ref()?.as_ref().ok()?.coin(token)?.decimals
    }

    #[allow(clippy::too_many_arguments)]
    pub fn transfer(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, token: Option<&str>, amount: &Amount, to: &str, again: Option<&str>, still: &dyn Fn() -> bool) {
        if let Err(e) = crate::house::asked(self.send_plan(me, token, amount, to, again)) {
            self.said = Some(e);
            return;
        }
        let got = crate::house::orders::wire(data, me).and_then(|w| {
            let k = eravon_wallet::tx::Keep { data, writer: crate::house::seam::WRITER, owner: me };
            eravon_wallet::send::transfer(&k, &w, s, me, token, amount, to, still).map_err(bad)
        });
        match got {
            Ok(x) => {
                let lang = crate::lang::lang();
                let word = match token {
                    Some(t) => crate::house::coin::amount(lang, &x.amount, self.decimals_of(t), t, crate::house::coin::Form::Short),
                    None => crate::house::coin::eth(lang, &x.amount, crate::house::coin::Form::Short),
                };
                self.said = Some(Said::ok(format!("{} · {word} · {}", crate::house::i18n::t(crate::house::i18n::Key::WlSentSaid), x.tx)));
                self.sheet = false;
                self.reload(s, data, me);
            }
            Err(e) => self.said = Some(e),
        }
    }

    pub fn move_former(&mut self, s: &eravon_vault::session::Session, data: &str, me: &str, from: &str, still: &dyn Fn() -> bool) {
        let Some(b) = self.formers.iter().find(|b| b.address.eq_ignore_ascii_case(from)).cloned() else {
            self.said = Some(Said::bad("E_APP_STALE", crate::house::i18n::t(crate::house::i18n::Key::ErrESTORESTALE)));
            return;
        };
        let got = crate::house::orders::wire(data, from).and_then(|w| {
            let k = eravon_wallet::tx::Keep { data, writer: crate::house::seam::WRITER, owner: me };
            let m = eravon_wallet::send::move_former(&k, &w, s, from, me, &b, still).map_err(bad)?;
            let lift = |e: eravon_cli::fault::Fault| bad(eravon_wallet::fault::lift(e));
            match m.stop {
                None => Ok(m.moved.len()),
                Some(eravon_cli::former::Stop::Eth(e)) if !m.moved.is_empty() => {
                    self.fill_said = Some(lift(e));
                    Ok(m.moved.len())
                }
                Some(eravon_cli::former::Stop::Token(e) | eravon_cli::former::Stop::Eth(e)) => Err(lift(e)),
            }
        });
        match got {
            Ok(n) => self.said = Some(Said::ok(crate::house::i18n::filln(crate::house::i18n::Key::WlFormerMoved, &[&n.to_string()]))),
            Err(e) => self.said = Some(e),
        }
        self.reload(s, data, me);
    }

    pub fn tx(&self, hash: &str) -> Option<&Tx> {
        self.txs.as_ref()?.as_ref().ok()?.iter().find(|t| t.hash.eq_ignore_ascii_case(hash))
    }

    pub fn rows(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        let mut m: Vec<(&'static str, Out)> = vec![("filled", Out::N(self.filled as u64)), ("me", s(&self.me)), ("qr", Out::B(self.qr.is_some())), ("sheet", Out::B(self.sheet))];
        match &self.balances {
            Some(Ok(b)) => {
                let one = |r: &Result<String, eravon_wallet::fault::Fault>| match r {
                    Ok(v) => Out::S(v.clone()),
                    Err(f) => Said::bad(&f.token, f.detail.clone()).out(),
                };
                m.push(("native", one(&b.native)));
                if let Ok(w) = &b.native {
                    m.push(("nativeText", s(&text_of(w, Some(eravon_literals::constants::NATIVE_DECIMALS)))));
                }
                m.push((
                    "coins",
                    Out::A(
                        b.coins
                            .iter()
                            .map(|c| {
                                let mut v = vec![("have", one(&c.have)), ("symbol", s(&c.symbol)), ("token", s(&c.token))];
                                if let Ok(h) = &c.have {
                                    v.push(("haveText", listed_text(h, &c.token)));
                                }
                                if let Some(d) = c.decimals {
                                    v.push(("decimals", Out::N(d)));
                                }
                                v.sort_by(|a, b| a.0.cmp(b.0));
                                Out::O(v)
                            })
                            .collect(),
                    ),
                ));
            }
            Some(Err(e)) => m.push(("native", e.out())),
            None => {}
        }
        m.push((
            "formers",
            Out::A(
                self.formers
                    .iter()
                    .map(|b| {
                        let one = |r: &Result<String, eravon_wallet::fault::Fault>| match r {
                            Ok(v) => Out::S(v.clone()),
                            Err(f) => Said::bad(&f.token, f.detail.clone()).out(),
                        };
                        let coins = b.coins.iter().filter(|c| c.have.as_ref().map_or(true, |x| x != "0")).map(|c| Out::O(vec![("have", one(&c.have)), ("token", s(&c.token))])).collect();
                        Out::O(vec![("address", s(&b.address)), ("coins", Out::A(coins)), ("native", one(&b.native))])
                    })
                    .collect(),
            ),
        ));
        match &self.txs {
            Some(Ok(v)) => m.push(("txs", Out::A(v.iter().map(tx_out).collect()))),
            Some(Err(e)) => m.push(("txs", e.out())),
            None => {}
        }
        if let Some(h) = &self.open_tx {
            m.push(("openTx", self.tx(h).map(tx_out).unwrap_or_else(|| Said::bad("E_APP_SCRIPT", crate::fill!(crate::house::i18n::Key::WlNoTx; h.to_string())).out())));
        }
        if let Some(x) = &self.said {
            m.push(("said", x.out()));
        }
        if let Some(x) = &self.fill_said {
            m.push(("fillSaid", x.out()));
        }
        if let Some(c) = &self.copied {
            m.push(("copied", s(c)));
        }
        match &self.vault {
            Some(Ok(v)) => {
                let rows = v
                    .rows
                    .iter()
                    .map(|(token, st)| match st {
                        Ok(x) => Out::O(vec![("allowance", s(&x.allowance.to_dec())), ("available", s(&x.available.to_dec())), ("balance", s(&x.balance.to_dec())), ("bound", Out::B(x.bound)), ("token", s(token))]),
                        Err(e) => Out::O(vec![("said", e.out()), ("token", s(token))]),
                    })
                    .collect();
                m.push(("vault", Out::O(vec![("company", s(&v.company)), ("fund", s(&v.fund)), ("own", Out::B(v.own)), ("rows", Out::A(rows))])));
            }
            Some(Err(e)) => m.push(("vault", e.out())),
            None => {}
        }
        if let Some(v) = &self.vault_set {
            let calls = match &v.calls {
                Ok(Setup::Calls(c)) => Out::O(vec![("calls", Out::N(c.len() as u64))]),
                Ok(Setup::Proposals(p)) => Out::O(vec![("proposals", Out::N(p.len() as u64))]),
                Ok(Setup::Sent(t)) => Out::O(vec![("sent", Out::A(t.iter().map(|x| s(x)).collect()))]),
                Err(e) => e.out(),
            };
            m.push(("vaultSet", Out::O(vec![("amount", s(&v.amount)), ("formed", calls), ("token", s(&v.token))])));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}

pub fn tx_out(t: &Tx) -> Out {
    let s = |x: &str| Out::S(x.to_string());
    Out::O(vec![
        ("amount", s(&t.amount)),
        ("amountText", if t.token.is_empty() { s(&text_of(&t.amount, Some(eravon_literals::constants::NATIVE_DECIMALS))) } else { listed_text(&t.amount, &t.token) }),
        ("at", Out::N(t.at)),
        ("block", s(&t.block)),
        ("fee", s(&t.fee)),
        ("feeText", s(&text_of(&t.fee, Some(eravon_literals::constants::NATIVE_DECIMALS)))),
        ("from", s(&t.from)),
        ("hash", s(&t.hash)),
        ("kind", s(&t.kind)),
        ("order", s(&t.order)),
        ("status", s(&t.status)),
        ("to", s(&t.to)),
        ("token", s(&t.token)),
    ])
}

pub fn kind_key(k: eravon_wallet::tx::Kind) -> crate::house::i18n::Key {
    use crate::house::i18n::Key;
    use eravon_wallet::tx::Kind;
    match k {
        Kind::Pay => Key::WlKindPay,
        Kind::Open => Key::WlKindOpen,
        Kind::Fee => Key::OrdersKvLf,
        Kind::Approve => Key::WlKindApprove,
        Kind::Dispute => Key::BoActDispute,
        Kind::Confirm => Key::BoActConfirm,
        Kind::Commit => Key::WlKindCommit,
        Kind::Advance => Key::BoActAdvance,
        Kind::Settle => Key::WlKindSettle,
        Kind::Expire => Key::BoActExpire,
        Kind::Sweep => Key::BoActSweep,
        Kind::Send => Key::WlSend,
        Kind::Cancel => Key::BoActCancel,
    }
}

pub fn kind_words(lang: crate::house::i18n::Lang, kind: &str) -> String {
    eravon_wallet::tx::Kind::of(kind).map(|k| crate::house::i18n::get(lang, kind_key(k)).0).unwrap_or_else(|| kind.to_string())
}

fn text_of(raw: &str, decimals: Option<u8>) -> String {
    match decimals {
        Some(d) if !raw.is_empty() && raw.bytes().all(|c| c.is_ascii_digit()) => eravon_vault::tokens::figure(raw, d, eravon_vault::tokens::Form::Plain),
        _ => String::new(),
    }
}

fn listed_text(raw: &str, token: &str) -> Out {
    match eravon_vault::tokens::decimals_of(&crate::house::coin::rows(), eravon_vault::tokens::chain(), token, None) {
        Some(d) if !raw.is_empty() && raw.bytes().all(|c| c.is_ascii_digit()) => Out::S(eravon_vault::tokens::figure(raw, d, eravon_vault::tokens::Form::Plain)),
        _ => Out::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_wallet_record_has_its_words_in_both_languages() {
        for k in eravon_wallet::tx::Kind::ALL {
            for lang in [crate::house::i18n::Lang::Zh, crate::house::i18n::Lang::En] {
                let w = kind_words(lang, k.word());
                assert!(!w.is_empty() && !w.starts_with('?') && !w.starts_with('\u{27e6}') && w != k.word(), "{k:?} {lang:?}: {w}");
            }
        }
    }

    #[test]
    fn the_vault_counts_the_waiting_orders_it_stands_each_by_its_own_deposit() {
        let u = eravon_terms::u256::U256::from_u64;
        let w = |id: &str, token: &str, d: Result<u64, ()>| Waiting { id: id.into(), token: token.into(), deposit: d.map(u).map_err(|_| Said::bad("E_ACT_QUOTE", String::new())) };
        let mut v = Vault { waiting: vec![w("a", "0xaa", Ok(150)), w("b", "0xbb", Ok(1)), w("c", "0xaa", Ok(200)), w("d", "0xaa", Ok(150))], ..Vault::default() };
        assert_eq!(v.stands("0xAA", &u(500)), (3, 3));
        assert_eq!(v.stands("0xaa", &u(349)), (1, 3));
        assert_eq!(v.stands("0xcc", &u(500)), (0, 0), "这枚币上没有待开案的单:不报够押几单");
        v.waiting[2].deposit = Err(Said::bad("E_ACT_QUOTE", String::new()));
        assert_eq!(v.stands("0xaa", &u(500)), (1, 3), "定不出价的那一单把数停在它前面");
    }

    #[test]
    fn another_key_is_refused_and_sends_nothing() {
        let (shown, other, to) = ("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000b2", "0x00000000000000000000000000000000000000c3");
        let amount = Amount::Of("1".to_string());
        let mut w = Wallet { me: shown.to_string(), sheet: true, ..Wallet::default() };
        assert_eq!(w.send_plan(other, None, &amount, to, None).map_err(|s| s.name), Err("E_APP_KEY_CHANGED".to_string()));
        let root = std::env::temp_dir().join("eravon-wallet-key-changed-never-made");
        let data = root.to_string_lossy().to_string();
        w.transfer(&eravon_vault::session::Session::locked(&data), &data, other, None, &amount, to, None, &|| true);
        assert_eq!(w.said.as_ref().map(|s| s.name.as_str()), Some("E_APP_KEY_CHANGED"));
        assert!(w.sheet && w.balances.is_none() && w.txs.is_none());
        assert!(!root.exists());
    }

    #[test]
    fn a_seller_transfer_takes_the_address_twice_and_two_that_differ_send_nothing() {
        let (me, to) = ("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000c3");
        let amount = Amount::Of("1".to_string());
        let mut w = Wallet { me: me.to_string(), sheet: true, ..Wallet::default() };
        assert_eq!(w.send_plan(me, None, &amount, to, Some("0x00000000000000000000000000000000000000c4")).map_err(|s| s.name), Err(E_WL_AGAIN.to_string()));
        assert_eq!(w.send_plan(me, None, &amount, to, Some("")).map_err(|s| s.name), Err(E_WL_AGAIN.to_string()));
        assert!(w.send_plan(me, None, &amount, to, Some(&to.to_ascii_uppercase().replace("0X", "0x"))).is_ok());
        assert!(w.send_plan(me, None, &amount, to, None).is_ok());
        let root = std::env::temp_dir().join("eravon-wallet-again-never-made");
        let data = root.to_string_lossy().to_string();
        w.transfer(&eravon_vault::session::Session::locked(&data), &data, me, None, &amount, to, Some("0x00000000000000000000000000000000000000c4"), &|| true);
        assert_eq!(w.said.as_ref().map(|s| s.name.as_str()), Some(E_WL_AGAIN));
        assert!(w.sheet && w.balances.is_none() && !root.exists());
    }
}

#[cfg(test)]
mod order_token_tests {
    #[test]
    fn each_order_token_once() {
        let rows = ["0xDC64a140Aa3E981100a9becA4E685f962f0cF6C9", "", "0xdc64a140aa3e981100a9beca4e685f962f0cf6c9", "0x00000000000000000000000000000000000000a4"];
        assert_eq!(super::tokens_in(rows.into_iter()), vec!["0xdc64a140aa3e981100a9beca4e685f962f0cf6c9".to_string(), "0x00000000000000000000000000000000000000a4".to_string()]);
    }
}

crate::house::merge::lands!(Wallet {
    whole: [me],
    data: [balances, txs, filled, qr, fill_said, copied, all, vault, vault_set, orders_in, formers],
    page: [open_tx, sheet],
    keyed(|_: &Wallet| ()): [],
    nested: [],
    said: [said],
});
