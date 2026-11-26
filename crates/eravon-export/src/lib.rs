#![forbid(unsafe_code)]

pub mod assemble;
pub mod docs;
pub mod export;
pub mod pick;
pub mod seam;

pub use eravon_watch::{Why, why};

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S7;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
