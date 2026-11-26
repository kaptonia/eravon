#![forbid(unsafe_code)]

pub mod balance;
pub mod fault;
pub mod send;
pub mod tx;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::P7;

    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
