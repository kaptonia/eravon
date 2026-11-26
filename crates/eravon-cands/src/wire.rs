use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;

use crate::fault::Absent;

#[derive(Clone)]
pub struct Wire {
    pub t: Auto,
    pub table: Table,
    pub registries: Vec<String>,
    pub now: u64,
}

pub fn registries(injected: &[String]) -> Result<Vec<String>, Absent> {
    if !injected.is_empty() {
        return Ok(injected.iter().map(|s| s.to_ascii_lowercase()).collect());
    }
    match eravon_literals::constants::judge_registries().get() {
        Some(list) if !list.is_empty() => Ok(list.iter().map(|r| r.address.to_ascii_lowercase()).collect()),
        _ => Err(Absent::NoRegistry),
    }
}

impl Wire {
    pub fn on(route: &Result<(Auto, Table), (String, String)>, injected: &[String], now: u64) -> Result<Wire, Absent> {
        let registries = registries(injected)?;
        let (t, table) = route.clone().map_err(|(n, d)| Absent::Unreachable(format!("{n}:{d}")))?;
        Ok(Wire { t, table, registries, now })
    }

    pub fn of(data: &str, addr: &str, injected: &[String], now: u64) -> Result<Wire, Absent> {
        let registries = registries(injected)?;
        let nodes = match eravon_vault::store::endpoints(data, addr) {
            Ok(t) => t,
            Err(e) => return Err(Absent::Unreachable(format!("无法读取身份设置:{}", e.name()))),
        };
        if nodes.is_empty() {
            return Err(Absent::Unreachable("端点表还没配".into()));
        }
        let table = Table::new(nodes).map_err(|e| Absent::Unreachable(e.name().to_string()))?;
        let t = Auto::web_roots().map_err(|e| Absent::Unreachable(e.name().to_string()))?;
        Ok(Wire { t, table, registries, now })
    }
}
