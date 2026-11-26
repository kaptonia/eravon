use eravon_cands::fault::Fault;
use eravon_cands::table::Side;
use eravon_store::record::Candidate;

#[derive(Debug)]
pub enum No {
    Cands(Fault),
    Stale,
}

pub fn plan(data: &str, me: &str, side: Side, input: &str, label: &str) -> Result<String, Fault> {
    let a = eravon_cands::table::target(side, input)?;
    eravon_cands::table::need_label(label)?;
    if let Ok(rows) = eravon_cands::table::list(data, me) {
        if rows.iter().any(|r| r.address == a) {
            return Err(Fault::Exists(a));
        }
        eravon_vault::store::candidates_fit(rows.len() + 1)?;
    }
    Ok(a)
}

#[allow(clippy::too_many_arguments)]
pub fn add(data: &str, writer: &str, now: u64, me: &str, side: Side, input: &str, label: &str, route: &eravon_goods::company::Route, registries: &[String], still: &dyn Fn() -> bool) -> Result<Candidate, No> {
    let a = plan(data, me, side, input, label).map_err(No::Cands)?;
    let seen = eravon_cands::view::seen_now(route, side, &a, registries, now);
    if !still() {
        return Err(No::Stale);
    }
    eravon_cands::table::add(data, writer, now, me, side, input, label, &seen).map_err(No::Cands)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: &str = "48261537";
    const PHRASE: &str = "test test test test test test test test test test test junk";
    const NOW: u64 = 1_790_000_000;
    const WRITER: &str = "cands-test";

    #[test]
    fn a_full_table_refuses_the_next_entry_before_reading_the_chain() {
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create(PIN, eravon_store::vault::Seat::Seller, "主钥", Some(PHRASE), NOW).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut s = eravon_vault::session::Session::locked(&data);
        s.unlock(PIN).expect("开得了");
        let me = eravon_vault::accounts::who(&data, eravon_store::vault::Seat::Seller).expect("首枚钥").to_ascii_lowercase();
        let st = eravon_vault::store::open_write(&data, WRITER, NOW).expect("可写");
        st.init(&eravon_store::settings::Global::default()).expect("铺得下");
        st.init_identity(&me, NOW).expect("登记身份");
        let judge = |i: usize| format!("0x{:040x}", 0xb000 + i);
        let route: eravon_goods::company::Route = Err(("E_TEST_NO_ROUTE".into(), "测试不连网".into()));
        let asked = std::cell::Cell::new(0);
        let still = || {
            asked.set(asked.get() + 1);
            true
        };
        let cap = eravon_literals::constants::CANDIDATE_CAP;
        for i in 0..cap - 1 {
            eravon_cands::table::add(&data, WRITER, NOW, &me, Side::Judge, &judge(i), "备注", "").expect("有位");
        }
        add(&data, WRITER, NOW, &me, Side::Judge, &judge(cap - 1), "第八", &route, &[], &still).expect("第八位有位");
        assert_eq!(asked.get(), 1, "the eighth went on to the read");
        match add(&data, WRITER, NOW, &me, Side::Judge, &judge(cap), "第九", &route, &[], &still) {
            Err(No::Cands(Fault::Store { name, .. })) => assert_eq!(name, eravon_vault::store::candidates_fit(cap + 1).unwrap_err().name(), "refused by the capacity rule"),
            other => panic!("the ninth is refused: {other:?}"),
        }
        assert_eq!(asked.get(), 1, "the ninth was refused before the chain was read for it");
    }
}
