use eravon_literals::json::{Out, write as canon};
use eravon_terms as k2;
use eravon_link as k3;

use crate::fault::{Fault, R};
use crate::seam;

fn checked<T>(bytes: Vec<u8>, read: impl FnOnce(&[u8]) -> R<T>) -> R<Vec<u8>> {
    read(&bytes)?;
    Ok(bytes)
}

pub fn land(path: &str, bytes: &[u8]) -> R<()> {
    if holds(path, bytes) {
        return Ok(());
    }
    eravon_store::landing::land_bytes(std::path::Path::new(path), bytes).map_err(|t| match t {
        eravon_store::landing::Trouble::Occupied(_) => occupied(path),
        eravon_store::landing::Trouble::Io(_, said) => Fault::Io(format!("写不进 {path}:{said}")),
    })
}

pub fn free(items: &[(&str, &[u8])]) -> R<()> {
    for (i, (p, bytes)) in items.iter().enumerate() {
        if std::fs::symlink_metadata(p).is_ok() && !holds(p, bytes) {
            return Err(occupied(p));
        }
        if items[..i].iter().any(|(q, b)| q == p && b != bytes) {
            return Err(occupied(p));
        }
    }
    Ok(())
}

fn holds(path: &str, bytes: &[u8]) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_file() && m.len() == bytes.len() as u64 => std::fs::read(path).map(|b| b == bytes).unwrap_or(false),
        _ => false,
    }
}

fn occupied(path: &str) -> Fault {
    Fault::Exists(format!("{path} 已经有了,一个字节也没盖"))
}

pub fn terms_document(block: &[u8]) -> R<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("```{}\n", seam::V_TERMS).as_bytes());
    out.extend_from_slice(block);
    out.extend_from_slice(b"\n```\n");
    checked(out, |b| k2::doc::terms(b).map_err(Fault::from))
}

pub fn receipt(
    terms_hash: &str,
    candidate: u64,
    buyer_key: &str,
    buyer_payout: &str,
    open_by: u64,
    prev: &str,
    sig: &str,
    seq: Option<u64>,
) -> R<Vec<u8>> {
    let mut m: Vec<(&'static str, Out)> = vec![
        ("buyerKey", Out::S(buyer_key.to_string())),
        ("buyerPayout", Out::S(buyer_payout.to_string())),
        ("candidate", Out::N(candidate)),
        ("openBy", Out::N(open_by)),
        ("prev", Out::S(prev.to_string())),
        ("sig", Out::S(sig.to_string())),
        ("termsHash", Out::S(terms_hash.to_string())),
        ("v", Out::S(seam::V_RECEIPT.to_string())),
    ];
    if let Some(s) = seq {
        m.push(("seq", Out::N(s)));
    }
    checked(canon(&Out::O(m)), |b| k3::receipt::receipt(b).map_err(Fault::from))
}

pub fn pack(
    chain_id: u64,
    instance: &str,
    case_id: &str,
    link_bytes: &[u8],
    listing_bytes: &[u8],
    receipt_bytes: &[u8],
    attachments: &[String],
) -> R<Vec<u8>> {
    let mut body = Vec::new();
    body.push(b'{');
    if !attachments.is_empty() {
        let list = Out::A(
            attachments
                .iter()
                .map(|t| {
                    let h = eravon_galeed::hash::hex_encode(&eravon_galeed::hash::sha256(t.as_bytes()));
                    Out::O(vec![("sha256", Out::S(h)), ("text_md", Out::S(t.clone()))])
                })
                .collect(),
        );
        body.extend_from_slice(b"\"attachments\":");
        body.extend_from_slice(&canon(&list));
        body.push(b',');
    }
    body.extend_from_slice(b"\"caseId\":");
    put_str(case_id, &mut body);
    body.extend_from_slice(format!(",\"chainId\":{chain_id},\"instance\":").as_bytes());
    put_str(instance, &mut body);
    body.extend_from_slice(b",\"link\":");
    body.extend_from_slice(link_bytes);
    body.extend_from_slice(b",\"listing\":");
    body.extend_from_slice(listing_bytes);
    body.extend_from_slice(b",\"receipt\":");
    body.extend_from_slice(receipt_bytes);
    body.extend_from_slice(b",\"v\":");
    put_str(seam::V_PACK, &mut body);
    body.push(b'}');
    checked(body, |b| k3::pack::pack(b).map_err(Fault::from))
}

fn put_str(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut b = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
            }
        }
    }
    out.push(b'"');
}
