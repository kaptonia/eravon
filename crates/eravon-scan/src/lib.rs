#![forbid(unsafe_code)]

pub mod bare;
pub mod board;
pub mod codes;
pub mod evidence;
pub mod fault;
pub mod judged;
pub mod kit;
pub mod ledger;
pub mod log;
pub mod records;
pub mod scan;
pub mod succession;
pub mod times;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::A3;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
