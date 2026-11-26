use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_terms as k2;

use crate::abi;
use crate::call;
use crate::chain;
use crate::fault::{Fault, R};
use crate::order;
use crate::precheck::GATE_PRICE;

#[derive(Clone, Debug)]
pub struct Round {
    pub round_id: [u8; 32],
    pub answer: String,
    pub started_at: u64,
    pub updated_at: u64,
    pub answered_in_round: [u8; 32],
    pub decimals: u64,
    pub agreement: &'static str,
}

fn int256_dec(w: &[u8; 32]) -> String {
    if w[0] & 0x80 == 0 {
        return k2::u256::U256::from_be(w).to_dec();
    }
    let mut n = [0u8; 32];
    for i in 0..32 {
        n[i] = !w[i];
    }
    let mut i = 31usize;
    loop {
        let (v, carry) = n[i].overflowing_add(1);
        n[i] = v;
        if !carry || i == 0 {
            break;
        }
        i -= 1;
    }
    format!("-{}", k2::u256::U256::from_be(&n).to_dec())
}

#[derive(Clone, Debug)]
pub enum Seen {
    Round(Round),
    Anomaly(String),
}

const DECIMALS_MAX: u64 = 36;
const ROUND_BYTES: usize = 160;

fn reverted(f: &Fault) -> bool {
    crate::chain::reverted(f)
}

pub fn observe<T: Transport>(t: &T, table: &Table, at: &str) -> R<Seen> {
    let _ = order::addr_of("价源", at)?;
    let r = match chain::call_raw(t, table, at, &call::feed_latest_round()?) {
        Ok(r) => r,
        Err(f) if reverted(&f) => return Ok(Seen::Anomaly(format!("价源 {at} 的 latestRoundData 回滚了"))),
        Err(f) => return Err(f),
    };
    if r.value.len() < ROUND_BYTES {
        let why = if r.value.is_empty() {
            format!("价源 {at} 上没有码(latestRoundData 一个字节也没答)")
        } else {
            format!("价源 {at} 的 latestRoundData 只答了 {} 字节,一轮该有 {ROUND_BYTES}", r.value.len())
        };
        return Ok(Seen::Anomaly(why));
    }
    let v = abi::decode(&abi::tuple_types(crate::sol::src_abi::I_PRICE_FEED_LATEST_ROUND_DATA_RETURNS)?, &r.value)?;
    use crate::sol::src_abi::{
        I_PRICE_FEED_LATEST_ROUND_DATA_COL_ANSWER as ANSWER, I_PRICE_FEED_LATEST_ROUND_DATA_COL_ANSWERED_IN_ROUND as ANSWERED_IN_ROUND,
        I_PRICE_FEED_LATEST_ROUND_DATA_COL_ROUND_ID as ROUND_ID, I_PRICE_FEED_LATEST_ROUND_DATA_COL_STARTED_AT as STARTED_AT,
        I_PRICE_FEED_LATEST_ROUND_DATA_COL_UPDATED_AT as UPDATED_AT,
    };
    for (i, name) in [(ROUND_ID, "roundId"), (ANSWERED_IN_ROUND, "answeredInRound")] {
        let w = v[i].word()?;
        if w[..22].iter().any(|b| *b != 0) {
            return Ok(Seen::Anomaly(format!("价源 {at} 的 {name} 是 {},超出 uint80", k2::u256::U256::from_be(&w).to_dec())));
        }
    }
    let updated = v[UPDATED_AT].word()?;
    if updated[..24].iter().any(|b| *b != 0) {
        return Ok(Seen::Anomaly(format!(
            "价源 {at} 的 updatedAt 是 {},超出 2^64",
            k2::u256::U256::from_be(&updated).to_dec()
        )));
    }
    let d = match chain::call_raw(t, table, at, &call::feed_decimals()?) {
        Ok(d) => d,
        Err(f) if reverted(&f) => return Ok(Seen::Anomaly(format!("价源 {at} 的 decimals 回滚了"))),
        Err(f) => return Err(f),
    };
    if d.value.len() < 32 {
        return Ok(Seen::Anomaly(format!("价源 {at} 的 decimals 只答了 {} 字节", d.value.len())));
    }
    let dw = abi::decode(&abi::tuple_types(crate::sol::src_abi::I_PRICE_FEED_DECIMALS_RETURNS)?, &d.value)?[0].word()?;
    let too_big = dw[..24].iter().any(|b| *b != 0) || u64::from_be_bytes(dw[24..].try_into().unwrap_or([0xff; 8])) > DECIMALS_MAX;
    if too_big {
        return Ok(Seen::Anomaly(format!(
            "价源 {at} 的 decimals 是 {},超出 {DECIMALS_MAX}",
            k2::u256::U256::from_be(&dw).to_dec()
        )));
    }
    Ok(Seen::Round(Round {
        round_id: v[ROUND_ID].word()?,
        answer: int256_dec(&v[ANSWER].word()?),
        started_at: v[STARTED_AT].small().unwrap_or(u64::MAX),
        updated_at: v[UPDATED_AT].small()?,
        answered_in_round: v[ANSWERED_IN_ROUND].word()?,
        decimals: u64::from_be_bytes(dw[24..].try_into().unwrap_or([0; 8])),
        agreement: r.agreement,
    }))
}

pub fn latest<T: Transport>(t: &T, table: &Table, at: &str) -> R<Round> {
    match observe(t, table, at)? {
        Seen::Round(r) => Ok(r),
        Seen::Anomaly(why) => Err(Fault::Abi(why)),
    }
}

pub fn gate<T: Transport>(t: &T, table: &Table, at: &str, now: u64, max_stale: u64) -> R<Round> {
    match observe(t, table, at)? {
        Seen::Anomaly(why) => Err(Fault::Precheck {
            gate: GATE_PRICE,
            why: format!("{why}:按陈旧算,链上那一步会以 RateStale 回滚"),
        }),
        Seen::Round(r) => {
            fresh(&r, now, max_stale)?;
            Ok(r)
        }
    }
}

pub fn fresh(r: &Round, now: u64, max_stale: u64) -> R<()> {
    stale_gate(&r.answer, r.updated_at, now, max_stale)
}

fn positive(answer: &str) -> bool {
    !answer.starts_with('-') && answer.bytes().any(|b| b.is_ascii_digit() && b != b'0')
}

fn stale_gate(answer: &str, updated_at: u64, now: u64, max_stale: u64) -> R<()> {
    if !positive(answer) {
        return Err(Fault::Precheck {
            gate: GATE_PRICE,
            why: format!("价源答数 {answer} 不是正数:按陈旧算,链上那一步会以 RateStale 回滚"),
        });
    }
    let age = now.saturating_sub(updated_at);
    if age > max_stale {
        return Err(Fault::Precheck {
            gate: GATE_PRICE,
            why: format!(
                "价源最后更新在 {updated_at},此刻是 {now}(旧了 {age} 秒),这一家给的上限是 {max_stale} 秒"
            ),
        });
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minus_one_reads_as_minus_one() {
        let mut w = [0xffu8; 32];
        assert_eq!(int256_dec(&w), "-1");
        w[31] = 0xfe;
        assert_eq!(int256_dec(&w), "-2");
        let mut z = [0u8; 32];
        z[31] = 7;
        assert_eq!(int256_dec(&z), "7");
        assert_eq!(int256_dec(&[0u8; 32]), "0");
    }

    #[test]
    fn exactly_at_the_limit_is_still_fresh() {
        assert!(stale_gate("7", 1000, 1000 + 59, 60).is_ok());
        assert!(stale_gate("7", 1000, 1000 + 60, 60).is_ok());
        assert!(stale_gate("7", 1000, 1000 + 61, 60).is_err());
    }

    #[test]
    fn non_positive_answer_is_stale() {
        assert!(stale_gate("0", 1000, 1000, 60).is_err());
        assert!(stale_gate("-1", 1000, 1000, 60).is_err());
        assert!(stale_gate("000", 1000, 1000, 60).is_err());
        assert!(stale_gate("1", 1000, 1000, 60).is_ok());
    }

    #[test]
    fn future_feed_time_counts_as_age_zero() {
        assert!(stale_gate("7", 2000, 1000, 60).is_ok());
        assert!(stale_gate("7", 2000, 1000, 0).is_ok());
    }
}
