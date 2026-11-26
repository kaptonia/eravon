use eravon_literals::json::Out;

use crate::answer::ok;
use crate::fault::{Fault, R};
use crate::home::Ctx;
use super::rotate::{self, Checklist, OnChain, Seg};
use crate::shared::Flags;

fn chains(cs: &[OnChain]) -> Out {
    Out::A(
        cs.iter()
            .map(|c| {
                let detail = match &c.seg {
                    Seg::Claim(k) | Seg::Moved(k) => k.clone(),
                    Seg::Unread(w) => w.clone(),
                    _ => String::new(),
                };
                Out::O(vec![("chainId", Out::N(c.chain)), ("detail", Out::S(detail)), ("name", Out::S(c.name.to_string())), ("seat", Out::S(c.seat.word().to_string())), ("state", Out::S(c.seg.word().to_string()))])
            })
            .collect(),
    )
}

fn hex(d: &[u8; 32]) -> String {
    eravon_standard::order::hex_of(d)
}

pub fn owed(v: &[eravon_store::settings::HandOn]) -> Out {
    Out::A(v.iter().map(|h| Out::O(vec![("chainId", Out::N(h.chain)), ("how", Out::S(h.how.clone())), ("id", Out::S(h.id.clone())), ("kind", Out::S(h.kind.clone()))])).collect())
}

fn list_rows(l: &Checklist) -> Vec<(&'static str, Out)> {
    let seat = |s: &eravon_store::vault::Seat| ("seat", Out::S(s.word().to_string()));
    let items = |v: &[(String, u64, [u8; 32], eravon_store::vault::Seat)]| Out::A(v.iter().map(|(id, c, d, st)| Out::O(vec![("chainId", Out::N(*c)), ("digest", Out::S(hex(d))), ("id", Out::S(id.clone())), seat(st)])).collect());
    vec![
        ("cases", Out::A(l.cases.iter().map(|x| Out::S(x.clone())).collect())),
        ("casesUnread", Out::A(l.cases_unread.iter().map(|(c, n, w, st)| Out::O(vec![("chainId", Out::N(*c)), ("name", Out::S(n.clone())), seat(st), ("why", Out::S(w.clone()))])).collect())),
        ("chains", chains(&l.chains)),
        ("deposits", Out::A(l.deposits.iter().map(|x| Out::S(x.clone())).collect())),
        ("fund", Out::S(l.fund.clone().unwrap_or_default())),
        ("handed", Out::A(l.handed.iter().map(|(id, c, inst, st)| Out::O(vec![("chainId", Out::N(*c)), ("id", Out::S(id.clone())), ("instance", Out::S(inst.clone())), seat(st)])).collect())),
        ("historyLinked", Out::B(true)),
        ("offers", items(&l.offers)),
        ("orders", items(&l.orders)),
        ("refunds", Out::A(l.refunds.iter().map(|(id, st)| Out::O(vec![("id", Out::S(id.clone())), seat(st)])).collect())),
        ("refused", Out::A(l.refused.iter().map(|(st, n, w)| Out::O(vec![("name", Out::S(n.clone())), seat(st), ("why", Out::S(w.clone()))])).collect())),
        ("terms", Out::A(l.terms.iter().map(|(id, c, h, _, st)| Out::O(vec![("chainId", Out::N(*c)), ("id", Out::S(id.clone())), seat(st), ("termsHash", Out::S(hex(h)))])).collect())),
        ("txs", Out::N(l.txs)),
        ("txsAtMost", Out::N(l.txs_most)),
        ("owed", owed(&l.owed)),
        ("owedOther", Out::N(l.owed_other as u64)),
        ("stranded", Out::A(l.stranded.iter().map(|(k, c, st)| Out::O(vec![("chainId", Out::N(*c)), ("key", Out::S(k.clone())), seat(st)])).collect())),
        ("unswept", Out::A(l.unswept.iter().map(|c| Out::N(*c)).collect())),
    ]
}

fn swept(d: &[rotate::Swept]) -> Out {
    Out::A(
        d.iter()
            .map(|x| {
                let mut m = vec![("chainId", Out::N(x.chain)), ("from", Out::S(x.from.clone())), ("state", Out::S(x.how.word().to_string()))];
                match &x.how {
                    rotate::Sweep::Moved { tx, amount } => m.extend([("amount", Out::S(amount.clone())), ("tx", Out::S(tx.clone()))]),
                    rotate::Sweep::Short { balance, fee } => m.extend([("balance", Out::S(balance.clone())), ("fee", Out::S(fee.clone()))]),
                }
                Out::O(m)
            })
            .collect(),
    )
}

fn flag_on(f: &Flags, k: &str) -> R<bool> {
    match f.one(k) {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(other) => Err(Fault::Arg(format!("{k} 只收 true 与 false,给的是 {other:?}"))),
    }
}

pub fn rotate(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    if let Some(path) = crate::company::asked(f) {
        return company(c, f, &who, &path);
    }
    let send = crate::send::asked(f);
    let told = f.one("--continue").is_some();
    let mut keep = flag_on(f, "--continue")?;
    let s = c.session(f.one("--passcode"))?;
    let (let_expire, revoke) = (f.many("--let-expire"), f.many("--revoke"));
    let reach = rotate::Reach::of(f)?;
    let leave = flag_on(f, "--leave-coins")?;
    let i = rotate::In { data: &c.data, s: &s, reach: &reach, seat: c.seat, old: &who, now: c.now, let_expire: &let_expire, revoke: &revoke, coins: None };
    let listing = if told && !keep { None } else { Some(rotate::listed(&i)?) };
    let part = match &listing {
        Some((from, _, l)) => rotate::part_way(&c.data, &who, from, l)?,
        None => false,
    };
    if !told {
        keep = part;
    }
    if !send {
        let Some((from, _, l)) = listing.filter(|_| keep) else {
            return Ok(ok(vec![("continue", Out::B(keep)), ("old", Out::S(who.to_ascii_lowercase())), ("unfinished", Out::B(part))]));
        };
        let mut rows = vec![("continue", Out::B(keep)), ("old", Out::S(from)), ("unfinished", Out::B(part))];
        rows.extend(list_rows(&l));
        return Ok(ok(rows));
    }
    let left_on = if leave && keep { rotate::held_on(&i) } else { Vec::new() };
    let none: [String; 0] = [];
    let i = if leave { rotate::In { coins: Some(&none), ..i } } else { i };
    let d = if keep { rotate::continue_on(&i, &c.writer)? } else { rotate::fresh(&i, &c.writer)? };
    let mut rows = vec![
        ("backupAgain", Out::B(d.backup_again)),
        ("chains", chains(&d.chains)),
        ("continue", Out::B(keep)),
        ("left", Out::A(d.left.iter().map(|l| Out::O(l.chain.map(|c| ("chainId", Out::N(c))).into_iter().chain([("id", Out::S(l.what.clone())), ("why", Out::S(l.why()))]).collect())).collect())),
        ("funds", Out::A(d.funds.iter().map(|(ch, f, call)| Out::O(vec![("chainId", Out::N(*ch)), ("data", Out::S(eravon_standard::order::hex_of(&call.data))), ("fund", Out::S(f.clone())), ("to", Out::S(call.to.clone()))])).collect())),
        ("letExpire", Out::A(d.let_expire.iter().map(|x| Out::S(x.clone())).collect())),
        ("new", Out::S(d.new.to_ascii_lowercase())),
        ("old", Out::S(d.finished.clone().unwrap_or_else(|| who.to_ascii_lowercase()))),
        ("sent", Out::A(d.sent.iter().map(|(what, ch, tx)| Out::O(vec![("chainId", Out::N(*ch)), ("tx", Out::S(tx.clone())), ("what", Out::S((*what).to_string()))])).collect())),
        ("swept", swept(&d.swept)),
        ("owed", owed(&d.owed)),
        ("owedOther", Out::N(d.owed_other as u64)),
        ("unswept", Out::A(d.unswept.iter().map(|c| Out::N(*c)).collect())),
    ];
    if let Some(p) = d.phrase {
        rows.push(("phrase", Out::S(p)));
    }
    if !left_on.is_empty() {
        rows.push(("leftOn", Out::A(left_on.into_iter().map(Out::S).collect())));
    }
    Ok(ok(rows))
}

fn company(c: &Ctx, f: &Flags, who: &str, dir: &str) -> R<Out> {
    let to = f.opt("--to").ok_or_else(|| Fault::Arg("公司换钥要 --to(新公司地址)".into()))?;
    let w = crate::shared::wire(f, c, Some(who))?;
    let old = crate::company::company(c, f, &w, who)?;
    let gate = crate::company::time_switch(c, f, who)?;
    let seg = rotate::seg_of(&w, &old.address, Some(&to));
    match &seg {
        Seg::Moved(k) => return Err(rotate::named(rotate::E_ROTATE_MOVED, format!("{} 的记录已交给 {k},它不再是现行钥", old.address))),
        Seg::Claim(k) if !k.eq_ignore_ascii_case(&to) => return Err(rotate::named(rotate::E_ROTATE_MOVED, format!("{} 已声明交给 {k},不是 {to}", old.address))),
        Seg::Open if eravon_standard::chain::entered(&w.t, &w.table, &to)?.value => {
            return Err(rotate::named(rotate::E_ROTATE_ENTERED, format!("新公司 {to} 在换钥簿上已经有记录,不能被声明")));
        }
        _ => {}
    }
    let (declare, accede) = rotate::company_plans(&old.address, &to)?;
    let new = crate::company::Company { address: to.to_ascii_lowercase(), views: eravon_standard::company::open(&w.t, &w.table, &to)?.map_err(|sh| Fault::State(format!("新公司地址读不成公司:{}", sh.why)))?.0 };
    let one = |co: &crate::company::Company, p: eravon_standard::proposal::Plan, leaf: &str| -> R<Out> {
        let mut e = crate::company::Export::of(p);
        e.deal = crate::company::Deal::None;
        let path = std::path::Path::new(dir).join(leaf).display().to_string();
        Ok(Out::O(crate::company::export(c, &w, co, &path, gate, e)?))
    };
    let rows = vec![("accede", one(&new, accede, "accede.json")?), ("declare", one(&old, declare, "declare.json")?), ("new", Out::S(new.address.clone())), ("old", Out::S(old.address.to_ascii_lowercase())), ("state", Out::S(seg.word().to_string()))];
    Ok(ok(rows))
}
