use eravon_store::record::Record;
use eravon_store::seam::Box_;
use eravon_watch::order;

use crate::seam;
use crate::{Why, why};

#[derive(Clone, Debug)]
pub struct Docs {
    pub posted: Option<(String, String)>,
    pub listing: Vec<u8>,
    pub receipt: Option<Vec<u8>>,
    pub at: Option<(String, String)>,
    pub attachments: Vec<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub enum Got {
    Docs(Docs),
    Missing(Vec<&'static str>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Seller,
    Buyer,
}

const ALL_MISSING: [&str; 3] = [seam::MISSING_POSTED, seam::MISSING_LISTING, seam::MISSING_RECEIPT];

fn store_why(e: eravon_store::fault::Fault) -> Why {
    why(e.name(), e.detail())
}

pub fn docs(data: &str, owners: &[String], side: Side, record: &str) -> Result<Got, Why> {
    if record.is_empty() {
        return Ok(Got::Missing(ALL_MISSING.to_vec()));
    }
    let s = eravon_vault::store::open_read(data).map_err(store_why)?;
    let b = match side {
        Side::Seller => Box_::Orders,
        Side::Buyer => Box_::Packs,
    };
    for o in owners {
        let r: Record = match s.get(o, b, record) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(store_why(e)),
        };
        let has = |n: &str| r.parts.iter().any(|(x, _)| x == n);
        return Ok(match side {
            Side::Seller => {
                if !has(&order::part_listing()) {
                    return Ok(Got::Missing(missing_of(&[true, false, has(&order::part_receipt())])));
                }
                let listing = s.part(o, b, record, &order::part_listing()).map_err(store_why)?;
                let receipt = if has(&order::part_receipt()) { Some(s.part(o, b, record, &order::part_receipt()).map_err(store_why)?) } else { None };
                let at = match (r.field("instance"), r.field("caseId")) {
                    (Some(i), Some(c)) if !i.is_empty() && !c.is_empty() => Some((i.to_ascii_lowercase(), c.to_ascii_lowercase())),
                    _ => None,
                };
                Got::Docs(Docs { posted: None, attachments: seller_texts(&s, owners, &listing)?, listing, receipt, at })
            }
            Side::Buyer => {
                if !has(PACK) {
                    Got::Missing(ALL_MISSING.to_vec())
                } else {
                    from_pack(&s.part(o, b, record, PACK).map_err(store_why)?)?
                }
            }
        });
    }
    Ok(Got::Missing(ALL_MISSING.to_vec()))
}

fn seller_texts(s: &eravon_vault::store::Store, owners: &[String], listing: &[u8]) -> Result<Vec<Vec<u8>>, Why> {
    let Ok(l) = eravon_link::listing::listing(listing) else { return Ok(Vec::new()) };
    let mut found = None;
    for o in owners {
        let rows = match s.list(o, Box_::Products) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(store_why(e)),
        };
        if let Some(r) = rows.into_iter().find(|r| r.kind == eravon_store::record::Kind::TermsDocument && r.field("termsHash").is_some_and(|h| h.eq_ignore_ascii_case(&l.terms_hash))) {
            found = Some((o, r));
            break;
        }
    }
    let Some((me, r)) = found else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (key, part) in [("goodsHash", eravon_goods::goods::PART_GOODS_TEXT), ("deliveryHash", eravon_goods::goods::PART_DELIVERY_TEXT)] {
        let Some(want) = l.terms.block.get(key).and_then(eravon_galeed::Value::as_str) else { continue };
        if !r.parts.iter().any(|(n, _)| n == part) {
            continue;
        }
        let text = s.part(me, Box_::Products, &r.id, part).map_err(store_why)?;
        if eravon_goods::terms::segment_hash(&String::from_utf8_lossy(&text)).eq_ignore_ascii_case(want) {
            out.push(text);
        }
    }
    Ok(out)
}

const PACK: &str = "pack.json";

fn from_pack(b: &[u8]) -> Result<Got, Why> {
    let p = eravon_link::pack::pack(b).map_err(|f| why("E_S7_PACK", format!("订单文件判不过:{:?}", f.to_value(true))))?;
    Ok(Got::Docs(Docs {
        posted: Some((p.link.tx.clone(), p.link.hash.clone())),
        listing: p.listing.bytes.clone(),
        receipt: Some(p.receipt.bytes.clone()),
        at: None,
        attachments: p.attachments.iter().map(|a| a.text_md.as_bytes().to_vec()).collect(),
    }))
}

fn missing_of(has: &[bool; 3]) -> Vec<&'static str> {
    ALL_MISSING.iter().zip(has).filter(|(_, h)| !**h).map(|(n, _)| *n).collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_names_follow_the_index_order() {
        assert_eq!(missing_of(&[true, false, false]), vec![seam::MISSING_LISTING, seam::MISSING_RECEIPT]);
        assert!(missing_of(&[true, true, true]).is_empty());
    }
}
