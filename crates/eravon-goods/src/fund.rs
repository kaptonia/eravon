use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_standard as a1;
use eravon_terms::u256::U256;

pub type Why = (String, String);

fn a1w(f: a1::fault::Fault) -> Why {
    (f.name(), f.detail())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub fund: String,
    pub bound: bool,
    pub balance: U256,
    pub allowance: U256,
    pub available: U256,
    pub agreement: &'static str,
}

impl State {
    pub fn orders(&self, d: &U256) -> Option<U256> {
        if d.is_zero() { None } else { Some(floor_div(&self.available, d)) }
    }
}

pub fn stands(available: &U256, ds: &[U256]) -> usize {
    let mut left = *available;
    for (i, d) in ds.iter().enumerate() {
        if left.cmp_u(d) == std::cmp::Ordering::Less {
            return i;
        }
        left = left.overflowing_sub(d).0;
    }
    ds.len()
}

fn floor_div(n: &U256, d: &U256) -> U256 {
    let one = U256::from_u64(1);
    let (mut q, mut r) = (U256::from_u64(0), U256::from_u64(0));
    for i in (0..256).rev() {
        let (mut r2, over) = r.overflowing_add(&r);
        if n.bit(i) {
            r2 = r2.overflowing_add(&one).0;
        }
        q = q.overflowing_add(&q).0;
        if over || r2.cmp_u(d) != std::cmp::Ordering::Less {
            r2 = r2.overflowing_sub(d).0;
            q = q.overflowing_add(&one).0;
        }
        r = r2;
    }
    q
}

pub fn state<T: Transport>(t: &T, table: &Table, logic: &str, seller: &str, fund: &str, token: &str) -> Result<State, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let fund = fund.to_ascii_lowercase();
    a1::chain::ahead_fund(t, table, logic, seller, &fund, token);
    let (bound, a0) = if fund.eq_ignore_ascii_case(seller) {
        (true, "agreed")
    } else {
        let r = a1::chain::funds(t, table, logic, seller, &fund).map_err(a1w)?;
        (r.value, r.agreement)
    };
    let b = a1::chain::erc20_balance(t, table, token, &fund).map_err(a1w)?;
    let a = a1::chain::erc20_allowance(t, table, token, &fund, logic).map_err(a1w)?;
    let (balance, allowance) = (U256::from_be(&b.value), U256::from_be(&a.value));
    let available = if balance.cmp_u(&allowance) == std::cmp::Ordering::Less { balance } else { allowance };
    let agreement = if [a0, b.agreement, a.agreement].contains(&"single") { "single" } else { "agreed" };
    Ok(State { fund, bound, balance, allowance, available, agreement })
}

pub const E_GOODS_FUND_SELLER: &str = "E_GOODS_FUND_SELLER";

pub fn setup(logic: &str, token: &str, seller: &str, fund: &str, amount: &U256, allowance: &U256, bound: bool, from: &str) -> Result<Vec<a1::call::Call>, Why> {
    if !from.eq_ignore_ascii_case(fund) {
        return Err(("E_GOODS_FUND_FROM".into(), format!("金库那两笔由金库 {fund} 自己发,不是 {from}")));
    }
    let mut out = Vec::new();
    if allowance != amount {
        if !allowance.is_zero() && !amount.is_zero() {
            out.push(a1::call::erc20_approve(token, logic, &[0u8; 32]).map_err(a1w)?);
        }
        out.push(a1::call::erc20_approve(token, logic, &amount.to_be()).map_err(a1w)?);
    }
    if !bound && !fund.eq_ignore_ascii_case(seller) {
        out.push(a1::call::accept_funding(logic, seller).map_err(a1w)?);
    }
    Ok(out)
}

pub fn binding_from(logic: &str, seller_keys: &[String], from: &str, seller: &str) -> Result<a1::call::Call, Why> {
    if seller_keys.iter().any(|k| k.eq_ignore_ascii_case(from)) {
        return Err((E_GOODS_FUND_SELLER.into(), "卖家钥不发 acceptFunding:它会把卖家钥所持的押金绑给别人的案子;由金库发".into()));
    }
    a1::call::accept_funding(logic, seller).map_err(a1w)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hint {
    LargeDeposit { deposit: U256, amount: U256 },
    NewStandard { logic: String },
}

impl Hint {
    pub fn word(&self) -> &'static str {
        match self {
            Hint::LargeDeposit { .. } => "largeDeposit",
            Hint::NewStandard { .. } => "newStandard",
        }
    }
    pub fn say(&self) -> String {
        match self {
            Hint::LargeDeposit { deposit, amount } => format!("每单押金 {} 不小于货款 {}:开着「允许买家自付 gas 开案」,任一买家开案都从金库拉这笔押金", deposit.to_dec(), amount.to_dec()),
            Hint::NewStandard { logic } => format!("标准已换代(现行逻辑合约 {logic}):金库要对新合约重新授权、重新登记"),
        }
    }
}

pub fn large_deposit(deposit: &U256, amount: &U256, buyer_may_open: bool) -> Option<Hint> {
    (buyer_may_open && deposit.cmp_u(amount) != std::cmp::Ordering::Less).then(|| Hint::LargeDeposit { deposit: *deposit, amount: *amount })
}

pub fn new_standard(doc_logic: &str) -> Option<Hint> {
    let now = eravon_literals::constants::active_standard().logic.to_ascii_lowercase();
    (!doc_logic.eq_ignore_ascii_case(&now)).then_some(Hint::NewStandard { logic: now })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIC: &str = "0x00000000000000000000000000000000000000c1";
    const TOKEN: &str = "0x00000000000000000000000000000000000000c2";
    const SELLER: &str = "0x00000000000000000000000000000000000000c3";
    const FUND: &str = "0x00000000000000000000000000000000000000c4";

    #[test]
    fn the_setup_is_the_vaults_own_two_steps() {
        let n = U256::from_u64(500);
        let sel = |c: &a1::call::Call| c.data[..4].to_vec();
        let approve = a1::sol::ERC20_APPROVE.selector().to_vec();
        let accept = a1::sol::ACCEPT_FUNDING.selector().to_vec();
        let got = setup(LOGIC, TOKEN, SELLER, FUND, &n, &U256::from_u64(0), false, FUND).unwrap();
        assert_eq!(got.iter().map(sel).collect::<Vec<_>>(), vec![approve.clone(), accept.clone()]);
        assert_eq!((got[0].to.as_str(), got[1].to.as_str()), (TOKEN, LOGIC), "直发币与逻辑合约,不经中介");
        let got = setup(LOGIC, TOKEN, SELLER, FUND, &n, &U256::from_u64(7), true, FUND).unwrap();
        assert_eq!(got.iter().map(sel).collect::<Vec<_>>(), vec![approve.clone(), approve.clone()], "先 0 后 n");
        assert!(setup(LOGIC, TOKEN, SELLER, FUND, &n, &n, true, FUND).unwrap().is_empty());
        assert!(setup(LOGIC, TOKEN, SELLER, SELLER, &n, &n, false, SELLER).unwrap().is_empty(), "金库即卖家钥:不登记");
        assert_eq!(setup(LOGIC, TOKEN, SELLER, FUND, &n, &n, false, SELLER).unwrap_err().0, "E_GOODS_FUND_FROM");
        assert_eq!(binding_from(LOGIC, &[SELLER.into()], SELLER, SELLER).unwrap_err().0, E_GOODS_FUND_SELLER);
        assert!(binding_from(LOGIC, &[SELLER.into()], FUND, SELLER).is_ok());
        let s = State { fund: FUND.into(), bound: true, balance: U256::from_u64(1_480), allowance: n, available: n, agreement: "agreed" };
        assert_eq!(s.orders(&U256::from_u64(132)), Some(U256::from_u64(3)));
        assert_eq!(s.orders(&U256::from_u64(0)), None);
        assert_eq!(floor_div(&U256::MAX, &U256::from_u64(1)), U256::MAX);
        assert_eq!(floor_div(&U256::MAX, &U256::MAX), U256::from_u64(1));
        assert_eq!(floor_div(&U256::from_u64(131), &U256::from_u64(132)), U256::from_u64(0));
        assert_eq!(floor_div(&U256::MAX, &U256::from_u64(2)).to_dec(), "57896044618658097711785492504343953926634992332820282019728792003956564819967");
        assert!(large_deposit(&U256::from_u64(10), &U256::from_u64(10), true).is_some());
        assert!(large_deposit(&U256::from_u64(10), &U256::from_u64(10), false).is_none());
        assert!(large_deposit(&U256::from_u64(9), &U256::from_u64(10), true).is_none());
        let u = U256::from_u64;
        assert_eq!(stands(&u(500), &[u(150), u(200), u(150)]), 3);
        assert_eq!(stands(&u(499), &[u(150), u(200), u(150)]), 2);
        assert_eq!(stands(&u(100), &[u(150), u(50)]), 0, "头一单不够就停:开案按次序走,后面的小单不越过它");
        assert_eq!(stands(&u(0), &[]), 0);
    }
}
