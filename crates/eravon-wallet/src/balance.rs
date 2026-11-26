use eravon_cli::shared::Wire;
use eravon_standard as a1;
use eravon_terms::u256::U256;

use crate::fault::Fault;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coin {
    pub token: String,
    pub symbol: String,
    pub decimals: Option<u64>,
    pub have: Result<String, Fault>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Balances {
    pub address: String,
    pub native: Result<String, Fault>,
    pub coins: Vec<Coin>,
}

impl Balances {
    pub fn coin(&self, token: &str) -> Option<&Coin> {
        self.coins.iter().find(|c| c.token.eq_ignore_ascii_case(token))
    }
}

pub fn settlement(data: &str) -> Vec<(String, String, Option<u8>)> {
    let rows = eravon_vault::tokens::list(data);
    eravon_vault::tokens::on_chain(&rows, eravon_vault::tokens::chain()).into_iter().map(|r| (r.label, r.address, r.decimals)).collect()
}

fn said(r: Result<String, Fault>) -> Result<String, Fault> {
    r
}


pub fn read(w: &Wire, data: &str, me: &str, extra: &[String]) -> Balances {
    crate::mark::mark(crate::mark::FEATURE);
    let mut tokens: Vec<(String, String, Option<u8>)> = settlement(data);
    let seen: Vec<String> = crate::tx::list(data, me).map(|v| v.into_iter().map(|t| t.token).collect()).unwrap_or_default();
    for t in seen.iter().chain(extra.iter()) {
        let t = t.to_ascii_lowercase();
        if eravon_chain::seam::address_ok(&t) && !tokens.iter().any(|(_, a, _)| *a == t) {
            tokens.push((String::new(), t, None));
        }
    }
    eravon_chain::read::together(&w.t, &w.table, || {
        eravon_chain::read::ahead_balances(&w.t, &w.table, &[me.to_string()], "latest");
        a1::chain::ahead_erc20_balances(&w.t, &w.table, &tokens.iter().map(|(_, a, _)| (a.clone(), me.to_string())).collect::<Vec<_>>());
        a1::chain::ahead_decimals(&w.t, &w.table, &tokens.iter().filter(|(_, _, listed)| listed.is_none()).map(|(_, a, _)| a.clone()).collect::<Vec<_>>());
    });
    let native = said(eravon_chain::read::balance(&w.t, &w.table, me, "latest").map(|x| x.0).map_err(Fault::from));
    let coins = tokens
        .into_iter()
        .map(|(symbol, token, listed)| {
            let have = said(a1::chain::erc20_balance(&w.t, &w.table, &token, me).map(|r| U256::from_be(&r.value).to_dec()).map_err(Fault::from));
            let decimals = listed.map(u64::from).or_else(|| if have.is_ok() { a1::chain::side_decimals(&w.t, &w.table, &token) } else { None });
            Coin { token, symbol, decimals, have }
        })
        .collect();
    Balances { address: me.to_ascii_lowercase(), native, coins }
}

pub fn formers(w: &Wire, data: &str, me: &str, extra: &[String]) -> Vec<Balances> {
    let Ok(keys) = eravon_vault::store::keys_of(data, me) else { return Vec::new() };
    keys.into_iter()
        .filter(|k| !k.eq_ignore_ascii_case(me))
        .map(|k| read(w, data, &k, extra))
        .filter(|b| movable(w, b, me))
        .collect()
}

pub fn movable(w: &Wire, b: &Balances, to: &str) -> bool {
    b.coins.iter().any(|c| eravon_cli::shared::former_movable(w, to, Some(c.token.as_str()), &c.have)) || eravon_cli::shared::former_movable(w, to, None, &b.native)
}

pub fn receive_qr(address: &str) -> Option<(usize, Vec<bool>)> {
    let code = eravon_qr::make(address.as_bytes()).ok()?;
    Some((code.size, code.modules.into_iter().flatten().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_is_the_pinned_rows_table() {
        let data = eravon_vault::testing::fresh();
        let rows = eravon_literals::constants::settlement_tokens();
        assert!(!rows.is_empty());
        let got = settlement(&data);
        assert_eq!(got.len(), rows.len());
        for (t, (gs, ga, gd)) in rows.iter().zip(got.iter()) {
            assert_eq!((t.symbol, t.address.to_ascii_lowercase(), Some(t.decimals)), (gs.as_str(), ga.clone(), *gd));
        }
    }

    #[test]
    fn qr_of_an_address() {
        let addr = "0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc";
        let (n, cells) = receive_qr(addr).expect("装得下");
        assert_eq!(cells.len(), n * n);
        let code = eravon_qr::make(addr.as_bytes()).expect("made");
        assert_eq!((code.version, n), (eravon_qr::version_for(addr.len()).expect("fits"), 29));
        assert_eq!(eravon_qr::decode(&code), Ok(addr.as_bytes().to_vec()));
        assert_eq!(cells, code.modules.concat());
    }
}
