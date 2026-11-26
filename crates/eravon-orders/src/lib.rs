#![forbid(unsafe_code)]

pub mod day;
pub mod detail;
pub mod ended;
pub mod label;
pub mod status;
pub mod table;
pub mod when;

pub use eravon_watch::{Why, Wire, why};

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S2;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

fn a3w(e: eravon_scan::fault::Fault) -> Why {
    why(&e.name(), e.detail())
}

fn a5w(e: eravon_store::fault::Fault) -> Why {
    why(e.name(), e.detail())
}
