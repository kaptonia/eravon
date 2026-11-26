use eravon_chain::endpoints::Table;
use eravon_chain::seam::Upto;
use eravon_chain::transport::Transport;
use eravon_galeed::book::{Accession, Declaration, Logs};
use eravon_literals::constants as k7;
use eravon_standard::order::hex_of;
use eravon_standard::sol;

use crate::fault::R;
use crate::log::Log;

pub fn logs<T: Transport>(t: &T, table: &Table) -> R<Logs> {
    records(t, table, 0, Upto::Latest)
}

fn records<T: Transport>(t: &T, table: &Table, from: u64, to: Upto) -> R<Logs> {
    let (d0, a0) = (hex_of(&sol::BOOK_DECLARED.topic0()), hex_of(&sol::BOOK_ACCEDED.topic0()));
    let (rows, _) = eravon_chain::read::logs_any(t, table, Some(k7::SUCCESSION_BOOK), &[vec![d0.clone(), a0.clone()]], from, to)?;
    let mut out = Logs::default();
    for row in &rows {
        let l = Log::read(row)?;
        if !l.address.eq_ignore_ascii_case(k7::SUCCESSION_BOOK) {
            continue;
        }
        let t0 = l.topics.first().map(|x| hex_of(x)).unwrap_or_default();
        if t0 == d0 && l.topics.len() == 3 {
            out.declarations.push(Declaration { emitter: l.topic_address(1)?, block_number: l.block, log_index: l.log_index, to: l.topic_address(2)? });
        } else if t0 == a0 && l.topics.len() == 4 {
            out.accessions.push(Accession { emitter: l.topic_address(2)?, block_number: l.block, log_index: l.log_index, holder: l.topic_address(1)?, root: l.topic_address(3)? });
        }
    }
    Ok(out)
}

pub fn current_at(book: &Logs, a: &str, at: (u64, u64)) -> String {
    eravon_galeed::book::current_at(book, a, at).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_at_follows_the_record_by_position() {
        let (a, b, c) = ("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000b1", "0x00000000000000000000000000000000000000c1");
        let book = Logs {
            publications: vec![],
            statuses: vec![],
            declarations: vec![Declaration { emitter: a.into(), block_number: 10, log_index: 0, to: b.into() }],
            accessions: vec![Accession { emitter: b.into(), block_number: 11, log_index: 0, holder: a.into(), root: a.into() }],
        };
        assert_eq!(current_at(&book, a, (11, 0)), a, "接任那一条之前仍是冻结钥");
        assert_eq!(current_at(&book, a, (11, 1)), b, "接任之后是新钥");
        assert_eq!(current_at(&book, c, (99, 0)), c, "没换过的钥即它自己");
    }
}
