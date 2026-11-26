use eravon::json::Value;

pub const SETTINGS_FILE: &str = "desk.json";

pub const CHAIN_ID: &str = "chainId";
pub const ENDPOINTS: &str = "endpoints";
pub const REST: &str = "restApis";
pub const REST_ON: &str = "restApiMode";
pub const LOG_ASKS: &str = "logAsks";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ground {
    pub chain_id: Option<u64>,
    pub endpoints: Vec<String>,
    pub rest: Vec<String>,
    pub rest_on: bool,
    pub log_asks: Option<u64>,
}

impl Ground {
    pub fn rest_asked(&self) -> &[String] {
        if self.rest_on { &self.rest } else { &[] }
    }
}

pub fn of_settings(v: &Value) -> Ground {
    Ground {
        chain_id: match v.member(CHAIN_ID) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        },
        endpoints: texts(v.member(ENDPOINTS)),
        rest: texts(v.member(REST)),
        rest_on: matches!(v.member(REST_ON), Some(Value::Bool(true))),
        log_asks: match v.member(LOG_ASKS) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        },
    }
}

fn texts(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Arr(a)) => a
            .iter()
            .filter_map(|x| match x {
                Value::Str(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_read_as_set_or_not_set() {
        let v = eravon::json::parse(br#"{"chainId":11155111,"endpoints":["11155111=https://a.example/k","x",7],"fromBlock":7,"registry":"0x36EA8A857A5FE813429D4D9947000C644A88809A"}"#).ok().expect("json");
        let g = of_settings(&v);
        assert_eq!(g.chain_id, Some(11155111));
        assert_eq!(g.endpoints, vec!["11155111=https://a.example/k".to_string(), "x".to_string()], "节点照写的原样读,形在用处判;不是字的不算节点");
        let none = of_settings(&eravon::json::parse(br#"{"chainId":"1","endpoints":"1=http://a","registry":"0x12"}"#).ok().expect("json"));
        assert_eq!(none, Ground::default(), "形不对的读作没设");
    }
}
