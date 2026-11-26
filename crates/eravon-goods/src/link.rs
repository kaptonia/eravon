use eravon_literals::json::Out;

use crate::resolve::Resolved;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Made {
    pub listing: Vec<u8>,
    pub expiry: u64,
    pub terms_hash: [u8; 32],
}

pub type Why = (String, String);

pub const E_GOODS_LINK_UNPOSTED: &str = "E_GOODS_LINK_UNPOSTED";

fn hex(b: &[u8]) -> String {
    eravon_galeed::hash::hex_encode(b)
}

pub fn make(terms_bytes: &[u8], sigs: &[String], resolved: &Resolved) -> Result<Made, Why> {
    let listing = listing(terms_bytes, sigs, &resolved.guilds)?;
    let t = eravon_terms::doc::terms(terms_bytes).map_err(|f| (f.token.name().to_string(), "无法读取条款".to_string()))?;
    Ok(Made { listing, expiry: t.expiry, terms_hash: t.terms_hash })
}

pub fn post_call(terms_hash: &[u8; 32], listing: &[u8]) -> Result<eravon_standard::call::Call, Why> {
    eravon_standard::call::post(eravon_literals::constants::BOARD, terms_hash, listing).map_err(|f| (f.name(), f.detail()))
}

pub fn link_of(chain_id: u64, tx: &str, document: &[u8]) -> Result<String, Why> {
    let doc = eravon_galeed::Value::Obj(vec![
        ("chainId".into(), eravon_galeed::Value::Int(chain_id)),
        ("hash".into(), eravon_galeed::Value::Str(hex(&eravon_galeed::hash::sha256(document)))),
        ("tx".into(), eravon_galeed::Value::Str(tx.to_ascii_lowercase())),
        ("v".into(), eravon_galeed::Value::Str(eravon_literals::literals::DOC_LINK.into())),
    ]);
    let bytes = eravon_galeed::canon::canonical(&doc).map_err(|t| ("E_GOODS_LINK".to_string(), format!("链接文书写不成:{t:?}")))?;
    eravon_link::link::link(&bytes).map_err(|f| (f.token.name().to_string(), "链接文书读不回".to_string()))?;
    Ok(eravon_link::transport::encode_link(&bytes))
}

pub fn listing(terms_bytes: &[u8], sigs: &[String], guilds_now: &[crate::resolve::GuildEl]) -> Result<Vec<u8>, Why> {
    let t = eravon_terms::doc::terms(terms_bytes).map_err(|f| (f.token.name().to_string(), "无法读取条款".to_string()))?;
    let order = Out::O(vec![
        ("amount", Out::S(t.amount.to_dec())),
        ("buyerMayOpen", Out::B(t.buyer_may_open)),
        ("confirmWindow", Out::N(t.confirm_window)),
        ("deliveryWindow", Out::N(t.delivery_window)),
        ("expiry", Out::N(t.expiry)),
        (
            "ladder",
            Out::A(
                t.ladder
                    .iter()
                    .map(|r| Out::O(vec![("buyerBps", Out::N(r.buyer_bps)), ("feeBearer", Out::N(r.fee_bearer)), ("sellerBps", Out::N(r.seller_bps))]))
                    .collect(),
            ),
        ),
        ("nonce", Out::N(t.nonce)),
        ("openWindow", Out::N(t.open_window)),
        ("sellerFund", Out::S(t.seller_fund.clone())),
        ("sellerKey", Out::S(t.seller_key.clone())),
        ("sellerPayout", Out::S(t.seller_payout.clone())),
    ]);
    let terms_judges = block_list(&t.block, "judges");
    let terms_guilds = block_list(&t.block, "guilds");
    if sigs.len() != terms_judges.len() + terms_guilds.len() {
        return Err(("E_GOODS_SIGS".to_string(), "签名数与候选数不一致".to_string()));
    }
    let judges = terms_judges
        .iter()
        .zip(sigs)
        .map(|(j, sig)| {
            let mut m = vec![
                ("amanahHash", Out::S(member(j, "amanahHash"))),
                ("judge", Out::S(member(j, "judge"))),
                ("quoteHash", Out::S(member(j, "quoteHash"))),
                ("sig", Out::S(sig.clone())),
            ];
            if j.get("source").is_some() {
                m.push(("source", Out::S(member(j, "source"))));
            }
            Out::O(m)
        })
        .collect();
    let mut guilds = Vec::new();
    for (g, sig) in terms_guilds.iter().zip(&sigs[terms_judges.len()..]) {
        let pointer = member(g, "pointer");
        let now = guilds_now.iter().find(|x| x.pointer == pointer).ok_or_else(|| {
            ("E_GOODS_GUILD_STATE".to_string(), format!("无法获取行会 {pointer} 的名单"))
        })?;
        if now.amanah_hash != member(g, "amanahHash") {
            return Err(("E_GOODS_GUILD_STATE".to_string(), format!("行会 {pointer} 的规则已更新,需重新签名")));
        }
        guilds.push(Out::O(vec![
            ("amanahHash", Out::S(member(g, "amanahHash"))),
            ("panelHash", Out::S(now.panel_hash.clone())),
            ("pointer", Out::S(pointer.clone())),
            ("quoteHash", Out::S(member(g, "quoteHash"))),
            ("seq", Out::N(now.seq)),
            ("sig", Out::S(sig.clone())),
        ]));
    }
    let parts = eravon_link::build::ListingParts {
        chain_id: t.chain_id,
        token: t.token.clone(),
        logic: t.logic.clone(),
        proxy_code_hash: hex(&eravon_terms::derive::proxy_code_hash(&t.logic)),
        terms_text: String::from_utf8_lossy(terms_bytes).into_owned(),
        order,
        judges,
        guilds,
    };
    eravon_link::build::listing(&parts).map_err(|f| (f.token.name().to_string(), "清单格式无效".to_string()))
}

fn block_list<'a>(block: &'a eravon_galeed::Value, k: &str) -> &'a [eravon_galeed::Value] {
    match block.get(k) {
        Some(eravon_galeed::Value::Arr(a)) => a.as_slice(),
        _ => &[],
    }
}

fn member(el: &eravon_galeed::Value, k: &str) -> String {
    el.get(k).and_then(eravon_galeed::Value::as_str).unwrap_or_default().to_string()
}

pub fn qr_fits(link: &str) -> bool {
    link.len() <= eravon_literals::constants::QR_BYTE_BUDGET
}

pub const E_GOODS_QR_OVER: &str = "E_GOODS_QR_OVER";
pub const E_GOODS_QR_SELFCHECK: &str = "E_GOODS_QR_SELFCHECK";

pub fn qr_made(link: &str) -> Result<(usize, Vec<bool>), Why> {
    let over = || (E_GOODS_QR_OVER.to_string(), format!("链接过长({} 字节),超过二维码容量 {} 字节", link.len(), eravon_literals::constants::QR_BYTE_BUDGET));
    if !qr_fits(link) {
        return Err(over());
    }
    match eravon_qr::make(link.as_bytes()) {
        Ok(code) => Ok((code.size, code.modules.into_iter().flatten().collect())),
        Err(eravon_qr::NotMade::TooLong) => Err(over()),
        Err(eravon_qr::NotMade::SelfCheck) => Err((E_GOODS_QR_SELFCHECK.to_string(), "画出的二维码读不回这条链接,不交出".to_string())),
    }
}

pub fn qr_modules(link: &str) -> Option<(usize, Vec<bool>)> {
    qr_made(link).ok()
}

pub fn qr_pixels(link: &str) -> Result<(usize, Vec<u8>), Why> {
    let (w, cells) = qr_made(link)?;
    const SCALE: usize = 4;
    const QUIET: usize = 4;
    let side = (w + 2 * QUIET) * SCALE;
    let mut px = vec![255u8; side * side];
    for y in 0..w {
        for x in 0..w {
            if cells[y * w + x] {
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        px[((y + QUIET) * SCALE + dy) * side + (x + QUIET) * SCALE + dx] = 0;
                    }
                }
            }
        }
    }
    Ok((side, px))
}

pub fn qr_png(link: &str, path: &str) -> Result<usize, Why> {
    let (side, px) = qr_pixels(link)?;
    let file = std::fs::File::create(path).map_err(|e| ("E_GOODS_IO".to_string(), format!("写不进 {path}:{e}")))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), side as u32, side as u32);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| ("E_GOODS_IO".to_string(), format!("PNG 头写不进:{e}")))?;
    wr.write_image_data(&px).map_err(|e| ("E_GOODS_IO".to_string(), format!("PNG 写不进:{e}")))?;
    Ok(side)
}

#[cfg(test)]
mod qr_tests {
    use super::*;

    #[test]
    fn the_byte_budget_is_the_codes_capacity() {
        assert_eq!(eravon_literals::constants::QR_BYTE_BUDGET, eravon_qr::capacity(40));
    }

    #[test]
    fn a_link_gives_the_one_code_and_one_over_is_named() {
        let link = "eravon://open?e=".to_string() + &"a".repeat(64);
        let made = link_of(1, &format!("0x{}", "ab".repeat(32)), b"{}").expect("链接成形");
        assert!(made.starts_with("eravon://open?e="));
        let (n, cells) = qr_made(&link).expect("made");
        let code = eravon_qr::make(link.as_bytes()).expect("made");
        assert_eq!((n, cells), (code.size, code.modules.concat()));
        let over = "a".repeat(eravon_literals::constants::QR_BYTE_BUDGET + 1);
        assert_eq!(qr_made(&over).map_err(|e| e.0), Err(E_GOODS_QR_OVER.to_string()));
    }
}
