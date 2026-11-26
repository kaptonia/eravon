use eravon_cands::book::ChainBook;
use eravon_cands::wire::Wire;
use eravon_roster::book::{Engaged, PanelBook, PanelRecord};

pub struct LiveBook<'a> {
    pub w: &'a Wire,
}

impl LiveBook<'_> {
    fn on(&self) -> ChainBook<'_, eravon_chain::transport::Auto> {
        ChainBook { t: &self.w.t, table: &self.w.table, logic: eravon_literals::constants::active_standard().logic }
    }
}

impl PanelBook for LiveBook<'_> {
    fn root_of(&self, guild: &str) -> Result<String, String> {
        self.on().root_of(guild)
    }
    fn count(&self, root: &str, terms_hash: &str) -> Result<u64, String> {
        self.on().count(root, terms_hash)
    }
    fn panel_at(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<PanelRecord>, String> {
        self.on().panel_at(root, terms_hash, index)
    }
    fn engaged(&self, root: &str, terms_hash: &str, index: u64) -> Result<Option<Engaged>, String> {
        self.on().engaged(root, terms_hash, index)
    }
}
