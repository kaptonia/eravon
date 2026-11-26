#![forbid(unsafe_code)]

pub mod company;
pub mod complaint;
pub mod entries;
pub mod evidence;
pub mod refund;
pub mod seal;
pub mod seat;
pub mod settle;
pub mod sweep;
pub mod tick;
pub mod view;

pub use eravon_watch::{Why, Wire, why};

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S6;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

fn a1w(e: eravon_standard::fault::Fault) -> Why {
    why(&e.name(), e.detail())
}

fn case_of(o: &eravon_watch::order::Order) -> Result<[u8; 32], Why> {
    eravon_standard::order::b32_of("案号", &o.case_id).map_err(a1w)
}

pub mod seat_col {
    pub use eravon_standard::sol::src_abi::{
        I_GALEED_VENUE_T_SEAT_COL_ACCEPTED as ACCEPTED, I_GALEED_VENUE_T_SEAT_COL_AMOUNT as AMOUNT,
        I_GALEED_VENUE_T_SEAT_COL_JUDGE as JUDGE, I_GALEED_VENUE_T_SEAT_COL_PARTIES as PARTIES,
        I_GALEED_VENUE_T_SEAT_COL_STATE as STATE, I_GALEED_VENUE_T_SEAT_COL_TOKEN as TOKEN,
        I_GALEED_VENUE_T_SEAT_COL_WINDOW_END as WINDOW_END, I_GALEED_VENUE_T_SEAT_COL_WINDOW_START as WINDOW_START,
    };
    pub const PARTY_BUYER_KEY: usize = 0;
    pub const PARTY_SELLER_KEY: usize = 1;
    pub const PARTY_BUYER_PAYOUT: usize = 3;
    pub const PARTY_SELLER_FUND: usize = 2;
    pub const PARTY_SELLER_PAYOUT: usize = 4;
    pub const STATE_RULING: u64 = 1;
    pub const STATE_RULED: u64 = 2;
    pub const STATE_SEATED: u64 = 0;
}

fn status(w: &Wire<'_>, instance: &str) -> Result<Vec<eravon_standard::abi::Val>, Why> {
    Ok(eravon_standard::chain::status(w.t, w.table, instance)
        .map_err(a1w)?
        .value)
}

fn small(v: &[eravon_standard::abi::Val], i: usize) -> Result<u64, Why> {
    v.get(i)
        .ok_or_else(|| why("E_DISPUTE_READ", format!("第 {i} 栏缺")))?
        .small()
        .map_err(a1w)
}

fn big(v: &[eravon_standard::abi::Val], i: usize) -> Result<eravon_terms::u256::U256, Why> {
    let w = v
        .get(i)
        .ok_or_else(|| why("E_DISPUTE_READ", format!("第 {i} 栏缺")))?
        .word()
        .map_err(a1w)?;
    Ok(eravon_terms::u256::U256::from_be(&w))
}

pub fn open_phase(phase: u64) -> bool {
    (1..=3).contains(&phase)
}
