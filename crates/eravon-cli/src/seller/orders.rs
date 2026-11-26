use super::*;

pub(super) fn s2(e: eravon_orders::Why) -> Fault {
    Fault::Down { layer: "S2", token: e.0, why: e.1 }
}

pub(super) fn order_row(f: &Flags, tokens: &[eravon_store::settings::TokenRow], r: &eravon_orders::table::Row) -> Out {
    let (state, say, reason) = match &r.status {
        Ok(s) => (s.word.word().to_string(), s.word.say().to_string(), s.reason.clone()),
        Err((n, d)) => (String::new(), n.clone(), d.clone()),
    };
    let d = eravon_vault::tokens::decimals_of(tokens, eravon_vault::tokens::chain(), &r.token, r.decimals);
    let mut m = shared::units_rows(f, "amount", &r.amount, d);
    m.extend(crate::deskops::orderpage::row_cells(r).into_iter().filter(|(k, _)| !matches!(*k, "state" | "reason")));
    m.extend(vec![
        ("amountWord", Out::S(eravon_orders::table::amount_word(tokens, &r.amount, r.decimals, &r.token, eravon_vault::tokens::Form::Plain))),
        ("deadline", Out::N(r.deadline)),
        ("placed", Out::S(eravon_orders::day::of(r.placed))),
        ("reason", Out::S(reason)),
        ("say", Out::S(say)),
        ("state", Out::S(state)),
        ("token", Out::S(r.token.clone())),
    ]);
    Out::O(m)
}

pub(super) fn order_list(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let t = eravon_orders::table::table(&c.data, &who, eravon_scan::scan::Side::Seller, &sw).and_then(eravon_orders::table::Table::complete).map_err(s2)?;
    let hits = eravon_orders::table::search(&t.rows, f.one("--q").unwrap_or(""), f.one("--from").unwrap_or(""), f.one("--to").unwrap_or("")).map_err(s2)?;
    let tab = f.opt("--tab");
    let book = eravon_vault::tokens::list(&c.data);
    let rows: Vec<Out> = hits
        .into_iter()
        .filter(|r| match tab.as_deref() {
            Some("active") => !eravon_orders::table::done(r),
            Some("done") => eravon_orders::table::done(r),
            _ => true,
        })
        .map(|r| order_row(f, &book, r))
        .collect();
    let mut out = vec![
        ("agreement", Out::S(t.agreement.clone())),
        ("count", Out::N(rows.len() as u64)),
        ("foreign", Out::N(t.foreign)),
        ("rows", Out::A(rows)),
    ];
    out.extend(w.rows());
    Ok(ok(out))
}

pub(super) fn order_show(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let t = eravon_orders::table::table_one(&c.data, &who, eravon_scan::scan::Side::Seller, &sw, &instance).and_then(eravon_orders::table::Table::complete).map_err(s2)?;
    let r = eravon_orders::table::find(&t, &instance).map_err(s2)?;
    let o = eravon_orders::detail::order_of(&c.data, &who, r);
    let (det, refused) = crate::deskops::orderpage::read_detail(&c.data, &who, &sw, r);
    let ov = &det.overview;
    let hist = det.record.clone().map_err(|e| s2((e.name, e.detail)))?;
    let fee = &det.fee;
    let (st, _) = a1::chain::status_and_fee(&w.t, &w.table, &instance)?;
    let book = eravon_vault::tokens::list(&c.data);
    let d = eravon_vault::tokens::decimals_of(&book, eravon_vault::tokens::chain(), &r.token, r.decimals);
    let terms = match &ov.terms {
        eravon_orders::detail::Terms::Ok(_) => "ok".to_string(),
        eravon_orders::detail::Terms::Missing => "missing".to_string(),
        eravon_orders::detail::Terms::Unreadable(..) => "unreadable".to_string(),
        eravon_orders::detail::Terms::Mismatch(..) => "mismatch".to_string(),
    };
    let mut rows = vec![
        ("agreement", Out::S(st.agreement.to_string())),
        (
            "fee",
            Out::O({
                let mut m = Vec::new();
                for (x, v) in [("amount", &fee.amount), ("judgeFee", &fee.judge_fee), ("lf", &fee.lf.0), ("lfHeld", &fee.lf.1)] {
                    m.extend(shared::units_rows(f, x, v, d));
                }
                m.extend(vec![
                    ("amount", Out::S(fee.amount.clone())),
                    ("judgeFee", Out::S(fee.judge_fee.clone())),
                    ("lf", Out::S(fee.lf.0.clone())),
                    ("lfHeld", Out::S(fee.lf.1.clone())),
                ]);
                m
            }),
        ),
        (
            "history",
            Out::A(
                hist.iter()
                    .map(|m| {
                        Out::O(vec![
                            ("at", Out::S(eravon_orders::day::at(m.at))),
                            ("name", Out::S(m.name.clone())),
                            ("say", Out::S(m.say.clone())),
                            ("tx", Out::S(m.tx.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("instance", Out::S(instance)),
        ("moments", Out::N(hist.len() as u64)),
        ("order", order_row(f, &book, r)),
        ("status", crate::buyer::status_row(&st.value)),
        ("terms", Out::S(terms)),
    ];
    if let Some(h) = det.delivery.hand_over.clone() {
        rows.push(("handOver", super::actions::hand_over_row(h)));
    }
    rows.push(("detail", crate::deskops::orderpage::detail_out(&det, r, o.as_ref(), &book, false)));
    if let Some(e) = refused {
        rows.push(("said", e.out()));
    }
    rows.extend(w.rows());
    Ok(ok(rows))
}

#[cfg(test)]
mod page_cells {
    #[test]
    fn a_verb_row_carries_every_cell_of_the_page_row() {
        let r = eravon_orders::table::Row {
            instance: "0x00000000000000000000000000000000000000a1".into(),
            case_id: String::new(),
            record: String::new(),
            number: String::new(),
            buyer: String::new(),
            label: String::new(),
            product: String::new(),
            amount: "10".into(),
            token: String::new(),
            status: Err(("E_X".into(), "读不成".into())),
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
            series: Some(eravon_terms::doc::Series { id: format!("0x{}", "22".repeat(32)), stage: 1, stages: 2 }),
        };
        let f = crate::shared::Flags::read(&[]).expect("旗");
        let eravon_literals::json::Out::O(m) = super::order_row(&f, &[], &r) else { panic!("行是对象") };
        let keys: Vec<&str> = m.iter().map(|(k, _)| *k).collect();
        let page: Vec<&str> = crate::deskops::orderpage::row_cells(&r).iter().map(|(k, _)| *k).chain(["amountWord", "deadline", "placed", "say", "token"]).collect();
        for k in page {
            assert!(keys.contains(&k), "动词的行缺页上的 {k}");
        }
    }
}
