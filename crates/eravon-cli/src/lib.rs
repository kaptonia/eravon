#![forbid(unsafe_code)]

pub const V1: &str = "V1";

pub fn seam() {
    eravon::trace::mark(V1);
}

pub mod args;
pub mod codes;
pub mod codes_own;
pub mod desk;
pub mod deskops;
pub mod deskverbs;
pub mod mcp;
pub mod mcptools;
pub mod named;
pub mod out;
pub use eravon_send::sent;

pub const GRANT_ORIGINALS_ALL: &str = "all";

pub const CLEAN_YES: &str = "yes";

pub const CLEAN_UNLISTED: &str = "kept";
pub mod verbs;
pub mod after_company;
pub mod answer;
pub mod buyer;
pub mod company;
pub mod complaintverbs;
pub mod deskwire;
pub mod door;
pub mod doorverbs;
pub mod fault;
pub mod former;
pub mod history;
pub mod home;
pub mod rand;
pub mod schema;
pub mod sealed;
pub mod seam;
pub mod seller;
pub mod send;
pub mod shared;
pub mod vaultverbs;
pub mod verbwire;
pub mod write;
pub mod verbs_own;
