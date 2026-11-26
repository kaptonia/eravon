use crate::{Code, Trouble, Why};

pub fn funds_check(need: u128, have: u128) -> Result<(), Trouble> {
    if have < need {
        return Err(Trouble::new(Code::InsufficientFunds, Why::Funds { need, have }));
    }
    Ok(())
}

pub(crate) fn with_left(t: Trouble, left: &[Why]) -> Trouble {
    if left.is_empty() {
        return t;
    }
    let mut all = match t.why.clone() {
        Why::Each(xs) => xs,
        other => vec![other],
    };
    all.extend(left.iter().cloned());
    t.retold(Why::Each(all))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_balance_gate_compares_at_its_edges() {
        assert!(funds_check(0, 0).is_ok());
        assert!(funds_check(10, 10).is_ok());
        assert_eq!(funds_check(10, 9), Err(Trouble::new(Code::InsufficientFunds, Why::Funds { need: 10, have: 9 })));
        assert!(funds_check(u128::MAX, u128::MAX).is_ok());
        assert_eq!(funds_check(u128::MAX, u128::MAX - 1).map_err(|t| t.code), Err(Code::InsufficientFunds));
    }
}
