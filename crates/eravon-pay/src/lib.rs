#![forbid(unsafe_code)]

pub mod check;
pub mod company;
pub mod fault;
pub mod pay;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::P3;

    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
