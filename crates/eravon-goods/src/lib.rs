#![forbid(unsafe_code)]

pub mod company;
pub mod draft;
pub mod fund;
pub mod goods;
pub mod link;
pub mod pool;
pub mod refs;
pub mod resign;
pub mod resolve;
pub mod series;
pub mod sign;
pub mod terms;
pub mod verdict;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S1;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
