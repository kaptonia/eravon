use eravon_galeed as k1;

pub const INT_MAX: u64 = k1::canon::INT_MAX;
pub const U32_MAX: u64 = 4_294_967_295;

pub fn as_int(v: &k1::Value) -> Option<u64> {
    match v {
        k1::Value::Int(n) => Some(*n),
        _ => None,
    }
}

pub fn is_u32(v: &k1::Value) -> bool {
    as_int(v).is_some_and(|n| n <= U32_MAX)
}

pub fn is_int_at_most(v: &k1::Value, bound: u64) -> bool {
    as_int(v).is_some_and(|n| n <= bound)
}

pub fn as_bool(v: &k1::Value) -> Option<bool> {
    match v {
        k1::Value::Bool(b) => Some(*b),
        _ => None,
    }
}

pub fn as_str(v: &k1::Value) -> Option<&str> {
    match v {
        k1::Value::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

pub fn as_obj(v: &k1::Value) -> Option<&[(String, k1::Value)]> {
    match v {
        k1::Value::Obj(m) => Some(m),
        _ => None,
    }
}

pub fn as_arr(v: &k1::Value) -> Option<&[k1::Value]> {
    match v {
        k1::Value::Arr(a) => Some(a.as_slice()),
        _ => None,
    }
}

pub fn is_zero_hex(s: &str) -> bool {
    s.as_bytes()[2..].iter().all(|c| *c == b'0')
}

pub fn is_nonzero_hex20(v: &k1::Value) -> bool {
    as_str(v).is_some_and(|s| k1::hexform::is_hex20(s) && !is_zero_hex(s))
}

pub fn is_nonzero_hex32(v: &k1::Value) -> bool {
    as_str(v).is_some_and(|s| k1::hexform::is_hex32(s) && !is_zero_hex(s))
}

pub fn is_hex32(v: &k1::Value) -> bool {
    as_str(v).is_some_and(k1::hexform::is_hex32)
}

pub fn int_spelling(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.is_empty() || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if b[0] == b'0' && b.len() > 1 {
        return None;
    }
    let n: u64 = s.parse().ok()?;
    (n <= INT_MAX).then_some(n)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pointer {
    pub chain_id: u64,
    pub address: String,
}

fn prefix_cell() -> &'static Option<String> {
    static CELL: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        let mut v = k1::literals::pointer_prefixes()?;
        if v.len() == 1 { v.pop() } else { None }
    })
}

pub fn registry_ok() -> bool {
    prefix_cell().is_some()
}

pub fn pointer(s: &str) -> Option<Pointer> {
    let prefix = prefix_cell().as_deref()?;
    let rest = s.strip_prefix(prefix)?;
    let (chain, addr) = rest.split_once(':')?;
    Some(Pointer { chain_id: int_spelling(chain)?, address: addr.to_string() })
        .filter(|p| k1::hexform::is_hex20(&p.address))
}

pub fn b64url_encode(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(b.len().div_ceil(3) * 4);
    for chunk in b.chunks(3) {
        let n = chunk.len();
        let mut buf = [0u8; 3];
        buf[..n].copy_from_slice(chunk);
        let v = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | buf[2] as u32;
        let take = [2usize, 3, 4][n - 1];
        for k in 0..take {
            out.push(char::from(A[((v >> (18 - 6 * k)) & 0x3f) as usize]));
        }
    }
    out
}

pub fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    if b.len() % 4 == 1 {
        return None;
    }
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    for chunk in b.chunks(4) {
        let n = chunk.len();
        let mut v = 0u32;
        for (k, c) in chunk.iter().enumerate() {
            v |= val(*c)? << (18 - 6 * k);
        }
        let take = match n {
            4 => 3,
            3 => 2,
            2 => 1,
            _ => return None,
        };
        let unused_bits = 6 * n - 8 * take;
        if unused_bits > 0 && (v >> (24 - 6 * n)) & ((1 << unused_bits) - 1) != 0 {
            return None;
        }
        let bytes = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        out.extend_from_slice(&bytes[..take]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_spelling_takes_only_the_laws_spelling() {
        assert_eq!(int_spelling("0"), Some(0));
        assert_eq!(int_spelling("9007199254740991"), Some(INT_MAX));
        assert_eq!(int_spelling("9007199254740992"), None, "越 2^53 − 1");
        assert_eq!(int_spelling("01"), None);
        assert_eq!(int_spelling("-1"), None);
        assert_eq!(int_spelling(""), None);
        assert_eq!(int_spelling("1 "), None);
    }

    #[test]
    fn pointer_reads_the_prefix_from_the_galeed_family() {
        let p = pointer("galeed:guild:1:0x1111111111111111111111111111111111111111").unwrap();
        assert_eq!(p.chain_id, 1);
        assert!(pointer("galeed:guild:01:0x1111111111111111111111111111111111111111").is_none());
        assert!(pointer("guild:1:0x1111111111111111111111111111111111111111").is_none());
        assert!(pointer("galeed:guild:1:0x11").is_none());
    }

    #[test]
    fn base64url_round_trips_and_refuses_the_three_bad_shapes() {
        assert_eq!(b64url_encode(b""), "");
        assert_eq!(b64url_encode(b"f"), "Zg");
        assert_eq!(b64url_encode(b"fo"), "Zm8");
        assert_eq!(b64url_encode(b"foo"), "Zm9v");
        assert_eq!(b64url_encode(&[0xfb, 0xff]), "-_8");
        assert_eq!(b64url_decode("Zm9v").as_deref(), Some(&b"foo"[..]));
        assert_eq!(b64url_decode("Zg").as_deref(), Some(&b"f"[..]));
        assert_eq!(b64url_decode("Zg=="), None, "补位不收");
        assert_eq!(b64url_decode("Z"), None, "长度模 4 余 1");
        assert_eq!(b64url_decode("Zh"), None, "末字符未用位非零");
        assert_eq!(b64url_decode("Zm+v"), None, "标准字母表不收");
    }
}
