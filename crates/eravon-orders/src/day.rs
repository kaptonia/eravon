pub fn of(secs: u64) -> String {
    crate::when::write(secs, 0, 10)
}

pub fn at(secs: u64) -> String {
    crate::when::write(secs, 0, 16)
}

pub fn bound(s: &str) -> Result<Option<String>, crate::Why> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let b = t.as_bytes();
    let ok = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !ok {
        return Err(crate::why("E_ORDERS_DATE", format!("日期格式应为 YYYY-MM-DD:{t}")));
    }
    Ok(Some(t.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_leap_day() {
        assert_eq!(of(0), "");
        assert_eq!(at(0), "");
        assert_eq!(of(1), "1970-01-01");
        assert_eq!(of(951_782_400), "2000-02-29");
        assert_eq!(at(3_723), "1970-01-01 01:02");
        assert!(bound("2026-9-1").is_err());
        assert_eq!(bound("").unwrap(), None);
    }
}
