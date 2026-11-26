#![forbid(unsafe_code)]

pub mod act;
pub mod book;
pub mod fault;
pub mod gas;
pub mod guild;
pub mod judge;
pub mod lapse;
pub mod quote;
pub mod registry;
pub mod table;
pub mod view;
pub mod wire;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S3;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
