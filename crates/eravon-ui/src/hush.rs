use crate::toast::{Act, Tone, Toasts};

#[derive(Default)]
pub struct Hush {
    on: bool,
    held: Vec<(String, String, String, Tone, Vec<(String, Act)>)>,
}

impl Hush {
    pub fn new() -> Hush {
        Hush::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn say_keys(&mut self, t: &mut Toasts, text: impl Into<String>, next: &str, raw: &str, tone: Tone, now: f64, keys: Vec<(String, Act)>) {
        let text = text.into();
        if self.on {
            if !self.held.iter().any(|h| h.0 == text && h.1 == next) {
                self.held.push((text, next.to_string(), raw.to_string(), tone, keys));
            }
            return;
        }
        t.say_keys(text, next, raw, tone, now, keys);
    }

    pub fn say(&mut self, t: &mut Toasts, text: impl Into<String>, tone: Tone, now: f64) {
        self.say_keys(t, text, "", "", tone, now, Vec::new());
    }

    pub fn set(&mut self, t: &mut Toasts, on: bool, now: f64) {
        let was = self.on;
        self.on = on;
        if was && !on {
            for (text, next, raw, tone, keys) in std::mem::take(&mut self.held) {
                t.say_keys(text, &next, &raw, tone, now, keys);
            }
        }
    }

    pub fn held(&self) -> usize {
        self.held.len()
    }
}
