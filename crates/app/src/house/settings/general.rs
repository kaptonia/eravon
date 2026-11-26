use super::*;

pub use crate::house::general::{every_parse, EVERY_MAX};

pub fn look_of(stored: &str) -> &'static str {
    LOOKS.iter().copied().find(|x| *x == stored && *x != "system").unwrap_or("system")
}

pub fn look_now() -> &'static str {
    crate::machine::read().map(|m| look_of(m.appearance.as_deref().unwrap_or_default())).unwrap_or("system")
}

pub fn lang_now() -> crate::house::i18n::Lang {
    crate::machine::read().ok().and_then(|m| m.lang).unwrap_or(crate::house::i18n::Lang::Zh)
}

pub fn zone_of(stored: &str) -> eravon_orders::when::Zone {
    eravon_orders::when::Zone::parse(stored).unwrap_or(eravon_orders::when::BORN)
}

pub fn every_of(stored: Option<u64>) -> u64 {
    stored.unwrap_or(eravon_literals::constants::READ_REFRESH_SECS)
}

pub fn every_words(lang: crate::house::i18n::Lang, n: u64) -> String {
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    match n {
        0 => t(crate::house::i18n::Key::SetEveryOff),
        n if n % 60 == 0 => crate::fill_in!(lang, crate::house::i18n::Key::SetEveryMins; (n / 60).to_string()),
        n => crate::fill_in!(lang, crate::house::i18n::Key::SetEvery; n.to_string()),
    }
}

pub fn zone_now() -> eravon_orders::when::Zone {
    let home = crate::register::now_row_listed().ok().flatten().and_then(|(row, seat)| row.home(seat));
    let set = home.and_then(|h| crate::home::Home::open(&h).ok()).and_then(|h| crate::settings::Settings::read(&h).ok()).and_then(|s| s.zone);
    zone_of(crate::when::chosen(set).as_str())
}

pub fn reading_now(data: &str) -> (eravon_orders::when::Zone, u64) {
    let every = eravon_vault::store::read_global(data).map(|g| every_of(g.refresh_every)).unwrap_or(eravon_literals::constants::READ_REFRESH_SECS);
    (zone_now(), every)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refresh_period_is_said_in_minutes_or_seconds() {
        let zh = crate::house::i18n::Lang::Zh;
        assert_eq!(every_words(zh, 0), crate::house::i18n::get(zh, crate::house::i18n::Key::SetEveryOff).0);
        assert_ne!(every_words(zh, 60), every_words(zh, 61), "minutes and seconds said apart");
    }
}
