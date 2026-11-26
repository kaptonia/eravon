use eravon_store::settings::TokenRow;
use eravon_vault::tokens::{self, Words};
pub use eravon_vault::tokens::Form;

use crate::house::i18n::Lang;

pub fn words(lang: Lang) -> &'static Words {
    match lang {
        Lang::Zh => &tokens::ZH,
        Lang::En => &tokens::EN,
    }
}

pub fn rows() -> std::sync::Arc<Vec<TokenRow>> {
    tokens::current_shared()
}

pub fn name(lang: Lang, token: &str) -> String {
    tokens::name(&rows(), tokens::chain(), token, words(lang))
}

pub fn amount(lang: Lang, raw: &str, recorded: Option<u64>, token: &str, form: Form) -> String {
    eravon_orders::table::amount_in(&rows(), words(lang), raw, recorded, token, form)
}

pub fn parts(lang: Lang, raw: &str, recorded: Option<u64>, token: &str, form: Form) -> (String, String) {
    let rows = rows();
    let chain = tokens::chain();
    let d = tokens::decimals_of(&rows, chain, token, recorded);
    (figure(lang, raw, d.map(u64::from), form), tokens::name(&rows, chain, token, words(lang)))
}

pub fn pair(lang: Lang, have: &str, need: &str, recorded: Option<u64>, token: &str, form: Form) -> String {
    let (h, name) = parts(lang, have, recorded, token, form);
    let (n, _) = parts(lang, need, recorded, token, form);
    let known = tokens::decimals_of(&rows(), tokens::chain(), token, recorded).is_some();
    if known { format!("{h} / {n} {name}") } else { format!("{h} / {n} · {name}") }
}

pub fn decimals_read(w: &eravon_cli::shared::Wire, token: &str) -> Option<u64> {
    tokens::decimals_of(&rows(), tokens::chain(), token, None).map(u64::from).or_else(|| eravon_standard::chain::side_decimals(&w.t, &w.table, token))
}

pub fn cut(lang: Lang, raw: &str, recorded: Option<u64>, token: &str) -> Option<String> {
    let full = amount(lang, raw, recorded, token, Form::Full);
    (amount(lang, raw, recorded, token, Form::Short) != full).then_some(full)
}

pub fn eth_cut(lang: Lang, wei: &str) -> Option<String> {
    let full = eth(lang, wei, Form::Full);
    (eth(lang, wei, Form::Short) != full).then_some(full)
}

pub fn word(raw: &str, recorded: Option<u64>, token: &str, form: Form) -> String {
    amount(Lang::Zh, raw, recorded, token, form)
}

pub fn cell(cell: &str) -> Option<String> {
    eravon_goods::draft::token_address(cell)
}

pub fn figure(lang: Lang, raw: &str, decimals: Option<u64>, form: Form) -> String {
    match decimals.and_then(|d| u8::try_from(d).ok()).filter(|d| *d <= eravon_literals::units::DECIMALS_MAX) {
        Some(d) if !raw.is_empty() && raw.bytes().all(|c| c.is_ascii_digit()) => tokens::figure(raw, d, form),
        Some(_) => raw.to_string(),
        None => format!("{raw} {}", words(lang).smallest),
    }
}

pub fn amount_tip(lang: Lang, raw: &str, recorded: Option<u64>, token: &str) -> eravon_ui::layer::Amount {
    eravon_ui::layer::Amount::new(amount(lang, raw, recorded, token, Form::Short), amount(lang, raw, recorded, token, Form::Full))
}

pub fn eth_tip(lang: Lang, wei: &str) -> eravon_ui::layer::Amount {
    eravon_ui::layer::Amount::new(eth(lang, wei, Form::Short), eth(lang, wei, Form::Full))
}

pub fn eth(lang: Lang, wei: &str, form: Form) -> String {
    let c = eravon_literals::constants::NATIVE_DECIMALS;
    tokens::amount(wei, Some(c), eravon_literals::constants::NATIVE_SYMBOL, form, words(lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_amount_is_handed_as_figure_and_name() {
        let usdc = tokens::on_chain(&tokens::factory(), tokens::chain()).into_iter().find(|r| r.label == "USDC").unwrap().address;
        let other = "0x4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e";
        assert_eq!(parts(Lang::En, "5", None, other, Form::Full), ("5 smallest units".to_string(), "Not listed 0x4e4e…4e4e".to_string()));
        assert_eq!(parts(Lang::Zh, "1500000", None, &usdc, Form::Full), ("1.5".to_string(), "USDC".to_string()));
        assert_eq!(pair(Lang::Zh, "0", "625000", None, &usdc, Form::Full), "0 / 0.625 USDC");
        assert_eq!(pair(Lang::Zh, "5", "7", None, other, Form::Full), "5 最小单位 / 7 最小单位 · 未登载 0x4e4e…4e4e");
    }

    #[test]
    fn eth_is_written_with_the_native_decimals() {
        assert_eq!(eth(Lang::Zh, "1234500000000000000000", Form::Full), "1,234.5 ETH");
        assert_eq!(eth(Lang::En, "123456789012345678", Form::Short), "0.123456… ETH");
    }
}
