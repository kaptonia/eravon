use eravon_chain as a2;
use eravon_link as k3;
use eravon_standard as a1;
use eravon_terms as k2;

use crate::fault::{Fault, R};

pub struct Chain {
    pub w: eravon_cli::shared::Wire,
    pub cw: eravon_cands::wire::Wire,
    pub registry: Result<(), Fault>,
    pub buyer: String,
    pub now: u64,
    pub data: String,
}

pub fn act(c: &Chain, o: &a1::order::Order) -> Result<eravon_cands::act::Passed, eravon_cands::act::Refused> {
    eravon_cli::deskops::paypage::act(&c.view(), o)
}

fn fault((name, why): eravon_cli::deskops::paypage::Fail) -> Fault {
    Fault::new(&name, why)
}

impl Chain {
    pub fn view(&self) -> eravon_cli::deskops::paypage::PayChain<'_> {
        eravon_cli::deskops::paypage::PayChain {
            w: &self.w,
            cw: &self.cw,
            registry: self.registry.clone().map_err(|f| (f.token, f.detail)),
            buyer: &self.buyer,
            now: self.now,
            data: &self.data,
        }
    }

    pub fn of(data: &str, buyer: &str, injected: &[String], now: u64) -> R<Chain> {
        let nodes = eravon_vault::store::endpoints(data, buyer)?;
        if nodes.is_empty() {
            return Err(Fault::new("E_P3_NO_ENDPOINT", "未配置节点"));
        }
        let table = || a2::endpoints::Table::new(nodes.clone()).map_err(Fault::from);
        let t = || a2::transport::Auto::web_roots().map_err(Fault::from);
        let (chain_id, logic, _) = a1::chain::standard_table();
        let (registries, registry) = match eravon_cands::wire::registries(injected) {
            Ok(r) => (r, Ok(())),
            Err(a) => (Vec::new(), Err(Fault::from(a))),
        };
        Ok(Chain {
            w: eravon_cli::shared::Wire { t: t()?, table: table()?, source: eravon_cli::home::Source::Identity, chain_id, logic: logic.to_ascii_lowercase(), registries: injected.to_vec() },
            cw: eravon_cands::wire::Wire { t: t()?, table: table()?, registries, now },
            registry,
            buyer: buyer.to_ascii_lowercase(),
            now,
            data: data.to_string(),
        })
    }
}

pub fn quote_of(c: &Chain, p: &k3::listing::Listing, n: usize) -> R<k2::quote::Quote> {
    k2::quote::decode(&quote_bytes_of(c, p, n)?).map_err(|f| Fault::new("E_P3_QUOTE", format!("报价读不成:{:?}", f.token)))
}

pub fn quote_bytes_of(c: &Chain, p: &k3::listing::Listing, n: usize) -> R<Vec<u8>> {
    eravon_cli::deskops::paypage::quote_bytes_of(&c.view(), p, n).map_err(fault)
}

pub fn fees(c: &Chain, p: &k3::listing::Listing, n: usize, quote: &[u8]) -> R<a1::precheck::Quoted> {
    eravon_cli::deskops::paypage::fees(&c.view(), p, n, quote).map_err(fault)
}

pub type Each = Vec<(usize, R<Vec<u8>>)>;

pub fn candidates(c: &Chain, p: &k3::listing::Listing) -> Each {
    candidates_on(&c.view(), p).into_iter().map(|(n, r)| (n, r.map_err(fault))).collect()
}

pub fn candidates_on(c: &eravon_cli::deskops::paypage::PayChain, p: &k3::listing::Listing) -> eravon_cli::deskops::paypage::Each {
    crate::mark::mark(crate::mark::FEATURE);
    eravon_cli::deskops::paypage::candidates(c, p)
}

pub fn standard(c: &Chain, p: &k3::listing::Listing) -> R<()> {
    standard_on(&c.view(), p).map_err(fault)
}

pub fn standard_on(c: &eravon_cli::deskops::paypage::PayChain, p: &k3::listing::Listing) -> Result<(), eravon_cli::deskops::paypage::Fail> {
    eravon_cli::deskops::paypage::standard(c, p)
}



pub use eravon_cli::deskops::paypage::feeds_of;

pub fn price(c: &Chain, q: &k2::quote::Quote) -> R<()> {
    price_on(&c.view(), q).map_err(fault)
}

pub fn price_on(c: &eravon_cli::deskops::paypage::PayChain, q: &k2::quote::Quote) -> Result<(), eravon_cli::deskops::paypage::Fail> {
    eravon_cli::deskops::paypage::price(c, q)
}



pub fn consent(c: &Chain, p: &k3::listing::Listing) -> Vec<(usize, bool, String)> {
    eravon_cli::deskops::paypage::consent(&c.view(), p)
}

pub struct Balance {
    pub have: k2::u256::U256,
    pub decimals: Option<u64>,
}

pub fn balance(c: &Chain, token: &str) -> R<Balance> {
    let b = eravon_cli::deskops::paypage::balance(&c.view(), token).map_err(fault)?;
    Ok(Balance { have: b.have, decimals: b.decimals })
}

pub fn gap(amount: &k2::u256::U256, have: &k2::u256::U256) -> Option<k2::u256::U256> {
    eravon_cli::deskops::paypage::gap(amount, have)
}

