use super::*;

pub(super) fn list(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let ks = eravon_vault::keyset::buyer(&*c.session(f.one("--passcode"))?, &who).map_err(home::down)?;
    let w = shared::wire(f, c, Some(&who))?;
    let from = f.int_or("--from-block", 0).map_err(|_| Fault::Arg("--from-block 要是整数".into()))?;
    let to = shared::upto(f)?;
    let sweep = eravon_scan::scan::by_keys(&w.t, &w.table, &w.logic, eravon_scan::scan::Side::Buyer, &ks.keys, from, to, &[])?;
    let mut local = Vec::new();
    for owner in &ks.owners {
        local.extend(home::or_missing((|| Ok(c.read()?.list(owner, Box_::Packs)?))())?);
    }
    let on_chain: Vec<String> = sweep.cases.iter().map(|x| x.case_id.clone()).collect();
    let pending: Vec<Out> = local
        .iter()
        .filter(|r| r.field("caseId").is_some_and(|c| !on_chain.iter().any(|x| x.eq_ignore_ascii_case(c))))
        .map(|r| r.row())
        .collect();
    let mut rows = vec![
        ("agreement", Out::S(sweep.agreement.to_string())),
        (
            "cases",
            Out::A(
                sweep
                    .cases
                    .iter()
                    .map(|x| {
                        Out::O(vec![
                            ("block", Out::N(x.block)),
                            ("caseId", Out::S(x.case_id.clone())),
                            ("instance", Out::S(x.instance.clone())),
                            ("seller", Out::S(x.seller.clone())),
                            ("tx", Out::S(x.tx.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("count", Out::N(sweep.cases.len() as u64)),
        ("foreign", Out::N(sweep.foreign)),
        ("pending", Out::A(pending)),
        ("rows", page_rows(c, f, &who, &w)?),
    ];
    rows.extend(w.rows());
    Ok(ok(rows))
}

fn page_rows(c: &Ctx, f: &Flags, who: &str, w: &shared::Wire) -> R<Out> {
    use crate::deskops::buyerpage as bp;
    let done = match f.opt("--tab").as_deref() {
        None => None,
        Some("active") => Some(false),
        Some("done") => Some(true),
        Some(x) => return Err(Fault::Arg(format!("--tab 只有 active 与 done 两种,给的是 {x:?}"))),
    };
    let session = c.session(f.one("--passcode"))?;
    let tokens = eravon_vault::tokens::list(&c.data);
    let shut = |at: String, kind: String, why: String| format!("{at} · {kind} · {why}");
    let page = bp::Page { tokens: &tokens, private_goods: "", company_shut: &shut, company_row: bp::Page::company_row_of, column_holder: bp::column_holder };
    let mut owners = Vec::new();
    let t = match bp::rows_of(&session, &c.data, who, w, &page, &mut owners) {
        Ok(t) => t,
        Err(s) => return Ok(s.out()),
    };
    let (shown, bad) = bp::shown_of(&t.rows, done, f.one("--q").unwrap_or(""), f.one("--from").unwrap_or(""), f.one("--to").unwrap_or(""));
    if let Some(s) = bad {
        return Err(Fault::Arg(format!("{}:{}", s.name, s.detail)));
    }
    let list = shown.into_iter().map(|r| bp::row_out(r, &tokens, bp::read_only)).collect();
    let mut out = vec![("list", Out::A(list))];
    if let Some(s) = t.unscanned {
        out.push(("unscanned", s.out()));
    }
    Ok(Out::O(out))
}

pub(super) fn show(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let from = f.int_or("--from-block", 0).map_err(|_| Fault::Arg("--from-block 要是整数".into()));
    let to = shared::upto(f);
    let hist = match (&from, &to) {
        (Ok(from), Ok(to)) => Some(eravon_scan::scan::history(&w.t, &w.table, w.chain_id, &w.logic, &instance, *from, *to)),
        _ => None,
    };
    let (row, unopened, detail) = page_row(c, f, &who, &w, &instance)?;
    let mut rows = if unopened {
        vec![
            ("agreement", Out::Null),
            ("feeQuoted", Out::A(Vec::new())),
            ("history", Out::A(Vec::new())),
            ("row", row),
            ("instance", Out::S(instance)),
            ("status", Out::Null),
        ]
    } else {
        let (st, fq) = a1::chain::status_and_fee(&w.t, &w.table, &instance)?;
        let (_, _) = (from?, to?);
        let hist = hist.unwrap_or_else(|| Ok(Vec::new()))?;
        vec![
            ("agreement", Out::S(st.agreement.to_string())),
            ("feeQuoted", Out::A(fq.value.iter().map(word_row).collect())),
            (
                "history",
                Out::A(
                    hist.iter()
                        .map(|m| {
                            Out::O(vec![
                                ("block", Out::N(m.block)),
                                ("name", Out::S(m.name.clone())),
                                ("tx", Out::S(m.tx.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("row", row),
            ("instance", Out::S(instance)),
            ("status", status_row(&st.value)),
        ]
    };
    rows.push(("detail", detail.unwrap_or(Out::Null)));
    rows.extend(w.rows());
    Ok(ok(rows))
}

fn page_row(c: &Ctx, f: &Flags, who: &str, w: &shared::Wire, instance: &str) -> R<(Out, bool, Option<Out>)> {
    use crate::deskops::buyerpage as bp;
    let session = c.session(f.one("--passcode"))?;
    let tokens = eravon_vault::tokens::list(&c.data);
    let shut = |at: String, kind: String, why: String| format!("{at} · {kind} · {why}");
    let page = bp::Page { tokens: &tokens, private_goods: "", company_shut: &shut, company_row: bp::Page::company_row_of, column_holder: bp::column_holder };
    let mut owners = Vec::new();
    Ok(match bp::row_one(&session, &c.data, who, w, &page, &mut owners, instance) {
        Ok(t) => t.rows.iter().find(|r| r.row.instance.eq_ignore_ascii_case(instance)).map_or((Out::Null, false, None), |r| (bp::row_out(r, &tokens, bp::read_only), r.budget.is_some(), Some(bp::detail_out(&c.data, r, w, &owners, bp::fee_line)))),
        Err(s) => (s.out(), false, None),
    })
}

pub fn status_row(v: &[Val]) -> Out {
    use eravon_standard::chain::status_col as c;
    let mut names: [(&str, usize); 16] = [
        ("claimedAt", c::CLAIMED_AT),
        ("deposit", c::DEPOSIT),
        ("disputedAt", c::DISPUTED_AT),
        ("drawBlock", c::DRAW_BLOCK),
        ("lfHeld", c::LF_HELD),
        ("panelHash", c::PANEL_HASH),
        ("panelIndex", c::PANEL_INDEX),
        ("panelSize", c::PANEL_SIZE),
        ("pendingRedraw", c::PENDING_REDRAW),
        ("phase", c::PHASE),
        ("premium", c::PREMIUM),
        ("redrawDeadline", c::REDRAW_DEADLINE),
        ("root", c::ROOT),
        ("router", c::ROUTER),
        ("seats", c::SEATS),
        ("unpaid", c::UNPAID),
    ];
    names.sort_by_key(|(n, _)| *n);
    Out::O(names.iter().map(|(n, i)| (*n, v.get(*i).map(word_row).unwrap_or(Out::S(String::new())))).collect())
}

pub(super) fn word_row(v: &Val) -> Out {
    match v {
        Val::Bool(b) => Out::B(*b),
        Val::B32(w) => Out::S(hex(w)),
        Val::Addr(a) => Out::S(hex(a)),
        Val::Bytes(b) => Out::S(hex(b)),
        Val::Str(x) => Out::S(x.clone()),
        Val::U256(w) => Out::S(k2::u256::U256::from_be(w).to_dec()),
        Val::Arr(xs) | Val::Tuple(xs) => Out::A(xs.iter().map(word_row).collect()),
    }
}
