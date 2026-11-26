use super::*;

impl Settings {
    pub fn go_section(&mut self, to: Option<Section>) -> bool {
        if to.is_some_and(|x| !x.in_seat(self.seat)) {
            self.said = Some(bad("E_S9_SECTION", crate::house::i18n::t(crate::house::i18n::Key::SetNoSection)));
            return false;
        }
        self.section = to;
        self.token_sheet = None;
        true
    }
}
