use eravon_dispute::view::CaseView;
use eravon_watch::order::State;

use crate::{Why, why};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Word {
    AwaitPay,
    AwaitDeploy,
    AwaitAccept,
    AwaitDelivery,
    AwaitConfirm,
    ConfirmHanded,
    Disputed,
    AwaitRedraw,
    Completed,
    Ruled,
    Settled,
    Refunded,
    Lapsed,
    Expired,
    Cancelled,
    Revoked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Deliver,
    PasteConfirm,
}

impl Act {
    pub fn word(self) -> &'static str {
        match self {
            Act::Deliver => "deliver",
            Act::PasteConfirm => "pasteConfirm",
        }
    }
}

impl Word {
    pub const ALL: [Word; 16] = [
        Word::AwaitPay,
        Word::AwaitDeploy,
        Word::AwaitAccept,
        Word::AwaitDelivery,
        Word::AwaitConfirm,
        Word::ConfirmHanded,
        Word::Disputed,
        Word::AwaitRedraw,
        Word::Completed,
        Word::Ruled,
        Word::Settled,
        Word::Refunded,
        Word::Lapsed,
        Word::Expired,
        Word::Cancelled,
        Word::Revoked,
    ];

    pub fn acts(self) -> &'static [Act] {
        match self {
            Word::AwaitDelivery => &[Act::Deliver],
            Word::AwaitConfirm | Word::ConfirmHanded => &[Act::PasteConfirm],
            _ => &[],
        }
    }


    pub fn say(self) -> &'static str {
        match self {
            Word::AwaitPay => "待付款",
            Word::AwaitDeploy => "待部署",
            Word::AwaitAccept => "待接单",
            Word::AwaitDelivery => "待交付",
            Word::AwaitConfirm => "待确认",
            Word::ConfirmHanded => "已确认,待对方提交",
            Word::Disputed => "争议中",
            Word::AwaitRedraw => "待抽签",
            Word::Completed => "已完成",
            Word::Ruled => "已裁决",
            Word::Settled => "已和解",
            Word::Refunded => "已退款",
            Word::Lapsed => "已结清",
            Word::Expired => "已过期",
            Word::Cancelled => "已撤单",
            Word::Revoked => "已下架",
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Word::AwaitPay => "awaitPay",
            Word::AwaitDeploy => "awaitDeploy",
            Word::AwaitAccept => "awaitAccept",
            Word::AwaitDelivery => "awaitDelivery",
            Word::AwaitConfirm => "awaitConfirm",
            Word::ConfirmHanded => "confirmHanded",
            Word::Disputed => "disputed",
            Word::AwaitRedraw => "awaitRedraw",
            Word::Completed => "completed",
            Word::Ruled => "ruled",
            Word::Settled => "settled",
            Word::Refunded => "refunded",
            Word::Lapsed => "lapsed",
            Word::Expired => "expired",
            Word::Cancelled => "cancelled",
            Word::Revoked => "revoked",
        }
    }

    pub fn done(self) -> bool {
        matches!(self, Word::Completed | Word::Ruled | Word::Settled | Word::Refunded | Word::Lapsed | Word::Expired | Word::Cancelled)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub word: Word,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    DeliveryLate,
    NotAccepted,
    SellerRefund,
    Declined,
    Resigned,
    NoneDrawn,
    ResignedInDispute,
    DisputedAfterResign,
    JudgeLate,
}

impl Ended {
    pub fn word(self) -> &'static str {
        match self {
            Ended::DeliveryLate => "deliveryLate",
            Ended::NotAccepted => "notAccepted",
            Ended::SellerRefund => "sellerRefund",
            Ended::Declined => "declined",
            Ended::Resigned => "resigned",
            Ended::NoneDrawn => "noneDrawn",
            Ended::ResignedInDispute => "resignedInDispute",
            Ended::DisputedAfterResign => "disputedAfterResign",
            Ended::JudgeLate => "judgeLate",
        }
    }
}

pub fn ended(v: &CaseView) -> Option<Ended> {
    match v.phase {
        4 => match last_arg(v, "Refunded", 0) {
            Some(0) => Some(Ended::DeliveryLate),
            Some(1) => Some(Ended::NotAccepted),
            Some(3) => Some(Ended::SellerRefund),
            _ => None,
        },
        8 => match last_arg(v, "Resigned", 1) {
            Some(0) => Some(Ended::Declined),
            Some(3) => Some(Ended::NoneDrawn),
            _ => Some(Ended::Resigned),
        },
        9 => match last_arg(v, "Resigned", 1) {
            Some(2) => Some(Ended::ResignedInDispute),
            Some(1) => Some(Ended::DisputedAfterResign),
            _ => Some(Ended::JudgeLate),
        },
        _ => None,
    }
}

fn last_arg(v: &CaseView, name: &str, i: usize) -> Option<u64> {
    v.events.iter().rev().find(|e| e.name == name).and_then(|e| e.args.get(i)).and_then(|x| x.parse().ok())
}

fn refunded(cause: Option<u64>) -> Result<Status, Why> {
    let (word, reason) = match cause {
        Some(0) => (Word::Refunded, "交付逾期"),
        Some(1) => (Word::Refunded, "判官未接单"),
        Some(2) => (Word::Expired, ""),
        Some(3) => (Word::Refunded, "卖家退款"),
        Some(4) => (Word::Cancelled, ""),
        other => return Err(why("E_ORDERS_STATE", format!("Refunded 的缘由认不出:{other:?}"))),
    };
    Ok(Status { word, reason: reason.into() })
}


fn plain(word: Word) -> Status {
    Status { word, reason: String::new() }
}

pub fn judge(local: Option<State>, chain: Option<&CaseView>) -> Result<Status, Why> {
    judge_handed(local, chain, false)
}

pub fn judge_handed(local: Option<State>, chain: Option<&CaseView>, handed: bool) -> Result<Status, Why> {
    judge_noted(local, chain, handed, "")
}

fn holds_hand(handed: bool) -> bool {
    handed
}


pub fn refund_open(handed: bool) -> Result<(), (String, String)> {
    if holds_hand(handed) {
        return Err((E_HANDED.to_string(), "这一单的确认已交出(交出即同意,eravon.terms/1 §5.5):不能再申请退款,只能等对方提交或自己发出确认".to_string()));
    }
    Ok(())
}

pub const E_HANDED: &str = "E_P6_HANDED";

pub fn judge_noted(local: Option<State>, chain: Option<&CaseView>, handed: bool, note: &str) -> Result<Status, Why> {
    if let Some(v) = chain.filter(|v| v.phase != 0) {
        return match v.phase {
            1 if v.pending_redraw => Ok(plain(Word::AwaitRedraw)),
            1 if v.accepted == 0 => Ok(plain(Word::AwaitAccept)),
            1 => Ok(plain(Word::AwaitDelivery)),
            2 if holds_hand(handed) => Ok(plain(Word::ConfirmHanded)),
            2 => Ok(plain(Word::AwaitConfirm)),
            3 => Ok(plain(Word::Disputed)),
            4 => refunded(last_arg(v, "Refunded", 0)),
            5 => Ok(plain(Word::Completed)),
            6 => Ok(plain(Word::Settled)),
            7 => Ok(plain(Word::Ruled)),
            8 => Ok(Status {
                word: Word::Refunded,
                reason: if last_arg(v, "Resigned", 1) == Some(3) { "未抽到判官" } else { "判官辞任" }.into(),
            }),
            9 => Ok(plain(Word::Lapsed)),
            n => Err(why("E_ORDERS_STATE", format!("阶段号 {n} 认不出"))),
        };
    }
    match local {
        Some(State::Awaiting) | Some(State::Short) => Ok(plain(Word::AwaitPay)),
        Some(State::Ready) if note == eravon_watch::send::OPEN_OFF_NOTE => Ok(Status { word: Word::AwaitDeploy, reason: note.to_string() }),
        Some(State::Ready) => Ok(plain(Word::AwaitDeploy)),
        Some(State::PendingConsent) | Some(State::PriceStale) | Some(State::Blocked) | Some(State::AwaitPrev) => Ok(Status { word: Word::AwaitDeploy, reason: note.to_string() }),
        Some(State::DepositShort) => Ok(Status { word: Word::AwaitDeploy, reason: "卖家押金不足".into() }),
        Some(State::Expired) => Ok(plain(Word::Expired)),
        Some(State::Revoked) => Ok(plain(Word::Revoked)),
        Some(State::Cancelled) => Ok(plain(Word::Cancelled)),
        Some(State::Deployed) => Err(why("E_ORDERS_STATE", "本机记录已部署,但链上未找到合约")),
        None => Err(why("E_ORDERS_STATE", "本机与链上都找不到该订单")),
    }
}

#[cfg(test)]
mod open_off_tests {
    use super::*;

    #[test]
    fn a_paid_order_waiting_for_the_seller_says_so() {
        let off = judge_noted(Some(State::Ready), None, false, eravon_watch::send::OPEN_OFF_NOTE).expect("读得出");
        assert_eq!((off.word, off.reason.as_str()), (Word::AwaitDeploy, eravon_watch::send::OPEN_OFF_NOTE));
        let plain = judge_noted(Some(State::Ready), None, false, eravon_watch::send::NO_KEY_NOTE).expect("读得出");
        assert_eq!((plain.word, plain.reason.as_str()), (Word::AwaitDeploy, ""));
    }
}

#[cfg(test)]
mod ended_tests {
    use super::*;
    use eravon_dispute::view::Ev;
    use eravon_terms::u256::U256;

    fn view(phase: u64, events: Vec<(&str, Vec<&str>)>) -> CaseView {
        let n = |x: u64| U256::from_dec(&x.to_string()).unwrap();
        CaseView {
            phase,
            seats: 1,
            premium: n(0),
            deposit: n(0),
            lf_held: n(0),
            pending_redraw: false,
            redraw_deadline: 0,
            judge: "0x11".into(),
            accepted: 7,
            window_end: 1_800_000_000,
            seat_state: Some(2),
            amount: n(1),
            token: "0x22".into(),
            buyer_payout: "0x33".into(),
            seller_fund: "0x44".into(),
            seller_payout: "0x55".into(),
            lf: n(0),
            events: events.into_iter().enumerate().map(|(i, (name, args))| Ev { name: name.into(), block: 10 + i as u64, log_index: 0, tx: String::new(), args: args.into_iter().map(str::to_string).collect() }).collect(),
            past: false,
            logic: "0x66".into(),
        }
    }

    #[test]
    fn why_a_case_ended_is_read_from_its_events() {
        assert_eq!(ended(&view(4, vec![("Refunded", vec!["0"])])), Some(Ended::DeliveryLate));
        assert_eq!(ended(&view(4, vec![("Refunded", vec!["1"])])), Some(Ended::NotAccepted));
        assert_eq!(ended(&view(4, vec![("Refunded", vec!["3"])])), Some(Ended::SellerRefund));
        assert_eq!(ended(&view(8, vec![("Resigned", vec!["0x00", "0"])])), Some(Ended::Declined));
        assert_eq!(ended(&view(8, vec![("Resigned", vec!["0x00", "1"])])), Some(Ended::Resigned));
        assert_eq!(ended(&view(8, vec![("Resigned", vec!["0x00", "3"])])), Some(Ended::NoneDrawn));
        assert_eq!(ended(&view(9, vec![("Disputed", vec![]), ("Resigned", vec!["0x00", "2"]), ("Lapsed", vec!["5000"])])), Some(Ended::ResignedInDispute));
        assert_eq!(ended(&view(9, vec![("Resigned", vec!["0x00", "1"]), ("Disputed", vec![]), ("Lapsed", vec!["0"])])), Some(Ended::DisputedAfterResign));
        assert_eq!(ended(&view(9, vec![("Disputed", vec![]), ("Lapsed", vec!["0"])])), Some(Ended::JudgeLate));
        assert_eq!(ended(&view(5, vec![])), None);
        assert_eq!(ended(&view(2, vec![])), None);
    }

    #[test]
    fn a_lapse_after_a_declined_or_undrawn_seat_is_the_judge_late() {
        assert_eq!(ended(&view(9, vec![("Resigned", vec!["0x00", "0"]), ("Accepted", vec![]), ("Disputed", vec![]), ("Lapsed", vec!["5000"])])), Some(Ended::JudgeLate));
        assert_eq!(ended(&view(9, vec![("Resigned", vec!["0x00", "3"]), ("Disputed", vec![]), ("Lapsed", vec!["0"])])), Some(Ended::JudgeLate));
    }
}
