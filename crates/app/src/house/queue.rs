use eravon_literals::json::Out;

pub use eravon_cli::deskops::direct::Direct;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderOut {
    pub instance: String,
    pub what: String,
    pub tx: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct View {
    pub direct: Option<Result<Vec<Direct>, (String, String)>>,
    pub orders: Option<Result<Vec<OrderOut>, (String, String)>>,
    pub open_direct: Option<String>,
    pub said: Option<crate::house::seller::Said>,
}

pub fn direct_of(session: &eravon_vault::session::Session, me: &str) -> Result<Vec<Direct>, (String, String)> {
    let key = session.key_for(me).map_err(|e| (e.name().to_string(), e.detail()))?;
    eravon_cli::deskops::direct::open(me, &key).map_err(|n| (n.name.clone(), n.detail()))
}

pub fn orders_of(data: &str, me: &str) -> Result<Vec<OrderOut>, (String, String)> {
    Ok(out_of(&eravon_watch::order::all(data, me)?))
}

fn out_of(all: &[eravon_watch::order::Order]) -> Vec<OrderOut> {
    all.iter().filter_map(|o| o.pending().map(|(what, tx)| OrderOut { instance: o.instance.clone(), what, tx })).collect()
}

pub fn direct_check(data: &str, me: &str, key: &[u8; 32], d: &Direct) -> crate::house::seller::Said {
    use crate::house::seller::{Said, Tone};
    use eravon_cli::deskops::direct::Checked;
    let w = match crate::house::orders::wire(data, me) {
        Ok(w) => w,
        Err(s) => return s,
    };
    match eravon_cli::deskops::direct::check(&w, d, key) {
        Ok(Checked::Done { tx, block, status: 1 }) => Said::ok(format!("{tx} · {block}")),
        Ok(Checked::Done { tx, .. }) => Said::bad("E_V_REVERTED", tx),
        Ok(Checked::Void(tx)) => Said::bad("E_V_SENT_VOID", tx),
        Ok(Checked::Pending(tx)) => Said { tone: Tone::Note, name: "submitted".into(), detail: tx },
        Err(n) => Said::bad(&n.name, n.detail()),
    }
}

pub fn direct_give_up(data: &str, me: &str, key: &[u8; 32], d: &Direct) -> crate::house::seller::Said {
    use crate::house::seller::Said;
    let w = match crate::house::orders::wire(data, me) {
        Ok(w) => w,
        Err(s) => return s,
    };
    match eravon_cli::deskops::direct::give_up(&w, d, key) {
        Ok(()) => Said::ok(crate::house::i18n::t(crate::house::i18n::Key::QueueDirectGone)),
        Err(n) => Said::bad(&n.name, n.detail()),
    }
}

impl View {
    pub fn rows(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        let err = |(n, d): &(String, String)| Out::O(vec![("detail", s(d)), ("name", s(n))]);
        let direct = match &self.direct {
            None => Out::Null,
            Some(Err(e)) => err(e),
            Some(Ok(v)) => Out::A(v.iter().map(|d| Out::O(vec![("from", s(&d.from)), ("nonce", Out::N(d.at.nonce)), ("to", s(&d.to)), ("txs", Out::A(d.txs.iter().map(|t| s(t)).collect()))])).collect()),
        };
        let orders = match &self.orders {
            None => Out::Null,
            Some(Err(e)) => err(e),
            Some(Ok(v)) => Out::A(v.iter().map(|o| Out::O(vec![("instance", s(&o.instance)), ("tx", s(&o.tx)), ("what", s(&o.what))])).collect()),
        };
        Out::O(vec![("direct", direct), ("directSaid", self.said.as_ref().map_or(Out::Null, |x| x.out())), ("orders", orders)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_rows_are_the_orders_with_a_transaction_out() {
        let order = |id: &str, instance: &str| eravon_watch::order::Order {
            id: id.into(),
            case_id: String::new(),
            instance: instance.into(),
            buyer_key: String::new(),
            buyer_payout: String::new(),
            verdict: String::new(),
            token: String::new(),
            need: "1".into(),
            open_by: 0,
            candidate: 0,
            expiry: 0,
            state: eravon_watch::order::State::Ready,
            balance: String::new(),
            gap: String::new(),
            note: String::new(),
            notice: String::new(),
            notice_at: 0,
            deploy_tx: String::new(),
            expire_tx: String::new(),
            source: "paste".into(),
            opened: 0,
            extra: Vec::new(),
        };
        let quiet = order("o1", "0x00000000000000000000000000000000000000b1");
        let mut out = order("o2", "0x00000000000000000000000000000000000000b2");
        out.set_pending("deploy", "0xabc", &eravon_watch::order::Broadcast { nonce: 4, max_fee: "2".into(), priority_fee: "1".into(), gas_limit: 21000 });
        assert_eq!(out_of(&[quiet.clone()]), Vec::new());
        assert_eq!(out_of(&[quiet, out.clone()]), vec![OrderOut { instance: out.instance.clone(), what: "deploy".into(), tx: "0xabc".into() }]);
    }
}
