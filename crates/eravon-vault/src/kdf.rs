#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    pub log_n: u8,
    pub r: u32,
    pub p: u32,
}

impl Params {
    fn of(k: crate::keystore::Params) -> Params {
        Params { log_n: k.n.trailing_zeros() as u8, r: k.r as u32, p: k.p as u32 }
    }

    fn keystore(self) -> crate::keystore::Params {
        crate::keystore::Params { n: 1usize << self.log_n, r: self.r as usize, p: self.p as usize }
    }
}

pub const STANDARD: Params = Params { log_n: 18, r: 8, p: 1 };

pub const LIGHT: Params = Params { log_n: 1, r: 8, p: 1 };

pub fn set_light(p: Option<Params>) -> bool {
    crate::keybox::set_light_kdf(p.map(Params::keystore))
}

pub fn params() -> Params {
    Params::of(crate::keybox::params())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_levels_are_the_parts_own() {
        assert_eq!(STANDARD.keystore(), crate::keystore::Params::standard());
        assert_eq!(LIGHT.keystore(), crate::keybox::PROBE_KDF);
        assert_eq!(Params::of(crate::keystore::Params::standard()), STANDARD);
    }

    #[test]
    fn the_floor_is_the_shipped_level_and_below_it_is_refused_by_name() {
        crate::glue::take_turn();
        set_light(None);
        let f = crate::keybox::floor();
        assert_eq!(f, crate::keybox::params());
        assert!(!crate::keybox::below_floor(f));
        assert!(!crate::keybox::below_floor(crate::keystore::Params { n: f.n * 2, ..f }), "高于下界照过");
        for low in [crate::keystore::Params { n: f.n / 2, ..f }, crate::keystore::Params { r: f.r / 2, ..f }, crate::keystore::Params { n: f.n / 2, r: f.r, p: f.p }] {
            assert!(crate::keybox::below_floor(low), "{low:?}");
        }
        let standard = crate::keystore::Params::standard();
        assert!(standard.n * standard.r >= f.n * f.r, "the light level is never above the shipped one");
    }

    #[test]
    fn the_same_passcode_and_salt_and_params_give_the_same_thirty_two_bytes() {
        let run = |salt: &[u8]| {
            let mut k = [0u8; 32];
            assert!(crate::cryptx::scrypt(b"48261537", salt, 1024, 8, 1, &mut k));
            k
        };
        assert_eq!(run(b"same-salt"), run(b"same-salt"));
        assert_ne!(run(b"same-salt"), run(b"other-salt"));
        let mut k = [0u8; 32];
        assert!(!crate::cryptx::scrypt(b"48261537", b"salt", 1000, 8, 1, &mut k), "n 不是二的幂即不派生");
    }

    #[test]
    fn only_test_code_sets_the_light_level() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut stack = vec![root];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if p.is_dir() {
                    if name != "target" {
                        stack.push(p);
                    }
                    continue;
                }
                if !name.ends_with(".rs") || p.components().any(|c| c.as_os_str() == "tests") {
                    continue;
                }
                let text = std::fs::read_to_string(&p).unwrap_or_default();
                let test_at = if name == "testing.rs" { 0 } else { text.find("#[cfg(test)]").unwrap_or(usize::MAX) };
                for (at, _) in text.match_indices("kdf::set_light(") {
                    assert!(at > test_at, "{} 在测试模块之外调 set_light", p.display());
                }
                if name != "kdf.rs" && name != "keybox.rs" {
                    for (at, _) in text.match_indices("set_light_kdf(") {
                        assert!(at > test_at, "{} 在测试模块之外调 set_light_kdf", p.display());
                    }
                }
            }
        }
    }
}
