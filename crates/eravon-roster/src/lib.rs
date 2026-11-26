#![forbid(unsafe_code)]

pub mod adapter;
pub mod book;
pub mod gate;
pub mod pool;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::K5;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
