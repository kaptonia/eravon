use super::{write_identity, No};
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_store::settings::Identity;

pub const R_FORM: &str = "E_S9_COMPANY_FORM";
pub const R_SHUT: &str = "E_S9_COMPANY_SHUT";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    pub bound: u64,
    pub wallet: String,
    pub ledger: String,
    pub agreement: String,
    pub endpoints: usize,
}

pub fn plan<W>(address: &str, wire: impl FnOnce() -> Result<W, No>) -> Result<(), No> {
    if !eravon_chain::seam::address_ok(address.trim()) {
        return Err(No::new(R_FORM, vec![format!("{address:?}")]));
    }
    wire().map(|_| ())
}

pub fn gate<T: Transport>(address: &str, wire: impl FnOnce() -> Result<(T, Table), No>) -> Result<Open, No> {
    let mut got = None;
    plan(address, || wire().map(|w| got = Some(w)))?;
    let (t, table) = got.expect("plan built the wire");
    match eravon_standard::company::open(&t, &table, address.trim()) {
        Err(f) => Err(No::new(&f.name(), vec![f.detail()])),
        Ok(Err(s)) => Err(No::new(R_SHUT, vec![s.at.to_string(), s.kind.word().to_string(), s.why])),
        Ok(Ok((v, _))) => Ok(Open { bound: v.calendar_bound, wallet: v.wallet, ledger: v.ledger, agreement: v.agreement.to_string(), endpoints: v.endpoints }),
    }
}


fn card_write(t: &mut Identity, on: bool, address: &str) {
    t.company_mode = Some(on);
    if on {
        company_to(t, address);
    }
}


fn company_to(t: &mut Identity, address: &str) {
    t.move_company(address);
}


fn time_write(t: &mut Identity, on: bool) {
    t.time_check = Some(on);
}


pub fn set(data: &str, writer: &str, now: u64, me: &str, on: bool, address: &str) -> Result<(), No> {
    let address = address.trim().to_string();
    write_identity(data, writer, now, me, |t| card_write(t, on, &address))
}

pub fn time_check(data: &str, writer: &str, now: u64, me: &str, on: bool) -> Result<(), No> {
    write_identity(data, writer, now, me, |t| time_write(t, on))
}
