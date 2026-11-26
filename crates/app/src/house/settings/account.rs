use super::*;

pub fn gas_of(data: &str, me: &str) -> Result<(String, String, usize), Said> {
    wire(data, me).and_then(|(t, table)| {
        eravon_chain::read::balance(&t, &table, me, eravon_chain::seam::TAG_LATEST).map(|(wei, r)| (wei, r.agreement.word().to_string(), r.endpoints)).map_err(|e| bad(e.name(), e.detail()))
    })
}

impl Settings {
    pub fn read_gas(&mut self, data: &str) {
        self.gas = Some(gas_of(data, &self.me));
    }

    pub fn drop_reads(&mut self) {
        self.gas = None;
        self.gas_book = None;
        self.token_read.clear();
    }

    pub fn from_book(&mut self, book: Option<&crate::house::book::Book>) {
        let Some(b) = book.filter(|b| !self.me.is_empty() && b.key.eq_ignore_ascii_case(&self.me)) else {
            self.gas_book = None;
            return;
        };
        self.gas_book = b.balances.as_ref().filter(|x| x.holder.eq_ignore_ascii_case(&self.me)).and_then(|x| x.native.clone().map(|w| (w, x.meta.agreement.clone(), x.meta.at)));
        self.take_read(b.tokens.clone());
    }
}

pub(crate) fn payout_land(data: &str, me: &str, a: &str, again: &str, now: u64, word: &str) -> Said {
    match h2::payout::set(data, me, a, again) {
        Err(e) => h2bad(e),
        Ok(got) => match eravon_goods::goods::note_payout(data, seam::WRITER, me, now) {
            Ok(_) => ok(format!("{word} · {got}")),
            Err(e) => bad(e.name(), e.detail()),
        },
    }
}
