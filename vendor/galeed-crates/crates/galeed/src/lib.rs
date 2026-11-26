//! The `galeed/1` core: the [E] and [C] predicates of the law, §3 to §9.5.
//!
//! §3 canonical form ([`json`]), §4.3 acceptance ([`envelope`]), §5 signing and recovery ([`sign`];
//! cryptography lives only in [`cryptox`]), §6 body tables ([`body`]), lineage, audit and anchors
//! (§7 to §9). Built from the law text and the conformance harness; where they disagree with the
//! frozen conformance core, the core wins.
#![forbid(unsafe_code)]

pub mod answer;
pub mod audit;
pub mod body;
pub mod cryptox;
pub mod draft;
pub mod entry;
pub mod envelope;
pub mod face;
pub mod hexform;
pub mod json;
pub mod kit;
pub mod literals;
pub mod marks;
pub mod rlp;
pub mod sign;
#[cfg(test)]
mod testkit;
pub mod token;
pub mod tokens;
pub mod trace;

/// The literal `galeed/1`: the `spec` value of an entry (§4.1) and the entry signing domain (§5.6) are
/// the same literal (registered as a signing-domain in FAMILY-LITERALS.md).
pub const LAW: &str = "galeed/1";
