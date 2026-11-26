#![forbid(unsafe_code)]

pub mod activity;
pub mod cards;
pub mod notes;
pub mod toast;

pub use eravon_watch::{Why, Wire, why};

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S8;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

pub fn chain_now(w: &Wire<'_>) -> Result<(u64, u64), Why> {
    let (n, _) = eravon_chain::clock::now(w.t, w.table).map_err(|e| why(e.name(), e.detail()))?;
    Ok((n.timestamp, n.block))
}
