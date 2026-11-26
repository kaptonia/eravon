use eravon_galeed::Value;
use eravon_terms::quote::Quote;
use eravon_terms::u256::U256;

pub const EXAMPLE_WHOLE: u64 = 1200;

#[derive(Clone, Debug)]
pub struct Row {
    pub token: String,
    pub spelling: String,
    pub quote: Result<Quote, String>,
    pub usable: bool,
}

pub fn paced(chain: u64, q: &Quote) -> Result<(), String> {
    eravon_roster::pool::pace(chain, q.offer_window).map_err(|_| format!("报价 offerWindow {} 秒短于链 {chain} 要的 256 个出块间隔", q.offer_window))
}

impl Row {
    pub fn hash(&self) -> Option<String> {
        let bytes = eravon_galeed::hexform::decode_digits(self.spelling.as_bytes().get(2..)?)?;
        Some(eravon_standard::order::hex_of(&eravon_terms::derive::keccak(&bytes)))
    }

    pub fn paced(&self) -> bool {
        matches!(&self.quote, Ok(q) if paced(eravon_roster::pool::standard_chain_id(), q).is_ok())
    }
}

fn ours(k: &eravon_roster::adapter::QuoteKey) -> bool {
    use eravon_roster::adapter::QuoteItem;
    k.chain == eravon_roster::pool::standard_chain_id()
        && matches!(&k.item, QuoteItem::Standard(h) if h == eravon_roster::pool::standard_code_hash())
        && k.token.is_some()
}


pub fn rows(quotes: Option<&Value>) -> Vec<Row> {
    let Some(Value::Obj(members)) = quotes else { return Vec::new() };
    let mut out = Vec::new();
    for (key, v) in members {
        let Ok(k) = eravon_roster::adapter::quote_key(key) else { continue };
        if !ours(&k) {
            continue;
        }
        let spelling = v.as_str().unwrap_or_default().to_string();
        let quote = decode(&spelling);
        let usable = quote.is_ok();
        out.push(Row { token: k.token.clone().unwrap_or_default(), spelling, quote, usable });
    }
    out.sort_by(|a, b| a.token.cmp(&b.token));
    out
}

pub fn decode(spelling: &str) -> Result<Quote, String> {
    if !eravon_roster::adapter::quote_spelling(spelling) {
        return Err("E_QUOTE_FORM 报价拼写不是小写 0x 十六进制".to_string());
    }
    let bytes = eravon_galeed::hexform::decode_digits(&spelling.as_bytes()[2..]).ok_or_else(|| "报价格式无效".to_string())?;
    eravon_terms::quote::decode(&bytes).map_err(|e| format!("{e:?}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cells {
    pub premium: String,
    pub lawsuit: String,
    pub ruling_days: String,
    pub lapse: String,
    pub cap: String,
    pub accept: String,
    pub reference: String,
    pub example_amount: String,
    pub example_premium: String,
    pub example_lawsuit: String,
}

fn pct(bps: u64) -> String {
    format!("{}.{:02}%", bps / 100, bps % 100)
}

fn human(v: &U256, token: &str, decimals: Option<u64>, w: &eravon_vault::tokens::Words) -> String {
    use eravon_vault::tokens;
    let rows = tokens::current();
    let d = decimals.and_then(|d| u8::try_from(d).ok());
    tokens::amount(&v.to_dec(), d, &tokens::name(&rows, tokens::chain(), token, w), tokens::Form::Full, w)
}

fn decimals_in(decimals: &[(String, Option<u64>)], token: &str) -> Option<u64> {
    decimals.iter().find(|(t, _)| t.eq_ignore_ascii_case(token)).and_then(|(_, d)| *d)
}

fn say(lang: eravon_vault::lang::Lang, k: eravon_vault::lang::Key) -> &'static str {
    let (zh, en) = eravon_vault::lang::both(k);
    if lang == eravon_vault::lang::Lang::Zh { zh } else { en }
}

pub fn lasts(lang: eravon_vault::lang::Lang, secs: u64) -> String {
    use eravon_vault::lang::Key as K;
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3_600, secs % 3_600 / 60);
    let out: Vec<String> = [(d, K::CliLastsDays), (h, K::CliLastsHours), (m, K::CliLastsMinutes)].into_iter().filter(|(n, _)| *n > 0).map(|(n, k)| format!("{n} {}", say(lang, k))).collect();
    if out.is_empty() { format!("0 {}", say(lang, K::CliLastsMinutes)) } else { out.join(" ") }
}

pub fn cells(q: &Quote, token: &str, all: &[(String, Option<u64>)], lang: eravon_vault::lang::Lang) -> Cells {
    use eravon_vault::lang::Key as K;
    let w = if lang == eravon_vault::lang::Lang::Zh { &eravon_vault::tokens::ZH } else { &eravon_vault::tokens::EN };
    let fill = |k: K, args: &[&str]| args.iter().enumerate().fold(say(lang, k).to_string(), |acc, (i, a)| acc.replace(&format!("{{{i}}}"), a));
    let decimals = decimals_in(all, token);
    let (abs_token, abs_decimals) = if q.fee_unit_is_zero() { (token.to_string(), decimals) } else { (q.fee_unit.clone(), decimals_in(all, &q.fee_unit)) };
    let abs = |v: &U256| human(v, &abs_token, abs_decimals, w);
    let unit = decimals
        .and_then(|d| u8::try_from(d).ok())
        .and_then(|d| eravon_literals::units::units_of(&EXAMPLE_WHOLE.to_string(), d).ok())
        .and_then(|raw| U256::from_dec(&raw));
    let (example_amount, example_premium, example_lawsuit) = match unit {
        Some(a) => match eravon_terms::fee::fee(&example_quote(q), &a) {
            Ok(f) => (
                human(&a, token, decimals, w),
                human(&f.premium, token, decimals, w),
                match f.lf {
                    Some(lf) => human(&lf, token, decimals, w),
                    None => say(lang, K::CandsQByReference).to_string(),
                },
            ),
            Err(e) => (say(lang, K::CandsAbsentUnreachable).into(), format!("{e:?}"), format!("{e:?}")),
        },
        None => (say(lang, K::CandsAbsentUnreachable).into(), say(lang, K::CandsQNoDecimals).into(), say(lang, K::CandsQNoDecimals).into()),
    };
    let bound = |v: &U256| if v.is_zero() { say(lang, K::CandsQUnset).to_string() } else { abs(v) };
    Cells {
        premium: pct(q.premium_bps),
        lawsuit: fill(K::CandsQLawsuitForm, &[&pct(q.fee_bps), &abs(&q.fee_fixed), &bound(&q.fee_min), &bound(&q.fee_max)]),
        ruling_days: lasts(lang, q.ruling_window),
        lapse: match crate::lapse::Lapse::of(q.lapse_default) {
            Some(crate::lapse::Lapse::FullRefund) => say(lang, K::LapseFullRefund).into(),
            Some(crate::lapse::Lapse::Half) => say(lang, K::LapseHalf).into(),
            Some(crate::lapse::Lapse::ToSeller) => say(lang, K::LapseToSeller).into(),
            None => format!("E_QUOTE_FIELD 13 lapseDefault {}", q.lapse_default),
        },
        cap: if q.amount_cap.is_zero() { say(lang, K::CandsQUncapped).into() } else { abs(&q.amount_cap) },
        accept: if q.auto_accept { say(lang, K::CandsQAuto).into() } else { fill(K::CandsQAnswerIn, &[&lasts(lang, q.offer_window)]) },
        reference: if q.fee_unit_is_zero() { String::new() } else { fill(K::CandsQReference, &[&eravon_vault::tokens::name(&eravon_vault::tokens::current(), eravon_vault::tokens::chain(), &q.fee_unit, w)]) },
        example_amount,
        example_premium,
        example_lawsuit,
    }
}

fn example_quote(q: &Quote) -> Quote {
    q.clone()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quote_is_read_only_in_its_lowercase_spelling() {
        let q = format!("0x{}", "00".repeat(512));
        assert!(decode(&q).map_err(|e| e).err().map_or(true, |e| !e.contains("E_QUOTE_FORM")), "小写形过拼写这一关");
        for bad in [format!("0x{}", "AB".repeat(512)), "00".repeat(512), format!("0X{}", "00".repeat(512))] {
            assert!(decode(&bad).expect_err("该拒").contains("E_QUOTE_FORM"), "{}", &bad[..6]);
        }
    }

    fn quote(offer_window: u64) -> Quote {
        Quote {
            token: format!("0x{}", "4e".repeat(20)),
            premium_bps: 100,
            fee_unit: format!("0x{}", "00".repeat(20)),
            feed_token: format!("0x{}", "00".repeat(20)),
            feed_unit: format!("0x{}", "00".repeat(20)),
            max_stale: 0,
            fee_fixed: U256::ZERO,
            fee_bps: 500,
            fee_min: U256::ZERO,
            fee_max: U256::ZERO,
            amount_cap: U256::ZERO,
            ruling_window: 14 * 86_400,
            offer_window,
            lapse_default: 0,
            auto_accept: false,
            bytes: Vec::new(),
        }
    }

    #[test]
    fn an_order_window_shorter_than_the_chain_needs_is_unpaced() {
        let need = eravon_roster::pool::SEED_BLOCKS * eravon_roster::pool::paced_interval(1).expect("主网有节奏");
        assert!(paced(1, &quote(need)).is_ok());
        assert!(paced(1, &quote(need - 1)).is_err());
    }

    #[test]
    fn the_quote_cells_say_spans_and_words_in_the_readers_language() {
        let q = quote(5_400);
        let zh = cells(&q, &q.token, &[], eravon_vault::lang::Lang::Zh);
        let en = cells(&q, &q.token, &[], eravon_vault::lang::Lang::En);
        assert_eq!(zh.accept, "1 小时 30 分钟 内答复");
        assert!(!zh.accept.contains("秒") && !en.accept.contains("秒") && !en.cap.contains("不"), "{} · {}", en.accept, en.cap);
        assert_eq!(zh.ruling_days, "14 天");
        assert!(en.lapse.chars().all(|c| c.is_ascii()), "{}", en.lapse);
    }

    #[test]
    fn human_units_never_round() {
        let rows = eravon_vault::tokens::factory();
        let usdc = eravon_vault::tokens::on_chain(&rows, eravon_vault::tokens::chain()).into_iter().find(|r| r.label == "USDC").unwrap().address;
        let v = U256::from_dec("1200000001").unwrap();
        assert_eq!(human(&v, &usdc, Some(6), &eravon_vault::tokens::ZH), "1,200.000001 USDC");
        assert_eq!(human(&U256::from_dec("12000000").unwrap(), &usdc, Some(6), &eravon_vault::tokens::ZH), "12 USDC");
        assert_eq!(human(&U256::from_dec("5").unwrap(), &usdc, Some(6), &eravon_vault::tokens::ZH), "0.000005 USDC");
        let other = format!("0x{}", "4e".repeat(20));
        assert_eq!(human(&U256::from_dec("5").unwrap(), &other, None, &eravon_vault::tokens::ZH), "5 最小单位 · 未登载 0x4e4e…4e4e");
        assert_eq!(human(&U256::from_dec("5").unwrap(), &other, None, &eravon_vault::tokens::EN), "5 smallest units · Not listed 0x4e4e…4e4e");
        assert_eq!(decimals_in(&[(usdc.to_ascii_uppercase().replacen("0X", "0x", 1), Some(6))], &usdc), Some(6));
        assert_eq!(pct(100), "1.00%");
        assert_eq!(pct(1), "0.01%");
    }
}
