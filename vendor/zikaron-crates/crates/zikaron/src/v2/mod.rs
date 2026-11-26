//! The `zikaron/2` core (`base/docs/zikaron-v2.md`, release digest `0x42ade567…9350e`, §12.5): every [E] and
//! [C] predicate of that law, written beside the `zikaron/1` core of this crate, which it embeds whole.
//!
//! - [`entry`] the envelope, the signature and the eight body tables (§4 to §6); the canonical form (§3) and
//!   the signature arithmetic (§5) are the `zikaron/1` core's own functions, called here.
//! - [`audit`] retirement, timeliness and lineage (§7.4), the audit and its report (§8), the basis (§9.4), and
//!   the continuation of a `zikaron/1` ledger (§12.6), whose old segment the `zikaron/1` core
//!   ([`crate::audit`], release digest `0xbecfb6f0…0132fc`) reads as a black box.
//! - [`tokens`] the closed tables this law changes; the ones it shares are `zikaron/1`'s ([`crate::tokens`]).

pub mod audit;
pub mod entry;
pub mod tokens;
