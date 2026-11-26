use eravon_store::record::Kind;
use eravon_store::seam::Box_;

use crate::draft::Draft;
use crate::resolve::{self, Resolved};
use crate::terms;
use crate::verdict::{self, Verdict};


mod judge;
pub use judge::{judge, judge_with, local, secs_of, span, terms_fees, DAYS_MAX, E_GOODS_QUOTE_MOVED, E_GOODS_FUND, E_GOODS_FUND_UNSET, E_GOODS_PAYOUT_KEY, HOURS_MAX, MINUTES_MAX};
mod save;
pub use save::{confirm_fees, delete, note_fund, note_payout, post, publish, publish_recorded, publish_seen, revoke, revoke_proposal, revoked_recorded, save, set_listed, sweep, Posted, Sender, POST, PRESIGN, E_GOODS_COMPANY_PROPOSES, E_GOODS_KEY_HANDED_ON, E_GOODS_POST_UNSEEN, E_GOODS_REVOKE_UNSEEN};
mod view;
pub use view::{Kept, LinkRow, Row, View, chain_here, chain_of, company_check, company_version, kept, dirty, export_proposal, list, list_every_chain, live_bytes, load, load_edit, name_of, names, proposal_dir, proposal_time, record, records, records_here, revoke_dir, signed_words, stored, terms_hashes, terms_of, using, view, REVOKED_BY_COMPANY};
use view::*;
pub const FEES_SHOWN: &str = field::FEES_SHOWN;
pub const TERMS_ID: &str = field::TERMS_ID;

mod field {
    pub const TITLE: &str = "title";
    pub const DRAFT: &str = "draft";
    pub const VERSION: &str = "version";
    pub const TERMS_HASH: &str = "termsHash";
    pub const TERMS_ID: &str = "termsId";
    pub const LISTED: &str = "listed";
    pub const LINK: &str = "link";
    pub const LINK_EXPIRY: &str = "linkExpiry";
    pub const LINK_STATE: &str = "linkState";
    pub const POSTED_TX: &str = "postedTx";
    pub const POSTED_HASH: &str = "postedHash";
    pub const REVOKED: &str = "revoked";
    pub const REVOKING: &str = "revoking";
    pub const PENDING: &str = "pending";
    pub const EXPIRY: &str = "expiry";
    pub const DECIMALS: &str = "decimals";
    pub const COMPANY: &str = "company";
    pub const COMPANY_DIGESTS: &str = "companyDigests";
    pub const COMPANY_ADDRESS: &str = "companyAddress";
    pub const EDITING: &str = "editing";
    pub const COMPANY_TIME: &str = "companyTime";
    pub const COMPANY_REFUSED: &str = "companyRefused";
    pub const FEES_SHOWN: &str = "feesShown";
    pub const FEES_SIGNED: &str = "feesSigned";
    pub const FEES_NEW: &str = "feesNew";
    pub const CHAIN: &str = "chainId";
}

pub const PART_DRAFT: &str = "draft.json";
pub const PART_EDIT: &str = "edit.json";
pub const PART_TERMS: &str = "terms.md";
pub const PART_GOODS_TEXT: &str = "goods.md";
pub const PART_DELIVERY_TEXT: &str = "delivery.md";
pub const PART_LISTING: &str = "listing.json";
pub const PART_PROPOSAL: &str = "proposal.json";

pub fn listing_id(id: &str, version: u64) -> String {
    format!("listing-{id}-v{version}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Store { name: &'static str, detail: String },
    Missing(String),
    Form(String),
    Refused(Verdict),
    Chain { name: String, detail: String },
    Unsent { name: String, detail: String },
    Revoked(String),
    Nothing(String),
    NotDraft(String),
    Company(String),
    Export(String),
    Unseen(String),
    Proposes(String),
    HandedOn(String),
    OtherChain { id: String, chain: u64, here: u64 },
}

pub const E_GOODS_OTHER_CHAIN: &str = "E_GOODS_OTHER_CHAIN";

pub const FEES_DUE: &str = "fees";
pub const E_GOODS_FEES_CHANGED: &str = "E_GOODS_FEES_CHANGED";

pub const E_GOODS_FEES_UNSEEN: &str = "E_GOODS_FEES_UNSEEN";

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::Store { name, .. } => name,
            Fault::Missing(_) => "E_GOODS_MISSING",
            Fault::Form(_) => "E_GOODS_DRAFT_FORM",
            Fault::Refused(_) => "E_GOODS_REFUSED",
            Fault::Chain { .. } | Fault::Unsent { .. } => "E_GOODS_CHAIN",
            Fault::Revoked(_) => "E_GOODS_REVOKED",
            Fault::Nothing(_) => "E_GOODS_NOTHING",
            Fault::NotDraft(_) => "E_GOODS_NOT_DRAFT",
            Fault::Company(_) => "E_GOODS_COMPANY",
            Fault::Export(_) => "E_GOODS_EXPORT",
            Fault::Unseen(_) => E_GOODS_FEES_UNSEEN,
            Fault::Proposes(_) => E_GOODS_COMPANY_PROPOSES,
            Fault::HandedOn(_) => E_GOODS_KEY_HANDED_ON,
            Fault::OtherChain { .. } => E_GOODS_OTHER_CHAIN,
        }
    }
    pub fn said(&self) -> (String, String) {
        match self {
            Fault::Unsent { name, detail } => (name.clone(), detail.clone()),
            other => (other.name().to_string(), other.detail()),
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Store { detail, .. } => detail.clone(),
            Fault::Missing(id) => format!("找不到商品 {id}"),
            Fault::Form(id) => format!("无法读取商品 {id}"),
            Fault::Refused(v) => format!("缺少 {} 项,未保存", v.missing.len()),
            Fault::Chain { name, detail } | Fault::Unsent { name, detail } => format!("{name}:{detail}"),
            Fault::Revoked(id) => format!("{id} 这一版已下架(链上 revoke),不能再上架;修改即发布新版本"),
            Fault::Nothing(d) => d.clone(),
            Fault::NotDraft(id) => format!("{id} 已上架,只能下架,不能删除"),
            Fault::Company(d) | Fault::Export(d) | Fault::Unseen(d) | Fault::Proposes(d) | Fault::HandedOn(d) => d.clone(),
            Fault::OtherChain { id, chain, here } => format!("商品 {id} 在链 {chain} 上,家现在在链 {here}:切到链 {chain} 再动它"),
        }
    }
}

impl From<eravon_store::fault::Fault> for Fault {
    fn from(e: eravon_store::fault::Fault) -> Fault {
        match e {
            eravon_store::fault::Fault::Missing(m) => Fault::Missing(m),
            other => Fault::Store { name: other.name(), detail: other.detail() },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub id: String,
    pub draft: bool,
    pub version: u64,
    pub terms_hash: Option<String>,
    pub verdict: Verdict,
    pub resolved: Resolved,
    pub link: Option<String>,
    pub link_state: Option<(String, String)>,
    pub company_refused: Option<(String, String)>,
    pub pending: Vec<String>,
}

pub struct Ctx<'a> {
    pub data: &'a str,
    pub writer: &'a str,
    pub now: u64,
    pub me: &'a str,
    pub registries: &'a [String],
    pub signer: &'a crate::sign::Signer,
    pub route: crate::company::Route,
}

#[cfg(test)]
mod unsent_tests {
    #[test]
    fn a_send_refused_is_said_by_its_own_name() {
        let f = super::Fault::Unsent { name: "E_V_PRECHECK".into(), detail: "fee:余额不够".into() };
        assert_eq!(f.said(), ("E_V_PRECHECK".to_string(), "fee:余额不够".to_string()));
        let c = super::Fault::Chain { name: "E_CHAIN_TRANSPORT".into(), detail: "down".into() };
        assert_eq!(c.said().0, "E_GOODS_CHAIN", "a chain read that failed stays the chain's");
    }
}
