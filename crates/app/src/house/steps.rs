use eravon_literals::trace;
use eravon_store::atomic;

use crate::house::seam;

pub const RING: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub n: u64,
    pub what: String,
    pub mark: trace::Feature,
    pub say: String,
}

#[derive(Clone, Debug, Default)]
pub struct Trace {
    pub steps: Vec<Step>,
    pub taken: u64,
}

fn on_disk(_say: &str) -> String {
    String::new()
}


fn laid(steps: &[Step]) -> Vec<u8> {
    let rows: Vec<trace::Step> = steps.iter().map(|s| trace::Step { n: s.n, what: s.what.clone(), mark: s.mark.as_str().to_string(), say: on_disk(&s.say) }).collect();
    trace::fit(rows, Vec::new(), trace::CAP_BYTES).0
}

impl Trace {
    pub fn push(&mut self, what: crate::house::action::Action, mark: trace::Feature, say: String) {
        self.taken += 1;
        if self.steps.len() == RING {
            self.steps.remove(0);
        }
        self.steps.push(Step { n: self.taken, what: what.name().to_string(), mark, say });
    }

    pub fn bytes(&self) -> Vec<u8> {
        laid(&self.steps)
    }

    pub fn save(&self, data: &str) -> Result<String, String> {
        self.write_to(data, &self.steps)
    }


    fn write_to(&self, data: &str, steps: &[Step]) -> Result<String, String> {
        if !trace::lands() {
            return Ok(String::new());
        }
        let bytes = laid(steps);
        let dir = seam::trace_dir(data);
        atomic::mkdirs(&dir).map_err(|e| e.detail())?;
        let path = seam::trace_path(data);
        atomic::write_atomic(&path, &bytes).map_err(|e| e.detail())?;
        Ok(path)
    }

    pub fn rows(&self) -> eravon_literals::json::Out {
        use eravon_literals::json::Out;
        Out::A(
            self.steps
                .iter()
                .map(|s| Out::O(vec![("mark", Out::S(s.mark.as_str().to_string())), ("n", Out::N(s.n)), ("say", Out::S(s.say.clone())), ("what", Out::S(s.what.clone()))]))
                .collect(),
        )
    }
}

pub fn load(data: &str) -> Result<Vec<u8>, String> {
    atomic::read_file(&seam::trace_path(data)).map_err(|e| e.detail())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_document_carries_its_form_and_every_step_in_order() {
        let mut t = Trace::default();
        t.push(crate::house::action::Action::Seat, trace::Feature::H1, "到卖家席 0x35DBdb98aa11bb22cc33dd44ee55ff6677889900".into());
        t.push(crate::house::action::Action::Mode, trace::Feature::H1, "到子页".into());
        let b = String::from_utf8(t.bytes()).expect("是 UTF-8");
        assert!(b.contains("\"v\":\"eravon.app.trace/1\""));
        assert!(b.contains("\"n\":1"));
        assert!(b.contains("\"n\":2"));
        assert!(!b.ends_with('\n'));
        assert!(!b.contains("卖家席") && !b.contains("35DBdb98"), "{b}");
        assert!(t.steps[0].say.contains("卖家席"));
    }

    #[test]
    fn past_the_ring_the_file_keeps_each_steps_own_number() {
        let mut t = Trace::default();
        for _ in 0..RING + 88 {
            t.push(crate::house::action::Action::Seat, trace::Feature::H1, String::new());
        }
        assert_eq!(t.taken, (RING + 88) as u64);
        let b = String::from_utf8(t.bytes()).expect("是 UTF-8");
        let first = b.find("\"n\":").map(|i| &b[i..i + 8]).unwrap_or_default().to_string();
        assert!(first.starts_with("\"n\":89,"), "{first}");
        assert!(b.contains(&format!("\"n\":{},", RING + 88)));
    }

    #[test]
    fn the_ring_keeps_the_last_steps() {
        let mut t = Trace::default();
        for _ in 0..RING + 3 {
            t.push(crate::house::action::Action::Seat, trace::Feature::H1, String::new());
        }
        assert_eq!(t.steps.len(), RING);
        assert_eq!((t.steps[0].n, t.steps[RING - 1].n), (4, (RING + 3) as u64));
    }
}
