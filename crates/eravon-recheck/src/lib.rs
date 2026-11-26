#![forbid(unsafe_code)]

pub mod check;
pub mod dossier;
pub mod fault;
pub mod judge;
pub mod read;
pub mod report;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::R1;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
