use crate::house::seller::Said;

#[derive(Clone, Debug)]
pub struct TimeAsk {
    pub say: String,
    pub replay: Replay,
}

#[derive(Clone, Debug)]
pub enum Replay {
    Goods { id: String, dir: String },
    Plan { id: String, dir: String, plan: eravon_standard::proposal::Plan, post: Post },
    Paylink,
}

#[derive(Clone, Debug)]
pub enum Post {
    None,
    Settle(eravon_dispute::settle::Offer),
    Open { buyer: String },
}

pub fn of_fault(f: &eravon_cli::fault::Fault) -> Option<String> {
    eravon_cli::company::time_asked(f)
}

pub fn out(a: &Option<TimeAsk>) -> Option<eravon_literals::json::Out> {
    a.as_ref().map(|x| eravon_literals::json::Out::S(x.say.clone()))
}

pub fn said(say: &str) -> Said {
    Said::bad(eravon_cli::company::TIME_ASK, say)
}
