use crate::fault::Fault;

pub const EVERY_MAX: u64 = i64::MAX as u64;

pub fn every_parse(v: &str) -> Option<u64> {
    let v = v.trim();
    if v.is_empty() || v == "off" { Some(0) } else { v.parse::<u64>().ok() }.filter(|n| *n <= EVERY_MAX)
}

pub fn every_words(n: u64) -> String {
    use crate::lang::{fill1, t, Key};
    match n {
        0 => t(Key::SetEveryOff).to_string(),
        n => fill1(Key::SetEvery, &n.to_string()),
    }
}

pub fn read() -> u64 {
    super::data()
        .ok()
        .and_then(|d| eravon_vault::store::read_global(&d).ok())
        .and_then(|g| g.refresh_every)
        .unwrap_or(eravon_literals::constants::READ_REFRESH_SECS)
}

pub fn set(typed: &str) -> Result<u64, Fault> {
    let Some(n) = every_parse(typed) else {
        return Err(Fault::unknown(crate::lang::fill1(crate::lang::Key::SetEveryUnknown, typed.trim())));
    };
    write(n)?;
    Ok(n)
}

fn write(n: u64) -> Result<(), Fault> {
    super::write_global(|g| g.refresh_every = Some(n))
}


#[cfg(test)]
mod tests {
    #[test]
    fn a_period_is_seconds_or_off_and_nothing_else() {
        assert_eq!(super::every_parse(" 90 "), Some(90));
        assert_eq!(super::every_parse("off"), Some(0));
        assert_eq!(super::every_parse("  "), Some(0), "an empty cell is manual only");
        assert_eq!(super::every_parse("1m"), None);
        assert_eq!(super::every_parse(&(super::EVERY_MAX as u128 + 1).to_string()), None);
    }
}
