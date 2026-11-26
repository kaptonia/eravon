use galeed::audit;
use galeed::hexform;
use galeed::json::Value;

pub const FILLER_ROOT: &str = "0x0000000000000000000000000000000000000000";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub chain_id: u64,
    pub from_block: u64,
    pub to_block: u64,
    pub registries: Vec<String>,
    pub classes: Vec<String>,
    pub senders: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Basis {
    pub value: Value,
    pub windows: Vec<Window>,
}

pub fn minimal_input(basis: &Value, anchors: Vec<Value>) -> Value {
    Value::Obj(vec![
        ("anchors".to_string(), Value::Arr(anchors)),
        ("basis".to_string(), basis.clone()),
        ("root".to_string(), Value::Str(FILLER_ROOT.to_string())),
    ])
}

pub fn read(basis: &Value) -> Option<Basis> {
    audit::read_input(&minimal_input(basis, vec![]))?;
    values(basis)
}


fn values(basis: &Value) -> Option<Basis> {
    let mut windows = Vec::new();
    if let Some(Value::Arr(cs)) = basis.get(audit::BASIS_CHAINS) {
        for c in cs {
            let (registries, classes) = two_forms(c.get("registries"))?;
            windows.push(Window {
                chain_id: int(c.get("chainId"))?,
                from_block: int(c.get("fromBlock"))?,
                to_block: int(c.get("toBlock"))?,
                registries,
                classes,
                senders: strs(c.get("senders"))?,
            });
        }
    }
    Some(Basis { value: basis.clone(), windows })
}

fn int(v: Option<&Value>) -> Option<u64> {
    match v? {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}

fn two_forms(v: Option<&Value>) -> Option<(Vec<String>, Vec<String>)> {
    let Value::Arr(a) = v? else { return None };
    let mut addresses = Vec::new();
    let mut classes = Vec::new();
    for e in a {
        match e {
            Value::Str(t) if hexform::is_hex20(t) => addresses.push(t.clone()),
            Value::Obj(_) => classes.push(e.get("codeHash")?.as_str().filter(|h| hexform::is_hex32(h))?.to_string()),
            _ => return None,
        }
    }
    Some((addresses, classes))
}

fn strs(v: Option<&Value>) -> Option<Vec<String>> {
    match v? {
        Value::Arr(a) => a.iter().map(|x| x.as_str().filter(|s| hexform::is_hex20(s)).map(str::to_string)).collect(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use galeed::json;

    fn basis(s: &str) -> Option<Basis> {
        read(&json::read_open(s.as_bytes()).unwrap())
    }

    #[test]
    fn a_well_formed_basis_is_taken_and_its_windows_come_out() {
        let b = basis(r#"{"chains":[{"chainId":1,"fromBlock":0,"toBlock":2,"registries":["0xabababababababababababababababababababab"],"senders":["0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"]}]}"#).unwrap();
        assert_eq!(b.windows.len(), 1);
        assert_eq!((b.windows[0].chain_id, b.windows[0].to_block), (1, 2));
        assert!(basis(r#"{"chains":[],"transactions":[]}"#).is_none());
    }

    #[test]
    fn the_form_rules_are_k1s_and_a3_refuses_whatever_k1_refuses() {
        assert!(basis(r#"{"chains":[],"extra":1}"#).is_none());
        assert!(basis(r#"{"chains":[{"chainId":2,"fromBlock":0,"toBlock":1,"registries":[],"senders":[]},{"chainId":1,"fromBlock":0,"toBlock":1,"registries":[],"senders":[]}]}"#).is_none());
        assert!(basis(r#"{"chains":[{"chainId":1,"fromBlock":5,"toBlock":1,"registries":[],"senders":[]}]}"#).is_none());
        assert!(basis(r#"{"chains":[{"chainId":1,"fromBlock":0,"toBlock":1,"registries":["0xabababababababababababababababababababab","0xabababababababababababababababababababab"],"senders":[]}]}"#).is_none());
        assert!(basis("[]").is_none(), "不是对象");
    }

    #[test]
    fn the_two_registry_forms_land_in_their_own_halves() {
        let b = basis(r#"{"chains":[{"chainId":1,"fromBlock":0,"toBlock":2,
          "registries":["0xabababababababababababababababababababab",{"codeHash":"0x1111111111111111111111111111111111111111111111111111111111111111"},{"codeHash":"0x2222222222222222222222222222222222222222222222222222222222222222"}],
          "senders":["0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"]}]}"#).unwrap();
        assert_eq!(b.windows[0].registries, vec!["0xabababababababababababababababababababab".to_string()]);
        assert_eq!(b.windows[0].classes.len(), 2, "两枚类项各进类项那一堆");
        let only = basis(r#"{"chains":[{"chainId":1,"fromBlock":0,"toBlock":2,"registries":[{"codeHash":"0x1111111111111111111111111111111111111111111111111111111111111111"}],"senders":[]}]}"#).unwrap();
        assert!(only.windows[0].registries.is_empty() && only.windows[0].classes.len() == 1);
        assert!(basis(r#"{"chains":[{"chainId":1,"fromBlock":0,"toBlock":2,"registries":[{"codeHash":"0x1111111111111111111111111111111111111111111111111111111111111111"},"0xabababababababababababababababababababab"],"senders":[]}]}"#).is_none());
        assert!(basis(r#"{"chains":[{"chainId":1,"fromBlock":0,"toBlock":2,"registries":[{"codeHash":"0x1111111111111111111111111111111111111111111111111111111111111111"},{"codeHash":"0x1111111111111111111111111111111111111111111111111111111111111111"}],"senders":[]}]}"#).is_none());
        assert!(basis(r#"{"chains":[{"chainId":1,"fromBlock":0,"toBlock":2,"registries":[{"codeHash":"0x1111111111111111111111111111111111111111111111111111111111111111","note":"x"}],"senders":[]}]}"#).is_none());
    }
}
