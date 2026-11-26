use eravon_literals::json::In;

use crate::fault::{Fault, R};

impl Table {
    pub fn nodes(&self) -> Vec<crate::seam::NodeAt> {
        self.list.iter().map(crate::seam::NodeAt::new).collect()
    }
}

pub const V_TABLE: &str = "eravon.chain.endpoints/1";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub list: Vec<String>,
    pub trace: Vec<(String, String)>,
    pub chain: std::sync::OnceLock<u64>,
}

fn fresh(seen: &[String], key: &str) -> bool {
    !seen.iter().any(|x| x == key)
}

impl Table {
    pub fn new(list: Vec<String>) -> R<Table> {
        if list.is_empty() {
            return Err(Fault::Usage(
                "端点表是空的:本 crate 不带出厂端点(契约 11),端点由你给".into(),
            ));
        }
        let mut seen: Vec<String> = Vec::new();
        let mut kept = Vec::new();
        for (i, e) in list.into_iter().enumerate() {
            crate::seam::node_at(i + 1, &e)?;
            let key = crate::endpoints::place_of(&e);
            if fresh(&seen, &key) {
                seen.push(key);
                kept.push(e);
            }
        }
        Ok(Table { list: kept, trace: Vec::new(), chain: std::sync::OnceLock::new() })
    }

    pub fn open(bytes: &[u8]) -> R<Table> {
        let v = eravon_literals::json::read(bytes)
            .map_err(|e| Fault::Usage(format!("端点表读不成 JSON:{e}")))?;
        match v.get("v").and_then(|x| x.as_str()) {
            Some(s) if s == V_TABLE => {}
            other => {
                return Err(Fault::Usage(format!(
                    "端点表的格式版本要是 {V_TABLE},档上是 {other:?}"
                )));
            }
        }
        let arr = match v.get("endpoints") {
            Some(In::Arr(a)) => a.clone(),
            other => return Err(Fault::Usage(format!("endpoints 要是一列:{other:?}"))),
        };
        let mut list = Vec::new();
        for x in arr {
            match x {
                In::Str(s) => list.push(s),
                other => return Err(Fault::Usage(format!("端点要是字符串:{other:?}"))),
            }
        }
        let mut t = Table::new(list)?;
        if let Some(rec) = v.get("trace") {
            let In::Obj(m) = rec else {
                return Err(Fault::Usage(format!("trace 要是一张「端点 → 支持」的表:{rec:?}")));
            };
            for (e, x) in m {
                match x.as_str() {
                    Some(w) if crate::trace::SUPPORT_WORDS.contains(&w) => t.trace.push((e.clone(), w.to_string())),
                    _ => return Err(Fault::Usage(format!("trace 里 {e} 的值要是 callTracer、parity 或 none:{x:?}"))),
                }
            }
        }
        Ok(t)
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

#[cfg(test)]
mod dedupe_tests {
    use super::*;

    #[test]
    fn same_endpoint_twice_is_one() {
        let t = Table::new(vec!["http://127.0.0.1:8545".into(), "http://127.0.0.1:8545".into()]).unwrap();
        assert_eq!(t.list, vec!["http://127.0.0.1:8545".to_string()]);
        let t = Table::new(vec!["http://Node.example".into(), "http://node.example:80/".into()]).unwrap();
        assert_eq!(t.list.len(), 1);
        let t = Table::new(vec!["http://127.0.0.1:8545".into(), "http://localhost:8545".into()]).unwrap();
        assert_eq!(t.list.len(), 2);
        let t = Table::new(vec!["http://[::1]:8545/rpc".into(), "HTTP://[0:0:0:0:0:0:0:1]:8545/rpc".into()]).unwrap();
        assert_eq!(t.list, vec!["http://[::1]:8545/rpc".to_string()], "an IPv6 literal in two spellings is one node");
        let t = Table::new(vec!["http://n.example/a".into(), "http://n.example/b".into()]).unwrap();
        assert_eq!(t.list.len(), 2, "another path is another node");
    }

    #[test]
    fn a_rest_row_is_a_node_and_one_service_is_one_row() {
        let lane = |b: &str| crate::rest::lane(b, 1);
        let t = Table::new(vec!["http://127.0.0.1:8545".into(), lane("https://api.etherscan.io/v2/api"), lane("https://API.etherscan.io/api")]).unwrap();
        assert_eq!(t.list, vec!["http://127.0.0.1:8545".to_string(), lane("https://api.etherscan.io/v2/api")], "one service, one row");
        let t = Table::new(vec![lane("https://api.etherscan.io/v2/api"), lane("https://eth.blockscout.com/api")]).unwrap();
        assert_eq!(t.list.len(), 2, "another service is another row");
        for (bad, word) in [("http://api.etherscan.io/v2/api", "scheme"), ("https://api.etherscan.io/v2/api?apikey=K3Y", "shape")] {
            let f = Table::new(vec![lane(bad)]).expect_err("a Base URL that does not read");
            assert_eq!((f.name(), f.detail().split(' ').next().unwrap_or("")), ("E_CHAIN_USAGE", word), "{bad}");
            assert!(!f.detail().contains("K3Y") && !f.detail().contains("/v2/api"), "{}", f.detail());
        }
    }
}
