#[derive(Clone, Debug)]
pub enum Out {
    S(String),
    N(u64),
    B(bool),
    A(Vec<Out>),
    O(Vec<(&'static str, Out)>),
    Null,
}

fn member_order(m: &[(&'static str, Out)]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..m.len()).collect();
    idx.sort_by(|a, b| m[*a].0.as_bytes().cmp(m[*b].0.as_bytes()));
    idx
}


pub fn write(v: &Out) -> Vec<u8> {
    let mut out = Vec::new();
    put(v, &mut out);
    out
}

fn put(v: &Out, out: &mut Vec<u8>) {
    match v {
        Out::S(s) => put_str(s, out),
        Out::N(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Out::B(true) => out.extend_from_slice(b"true"),
        Out::B(false) => out.extend_from_slice(b"false"),
        Out::Null => out.extend_from_slice(b"null"),
        Out::A(a) => {
            out.push(b'[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                put(x, out);
            }
            out.push(b']');
        }
        Out::O(m) => {
            out.push(b'{');
            for (i, idx) in member_order(m).into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                put_str(m[idx].0, out);
                out.push(b':');
                put(&m[idx].1, out);
            }
            out.push(b'}');
        }
    }
}

pub fn write_in(v: &In) -> Vec<u8> {
    let mut out = Vec::new();
    put_in(v, &mut out);
    out
}

fn put_in(v: &In, out: &mut Vec<u8>) {
    match v {
        In::Null => out.extend_from_slice(b"null"),
        In::Bool(true) => out.extend_from_slice(b"true"),
        In::Bool(false) => out.extend_from_slice(b"false"),
        In::Int(n) => out.extend_from_slice(n.to_string().as_bytes()),
        In::Str(s) => put_str(s, out),
        In::Arr(a) => {
            out.push(b'[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                put_in(x, out);
            }
            out.push(b']');
        }
        In::Obj(m) => {
            let mut idx: Vec<usize> = (0..m.len()).collect();
            idx.sort_by(|a, b| m[*a].0.as_bytes().cmp(m[*b].0.as_bytes()));
            out.push(b'{');
            for (i, j) in idx.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                put_str(&m[j].0, out);
                out.push(b':');
                put_in(&m[j].1, out);
            }
            out.push(b'}');
        }
    }
}

fn put_str(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                let n = c as u32;
                out.extend_from_slice(b"\\u00");
                out.push(b"0123456789abcdef"[(n >> 4) as usize]);
                out.push(b"0123456789abcdef"[(n & 0xf) as usize]);
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum In {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<In>),
    Obj(Vec<(String, In)>),
}

impl In {
    pub fn get(&self, key: &str) -> Option<&In> {
        match self {
            In::Obj(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            In::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
    pub fn as_arr(&self) -> Option<&[In]> {
        match self {
            In::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }
}

const DEPTH_MAX: usize = 128;

pub fn read(b: &[u8]) -> Result<In, &'static str> {
    let mut p = P { b, i: 0, depth: 0 };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err("E_JSON_TRAILING");
    }
    Ok(v)
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn eat(&mut self, c: u8) -> Result<(), &'static str> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err("E_JSON_SYNTAX")
        }
    }
    fn lit(&mut self, s: &[u8]) -> Result<(), &'static str> {
        if self.b.len() >= self.i + s.len() && &self.b[self.i..self.i + s.len()] == s {
            self.i += s.len();
            Ok(())
        } else {
            Err("E_JSON_SYNTAX")
        }
    }
    fn value(&mut self) -> Result<In, &'static str> {
        match self.peek().ok_or("E_JSON_SYNTAX")? {
            b'n' => {
                self.lit(b"null")?;
                Ok(In::Null)
            }
            b't' => {
                self.lit(b"true")?;
                Ok(In::Bool(true))
            }
            b'f' => {
                self.lit(b"false")?;
                Ok(In::Bool(false))
            }
            b'"' => Ok(In::Str(self.string()?)),
            b'[' => {
                if self.depth + 1 > DEPTH_MAX {
                    return Err("E_JSON_DEPTH");
                }
                self.i += 1;
                self.depth += 1;
                let mut a = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(In::Arr(a));
                }
                loop {
                    self.ws();
                    a.push(self.value()?);
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            self.depth -= 1;
                            return Ok(In::Arr(a));
                        }
                        _ => return Err("E_JSON_SYNTAX"),
                    }
                }
            }
            b'{' => {
                if self.depth + 1 > DEPTH_MAX {
                    return Err("E_JSON_DEPTH");
                }
                self.i += 1;
                self.depth += 1;
                let mut m: Vec<(String, In)> = Vec::new();
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(In::Obj(m));
                }
                let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
                loop {
                    self.ws();
                    let k = self.string()?;
                    if !seen.insert(k.clone()) {
                        return Err("E_JSON_DUPLICATE_KEY");
                    }
                    self.ws();
                    self.eat(b':')?;
                    self.ws();
                    let v = self.value()?;
                    m.push((k, v));
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            self.depth -= 1;
                            return Ok(In::Obj(m));
                        }
                        _ => return Err("E_JSON_SYNTAX"),
                    }
                }
            }
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err("E_JSON_SYNTAX"),
        }
    }
    fn number(&mut self) -> Result<In, &'static str> {
        let start = self.i;
        let neg = self.peek() == Some(b'-');
        if neg {
            self.i += 1;
        }
        let digits = self.i;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        let n = self.i - digits;
        if n == 0 || (n > 1 && self.b[digits] == b'0') || (neg && n == 1 && self.b[digits] == b'0') {
            return Err("E_JSON_NUMBER");
        }
        if matches!(self.peek(), Some(b'.') | Some(b'e') | Some(b'E')) {
            return Err("E_JSON_NOT_INT");
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).map_err(|_| "E_JSON_SYNTAX")?;
        s.parse::<i64>().map(In::Int).map_err(|_| "E_JSON_INT_RANGE")
    }
    fn string(&mut self) -> Result<String, &'static str> {
        self.eat(b'"')?;
        let mut s = String::new();
        loop {
            let c = self.peek().ok_or("E_JSON_SYNTAX")?;
            self.i += 1;
            match c {
                b'"' => return Ok(s),
                b'\\' => {
                    let e = self.peek().ok_or("E_JSON_SYNTAX")?;
                    self.i += 1;
                    match e {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'b' => s.push('\u{8}'),
                        b'f' => s.push('\u{c}'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            let cp = self.hex4()?;
                            let ch = if (0xD800..0xDC00).contains(&cp) {
                                self.eat(b'\\')?;
                                self.eat(b'u')?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err("E_JSON_ESCAPE");
                                }
                                let v = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                char::from_u32(v).ok_or("E_JSON_ESCAPE")?
                            } else {
                                char::from_u32(cp).ok_or("E_JSON_ESCAPE")?
                            };
                            s.push(ch);
                        }
                        _ => return Err("E_JSON_ESCAPE"),
                    }
                }
                c if c < 0x20 => return Err("E_JSON_CONTROL"),
                _ => {
                    let start = self.i - 1;
                    let len = utf8_len(c).ok_or("E_JSON_UTF8")?;
                    if start + len > self.b.len() {
                        return Err("E_JSON_UTF8");
                    }
                    let t = std::str::from_utf8(&self.b[start..start + len])
                        .map_err(|_| "E_JSON_UTF8")?;
                    s.push_str(t);
                    self.i = start + len;
                }
            }
        }
    }
    fn hex4(&mut self) -> Result<u32, &'static str> {
        if self.i + 4 > self.b.len() {
            return Err("E_JSON_ESCAPE");
        }
        let mut v = 0u32;
        for k in 0..4 {
            let d = self.b[self.i + k];
            let n = match d {
                b'0'..=b'9' => (d - b'0') as u32,
                b'a'..=b'f' => (d - b'a' + 10) as u32,
                b'A'..=b'F' => (d - b'A' + 10) as u32,
                _ => return Err("E_JSON_ESCAPE"),
            };
            v = (v << 4) | n;
        }
        self.i += 4;
        Ok(v)
    }
}

const fn utf8_len(b: u8) -> Option<usize> {
    match b {
        0x00..=0x7F => Some(1),
        0xC2..=0xDF => Some(2),
        0xE0..=0xEF => Some(3),
        0xF0..=0xF4 => Some(4),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Raw {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Raw>),
    Obj(Vec<(String, Raw)>),
}

impl Raw {
    pub fn get(&self, key: &str) -> Option<&Raw> {
        match self {
            Raw::Obj(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Raw::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Raw::Null)
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Raw::Num(n) if n.bytes().all(|b| b.is_ascii_digit()) => n.parse().ok(),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<&str> {
        match self {
            Raw::Num(n) => Some(n),
            _ => None,
        }
    }

    pub fn project(&self, facts: &[&str]) -> Raw {
        match self {
            Raw::Obj(m) => Raw::Obj(m.iter().filter(|(k, _)| facts.contains(&k.as_str())).cloned().collect()),
            other => other.clone(),
        }
    }

    pub fn to_in(&self) -> Result<In, &'static str> {
        Ok(match self {
            Raw::Null => In::Null,
            Raw::Bool(b) => In::Bool(*b),
            Raw::Num(n) => {
                if n.contains(['.', 'e', 'E']) {
                    return Err("E_JSON_NOT_INT");
                }
                In::Int(n.parse::<i64>().map_err(|_| "E_JSON_INT_RANGE")?)
            }
            Raw::Str(s) => In::Str(s.clone()),
            Raw::Arr(a) => In::Arr(a.iter().map(Raw::to_in).collect::<Result<_, _>>()?),
            Raw::Obj(m) => In::Obj(m.iter().map(|(k, v)| Ok((k.clone(), v.to_in()?))).collect::<Result<_, &'static str>>()?),
        })
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.put(&mut out);
        out
    }

    fn put(&self, out: &mut Vec<u8>) {
        match self {
            Raw::Null => out.extend_from_slice(b"null"),
            Raw::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
            Raw::Num(n) => out.extend_from_slice(n.as_bytes()),
            Raw::Str(s) => put_str(s, out),
            Raw::Arr(a) => {
                out.push(b'[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    v.put(out);
                }
                out.push(b']');
            }
            Raw::Obj(m) => {
                out.push(b'{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    put_str(k, out);
                    out.push(b':');
                    v.put(out);
                }
                out.push(b'}');
            }
        }
    }
}

pub fn read_raw(b: &[u8]) -> Result<Raw, &'static str> {
    let mut p = P { b, i: 0, depth: 0 };
    p.ws();
    let v = p.raw()?;
    p.ws();
    if p.i != p.b.len() {
        return Err("E_JSON_TRAILING");
    }
    Ok(v)
}

impl<'a> P<'a> {
    fn raw(&mut self) -> Result<Raw, &'static str> {
        match self.peek().ok_or("E_JSON_SYNTAX")? {
            b'n' => {
                self.lit(b"null")?;
                Ok(Raw::Null)
            }
            b't' => {
                self.lit(b"true")?;
                Ok(Raw::Bool(true))
            }
            b'f' => {
                self.lit(b"false")?;
                Ok(Raw::Bool(false))
            }
            b'"' => Ok(Raw::Str(self.string()?)),
            b'[' | b'{' => {
                if self.depth + 1 > DEPTH_MAX {
                    return Err("E_JSON_DEPTH");
                }
                let obj = self.peek() == Some(b'{');
                let close = if obj { b'}' } else { b']' };
                self.i += 1;
                self.depth += 1;
                let mut a: Vec<Raw> = Vec::new();
                let mut m: Vec<(String, Raw)> = Vec::new();
                let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
                self.ws();
                if self.peek() != Some(close) {
                    loop {
                        self.ws();
                        if obj {
                            let k = self.string()?;
                            if !seen.insert(k.clone()) {
                                return Err("E_JSON_DUPLICATE_KEY");
                            }
                            self.ws();
                            self.eat(b':')?;
                            self.ws();
                            let v = self.raw()?;
                            m.push((k, v));
                        } else {
                            a.push(self.raw()?);
                        }
                        self.ws();
                        match self.peek() {
                            Some(b',') => self.i += 1,
                            Some(c) if c == close => break,
                            _ => return Err("E_JSON_SYNTAX"),
                        }
                    }
                }
                self.i += 1;
                self.depth -= 1;
                Ok(if obj { Raw::Obj(m) } else { Raw::Arr(a) })
            }
            b'-' | b'0'..=b'9' => self.raw_number(),
            _ => Err("E_JSON_SYNTAX"),
        }
    }

    fn raw_number(&mut self) -> Result<Raw, &'static str> {
        let start = self.i;
        let neg = self.peek() == Some(b'-');
        if neg {
            self.i += 1;
        }
        let digits = self.i;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        let n = self.i - digits;
        if n == 0 || (n > 1 && self.b[digits] == b'0') || (neg && n == 1 && self.b[digits] == b'0' && !matches!(self.peek(), Some(b'.' | b'e' | b'E'))) {
            return Err("E_JSON_NUMBER");
        }
        let run = |p: &mut P<'a>| {
            let at = p.i;
            while matches!(p.peek(), Some(b'0'..=b'9')) {
                p.i += 1;
            }
            p.i > at
        };
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !run(self) {
                return Err("E_JSON_NUMBER");
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !run(self) {
                return Err("E_JSON_NUMBER");
            }
        }
        Ok(Raw::Num(std::str::from_utf8(&self.b[start..self.i]).map_err(|_| "E_JSON_SYNTAX")?.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_sorts_members_by_key_bytes() {
        let v = Out::O(vec![("b", Out::N(2)), ("a", Out::N(1))]);
        assert_eq!(write(&v), b"{\"a\":1,\"b\":2}".to_vec());
    }

    #[test]
    fn writer_escapes_the_seven_and_the_controls() {
        let v = Out::S("a\"b\\c\nd\u{1}".to_string());
        assert_eq!(write(&v), b"\"a\\\"b\\\\c\\nd\\u0001\"".to_vec());
    }

    #[test]
    fn reader_round_trips_a_nested_document() {
        let src = b"{\"a\":[1,-2,{\"b\":\"x\\u00e9\\ud83d\\ude00\"}],\"c\":true,\"d\":null}";
        let v = read(src).expect("读得回");
        assert_eq!(v.get("c"), Some(&In::Bool(true)));
        let inner = v.get("a").unwrap().as_arr().unwrap();
        assert_eq!(inner[1], In::Int(-2));
        assert_eq!(inner[2].get("b").unwrap().as_str().unwrap(), "xé😀");
    }

    #[test]
    fn reader_refuses_rather_than_guessing() {
        assert_eq!(read(b"{\"a\":1}x"), Err("E_JSON_TRAILING"));
        assert_eq!(read(b"{\"a\":1,\"a\":2}"), Err("E_JSON_DUPLICATE_KEY"));
        assert_eq!(read(b"1.5"), Err("E_JSON_NOT_INT"));
        assert_eq!(read(b"\"a\tb\""), Err("E_JSON_CONTROL"));
    }

    #[test]
    fn named_facts_first_keeps_an_answer_with_fractions_readable() {
        let fee = br#"{"oldestBlock":"0x1","baseFeePerGas":["0x10"],"gasUsedRatio":[0.0,0.25,1.5e-3],"reward":[["0x64"]]}"#;
        assert_eq!(read(fee), Err("E_JSON_NOT_INT"), "整份读:小数整份拒");
        let r = read_raw(fee).expect("线上形读得成");
        assert_eq!(r.project(&["reward"]).to_in(), Ok(In::Obj(vec![("reward".into(), In::Arr(vec![In::Arr(vec![In::Str("0x64".into())])]))])));
        assert_eq!(r.project(&["gasUsedRatio"]).to_in(), Err("E_JSON_NOT_INT"), "点名的那一格是小数仍拒");
        assert_eq!(r.project(&["absent"]).to_in(), Ok(In::Obj(Vec::new())));
        for bad in [&b"01"[..], b"-0", b"1.", b"1e", b".5", b"-", b"1.5.2"] {
            assert!(read_raw(bad).is_err(), "{}", String::from_utf8_lossy(bad));
        }
        assert_eq!(read_raw(b"-0.5"), Ok(Raw::Num("-0.5".into())));
        assert_eq!(read_raw(b"{\"a\":1,\"a\":2}"), Err("E_JSON_DUPLICATE_KEY"));
        assert_eq!(read_raw(br#"{"a":[1,2.5,"x",null,true]}"#).map(|r| r.write()), Ok(br#"{"a":[1,2.5,"x",null,true]}"#.to_vec()));
    }
}
