use crate::fault::{Fault, R};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Val {
    Addr([u8; 20]),
    U256([u8; 32]),
    Bool(bool),
    B32([u8; 32]),
    Bytes(Vec<u8>),
    Str(String),
    Arr(Vec<Val>),
    Tuple(Vec<Val>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    Addr,
    Uint,
    Bool,
    B32,
    Bytes,
    Str,
    Arr(Box<Ty>),
    Fixed(Box<Ty>, usize),
    Tuple(Vec<Ty>),
}

impl Val {
    pub fn u64(n: u64) -> Val {
        let mut w = [0u8; 32];
        w[24..].copy_from_slice(&n.to_be_bytes());
        Val::U256(w)
    }
    pub fn dynamic(&self) -> bool {
        match self {
            Val::Bytes(_) | Val::Str(_) | Val::Arr(_) => true,
            Val::Tuple(v) => v.iter().any(Val::dynamic),
            _ => false,
        }
    }
}

impl Ty {
    fn dynamic(&self) -> bool {
        match self {
            Ty::Bytes | Ty::Str | Ty::Arr(_) => true,
            Ty::Tuple(v) => v.iter().any(Ty::dynamic),
            Ty::Fixed(t, _) => t.dynamic(),
            _ => false,
        }
    }
}

fn word_of(n: usize) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[24..].copy_from_slice(&(n as u64).to_be_bytes());
    w
}

pub fn args_of(sig: &str) -> R<Vec<Ty>> {
    let open = sig.find('(').ok_or_else(|| Fault::Abi(format!("{sig} 不成一条签名")))?;
    tuple_types(&sig[open..])
}

pub fn tuple_types(list: &str) -> R<Vec<Ty>> {
    let s = list.trim();
    let body = s
        .strip_prefix('(')
        .and_then(|x| x.strip_suffix(')'))
        .ok_or_else(|| Fault::Abi(format!("{list} 不是一张括起来的类型表")))?;
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in body.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(ty_of(&cur)?);
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(ty_of(&cur)?);
    }
    Ok(out)
}

fn members(list: &str) -> R<Vec<String>> {
    let s = list.trim();
    let body = s.strip_prefix('(').and_then(|x| x.strip_suffix(')')).ok_or_else(|| Fault::Abi(format!("{list} 不是一张括起来的类型表")))?;
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in body.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            ',' if depth == 0 => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

pub fn fits(list: &str, vals: &[Val]) -> R<()> {
    let ms = members(list)?;
    if ms.len() != vals.len() {
        return Err(Fault::Abi(format!("{list} 有 {} 位,值有 {} 个", ms.len(), vals.len())));
    }
    for (i, (m, v)) in ms.iter().zip(vals).enumerate() {
        fits_one(m.trim(), v).map_err(|why| Fault::Abi(format!("第 {i} 位 {m}:{why}")))?;
    }
    Ok(())
}

fn fits_one(t: &str, v: &Val) -> Result<(), String> {
    if let Some(inner) = t.strip_suffix("[]") {
        return match v {
            Val::Arr(items) => items.iter().try_for_each(|x| fits_one(inner, x)),
            _ => Ok(()),
        };
    }
    if let Some(open) = t.strip_suffix(']').and_then(|x| x.rfind('[')) {
        return match v {
            Val::Arr(items) => items.iter().try_for_each(|x| fits_one(&t[..open], x)),
            _ => Ok(()),
        };
    }
    if t.starts_with('(') {
        return match v {
            Val::Tuple(items) => fits(t, items).map_err(|f| f.detail()),
            _ => Ok(()),
        };
    }
    let word = match v {
        Val::U256(w) | Val::B32(w) => *w,
        Val::Bool(_) | Val::Addr(_) | Val::Bytes(_) | Val::Str(_) | Val::Arr(_) | Val::Tuple(_) => return Ok(()),
    };
    if let Some(bits) = t.strip_prefix("uint") {
        let n: usize = bits.parse().map_err(|_| format!("{t} 不是一种整数宽"))?;
        let free = (256 - n) / 8;
        return if word[..free].iter().all(|b| *b == 0) { Ok(()) } else { Err(format!("值超出 {t} 的字宽")) };
    }
    if t == "bool" {
        return if word[..31].iter().all(|b| *b == 0) && word[31] <= 1 { Ok(()) } else { Err("值不是 0 或 1".into()) };
    }
    Ok(())
}

fn ty_of(s: &str) -> R<Ty> {
    let s = s.trim();
    if let Some(inner) = s.strip_suffix("[]") {
        return Ok(Ty::Arr(Box::new(ty_of(inner)?)));
    }
    if let Some(open) = s.strip_suffix(']').and_then(|x| x.rfind('[')) {
        let n: usize = s[open + 1..s.len() - 1].parse().map_err(|_| Fault::Abi(format!("定长数组 {s} 的长度读不成")))?;
        let inner = ty_of(&s[..open])?;
        if inner.dynamic() {
            return Err(Fault::Abi(format!("动态件的定长数组 {s} 不在本层的读法里")));
        }
        return Ok(Ty::Fixed(Box::new(inner), n));
    }
    if s.starts_with('(') && s.ends_with(')') {
        return Ok(Ty::Tuple(tuple_types(s)?));
    }
    Ok(match s {
        "address" => Ty::Addr,
        "bool" => Ty::Bool,
        "bytes32" => Ty::B32,
        t if t.strip_prefix("bytes").and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..32).contains(&n)) => Ty::B32,
        "bytes" => Ty::Bytes,
        "string" => Ty::Str,
        t if t.starts_with("uint") || t.starts_with("int") => Ty::Uint,
        other => return Err(Fault::Abi(format!("不认得的 ABI 类型 {other:?}"))),
    })
}

pub fn encode(vals: &[Val]) -> Vec<u8> {
    let mut head = Vec::new();
    let mut tail = Vec::new();
    let head_len: usize = vals.iter().map(head_words).sum::<usize>() * 32;
    for v in vals {
        if v.dynamic() {
            head.extend_from_slice(&word_of(head_len + tail.len()));
            tail.extend_from_slice(&encode_one(v));
        } else {
            head.extend_from_slice(&encode_one(v));
        }
    }
    head.extend_from_slice(&tail);
    head
}

fn head_words(v: &Val) -> usize {
    if v.dynamic() {
        1
    } else {
        match v {
            Val::Tuple(c) => c.iter().map(head_words).sum(),
            _ => 1,
        }
    }
}

fn encode_one(v: &Val) -> Vec<u8> {
    match v {
        Val::Addr(a) => {
            let mut w = [0u8; 32];
            w[12..].copy_from_slice(a);
            w.to_vec()
        }
        Val::U256(w) | Val::B32(w) => w.to_vec(),
        Val::Bool(b) => {
            let mut w = [0u8; 32];
            w[31] = u8::from(*b);
            w.to_vec()
        }
        Val::Bytes(b) => {
            let mut out = word_of(b.len()).to_vec();
            out.extend_from_slice(b);
            let pad = (32 - b.len() % 32) % 32;
            out.extend(std::iter::repeat_n(0u8, pad));
            out
        }
        Val::Str(s) => encode_one(&Val::Bytes(s.as_bytes().to_vec())),
        Val::Arr(items) => {
            let mut out = word_of(items.len()).to_vec();
            out.extend_from_slice(&encode(items));
            out
        }
        Val::Tuple(items) => encode(items),
    }
}

fn word(b: &[u8], at: usize) -> R<[u8; 32]> {
    let end = at
        .checked_add(32)
        .ok_or_else(|| Fault::Abi(format!("第 {at} 个字节起的偏移超出可寻的界")))?;
    if end > b.len() {
        return Err(Fault::Abi(format!("要第 {at} 个字节起的一个字,而这一段只有 {} 字节", b.len())));
    }
    let mut w = [0u8; 32];
    w.copy_from_slice(&b[at..at + 32]);
    Ok(w)
}

fn as_usize(w: &[u8; 32]) -> R<usize> {
    if w[..24].iter().any(|x| *x != 0) {
        return Err(Fault::Abi("长度或偏移超出 usize 的界".into()));
    }
    Ok(u64::from_be_bytes(w[24..].try_into().expect("八字节")) as usize)
}

pub fn decode(tys: &[Ty], data: &[u8]) -> R<Vec<Val>> {
    let mut out = Vec::with_capacity(tys.len());
    let mut at = 0usize;
    for t in tys {
        if t.dynamic() {
            let off = as_usize(&word(data, at)?)?;
            out.push(decode_one(t, data, off)?);
            at += 32;
        } else {
            let (v, used) = decode_static(t, data, at)?;
            out.push(v);
            at += used;
        }
    }
    Ok(out)
}

fn decode_static(t: &Ty, data: &[u8], at: usize) -> R<(Val, usize)> {
    Ok(match t {
        Ty::Addr => {
            let w = word(data, at)?;
            if w[..12].iter().any(|x| *x != 0) {
                return Err(Fault::Abi("地址字的高十二字节该是零".into()));
            }
            (Val::Addr(w[12..].try_into().expect("二十字节")), 32)
        }
        Ty::Uint => (Val::U256(word(data, at)?), 32),
        Ty::B32 => (Val::B32(word(data, at)?), 32),
        Ty::Bool => {
            let w = word(data, at)?;
            if w[..31].iter().any(|x| *x != 0) || w[31] > 1 {
                return Err(Fault::Abi("真假字只许是 0 或 1".into()));
            }
            (Val::Bool(w[31] == 1), 32)
        }
        Ty::Tuple(cs) => {
            let mut vals = Vec::with_capacity(cs.len());
            let mut used = 0usize;
            for c in cs {
                let (v, n) = decode_static(c, data, at + used)?;
                vals.push(v);
                used += n;
            }
            (Val::Tuple(vals), used)
        }
        Ty::Fixed(inner, n) => {
            let mut vals = Vec::with_capacity((*n).min(data.len() / 32 + 1));
            let mut used = 0usize;
            for _ in 0..*n {
                let (v, k) = decode_static(inner, data, at + used)?;
                vals.push(v);
                used += k;
            }
            (Val::Arr(vals), used)
        }
        Ty::Bytes | Ty::Str | Ty::Arr(_) => {
            return Err(Fault::Abi("动态形不走静态那一路".into()));
        }
    })
}

fn decode_one(t: &Ty, data: &[u8], at: usize) -> R<Val> {
    Ok(match t {
        Ty::Bytes | Ty::Str => {
            let n = as_usize(&word(data, at)?)?;
            let head = at
                .checked_add(32)
                .ok_or_else(|| Fault::Abi("变长段的偏移超出可寻的界".into()))?;
            let end = head
                .checked_add(n)
                .ok_or_else(|| Fault::Abi(format!("变长段说有 {n} 字节,加起来超出可寻的界")))?;
            if end > data.len() {
                return Err(Fault::Abi(format!("变长段说有 {n} 字节,而这一段装不下")));
            }
            let b = data[head..end].to_vec();
            if matches!(t, Ty::Str) {
                Val::Str(
                    String::from_utf8(b).map_err(|_| Fault::Abi("文本段不是 UTF-8".into()))?,
                )
            } else {
                Val::Bytes(b)
            }
        }
        Ty::Arr(inner) => {
            let n = as_usize(&word(data, at)?)?;
            let head = at
                .checked_add(32)
                .ok_or_else(|| Fault::Abi("数组头的偏移超出可寻的界".into()))?;
            if head > data.len() {
                return Err(Fault::Abi(format!(
                    "数组头在第 {head} 个字节,而这一段只有 {} 字节",
                    data.len()
                )));
            }
            let block = &data[head..];
            if n > block.len() / 32 {
                return Err(Fault::Abi(format!(
                    "数组说有 {n} 件,而这一段只剩 {} 字节,装不下",
                    block.len()
                )));
            }
            let mut items = Vec::with_capacity(n);
            let mut cur = 0usize;
            for _ in 0..n {
                if inner.dynamic() {
                    let off = as_usize(&word(block, cur)?)?;
                    items.push(decode_one(inner, block, off)?);
                    cur += 32;
                } else {
                    let (v, used) = decode_static(inner, block, cur)?;
                    items.push(v);
                    cur += used;
                }
            }
            Val::Arr(items)
        }
        Ty::Tuple(cs) => {
            if at > data.len() {
                return Err(Fault::Abi(format!(
                    "元组在第 {at} 个字节起,而这一段只有 {} 字节",
                    data.len()
                )));
            }
            let block = &data[at..];
            let mut vals = Vec::with_capacity(cs.len());
            let mut cur = 0usize;
            for c in cs {
                if c.dynamic() {
                    let off = as_usize(&word(block, cur)?)?;
                    vals.push(decode_one(c, block, off)?);
                    cur += 32;
                } else {
                    let (v, used) = decode_static(c, block, cur)?;
                    vals.push(v);
                    cur += used;
                }
            }
            Val::Tuple(vals)
        }
        _ => decode_static(t, data, at)?.0,
    })
}

impl Val {
    pub fn addr(&self) -> R<[u8; 20]> {
        match self {
            Val::Addr(a) => Ok(*a),
            _ => Err(Fault::Abi(format!("该是一个地址,读回来的是 {self:?}"))),
        }
    }
    pub fn word(&self) -> R<[u8; 32]> {
        match self {
            Val::U256(w) | Val::B32(w) => Ok(*w),
            _ => Err(Fault::Abi(format!("该是一个字,读回来的是 {self:?}"))),
        }
    }
    pub fn small(&self) -> R<u64> {
        let w = self.word()?;
        if w[..24].iter().any(|x| *x != 0) {
            return Err(Fault::Abi("这一枚数超出 u64 的界,本处不截断".into()));
        }
        Ok(u64::from_be_bytes(w[24..].try_into().expect("八字节")))
    }
    pub fn boolean(&self) -> R<bool> {
        match self {
            Val::Bool(b) => Ok(*b),
            _ => Err(Fault::Abi(format!("该是真假,读回来的是 {self:?}"))),
        }
    }
    pub fn bytes(&self) -> R<&[u8]> {
        match self {
            Val::Bytes(b) => Ok(b),
            _ => Err(Fault::Abi(format!("该是一段字节,读回来的是 {self:?}"))),
        }
    }
    pub fn items(&self) -> R<&[Val]> {
        match self {
            Val::Arr(v) | Val::Tuple(v) => Ok(v),
            _ => Err(Fault::Abi(format!("该是一列,读回来的是 {self:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_static_tuple_lies_flat_in_the_head() {
        let v = Val::Tuple(vec![Val::u64(1), Val::Bool(true)]);
        let b = encode(&[v]);
        assert_eq!(b.len(), 64);
        assert_eq!(b[31], 1);
        assert_eq!(b[63], 1);
    }

    #[test]
    fn bytes_are_length_then_content_then_padding() {
        let b = encode(&[Val::Bytes(vec![0xaa; 33])]);
        assert_eq!(b.len(), 32 * 4);
        assert_eq!(b[31], 32);
        assert_eq!(b[63], 33);
        assert_eq!(b[64], 0xaa);
        assert_eq!(b[64 + 33], 0);
    }

    #[test]
    fn what_goes_in_comes_back_out() {
        let vals = vec![
            Val::Addr([0x11; 20]),
            Val::Bytes(vec![1, 2, 3]),
            Val::Arr(vec![Val::u64(7), Val::u64(8)]),
            Val::Tuple(vec![Val::B32([0x22; 32]), Val::Arr(vec![Val::Bytes(vec![9])])]),
        ];
        let tys = vec![
            Ty::Addr,
            Ty::Bytes,
            Ty::Arr(Box::new(Ty::Uint)),
            Ty::Tuple(vec![Ty::B32, Ty::Arr(Box::new(Ty::Bytes))]),
        ];
        let back = decode(&tys, &encode(&vals)).expect("读得回来");
        assert_eq!(back, vals);
    }

    #[test]
    fn a_short_segment_is_refused_by_name() {
        let f = decode(&[Ty::Uint], &[0u8; 31]).expect_err("该拒");
        assert_eq!(f.name(), "E_A1_ABI");
    }

    #[test]
    fn a_number_past_the_u64_bound_is_refused_and_never_truncated() {
        let mut w = [0u8; 32];
        w[0] = 1;
        let f = Val::U256(w).small().expect_err("该拒");
        assert_eq!(f.name(), "E_A1_ABI");
    }
}

#[cfg(test)]
mod hostile {
    use super::*;

    #[test]
    fn arbitrary_bytes_never_panic() {
        let ty = [Ty::Arr(Box::new(Ty::Tuple(vec![Ty::Addr, Ty::Bool, Ty::Bytes])))];
        let w = |n: u64| {
            let mut x = [0u8; 32];
            x[24..].copy_from_slice(&n.to_be_bytes());
            x
        };
        let mut d = Vec::new();
        d.extend_from_slice(&w(32));
        d.extend_from_slice(&w(1));
        d.extend_from_slice(&w(0xffff));
        assert!(decode(&ty, &d).is_err());

        let mut d = Vec::new();
        d.extend_from_slice(&w(32));
        d.extend_from_slice(&w(1));
        d.extend_from_slice(&w(u64::MAX));
        assert!(decode(&ty, &d).is_err());

        let mut d = Vec::new();
        d.extend_from_slice(&w(32));
        d.extend_from_slice(&w(u64::MAX));
        assert!(decode(&ty, &d).is_err());

        assert!(decode(&ty, &[]).is_err());
        assert!(decode(&ty, &[0u8; 31]).is_err());
    }
}
