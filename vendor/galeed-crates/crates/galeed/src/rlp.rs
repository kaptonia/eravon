//! RLP reading and writing: block headers, transactions, receipts, accounts and MPT nodes in an
//! evidence kit (§9.6) all use it. Form only: length prefixes and nesting, no field semantics.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rlp<'a> {
    Bytes(&'a [u8]),
    List(Vec<Rlp<'a>>),
}

impl<'a> Rlp<'a> {
    pub fn bytes(&self) -> Option<&'a [u8]> {
        match self {
            Rlp::Bytes(b) => Some(b),
            Rlp::List(_) => None,
        }
    }
    pub fn list(&self) -> Option<&[Rlp<'a>]> {
        match self {
            Rlp::List(l) => Some(l),
            Rlp::Bytes(_) => None,
        }
    }
    /// A big-endian minimal integer as u64; more than 8 bytes or a leading zero gives None.
    pub fn uint(&self) -> Option<u64> {
        let b = self.bytes()?;
        if b.len() > 8 || (b.len() > 1 && b[0] == 0) {
            return None;
        }
        Some(b.iter().fold(0u64, |acc, x| (acc << 8) | u64::from(*x)))
    }
}

/// Nesting limit. Headers, transactions, receipts, accounts and MPT nodes are a few levels deep;
/// deeper input is malformed. Unbounded recursion on foreign bytes would overflow the stack.
pub const DEPTH_MAX: usize = 64;

/// One complete RLP item; trailing bytes give None.
pub fn decode(b: &[u8]) -> Option<Rlp<'_>> {
    let (item, rest) = decode_one(b)?;
    if rest.is_empty() { Some(item) } else { None }
}

pub fn decode_one(b: &[u8]) -> Option<(Rlp<'_>, &[u8])> {
    decode_at(b, 0)
}

fn decode_at(b: &[u8], depth: usize) -> Option<(Rlp<'_>, &[u8])> {
    if depth > DEPTH_MAX {
        return None;
    }
    let first = *b.first()?;
    let (payload, rest, is_list) = match first {
        0x00..=0x7f => return Some((Rlp::Bytes(&b[..1]), &b[1..])),
        0x80..=0xb7 => {
            let n = usize::from(first - 0x80);
            let p = b.get(1..1 + n)?;
            if n == 1 && p[0] < 0x80 {
                return None;
            }
            (p, &b[1 + n..], false)
        }
        0xb8..=0xbf => {
            let ll = usize::from(first - 0xb7);
            let (n, after) = len_of(b, ll)?;
            if n < 56 {
                return None;
            }
            let end = after.checked_add(n)?;
            (b.get(after..end)?, &b[end..], false)
        }
        0xc0..=0xf7 => {
            let n = usize::from(first - 0xc0);
            (b.get(1..1 + n)?, &b[1 + n..], true)
        }
        0xf8..=0xff => {
            let ll = usize::from(first - 0xf7);
            let (n, after) = len_of(b, ll)?;
            if n < 56 {
                return None;
            }
            let end = after.checked_add(n)?;
            (b.get(after..end)?, &b[end..], true)
        }
    };
    if !is_list {
        return Some((Rlp::Bytes(payload), rest));
    }
    let mut items = Vec::new();
    let mut cur = payload;
    while !cur.is_empty() {
        let (it, r) = decode_at(cur, depth + 1)?;
        items.push(it);
        cur = r;
    }
    Some((Rlp::List(items), rest))
}

/// Long-form length field: `ll` bytes of big-endian integer without a leading zero. Answers
/// (length, payload start).
fn len_of(b: &[u8], ll: usize) -> Option<(usize, usize)> {
    let lb = b.get(1..1 + ll)?;
    if lb[0] == 0 || ll > 8 {
        return None;
    }
    let n = lb.iter().fold(0usize, |acc, x| (acc << 8) | usize::from(*x));
    Some((n, 1 + ll))
}

fn put_len(out: &mut Vec<u8>, n: usize, short: u8, long: u8) {
    if n < 56 {
        out.push(short + n as u8);
    } else {
        let be = n.to_be_bytes();
        let lead = be.iter().position(|x| *x != 0).unwrap_or(be.len() - 1);
        out.push(long + (be.len() - lead) as u8);
        out.extend_from_slice(&be[lead..]);
    }
}

pub fn encode(item: &Rlp<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    write(item, &mut out);
    out
}

fn write(item: &Rlp<'_>, out: &mut Vec<u8>) {
    match item {
        Rlp::Bytes(b) if b.len() == 1 && b[0] < 0x80 => out.push(b[0]),
        Rlp::Bytes(b) => {
            put_len(out, b.len(), 0x80, 0xb7);
            out.extend_from_slice(b);
        }
        Rlp::List(l) => {
            let mut body = Vec::new();
            for x in l {
                write(x, &mut body);
            }
            put_len(out, body.len(), 0xc0, 0xf7);
            out.extend_from_slice(&body);
        }
    }
}

/// A 32-byte big-endian integer without leading zeros (how r and s of a signature are written in a
/// transaction); zero is the empty string.
pub fn uint_bytes_of(b: &[u8; 32]) -> Vec<u8> {
    match b.iter().position(|x| *x != 0) {
        Some(i) => b[i..].to_vec(),
        None => Vec::new(),
    }
}

/// RLP byte-string form of an integer: 0 is empty, anything else big-endian minimal.
pub fn uint_bytes(n: u64) -> Vec<u8> {
    let be = n.to_be_bytes();
    match be.iter().position(|x| *x != 0) {
        Some(i) => be[i..].to_vec(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let v = Rlp::List(vec![Rlp::Bytes(b"cat"), Rlp::List(vec![Rlp::Bytes(b""), Rlp::Bytes(&[0x7f])]), Rlp::Bytes(&[0u8; 60])]);
        let e = encode(&v);
        assert_eq!(decode(&e), Some(v));
        assert_eq!(encode(&Rlp::Bytes(&[])), vec![0x80]);
        assert_eq!(encode(&Rlp::Bytes(&[0x00])), vec![0x00]);
        assert_eq!(uint_bytes(0), Vec::<u8>::new());
        assert_eq!(uint_bytes(1024), vec![0x04, 0x00]);
        assert_eq!(decode(&[0x81, 0x05]), None, "单字节 < 0x80 不得用短串形");
        assert_eq!(decode(&[0xc1, 0x80, 0x00]), None, "尾上多字节");
        assert_eq!(decode(&[0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]), None, "长度溢出不溢");
        fn nest(n: usize) -> Vec<u8> {
            let mut cur = vec![0xc0u8];
            for _ in 0..n {
                let mut out = Vec::new();
                put_len(&mut out, cur.len(), 0xc0, 0xf7);
                out.extend_from_slice(&cur);
                cur = out;
            }
            cur
        }
        assert!(decode(&nest(DEPTH_MAX)).is_some(), "恰在上限可读");
        assert_eq!(decode(&nest(DEPTH_MAX + 1)), None, "过一层即形坏");
    }
}
