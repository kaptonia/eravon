use std::collections::BTreeMap;

use eravon_galeed as k1;
use eravon_galeed::Value;
use eravon_terms as k2;

use crate::listing;
use crate::token::Verdict;

pub type Pos = (u64, u64);

#[derive(Clone, Debug)]
pub struct Rec {
    pub pos: Pos,
    pub value: Value,
}

impl Rec {
    pub fn s(&self, k: &str) -> &str {
        self.value.get(k).and_then(Value::as_str).unwrap_or("")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Records {
    pub chain_id: u64,
    pub posted: Vec<Rec>,
    pub consented: Vec<Rec>,
    pub revoked: Vec<Rec>,
    pub pulled: Vec<Rec>,
    pub opened: Vec<Rec>,
    pub events: Vec<Rec>,
    pub book: k1::book::Logs,
}

#[derive(Clone, Copy)]
enum F {
    A,
    D,
    N,
    B,
    S,
}

const KINDS: [(&str, &[(&str, F)]); 8] = [
    ("posted", &[("poster", F::A), ("key", F::D), ("hash", F::D), ("document", F::B)]),
    ("consented", &[("who", F::A), ("digest", F::D)]),
    ("revoked", &[("seller", F::A), ("termsHash", F::D)]),
    ("pulled", &[("seller", F::A), ("fund", F::A), ("token", F::A), ("instance", F::A), ("amount", F::N)]),
    ("opened", &[("emitter", F::A), ("caseId", F::D), ("seller", F::A), ("buyer", F::A), ("judge", F::A), ("amount", F::N), ("termsHash", F::D)]),
    ("events", &[("emitter", F::A), ("name", F::S)]),
    ("declarations", &[("from", F::A), ("to", F::A)]),
    ("accessions", &[("holder", F::A), ("to", F::A), ("root", F::A)]),
];

fn form_ok(f: F, v: &Value) -> bool {
    let Some(s) = v.as_str() else { return false };
    match f {
        F::A => k1::hexform::is_hex20(s),
        F::D => k1::hexform::is_hex32(s),
        F::N => k2::u256::U256::from_dec(s).is_some(),
        F::B => s.strip_prefix("0x").is_some_and(|d| d.len() % 2 == 0 && d.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))),
        F::S => true,
    }
}

impl Records {
    pub fn read(v: &Value) -> Option<Records> {
        let m = v.as_obj()?;
        if m.len() != 1 + KINDS.len() || !m.iter().all(|(k, _)| k == "chainId" || KINDS.iter().any(|(n, _)| n == k)) {
            return None;
        }
        let Value::Int(chain_id) = v.get("chainId")? else { return None };
        let mut out = Records { chain_id: *chain_id, ..Records::default() };
        let mut seen = std::collections::BTreeSet::new();
        for (kind, fields) in KINDS {
            let mut list = Vec::new();
            for el in v.get(kind)?.as_arr()? {
                let em = el.as_obj()?;
                if em.len() != 2 + fields.len()
                    || !em.iter().all(|(k, _)| k == "blockNumber" || k == "logIndex" || fields.iter().any(|(f, _)| f == k))
                {
                    return None;
                }
                let (Value::Int(bn), Value::Int(li)) = (el.get("blockNumber")?, el.get("logIndex")?) else { return None };
                if !fields.iter().all(|(f, form)| el.get(f).is_some_and(|x| form_ok(*form, x))) {
                    return None;
                }
                if !seen.insert((*bn, *li)) {
                    return None;
                }
                list.push(Rec { pos: (*bn, *li), value: el.clone() });
            }
            match kind {
                "posted" => out.posted = list,
                "consented" => out.consented = list,
                "revoked" => out.revoked = list,
                "pulled" => out.pulled = list,
                "opened" => out.opened = list,
                "events" => out.events = list,
                "declarations" => {
                    out.book.declarations = list
                        .iter()
                        .map(|r| k1::book::Declaration { emitter: r.s("from").into(), block_number: r.pos.0, log_index: r.pos.1, to: r.s("to").into() })
                        .collect()
                }
                _ => {
                    out.book.accessions = list
                        .iter()
                        .map(|r| k1::book::Accession {
                            emitter: r.s("to").into(),
                            block_number: r.pos.0,
                            log_index: r.pos.1,
                            holder: r.s("holder").into(),
                            root: r.s("root").into(),
                        })
                        .collect()
                }
            }
        }
        Some(out)
    }

    pub fn stands(&self, k: &str, p: Pos) -> bool {
        k1::book::current_at(&self.book, k, p) == k
    }

    pub fn keys_of(&self, a: &str) -> Vec<String> {
        k1::book::record_keys(&self.book, a)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Own,
    Opened,
    Revoked,
}

impl Source {
    pub fn word(self) -> &'static str {
        match self {
            Source::Own => "self",
            Source::Opened => "opened",
            Source::Revoked => "revoked",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    pub seller: String,
    pub terms: String,
    pub postings: Vec<Rec>,
    pub opened: Vec<Rec>,
    pub revoked: Vec<Rec>,
    pub position: Pos,
    pub source: Source,
    pub stands: bool,
}

fn document_bytes(r: &Rec) -> Vec<u8> {
    k1::hexform::decode_digits(&r.s("document").as_bytes()[2..]).unwrap_or_default()
}

fn own_posting(r: &Rec) -> Option<(String, String)> {
    let s = listing::listing(&document_bytes(r)).ok()?;
    (r.s("key") == s.terms_hash && r.s("poster") == s.order.seller_key).then(|| (s.order.seller_key, s.terms_hash))
}

pub fn documents(rs: &Records) -> BTreeMap<(String, String), Document> {
    fn get(docs: &mut BTreeMap<(String, String), Document>, key: (String, String)) -> &mut Document {
        docs.entry(key.clone()).or_insert_with(|| Document {
            seller: key.0,
            terms: key.1,
            postings: Vec::new(),
            opened: Vec::new(),
            revoked: Vec::new(),
            position: (0, 0),
            source: Source::Own,
            stands: true,
        })
    }
    let mut docs: BTreeMap<(String, String), Document> = BTreeMap::new();
    for r in &rs.posted {
        if let Some(key) = own_posting(r) {
            get(&mut docs, key).postings.push(r.clone());
        }
    }
    for r in &rs.opened {
        get(&mut docs, (r.s("seller").into(), r.s("termsHash").into())).opened.push(r.clone());
    }
    for r in &rs.revoked {
        get(&mut docs, (r.s("seller").into(), r.s("termsHash").into())).revoked.push(r.clone());
    }
    for d in docs.values_mut() {
        for l in [&mut d.postings, &mut d.opened, &mut d.revoked] {
            l.sort_by_key(|r| r.pos);
        }
        (d.position, d.source, d.stands) = placed(rs, d);
    }
    docs
}

fn placed(rs: &Records, d: &Document) -> (Pos, Source, bool) {
    if let Some(first) = d.postings.first() {
        return (first.pos, Source::Own, rs.stands(first.s("poster"), first.pos));
    }
    match d.opened.first() {
        Some(o) => (o.pos, Source::Opened, true),
        None => (d.revoked.first().map(|r| r.pos).unwrap_or_default(), Source::Revoked, true),
    }
}


pub fn place(rs: &Records, seller: &str, terms_hash: &str) -> Option<(Pos, Source)> {
    documents(rs).remove(&(seller.to_string(), terms_hash.to_string())).map(|d| (d.position, d.source))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pay {
    Payable,
    SellerKeyRetired,
    Unplaced,
}

impl Pay {
    pub fn word(self) -> &'static str {
        match self {
            Pay::Payable => "PAYABLE",
            Pay::SellerKeyRetired => "SELLER_KEY_RETIRED",
            Pay::Unplaced => "UNPLACED",
        }
    }
}

pub fn pay_check(rs: &Records, sb: &[u8]) -> Result<Pay, Verdict> {
    let s = listing::listing(sb).map_err(|f| Verdict::new("LISTING_INVALID").own(f))?;
    let first = rs
        .posted
        .iter()
        .filter(|r| r.s("key") == s.terms_hash && r.s("poster") == s.order.seller_key && document_bytes(r) == sb)
        .min_by_key(|r| r.pos);
    Ok(match first {
        None => Pay::Unplaced,
        Some(r) if posting_stands(rs, r) => Pay::Payable,
        Some(_) => Pay::SellerKeyRetired,
    })
}

fn posting_stands(rs: &Records, r: &Rec) -> bool {
    rs.stands(r.s("poster"), r.pos)
}


#[derive(Clone, Debug)]
pub struct History {
    pub keys: Vec<String>,
    pub documents: Vec<Document>,
    pub consents: Vec<Rec>,
    pub cases: Vec<Rec>,
}

impl History {
    pub fn attempts(&self) -> impl Iterator<Item = &Document> {
        self.documents.iter().filter(|d| !d.stands)
    }
}

pub fn history(rs: &Records, a: &str) -> History {
    let keys = record_keys(rs, a);
    let mine = |k: &str| keys.iter().any(|x| x == k);
    let mut documents: Vec<Document> = documents(rs).into_values().filter(|d| mine(&d.seller)).collect();
    documents.sort_by(|x, y| (x.position, &x.terms).cmp(&(y.position, &y.terms)));
    let mut consents: Vec<Rec> = rs.consented.iter().filter(|r| mine(r.s("who"))).cloned().collect();
    consents.sort_by_key(|r| r.pos);
    let mut cases: Vec<Rec> = rs.opened.iter().filter(|r| mine(r.s("buyer"))).cloned().collect();
    cases.sort_by_key(|r| r.pos);
    History { keys, documents, consents, cases }
}

fn record_keys(rs: &Records, a: &str) -> Vec<String> {
    rs.keys_of(a)
}


pub fn case_records(rs: &Records, opened: &Rec) -> Vec<Rec> {
    let inst = opened.s("emitter");
    let mut out: Vec<Rec> = rs.opened.iter().chain(&rs.events).filter(|r| r.s("emitter") == inst).cloned().collect();
    out.extend(rs.pulled.iter().filter(|r| r.s("instance") == inst).cloned());
    out.sort_by_key(|r| r.pos);
    out
}

pub fn position_value(p: Pos, source: Option<Source>) -> Value {
    let mut m = vec![("blockNumber".to_string(), Value::Int(p.0)), ("logIndex".to_string(), Value::Int(p.1))];
    m.extend(source.map(|x| ("source".to_string(), Value::Str(x.word().into()))));
    Value::Obj(m)
}

pub fn case_value(rs: &Records, o: &Rec) -> Value {
    Value::Obj(vec![
        ("buyer".to_string(), Value::Str(o.s("buyer").into())),
        ("caseId".to_string(), Value::Str(o.s("caseId").into())),
        ("instance".to_string(), Value::Str(o.s("emitter").into())),
        ("position".into(), position_value(o.pos, None)),
        ("records".into(), Value::Arr(case_records(rs, o).into_iter().map(|r| r.value).collect())),
    ])
}

pub fn document_value(rs: &Records, d: &Document) -> Value {
    let standing = |b: bool| Value::Str(if b { "stands" } else { "attempt" }.into());
    let postings = d
        .postings
        .iter()
        .map(|r| {
            let mut m = r.value.as_obj().map(<[_]>::to_vec).unwrap_or_default();
            m.push(("standing".into(), standing(rs.stands(r.s("poster"), r.pos))));
            Value::Obj(m)
        })
        .collect();
    Value::Obj(vec![
        ("cases".into(), Value::Arr(d.opened.iter().map(|o| case_value(rs, o)).collect())),
        ("position".into(), position_value(d.position, Some(d.source))),
        ("postings".into(), Value::Arr(postings)),
        ("revoked".into(), Value::Arr(d.revoked.iter().map(|r| r.value.clone()).collect())),
        ("sellerKey".to_string(), Value::Str(d.seller.clone())),
        ("source".to_string(), Value::Str(d.source.word().into())),
        ("standing".into(), standing(d.stands)),
        ("termsHash".to_string(), Value::Str(d.terms.clone())),
    ])
}

pub fn history_members(rs: &Records, h: &History) -> Vec<(String, Value)> {
    vec![
        ("attempts".into(), Value::Arr(h.attempts().map(|d| document_value(rs, d)).collect())),
        ("cases".into(), Value::Arr(h.cases.iter().map(|o| case_value(rs, o)).collect())),
        ("consents".into(), Value::Arr(h.consents.iter().map(|r| r.value.clone()).collect())),
        ("documents".into(), Value::Arr(h.documents.iter().map(|d| document_value(rs, d)).collect())),
        ("keys".into(), Value::Arr(h.keys.iter().map(|k| Value::Str(k.clone())).collect())),
    ]
}
