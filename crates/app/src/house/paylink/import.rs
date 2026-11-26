use super::*;

impl PayLink {
    pub fn import_plan(&self) -> Result<(), Said> {
        let Some((_, bytes)) = self.import.file.as_ref() else { return Err(Said::bad("E_APP_SCRIPT", crate::house::i18n::t(crate::house::i18n::Key::PlOrderFileFirst))) };
        k3::pack::pack(bytes).map_err(|e| Said::bad(e.token.name(), crate::house::i18n::t(crate::house::i18n::Key::PlNotOrderFile)))?;
        Ok(())
    }

    pub fn bump(&mut self, s: &eravon_vault::session::Session, buyer: &str, n: Option<usize>, now: u64) {
        self.key_for = Some((|| {
            let Some(Ok(o)) = self.verified() else { return Err(Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_UNOPENED)) };
            let n = n.or(self.pick).ok_or_else(|| Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_NO_PICK))?;
            if self.payout.is_empty() || self.payout_said.is_some() {
                return Err(Said::bad("E_APP_SCRIPT", crate::house::seam::PL_KEY_NO_PAYOUT));
            }
            let p = k3::listing::listing(&o.listing).map_err(|e| Said::bad(e.token.name(), crate::house::i18n::t(crate::house::i18n::Key::PlLinkBroken)))?;
            let open_by = eravon_cli::shared::open_by_at(p.order.expiry, p.order.open_window, now).map_err(|e| Said::bad(e.name(), e.detail()))?;
            let w = crate::house::orders::wire(s.data(), buyer)?;
            let prev = eravon_cli::buyer::series_prev(s.data(), buyer, buyer, &p, &eravon_cli::buyer::live_on(&w, now)).map_err(|e| Said::bad(e.name(), e.detail()))?;
            eravon_cli::buyer::key_pick(s, buyer, &w, &p, n, &self.payout, open_by, Some(now), &prev).map_err(|e| Said::bad(e.name(), e.detail()))
        })());
    }

    pub fn take_file(&mut self, name: &str, bytes: Vec<u8>) {
        self.tab = Tab::Import;
        self.import.file = Some((name.to_string(), bytes));
        self.import.said = None;
        self.import.done = None;
    }

    pub fn take_path(&mut self, path: &str) {
        match std::fs::read(path) {
            Ok(b) => {
                let name = std::path::Path::new(path).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                self.take_file(&name, b);
            }
            Err(e) => self.import.said = Some(Said::bad("E_APP_IO", crate::fill!(crate::house::i18n::Key::FirstPathUnreadable; path.to_string(), e.to_string()))),
        }
    }

    pub fn import_file(&mut self, s: &eravon_vault::session::Session, me: &str, now: u64, word: &str) {
        let Some((_, bytes)) = self.import.file.clone() else {
            self.import.said = Some(Said::bad("E_APP_SCRIPT", crate::house::i18n::t(crate::house::i18n::Key::PlOrderFileFirst)));
            return;
        };
        self.land(eravon_import::import::run(s, me, &bytes, crate::house::seam::WRITER, now), word);
    }

    pub(super) fn land(&mut self, r: eravon_import::fault::R<eravon_import::import::Done>, word: &str) {
        match r {
            Ok(d) => {
                let note = d.mode.note();
                self.import.said = Some(Said::ok(if note.is_empty() { format!("{word} · {}", d.label) } else { format!("{word} · {} · {note}", d.label) }));
                self.import.done = Some(d);
            }
            Err(e) => {
                self.import.done = None;
                self.import.said = Some(Said::bad(e.name(), e.detail()));
            }
        }
    }
}

