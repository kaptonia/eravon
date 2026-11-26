use eravon_terms as k2;

pub mod src_abi {
    include!(concat!(env!("OUT_DIR"), "/abi_gen.rs"));
}

pub mod past_trial {
    include!(concat!(env!("OUT_DIR"), "/abi_gen_past_trial.rs"));
}

#[derive(Clone, Copy, Debug)]
pub struct Fun {
    pub name: &'static str,
    pub sig: &'static str,
}

impl Fun {
    pub fn selector(&self) -> [u8; 4] {
        let h = k2::derive::keccak(self.sig.as_bytes());
        [h[0], h[1], h[2], h[3]]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ev {
    pub name: &'static str,
    pub sig: &'static str,
    pub indexed: &'static [bool],
}

impl Ev {
    pub fn topic0(&self) -> [u8; 32] {
        k2::derive::keccak(self.sig.as_bytes())
    }
    pub fn topics(&self) -> usize {
        self.indexed.iter().filter(|x| **x).count()
    }
}

pub const T_RUNG: &str = src_abi::ERAVON_T_RUNG;
pub const T_OFFER: &str = src_abi::ERAVON_T_OFFER;
pub const T_ORDER_TERMS: &str = src_abi::ERAVON_T_ORDER_TERMS;

pub const INIT: Fun = Fun { name: "init", sig: src_abi::ERAVON_INIT };


pub const CANCEL: Fun = Fun { name: "cancel", sig: src_abi::ERAVON_CANCEL };
pub const CLAIM_DELIVERY: Fun = Fun { name: "claimDelivery", sig: src_abi::ERAVON_CLAIM_DELIVERY };
pub const CONFIRM: Fun = Fun { name: "confirm", sig: src_abi::ERAVON_CONFIRM };
pub const DISPUTE: Fun = Fun { name: "dispute", sig: src_abi::ERAVON_DISPUTE };
pub const REFUND: Fun = Fun { name: "refund", sig: src_abi::ERAVON_REFUND };
pub const SETTLE: Fun = Fun { name: "settle", sig: src_abi::ERAVON_SETTLE };
pub const COMMIT: Fun = Fun { name: "commit", sig: src_abi::ERAVON_COMMIT };
pub const ADVANCE: Fun = Fun { name: "advance", sig: src_abi::ERAVON_ADVANCE };
pub const REDRAW: Fun = Fun { name: "redraw", sig: src_abi::ERAVON_REDRAW };
pub const SWEEP: Fun = Fun { name: "sweep", sig: src_abi::ERAVON_SWEEP };

pub const CLAIM_DELIVERY_BY_SIG: Fun = Fun { name: "claimDeliveryBySig", sig: src_abi::ERAVON_CLAIM_DELIVERY_BY_SIG };
pub const CONFIRM_BY_SIG: Fun = Fun { name: "confirmBySig", sig: src_abi::ERAVON_CONFIRM_BY_SIG };
pub const DISPUTE_BY_SIG: Fun = Fun { name: "disputeBySig", sig: src_abi::ERAVON_DISPUTE_BY_SIG };
pub const REFUND_BY_SIG: Fun = Fun { name: "refundBySig", sig: src_abi::ERAVON_REFUND_BY_SIG };

pub const REVOKE: Fun = Fun { name: "revoke", sig: src_abi::ERAVON_REVOKE };
pub const ACCEPT_FUNDING: Fun = Fun { name: "acceptFunding", sig: src_abi::ERAVON_ACCEPT_FUNDING };
pub const CONSENT: Fun = Fun { name: "consent", sig: src_abi::ERAVON_CONSENT };

pub const POST: Fun = Fun { name: "post", sig: src_abi::ERAVON_BOARD_POST };

pub const STATUS: Fun = Fun { name: "status", sig: src_abi::ERAVON_STATUS };
pub const FEE_QUOTED: Fun = Fun { name: "feeQuoted", sig: src_abi::ERAVON_FEE_QUOTED };
pub const SEAT_OF: Fun = Fun { name: "seatOf", sig: src_abi::ERAVON_SEAT_OF };
pub const ACCOUNT_OF: Fun = Fun { name: "accountOf", sig: src_abi::ERAVON_ACCOUNT_OF };
pub const OWED: Fun = Fun { name: "owed", sig: src_abi::ERAVON_OWED };
pub const CONSENTED: Fun = Fun { name: "consented", sig: src_abi::ERAVON_CONSENTED };
pub const REVOKED: Fun = Fun { name: "revoked", sig: src_abi::ERAVON_REVOKED };
pub const FUNDS: Fun = Fun { name: "funds", sig: src_abi::ERAVON_FUNDS };
pub const PANEL_BOOK: Fun = Fun { name: "panelBook", sig: src_abi::ERAVON_PANEL_BOOK };
pub const SUCCESSION_BOOK: Fun = Fun { name: "successionBook", sig: src_abi::ERAVON_SUCCESSION_BOOK };
pub const ROUTE_GAS: Fun = Fun { name: "routeGas", sig: src_abi::ERAVON_ROUTE_GAS };
pub const PARTY_ROLES: Fun = Fun { name: "partyRoles", sig: src_abi::ERAVON_PARTY_ROLES };
pub const STAGE: Fun = Fun { name: "stage", sig: src_abi::ERAVON_STAGE };
pub const OUTCOME_OF: Fun = Fun { name: "outcomeOf", sig: src_abi::ERAVON_OUTCOME_OF };
pub const ADVANCE_OF: Fun = Fun { name: "advanceOf", sig: src_abi::ERAVON_ADVANCE_OF };
pub const PANEL_INIT_CODE_HASH: Fun = Fun { name: "panelInitCodeHash", sig: src_abi::ERAVON_PANEL_INIT_CODE_HASH };
pub const QUOTE_TYPE: Fun = Fun { name: "quoteType", sig: src_abi::ERAVON_QUOTE_TYPE };
pub const EXTRA_TYPE: Fun = Fun { name: "extraType", sig: src_abi::ERAVON_EXTRA_TYPE };
pub const LOGIC: Fun = Fun { name: "LOGIC", sig: src_abi::ERAVON_LOGIC };
pub const PROXY_CODEHASH: Fun = Fun { name: "PROXY_CODEHASH", sig: src_abi::ERAVON_PROXY_CODEHASH };
pub const HASH_OFFER: Fun = Fun { name: "hashOffer", sig: src_abi::ERAVON_VERIFIER_HASH_OFFER };
pub const VERIFIER_CHECK: Fun = Fun { name: "check", sig: src_abi::ERAVON_VERIFIER_CHECK };

pub const BOOK_COUNT: Fun = Fun { name: "count", sig: src_abi::ERAVON_PANEL_BOOK_COUNT };
pub const BOOK_PANEL_AT: Fun = Fun { name: "panelAt", sig: src_abi::ERAVON_PANEL_BOOK_PANEL_AT };

pub const SUCCESSION_CURRENT: Fun = Fun { name: "current", sig: src_abi::I_GALEED_SUCCESSION_BOOK_CURRENT };
pub const SUCCESSION_ROOT: Fun = Fun { name: "root", sig: src_abi::I_GALEED_SUCCESSION_BOOK_ROOT };
pub const SUCCESSION_DECLARE: Fun = Fun { name: "declare", sig: src_abi::GALEED_SUCCESSION_BOOK_DECLARE };
pub const SUCCESSION_ACCEDE: Fun = Fun { name: "accede", sig: src_abi::GALEED_SUCCESSION_BOOK_ACCEDE };
pub const SUCCESSION_PENDING: Fun = Fun { name: "pending", sig: src_abi::GALEED_SUCCESSION_BOOK_PENDING };
pub const SUCCESSION_ENTERED: Fun = Fun { name: "entered", sig: src_abi::GALEED_SUCCESSION_BOOK_ENTERED };

pub const TRIAL_CONSENT: Fun = Fun { name: "consent", sig: past_trial::ERAVON_CONSENT };
pub const TRIAL_REVOKE: Fun = Fun { name: "revoke", sig: past_trial::ERAVON_REVOKE };
pub const TRIAL_INIT: Fun = Fun { name: "init", sig: past_trial::ERAVON_INIT };
pub const TRIAL_CANCEL: Fun = Fun { name: "cancel", sig: past_trial::ERAVON_CANCEL };
pub const TRIAL_PANEL_BOOK_OF: Fun = Fun { name: "panelBookOf", sig: past_trial::ERAVON_PANEL_BOOK_OF };
pub const TRIAL_PANEL_FROZEN: Ev = ev("PanelFrozen", past_trial::ERAVON_EV_PANEL_FROZEN, past_trial::ERAVON_EV_PANEL_FROZEN_INDEXED);
pub const TRIAL_LAPSED: Ev = ev("Lapsed", past_trial::ERAVON_EV_LAPSED, past_trial::ERAVON_EV_LAPSED_INDEXED);
pub const TRIAL_ENGAGED: Ev = ev("Engaged", past_trial::ERAVON_PANEL_BOOK_EV_ENGAGED, past_trial::ERAVON_PANEL_BOOK_EV_ENGAGED_INDEXED);
pub const TRIAL_STATUS: Fun = Fun { name: "status", sig: past_trial::ERAVON_STATUS };

pub const ERC20_BALANCE_OF: Fun = Fun { name: "balanceOf", sig: src_abi::IERC20_BALANCE_OF };
pub const ERC20_DECIMALS: Fun = Fun { name: "decimals", sig: src_abi::IERC20_DECIMALS };
pub const ERC20_TRANSFER: Fun = Fun { name: "transfer", sig: src_abi::IERC20_TRANSFER };

pub const FEED_DECIMALS: Fun = Fun { name: "decimals", sig: src_abi::I_PRICE_FEED_DECIMALS };
pub const FEED_LATEST_ROUND: Fun = Fun { name: "latestRoundData", sig: src_abi::I_PRICE_FEED_LATEST_ROUND_DATA };

pub const ACTIONS: [Fun; 14] = [
    CANCEL,
    CLAIM_DELIVERY,
    CLAIM_DELIVERY_BY_SIG,
    CONFIRM,
    CONFIRM_BY_SIG,
    DISPUTE,
    DISPUTE_BY_SIG,
    REFUND,
    REFUND_BY_SIG,
    SETTLE,
    COMMIT,
    ADVANCE,
    REDRAW,
    SWEEP,
];

pub const REGISTRY_ACTIONS: [Fun; 3] = [REVOKE, ACCEPT_FUNDING, POST];

pub const EXPORTED: [Fun; 1] = [CONSENT];

pub const VIEWS: [Fun; 24] = [
    STATUS,
    FEE_QUOTED,
    SEAT_OF,
    ACCOUNT_OF,
    OWED,
    PANEL_BOOK,
    SUCCESSION_BOOK,
    ROUTE_GAS,
    PARTY_ROLES,
    STAGE,
    OUTCOME_OF,
    ADVANCE_OF,
    SUCCESSION_CURRENT,
    SUCCESSION_ROOT,
    PANEL_INIT_CODE_HASH,
    QUOTE_TYPE,
    EXTRA_TYPE,
    LOGIC,
    PROXY_CODEHASH,
    HASH_OFFER,
    CONSENTED,
    REVOKED,
    FUNDS,
    VERIFIER_CHECK,
];

const fn ev(name: &'static str, sig: &'static str, indexed: &'static [bool]) -> Ev {
    Ev { name, sig, indexed }
}

pub const INSTANCE_EVENTS: [Ev; 14] = [
    ev("Opened", src_abi::ERAVON_EV_OPENED, src_abi::ERAVON_EV_OPENED_INDEXED),
    ev("Priced", src_abi::ERAVON_EV_PRICED, src_abi::ERAVON_EV_PRICED_INDEXED),
    ev("PanelFrozen", src_abi::ERAVON_EV_PANEL_FROZEN, src_abi::ERAVON_EV_PANEL_FROZEN_INDEXED),
    ev("DrawArmed", src_abi::ERAVON_EV_DRAW_ARMED, src_abi::ERAVON_EV_DRAW_ARMED_INDEXED),
    ev("DeliveryClaimed", src_abi::ERAVON_EV_DELIVERY_CLAIMED, src_abi::ERAVON_EV_DELIVERY_CLAIMED_INDEXED),
    ev("Confirmed", src_abi::ERAVON_EV_CONFIRMED, src_abi::ERAVON_EV_CONFIRMED_INDEXED),
    ev("Disputed", src_abi::ERAVON_EV_DISPUTED, src_abi::ERAVON_EV_DISPUTED_INDEXED),
    ev("Settled", src_abi::ERAVON_EV_SETTLED, src_abi::ERAVON_EV_SETTLED_INDEXED),
    ev("Ruled", src_abi::ERAVON_EV_RULED, src_abi::ERAVON_EV_RULED_INDEXED),
    ev("Resigned", src_abi::ERAVON_EV_RESIGNED, src_abi::ERAVON_EV_RESIGNED_INDEXED),
    ev("Lapsed", src_abi::ERAVON_EV_LAPSED, src_abi::ERAVON_EV_LAPSED_INDEXED),
    ev("Refunded", src_abi::ERAVON_EV_REFUNDED, src_abi::ERAVON_EV_REFUNDED_INDEXED),
    ev("Closed", src_abi::ERAVON_EV_CLOSED, src_abi::ERAVON_EV_CLOSED_INDEXED),
    ev("Swept", src_abi::ERAVON_EV_SWEPT, src_abi::ERAVON_EV_SWEPT_INDEXED),
];

pub const OPENED: Ev = INSTANCE_EVENTS[0];

pub const LOGIC_EVENTS: [Ev; 4] = [
    ev("Consented", src_abi::ERAVON_EV_CONSENTED, src_abi::ERAVON_EV_CONSENTED_INDEXED),
    ev("Pulled", src_abi::ERAVON_EV_PULLED, src_abi::ERAVON_EV_PULLED_INDEXED),
    ev("Revoked", src_abi::ERAVON_EV_REVOKED, src_abi::ERAVON_EV_REVOKED_INDEXED),
    ev("FundBound", src_abi::ERAVON_EV_FUND_BOUND, src_abi::ERAVON_EV_FUND_BOUND_INDEXED),
];

pub const CONSENTED_EV: Ev = LOGIC_EVENTS[0];
pub const PULLED_EV: Ev = LOGIC_EVENTS[1];
pub const REVOKED_EV: Ev = LOGIC_EVENTS[2];
pub const FUND_BOUND_EV: Ev = LOGIC_EVENTS[3];

pub const POSTED: Ev = ev("Posted", src_abi::ERAVON_BOARD_EV_POSTED, src_abi::ERAVON_BOARD_EV_POSTED_INDEXED);

pub const VENUE_EVENTS: [Ev; 3] = [
    ev("Seated", src_abi::I_GALEED_VENUE_EV_SEATED, src_abi::I_GALEED_VENUE_EV_SEATED_INDEXED),
    ev("Committed", src_abi::I_GALEED_VENUE_EV_COMMITTED, src_abi::I_GALEED_VENUE_EV_COMMITTED_INDEXED),
    ev("Anchored", src_abi::I_GALEED_VENUE_EV_ANCHORED, src_abi::I_GALEED_VENUE_EV_ANCHORED_INDEXED),
];

pub const REGISTRY_EVENTS: [Ev; 2] = [
    ev("Published", src_abi::GALEED_REGISTRY_EV_PUBLISHED, src_abi::GALEED_REGISTRY_EV_PUBLISHED_INDEXED),
    ev("StatusChanged", src_abi::GALEED_REGISTRY_EV_STATUS_CHANGED, src_abi::GALEED_REGISTRY_EV_STATUS_CHANGED_INDEXED),
];

pub const BOOK_ENGAGED: Ev = ev("Engaged", src_abi::ERAVON_PANEL_BOOK_EV_ENGAGED, src_abi::ERAVON_PANEL_BOOK_EV_ENGAGED_INDEXED);

pub fn all_events() -> Vec<Ev> {
    let mut v: Vec<Ev> = INSTANCE_EVENTS.to_vec();
    v.extend_from_slice(&LOGIC_EVENTS);
    v.extend_from_slice(&VENUE_EVENTS);
    v
}

pub fn event_by_topic(t0: &[u8; 32]) -> Option<Ev> {
    all_events().into_iter().chain([POSTED, BOOK_ENGAGED]).chain(REGISTRY_EVENTS).chain(TRIAL_EVENTS).find(|e| &e.topic0() == t0)
}

pub const TRIAL_EVENTS: [Ev; 3] = [TRIAL_PANEL_FROZEN, TRIAL_LAPSED, TRIAL_ENGAGED];

pub fn error_by_selector(sel: &[u8; 4]) -> Option<&'static str> {
    src_abi::ITEMS
        .iter()
        .filter(|(_, kind, _, _)| *kind == "error")
        .map(|(_, _, sig, _)| *sig)
        .find(|sig| {
            let h = k2::derive::keccak(sig.as_bytes());
            &h[..4] == sel
        })
}

pub const JUDGE_FACE: [Fun; 3] = [
    Fun { name: "accept", sig: src_abi::ERAVON_ACCEPT },
    Fun { name: "setPayout", sig: src_abi::ERAVON_SET_PAYOUT },
    Fun { name: "withdraw", sig: src_abi::ERAVON_WITHDRAW },
];

pub const JUDGE_ONLY: [&str; 5] = ["accept", "rule", "resign", "setPayout", "withdraw"];

pub const GUILD_ONLY: [&str; 1] = ["engage"];

pub const ERC20_ALLOWANCE: Fun = Fun { name: "allowance", sig: "allowance(address,address)" };
pub const ERC20_APPROVE: Fun = Fun { name: "approve", sig: "approve(address,uint256)" };
pub const ERC20_TRANSFER_EV: Ev = Ev { name: "Transfer", sig: "Transfer(address,address,uint256)", indexed: &[true, true, false] };
pub const ERC20_PERMIT: Fun = Fun { name: "permit", sig: "permit(address,address,uint256,uint256,uint8,bytes32,bytes32)" };
pub const ERC20_NONCES: Fun = Fun { name: "nonces", sig: "nonces(address)" };
pub const ERC20_DOMAIN_SEPARATOR: Fun = Fun { name: "DOMAIN_SEPARATOR", sig: "DOMAIN_SEPARATOR()" };
pub const BOOK_DECLARED: Ev = Ev { name: "Declared", sig: "Declared(address,address)", indexed: &[true, true] };
pub const BOOK_ACCEDED: Ev = Ev { name: "Acceded", sig: "Acceded(address,address,address)", indexed: &[true, true, true] };
pub const AGGREGATE3: Fun = Fun { name: "aggregate3", sig: "aggregate3((address,bool,bytes)[])" };
pub const MULTICALL3_ETH_BALANCE: Fun = Fun { name: "getEthBalance", sig: "getEthBalance(address)" };
pub const MULTICALL3_TIMESTAMP: Fun = Fun { name: "getCurrentBlockTimestamp", sig: "getCurrentBlockTimestamp()" };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guild_face_has_no_door_here() {
        for n in GUILD_ONLY {
            assert!(
                !ACTIONS.iter().any(|f| f.name == n),
                "{n} 是行会面,客户端不许有这一扇门"
            );
        }
    }

    #[test]
    fn the_judge_face_has_no_door_here() {
        for n in JUDGE_ONLY {
            assert!(
                !ACTIONS.iter().any(|f| f.name == n),
                "{n} 是判官面,客户端不许有这一扇门"
            );
            assert!(
                !VIEWS.iter().any(|f| f.name == n) || n == "accountOf",
                "{n} 不该在读面表上"
            );
        }
    }

    #[test]
    fn the_judge_face_is_only_recognised() {
        for f in JUDGE_FACE {
            assert!(JUDGE_ONLY.contains(&f.name), "{} 该是判官面的名", f.name);
            assert!(!ACTIONS.iter().any(|x| x.name == f.name), "{} 不该成为一扇门", f.name);
        }
    }

    #[test]
    fn every_name_appears_once() {
        let mut names: Vec<&str> =
            ACTIONS.iter().chain(VIEWS.iter()).map(|f| f.name).collect();
        let n = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "函数名在表上该各出现一次");
        let mut evs: Vec<&str> = all_events().iter().map(|e| e.name).collect();
        let m = evs.len();
        evs.sort_unstable();
        evs.dedup();
        assert_eq!(evs.len(), m, "事件名在表上该各出现一次");
        assert_eq!(m, 21, "实例十四、逻辑四、venue 三");
    }

    #[test]
    fn a_selector_is_the_first_four_bytes_of_the_signatures_keccak() {
        let s = CONFIRM.selector();
        let h = eravon_terms::derive::keccak(b"confirm()");
        assert_eq!(s, [h[0], h[1], h[2], h[3]]);
    }

    #[test]
    fn an_event_is_found_by_its_topic_zero() {
        let e = event_by_topic(&INSTANCE_EVENTS[0].topic0()).expect("认得回来");
        assert_eq!(e.name, "Opened");
        assert_eq!(e.topics(), 3);
    }
}
