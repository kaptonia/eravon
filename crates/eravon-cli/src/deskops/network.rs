use super::{write_identity, No};

pub const NETWORK_UNKNOWN: &str = "E_S9_NETWORK_UNKNOWN";

pub fn known(typed: &str) -> Result<eravon_literals::constants::Network, No> {
    let typed = typed.trim();
    typed.parse::<u64>().ok().and_then(eravon_literals::constants::network_of).ok_or_else(|| No::new(NETWORK_UNKNOWN, vec![typed.to_string()]))
}

pub fn in_effect(chain: u64) {
    eravon_literals::constants::select_chain(chain);
    eravon_chain::read::forget_checked();
    eravon_vault::chainx::forget_serving();
}

pub fn choose_by(typed: &str, write: impl FnOnce(u64) -> Result<(), No>) -> Result<eravon_literals::constants::Network, No> {
    let n = known(typed)?;
    write(n.standard.chain_id)?;
    in_effect(n.standard.chain_id);
    Ok(n)
}

pub fn set_past_cases(data: &str, writer: &str, now: u64, on: bool) -> Result<(), No> {
    let s = eravon_vault::store::open_write(data, writer, now).map_err(No::store)?;
    let mut g = eravon_vault::store::global_to_change(s.data()).map_err(No::store)?;
    g.past_cases = on;
    eravon_vault::store::write_global(s.data(), &g).map_err(No::store)
}

pub fn set_zikaron(data: &str, writer: &str, now: u64, me: &str, dir: &str) -> Result<(), No> {
    let dir = dir.trim().to_string();
    write_identity(data, writer, now, me, |t| t.zikaron_dir = dir)
}

pub fn set_zikaron_link(data: &str, writer: &str, now: u64, me: &str, on: bool) -> Result<(), No> {
    write_identity(data, writer, now, me, |t| t.zikaron_link = Some(on))
}

pub const ZK_OFF: &str = "E_S9_ZK_OFF";

pub fn zk_linked(ident: &eravon_store::settings::Identity) -> bool {
    ident.zikaron_on()
}


pub fn zikaron_of(data: &str, me: &str) -> Result<(bool, String), No> {
    let ident = match eravon_vault::store::read_identity(data, me) {
        Ok(i) => i,
        Err(eravon_store::fault::Fault::Missing(_)) => eravon_store::settings::Identity::default(),
        Err(e) => return Err(No::store(e)),
    };
    Ok((zk_linked(&ident), ident.zikaron_dir.clone()))
}

#[cfg(test)]
mod past_tests {
    use eravon_store::vault::Seat;

    const PIN: &str = "48261537";
    const PHRASE: &str = "test test test test test test test test test test test junk";

    #[test]
    fn the_past_switch_is_written_in_one_place_and_put_in_effect() {
        let data = eravon_vault::testing::fresh();
        eravon_vault::onboard::land(&data, eravon_vault::onboard::Mode::First, Seat::Seller, "主钥", false, eravon_vault::onboard::Source::Words(PHRASE), PIN, None, None, 900).expect("首启");
        let mut s = eravon_vault::session::Session::locked(&data);
        s.unlock(PIN).expect("opens");
        for on in [true, false, true] {
            super::set_past_cases(&data, "past-test", 900, on).expect("written");
            assert_eq!(eravon_vault::store::read_global(&data).expect("read").past_cases, on);
            assert_eq!(eravon_literals::constants::past_switch(), on, "in effect at once");
        }
        super::set_past_cases(&data, "past-test", 900, false).expect("back off");
    }
}
