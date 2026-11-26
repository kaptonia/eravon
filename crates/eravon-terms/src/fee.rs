use crate::quote::Quote;
use crate::token::{Fault, Token};
use crate::u256::U256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fee {
    pub premium: U256,
    pub lf: Option<U256>,
}

pub fn fee(q: &Quote, amount: &U256) -> Result<Fee, Fault> {
    let premium = amount
        .mul_div_small(q.premium_bps, 10000)
        .ok_or_else(|| Fault::new(Token::FeeOverflow))?;
    if !q.fee_unit_is_zero() {
        return Ok(Fee { premium, lf: None });
    }
    let part = amount.mul_div_small(q.fee_bps, 10000).ok_or_else(|| Fault::new(Token::FeeOverflow))?;
    let (mut lf, over) = q.fee_fixed.overflowing_add(&part);
    if over {
        return Err(Fault::new(Token::FeeOverflow));
    }
    if !q.fee_min.is_zero() && lf.cmp_u(&q.fee_min) == core::cmp::Ordering::Less {
        lf = q.fee_min;
    }
    if !q.fee_max.is_zero() && lf.cmp_u(&q.fee_max) == core::cmp::Ordering::Greater {
        lf = q.fee_max;
    }
    if premium.overflowing_add(&lf).1 {
        return Err(Fault::new(Token::FeeOverflow));
    }
    Ok(Fee { premium, lf: Some(lf) })
}

