use eravon_literals::misuse::Misuse;
use super::*;
use eravon_standard::chain::{panel_col, seat_col};

pub(super) fn table(f: &Flags) -> Result<Result<Table, a2::fault::Fault>, Misuse> {
    let inline = f.many("--endpoint");
    if inline.is_empty() {
        return Err(Misuse::missing("--endpoint"));
    }
    Ok(Table::new(inline.iter().map(|x| (*x).to_string()).collect()))
}

pub(super) fn wire(f: &Flags) -> a2::fault::R<Auto> {
    Auto::for_flags(f.one("--ca-file"), f.one("--proxy"))
}

pub(super) fn words(v: &[a1::abi::Val]) -> Out {
    Out::A(
        v.iter()
            .map(|x| match x {
                a1::abi::Val::Addr(a) => Out::S(order::hex_of(a)),
                a1::abi::Val::Bool(b) => Out::B(*b),
                a1::abi::Val::U256(w) | a1::abi::Val::B32(w) => Out::S(order::hex_of(w)),
                other => Out::S(format!("{other:?}")),
            })
            .collect(),
    )
}

pub(super) fn read(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let what = need(f, "--what")?;
    let (chain, logic) = chain_and_logic(f)?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let inst = f.one("--instance").unwrap_or(&logic).to_string();
        let out = match what.as_str() {
            "code" => {
                let r = a1::chain::code_matches(&w, &t, &inst, &logic)?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("matches", Out::B(r.value)),
                ]
            }
            "verifier" => {
                let r = a1::chain::verifier_present(&w, &t)?;
                let (present, got, want) = r.value.clone();
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("codeHash", Out::S(got)),
                    ("present", Out::B(present)),
                    ("verifier", Out::S(k7::VERIFIER.to_string())),
                    ("want", Out::S(want)),
                ]
            }
            "board" => {
                let r = a1::chain::board_present(&w, &t)?;
                let (present, got, want) = r.value.clone();
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("board", Out::S(k7::BOARD.to_string())),
                    ("codeHash", Out::S(got)),
                    ("present", Out::B(present)),
                    ("want", Out::S(want)),
                ]
            }
            "status" => {
                use a1::chain::status_col as col;
                let r = a1::chain::status(&w, &t, &inst)?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("phase", Out::N(r.value[col::PHASE].small()?)),
                    ("seats", Out::N(r.value[col::SEATS].small()?)),
                    ("values", words(&r.value)),
                ]
            }
            "revoked" => {
                let th = order::b32_of("termsHash", &flag(f, "--key")?)?;
                let r = a1::chain::revoked(&w, &t, &logic, &flag(f, "--who")?, &th)?;
                vec![("agreement", Out::S(r.agreement.to_string())), ("revoked", Out::B(r.value))]
            }
            "funds" => {
                let r = a1::chain::funds(&w, &t, &logic, &flag(f, "--who")?, &flag(f, "--fund")?)?;
                vec![("agreement", Out::S(r.agreement.to_string())), ("bound", Out::B(r.value))]
            }
            "check" => {
                let o = read_order(&flag(f, "--order")?)?;
                let quote = order::bytes_of("报价", &flag(f, "--quote")?)?;
                let r = a1::chain::verifier_check(&w, &t, &o, &quote)?;
                let c = &r.value;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("cap", Out::S(k2::u256::U256::from_be(&c.cap).to_dec())),
                    ("lf", Out::S(k2::u256::U256::from_be(&c.lf).to_dec())),
                    ("overflow", Out::B(c.overflow)),
                    ("quoteBad", Out::B(c.quote_bad)),
                    ("stale", Out::B(c.stale)),
                    ("termsFault", Out::N(c.terms_fault)),
                ]
            }
            "fee" => {
                let r = a1::chain::fee_quoted(&w, &t, &inst)?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("values", words(&r.value)),
                ]
            }
            "seat" => {
                let case_id = order::b32_of("案号", &flag(f, "--case-id")?)?;
                let seat = f.one("--seat").unwrap_or("0").parse::<u64>().unwrap_or(0);
                let r = a1::chain::seat_of(&w, &t, &inst, &case_id, seat)?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("judge", Out::S(order::hex_of(&r.value[seat_col::JUDGE].addr()?))),
                    ("state", Out::N(r.value[seat_col::STATE].small()?)),
                    ("values", words(&r.value[..=seat_col::AMOUNT])),
                ]
            }
            "account" => {
                let r = a1::chain::account_of(
                    &w,
                    &t,
                    &inst,
                    &flag(f, "--who")?,
                    &flag(f, "--token")?,
                )?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("claimable", Out::S(order::hex_of(&r.value.1))),
                    ("payout", Out::S(r.value.0.clone())),
                ]
            }
            "hash-offer" => {
                let path = flag(f, "--order")?;
                let o = read_order(&path)?;
                let data = a1::call::hash_offer(&o)?;
                let (hex, r) = a2::read::call(&w, &t, k7::VERIFIER, &order::hex_of(&data), a2::seam::TAG_LATEST)?;
                let on_chain = order::bytes_of("答", &hex)?;
                let mine = o.offer.struct_hash();
                vec![
                    ("agreement", Out::S(r.agreement.word().to_string())),
                    ("k2", Out::S(order::hex_of(&mine))),
                    ("onChain", Out::S(order::hex_of(&on_chain))),
                    ("same", Out::B(on_chain == mine)),
                    ("verifier", Out::S(k7::VERIFIER.to_string())),
                ]
            }
            "panel-book" => {
                let r = a1::chain::panel_book(&w, &t, &logic)?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("book", Out::S(r.value.clone())),
                ]
            }
            "book-latest" => {
                let r = a1::chain::book_latest(
                    &w,
                    &t,
                    &flag(f, "--book")?,
                    &flag(f, "--guild")?,
                    &order::b32_of("键", &flag(f, "--key")?)?,
                )?
                .ok_or_else(|| a1::fault::Fault::Form("名单簿在这一根这一键下没有记录".into()))?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("expires", Out::N(r.value[1 + panel_col::EXPIRES].small()?)),
                    ("index", Out::N(r.value[0].small()?)),
                    ("panelHash", Out::S(order::hex_of(&r.value[1 + panel_col::PANEL_HASH].word()?))),
                    ("recorded", Out::N(r.value[1 + panel_col::RECORDED].small()?)),
                    ("rosterHash", Out::S(order::hex_of(&r.value[1 + panel_col::ROSTER_HASH].word()?))),
                    ("router", Out::S(order::hex_of(&r.value[1 + panel_col::ROUTER].addr()?))),
                    ("routerHash", Out::S(order::hex_of(&r.value[1 + panel_col::ROUTER_HASH].word()?))),
                ]
            }
            "price" => {
                let r = a1::feed::latest(&w, &t, &flag(f, "--feed")?)?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("answer", Out::S(r.answer.clone())),
                    ("answeredInRound", Out::S(order::hex_of(&r.answered_in_round))),
                    ("decimals", Out::N(r.decimals)),
                    ("roundId", Out::S(order::hex_of(&r.round_id))),
                    ("startedAt", Out::N(r.started_at)),
                    ("updatedAt", Out::N(r.updated_at)),
                ]
            }
            "balance" => {
                let r = a1::chain::erc20_balance(&w, &t, &flag(f, "--token")?, &flag(f, "--who")?)?;
                vec![
                    ("agreement", Out::S(r.agreement.to_string())),
                    ("balance", Out::S(order::hex_of(&r.value))),
                ]
            }
            "consented" => {
                let digest = order::b32_of("摘要", &flag(f, "--digest")?)?;
                let r = a1::consent::registered(&w, &t, &logic, &flag(f, "--who")?, &digest);
                vec![
                    ("endpoints", Out::N(r.endpoints as u64)),
                    ("state", Out::S(r.state.word().to_string())),
                    ("why", Out::S(r.why.clone())),
                ]
            }
            "settle-gate" => {
                let act = k2::derive::Action::Settle {
                    buyer_bps: numf(f, "--buyer-bps")?,
                    expiry: k2::u256::U256::from_u64(numf(f, "--settle-expiry")?),
                };
                let d = k2::derive::digest(chain, &inst, &act.struct_hash());
                let (g, r) = a1::consent::settle_gate(&w, &t, &logic, &flag(f, "--who")?, &d);
                vec![
                    ("digest", Out::S(order::hex_of(&d))),
                    ("endpoints", Out::N(r.endpoints as u64)),
                    ("gate", Out::S(g.word().to_string())),
                    ("say", Out::S(g.say().to_string())),
                    ("why", Out::S(r.why.clone())),
                ]
            }
            other => return Err(Fault::Form(format!("读面之外的词:{other:?}"))),
        };
        Ok(ok(out))
    })())
}
