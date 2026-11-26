#![forbid(unsafe_code)]

pub mod act;
pub mod company;
pub mod order;
pub mod paste;
pub mod poll;
pub mod pool;
pub mod scan;
pub mod send;
pub mod units;

pub type Why = (String, String);

pub fn why(name: &str, detail: impl Into<String>) -> Why {
    (name.to_string(), detail.into())
}

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::S4;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

pub struct Wire<'a> {
    pub t: &'a eravon_chain::transport::Auto,
    pub table: &'a eravon_chain::endpoints::Table,
    pub chain_id: u64,
    pub logic: &'a str,
    pub registries: &'a [String],
}

impl Wire<'_> {
    pub fn held(&self) -> Held {
        Held { t: self.t.clone(), table: self.table.clone(), chain_id: self.chain_id, logic: self.logic.to_string(), registries: self.registries.to_vec() }
    }
}

#[derive(Clone)]
pub struct Held {
    pub t: eravon_chain::transport::Auto,
    pub table: eravon_chain::endpoints::Table,
    pub chain_id: u64,
    pub logic: String,
    pub registries: Vec<String>,
}

impl Held {
    pub fn wire(&self) -> Wire<'_> {
        Wire { t: &self.t, table: &self.table, chain_id: self.chain_id, logic: &self.logic, registries: &self.registries }
    }
}

pub const E_NO_ENDPOINT: &str = "E_WATCH_NO_ENDPOINT";

pub fn wire_parts(
    data: &str,
    me: &str,
) -> Result<
    (
        eravon_chain::transport::Auto,
        eravon_chain::endpoints::Table,
    ),
    Why,
> {
    let nodes =
        eravon_vault::store::endpoints(data, me).map_err(|e| why(e.name(), e.detail()))?;
    if nodes.is_empty() {
        return Err(why(E_NO_ENDPOINT, "未配置节点"));
    }
    let table = eravon_chain::endpoints::Table::new(nodes)
        .map_err(|e| why(e.name(), e.detail()))?;
    let t = eravon_chain::transport::Auto::web_roots().map_err(|e| why(e.name(), e.detail()))?;
    Ok((t, table))
}

pub fn standard() -> (u64, &'static str) {
    let (c, l, _) = eravon_standard::chain::standard_table();
    (c, l)
}
