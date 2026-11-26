use super::{write_global, No};
use eravon_standard::chain::TokenProbe;
use eravon_store::settings::TokenRow;
use eravon_vault::tokens;

pub const R_FIXED: &str = "token-decimals-fixed";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Added {
    Row(TokenRow),
    Manual(String),
}

fn in_use(data: &str, token: &str) -> Result<Vec<(String, String)>, eravon_goods::goods::Fault> {
    eravon_goods::goods::using(data, token)
}


fn edit(data: &str, writer: &str, now: u64, change: impl FnOnce(&mut Vec<TokenRow>)) -> Result<(), No> {
    write_global(data, writer, now, |g| {
        let mut rows = g.tokens.clone().unwrap_or_else(tokens::factory);
        change(&mut rows);
        if g.tokens.is_some() || rows != tokens::factory() {
            g.tokens = Some(rows);
        }
    })
}

pub fn add(data: &str, writer: &str, now: u64, address: &str, label: &str, manual: Option<&str>, probe: impl FnOnce(&str) -> TokenProbe) -> Result<Added, No> {
    let chain = tokens::chain();
    let (address, label) = tokens::admit(&tokens::list(data), chain, address, label).map_err(|name| No::new(name, Vec::new()))?;
    let row = |decimals: Option<u8>, source: &str| TokenRow { chain, address: address.clone(), label: label.clone(), decimals, source: source.to_string() };
    let made = match manual {
        Some(typed) => row(Some(tokens::manual_decimals(typed).map_err(|name| No::new(name, Vec::new()))?), tokens::FROM_MANUAL),
        None => match probe(&address) {
            TokenProbe::NoCode => return Err(No::new(tokens::R_NO_CODE, vec![address])),
            TokenProbe::NoDecimals => return Ok(Added::Manual(address)),
            TokenProbe::Decimals { decimals, single } => row(Some(decimals), if single { tokens::FROM_CHAIN_SINGLE } else { tokens::FROM_CHAIN }),
            TokenProbe::Unreachable(_) => row(None, tokens::FROM_PENDING),
        },
    };
    let mut repeated = false;
    let at = made.address.clone();
    let kept = made.clone();
    edit(data, writer, now, |rows| {
        repeated = rows.iter().any(|r| r.chain == chain && r.address.eq_ignore_ascii_case(&at));
        if !repeated {
            rows.push(kept);
        }
    })?;
    if repeated {
        return Err(No::new(tokens::R_REPEATED, vec![made.address]));
    }
    Ok(Added::Row(made))
}

pub fn remove(data: &str, writer: &str, now: u64, address: &str) -> Result<TokenRow, No> {
    let chain = tokens::chain();
    let row = tokens::find(&tokens::list(data), chain, address).cloned().ok_or_else(|| No::new(tokens::R_UNKNOWN, vec![address.to_string()]))?;
    let used = in_use(data, &row.address).map_err(|e| No::new(e.name(), vec![e.detail()]))?;
    if !used.is_empty() {
        let mut facts = vec![row.label.clone()];
        facts.extend(used.into_iter().map(|(_, n)| n));
        return Err(No::new(tokens::R_IN_USE, facts));
    }
    edit(data, writer, now, |rows| rows.retain(|r| !(r.chain == chain && r.address.eq_ignore_ascii_case(&row.address))))?;
    Ok(row)
}

pub fn decimals(data: &str, writer: &str, now: u64, address: &str, typed: &str) -> Result<u8, No> {
    let chain = tokens::chain();
    let row = tokens::find(&tokens::list(data), chain, address.trim()).cloned().ok_or_else(|| No::new(tokens::R_UNKNOWN, vec![address.to_string()]))?;
    if !matches!(row.source.as_str(), tokens::FROM_MANUAL | tokens::FROM_PENDING) {
        return Err(No::new(R_FIXED, vec![row.address]));
    }
    let d = tokens::manual_decimals(typed).map_err(|name| No::new(name, Vec::new()))?;
    let mut refused: Option<&'static str> = None;
    edit(data, writer, now, |rows| match rows.iter_mut().find(|r| r.chain == chain && r.address.eq_ignore_ascii_case(&row.address)) {
        None => refused = Some(tokens::R_UNKNOWN),
        Some(r) if !matches!(r.source.as_str(), tokens::FROM_MANUAL | tokens::FROM_PENDING) => refused = Some(R_FIXED),
        Some(r) => {
            r.decimals = Some(d);
            r.source = tokens::FROM_MANUAL.to_string();
        }
    })?;
    match refused {
        Some(name) => Err(No::new(name, vec![row.address])),
        None => Ok(d),
    }
}
