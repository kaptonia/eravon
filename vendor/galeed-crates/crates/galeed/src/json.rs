//! §3 canonical form: an own RFC 8259 reader with the law's limits, §3.4 canonical bytes, the six
//! acceptance checks of §3.5. This is the one canonical-bytes implementation: papers, payloads, judge
//! documents and rosters take their bytes from here. No third-party JSON library.
//!
//! The value universe (§3.1) is null, true, false, integers in [0, 2^53 − 1], strings, arrays and
//! objects. Object members are kept in order of appearance (duplicates are settled by §3.5 check 3)
//! and written canonically in byte order of the decoded keys.

/// File extension of a canonical JSON file. Ledger entry files and fixture file selection share it.
pub const FILE_EXT: &str = ".json";

use crate::token::Token;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(u64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

/// §3.2: the integer bound 2^53 − 1.
pub const INT_MAX: u64 = (1u64 << 53) - 1;
/// §3.5 check 2: a container opened at depth 129 is `E_DEPTH`; the root container is depth 1; 128 passes.
pub const DEPTH_MAX: usize = 128;

impl Value {
    pub fn as_obj(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Obj(m) => Some(m),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn get(&self, k: &str) -> Option<&Value> {
        self.as_obj()?.iter().find(|(key, _)| key == k).map(|(_, v)| v)
    }
    /// [`Value::get`] by the name the family's shared code calls it (`galeed_glue::door`).
    pub fn member(&self, k: &str) -> Option<&Value> {
        self.get(k)
    }
    /// The whole number this is, if it is one (the family's shared chain code reads it so).
    pub fn as_int(&self) -> Option<u64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }
    /// Whether this is an object (the family's shared desk code asks it so).
    pub fn is_obj(&self) -> bool {
        matches!(self, Value::Obj(_))
    }
    pub fn as_arr(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Arr(a) => Some(a),
            _ => None,
        }
    }
}

/// All six checks of §3.5: the bytes accepted as canonical, answered as their value.
pub fn accept(b: &[u8]) -> Result<Value, Token> {
    let v = read(b)?;
    if canonical(&v).as_deref() != Ok(b) {
        return Err(Token::NotCanonical);
    }
    Ok(v)
}

/// §3.5 checks 1 to 5 (the reading of `gl1 canon` and `gl1 sign`): the value, without check 6.
pub fn read(b: &[u8]) -> Result<Value, Token> {
    // Check 1: the whole string is RFC 3629 UTF-8 (std's check is exactly that: shortest form, no
    // surrogates, nothing above U+10FFFF).
    if std::str::from_utf8(b).is_err() {
        return Err(Token::Utf8);
    }
    // Check 2: one left-to-right pass that stops at the first error.
    let v = Parser { b, i: 0, pairs: false }.text()?;
    // Checks 3, 4 and 5 each settle over the whole value before the next one runs.
    if has_dup_key(&v) {
        return Err(Token::DupKey);
    }
    if !keys_ok(&v) {
        return Err(Token::KeyCharset);
    }
    if !values_ok(&v, false) {
        return Err(Token::ValueCharset);
    }
    Ok(v)
}

/// §3.5 checks 1 and 2 only (valid UTF-8, one JSON text): how the family's shared desk code reads its own
/// files. Law bytes are read by [`accept`].
pub fn parse(b: &[u8]) -> Result<Value, Token> {
    if std::str::from_utf8(b).is_err() {
        return Err(Token::Utf8);
    }
    Parser { b, i: 0, pairs: false }.text()
}

/// §3.5 checks 1 to 3 (valid UTF-8, one JSON text, no member given twice): how a message from outside the law is
/// read (the command line's `mcp` lines). Law bytes are read by [`accept`].
pub fn parse_tests_1_3(b: &[u8]) -> Result<Value, Token> {
    let v = parse(b)?;
    if has_dup_key(&v) {
        return Err(Token::DupKey);
    }
    Ok(v)
}

/// §3.4 canonical bytes as the family's shared desk code writes its own files. The values it builds hold
/// integers within the universe; law bytes leave through [`canonical`], the write exit that refuses one outside.
pub fn canon_bytes(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write(v, &mut out);
    out
}

/// The harness reader of audit input: RFC 8259 plus five limits (UTF-8; integer form and the 2^53
/// bound; no duplicate decoded names; strings decoded to scalar values, lone surrogates refused and
/// pairs read as one scalar; depth 129 unreadable), nothing else: the §3.3 character set and §3.5
/// check 6 do not apply. Unreadable is None (the caller answers NO_LABEL). A different rule from [`read`].
pub fn read_open(b: &[u8]) -> Option<Value> {
    if std::str::from_utf8(b).is_err() {
        return None;
    }
    let v = Parser { b, i: 0, pairs: true }.text().ok()?;
    if has_dup_key(&v) {
        return None;
    }
    Some(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    /// Surrogate escapes: the entry reader (§3.3) refuses any surrogate with `E_JSON`; the audit-input
    /// reader reads a pair as one scalar and refuses a lone one. Two readers, one machine.
    pairs: bool,
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while self.peek().is_some_and(is_ws) {
            self.i += 1;
        }
    }

    /// `JSON-text = ws value ws`; trailing bytes are a grammar error.
    fn text(&mut self) -> Result<Value, Token> {
        self.ws();
        let v = self.value(0)?;
        self.ws();
        if self.i != self.b.len() {
            return Err(Token::Json);
        }
        Ok(v)
    }

    fn value(&mut self, depth: usize) -> Result<Value, Token> {
        match self.peek() {
            None => Err(Token::Json),
            Some(b'{') => {
                if depth + 1 > DEPTH_MAX {
                    return Err(Token::Depth);
                }
                self.i += 1;
                self.object(depth + 1)
            }
            Some(b'[') => {
                if depth + 1 > DEPTH_MAX {
                    return Err(Token::Depth);
                }
                self.i += 1;
                self.array(depth + 1)
            }
            Some(b'"') => {
                self.i += 1;
                self.string().map(Value::Str)
            }
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'n') => self.literal(b"null", Value::Null),
            // §3.1 numeric value position: the first byte is - + . or a digit, and the token is the longest run
            // of 0-9 + - . e E.
            Some(c) if c == b'-' || c == b'+' || c == b'.' || c.is_ascii_digit() => self.number(),
            Some(_) => Err(Token::Json),
        }
    }

    fn literal(&mut self, lit: &[u8], v: Value) -> Result<Value, Token> {
        if self.b[self.i..].starts_with(lit) {
            self.i += lit.len();
            Ok(v)
        } else {
            Err(Token::Json)
        }
    }

    /// §3.2: unsigned, no leading zero (zero is `0`), within [0, 2^53 − 1]; otherwise `E_NUMBER`.
    fn number(&mut self) -> Result<Value, Token> {
        let start = self.i;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.' | b'e' | b'E'))
        {
            self.i += 1;
        }
        let run = &self.b[start..self.i];
        let ok_form = !run.is_empty()
            && run.iter().all(u8::is_ascii_digit)
            && (run == b"0" || run[0] != b'0');
        if !ok_form {
            return Err(Token::Number);
        }
        let mut n: u64 = 0;
        for &d in run {
            n = n
                .checked_mul(10)
                .and_then(|n| n.checked_add(u64::from(d - b'0')))
                .filter(|n| *n <= INT_MAX)
                .ok_or(Token::Number)?;
        }
        Ok(Value::Int(n))
    }

    /// The opening quote is consumed. Raw 0x00 to 0x1F is a grammar error; a `\u` surrogate code point is
    /// `E_JSON` (§3.3), paired or not.
    fn string(&mut self) -> Result<String, Token> {
        let mut s = String::new();
        loop {
            let c = self.peek().ok_or(Token::Json)?;
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(s);
                }
                b'\\' => {
                    self.i += 1;
                    let e = self.peek().ok_or(Token::Json)?;
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
                            let scalar = match cp {
                                0xD800..=0xDBFF if self.pairs => {
                                    // A high surrogate is one scalar only when followed by a `\u` low surrogate; otherwise it is lone.
                                    if self.b.get(self.i..self.i + 2) != Some(b"\\u") {
                                        return Err(Token::Json);
                                    }
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&lo) {
                                        return Err(Token::Json);
                                    }
                                    0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00)
                                }
                                0xD800..=0xDFFF => return Err(Token::Json),
                                _ => cp,
                            };
                            s.push(char::from_u32(scalar).ok_or(Token::Json)?);
                        }
                        _ => return Err(Token::Json),
                    }
                }
                0x00..=0x1F => return Err(Token::Json),
                0x20..=0x7F => {
                    s.push(char::from(c));
                    self.i += 1;
                }
                _ => {
                    // Check 1 already guaranteed UTF-8; this only slices out the bytes of one scalar.
                    let len = match c {
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        _ => 4,
                    };
                    let piece = self.b.get(self.i..self.i + len).ok_or(Token::Json)?;
                    let ch = std::str::from_utf8(piece).map_err(|_| Token::Json)?;
                    s.push_str(ch);
                    self.i += len;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, Token> {
        let mut cp: u32 = 0;
        for _ in 0..4 {
            let h = self.peek().ok_or(Token::Json)?;
            let d = match h {
                b'0'..=b'9' => h - b'0',
                b'a'..=b'f' => h - b'a' + 10,
                b'A'..=b'F' => h - b'A' + 10,
                _ => return Err(Token::Json),
            };
            cp = (cp << 4) | u32::from(d);
            self.i += 1;
        }
        Ok(cp)
    }

    fn object(&mut self, depth: usize) -> Result<Value, Token> {
        let mut members: Vec<(String, Value)> = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Obj(members));
        }
        loop {
            // A member-name position is never numeric: anything but a quote is a grammar error.
            if self.peek() != Some(b'"') {
                return Err(Token::Json);
            }
            self.i += 1;
            let k = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(Token::Json);
            }
            self.i += 1;
            self.ws();
            let v = self.value(depth)?;
            members.push((k, v));
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.ws();
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Value::Obj(members));
                }
                _ => return Err(Token::Json),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Token> {
        let mut items: Vec<Value> = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::Arr(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.ws();
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::Arr(items));
                }
                _ => return Err(Token::Json),
            }
        }
    }
}

/// Check 3: in every object at every depth, decoded keys are unique.
fn has_dup_key(v: &Value) -> bool {
    match v {
        Value::Obj(m) => {
            let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
            m.iter().any(|(k, _)| !seen.insert(k.as_str())) || m.iter().any(|(_, v)| has_dup_key(v))
        }
        Value::Arr(a) => a.iter().any(has_dup_key),
        _ => false,
    }
}

/// Skeleton string: every scalar value is in U+0020 to U+007E.
pub fn is_skeleton(s: &str) -> bool {
    s.chars().all(|c| ('\u{20}'..='\u{7e}').contains(&c))
}

/// Check 4: keys are non-empty skeleton strings.
fn keys_ok(v: &Value) -> bool {
    match v {
        Value::Obj(m) => m
            .iter()
            .all(|(k, v)| !k.is_empty() && is_skeleton(k) && keys_ok(v)),
        Value::Arr(a) => a.iter().all(keys_ok),
        _ => true,
    }
}

/// Check 5: string values outside prose subtrees are skeleton strings. A member whose key ends in `_md`
/// (or is `_md`) opens a prose subtree: its value and every string value inside it at any depth are
/// prose; keys inside it still fall under check 4.
fn values_ok(v: &Value, prose: bool) -> bool {
    match v {
        Value::Str(s) => prose || is_skeleton(s),
        Value::Arr(a) => a.iter().all(|x| values_ok(x, prose)),
        Value::Obj(m) => m
            .iter()
            .all(|(k, x)| values_ok(x, prose || k.ends_with("_md"))),
        _ => true,
    }
}

/// §3.4 canonical bytes, and **the one write exit of the §3.1 value universe**: a value holding an integer outside
/// [0, 2^53 − 1] is refused with the token the reader answers for that same integer (`E_NUMBER`, §3.2), so whatever
/// is written here reads back. Every galeed/1 byte the product writes (documents, books, bases, papers, signatures,
/// hashes, answers) comes from here; a new field lands behind this one check instead of carrying its own bound.
pub fn canonical(v: &Value) -> Result<Vec<u8>, Token> {
    if !in_universe(v) {
        return Err(Token::Number);
    }
    let mut out = Vec::new();
    write(v, &mut out);
    Ok(out)
}

/// A value as canonical text, for showing to a person only: a value the write exit refuses shows that refusal's
/// token, never a number the reader would refuse.
pub fn canonical_text(v: &Value) -> String {
    match canonical(v) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(t) => t.name().to_string(),
    }
}

/// Typed text read as a §3.2 integer, **by the reader itself**: digits only, no leading zero, within [0, 2^53 − 1];
/// anything else is `None`. Every place that takes an integer from a person or a command line reads it here, so a
/// number the write exit would refuse is refused at the door that took it.
pub fn int_text(t: &str) -> Option<u64> {
    match read(t.as_bytes()) {
        Ok(Value::Int(n)) => Some(n),
        _ => None,
    }
}

/// §3.1: every integer in the value is within [0, 2^53 − 1].
fn in_universe(v: &Value) -> bool {
    match v {
        Value::Int(n) => *n <= INT_MAX,
        Value::Arr(a) => a.iter().all(in_universe),
        Value::Obj(m) => m.iter().all(|(_, x)| in_universe(x)),
        _ => true,
    }
}

fn write(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Int(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Value::Str(s) => write_str(s, out),
        Value::Arr(a) => {
            out.push(b'[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write(x, out);
            }
            out.push(b']');
        }
        Value::Obj(m) => {
            out.push(b'{');
            for (i, idx) in member_order(m).into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_str(&m[idx].0, out);
                out.push(b':');
                write(&m[idx].1, out);
            }
            out.push(b'}');
        }
    }
}

/// §3.4 item 5: members sorted by the UTF-8 bytes of their decoded keys.
#[cfg(not(feature = "ablate-k1-json-canon"))]
fn member_order(m: &[(String, Value)]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..m.len()).collect();
    idx.sort_by(|a, b| m[*a].0.as_bytes().cmp(m[*b].0.as_bytes()));
    idx
}

/// Fault-injection variant for the conformance gauges: members are written in order of appearance.
#[cfg(feature = "ablate-k1-json-canon")]
fn member_order(m: &[(String, Value)]) -> Vec<usize> {
    (0..m.len()).collect()
}

/// §3.4 item 3: `"`, `\` and the five short escapes; other U+0000–U+001F as lowercase `\u00xx`;
/// everything else as its raw bytes.
fn write_str(s: &str, out: &mut Vec<u8>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(b: &[u8]) -> Result<Vec<u8>, Token> {
        accept(b).map(|v| canonical(&v).unwrap())
    }

    #[test]
    fn six_tests_in_order() {
        assert_eq!(tok(b"\xff"), Err(Token::Utf8));
        assert_eq!(tok(b"\xef\xbb\xbf{}"), Err(Token::Json));
        assert_eq!(tok(b"{\"a\":1}"), Ok(b"{\"a\":1}".to_vec()));
        assert_eq!(tok(b"{\"a\":01}"), Err(Token::Number));
        assert_eq!(tok(b"[-0]"), Err(Token::Number));
        assert_eq!(tok(b"[1.0]"), Err(Token::Number));
        assert_eq!(tok(b"[9007199254740992]"), Err(Token::Number));
        assert_eq!(tok(b"[9007199254740991]"), Ok(b"[9007199254740991]".to_vec()));
        assert_eq!(tok(b"0x10"), Err(Token::Json));
        assert_eq!(tok(b"-Infinity"), Err(Token::Number));
        assert_eq!(tok(b"NaN"), Err(Token::Json));
        assert_eq!(tok(b"{\"a\":1,\"a\":2}"), Err(Token::DupKey));
        assert_eq!(tok(b"{\"\":1}"), Err(Token::KeyCharset));
        assert_eq!(tok("{\"é\":1}".as_bytes()), Err(Token::KeyCharset));
        assert_eq!(tok("{\"a\":\"é\"}".as_bytes()), Err(Token::ValueCharset));
        assert_eq!(tok("{\"x_md\":\"é\"}".as_bytes()), Ok("{\"x_md\":\"é\"}".as_bytes().to_vec()));
        assert_eq!(tok(b"{\"b\":1,\"a\":2}"), Err(Token::NotCanonical));
        assert_eq!(tok(b"{\"a\":1}\n"), Err(Token::NotCanonical));
        assert_eq!(tok(b"[\"\\ud83d\\ude00\"]"), Err(Token::Json));
        assert_eq!(tok(b"[\"\x01\"]"), Err(Token::Json));
        assert_eq!(tok(b"{\"a_md\":\"\\u0001\"}"), Ok(b"{\"a_md\":\"\\u0001\"}".to_vec()));
        assert_eq!(tok(b"{\"a\":1,\"a\":\xc3\xa9}"), Err(Token::Json));
    }

    /// §3.1, §3.2 and §3.5 test 2 on numbers, the law's own examples and the shapes around them: a value position of
    /// numeric shape opens at `-`, `+`, `.` or a digit (also right after `[`), its bytes are the maximal run of
    /// digits, `+`, `-`, `.`, `e`, `E`, and a run that is not an integer in [0, 2^53 − 1] is `E_NUMBER`, named at its
    /// first byte; a byte after a whole integer that the grammar refuses is `E_JSON`. A judge's tooling that wrote a
    /// float, a signed or an overflowing number is refused at the right name, and a reader of another tool's bytes
    /// names the fault the core of the law names.
    #[test]
    fn number_faults_named_at_the_position() {
        for spelled in [
            "[+1]", "[.5]", "[01]", "[-0]", "[-1]", "[1.0]", "[1e5]", "[1E5]", "[1e+5]", "0-", "[1-2]", "[1,-2]", "[-]", "[+]",
            "[9007199254740992]", "[18446744073709551616]", "[99999999999999999999999999]", "-Infinity", "{\"a\":-1}",
        ] {
            assert_eq!(tok(spelled.as_bytes()), Err(Token::Number), "{spelled}");
        }
        for spelled in ["0x10", "[1 -2]", "[1x]", "{1:2}", "[Infinity]", "[1,]", "[,1]"] {
            assert_eq!(tok(spelled.as_bytes()), Err(Token::Json), "{spelled}");
        }
        for (spelled, canon) in [("0", "0"), ("[]", "[]"), ("[0]", "[0]"), ("[0,1]", "[0,1]"), ("9007199254740991", "9007199254740991")] {
            assert_eq!(tok(spelled.as_bytes()), Ok(canon.as_bytes().to_vec()), "{spelled}");
        }
        // The earliest triggering byte is named: a number fault before a later grammar fault, and the reverse.
        assert_eq!(tok(b"[01,x]"), Err(Token::Number));
        assert_eq!(tok(b"[x,01]"), Err(Token::Json));
        // A number fault before a repeated key: test 2 is decided before test 3.
        assert_eq!(tok(b"{\"a\":1,\"a\":-1}"), Err(Token::Number));
    }

    #[test]
    fn open_reader_two_laws() {
        assert_eq!(read(b"[\"\\ud83d\\ude00\"]"), Err(Token::Json));
        assert_eq!(read_open(b"[\"\\ud83d\\ude00\"]"), Some(Value::Arr(vec![Value::Str("\u{1F600}".into())])));
        assert_eq!(read_open(b"[\"\\ud83d\"]"), None);
        assert_eq!(read_open(b"[\"\\ude00\"]"), None);
        assert_eq!(read_open(b"[\"\\ud83dx\"]"), None);
        assert!(read_open("{\"\u{e9}\":\"\\u0001\"}".as_bytes()).is_some(), "字符集不及 audit 输入");
        assert_eq!(read_open(b"{\"a\":1,\"\\u0061\":2}"), None, "解码后重名");
        assert_eq!(read_open(b"[1.5]"), None);
        assert_eq!(read_open(b"\xef\xbb\xbf{}"), None);
        assert_eq!(read_open(b"[\"\x01\"]"), None, "原生控制字节是文法错");
        assert!(read_open(b"{ \"b\" : 1 , \"a\" : 2 }").is_some(), "第 6 道不问");
    }

    #[test]
    fn depth_wall() {
        let ok = format!("{}{}", "[".repeat(128), "]".repeat(128));
        assert!(accept(ok.as_bytes()).is_ok());
        let bad = format!("{}{}", "[".repeat(129), "]".repeat(129));
        assert_eq!(accept(bad.as_bytes()), Err(Token::Depth));
        // The `[` at depth 129 is read before any later error.
        let bad2 = format!("{}\x01", "[".repeat(129));
        assert_eq!(accept(bad2.as_bytes()), Err(Token::Depth));
    }

    #[test]
    fn canon_reorders_and_escapes() {
        let v = read(b" { \"b_md\" : \"\\u0041\\/\\u001f\" , \"a\" : [ true , null ] } ").unwrap();
        assert_eq!(canonical(&v).unwrap(), b"{\"a\":[true,null],\"b_md\":\"A/\\u001f\"}");
    }

    #[test]
    fn the_write_exit_writes_only_what_the_reader_reads_back() {
        let at = |n: u64| Value::Obj(vec![("a".to_string(), Value::Arr(vec![Value::Int(n)]))]);
        assert_eq!(canonical(&at(INT_MAX)).unwrap(), b"{\"a\":[9007199254740991]}");
        assert_eq!(canonical(&at(INT_MAX + 1)), Err(Token::Number), "上界加一:拒写,与读那一侧同名");
        assert_eq!(canonical(&at(u64::MAX)), Err(Token::Number));
        assert_eq!(read(&canonical(&at(INT_MAX)).unwrap()), Ok(at(INT_MAX)), "写得出即读得回");
        assert_eq!(canonical_text(&at(INT_MAX + 1)), "E_NUMBER");
        assert_eq!(int_text("9007199254740991"), Some(INT_MAX));
        assert_eq!(int_text("9007199254740992"), None, "过界");
        assert_eq!(int_text("18446744073709551616"), None, "过 u64");
        assert_eq!(int_text("0"), Some(0));
        assert_eq!(int_text("054"), None, "前导零不是 §3.2 的 int");
        assert_eq!(int_text("-1"), None);
        assert_eq!(int_text("x"), None);
        assert_eq!(int_text(""), None);
    }
}
