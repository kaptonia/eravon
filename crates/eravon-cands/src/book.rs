use eravon_chain::endpoints::Table;
use eravon_chain::seam::Upto;
use eravon_chain::transport::Transport;
use eravon_literals::constants as k7;
use eravon_roster::book::{Engaged, PanelBook, PanelRecord};
use eravon_scan::log::Log;
use eravon_standard::order::hex_of;

pub struct ChainBook<'a, T: Transport> {
    pub t: &'a T,
    pub table: &'a Table,
    pub logic: &'a str,
}

impl<T: Transport> ChainBook<'_, T> {
    fn book(&self) -> Result<Option<&'static str>, String> {
        let deployed = eravon_standard::chain::has_code(self.t, self.table, k7::PANEL).map_err(|e| format!("{}:{}", e.name(), e.detail()))?;
        Ok(deployed.then_some(k7::PANEL))
    }
}

fn b32(h: &str) -> Result<[u8; 32], String> {
    eravon_standard::order::b32_of("键", h).map_err(|e| format!("{}:{}", e.name(), e.detail()))
}

fn form<V>(what: &str, v: Option<V>) -> Result<V, String> {
    v.ok_or_else(|| format!("名单簿的答缺 {what} 或读不成"))
}

fn named(e: eravon_standard::fault::Fault) -> String {
    format!("{}:{}", e.name(), e.detail())
}

impl<T: Transport> PanelBook for ChainBook<'_, T> {
    fn root_of(&self, guild: &str) -> Result<String, String> {
        Ok(eravon_standard::chain::root(self.t, self.table, guild).map_err(named)?.value)
    }

    fn count(&self, root: &str, terms_hash: &str) -> Result<u64, String> {
        let Some(book) = self.book()? else { return Ok(0) };
        Ok(eravon_standard::chain::book_count(self.t, self.table, book, root, &b32(terms_hash)?).map_err(named)?.value)
    }

    fn panel_at(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<PanelRecord>, String> {
        let Some(book) = self.book()? else { return Ok(None) };
        let v = match eravon_standard::chain::book_panel_at(self.t, self.table, book, root, &b32(terms_hash)?, index) {
            Ok(r) => r.value,
            Err(e) if eravon_standard::chain::reverted(&e) => return Ok(None),
            Err(e) => return Err(named(e)),
        };
        use eravon_standard::chain::panel_col as k;
        Ok(Some(PanelRecord {
            index,
            panel_hash: hex_of(&form("panelHash", v.get(k::PANEL_HASH).and_then(|x| x.word().ok()))?),
            roster_hash: hex_of(&form("rosterHash", v.get(k::ROSTER_HASH).and_then(|x| x.word().ok()))?),
            expires: form("expires", v.get(k::EXPIRES).and_then(|x| x.small().ok()))?,
            recorded: form("recorded", v.get(k::RECORDED).and_then(|x| x.small().ok()))?,
            router: hex_of(&form("router", v.get(k::ROUTER).and_then(|x| x.addr().ok()))?),
            router_hash: hex_of(&form("routerHash", v.get(k::ROUTER_HASH).and_then(|x| x.word().ok()))?),
        }))
    }

    fn engaged(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<Engaged>, String> {
        let Some(book) = self.book()? else { return Ok(None) };
        let topic0 = hex_of(&eravon_standard::sol::BOOK_ENGAGED.topic0());
        let g = eravon_scan::log::address_topic(root).map_err(|e| format!("{}:{}", e.name(), e.detail()))?;
        let (rows, _) = eravon_chain::read::logs_any(self.t, self.table, Some(book), &[vec![topic0], vec![g], vec![terms_hash.to_string()]], 0, Upto::Latest)
            .map_err(|e| format!("{}:{}", e.name(), e.detail()))?;
        for row in &rows {
            let l = Log::read(row).map_err(|e| format!("{}:{}", e.name(), e.detail()))?;
            let v = eravon_standard::event::decode(&l.topics, &l.data).map_err(named)?;
            if form("index", v.at(2).ok().and_then(|x| x.small().ok()))? != index {
                continue;
            }
            let members = form("members", v.at(4).ok().and_then(|x| x.items().ok()).and_then(|xs| xs.iter().map(|x| x.addr().ok().map(|a| hex_of(&a))).collect::<Option<Vec<_>>>()))?;
            let hashes = form("amanahHashes", v.at(5).ok().and_then(|x| x.items().ok()).and_then(|xs| xs.iter().map(|x| x.word().ok().map(|a| hex_of(&a))).collect::<Option<Vec<_>>>()))?;
            let roster_hash = hex_of(&form("rosterHash", v.at(6).ok().and_then(|x| x.word().ok()))?);
            let expires = form("expires", v.at(7).ok().and_then(|x| x.small().ok()))?;
            let router = hex_of(&form("router", v.at(8).ok().and_then(|x| x.addr().ok()))?);
            return Ok(Some(Engaged { index, members, amanah_hashes: hashes, roster_hash, expires, router }));
        }
        Ok(None)
    }
}
