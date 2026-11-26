#![forbid(unsafe_code)]

pub mod abi;
pub mod call;
pub mod chain;
pub mod company;
pub mod consent;
pub mod event;
pub mod fault;
pub mod gate;
pub mod multi;
pub mod feed;
pub mod order;
pub mod precheck;
pub mod proposal;
pub mod sol;
pub mod tx;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::A1;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
