#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct U256(pub [u64; 4]);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct U512(pub [u64; 8]);

impl U256 {
    pub const ZERO: U256 = U256([0, 0, 0, 0]);
    pub const ONE: U256 = U256([1, 0, 0, 0]);
    pub const MAX: U256 = U256([u64::MAX; 4]);

    pub const fn from_u64(x: u64) -> U256 {
        U256([x, 0, 0, 0])
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0, 0, 0, 0]
    }

    pub fn from_be(b: &[u8; 32]) -> U256 {
        let mut l = [0u64; 4];
        for i in 0..4 {
            let mut w = [0u8; 8];
            w.copy_from_slice(&b[i * 8..i * 8 + 8]);
            l[3 - i] = u64::from_be_bytes(w);
        }
        U256(l)
    }

    pub fn to_be(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..4 {
            out[i * 8..i * 8 + 8].copy_from_slice(&self.0[3 - i].to_be_bytes());
        }
        out
    }

    pub fn cmp_u(&self, o: &U256) -> core::cmp::Ordering {
        for i in (0..4).rev() {
            match self.0[i].cmp(&o.0[i]) {
                core::cmp::Ordering::Equal => continue,
                x => return x,
            }
        }
        core::cmp::Ordering::Equal
    }

    pub fn overflowing_add(&self, o: &U256) -> (U256, bool) {
        let mut r = [0u64; 4];
        let mut carry = 0u128;
        for i in 0..4 {
            let s = self.0[i] as u128 + o.0[i] as u128 + carry;
            r[i] = s as u64;
            carry = s >> 64;
        }
        (U256(r), carry != 0)
    }

    pub fn overflowing_sub(&self, o: &U256) -> (U256, bool) {
        let mut r = [0u64; 4];
        let mut borrow = 0i128;
        for i in 0..4 {
            let d = self.0[i] as i128 - o.0[i] as i128 - borrow;
            if d < 0 {
                r[i] = (d + (1i128 << 64)) as u64;
                borrow = 1;
            } else {
                r[i] = d as u64;
                borrow = 0;
            }
        }
        (U256(r), borrow != 0)
    }

    pub fn mul_wide(&self, o: &U256) -> U512 {
        let mut r = [0u64; 8];
        for i in 0..4 {
            let mut carry = 0u128;
            for j in 0..4 {
                let t = self.0[i] as u128 * o.0[j] as u128 + r[i + j] as u128 + carry;
                r[i + j] = t as u64;
                carry = t >> 64;
            }
            let mut k = i + 4;
            while carry != 0 && k < 8 {
                let t = r[k] as u128 + carry;
                r[k] = t as u64;
                carry = t >> 64;
                k += 1;
            }
        }
        U512(r)
    }

    pub fn from_dec(s: &str) -> Option<U256> {
        let b = s.as_bytes();
        if b.is_empty() {
            return None;
        }
        if b[0] == b'0' && b.len() > 1 {
            return None;
        }
        let mut v = U256::ZERO;
        let ten = U256::from_u64(10);
        for c in b {
            if !c.is_ascii_digit() {
                return None;
            }
            let w = v.mul_wide(&ten);
            if w.0[4..].iter().any(|x| *x != 0) {
                return None;
            }
            let lo = U256([w.0[0], w.0[1], w.0[2], w.0[3]]);
            let (s2, o) = lo.overflowing_add(&U256::from_u64((c - b'0') as u64));
            if o {
                return None;
            }
            v = s2;
        }
        Some(v)
    }

    pub fn to_dec(&self) -> String {
        if self.is_zero() {
            return String::from("0");
        }
        let mut digits = Vec::new();
        let mut cur = *self;
        while !cur.is_zero() {
            let (q, r) = cur.div_rem_small(10);
            digits.push(b'0' + r as u8);
            cur = q;
        }
        digits.reverse();
        String::from_utf8(digits).expect("十进制数字都是 ASCII")
    }

    pub fn div_rem_small(&self, d: u64) -> (U256, u64) {
        let mut q = [0u64; 4];
        let mut rem = 0u128;
        for i in (0..4).rev() {
            let cur = (rem << 64) | self.0[i] as u128;
            q[i] = (cur / d as u128) as u64;
            rem = cur % d as u128;
        }
        (U256(q), rem as u64)
    }

    pub fn mul_div_small(&self, mul: u64, div: u64) -> Option<U256> {
        let w = self.mul_wide(&U256::from_u64(mul));
        let q = w.div_small(div);
        if q.0[4..].iter().any(|x| *x != 0) {
            return None;
        }
        Some(U256([q.0[0], q.0[1], q.0[2], q.0[3]]))
    }

    pub fn bit(&self, i: usize) -> bool {
        i < 256 && (self.0[i / 64] >> (i % 64)) & 1 == 1
    }
}

impl U512 {
    pub fn div_small(&self, d: u64) -> U512 {
        let mut q = [0u64; 8];
        let mut rem = 0u128;
        for i in (0..8).rev() {
            let cur = (rem << 64) | self.0[i] as u128;
            q[i] = (cur / d as u128) as u64;
            rem = cur % d as u128;
        }
        U512(q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec_round_trips_and_refuses_the_two_bad_spellings() {
        assert_eq!(U256::from_dec("0").unwrap().to_dec(), "0");
        assert_eq!(U256::from_dec("12345678901234567890").unwrap().to_dec(), "12345678901234567890");
        let max = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
        assert_eq!(U256::from_dec(max).unwrap(), U256::MAX);
        assert_eq!(U256::from_dec(max).unwrap().to_dec(), max);
        let over = "115792089237316195423570985008687907853269984665640564039457584007913129639936";
        assert!(U256::from_dec(over).is_none());
        assert!(U256::from_dec("01").is_none(), "前导零不是 dec256");
        assert!(U256::from_dec("").is_none());
        assert!(U256::from_dec("-1").is_none());
        assert!(U256::from_dec("1e3").is_none());
    }

    #[test]
    fn word_is_thirty_two_bytes_big_endian() {
        assert_eq!(U256::from_u64(1).to_be()[31], 1);
        assert_eq!(U256::MAX.to_be(), [0xffu8; 32]);
        assert_eq!(U256::from_be(&U256::MAX.to_be()), U256::MAX);
    }

    #[test]
    fn mul_div_small_floors_and_names_the_overflow() {
        assert_eq!(U256::MAX.mul_div_small(10000, 10000), Some(U256::MAX));
        assert_eq!(U256::from_u64(7).mul_div_small(3, 2), Some(U256::from_u64(10)));
        assert_eq!(U256::MAX.mul_div_small(2, 1), None);
    }
}
