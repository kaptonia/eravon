#![forbid(unsafe_code)]

pub mod action;
pub mod ahead;
pub mod ask;
pub mod clock;
pub mod endpoints;
pub mod fault;
pub mod gas;
pub mod judge;
pub mod oneread;
pub mod patience;
pub mod poll;
pub mod read;
pub mod recall;
pub mod proofs;
pub mod rest;
pub mod rlp;
pub mod rpc;
pub mod said;
pub mod scan;
pub mod seam;
pub mod send;
pub mod settled;
pub mod sending;
pub mod table;
pub mod task;
pub mod trace;
pub mod transport;
pub mod trie;
pub mod tx;
pub mod wire;

pub fn seam() {
    mark::mark(mark::FEATURE);
}

pub mod json_in {
    pub type In = eravon_literals::json::In;
}

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::A2;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
