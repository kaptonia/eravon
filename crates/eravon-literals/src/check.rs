use crate::foreign::Family;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collision {
    pub literal: String,
    pub families: Vec<String>,
}

pub fn distinct(families: &[Family]) -> Vec<Collision> {
    use std::collections::BTreeMap;
    let mut seen: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for f in families {
        for l in &f.literals {
            let e = seen.entry(l.as_str()).or_default();
            if !e.contains(&f.name) {
                e.push(f.name.clone());
            }
        }
    }
    let mut out: Vec<Collision> = Vec::new();
    for f in families {
        for l in &f.literals {
            if seen.get(l.as_str()).is_some_and(|v| v.len() > 1)
                && !out.iter().any(|c| c.literal == *l)
            {
                out.push(Collision {
                    literal: l.clone(),
                    families: seen[l.as_str()].clone(),
                });
            }
        }
    }
    out
}


pub fn self_check() -> Result<(), &'static str> {
    let a = Family {
        name: "self-check-a".to_string(),
        source: "内建定值".to_string(),
        literals: vec!["x/1".to_string(), "y/1".to_string()],
    };
    let b = Family {
        name: "self-check-b".to_string(),
        source: "内建定值".to_string(),
        literals: vec!["y/1".to_string(), "z/1".to_string()],
    };
    let seen = distinct(&[a, b]);
    if seen.len() != 1 || seen[0].literal != "y/1" {
        return Err("E_DISTINCT_BLIND");
    }
    let c = Family {
        name: "self-check-c".to_string(),
        source: "内建定值".to_string(),
        literals: vec!["p/1".to_string()],
    };
    let d = Family {
        name: "self-check-d".to_string(),
        source: "内建定值".to_string(),
        literals: vec!["q/1".to_string()],
    };
    if !distinct(&[c, d]).is_empty() {
        return Err("E_DISTINCT_FALSE_ALARM");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fam(n: &str, ls: &[&str]) -> Family {
        Family {
            name: n.to_string(),
            source: "测试".to_string(),
            literals: ls.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn comparator_passes_its_own_self_check() {
        assert_eq!(self_check(), Ok(()));
    }

    #[test]
    fn three_clean_families_collide_nowhere() {
        let f = [fam("a", &["a/1"]), fam("b", &["b/1"]), fam("c", &["c/1"])];
        assert!(distinct(&f).is_empty());
    }

    #[test]
    fn one_literal_in_three_families_names_all_three() {
        let f = [fam("a", &["same"]), fam("b", &["same"]), fam("c", &["same"])];
        let got = distinct(&f);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].families, vec!["a", "b", "c"]);
    }

    #[test]
    fn a_repeat_inside_one_family_is_not_a_cross_family_collision() {
        let f = [fam("a", &["dup", "dup"]), fam("b", &["b/1"])];
        assert!(distinct(&f).is_empty());
    }
}
