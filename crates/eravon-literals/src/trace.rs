use crate::json::{Out, write};

pub const V_TRACE_DOC: &str = "eravon.app.trace/1";

pub const ENV_TRACE: &str = "ERAVON_TRACE";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    A1,
    A2,
    A3,
    A4,
    A5,
    F2,
    H0,
    H1,
    H2,
    H3,
    H4,
    H5,
    K1,
    K2,
    K3,
    K4,
    K5,
    K6,
    K7,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    R1,
    R2,
    S1,
    S10,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    V1,
    V2,
    V3,
    W1,
    W2,
    W3,
    W4,
    W5,
    W6,
    W7,
    W8,
    W9,
}

impl Feature {
    pub const ALL: [Feature; 51] = [Feature::A1, Feature::A2, Feature::A3, Feature::A4, Feature::A5, Feature::F2, Feature::H0, Feature::H1, Feature::H2, Feature::H3, Feature::H4, Feature::H5, Feature::K1, Feature::K2, Feature::K3, Feature::K4, Feature::K5, Feature::K6, Feature::K7, Feature::P1, Feature::P2, Feature::P3, Feature::P4, Feature::P5, Feature::P6, Feature::P7, Feature::P8, Feature::R1, Feature::R2, Feature::S1, Feature::S10, Feature::S2, Feature::S3, Feature::S4, Feature::S5, Feature::S6, Feature::S7, Feature::S8, Feature::S9, Feature::V1, Feature::V2, Feature::V3, Feature::W1, Feature::W2, Feature::W3, Feature::W4, Feature::W5, Feature::W6, Feature::W7, Feature::W8, Feature::W9];

    pub const fn as_str(self) -> &'static str {
        match self {
            Feature::A1 => "A1",
            Feature::A2 => "A2",
            Feature::A3 => "A3",
            Feature::A4 => "A4",
            Feature::A5 => "A5",
            Feature::F2 => "F2",
            Feature::H0 => "H0",
            Feature::H1 => "H1",
            Feature::H2 => "H2",
            Feature::H3 => "H3",
            Feature::H4 => "H4",
            Feature::H5 => "H5",
            Feature::K1 => "K1",
            Feature::K2 => "K2",
            Feature::K3 => "K3",
            Feature::K4 => "K4",
            Feature::K5 => "K5",
            Feature::K6 => "K6",
            Feature::K7 => "K7",
            Feature::P1 => "P1",
            Feature::P2 => "P2",
            Feature::P3 => "P3",
            Feature::P4 => "P4",
            Feature::P5 => "P5",
            Feature::P6 => "P6",
            Feature::P7 => "P7",
            Feature::P8 => "P8",
            Feature::R1 => "R1",
            Feature::R2 => "R2",
            Feature::S1 => "S1",
            Feature::S10 => "S10",
            Feature::S2 => "S2",
            Feature::S3 => "S3",
            Feature::S4 => "S4",
            Feature::S5 => "S5",
            Feature::S6 => "S6",
            Feature::S7 => "S7",
            Feature::S8 => "S8",
            Feature::S9 => "S9",
            Feature::V1 => "V1",
            Feature::V2 => "V2",
            Feature::V3 => "V3",
            Feature::W1 => "W1",
            Feature::W2 => "W2",
            Feature::W3 => "W3",
            Feature::W4 => "W4",
            Feature::W5 => "W5",
            Feature::W6 => "W6",
            Feature::W7 => "W7",
            Feature::W8 => "W8",
            Feature::W9 => "W9",
        }
    }

    pub fn of(code: &str) -> Option<Feature> {
        Feature::ALL.into_iter().find(|f| f.as_str() == code)
    }
}

pub const CAP_ENV: &str = "ERAVON_TRACE_CAP_BYTES";

pub const CAP_BYTES: usize = digits(env!("ERAVON_TRACE_CAP_BYTES"));

const fn digits(s: &str) -> usize {
    let b = s.as_bytes();
    let mut i = 0;
    let mut n: usize = 0;
    assert!(!b.is_empty(), "ERAVON_TRACE_CAP_BYTES 是空的");
    while i < b.len() {
        assert!(b[i].is_ascii_digit(), "ERAVON_TRACE_CAP_BYTES 不是十进制数");
        n = n * 10 + (b[i] - b'0') as usize;
        i += 1;
    }
    n
}

pub const WHAT_CAP: &str = "trace-cap";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub n: u64,
    pub what: String,
    pub mark: String,
    pub say: String,
}

pub fn doc<'a>(steps: impl IntoIterator<Item = (u64, &'a str, &'a str, &'a str)>) -> Vec<u8> {
    let steps: Vec<Out> = steps
        .into_iter()
        .map(|(n, what, mark, say)| {
            Out::O(vec![
                ("mark", Out::S(mark.to_string())),
                ("n", Out::N(n)),
                ("say", Out::S(say.to_string())),
                ("what", Out::S(what.to_string())),
            ])
        })
        .collect();
    write(&Out::O(vec![("steps", Out::A(steps)), ("v", Out::S(V_TRACE_DOC.to_string()))]))
}

pub fn mark(feature: Feature) {
    eravon::trace::mark(feature.as_str());
}

pub fn mark_code(code: &str) {
    if let Some(f) = Feature::of(code) {
        mark(f);
    }
}

pub fn entry(bin: &str, feature: Feature, verb: &str) {
    let _ = (bin, verb);
    mark(feature);
}

pub fn fit(prior: Vec<Step>, more: Vec<(String, String, String)>, cap: usize) -> (Vec<u8>, bool) {
    let lay = |steps: &[Step]| doc(steps.iter().map(|t| (t.n, t.what.as_str(), t.mark.as_str(), t.say.as_str())));
    let mut steps = prior;
    let base = steps.len();
    for (what, mark, say) in more.iter().cloned() {
        let n = steps.last().map_or(1, |s| s.n + 1);
        steps.push(Step { n, what, mark, say });
    }
    let all = lay(&steps);
    if all.len() <= cap {
        return (all, false);
    }
    let stop = |kept: &mut Vec<Step>| {
        let n = kept.last().map_or(1, |s| s.n + 1);
        kept.push(Step { n, what: WHAT_CAP.to_string(), mark: Feature::K7.as_str().to_string(), say: format!("{CAP_ENV} {cap}") });
    };
    let _ = base;
    let mut keep = steps.len();
    while keep > 0 {
        keep -= 1;
        let mut kept = steps[..keep].to_vec();
        stop(&mut kept);
        let b = lay(&kept);
        if b.len() <= cap {
            return (b, true);
        }
    }
    let mut only = Vec::new();
    stop(&mut only);
    (lay(&only), true)
}

pub fn lands() -> bool {
    from_env().is_some()
}

fn from_env() -> Option<String> {
    std::env::var(ENV_TRACE).ok().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{read, In};

    fn steps(n: usize) -> Vec<(String, String, String)> {
        (0..n).map(|i| (format!("step{i}"), Feature::H1.as_str().to_string(), String::new())).collect()
    }

    #[test]
    fn a_trace_at_its_cap_is_whole_and_one_byte_over_stops_by_name() {
        let (whole, stopped) = fit(Vec::new(), steps(40), usize::MAX);
        assert!(!stopped);
        let n = whole.len();
        let (at, stopped) = fit(Vec::new(), steps(40), n);
        assert_eq!((at.len(), stopped), (n, false), "恰 N 字节整份留下");
        let (over, stopped) = fit(Vec::new(), steps(40), n - 1);
        assert!(stopped && over.len() <= n - 1, "N-1 即停记");
        let v = read(&over).unwrap();
        let last = v.get("steps").and_then(In::as_arr).and_then(|a| a.last().cloned()).unwrap();
        assert_eq!(last.get("what").and_then(In::as_str), Some(WHAT_CAP));
        assert_eq!(last.get("say").and_then(In::as_str), Some(format!("{CAP_ENV} {}", n - 1).as_str()));
    }

    #[test]
    fn the_cap_step_lands_even_when_the_prior_steps_fill_the_file() {
        let prior: Vec<Step> = steps(40).into_iter().enumerate().map(|(i, (what, mark, say))| Step { n: i as u64 + 1, what, mark, say }).collect();
        let (whole, _) = fit(prior.clone(), Vec::new(), usize::MAX);
        let cap = whole.len() - 1;
        let (b, stopped) = fit(prior, steps(3), cap);
        assert!(stopped && b.len() <= cap, "{} > {cap}", b.len());
        let v = read(&b).unwrap();
        let last = v.get("steps").and_then(In::as_arr).and_then(|a| a.last().cloned()).unwrap();
        assert_eq!(last.get("what").and_then(In::as_str), Some(WHAT_CAP));
    }

    #[test]
    fn a_mark_is_one_of_the_features() {
        for f in Feature::ALL {
            assert_eq!(Feature::of(f.as_str()), Some(f));
        }
        assert_eq!(Feature::of("free text"), None);
        assert!(CAP_BYTES > 0);
    }
}
