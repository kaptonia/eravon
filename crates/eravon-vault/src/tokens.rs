use eravon_literals::constants::{current_network, NETWORKS};
use eravon_literals::units;

pub use eravon_literals::units::{EN, Form, Words, ZH, amount, figure, short};

use eravon_store::settings::{self, TokenRow};
use crate::store as settings_io;

pub const FROM_FACTORY: &str = "factory";
pub const FROM_CHAIN: &str = "chain";
pub const FROM_CHAIN_SINGLE: &str = "chain-single";
pub const FROM_MANUAL: &str = "manual";
pub const FROM_PENDING: &str = "pending";

pub const R_BAD_ADDRESS: &str = "token-bad-address";
pub const R_LABEL_EMPTY: &str = "token-label-empty";
pub const R_REPEATED: &str = "token-repeated";
pub const R_UNKNOWN: &str = "token-unknown";
pub const R_IN_USE: &str = "token-in-use";
pub const R_NO_CODE: &str = "token-no-code";
pub const R_DECIMALS_BAD: &str = "token-decimals-bad";

pub fn chain() -> u64 {
    current_network().standard.chain_id
}

pub fn factory() -> Vec<TokenRow> {
    NETWORKS
        .iter()
        .flat_map(|n| {
            n.tokens.iter().map(move |t| TokenRow {
                chain: n.standard.chain_id,
                address: t.address.to_ascii_lowercase(),
                label: t.symbol.to_string(),
                decimals: Some(t.decimals),
                source: FROM_FACTORY.to_string(),
            })
        })
        .collect()
}

pub fn list(data: &str) -> Vec<TokenRow> {
    let rows = shared(data).as_ref().clone();
    kept_back(data);
    rows
}

fn kept_back(_data: &str) {}


type Kept = Vec<(String, Option<(u64, std::time::SystemTime)>, std::time::Instant, std::sync::Arc<Vec<TokenRow>>)>;
static KEPT: std::sync::Mutex<Kept> = std::sync::Mutex::new(Vec::new());

const RESTAT: std::time::Duration = std::time::Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Watch {
    Global,
}

impl crate::reads::Watched for Watch {
    const ALL: &'static [Watch] = &[Watch::Global];
    fn path(self, machine: &std::path::Path) -> std::path::PathBuf {
        match self {
            Watch::Global => machine.join(crate::records::GLOBAL_FILE),
        }
    }
}

fn stamp_now() -> Option<(u64, std::time::SystemTime)> {
    let m = crate::home::machine_dir().ok()?;
    crate::reads::look::<Watch>(&m).into_iter().find_map(|(_, s)| s).and_then(|s| Some((s.len, s.modified?)))
}

fn kept(data: &str) -> Option<std::sync::Arc<Vec<TokenRow>>> {
    KEPT.lock().ok()?.iter().find(|(d, _, _, _)| d == data).map(|(_, _, _, rows)| rows.clone())
}

fn keep(data: &str, stamp: Option<(u64, std::time::SystemTime)>, rows: std::sync::Arc<Vec<TokenRow>>) {
    if let Ok(mut k) = KEPT.lock() {
        k.retain(|(d, _, _, _)| d != data);
        k.push((data.to_string(), stamp, std::time::Instant::now(), rows));
    }
}

fn rows_of(g: settings::Global) -> std::sync::Arc<Vec<TokenRow>> {
    std::sync::Arc::new(match g.tokens.clone() {
        Some(rows) => with_unseen_networks(rows.into_iter().filter_map(bounded).collect()),
        None => factory(),
    })
}

pub fn shared(data: &str) -> std::sync::Arc<Vec<TokenRow>> {
    if let Ok(mut k) = KEPT.lock()
        && let Some(e) = k.iter_mut().find(|(d, _, _, _)| d == data)
    {
        if e.2.elapsed() < RESTAT {
            return e.3.clone();
        }
        let stamp = stamp_now();
        if stamp.is_some() && stamp == e.1 {
            e.2 = std::time::Instant::now();
            return e.3.clone();
        }
    }
    read_now(data).unwrap_or_else(|| std::sync::Arc::new(factory()))
}

fn read_now(data: &str) -> Option<std::sync::Arc<Vec<TokenRow>>> {
    let stamp = stamp_now();
    let g = settings_io::read_global(data).ok()?;
    let rows = rows_of(g);
    keep(data, stamp, rows.clone());
    Some(rows)
}

pub fn look() -> bool {
    let data = ADOPTED.lock().map(|a| a.clone()).unwrap_or_default();
    if data.is_empty() {
        return false;
    }
    let before = KEPT.lock().ok().and_then(|k| k.iter().find(|(d, _, _, _)| *d == data).map(|(_, s, _, r)| (*s, r.clone())));
    if let Some((stamp, _)) = &before
        && stamp.is_some()
        && *stamp == stamp_now()
    {
        return false;
    }
    match (read_now(&data), before) {
        (Some(now), Some((_, was))) => now != was,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

fn with_unseen_networks(mut rows: Vec<TokenRow>) -> Vec<TokenRow> {
    let seen: Vec<u64> = rows.iter().map(|r| r.chain).collect();
    rows.extend(factory().into_iter().filter(|r| !seen.contains(&r.chain)));
    rows
}

pub fn forget(data: &str) {
    if let Ok(mut k) = KEPT.lock() {
        k.retain(|(d, _, _, _)| d != data);
    }
}

pub fn wrote(data: &str, g: &settings::Global) {
    keep(data, stamp_now(), rows_of(g.clone()));
}

fn bounded(mut r: TokenRow) -> Option<TokenRow> {
    let a = r.address.trim().to_ascii_lowercase();
    if a.len() != 42 || !a.starts_with("0x") || !a[2..].bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    r.address = a;
    let known = [FROM_FACTORY, FROM_CHAIN, FROM_CHAIN_SINGLE, FROM_MANUAL, FROM_PENDING].contains(&r.source.as_str());
    if !known || r.decimals.is_some_and(|d| d > units::DECIMALS_MAX) {
        r.decimals = None;
        if !known {
            r.source = FROM_PENDING.to_string();
        }
    }
    Some(r)
}

static ADOPTED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn adopt(data: &str) {
    if let Ok(mut a) = ADOPTED.lock() {
        *a = data.to_string();
    }
    let _ = look();
}

pub fn current() -> Vec<TokenRow> {
    current_shared().as_ref().clone()
}

pub fn current_shared() -> std::sync::Arc<Vec<TokenRow>> {
    let data = ADOPTED.lock().map(|a| a.clone()).unwrap_or_default();
    kept(&data).unwrap_or_else(|| std::sync::Arc::new(factory()))
}

pub fn on_chain(rows: &[TokenRow], chain: u64) -> Vec<TokenRow> {
    rows.iter().filter(|t| t.chain == chain).cloned().collect()
}

pub fn find<'a>(rows: &'a [TokenRow], chain: u64, address: &str) -> Option<&'a TokenRow> {
    rows.iter().find(|t| t.chain == chain && t.address.eq_ignore_ascii_case(address.trim()))
}

pub fn builtin(row: &TokenRow) -> bool {
    factory().iter().any(|f| f.chain == row.chain && f.address.eq_ignore_ascii_case(&row.address))
}

pub fn label_of(rows: &[TokenRow], chain: u64, address: &str) -> Option<String> {
    find(rows, chain, address).map(|t| t.label.clone())
}

pub fn decimals_of(rows: &[TokenRow], chain: u64, address: &str, recorded: Option<u64>) -> Option<u8> {
    find(rows, chain, address).and_then(|t| t.decimals).or_else(|| recorded.filter(|d| *d <= u64::from(units::DECIMALS_MAX)).map(|d| d as u8))
}

pub fn name(rows: &[TokenRow], chain: u64, address: &str, w: &Words) -> String {
    label_of(rows, chain, address).unwrap_or_else(|| format!("{} {}", w.unlisted, short(address)))
}

pub fn admit(rows: &[TokenRow], chain: u64, address: &str, label: &str) -> Result<(String, String), &'static str> {
    let address = address.trim().to_ascii_lowercase();
    if !eravon_store::seam::address_ok(&address) {
        return Err(R_BAD_ADDRESS);
    }
    let label = label.trim();
    if label.is_empty() {
        return Err(R_LABEL_EMPTY);
    }
    if find(rows, chain, &address).is_some() {
        return Err(R_REPEATED);
    }
    Ok((address, label.to_string()))
}

pub fn manual_decimals(text: &str) -> Result<u8, &'static str> {
    text.trim().parse::<u8>().ok().filter(|d| *d <= units::DECIMALS_MAX).ok_or(R_DECIMALS_BAD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_row_is_bounded_like_a_chain_answer() {
        let row = |a: &str, d: Option<u8>, s: &str| TokenRow { chain: 1, address: a.into(), label: "X".into(), decimals: d, source: s.into() };
        let a = "0x00000000000000000000000000000000000000AA";
        let ok = bounded(row(a, Some(6), FROM_CHAIN)).unwrap();
        assert_eq!((ok.address.as_str(), ok.decimals, ok.source.as_str()), ("0x00000000000000000000000000000000000000aa", Some(6), FROM_CHAIN));
        let far = bounded(row(a, Some(200), FROM_MANUAL)).unwrap();
        assert_eq!((far.decimals, far.source.as_str()), (None, FROM_MANUAL));
        let odd = bounded(row(a, Some(6), "guessed")).unwrap();
        assert_eq!((odd.decimals, odd.source.as_str()), (None, FROM_PENDING));
        assert!(bounded(row("0x1234", Some(6), FROM_CHAIN)).is_none());
        assert!(bounded(row("0xZZ000000000000000000000000000000000000aa", Some(6), FROM_CHAIN)).is_none());
    }

    #[test]
    fn the_factory_table_is_every_networks_rows_with_their_decimals() {
        let f = factory();
        let want: Vec<(u64, &eravon_literals::constants::Token)> = NETWORKS.iter().flat_map(|n| n.tokens.iter().map(move |t| (n.standard.chain_id, t))).collect();
        assert_eq!(f.len(), want.len());
        for (r, (c, t)) in f.iter().zip(want) {
            assert_eq!((r.label.as_str(), r.address.as_str(), r.decimals, r.source.as_str(), r.chain), (t.symbol, t.address.to_ascii_lowercase().as_str(), Some(t.decimals), FROM_FACTORY, c));
            assert!(builtin(r));
        }
        assert_eq!(on_chain(&f, chain()).len(), current_network().tokens.len(), "选币只列当前网那几行");
    }

    #[test]
    fn a_table_naming_one_network_reads_the_others_from_the_factory() {
        let mine = TokenRow { chain: 1, address: format!("0x{}", "ab".repeat(20)), label: "MINE".into(), decimals: Some(6), source: FROM_MANUAL.into() };
        let rows = with_unseen_networks(vec![mine.clone()]);
        assert_eq!(on_chain(&rows, 1), vec![mine]);
        for n in NETWORKS.iter().filter(|n| n.standard.chain_id != 1) {
            assert_eq!(on_chain(&rows, n.standard.chain_id), on_chain(&factory(), n.standard.chain_id));
        }
    }

    #[test]
    fn names_and_amounts_come_from_the_registry_or_say_unlisted() {
        let rows = factory();
        let usdc = on_chain(&rows, chain()).into_iter().find(|r| r.label == "USDC").expect("每一网都有 USDC").address;
        assert_eq!(name(&rows, chain(), &usdc.to_ascii_uppercase().replacen("0X", "0x", 1), &ZH), "USDC");
        let other = format!("0x{}", "4e".repeat(20));
        assert_eq!(name(&rows, chain(), &other, &ZH), "未登载 0x4e4e…4e4e");
        assert_eq!(name(&rows, chain() + 1, &usdc, &EN), format!("Not listed {}", short(&usdc)));
        assert_eq!(decimals_of(&rows, chain(), &usdc, Some(18)), Some(6), "登载了的取登载表");
        assert_eq!(decimals_of(&rows, chain(), &other, Some(18)), Some(18), "没登载的取记录自带");
        assert_eq!(decimals_of(&rows, chain(), &other, Some(78)), None, "记录里出界的不收");
        assert_eq!(amount("1234560000", Some(6), "USDC", Form::Full, &ZH), "1,234.56 USDC");
        assert_eq!(amount("123456789012345678", Some(18), "WETH", Form::Short, &ZH), "0.123456… WETH");
        assert_eq!(amount("1234560000", Some(6), "USDC", Form::Plain, &ZH), "1234.56 USDC");
        assert_eq!(amount("300", None, &name(&rows, chain(), &other, &ZH), Form::Short, &ZH), "300 最小单位 · 未登载 0x4e4e…4e4e");
        assert_eq!(amount("300", None, "X", Form::Full, &EN), "300 smallest units · X");
    }

    #[test]
    fn a_row_to_add_is_admitted_by_name() {
        let rows = factory();
        let usdc = on_chain(&rows, chain()).into_iter().find(|r| r.label == "USDC").unwrap().address;
        assert_eq!(admit(&rows, chain(), "0x12", "A"), Err(R_BAD_ADDRESS));
        assert_eq!(admit(&rows, chain(), &format!("0x{}", "ab".repeat(20)), "  "), Err(R_LABEL_EMPTY));
        assert_eq!(admit(&rows, chain(), &usdc.to_ascii_uppercase().replacen("0X", "0x", 1), "B"), Err(R_REPEATED));
        assert_eq!(admit(&rows, chain(), &format!("0x{}", "AB".repeat(20)), " C "), Ok((format!("0x{}", "ab".repeat(20)), "C".to_string())));
        assert_eq!(manual_decimals("78"), Err(R_DECIMALS_BAD));
        assert_eq!(manual_decimals(" 77 "), Ok(77));
    }
}
