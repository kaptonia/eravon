use std::sync::Mutex;

static HIDDEN: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

pub fn hide(typed: &str, said: &str) {
    let mut h = HIDDEN.lock().unwrap_or_else(|e| e.into_inner());
    for t in [typed, typed.trim()] {
        if !t.is_empty() && t != said && !h.iter().any(|(x, _)| x == t) {
            h.push((t.to_string(), said.to_string()));
        }
    }
    h.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
}

pub fn scrub(bytes: &[u8]) -> Vec<u8> {
    let h = HIDDEN.lock().unwrap_or_else(|e| e.into_inner());
    if h.is_empty() {
        return bytes.to_vec();
    }
    let Ok(s) = std::str::from_utf8(bytes) else { return bytes.to_vec() };
    let mut s = s.to_string();
    for (typed, said) in h.iter() {
        if s.contains(typed.as_str()) {
            s = s.replace(typed.as_str(), said);
        }
    }
    s.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorded_address_is_never_said_whole() {
        hide("https://node.example/v3/KEY1", "https://node.example:443/…");
        hide("https://node.example/v3/KEY1?x=KEY2", "https://node.example:443/…");
        let out = scrub(br#"{"endpoint":"https://node.example/v3/KEY1?x=KEY2","left":["https://node.example/v3/KEY1 E_CHAIN_TRANSPORT"]}"#);
        let out = String::from_utf8(out).expect("utf-8");
        assert!(!out.contains("KEY1") && !out.contains("KEY2"), "{out}");
        assert!(out.contains("https://node.example:443/…"), "{out}");
        assert_eq!(scrub(b"{\"ok\":true}"), b"{\"ok\":true}".to_vec());
    }
}
