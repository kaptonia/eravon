use eravon_galeed as k1;

use crate::vfault::{Fault, R};

fn shape_ok(s: &str) -> bool {
    let body = s.strip_prefix("0x").unwrap_or(s);
    body.len() == 40 && body.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn eip55(address: &str) -> R<String> {
    let body = address.strip_prefix("0x").unwrap_or(address).to_ascii_lowercase();
    if body.len() != 40 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Fault::Address(format!("{address} 不是四十位十六进制")));
    }
    let h = k1::hash::keccak256(body.as_bytes());
    let mut out = String::with_capacity(42);
    out.push_str("0x");
    for (i, c) in body.chars().enumerate() {
        let nibble = if i % 2 == 0 { h[i / 2] >> 4 } else { h[i / 2] & 0x0f };
        if c.is_ascii_digit() || nibble < 8 {
            out.push(c);
        } else {
            out.push(c.to_ascii_uppercase());
        }
    }
    Ok(out)
}

pub fn accept(address: &str) -> R<String> {
    if !shape_ok(address) {
        return Err(Fault::Address(format!("{address} 不是四十位十六进制")));
    }
    let body = address.strip_prefix("0x").unwrap_or(address);
    let mixed = body.chars().any(|c| c.is_ascii_uppercase()) && body.chars().any(|c| c.is_ascii_lowercase());
    if mixed {
        let want = eip55(address)?;
        if want.trim_start_matches("0x") != body {
            return Err(Fault::Address(format!(
                "{address} 的 EIP-55 校验和不对;这一枚地址的正确写法是 {want}"
            )));
        }
    }
    Ok(format!("0x{}", body.to_ascii_lowercase()))
}


pub fn set(data: &str, who: &str, address: &str, again: &str) -> R<String> {
    if address != again {
        return Err(Fault::Address("两次输入不一致".into()));
    }
    let ok = accept(address)?;
    if ok.eq_ignore_ascii_case(who) && crate::records::seat_of(who).is_ok_and(|(_, r)| crate::vault::seat_of(r) == eravon_store::vault::Seat::Seller) {
        return Err(Fault::SellerKey("收款地址不能是卖家钥:填别的钱包地址".into()));
    }
    let mut s = crate::store::read_identity(data, who).map_err(Fault::from)?;
    s.payout = ok.clone();
    crate::store::write_identity(data, who, &s).map_err(Fault::from)?;
    Ok(ok)
}

pub fn get(data: &str, who: &str) -> R<String> {
    Ok(crate::store::read_identity(data, who).map_err(Fault::from)?.payout)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASES: [&str; 4] = [
        "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
        "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
        "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
        "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
    ];

    #[test]
    fn the_four_addresses_the_law_itself_prints_come_back_verbatim() {
        for c in CASES {
            assert_eq!(eip55(c).unwrap(), c, "EIP-55 本文那一枚");
            assert!(accept(c).is_ok(), "带校验和的照收");
        }
    }

    #[test]
    fn all_lower_and_all_upper_carry_no_checksum_so_both_are_taken() {
        let lower = "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed";
        let upper = "0x5AAEB6053F3E94C9B9A09F33669435E7EF1BEAED";
        assert_eq!(accept(lower).unwrap(), lower);
        assert_eq!(accept(upper).unwrap(), lower, "答的一律是小写形");
    }

    #[test]
    fn one_letter_typed_wrong_in_a_mixed_case_address_is_refused_by_name() {
        let bad = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAEd";
        let e = accept(bad).unwrap_err();
        assert_eq!(e.name(), "E_VAULT_ADDRESS");
        assert!(e.detail().contains(CASES[0]), "拒的时候把正确写法摆出来:{}", e.detail());
    }

    #[test]
    fn a_thing_that_is_not_forty_hex_digits_is_refused_before_the_checksum() {
        for bad in ["0x123", "", "0xZZAeb6053F3E94C9b9A09f33669435E7Ef1BeAed", "5aaeb6053f3e94c9b9a09f33669435e7ef1beae"] {
            assert_eq!(accept(bad).unwrap_err().name(), "E_VAULT_ADDRESS", "{bad}");
        }
    }
}
