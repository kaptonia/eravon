#![forbid(unsafe_code)]

pub mod archive;
pub mod atomic;
pub mod doc;
pub mod fault;
pub mod lock;
pub mod mirror;
pub mod record;
pub mod seam;
pub mod settings;
pub mod vault;
pub mod walk;
pub mod writer;

pub use eravon_glue::{codes, landing, layout, ledger, sealed, trace};
pub use codes::{Code, Trouble, Why};
pub use layout::{EntryName, Kind};
pub use ledger::{LedgerDir, Layout, Pile, Skip, Stored, Survey, Sweep, ENTRY_MAX};

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::A5;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
