pub const DECIMALS_MAX: u8 = 77;

pub const AMOUNT_EMPTY: &str = "amount-empty";
pub const AMOUNT_FORM: &str = "amount-form";
pub const AMOUNT_TOO_PRECISE: &str = "amount-too-precise";
pub const AMOUNT_TOO_LARGE: &str = "amount-too-large";

pub fn units_text(raw: &str, decimals: u8, grouped: bool) -> String {
    let d = decimals as usize;
    let raw = raw.trim_start_matches('0');
    let raw = if raw.is_empty() { "0" } else { raw };
    let (int, frac) = if raw.len() > d {
        (raw[..raw.len() - d].to_string(), raw[raw.len() - d..].to_string())
    } else {
        ("0".to_string(), format!("{}{}", "0".repeat(d - raw.len()), raw))
    };
    let frac = frac.trim_end_matches('0');
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if grouped && i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if frac.is_empty() { out } else { format!("{out}.{frac}") }
}


pub fn units_of(text: &str, decimals: u8) -> Result<String, &'static str> {
    let t: String = text
        .chars()
        .map(|c| match c {
            '０'..='９' => char::from(b'0' + (c as u32 - '０' as u32) as u8),
            '．' | '。' => '.',
            '，' => ',',
            _ => c,
        })
        .collect();
    let t = t.trim();
    if t.is_empty() {
        return Err(AMOUNT_EMPTY);
    }
    let (int, frac) = match t.split_once('.') {
        Some((i, f)) => (i, f),
        None => (t, ""),
    };
    if frac.contains('.') {
        return Err(AMOUNT_FORM);
    }
    if !grouped(int) {
        return Err(AMOUNT_FORM);
    }
    let int: String = int.chars().filter(|c| *c != ',').collect();
    if int.is_empty() && frac.is_empty() {
        return Err(AMOUNT_FORM);
    }
    if !int.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(AMOUNT_FORM);
    }
    let frac = cut(frac, decimals)?;
    let digits = format!("{int}{frac}{}", "0".repeat(decimals as usize - frac.len()));
    let digits = digits.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    if digits.len() > UINT256_MAX.len() || (digits.len() == UINT256_MAX.len() && digits > UINT256_MAX) {
        return Err(AMOUNT_TOO_LARGE);
    }
    Ok(digits.to_string())
}

fn grouped(int: &str) -> bool {
    let mut parts = int.split(',');
    let first = parts.next().unwrap_or_default().chars().count();
    !int.contains(',') || ((1..=3).contains(&first) && parts.all(|g| g.chars().count() == 3))
}


fn cut(frac: &str, decimals: u8) -> Result<&str, &'static str> {
    if frac.len() > decimals as usize { Err(AMOUNT_TOO_PRECISE) } else { Ok(frac) }
}


pub const UINT256_MAX: &str = "115792089237316195423570985008687907853269984665640564039457584007913129639935";

pub const SHORT_PLACES: usize = 6;

pub fn units_short(full: &str) -> String {
    let Some((int, frac)) = full.split_once('.') else { return full.to_string() };
    if frac.len() <= SHORT_PLACES {
        return full.to_string();
    }
    let kept = frac[..SHORT_PLACES].trim_end_matches('0');
    let int_zero = int.chars().all(|c| c == '0' || c == ',');
    if int_zero && kept.is_empty() {
        return format!("< 0.{}1", "0".repeat(SHORT_PLACES - 1));
    }
    if kept.is_empty() { format!("{int}…") } else { format!("{int}.{kept}…") }
}

pub fn short(address: &str) -> String {
    let c: Vec<char> = address.chars().collect();
    if c.len() > 12 { format!("{}…{}", c[..6].iter().collect::<String>(), c[c.len() - 4..].iter().collect::<String>()) } else { address.to_string() }
}

#[derive(Clone, Copy, Debug)]
pub struct Words {
    pub smallest: &'static str,
    pub unlisted: &'static str,
}

pub const ZH: Words = Words { smallest: "最小单位", unlisted: "未登载" };
pub const EN: Words = Words { smallest: "smallest units", unlisted: "Not listed" };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Short,
    Full,
    Plain,
}

pub fn amount(raw: &str, decimals: Option<u8>, name: &str, form: Form, w: &Words) -> String {
    match decimals {
        Some(d) if is_raw(raw) => format!("{} {name}", figure(raw, d, form)),
        Some(_) => format!("{raw} {name}"),
        None => format!("{raw} {} · {name}", w.smallest),
    }
}

pub fn figure(raw: &str, decimals: u8, form: Form) -> String {
    match form {
        Form::Short => units_short(&units_text(raw, decimals, true)),
        Form::Full => units_text(raw, decimals, true),
        Form::Plain => units_text(raw, decimals, false),
    }
}

fn is_raw(raw: &str) -> bool {
    !raw.is_empty() && raw.bytes().all(|c| c.is_ascii_digit())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_rows_read_and_write_back() {
        let rows: [(&str, u8, Result<&str, &str>, &str); 9] = [
            ("0.5", 18, Ok("500000000000000000"), "0.5"),
            ("1", 18, Ok("1000000000000000000"), "1"),
            ("25", 6, Ok("25000000"), "25"),
            ("1,234.56", 6, Ok("1234560000"), "1,234.56"),
            ("0.00000001", 8, Ok("1"), "0.00000001"),
            ("0.0000001", 6, Err(AMOUNT_TOO_PRECISE), ""),
            (".5", 18, Ok("500000000000000000"), "0.5"),
            ("1.2.3", 18, Err(AMOUNT_FORM), ""),
            ("０．５", 6, Ok("500000"), "0.5"),
        ];
        for (typed, d, stored, shown) in rows {
            let got = units_of(typed, d);
            assert_eq!(got.as_ref().map(String::as_str).map_err(|e| *e), stored, "{typed} · {d}");
            if let Ok(raw) = got {
                assert_eq!(units_text(&raw, d, true), shown, "{raw} · {d}");
            }
        }
        assert_eq!(units_text("1234560000", 6, false), "1234.56");
    }

    #[test]
    fn commas_only_on_the_thousands_places() {
        let rows: [(&str, u8, Result<&str, &str>, &str); 5] = [
            ("1,5", 6, Err(AMOUNT_FORM), ""),
            ("1,00", 6, Err(AMOUNT_FORM), ""),
            ("12,345", 0, Ok("12345"), "12,345"),
            ("1,234,567.5", 6, Ok("1234567500000"), "1,234,567.5"),
            (",5", 6, Err(AMOUNT_FORM), ""),
        ];
        for (typed, d, stored, shown) in rows {
            let got = units_of(typed, d);
            assert_eq!(got.as_ref().map(String::as_str).map_err(|e| *e), stored, "{typed} · {d}");
            if let Ok(raw) = got {
                assert_eq!(units_text(&raw, d, true), shown, "{raw} · {d}");
            }
        }
    }

    #[test]
    fn the_six_short_rows() {
        let rows = [
            ("0.5", "0.5"),
            ("1,234.56", "1,234.56"),
            ("0.123456789012345678", "0.123456…"),
            ("0.0000009", "< 0.000001"),
            ("12.0000004", "12…"),
            ("0.999999999", "0.999999…"),
        ];
        for (full, short) in rows {
            assert_eq!(units_short(full), short, "{full}");
        }
    }

    #[test]
    fn the_uint256_bound_two_colours() {
        assert_eq!(units_of(UINT256_MAX, 0).as_deref(), Ok(UINT256_MAX));
        assert_eq!(units_text(UINT256_MAX, 0, false), UINT256_MAX);
        let one_more = "115792089237316195423570985008687907853269984665640564039457584007913129639936";
        assert_eq!(units_of(one_more, 0), Err(AMOUNT_TOO_LARGE));
        assert_eq!(units_of("1", DECIMALS_MAX).map(|r| r.len()), Ok(78));
        assert_eq!(units_of("2", DECIMALS_MAX), Err(AMOUNT_TOO_LARGE));
        assert_eq!(units_of("  ", 6), Err(AMOUNT_EMPTY));
        assert_eq!(units_of("1e5", 6), Err(AMOUNT_FORM));
    }
}
