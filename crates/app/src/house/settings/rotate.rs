use super::*;

#[derive(Clone, Debug, Default)]
pub struct RotView {
    pub chains: Vec<(String, u64, &'static str, String, eravon_store::vault::Seat)>,
    pub txs: u64,
    pub txs_most: u64,
    pub owed: Vec<eravon_store::settings::HandOn>,
    pub owed_other: usize,
    pub cases: usize,
    pub cases_unread: Vec<(u64, String, String)>,
    pub orders: usize,
    pub offers: usize,
    pub deposits: usize,
    pub refunds: Vec<String>,
    pub handed: usize,
    pub order_ids: Vec<String>,
    pub product_ids: Vec<String>,
    pub series_short: Vec<(String, u64, u64)>,
    pub new: String,
    pub phrase: Vec<eravon_ui::secret::Secret>,
    pub phrase_open: bool,
    pub sent: usize,
    pub left: Vec<String>,
    pub stops: Vec<eravon_cli::vaultverbs::rotate::Left>,
    pub swept: Vec<eravon_cli::vaultverbs::rotate::Swept>,
    pub unswept: Vec<u64>,
    pub stranded: Vec<(String, u64)>,
    pub refused: Vec<(eravon_store::vault::Seat, String, String)>,
    pub unfinished: bool,
    pub backup_again: bool,
    pub left_on: Vec<String>,
}

impl RotView {
    fn chains_of(cs: &[eravon_cli::vaultverbs::rotate::OnChain]) -> Vec<(String, u64, &'static str, String, eravon_store::vault::Seat)> {
        cs.iter()
            .map(|c| {
                let detail = match &c.seg {
                    eravon_cli::vaultverbs::rotate::Seg::Claim(k) | eravon_cli::vaultverbs::rotate::Seg::Moved(k) => k.clone(),
                    eravon_cli::vaultverbs::rotate::Seg::Unread(w) => w.clone(),
                    _ => String::new(),
                };
                (c.name.to_string(), c.chain, c.seg.word(), detail, c.seat)
            })
            .collect()
    }

    pub fn awaiting(&self) -> bool {
        self.unfinished || self.chains.iter().any(|c| c.2 == "awaitingAccession") || !self.unswept.is_empty() || !self.owed.is_empty() || self.owed_other > 0
    }

    pub fn opens_on_resume(&self) -> bool {
        self.new.is_empty() && self.awaiting()
    }

    pub fn out(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        Out::O(vec![
            ("backupAgain", Out::B(self.backup_again)),
            ("cases", Out::N(self.cases as u64)),
            ("casesUnread", Out::A(self.cases_unread.iter().map(|(c, n, w)| Out::O(vec![("chainId", Out::N(*c)), ("name", Out::S(n.clone())), ("why", Out::S(w.clone()))])).collect())),
            ("chains", Out::A(self.chains.iter().map(|(n, c, st, d, _)| Out::O(vec![("chainId", Out::N(*c)), ("detail", s(d)), ("name", s(n)), ("state", s(st))])).collect())),
            ("deposits", Out::N(self.deposits as u64)),
            ("handed", Out::N(self.handed as u64)),
            ("left", Out::A(self.left.iter().map(|x| s(x)).collect())),
            ("leftOn", Out::A(self.left_on.iter().map(|x| s(x)).collect())),
            ("new", s(&self.new)),
            ("offers", Out::N(self.offers as u64)),
            ("productIds", Out::A(self.product_ids.iter().map(|x| s(x)).collect())),
            ("refunds", Out::A(self.refunds.iter().map(|x| s(x)).collect())),
            ("refused", Out::A(self.refused.iter().map(|(st, n, w)| Out::O(vec![("name", s(n)), ("seat", s(st.word())), ("why", s(w))])).collect())),
            ("seriesShort", Out::A(self.series_short.iter().map(|(id, have, all)| Out::O(vec![("id", s(id)), ("listed", Out::N(*have)), ("stages", Out::N(*all))])).collect())),
            ("orderIds", Out::A(self.order_ids.iter().map(|x| s(x)).collect())),
            ("orders", Out::N(self.orders as u64)),
            ("owed", Out::A(self.owed.iter().map(|h| Out::O(vec![("chainId", Out::N(h.chain)), ("how", s(&h.how)), ("id", s(&h.id)), ("kind", s(&h.kind))])).collect())),
            ("owedOther", Out::N(self.owed_other as u64)),
            ("phraseShown", Out::B(!self.phrase.is_empty())),
            ("sent", Out::N(self.sent as u64)),
            ("swept", Out::A(self.swept.iter().map(|x| {
                let mut m = vec![("chainId", Out::N(x.chain)), ("state", s(x.how.word()))];
                match &x.how {
                    eravon_cli::vaultverbs::rotate::Sweep::Moved { tx, amount } => m.extend([("amount", s(amount)), ("tx", s(tx))]),
                    eravon_cli::vaultverbs::rotate::Sweep::Short { balance, fee } => m.extend([("balance", s(balance)), ("fee", s(fee))]),
                }
                Out::O(m)
            }).collect())),
            ("txs", Out::N(self.txs)),
            ("unfinished", Out::B(self.unfinished)),
            ("txsAtMost", Out::N(self.txs_most)),
            ("stranded", Out::A(self.stranded.iter().map(|(k, c)| Out::O(vec![("chainId", Out::N(*c)), ("key", s(k))])).collect())),
            ("unswept", Out::A(self.unswept.iter().map(|c| Out::N(*c)).collect())),
        ])
    }
}

fn ids<'a>(it: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for x in it {
        if !out.contains(x) {
            out.push(x.clone());
        }
    }
    out
}

fn said(f: eravon_cli::fault::Fault) -> Said {
    match f {
        eravon_cli::fault::Fault::Down { token, why, .. } => bad(&token, why),
        other => bad(other.name(), other.detail()),
    }
}

impl Settings {
    pub fn rotate_list(&mut self, data: &str, s: &eravon_vault::session::Session, now: u64) {
        let i = eravon_cli::vaultverbs::rotate::In { data, s, reach: &eravon_cli::vaultverbs::rotate::Reach::default(), seat: self.seat.store(), old: &self.me, now, let_expire: &[], revoke: &[], coins: Some(&[]) };
        let me = self.me.clone();
        self.rotate = Some(eravon_cli::vaultverbs::rotate::listed(&i).and_then(|(from, n, l)| eravon_cli::vaultverbs::rotate::part_way(data, &me, &from, &l).map(|part| ((from, n, l), part))).map_err(said).map(|((_, _, l), part)| RotView {
            unfinished: part,
            chains: RotView::chains_of(&l.chains),
            txs: l.txs,
            txs_most: l.txs_most,
            owed: l.owed.clone(),
            owed_other: l.owed_other,
            cases: l.cases.len(),
            cases_unread: l.cases_unread.iter().map(|(c, n, w, _)| (*c, n.clone(), w.clone())).collect(),
            orders: l.orders.len(),
            offers: l.offers.len(),
            deposits: l.deposits.len(),
            refunds: ids(l.refunds.iter().map(|r| &r.0)),
            handed: l.handed.len(),
            order_ids: ids(l.orders.iter().map(|o| &o.0)),
            product_ids: ids(l.terms.iter().map(|t| &t.0)),
            series_short: l.series_short.clone(),
            unswept: l.unswept.clone(),
            stranded: l.stranded.iter().map(|(k, c, _)| (k.clone(), *c)).collect(),
            refused: l.refused.clone(),
            ..RotView::default()
        }));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rotate_go(&mut self, data: &str, s: &eravon_vault::session::Session, keep: bool, let_expire: &[String], revoke: &[String], leave: bool, now: u64) {
        let reach = eravon_cli::vaultverbs::rotate::Reach::default();
        let i = eravon_cli::vaultverbs::rotate::In { data, s, reach: &reach, seat: self.seat.store(), old: &self.me, now, let_expire, revoke, coins: None };
        let left_on = if leave && keep { eravon_cli::vaultverbs::rotate::held_on(&i) } else { Vec::new() };
        let none: [String; 0] = [];
        let i = if leave { eravon_cli::vaultverbs::rotate::In { coins: Some(&none), ..i } } else { i };
        let got = if keep { eravon_cli::vaultverbs::rotate::continue_on(&i, seam::WRITER) } else { eravon_cli::vaultverbs::rotate::fresh(&i, seam::WRITER) };
        self.rotate = Some(got.map_err(said).map(|d| RotView {
            chains: RotView::chains_of(&d.chains),
            new: d.new.clone(),
            phrase: d.phrase.as_deref().map(|p| p.split_whitespace().map(eravon_ui::secret::Secret::of).collect()).unwrap_or_default(),
            sent: d.sent.len(),
            left: d.left.iter().map(|l| l.chain.map_or_else(|| format!("{} · {}", l.what, l.why()), |c| format!("{} · {c} · {}", l.what, l.why()))).collect(),
            stops: d.left.clone(),
            swept: d.swept.clone(),
            unswept: d.unswept.clone(),
            owed: d.owed.clone(),
            owed_other: d.owed_other,
            backup_again: d.backup_again,
            left_on: left_on.clone(),
            ..RotView::default()
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::RotView;
    use eravon_store::vault::Seat;

    fn one_chain_took_the_other_did_not(unfinished: bool) -> RotView {
        RotView {
            chains: vec![("sepolia".into(), 11_155_111, "attempt", String::new(), Seat::Seller), ("sepolia".into(), 11_155_111, "attempt", String::new(), Seat::Buyer)],
            unfinished,
            ..RotView::default()
        }
    }

    #[test]
    fn a_change_left_part_way_opens_on_going_on_with_it() {
        let left = one_chain_took_the_other_did_not(true);
        assert!(left.awaiting() && left.opens_on_resume());
        let words_alone = one_chain_took_the_other_did_not(false);
        assert!(!words_alone.awaiting(), "the chains' words alone: nothing waits");
        let done = RotView { new: "0x00000000000000000000000000000000000000b2".into(), ..one_chain_took_the_other_did_not(true) };
        assert!(done.awaiting() && !done.opens_on_resume(), "after a run the card shows the run");
        let out = String::from_utf8(eravon_literals::json::write(&left.out())).expect("text");
        assert!(out.contains("\"unfinished\":true"), "{out}");
    }
}
