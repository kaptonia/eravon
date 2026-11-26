use eravon_literals::misuse::Misuse;
use eravon_link as k3;
use eravon_literals::json::Out;
use eravon_standard as a1;
use eravon_standard::abi::Val;
use eravon_store::record::Kind;
use eravon_store::seam::Box_;
use eravon_store::vault::Seat;
use eravon_terms as k2;

use crate::fault::{Fault, R};
use crate::home::{self, Ctx};
use crate::answer::{hex, ok, strs};
use crate::seam;
use crate::seller;
use crate::shared::{self, Flags};
use eravon_vault as h2;
use crate::write;


mod vault;
use vault::*;
mod order;
pub use order::{Live, Pick, SignIn, Signed, deposit_short_gate, deposit_short_of, keep_order, key_pick, live_on, new_order_held, new_order_under, occupied, pool_seq_on, posted, series_held, series_held_order, series_prev, sign_order, signing_key, stage_gate, E_SERIES_PREV, E_SERIES_SAME};
use order::*;
mod orders;
pub use orders::{status_row};
use orders::*;
mod actions;
pub use actions::{Pull, candidate_gate, pull_calls, refund_after};
use actions::*;
pub(crate) use actions::{wallet_former_move, withdraw_to};
mod company;
pub use company::{CoDraft, CoOrder, co_draft, co_keep, co_registered, expire_export, expire_plan, open_plan, seat_party, E_COMPANY_GUILD, E_COMPANY_PROPOSES};
use company::*;
mod case;
pub use case::{cancel_calls, cancel_lost, keep_part_in, record_confirm_in, Cancel};
use case::*;
mod standing;
pub use standing::{may_open, pay_gate, place, sign_gate, standing, stands, Gate, Standing, E_OFFER_RETIRED, E_REVOKED};
const SEAT: Seat = Seat::Buyer;

pub fn run(verb: &str, f: &Flags) -> Result<Out, Misuse> {
    if crate::vaultverbs::VERBS.contains(&verb) {
        let c = shared::ctx(f, SEAT)?;
        return Ok(match crate::vaultverbs::run(verb, &c, f) {
            Some(Ok(v)) => v,
            Some(Err(e)) => crate::answer::refuse(&c.why_locked(e)),
            None => return Err(Misuse::verb(Some(verb))),
        });
    }
    if crate::deskverbs::VERBS.contains(&verb) {
        let c = shared::ctx(f, SEAT)?;
        return Ok(match crate::deskverbs::run(verb, &c, f) {
            Some(Ok(v)) => v,
            Some(Err(e)) => crate::answer::refuse(&c.why_locked(e)),
            None => return Err(Misuse::verb(Some(verb))),
        });
    }
    match verb {
        "vault-init" => Ok(one(f, vault_init)?),
        "vault-import" => Ok(one(f, vault_import)?),
        "vault-lock" => Ok(one(f, |c, f| seat_lock(c, f))?),
        "vault-derive" => Ok(one(f, |c, f| seat_derive(c, f))?),
        "vault-unlock" => Ok(one(f, |c, f| seat_unlock(c, f))?),
        "vault-passcode" => Ok(one(f, |c, f| seat_passcode(c, f))?),
        "vault-recover" => Ok(one(f, |c, f| seat_recover(c, f))?),
        "vault-list" => Ok(one(f, |c, f| seat_list(c, f))?),
        "open" => Ok(one(f, open)?),
        "choose" => Ok(one(f, choose)?),
        "history" => Ok(one(f, crate::history::history)?),
        "order-sign" => Ok(one(f, order_sign)?),
        "pay" => Ok(one(f, pay)?),
        "deploy" => Ok(one(f, deploy)?),
        "import" => Ok(one(f, import)?),
        "list" => Ok(one(f, list)?),
        "show" => Ok(one(f, show)?),
        "complaint-open" => Ok(one(f, |c, f| crate::complaintverbs::open(c, f, true))?),
        "complaint-write" => Ok(one(f, |c, f| crate::complaintverbs::write(c, f, true))?),
        "confirm" => Ok(one(f, confirm)?),
        "cancel" => Ok(one(f, cancel)?),
        "attach" => Ok(one(f, attach)?),
        "offers" => Ok(one(f, offers)?),
        "dispute" => Ok(one(f, dispute)?),
        "settle-sign" => Ok(one(f, settle_sign)?),
        "settle" => Ok(one(f, settle)?),
        "commit" => Ok(one(f, commit)?),
        "advance" => Ok(one(f, advance)?),
        "redraw" => Ok(one(f, redraw)?),
        "sweep" => Ok(one(f, sweep)?),
        "withdraw-to" => Ok(one(f, withdraw_to)?),
        "wallet-balance" => Ok(one(f, wallet_balance)?),
        "wallet-send" => Ok(one(f, wallet_send)?),
        "wallet-former-move" => Ok(one(f, wallet_former_move)?),
        "wallet-tx" => Ok(one(f, wallet_tx)?),
        "expire" => Ok(one(f, expire)?),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn one(f: &Flags, g: impl FnOnce(&Ctx, &Flags) -> R<Out>) -> Result<Out, Misuse> {
    let c = shared::ctx(f, Seat::Buyer)?;
    Ok(match g(&c, f) {
        Ok(v) => v,
        Err(e) => crate::answer::refuse(&c.why_locked(e)),
    })
}


pub const PERMIT_TTL: u64 = 3_600;
