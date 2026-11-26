#![forbid(unsafe_code)]

pub mod company;
pub mod deliver;
pub mod todo;

pub use eravon_watch::{Why, Wire, why};

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S5;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

fn a1w(e: eravon_standard::fault::Fault) -> Why {
    why(&e.name(), e.detail())
}
