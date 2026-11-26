//! §1 hex forms: `hex20` / `hex32` / `hex65` are `0x` plus exactly 40 / 64 / 130 lowercase hex digits;
//! uppercase, a missing prefix or another length is not the form. [`encode`] writes it.

fn is_hex_of(s: &str, bytes: usize) -> bool {
    let b = s.as_bytes();
    b.len() == 2 + 2 * bytes
        && b[0] == b'0'
        && b[1] == b'x'
        && b[2..].iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
}

pub fn is_hex20(s: &str) -> bool {
    is_hex_of(s, 20)
}
pub fn is_hex32(s: &str) -> bool {
    is_hex_of(s, 32)
}
pub fn is_hex65(s: &str) -> bool {
    is_hex_of(s, 65)
}

/// **A hexadecimal text given the `0x` prefix**, the one place a prefix is put on hex that is text already (an entry
/// file's name, a shortened address, a manifest word read back): the text as it is, after `0x`.
pub fn prefixed(bare: &str) -> String {
    let mut s = String::with_capacity(2 + bare.len());
    s.push_str("0x");
    s.push_str(bare);
    s
}

pub fn encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + 2 * bytes.len());
    s.push_str("0x");
    for b in bytes {
        s.push(char::from(b"0123456789abcdef"[(b >> 4) as usize]));
        s.push(char::from(b"0123456789abcdef"[(b & 0xf) as usize]));
    }
    s
}

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// A private key scalar on the command line (HARNESS `sign` `<privkey-hex>`): exactly 64 hex digits of either
/// case, optionally after `0x` or `0X`; anything else is misuse. The range (1 to n − 1) is judged by cryptox.
pub fn scalar32(s: &str) -> Option<[u8; 32]> {
    let b = s.as_bytes();
    let rest = if b.len() >= 2 && b[0] == b'0' && (b[1] == b'x' || b[1] == b'X') { &b[2..] } else { b };
    if rest.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in rest.chunks(2).enumerate() {
        out[i] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(out)
}

/// Hex digits (no prefix, either case) into bytes; an odd count or a non-digit gives None.
pub fn decode_digits(digits: &[u8]) -> Option<Vec<u8>> {
    if digits.len() % 2 != 0 {
        return None;
    }
    digits
        .chunks(2)
        .map(|p| Some((nibble(p[0])? << 4) | nibble(p[1])?))
        .collect()
}

/// `0x` plus an even number of hex digits (either case), as bytes; a missing or uppercase prefix, an odd count or a
/// non-digit gives `None` (the reading the family's shared chain code takes, `galeed_chain`).
pub fn decode(s: &str) -> Option<Vec<u8>> {
    decode_digits(s.strip_prefix("0x")?.as_bytes())
}

/// A `0x`-form's bytes (the two leading characters are passed over); text that does not read gives no bytes, and text
/// shorter than the prefix is no bytes too, never a panic.
pub fn decode_form(s: &str) -> Vec<u8> {
    s.as_bytes().get(2..).and_then(decode_digits).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    /// Text shorter than the prefix is no bytes, never a panic; a whole form reads as its bytes.
    #[test]
    fn a_short_form_is_no_bytes() {
        assert_eq!(super::decode_form(""), Vec::<u8>::new());
        assert_eq!(super::decode_form("0"), Vec::<u8>::new());
        assert_eq!(super::decode_form("0x"), Vec::<u8>::new());
        assert_eq!(super::decode_form("0xab01"), vec![0xab, 0x01]);
    }
}
