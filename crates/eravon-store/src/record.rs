use eravon_literals::json::{In, Out};
use eravon_literals::literals as lit;

use crate::doc;
use crate::fault::{Fault, R};
use crate::seam::{self, Box_};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Product,
    TermsDocument,
    OfferSignature,
    Payload,
    Receipt,
    OrderFile,
    Dossier,
    DeliveryListing,
    Candidates,
    OrderLabel,
    TxReceipt,
    NotificationRead,
}

impl Kind {
    pub const ALL: [Kind; 12] = [
        Kind::Product,
        Kind::TermsDocument,
        Kind::OfferSignature,
        Kind::Payload,
        Kind::Receipt,
        Kind::OrderFile,
        Kind::Dossier,
        Kind::DeliveryListing,
        Kind::Candidates,
        Kind::OrderLabel,
        Kind::TxReceipt,
        Kind::NotificationRead,
    ];

    pub const fn word(self) -> &'static str {
        match self {
            Kind::Product => "product",
            Kind::TermsDocument => lit::DOC_TERMS,
            Kind::OfferSignature => "offer-signature",
            Kind::Payload => lit::DOC_LINK,
            Kind::Receipt => lit::DOC_RECEIPT,
            Kind::OrderFile => lit::DOC_PACK,
            Kind::Dossier => lit::DOC_DOSSIER,
            Kind::DeliveryListing => "delivery-listing",
            Kind::Candidates => "candidates",
            Kind::OrderLabel => "order-label",
            Kind::TxReceipt => "tx-receipt",
            Kind::NotificationRead => "notification-read",
        }
    }

    pub const fn home(self) -> Option<Box_> {
        match self {
            Kind::Product | Kind::TermsDocument | Kind::OfferSignature | Kind::Payload => {
                Some(Box_::Products)
            }
            Kind::Receipt
            | Kind::DeliveryListing
            | Kind::OrderLabel
            | Kind::TxReceipt
            | Kind::NotificationRead => Some(Box_::Orders),
            Kind::Dossier => Some(Box_::Cases),
            Kind::OrderFile => Some(Box_::Packs),
            Kind::Candidates => None,
        }
    }

    pub fn from_word(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.word() == s)
    }

    pub fn need(s: &str) -> R<Kind> {
        Kind::from_word(s).ok_or_else(|| Fault::Kind(format!("不在十二类闭集里:{s:?}")))
    }

    pub fn need_home(self) -> R<Box_> {
        self.home().ok_or_else(|| {
            Fault::Kind(format!("{} 是身份目录里的单档,不经收纳处", self.word()))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub kind: Kind,
    pub created: u64,
    pub updated: u64,
    pub writer: String,
    pub fields: Vec<(String, String)>,
    pub parts: Vec<(String, u64)>,
}

impl Record {
    pub fn seal(&self) -> Vec<u8> {
        let mut fields = self.fields.clone();
        fields.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        let mut parts = self.parts.clone();
        parts.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        doc::seal(
            seam::V_RECORD,
            vec![
                ("createdAt", Out::N(self.created)),
                (
                    "fields",
                    Out::A(
                        fields
                            .into_iter()
                            .map(|(k, v)| Out::O(vec![("k", Out::S(k)), ("v", Out::S(v))]))
                            .collect(),
                    ),
                ),
                ("id", Out::S(self.id.clone())),
                ("kind", Out::S(self.kind.word().to_string())),
                (
                    "parts",
                    Out::A(
                        parts
                            .into_iter()
                            .map(|(n, b)| Out::O(vec![("bytes", Out::N(b)), ("name", Out::S(n))]))
                            .collect(),
                    ),
                ),
                ("updatedAt", Out::N(self.updated)),
                ("writer", Out::S(self.writer.clone())),
            ],
        )
    }

    pub fn open(bytes: &[u8]) -> R<Record> {
        let v = doc::open(seam::V_RECORD, bytes)?;
        let kind = Kind::need(&doc::need_str(&v, "kind")?)?;
        let id = doc::need_str(&v, "id")?;
        seam::need_name("记录号", &id)?;
        let mut fields = Vec::new();
        for f in doc::need_arr(&v, "fields")? {
            fields.push((doc::need_str(f, "k")?, doc::need_str(f, "v")?));
        }
        let mut parts = Vec::new();
        for p in doc::need_arr(&v, "parts")? {
            let name = doc::need_str(p, "name")?;
            seam::need_name("零件名", &name)?;
            parts.push((name, doc::need_u64(p, "bytes")?));
        }
        Ok(Record {
            id,
            kind,
            created: doc::need_u64(&v, "createdAt")?,
            updated: doc::need_u64(&v, "updatedAt")?,
            writer: doc::need_str(&v, "writer")?,
            fields,
            parts,
        })
    }

    pub fn row(&self) -> Out {
        Out::O(vec![
            ("createdAt", Out::N(self.created)),
            ("id", Out::S(self.id.clone())),
            ("kind", Out::S(self.kind.word().to_string())),
            ("parts", Out::N(self.parts.len() as u64)),
            ("updatedAt", Out::N(self.updated)),
            ("writer", Out::S(self.writer.clone())),
        ])
    }

    pub fn field(&self, k: &str) -> Option<&str> {
        self.fields.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
}

pub fn newest_first_by<T, W, I>(rows: &mut [T], when: W, tie: I)
where
    W: Fn(&T) -> u64,
    I: Fn(&T) -> &[u8],
{
    rows.sort_by(|a, b| when(b).cmp(&when(a)).then(tie(a).cmp(tie(b))));
}


pub fn newest_first(rows: &mut [Record]) {
    newest_first_by(rows, |r| r.updated, |r| r.id.as_bytes());
}

pub fn candidates_newest_first(rows: &mut [(String, String, String, u64)]) {
    newest_first_by(rows, |r| r.3, |r| r.1.as_bytes());
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub kind: String,
    pub address: String,
    pub label: String,
    pub added_at: u64,
    pub seen: String,
}

pub fn candidate_rows_seen(v: &In) -> R<Vec<Candidate>> {
    let four = candidate_rows(v)?;
    let entries = doc::need_arr(v, "entries")?;
    let mut out = Vec::with_capacity(four.len());
    for (e, (kind, address, label, added_at)) in entries.iter().zip(four) {
        let seen = match e.get("seen") {
            None => String::new(),
            Some(In::Str(s)) => s.clone(),
            Some(other) => {
                return Err(crate::fault::Fault::Form(format!("候选表上 {address} 那一行的 seen 要是串,档上是 {other:?}")));
            }
        };
        out.push(Candidate { kind, address, label, added_at, seen });
    }
    Ok(out)
}

pub fn candidate_rows(v: &In) -> R<Vec<(String, String, String, u64)>> {
    let mut out = Vec::new();
    for (i, e) in doc::need_arr(v, "entries")?.iter().enumerate() {
        let row = (
            doc::need_str(e, "kind")?,
            doc::need_str(e, "address")?,
            doc::need_str(e, "label")?,
            doc::need_u64(e, "addedAt")?,
        );
        crate::seam::need_address(&row.1).map_err(|_| {
            crate::fault::Fault::Name(format!(
                "候选表第 {i} 行的 address 是 {:?},它不合地址的形",
                row.1
            ))
        })?;
        out.push(row);
    }
    Ok(out)
}
