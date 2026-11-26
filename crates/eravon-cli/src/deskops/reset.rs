use eravon_store::vault::Seat;
use eravon_vault as h2;

pub struct Reset {
    pub address: String,
    pub phrase: Option<String>,
    pub trouble: Vec<String>,
    pub born: Result<h2::onboard::Born, h2::vfault::Fault>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(session: &mut h2::session::Session, store: &h2::store::Store, data: &str, seat: Seat, label: &str, named: bool, phrase: Option<&str>, passcode: &str, now: u64) -> Result<Reset, h2::vfault::Fault> {
    let (address, phrase, mut trouble) = h2::accounts::new_primary(session, seat, label, named, phrase, passcode, now)?;
    let b = h2::onboard::born_added(store, data, &address, eravon_literals::constants::current_network().standard.chain_id, now);
    for o in b.others {
        if let Err(e) = o {
            trouble.push(format!("{}:{}", e.name(), e.detail()));
        }
    }
    Ok(Reset { address, phrase, trouble, born: b.own })
}
