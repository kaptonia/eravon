pub use eravon_os::secret::*;

pub struct Field<'a>(pub &'a mut Secret);

impl egui::TextBuffer for Field<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.0.expose()
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        self.0.insert_chars(text, char_index)
    }

    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {
        self.0.remove_chars(char_range);
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn replace_with(&mut self, text: &str) {
        self.0.clear();
        self.0.push_str(text);
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Secret>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_through_the_field_are_the_secrets_own() {
        let mut s = Secret::of("K7m2");
        let mut f = Field(&mut s);
        egui::TextBuffer::insert_text(&mut f, "xy", 2);
        egui::TextBuffer::delete_char_range(&mut f, 0..1);
        assert_eq!(s.expose(), "7xym2");
        let mut full = Secret::of(&"a".repeat(CAP));
        assert_eq!(egui::TextBuffer::insert_text(&mut Field(&mut full), "b", 0), 0);
    }
}
