use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub contract: String,
    pub kind: &'static str,
    pub name: String,
    pub sig: String,
    pub indexed: Vec<bool>,
    pub returns: String,
    pub return_names: Vec<String>,
}

pub fn blank(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                out.push(b' ');
                i += 1;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            out.extend_from_slice(b"  ");
            i += 2;
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                out.push(if b[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            if i < b.len() {
                out.extend_from_slice(b"  ");
                i += 2;
            }
        } else if b[i] == b'"' || b[i] == b'\'' {
            let q = b[i];
            out.push(q);
            i += 1;
            while i < b.len() && b[i] != q {
                if b[i] == b'\\' && i + 1 < b.len() {
                    out.extend_from_slice(b"  ");
                    i += 2;
                    continue;
                }
                out.push(b' ');
                i += 1;
            }
            if i < b.len() {
                out.push(q);
                i += 1;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("blanking keeps UTF-8 (multibyte runs are inside comments or strings and become spaces)")
}

fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '$' {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            if !c.is_whitespace() {
                out.push(c.to_string());
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[derive(Clone, Debug)]
struct Param {
    ty: Vec<String>,
    indexed: bool,
    name: String,
}

fn params(toks: &[String]) -> Vec<Param> {
    let mut out = Vec::new();
    if toks.is_empty() {
        return out;
    }
    let mut depth = 0i32;
    let mut cur: Vec<String> = Vec::new();
    let mut push = |cur: &mut Vec<String>| {
        let mut ty = Vec::new();
        let mut indexed = false;
        for t in cur.drain(..) {
            match t.as_str() {
                "indexed" => indexed = true,
                "calldata" | "memory" | "storage" | "payable" => {}
                _ => ty.push(t),
            }
        }
        out.push(Param { ty, indexed, name: String::new() });
    };
    for t in toks {
        match t.as_str() {
            "(" | "[" => {
                depth += 1;
                cur.push(t.clone());
            }
            ")" | "]" => {
                depth -= 1;
                cur.push(t.clone());
            }
            "," if depth == 0 => push(&mut cur),
            _ => cur.push(t.clone()),
        }
    }
    push(&mut cur);
    for p in out.iter_mut() {
        if p.ty.len() >= 2 {
            let last = p.ty.last().expect("two or more").clone();
            let before = &p.ty[p.ty.len() - 2];
            let is_word = |s: &str| s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
            if is_word(&last) && (is_word(before) || before == "]" || before == ")") && last != "mapping" {
                p.name = p.ty.pop().expect("two or more");
            }
        }
    }
    out
}

fn group(toks: &[String], open: usize) -> (Vec<String>, usize) {
    let mut depth = 0i32;
    let mut i = open;
    while i < toks.len() {
        match toks[i].as_str() {
            "(" => depth += 1,
            ")" => {
                depth -= 1;
                if depth == 0 {
                    return (toks[open + 1..i].to_vec(), i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unclosed parenthesis in a declaration");
}

#[derive(Default)]
struct Types {
    structs: BTreeMap<String, Vec<Vec<String>>>,
    enums: Vec<String>,
    contracts: Vec<String>,
}

impl Types {
    fn canon(&self, ty: &[String], scope: &str) -> String {
        if ty.last().map(String::as_str) == Some("]") {
            let open = ty.iter().rposition(|t| t == "[").expect("an array suffix opens");
            let inner = self.canon(&ty[..open], scope);
            let n: String = ty[open + 1..ty.len() - 1].concat();
            return format!("{inner}[{n}]");
        }
        let name = ty.concat();
        let base = match name.as_str() {
            "uint" => return "uint256".into(),
            "int" => return "int256".into(),
            "byte" => return "bytes1".into(),
            "address" | "bool" | "string" | "bytes" => return name,
            n if (n.starts_with("uint") || n.starts_with("int") || n.starts_with("bytes"))
                && n.trim_start_matches(char::is_alphabetic).chars().all(|c| c.is_ascii_digit()) =>
            {
                return name;
            }
            n => n.to_string(),
        };
        let local = format!("{scope}.{base}");
        let key = if self.structs.contains_key(&local) { local.clone() } else { base.clone() };
        if let Some(fields) = self.structs.get(&key) {
            let inner: Vec<String> = fields.iter().map(|f| self.canon(f, scope_of(&key, scope))).collect();
            return format!("({})", inner.join(","));
        }
        if self.enums.contains(&local) || self.enums.contains(&base) {
            return "uint8".into();
        }
        if self.contracts.iter().any(|c| *c == base || base.ends_with(&format!(".{c}"))) {
            return "address".into();
        }
        panic!("type {base} is neither elementary nor declared in the read sources");
    }
}

fn scope_of<'a>(key: &'a str, scope: &'a str) -> &'a str {
    key.split_once('.').map(|(c, _)| c).unwrap_or(scope)
}

struct Decl {
    contract: String,
    toks: Vec<String>,
}

fn decls(toks: &[String], file: &str) -> Vec<Decl> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i].as_str();
        if matches!(t, "struct" | "enum") && i + 1 < toks.len() {
            let mut cur = vec![toks[i].clone(), toks[i + 1].clone()];
            while i < toks.len() && toks[i] != "{" {
                i += 1;
            }
            let mut depth = 0i32;
            while i < toks.len() {
                match toks[i].as_str() {
                    "{" => depth += 1,
                    "}" => depth -= 1,
                    _ => {}
                }
                cur.push(toks[i].clone());
                i += 1;
                if depth == 0 {
                    break;
                }
            }
            out.push(Decl { contract: file.to_string(), toks: cur });
            continue;
        }
        if matches!(t, "contract" | "interface" | "library") && i + 1 < toks.len() {
            let name = toks[i + 1].clone();
            while i < toks.len() && toks[i] != "{" {
                i += 1;
            }
            i += 1;
            let mut cur: Vec<String> = Vec::new();
            while i < toks.len() && toks[i] != "}" {
                if toks[i] == "{" {
                    let mut depth = 0i32;
                    let starts_struct = cur.first().map(String::as_str) == Some("struct")
                        || cur.first().map(String::as_str) == Some("enum");
                    let mut body = Vec::new();
                    while i < toks.len() {
                        match toks[i].as_str() {
                            "{" => depth += 1,
                            "}" => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        if depth >= 1 && !(depth == 1 && toks[i] == "{") {
                            body.push(toks[i].clone());
                        }
                        i += 1;
                    }
                    if starts_struct {
                        cur.push("{".into());
                        cur.extend(body);
                        cur.push("}".into());
                    }
                    out.push(Decl { contract: name.clone(), toks: std::mem::take(&mut cur) });
                    i += 1;
                    continue;
                }
                if toks[i] == ";" {
                    out.push(Decl { contract: name.clone(), toks: std::mem::take(&mut cur) });
                    i += 1;
                    continue;
                }
                cur.push(toks[i].clone());
                i += 1;
            }
        }
        i += 1;
    }
    out
}

pub fn read(sources: &[(String, String)]) -> Vec<Item> {
    read_all(sources).0
}

pub fn structs(sources: &[(String, String)]) -> Vec<(String, String, String, Vec<String>)> {
    read_all(sources).1
}

#[allow(clippy::type_complexity)]
fn read_all(sources: &[(String, String)]) -> (Vec<Item>, Vec<(String, String, String, Vec<String>)>) {
    let mut all: Vec<Decl> = Vec::new();
    for (file, text) in sources {
        let stem = std::path::Path::new(file).file_stem().and_then(|s| s.to_str()).unwrap_or(file);
        all.extend(decls(&tokens(&blank(text)), stem));
    }
    let mut ty = Types::default();
    for d in &all {
        if !ty.contracts.contains(&d.contract) {
            ty.contracts.push(d.contract.clone());
        }
    }
    for d in &all {
        match d.toks.first().map(String::as_str) {
            Some("struct") => {
                let name = d.toks[1].clone();
                let open = d.toks.iter().position(|t| t == "{").expect("a struct body");
                let body = &d.toks[open + 1..d.toks.len() - 1];
                let fields: Vec<Vec<String>> = body
                    .split(|t| t == ";")
                    .filter(|f| !f.is_empty())
                    .map(|f| f[..f.len() - 1].to_vec())
                    .collect();
                ty.structs.insert(format!("{}.{name}", d.contract), fields.clone());
                ty.structs.entry(name).or_insert(fields);
            }
            Some("enum") => {
                let name = d.toks[1].clone();
                ty.enums.push(format!("{}.{name}", d.contract));
                ty.enums.push(name);
            }
            _ => {}
        }
    }
    let mut tuples = Vec::new();
    for d in &all {
        if d.toks.first().map(String::as_str) == Some("struct") {
            let name = d.toks[1].clone();
            let open = d.toks.iter().position(|t| t == "{").expect("a struct body");
            let fields: Vec<String> = d.toks[open + 1..d.toks.len() - 1]
                .split(|t| t == ";")
                .filter(|f| !f.is_empty())
                .map(|f| f.last().expect("a field has a name").clone())
                .collect();
            tuples.push((d.contract.clone(), name.clone(), ty.canon(&[name], &d.contract), fields));
        }
    }
    let mut out = Vec::new();
    for d in &all {
        let Some(head) = d.toks.first().map(String::as_str) else { continue };
        let c = d.contract.as_str();
        let sig_of = |name: &str, ps: &[Param]| {
            let tys: Vec<String> = ps.iter().filter(|p| !p.ty.is_empty()).map(|p| ty.canon(&p.ty, c)).collect();
            format!("{name}({})", tys.join(","))
        };
        match head {
            "function" => {
                let name = d.toks[1].clone();
                let (inside, after) = group(&d.toks, 2);
                let rest = &d.toks[after..];
                let visible = rest.iter().any(|t| t == "external" || t == "public");
                if !visible {
                    continue;
                }
                let ps = params(&inside);
                let (returns, return_names) = match rest.iter().position(|t| t == "returns") {
                    Some(r) => {
                        let (ret, _) = group(rest, r + 1);
                        let rs: Vec<Param> = params(&ret).into_iter().filter(|p| !p.ty.is_empty()).collect();
                        let tys: Vec<String> = rs.iter().map(|p| ty.canon(&p.ty, c)).collect();
                        (format!("({})", tys.join(",")), rs.into_iter().map(|p| p.name).collect())
                    }
                    None => ("()".to_string(), Vec::new()),
                };
                out.push(Item { contract: c.into(), kind: "function", sig: sig_of(&name, &ps), name, indexed: Vec::new(), returns, return_names });
            }
            "event" | "error" => {
                let name = d.toks[1].clone();
                let (inside, _) = group(&d.toks, 2);
                let ps = params(&inside);
                let indexed = if head == "event" { ps.iter().filter(|p| !p.ty.is_empty()).map(|p| p.indexed).collect() } else { Vec::new() };
                out.push(Item { contract: c.into(), kind: if head == "event" { "event" } else { "error" }, sig: sig_of(&name, &ps), name, indexed, returns: String::new(), return_names: Vec::new() });
            }
            "struct" | "enum" | "modifier" | "constructor" | "using" | "receive" | "fallback" | "pragma" | "import" => {}
            _ => {
                let Some(p) = d.toks.iter().position(|t| t == "public") else { continue };
                let eq = d.toks[p..].iter().position(|t| t == "=").map(|e| p + e).unwrap_or(d.toks.len());
                let name = d.toks[p..eq]
                    .iter()
                    .rev()
                    .find(|t| t.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_') && !matches!(t.as_str(), "immutable" | "constant" | "public" | "override"))
                    .expect("a variable name")
                    .clone();
                let mut args: Vec<String> = Vec::new();
                let mut cur: Vec<String> = d.toks[..p].to_vec();
                while cur.first().map(String::as_str) == Some("mapping") {
                    let (inside, _) = group(&cur, 1);
                    let arrow = inside.iter().position(|t| t == "=").expect("a mapping arrow");
                    args.push(ty.canon(&inside[..arrow], c));
                    cur = inside[arrow + 2..].to_vec();
                }
                let mut val = cur;
                while val.last().map(String::as_str) == Some("]") {
                    let open = val.iter().rposition(|t| t == "[").expect("an array suffix opens");
                    args.push("uint256".into());
                    val.truncate(open);
                }
                let returns = format!("({})", ty.canon(&val, c));
                out.push(Item { contract: c.into(), kind: "function", sig: format!("{name}({})", args.join(",")), name, indexed: Vec::new(), returns, return_names: Vec::new() });
            }
        }
    }
    (out, tuples)
}

pub fn upper_snake(name: &str) -> String {
    if name.chars().all(|c| !c.is_ascii_lowercase()) {
        return name.to_string();
    }
    let mut out = String::new();
    let cs: Vec<char> = name.chars().collect();
    for (i, c) in cs.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 && (cs[i - 1].is_ascii_lowercase() || cs.get(i + 1).is_some_and(|n| n.is_ascii_lowercase()) && cs[i - 1].is_ascii_uppercase()) {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}
