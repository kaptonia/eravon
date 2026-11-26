use eravon_literals::misuse::Misuse;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;
use eravon_literals::json::Out;
use eravon_standard::call::Call;
use eravon_standard::order::Order;
use eravon_store::vault::Seat;
use eravon_terms as k2;

use crate::fault::{Fault, R};
use crate::home::{self, Ctx, Source};
use crate::answer;

pub struct Flags {
    pairs: Vec<(String, String)>,
}

impl Flags {
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    pub fn read(args: &[String]) -> Result<Flags, Misuse> {
        Ok(Flags { pairs: eravon_literals::schema::read_pairs(args)? })
    }

    pub fn one(&self, k: &str) -> Option<&str> {
        self.pairs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
    pub fn many(&self, k: &str) -> Vec<String> {
        self.pairs.iter().filter(|(a, _)| a == k).map(|(_, v)| v.clone()).collect()
    }
    pub fn opt(&self, k: &str) -> Option<String> {
        self.one(k).map(str::to_string)
    }
    pub fn need(&self, k: &str) -> Result<String, Misuse> {
        self.opt(k).ok_or_else(|| Misuse::missing(k))
    }
    pub fn int(&self, k: &str) -> Result<u64, Misuse> {
        let v = self.one(k).ok_or_else(|| Misuse::missing(k))?;
        v.parse().map_err(|_| Misuse::not_int(k, v))
    }
    pub fn int_or(&self, k: &str, d: u64) -> Result<u64, Misuse> {
        match self.one(k) {
            None => Ok(d),
            Some(s) => s.parse().map_err(|_| Misuse::not_int(k, s)),
        }
    }

    pub fn opt_int(&self, k: &str) -> R<Option<u64>> {
        match self.one(k) {
            None => Ok(None),
            Some(v) => v
                .parse()
                .map(Some)
                .map_err(|_| Fault::Arg(format!("{k} 要是十进制整数,给的是 {v:?}"))),
        }
    }
    pub fn flag(&self, k: &str) -> bool {
        self.one(k) == Some("true")
    }
}

pub const CTX_REQUIRED: &[&str] = &["--data", "--now"];

pub fn ctx_musts() -> &'static [&'static str] {
    CTX_REQUIRED
}

pub fn ctx(f: &Flags, seat: Seat) -> Result<Ctx, Misuse> {
    let (data, now) = ctx_shape(f)?;
    let c = home::open_ctx(data, now, f.opt("--writer").unwrap_or_else(|| "eravon".to_string()), seat, f.one("--passcode"));
    home::settle_run_network(&c, f);
    Ok(c)
}

pub fn ctx_shape(f: &Flags) -> Result<(String, u64), Misuse> {
    Ok((f.need(CTX_REQUIRED[0])?, f.int(CTX_REQUIRED[1])?))
}


#[derive(Clone)]
pub struct Wire {
    pub t: Auto,
    pub table: Table,
    pub source: Source,
    pub chain_id: u64,
    pub logic: String,
    pub registries: Vec<String>,
}

impl Wire {
    pub fn rows(&self) -> Vec<(&'static str, Out)> {
        vec![
            ("endpointCount", Out::N(self.table.list.len() as u64)),
            ("endpointSource", Out::S(self.source.word().to_string())),
        ]
    }
}

pub fn route(f: &Flags, ctx: &Ctx, who: Option<&str>) -> eravon_goods::company::Route {
    wire(f, ctx, who).map(|w| (w.t, w.table)).map_err(|e| (e.name().to_string(), e.detail()))
}

pub fn payout_flag(f: &Flags, default: &str) -> R<String> {
    match f.opt("--payout") {
        Some(x) => eravon_vault::payout::accept(x.trim()).map_err(home::down),
        None => Ok(default.to_string()),
    }
}

pub fn send_plan(token: Option<&str>, amount: Option<&str>, to: &str) -> Result<String, (String, String)> {
    let v = |e: eravon_vault::vfault::Fault| (e.name().to_string(), e.detail());
    let to = eravon_vault::payout::accept(to.trim()).map_err(v)?;
    if let Some(t) = token {
        eravon_vault::payout::accept(t.trim()).map_err(v)?;
    }
    if let Some(x) = amount {
        match eravon_literals::units::units_of(x, 0) {
            Ok(_) | Err(eravon_literals::units::AMOUNT_TOO_PRECISE) => {}
            Err(name) => return Err((name.to_string(), format!("金额 {x:?} 的形不合"))),
        }
    }
    Ok(to)
}

pub fn send_amount(v: &k2::u256::U256, shown: &str) -> Result<(), (String, String)> {
    if v.is_zero() {
        return Err(("E_P7_AMOUNT".into(), format!("金额 {shown:?} 是零,没有可转的")));
    }
    Ok(())
}

pub fn advance_gate(f: &Flags, c: &Ctx, instance: &str, case_id: &[u8; 32], seat: u64) -> R<()> {
    let down = |token: &str, why: String| Fault::Down { layer: "watch", token: token.to_string(), why };
    let who = c.who(f.one("--address"))?;
    let w = wire(f, c, Some(&who))?;
    let ww = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let due = eravon_watch::pool::advance_due(&ww, instance, case_id, seat).map_err(|(n, d)| down(&n, d))?;
    if !due.moves() {
        return Err(down(eravon_watch::pool::ADVANCE_NOT_DUE, eravon_watch::pool::ADVANCE_NOT_DUE_SAYS.to_string()));
    }
    Ok(())
}

pub fn redraw_lists(f: &Flags, w: &Wire, instance: &str) -> R<(Vec<String>, Vec<[u8; 32]>)> {
    let down = |token: &str, why: String| Fault::Down { layer: "watch", token: token.to_string(), why };
    let ww = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (members, hashes) = eravon_watch::pool::draw_lists(&ww, instance)
        .map_err(|(n, d)| down(&n, d))?
        .ok_or_else(|| down("E_DISPUTE_DRAW_NOT_DUE", format!("{instance} 没有待抽的签,或抽签那一块还没过")))?;
    let given_members = f.many("--member");
    let given_hashes = f.many("--amanah");
    if given_members.is_empty() && given_hashes.is_empty() {
        return Ok((members, hashes));
    }
    let mut typed = Vec::new();
    for h in &given_hashes {
        typed.push(eravon_standard::order::b32_of("--amanah", h)?);
    }
    let same_members = given_members.len() == members.len() && given_members.iter().zip(&members).all(|(a, b)| a.eq_ignore_ascii_case(b));
    if !same_members || typed != hashes {
        return Err(down("E_DISPUTE_DRAW_GIVEN", format!("给的 --member/--amanah 不是这一单冻结的名单({} 人)", members.len())));
    }
    Ok((members, hashes))
}

pub fn former_movable<E>(w: &Wire, to: &str, token: Option<&str>, held: &Result<String, E>) -> bool {
    match held {
        Err(_) => true,
        Ok(x) if x == "0" => false,
        Ok(x) => token.is_some() || eth_after_fee(w, to, x).map_or(true, |left| left.is_none_or(|l| !l.is_zero())),
    }
}

const PLAIN_TRANSFER_GAS: u64 = 21_000;

fn eth_after_fee(w: &Wire, to: &str, have: &str) -> R<Option<k2::u256::U256>> {
    let have_u = k2::u256::U256::from_dec(have).ok_or_else(|| Fault::State(format!("余额读成了 {have}")))?;
    if !eravon_chain::read::code(&w.t, &w.table, to, "latest").map_err(Fault::from)?.0.is_empty() {
        return Ok(None);
    }
    let (head, _) = eravon_chain::read::height(&w.t, &w.table).map_err(Fault::from)?;
    let block = eravon_chain::read::read_agreed(&w.t, &w.table, eravon_chain::seam::M_GET_BLOCK, vec![Out::S(eravon_chain::seam::quantity(head)), Out::B(false)]).map_err(Fault::from)?;
    let base = eravon_chain::ask::core_of(&block.value).and_then(|b| eravon_chain::send::base_fee_of(&b)).map(u128::from);
    let price = k2::u256::U256::from_dec(&eravon_chain::gas::Fees::of(base, None).cap).unwrap_or(k2::u256::U256::ZERO);
    let wide = price.mul_wide(&k2::u256::U256::from_u64(eravon_chain::send::limit_for(PLAIN_TRANSFER_GAS)));
    let fee = k2::u256::U256([wide.0[0], wide.0[1], wide.0[2], wide.0[3]]);
    Ok(Some(if have_u.cmp_u(&fee) == std::cmp::Ordering::Greater { have_u.overflowing_sub(&fee).0 } else { k2::u256::U256::ZERO }))
}

pub fn wire(f: &Flags, ctx: &Ctx, who: Option<&str>) -> R<Wire> {
    let (inline, file) = (f.many("--endpoint"), f.one("--table"));
    let nodes = match who {
        Some(a) if inline.is_empty() && file.is_none() => Some(crate::home::or_missing(ctx.endpoints(a))?),
        _ => None,
    };
    let (table, source) = home::table(&inline, file, nodes.as_deref())?;
    home::keep_bare_reads();
    let t = home::transport(f.one("--roots"), f.one("--proxy"), source == home::Source::Identity)?;
    let chain_id = home::chain_id(f.opt_int("--chain")?);
    if source == home::Source::Identity && f.opt_int("--chain")?.is_some_and(|c| eravon_literals::constants::network_of(c).is_none()) {
        eravon_chain::read::chain_guard(&t, &table, chain_id)?;
    }
    Ok(Wire { t, table, source, chain_id, logic: home::logic(f.one("--logic")), registries: f.many("--registry") })
}

pub fn evidence_built(c: &Ctx, f: &Flags, who: &str, party: &str, instance: &str, case_id: &str, w: &Wire) -> R<Option<eravon_dispute::evidence::Evidence>> {
    let (drop, kit) = (f.opt("--manifest-dir"), f.opt("--kit"));
    if drop.is_none() && kit.is_none() {
        return Ok(None);
    }
    if f.opt("--manifest").is_some() || f.opt("--manifest-file").is_some() {
        return Err(Fault::Arg("证据清单要么由 --manifest-dir/--kit 现建,要么由 --manifest/--manifest-file 给现成的,只取一种".into()));
    }
    let (home, machine) = evidence_places(c, f, who, kit.is_some())?;
    let drop = drop.map(|p| whole_place(&p));
    let s6 = |(token, why): (String, String)| Fault::Down { layer: "S6", token, why };
    let (items, source) = eravon_dispute::evidence::items(&home, machine.as_deref(), kit.as_deref(), drop.as_deref()).map_err(s6)?;
    let note = f.opt("--note").filter(|n| !n.trim().is_empty());
    eravon_dispute::evidence::sheet_for(items, source, &party.to_ascii_lowercase(), &instance.to_ascii_lowercase(), &case_id.to_ascii_lowercase(), w.chain_id, &w.logic, note).map(Some).map_err(s6)
}

pub fn evidence_places(c: &Ctx, f: &Flags, who: &str, kit: bool) -> R<(String, Option<String>)> {
    if !kit {
        return Ok((String::new(), None));
    }
    let (linked, dir) = crate::deskops::network::zikaron_of(&c.data, who)?;
    if !linked {
        return Err(Fault::Down { layer: "S9", token: crate::deskops::network::ZK_OFF.to_string(), why: "这一身份的设置里关了「连接 ZIKARON」,不探".into() });
    }
    let home = f.opt("--home").ok_or_else(|| Fault::Arg("要 --home".into()))?;
    let machine = f.opt("--machine").or(Some(dir).filter(|d| !d.trim().is_empty()));
    Ok((home, machine))
}

pub fn evidence_rows(ev: &eravon_dispute::evidence::Evidence) -> Vec<(&'static str, Out)> {
    vec![("files", Out::A(ev.files.iter().map(|(n, _)| Out::S(n.clone())).collect())), ("manifest", Out::S(eravon_standard::order::hex_of(&ev.manifest)))]
}

pub fn whole_place(p: &str) -> String {
    match std::fs::canonicalize(p) {
        Ok(x) => x.display().to_string(),
        Err(_) => std::path::absolute(p).map(|x| x.display().to_string()).unwrap_or_else(|_| p.to_string()),
    }
}

pub fn evidence_arg(f: &Flags) -> R<([u8; 32], Option<Vec<u8>>)> {
    let given = f.opt("--manifest").map(|m| eravon_standard::order::b32_of("--manifest", &m)).transpose()?;
    let Some(path) = f.opt("--manifest-file") else {
        return Ok((given.ok_or_else(|| Fault::Arg("要 --manifest 或 --manifest-file".into()))?, None));
    };
    let bytes = home::read_bounded(&path)?;
    let at = eravon_evidence::sheet::manifest(&bytes).map_err(|e| Fault::Arg(format!("--manifest-file 那一份不是正典的证据清单:{e:?}")))?;
    if let Some(m) = given.filter(|m| *m != at) {
        return Err(Fault::Arg(format!(
            "--manifest {} 与 --manifest-file 现算的清单哈希 {} 不同:随附的原文对不上承诺的哈希",
            eravon_standard::order::hex_of(&m),
            eravon_standard::order::hex_of(&at)
        )));
    }
    Ok((at, Some(bytes)))
}


pub fn tx_rows(c: &Call, value: &str) -> Vec<(&'static str, Out)> {
    let q = crate::send::word(value)
        .map(|w| crate::send::quantity(&w))
        .unwrap_or_else(|_| value.to_string());
    vec![(
        "tx",
        Out::O(vec![
            ("data", Out::S(c.hex())),
            ("to", Out::S(c.to.clone())),
            ("value", Out::S(q)),
        ]),
    )]
}

pub fn deliver_rows(
    c: &Ctx,
    f: &Flags,
    w: &Wire,
    who: &str,
    call: &Call,
    value: &str,
) -> R<Vec<(&'static str, Out)>> {
    let mut rows = tx_rows(call, value);
    if !crate::send::asked(f) {
        rows.push(("sent", Out::B(false)));
        rows.push(("why", Out::S("没给 --send true,故只把这一笔的字节交出来".into())));
        return Ok(rows);
    }
    let d = crate::send::deliver(c, f, w, who, call, value)?;
    let verdict = d.verdict();
    rows.extend(d.rows());
    if let crate::send::Verdict::Reverted = verdict {
        return Err(Fault::Reverted { tx_hash: d.tx_hash.clone() });
    }
    if let Err(e) = &d.receipt {
        return Err(e.clone());
    }
    Ok(rows)
}

#[allow(clippy::type_complexity)]
pub fn deliver_two(c: &Ctx, f: &Flags, w: &Wire, who: &str, first: &Call, second: &Call) -> R<(Vec<(&'static str, Out)>, Option<Vec<(&'static str, Out)>>)> {
    if !crate::send::asked(f) {
        return Ok((deliver_rows(c, f, w, who, first, "0")?, None));
    }
    for call in [first, second] {
        eravon_standard::gate::sender_current(&w.t, &w.table, who, call)?;
    }
    let mut each: Vec<Vec<(&'static str, Out)>> = Vec::new();
    let txs = crate::send::series([first, second], |call| {
        let d = crate::send::deliver(c, f, w, who, call, "0")?;
        let mut r = tx_rows(call, "0");
        r.extend(d.rows());
        each.push(r);
        Ok(d)
    })?;
    let mut one = each.remove(0);
    one.push(("txs", Out::A(txs.into_iter().map(Out::S).collect())));
    Ok((one, each.pop()))
}

pub fn upto(f: &Flags) -> R<eravon_chain::seam::Upto> {
    let to = match f.one("--to-block") {
        None => None,
        Some(_) => Some(f.int("--to-block").map_err(|_| Fault::Arg("--to-block 要是整数".into()))?),
    };
    eravon_chain::seam::Upto::of_range(0, to, None).map_err(|e| Fault::Arg(e.detail()))
}

pub fn gate_row(gates: &[(&'static str, bool)]) -> Out {
    Out::A(
        gates
            .iter()
            .map(|(g, ok)| {
                Out::O(vec![("gate", Out::S((*g).to_string())), ("passed", Out::B(*ok))])
            })
            .collect(),
    )
}

pub fn order_of(terms_bytes: &[u8], candidate: u64, buyer_key: &str, buyer_payout: &str, open_by: u64, prev: &str) -> R<Order> {
    let t = k2::doc::terms(terms_bytes).map_err(Fault::from)?;
    let offer = k2::derive::offer(&t, candidate).map_err(Fault::from)?;
    Ok(Order {
        offer,
        buyer_key: eravon_standard::order::addr_norm("买家签名钥", buyer_key)?,
        buyer_payout: eravon_standard::order::addr_norm("买家退款地址", buyer_payout)?,
        open_by,
        prev: eravon_standard::order::addr_norm("前段实例", prev)?,
    })
}

pub fn open_by_at(expiry: u64, open_window: u64, now: u64) -> R<u64> {
    let open_by = expiry.min(now.saturating_add(open_window));
    if open_by <= now {
        return Err(Fault::State(format!("要约已过期({expiry}),这一单的 openBy 落在签名之前,不签")));
    }
    Ok(open_by)
}

pub fn open_by_given(f: &Flags, expiry: u64, open_window: u64, now: u64) -> R<u64> {
    let most = open_by_at(expiry, open_window, now)?;
    match f.opt_int("--open-by")? {
        None => Ok(most),
        Some(n) if n > now && n <= most => Ok(n),
        Some(n) => Err(Fault::Arg(format!("--open-by {n} 要晚于现在 {now}、不晚于 min(expiry, 签字 + openWindow) = {most}"))),
    }
}

pub fn open_by_flag(f: &Flags) -> R<u64> {
    f.opt_int("--open-by")?.ok_or_else(|| Fault::Arg("要 --open-by(买家回执里的 openBy):它定预算地址".into()))
}

pub const COMPANY_OPEN_MARGIN: u64 = 12 * 12;

pub fn open_by_by_call(expiry: u64, open_window: u64, now: u64, bound: u64) -> R<(u64, u64)> {
    let e = now.saturating_add(bound).saturating_add(COMPANY_OPEN_MARGIN);
    let open_by = expiry.min(e.saturating_add(open_window));
    if open_by <= e {
        return Err(Fault::Down {
            layer: "V2",
            token: "E_ORDER_OPENBY_BYCALL".into(),
            why: format!("公司同意最迟落链于 {e}(导出时刻 + 日历上界 {bound} 秒 + 提前量 {COMPANY_OPEN_MARGIN} 秒),openBy {open_by} 不晚于它:这一单落了同意也开不了案,不导出"),
        });
    }
    Ok((e, open_by))
}

pub fn derive_rows(o: &Order, chain_id: u64, logic: &str) -> R<(String, String, [u8; 32])> {
    let d = o.derive(chain_id, logic)?;
    Ok((answer::hex(&d.salt), d.instance.clone(), d.order_digest))
}

pub fn sign(key: &[u8; 32], digest: &[u8; 32]) -> R<String> {
    k2::sig::sign_secp256k1(key, digest)
        .map(|(sig, _addr)| sig)
        .ok_or_else(|| Fault::State("这三十二字节不是一枚 secp256k1 私钥".into()))
}

pub fn address_of(key: &[u8; 32]) -> R<String> {
    k2::sig::sign_secp256k1(key, &[0u8; 32])
        .map(|(_sig, addr)| addr)
        .ok_or_else(|| Fault::State("这三十二字节不是一枚 secp256k1 私钥".into()))
}

pub const DECIMALS_UNREAD: &str = "token-decimals-unread";

pub fn decimals_for(data: &str, w: Option<&Wire>, token: &str) -> Option<u8> {
    let rows = eravon_vault::tokens::list(data);
    match w {
        Some(w) => eravon_standard::chain::amount_decimals(&w.t, &w.table, &rows, token),
        None => eravon_vault::tokens::decimals_of(&rows, eravon_vault::tokens::chain(), token, None),
    }
}

pub fn amount_flag(f: &Flags, raw: &str, text: &str, decimals: impl FnOnce() -> Option<u8>) -> R<Option<String>> {
    if let Some(v) = f.opt(raw) {
        return Ok(Some(v));
    }
    let Some(t) = f.opt(text) else { return Ok(None) };
    let d = decimals().ok_or_else(|| Fault::Down { layer: "K7", token: DECIMALS_UNREAD.to_string(), why: format!("{text} 要按币的小数位折成最小单位,小数位读不成;请改用 {raw}") })?;
    eravon_literals::units::units_of(&t, d).map(Some).map_err(|name| Fault::Down { layer: "K7", token: name.to_string(), why: format!("{text} {t:?} 折不成最小单位(至多 {d} 位小数)") })
}

pub fn units_asked(f: &Flags) -> bool {
    f.flag("--units-text")
}

pub fn units_rows(f: &Flags, x: &'static str, raw: &str, decimals: Option<u8>) -> Vec<(&'static str, Out)> {
    if !units_asked(f) {
        return Vec::new();
    }
    let Some((text, dec)) = units_names(x) else { return Vec::new() };
    match decimals {
        Some(d) if !raw.is_empty() && raw.bytes().all(|c| c.is_ascii_digit()) => vec![(text, Out::S(eravon_literals::units::units_text(raw, d, false))), (dec, Out::N(u64::from(d)))],
        _ => vec![(text, Out::Null), (dec, Out::Null)],
    }
}

pub fn units_names(x: &str) -> Option<(&'static str, &'static str)> {
    Some(match x {
        "amount" => ("amountText", "amountDecimals"),
        "balance" => ("balanceText", "balanceDecimals"),
        "judgeFee" => ("judgeFeeText", "judgeFeeDecimals"),
        "lf" => ("lfText", "lfDecimals"),
        "lfHeld" => ("lfHeldText", "lfHeldDecimals"),
        _ => return None,
    })
}

#[cfg(test)]
mod open_by_tests {
    use super::*;

    #[test]
    fn open_by_is_the_earlier_of_expiry_and_now_plus_the_window() {
        assert_eq!(open_by_at(2_000, 100, 1_000).unwrap(), 1_100);
        assert_eq!(open_by_at(1_050, 100, 1_000).unwrap(), 1_050);
    }

    #[test]
    fn an_open_by_at_or_before_signing_is_refused() {
        for (expiry, window) in [(999, 100), (1_000, 100), (2_000, 0)] {
            assert!(open_by_at(expiry, window, 1_000).is_err(), "expiry {expiry} window {window}");
        }
    }

    #[test]
    fn the_sum_saturates_at_the_top() {
        assert_eq!(open_by_at(u64::MAX, u64::MAX, 1).unwrap(), u64::MAX);
        assert_eq!(open_by_at(u64::MAX - 1, u64::MAX, u64::MAX - 2).unwrap(), u64::MAX - 1);
    }

    #[test]
    fn an_open_by_is_never_later_than_the_window() {
        let flags = |v: &[&str]| Flags::read(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>()).expect("旗");
        assert_eq!(open_by_given(&flags(&[]), 1_000, 100, 10).unwrap(), 110);
        assert_eq!(open_by_given(&flags(&["--open-by", "50"]), 1_000, 100, 10).unwrap(), 50);
        assert!(open_by_given(&flags(&["--open-by", "111"]), 1_000, 100, 10).is_err(), "晚于窗");
        assert!(open_by_given(&flags(&["--open-by", "10"]), 1_000, 100, 10).is_err(), "不晚于现在");
        assert!(open_by_flag(&flags(&[])).is_err());
        assert_eq!(open_by_flag(&flags(&["--open-by", "7"])).unwrap(), 7);
    }

    #[test]
    fn a_by_call_open_by_starts_at_e() {
        let m = COMPANY_OPEN_MARGIN;
        assert_eq!(open_by_by_call(u64::MAX, 100, 10, 50).unwrap(), (60 + m, 160 + m));
        assert_eq!(open_by_by_call(70 + m, 100, 10, 50).unwrap(), (60 + m, 70 + m), "expiry 先到");
        for expiry in [60 + m, 10] {
            match open_by_by_call(expiry, 100, 10, 50) {
                Err(Fault::Down { token, .. }) => assert_eq!(token, "E_ORDER_OPENBY_BYCALL"),
                other => panic!("不晚于 E 要具名拒,得的是 {other:?}"),
            }
        }
    }
}
