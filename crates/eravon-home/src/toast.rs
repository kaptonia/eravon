use crate::Why;
use eravon_vault::lang::{fill3, t, Key};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub head: String,
    pub next: String,
    pub detail: Option<String>,
}

pub fn of(e: &Why, endpoints: &[String]) -> Said {
    Said {
        head: t(Key::HomeToastHead).to_string(),
        next: t(Key::HomeToastNext).to_string(),
        detail: Some(fill3(Key::HomeToastDetail, &e.0, &e.1, &endpoints.join(t(Key::ListJoin)))),
    }
}

pub fn local(e: &Why) -> Said {
    Said {
        head: t(Key::HomeLocalFailed).to_string(),
        next: t(Key::HomeLocalNext).to_string(),
        detail: Some(format!("{} · {}", e.0, e.1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_read_failure_names_no_page_and_keeps_the_code_in_detail() {
        let s = of(&("E_CHAIN_TRANSPORT".to_string(), "连不上".to_string()), &["http://a".to_string()]);
        assert!(!s.head.contains("首页") && !s.head.contains("E_"), "{}", s.head);
        assert!(s.detail.unwrap_or_default().contains("E_CHAIN_TRANSPORT"));
    }
}
