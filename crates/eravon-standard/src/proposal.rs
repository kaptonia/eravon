use eravon_literals::constants as k7;
use eravon_literals::json::In;
use eravon_terms as k2;

use crate::abi::{self, Val};
use crate::call::Call;
use crate::fault::{Fault, R};
use crate::order;
use crate::sol;

pub const V: &str = eravon_literals::literals::DOC_CHELEK_PROPOSAL;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PCall {
    pub to: [u8; 20],
    pub value: [u8; 32],
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    pub sha256: [u8; 32],
    pub kind: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub holder: [u8; 20],
    pub payer: [u8; 20],
    pub seats: u64,
    pub amount: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    pub chain: u64,
    pub window: Option<u64>,
    pub calls: Vec<PCall>,
    pub documents: Vec<Doc>,
    pub token: Option<[u8; 20]>,
    pub share: Option<[u8; 32]>,
    pub issue: Option<Vec<Issue>>,
    pub tags: Vec<String>,
    pub note_md: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Original {
    pub kind: String,
    pub name: String,
    pub bytes: Vec<u8>,
}

const MAX_INT: u64 = (1u64 << 53) - 1;

fn skeleton(s: &str) -> bool {
    s.chars().all(|c| ('\u{20}'..='\u{7e}').contains(&c))
}

fn symbol(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| ('\u{21}'..='\u{7e}').contains(&c))
}

fn zero20(a: &[u8; 20]) -> bool {
    a.iter().all(|b| *b == 0)
}

fn zero32(a: &[u8; 32]) -> bool {
    a.iter().all(|b| *b == 0)
}

fn bad(token: &str, why: impl Into<String>) -> Fault {
    Fault::Form(format!("{token}:{}", why.into()))
}

impl Proposal {
    pub fn check(&self) -> R<()> {
        if self.chain > MAX_INT {
            return Err(bad("E_PROPOSAL_CHAIN", "chain 超出 2^53 − 1"));
        }
        if let Some(w) = self.window {
            if w == 0 || w > MAX_INT {
                return Err(bad("E_PROPOSAL_WINDOW", "window 至少 1 且不过 2^53 − 1"));
            }
        }
        for (i, c) in self.calls.iter().enumerate() {
            if zero20(&c.to) {
                return Err(bad("E_CALL_TO", format!("第 {i} 条调用的 to 是零地址")));
            }
        }
        for (i, d) in self.documents.iter().enumerate() {
            if zero32(&d.sha256) {
                return Err(bad("E_DOCUMENT_SHA256", format!("第 {i} 份文书的 sha256 是零")));
            }
            if !symbol(&d.kind) {
                return Err(bad("E_DOCUMENT_KIND", format!("第 {i} 份文书的 kind 不是 symbol")));
            }
            if d.name.is_empty() || !skeleton(&d.name) {
                return Err(bad("E_DOCUMENT_NAME", format!("第 {i} 份文书的 name 不是非空骨架串")));
            }
            if self.documents[..i].iter().any(|x| x.sha256 == d.sha256) {
                return Err(bad("E_PROPOSAL_DUP_DOCUMENT", format!("第 {i} 份文书的 sha256 前面已有")));
            }
        }
        if let Some(t) = &self.token {
            if zero20(t) {
                return Err(bad("E_PROPOSAL_TOKEN", "token 是零地址"));
            }
        }
        if self.token.is_some() != self.share.is_some() {
            return Err(bad("E_PROPOSAL_CONTRIBUTION", "token 与 share 要么都在要么都缺"));
        }
        if let Some(is) = &self.issue {
            if is.is_empty() {
                return Err(bad("E_PROPOSAL_ISSUE", "issue 在即不许空"));
            }
            if self.token.is_none() {
                return Err(bad("E_PROPOSAL_ISSUE_TOKEN", "issue 要有 token"));
            }
            for (i, e) in is.iter().enumerate() {
                if zero20(&e.holder) || zero20(&e.payer) || e.seats == 0 || e.seats > MAX_INT {
                    return Err(bad("E_ISSUE", format!("第 {i} 位认购人的形不合")));
                }
                if is[..i].iter().any(|x| x.holder == e.holder) {
                    return Err(bad("E_PROPOSAL_DUP_ISSUE", format!("第 {i} 位认购人前面已有")));
                }
            }
        }
        if self.share.as_ref().is_some_and(zero32) && self.issue.is_none() {
            return Err(bad("E_PROPOSAL_SHARE_ZERO", "share 为零只许伴 issue"));
        }
        if self.calls.is_empty() && self.token.is_none() {
            return Err(bad("E_PROPOSAL_EMPTY", "一份提案至少做一件事"));
        }
        for (i, t) in self.tags.iter().enumerate() {
            if t.is_empty() || !skeleton(t) {
                return Err(bad("E_TAG", format!("第 {i} 枚标签不是非空骨架串")));
            }
        }
        Ok(())
    }

    pub fn bytes(&self) -> R<Vec<u8>> {
        self.check()?;
        let calls = J::A(
            self.calls
                .iter()
                .map(|c| J::O(vec![("data", J::S(order::hex_of(&c.data))), ("to", addr_s(&c.to)), ("value", dec_s(&c.value))]))
                .collect(),
        );
        let docs = J::A(
            self.documents
                .iter()
                .map(|d| J::O(vec![("kind", J::S(d.kind.clone())), ("name", J::S(d.name.clone())), ("sha256", J::S(order::hex_of(&d.sha256)))]))
                .collect(),
        );
        let mut m = vec![
            ("calls", calls),
            ("chain", J::N(self.chain)),
            ("documents", docs),
            ("note_md", J::S(self.note_md.clone())),
            ("tags", J::A(self.tags.iter().map(|t| J::S(t.clone())).collect())),
            ("v", J::S(V.to_string())),
        ];
        if let Some(w) = self.window {
            m.push(("window", J::N(w)));
        }
        if let Some(t) = &self.token {
            m.push(("token", addr_s(t)));
        }
        if let Some(s) = &self.share {
            m.push(("share", dec_s(s)));
        }
        if let Some(is) = &self.issue {
            m.push((
                "issue",
                J::A(
                    is.iter()
                        .map(|e| {
                            J::O(vec![
                                ("amount", dec_s(&e.amount)),
                                ("holder", addr_s(&e.holder)),
                                ("payer", addr_s(&e.payer)),
                                ("seats", J::N(e.seats)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        let mut out = Vec::new();
        J::O(m).put(&mut out);
        Ok(out)
    }

    pub fn digest(&self) -> [u8; 32] {
        let kk = k2::derive::keccak;
        let th_call = kk(b"Call(address to,uint256 value,bytes data)");
        let th_issue = kk(b"Issue(address holder,address payer,uint256 seats,uint256 amount)");
        let th_prop = kk(b"Proposal(uint256 chain,uint256 window,Call[] calls,address token,uint256 share,Issue[] issue)Call(address to,uint256 value,bytes data)Issue(address holder,address payer,uint256 seats,uint256 amount)");
        let mut ch = Vec::new();
        for c in &self.calls {
            let mut b = th_call.to_vec();
            b.extend_from_slice(&word20(&c.to));
            b.extend_from_slice(&c.value);
            b.extend_from_slice(&call_data_hash(&c.data));
            ch.extend_from_slice(&kk(&b));
        }
        let mut ih = Vec::new();
        for e in self.issue.as_deref().unwrap_or(&[]) {
            let mut b = th_issue.to_vec();
            b.extend_from_slice(&word20(&e.holder));
            b.extend_from_slice(&word20(&e.payer));
            b.extend_from_slice(&word64(e.seats));
            b.extend_from_slice(&e.amount);
            ih.extend_from_slice(&kk(&b));
        }
        let mut b = th_prop.to_vec();
        b.extend_from_slice(&word64(self.chain));
        b.extend_from_slice(&word64(self.window.unwrap_or(0)));
        b.extend_from_slice(&kk(&ch));
        b.extend_from_slice(&word20(&self.token.unwrap_or([0u8; 20])));
        b.extend_from_slice(&self.share.unwrap_or([0u8; 32]));
        b.extend_from_slice(&kk(&ih));
        kk(&b)
    }

    pub fn structural(&self) -> bool {
        if self.issue.is_some() {
            return true;
        }
        let sels: Vec<[u8; 4]> = STRUCTURAL.iter().map(|s| sel4(s)).collect();
        self.calls.iter().any(|c| c.data.len() >= 4 && sels.iter().any(|s| c.data[..4] == s[..]))
    }

    pub fn contribution(&self) -> bool {
        self.token.is_some()
    }
}

const STRUCTURAL: [&str; 8] = [
    "setSchedule(uint256,uint256,uint256[40])",
    "setSuccessor(address)",
    "appoint(address,string[])",
    "setRetention(address,uint16,uint256,uint32)",
    "affirm(uint256)",
    "transferSeats(address,uint256)",
    "bindWallet(address,address)",
    "allow(address,address,uint256)",
];

fn sel4(s: &str) -> [u8; 4] {
    let h = k2::derive::keccak(s.as_bytes());
    [h[0], h[1], h[2], h[3]]
}

fn word20(a: &[u8; 20]) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[12..].copy_from_slice(a);
    w
}

fn word64(n: u64) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[24..].copy_from_slice(&n.to_be_bytes());
    w
}

fn call_data_hash(data: &[u8]) -> [u8; 32] {
    k2::derive::keccak(data)
}


fn addr_s(a: &[u8; 20]) -> J {
    J::S(order::hex_of(a))
}

fn dec_s(w: &[u8; 32]) -> J {
    J::S(k2::u256::U256::from_be(w).to_dec())
}

enum J {
    N(u64),
    S(String),
    A(Vec<J>),
    O(Vec<(&'static str, J)>),
}

impl J {
    fn put(&self, out: &mut Vec<u8>) {
        match self {
            J::N(n) => out.extend_from_slice(n.to_string().as_bytes()),
            J::S(s) => put_str(s, out),
            J::A(v) => {
                out.push(b'[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    x.put(out);
                }
                out.push(b']');
            }
            J::O(m) => {
                let mut idx: Vec<usize> = (0..m.len()).collect();
                idx.sort_by(|a, b| m[*a].0.as_bytes().cmp(m[*b].0.as_bytes()));
                out.push(b'{');
                for (i, k) in idx.into_iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    put_str(m[k].0, out);
                    out.push(b':');
                    m[k].1.put(out);
                }
                out.push(b'}');
            }
        }
    }
}

fn put_str(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{08}' => out.extend_from_slice(b"\\b"),
            '\u{09}' => out.extend_from_slice(b"\\t"),
            '\u{0a}' => out.extend_from_slice(b"\\n"),
            '\u{0c}' => out.extend_from_slice(b"\\f"),
            '\u{0d}' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes()),
            c => {
                let mut b = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
            }
        }
    }
    out.push(b'"');
}

fn s_of<'a>(v: &'a In, k: &str) -> R<&'a str> {
    v.get(k).and_then(In::as_str).ok_or_else(|| bad("E_PROPOSAL_READ", format!("{k} 不是字符串或缺")))
}

fn addr_in(s: &str, k: &str) -> R<[u8; 20]> {
    if s.len() != 42 || !s.starts_with("0x") || !s[2..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err(bad("E_PROPOSAL_READ", format!("{k} 不是 hex20")));
    }
    order::addr_of(k, s)
}

fn dec_in(s: &str, k: &str) -> R<[u8; 32]> {
    let ok = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'));
    let v = if ok { k2::u256::U256::from_dec(s) } else { None };
    v.map(|u| u.to_be()).ok_or_else(|| bad("E_PROPOSAL_READ", format!("{k} 不是 dec256")))
}

fn int_in(v: &In, k: &str) -> R<u64> {
    match v.get(k) {
        Some(In::Int(n)) if *n >= 0 => Ok(*n as u64),
        _ => Err(bad("E_PROPOSAL_READ", format!("{k} 不是非负整数"))),
    }
}

fn arr<'a>(v: &'a In, k: &str) -> R<&'a [In]> {
    v.get(k).and_then(In::as_arr).ok_or_else(|| bad("E_PROPOSAL_READ", format!("{k} 不是数组或缺")))
}

pub fn read(b: &[u8]) -> R<Proposal> {
    let v = eravon_literals::json::read(b).map_err(|e| bad("E_PROPOSAL_READ", format!("读不成 JSON:{e}")))?;
    if s_of(&v, "v")? != V {
        return Err(bad("E_PROPOSAL_V", "v 不是 chelek.proposal/1"));
    }
    let mut calls = Vec::new();
    for c in arr(&v, "calls")? {
        let d = s_of(c, "data")?;
        calls.push(PCall { to: addr_in(s_of(c, "to")?, "to")?, value: dec_in(s_of(c, "value")?, "value")?, data: order::bytes_of("data", d)? });
    }
    let mut documents = Vec::new();
    for d in arr(&v, "documents")? {
        documents.push(Doc {
            sha256: order::b32_of("sha256", s_of(d, "sha256")?)?,
            kind: s_of(d, "kind")?.to_string(),
            name: s_of(d, "name")?.to_string(),
        });
    }
    let token = match v.get("token") {
        None => None,
        Some(_) => Some(addr_in(s_of(&v, "token")?, "token")?),
    };
    let share = match v.get("share") {
        None => None,
        Some(_) => Some(dec_in(s_of(&v, "share")?, "share")?),
    };
    let issue = match v.get("issue") {
        None => None,
        Some(_) => {
            let mut out = Vec::new();
            for e in arr(&v, "issue")? {
                out.push(Issue {
                    holder: addr_in(s_of(e, "holder")?, "holder")?,
                    payer: addr_in(s_of(e, "payer")?, "payer")?,
                    seats: int_in(e, "seats")?,
                    amount: dec_in(s_of(e, "amount")?, "amount")?,
                });
            }
            Some(out)
        }
    };
    let tags = arr(&v, "tags")?
        .iter()
        .map(|t| t.as_str().map(str::to_string).ok_or_else(|| bad("E_TAG", "标签不是字符串")))
        .collect::<R<Vec<_>>>()?;
    let p = Proposal {
        chain: int_in(&v, "chain")?,
        window: match v.get("window") {
            None => None,
            Some(_) => Some(int_in(&v, "window")?),
        },
        calls,
        documents,
        token,
        share,
        issue,
        tags,
        note_md: s_of(&v, "note_md")?.to_string(),
    };
    if p.bytes()? != b {
        return Err(bad("E_NOT_CANONICAL", "重写一遍与原字节不同:不是正典形,或带了本法之外的成员"));
    }
    Ok(p)
}

pub fn of_call(c: &Call) -> R<PCall> {
    Ok(PCall { to: order::addr_of("收件地址", &c.to)?, value: [0u8; 32], data: c.data.clone() })
}

fn wallet_sig(name: &str, want: &[&str]) -> R<String> {
    let doc = eravon_literals::json::read(k7::CHELEK_ABI.wallet.as_bytes())
        .map_err(|e| Fault::Abi(format!("K7 的钱包 ABI 读不成:{e}")))?;
    let item = doc
        .as_arr()
        .unwrap_or(&[])
        .iter()
        .find(|x| x.get("type").and_then(In::as_str) == Some("function") && x.get("name").and_then(In::as_str) == Some(name))
        .ok_or_else(|| Fault::Abi(format!("K7 的钱包 ABI 里没有 {name}")))?;
    let tys: Vec<&str> = item
        .get("inputs")
        .and_then(In::as_arr)
        .unwrap_or(&[])
        .iter()
        .filter_map(|p| p.get("type").and_then(In::as_str))
        .collect();
    if tys != want {
        return Err(Fault::Abi(format!("K7 钱包 ABI 的 {name} 形是 {tys:?},不是 {want:?}")));
    }
    Ok(format!("{name}({})", tys.join(",")))
}

fn pay_sig() -> R<String> {
    wallet_sig("pay", &["address", "address", "uint256"])
}

fn wallet_call(wallet: &str, sig: &str, args: &[Val]) -> R<(PCall, Original)> {
    let h = k2::derive::keccak(sig.as_bytes());
    let mut data = h[..4].to_vec();
    data.extend_from_slice(&abi::encode(args));
    let call = PCall { to: order::addr_of("公司钱包", wallet)?, value: [0u8; 32], data };
    Ok((call, Original { kind: "abi".into(), name: WALLET_ABI_NAME.into(), bytes: k7::CHELEK_ABI.wallet.as_bytes().to_vec() }))
}

pub fn wallet_fund(wallet: &str, logic: &str) -> R<(PCall, Original)> {
    let sig = wallet_sig("fund", &["address"])?;
    wallet_call(wallet, &sig, &[Val::Addr(order::addr_of("逻辑合约", logic)?)])
}

pub fn wallet_allow(wallet: &str, token: &str, logic: &str, amount: &[u8; 32]) -> R<(PCall, Original)> {
    let sig = wallet_sig("allow", &["address", "address", "uint256"])?;
    wallet_call(
        wallet,
        &sig,
        &[Val::Addr(order::addr_of("币种", token)?), Val::Addr(order::addr_of("逻辑合约", logic)?), Val::U256(*amount)],
    )
}

pub fn wallet_pay(wallet: &str, token: &str, to: &str, amount: &[u8; 32]) -> R<(PCall, Original)> {
    let sig = pay_sig()?;
    let h = k2::derive::keccak(sig.as_bytes());
    let (a, b) = pay_order(order::addr_of("币种", token)?, order::addr_of("收款地址", to)?);
    let mut data = h[..4].to_vec();
    data.extend_from_slice(&abi::encode(&[Val::Addr(a), Val::Addr(b), Val::U256(*amount)]));
    let call = PCall { to: order::addr_of("公司钱包", wallet)?, value: [0u8; 32], data };
    Ok((call, Original { kind: "abi".into(), name: WALLET_ABI_NAME.into(), bytes: k7::CHELEK_ABI.wallet.as_bytes().to_vec() }))
}

pub const WALLET_ABI_NAME: &str = "chelek-wallet.abi.json";

fn pay_order(token: [u8; 20], to: [u8; 20]) -> ([u8; 20], [u8; 20]) {
    (token, to)
}


pub fn consent(logic: &str, party: &str, digest: &[u8; 32], original: Original) -> R<(PCall, Vec<Original>)> {
    let call = PCall {
        to: order::addr_of("逻辑合约", logic)?,
        value: [0u8; 32],
        data: crate::call::with(sol::CONSENT, &[Val::Addr(order::addr_of("同意方", party)?), Val::B32(*digest)])?,
    };
    let mut docs = vec![abi_doc(&[sol::CONSENT], LOGIC_ABI_NAME)?];
    docs.extend(keep_original(original));
    Ok((call, docs))
}

pub fn consent_at(logic: &str, party: &str, digest: &[u8; 32], original: Original) -> R<(PCall, Vec<Original>)> {
    match k7::generation_of_logic(logic).map_or(k7::Shape::Current, |g| g.shape()) {
        k7::Shape::Current => consent(logic, party, digest, original),
        k7::Shape::Trial => {
            let call = PCall { to: order::addr_of("逻辑合约", logic)?, value: [0u8; 32], data: crate::call::with(sol::TRIAL_CONSENT, &[Val::B32(*digest)])? };
            let mut docs = vec![abi_doc(&[sol::TRIAL_CONSENT], LOGIC_ABI_NAME)?];
            docs.extend(keep_original(original));
            Ok((call, docs))
        }
    }
}

pub fn board_post(key: &[u8; 32], bytes: &[u8]) -> R<(PCall, Original)> {
    let call = of_call(&crate::call::post(eravon_literals::constants::BOARD, key, bytes)?)?;
    Ok((call, abi_doc(&[sol::POST], BOARD_ABI_NAME)?))
}

pub const BOARD_ABI_NAME: &str = "eravon-board.abi.json";

pub const LOGIC_ABI_NAME: &str = "eravon-logic.abi.json";
pub const INSTANCE_ABI_NAME: &str = "eravon-instance.abi.json";

pub const EVIDENCE_DOC_KIND: &str = "evidence";
pub const EVIDENCE_DOC_NAME: &str = "evidence.json";

fn keep_original(o: Original) -> Option<Original> {
    Some(o)
}


pub fn abi_doc(funs: &[sol::Fun], name: &str) -> R<Original> {
    let mut items = Vec::new();
    for f in funs {
        let inner = f.sig.strip_prefix(f.name).and_then(|r| r.strip_prefix('(')).and_then(|r| r.strip_suffix(')')).ok_or_else(|| Fault::Abi(format!("{} 的签名形不合", f.sig)))?;
        let inputs = J::A(params(inner).map_err(|w| Fault::Abi(format!("{} 的签名形不合:{w}", f.sig)))?);
        items.push(J::O(vec![
            ("inputs", inputs),
            ("name", J::S(f.name.to_string())),
            ("outputs", J::A(Vec::new())),
            ("stateMutability", J::S("nonpayable".into())),
            ("type", J::S("function".into())),
        ]));
    }
    let mut bytes = Vec::new();
    J::A(items).put(&mut bytes);
    Ok(Original { kind: "abi".into(), name: name.to_string(), bytes })
}

fn params(list: &str) -> Result<Vec<J>, String> {
    let mut out = Vec::new();
    let (mut depth, mut from) = (0i32, 0usize);
    let b = list.as_bytes();
    for (i, c) in b.iter().enumerate() {
        match c {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                out.push(param(&list[from..i])?);
                from = i + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return Err("括号不配".into());
        }
    }
    if depth != 0 {
        return Err("括号不配".into());
    }
    if !list[from..].is_empty() {
        out.push(param(&list[from..])?);
    }
    Ok(out)
}

fn param(ty: &str) -> Result<J, String> {
    if !ty.starts_with('(') {
        if ty.is_empty() || ty.contains(')') {
            return Err(format!("参数类型「{ty}」不合"));
        }
        return Ok(J::O(vec![("name", J::S(String::new())), ("type", J::S(ty.to_string()))]));
    }
    let close = ty.rfind(')').ok_or_else(|| "括号不配".to_string())?;
    let suffix = &ty[close + 1..];
    if !suffix.chars().all(|c| c == '[' || c == ']' || c.is_ascii_digit()) {
        return Err(format!("元组后缀「{suffix}」不合"));
    }
    Ok(J::O(vec![
        ("components", J::A(params(&ty[1..close])?)),
        ("name", J::S(String::new())),
        ("type", J::S(format!("tuple{suffix}"))),
    ]))
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub verb: &'static str,
    pub calls: Vec<PCall>,
    pub originals: Vec<Original>,
    pub items: Vec<crate::company::Item>,
    pub face: Vec<String>,
    pub instance: String,
}

pub fn assemble(
    chain: u64,
    calls: Vec<PCall>,
    originals: Vec<Original>,
    tags: Vec<String>,
    note_md: String,
    sha256: &dyn Fn(&[u8]) -> [u8; 32],
) -> R<(Proposal, Vec<Original>)> {
    let mut docs: Vec<Doc> = Vec::new();
    let mut kept: Vec<Original> = Vec::new();
    for o in originals {
        let h = sha256(&o.bytes);
        if docs.iter().any(|d| d.sha256 == h) {
            continue;
        }
        docs.push(Doc { sha256: h, kind: o.kind.clone(), name: o.name.clone() });
        kept.push(o);
    }
    let p = Proposal { chain, window: None, calls, documents: docs, token: None, share: None, issue: None, tags, note_md };
    p.check()?;
    Ok((p, kept))
}

pub fn face_consent(expiry: u64) -> String {
    format!("登记后不可撤回,至 expiry({expiry},Unix 秒)失效。")
}

pub const FACE_OFFER: &str = "要约登记之后,到期之前付款的每一位买家都各开一案,本公司须各自交付或退款。";

pub const FACE_SETTLE_POST: &str = "和解要约随本提案上公告板:同意登记与上板同成或同不成,买家按案号扫板即见。";
pub const FACE_EVIDENCE: &str = "判官可能在这一份承诺落链之前就裁决。";

#[cfg(test)]
mod tests {
    use super::*;

    fn one_call() -> Proposal {
        Proposal {
            chain: 1,
            window: None,
            calls: vec![PCall { to: [0x11; 20], value: [0u8; 32], data: vec![0xab, 0xcd] }],
            documents: vec![],
            token: None,
            share: None,
            issue: None,
            tags: vec![],
            note_md: "é\n\"x\"".into(),
        }
    }

    #[test]
    fn canonical_bytes_follow_2_4() {
        let b = one_call().bytes().expect("是一份提案");
        let want = "{\"calls\":[{\"data\":\"0xabcd\",\"to\":\"0x1111111111111111111111111111111111111111\",\"value\":\"0\"}],\"chain\":1,\"documents\":[],\"note_md\":\"é\\n\\\"x\\\"\",\"tags\":[],\"v\":\"chelek.proposal/1\"}";
        assert_eq!(String::from_utf8(b.clone()).unwrap(), want);
        assert_eq!(read(&b).expect("读得回"), one_call());
    }

    #[test]
    fn an_empty_proposal_is_no_act() {
        let mut p = one_call();
        p.calls.clear();
        assert!(p.bytes().is_err());
    }

    #[test]
    fn a_pay_to_the_wallet_takes_the_k7_shape() {
        let (c, doc) = wallet_pay(&format!("0x{}", "22".repeat(20)), &format!("0x{}", "33".repeat(20)), &format!("0x{}", "44".repeat(20)), &[0u8; 32]).unwrap();
        assert_eq!(&c.data[..4], &k2::derive::keccak(b"pay(address,address,uint256)")[..4]);
        assert_eq!(doc.bytes, k7::CHELEK_ABI.wallet.as_bytes());
    }
}
