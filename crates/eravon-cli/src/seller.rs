use eravon_literals::misuse::Misuse;
use eravon_link as k3;
use eravon_literals::json::Out;
use eravon_standard as a1;
use eravon_zikaron as a4;
use eravon_store::record::Kind;
use eravon_store::seam::Box_;
use eravon_store::vault::Seat;
use eravon_terms as k2;

use crate::fault::{Fault, R};
use crate::home::{self, Ctx};
use crate::answer::{hex, ok, strs};
use crate::seam;
use crate::shared::{self, Flags};
use eravon_vault as h2;
use crate::write;


mod vault;
use vault::*;
mod terms;
pub use terms::decode_receipt_field;
use terms::*;
mod watch;
pub use watch::{E_OPEN_OPENED, Held, OpenIn, Opened, act_gate_of, act_refused, deploy, expire, expire_calls, held, kept_end, open_calls};
pub use fund::{bind_path, company_setup, CompanySetup, E_P7_AGAIN};
pub(crate) use watch::two_calls;
use watch::*;
mod company;
use company::*;
mod actions;
use actions::*;
mod goods;
pub(crate) use goods::{record_id};
use goods::*;
mod cands;
use cands::*;
mod fund;
use fund::*;
mod orders;
use orders::*;
mod desk;
use desk::*;
const SEAT: Seat = Seat::Seller;

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
        "vault-derive" => Ok(one(f, vault_derive)?),
        "vault-lock" => Ok(one(f, vault_lock)?),
        "vault-unlock" => Ok(one(f, vault_unlock)?),
        "vault-passcode" => Ok(one(f, vault_passcode)?),
        "vault-recover" => Ok(one(f, vault_recover)?),
        "vault-list" => Ok(one(f, vault_list)?),
        "terms-build" => Ok(one(f, terms_build)?),
        "terms-hash" => Ok(one(f, terms_hash)?),
        "history" => Ok(one(f, crate::history::history)?),
        "terms-check" => Ok(one(f, terms_check)?),
        "offer-sign" => Ok(one(f, offer_sign)?),
        "link-encode" => Ok(one(f, link_encode)?),
        "link-decode" => Ok(one(f, link_decode)?),
        "receipt-paste" => Ok(one(f, receipt_paste)?),
        "watch" => Ok(one(f, watch)?),
        "deploy" => Ok(one(f, deploy)?),
        "expire" => Ok(one(f, expire)?),
        "claim" => Ok(one(f, claim)?),
        "refund" => Ok(one(f, |c, f| action(c, f, a1::call::Action::Refund))?),
        "settle-offer" => Ok(one(f, settle_offer)?),
        "settle" => Ok(one(f, settle)?),
        "confirm-bysig" => Ok(one(f, confirm_bysig)?),
        "commit" => Ok(one(f, commit)?),
        "sweep" => Ok(one(f, sweep)?),
        "dossier-export" => Ok(one(f, |c, f| crate::deskverbs::export(c, f, Some(crate::deskops::export::Kind::Dossier)))?),
        "statement-export" => Ok(one(f, |c, f| crate::deskverbs::export(c, f, Some(crate::deskops::export::Kind::Statement)))?),
        "payways-set" => Ok(one(f, payways_set)?),
        "candidate-rename" => Ok(one(f, candidate_rename)?),
        "product-listed" => Ok(one(f, product_listed)?),
        "product-delete" => Ok(one(f, product_delete)?),
        "product-export-proposal" => Ok(one(f, product_export_proposal)?),
        "pending-bump" => Ok(one(f, pending_bump)?),
        "pending-void" => Ok(one(f, pending_void)?),
        "order-label" => Ok(one(f, order_label)?),
        "deliver-save" => Ok(one(f, deliver_save)?),
        "advance" => Ok(one(f, advance)?),
        "redraw" => Ok(one(f, redraw)?),
        "zikaron-status" => Ok(one(f, zikaron_status)?),
        "zikaron-kits" => Ok(one(f, zikaron_kits)?),
        "product-new" | "product-edit" | "product-save" => Ok(one(f, product_save)?),
        "product-publish" => Ok(one(f, product_publish)?),
        "product-post" => Ok(one(f, product_post)?),
        "fund-check" => Ok(one(f, fund_check)?),
        "fund-setup" => Ok(one(f, fund_setup)?),
        "wallet-send" => Ok(one(f, wallet_send)?),
        "wallet-former-move" => Ok(one(f, crate::buyer::wallet_former_move)?),
        "product-revoke" => Ok(one(f, product_revoke)?),
        "product-list" => Ok(one(f, |c, f| product_list(c, f))?),
        "product-show" => Ok(one(f, product_show)?),
        "product-live" => Ok(one(f, product_live)?),
        "product-link" => Ok(one(f, product_link)?),
        "product-qr" => Ok(one(f, product_qr)?),
        "product-resign" => Ok(one(f, product_resign)?),
        "candidate-add" => Ok(one(f, candidate_add)?),
        "candidate-list" => Ok(one(f, candidate_list)?),
        "candidate-show" => Ok(one(f, candidate_show)?),
        "complaint-open" => Ok(one(f, |c, f| crate::complaintverbs::open(c, f, false))?),
        "complaint-write" => Ok(one(f, |c, f| crate::complaintverbs::write(c, f, false))?),
        "candidate-remove" => Ok(one(f, candidate_remove)?),
        "order-list" => Ok(one(f, order_list)?),
        "order-show" => Ok(one(f, order_show)?),
        _ => Err(Misuse::verb(Some(verb))),
    }
}

fn one(f: &Flags, g: impl FnOnce(&Ctx, &Flags) -> R<Out>) -> Result<Out, Misuse> {
    let c = shared::ctx(f, Seat::Seller)?;
    Ok(match g(&c, f) {
        Ok(v) => v,
        Err(e) => crate::answer::refuse(&c.why_locked(e)),
    })
}

const NOT_SENT: &str = "E_V_NOT_SENT";
