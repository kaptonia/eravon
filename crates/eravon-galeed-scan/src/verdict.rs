use galeed::audit::{Verdict, VERDICT_COUNTED, VERDICT_UNPROVEN, VERDICT_VOID};

pub const CODE_METHOD: &str = "eth_getCode";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    Codeless,
    HasCode,
    Unconsulted,
}

pub fn boundary_of(code: Option<&[u8]>) -> Boundary {
    match code {
        Some(b) if b.is_empty() => Boundary::Codeless,
        Some(_) => Boundary::HasCode,
        None => Boundary::Unconsulted,
    }
}

pub fn block_tag(n: u64) -> String {
    format!("0x{n:x}")
}

pub fn decide(at: &mut dyn FnMut(u64) -> Boundary, block: u64) -> Verdict {
    let before = if block == 0 { Boundary::Codeless } else { at(block - 1) };
    let after = at(block);
    match (before, after) {
        (Boundary::Unconsulted, _) | (_, Boundary::Unconsulted) => Verdict::Unproven,
        (Boundary::HasCode, _) | (_, Boundary::HasCode) => Verdict::Void,
        _ => Verdict::Counted,
    }
}


pub fn name(v: &Verdict) -> &'static str {
    match v {
        Verdict::Counted => VERDICT_COUNTED,
        Verdict::Unproven => VERDICT_UNPROVEN,
        Verdict::Void => VERDICT_VOID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_boundaries_decide_counted_void_and_unproven() {
        let run = |before: Boundary, after: Boundary, block: u64| {
            let mut asked = Vec::new();
            let v = decide(&mut |b| {
                asked.push(b);
                if b + 1 == block { before } else { after }
            }, block);
            (v, asked)
        };
        assert_eq!(run(Boundary::Codeless, Boundary::Codeless, 9).0, Verdict::Counted);
        assert_eq!(run(Boundary::HasCode, Boundary::Codeless, 9).0, Verdict::Void);
        assert_eq!(run(Boundary::Codeless, Boundary::HasCode, 9).0, Verdict::Void);
        assert_eq!(run(Boundary::Unconsulted, Boundary::HasCode, 9).0, Verdict::Unproven);
        assert_eq!(run(Boundary::Codeless, Boundary::Unconsulted, 9).0, Verdict::Unproven);
        let (v, asked) = run(Boundary::HasCode, Boundary::Codeless, 0);
        assert_eq!((v, asked), (Verdict::Counted, vec![0]), "block 0: only its own end is asked");
        assert_eq!(boundary_of(Some(&[0x00])), Boundary::HasCode, "one byte of code is code");
    }
}
