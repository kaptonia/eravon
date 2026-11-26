use eravon_galeed::Value;

use crate::resolve::{GuildEl, JudgeEl};

pub struct Parts<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub delivery_note: &'a str,
    pub goods_private: bool,
    pub delivery_private: bool,
    pub seller_key: &'a str,
    pub seller_payout: &'a str,
    pub seller_fund: &'a str,
    pub token: &'a str,
    pub amount: &'a str,
    pub t1_secs: u64,
    pub t2_secs: u64,
    pub open_window: u64,
    pub buyer_may_open: bool,
    pub ladder: &'a [(String, u64)],
    pub judges: &'a [JudgeEl],
    pub guilds: &'a [GuildEl],
    pub nonce: u64,
    pub expiry: u64,
    pub series: Option<(&'a str, u64, u64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    pub bytes: Vec<u8>,
    pub goods_text: Option<String>,
    pub delivery_text: Option<String>,
}

pub fn say_window(secs: u64) -> String {
    let (d, h, m) = crate::goods::span(secs);
    let mut out: Vec<String> = Vec::new();
    for (n, unit) in [(d, "天"), (h, "小时"), (m, "分钟")] {
        if n > 0 {
            out.push(format!("{n} {unit}"));
        }
    }
    out.join(" ")
}

pub fn goods_text(name: &str, description: &str) -> String {
    if description.is_empty() { name.to_string() } else { format!("{name}\n\n{description}") }
}

pub fn delivery_text(t1: u64, t2: u64, note: &str) -> String {
    let line = format!("{}内交付 · 交付后 {}内确认", say_window(t1), say_window(t2));
    if note.is_empty() { line } else { format!("{line}\n\n{note}") }
}

pub fn name_of(goods_text: &str) -> String {
    goods_text.lines().next().unwrap_or_default().trim().to_string()
}

pub fn segment_hash(text: &str) -> String {
    eravon_galeed::hash::hex_encode(&eravon_galeed::hash::sha256(text.as_bytes()))
}

pub fn quote_hash(quote: &str) -> String {
    let bytes = quote.strip_prefix("0x").and_then(|d| eravon_galeed::hexform::decode_digits(d.as_bytes())).unwrap_or_default();
    eravon_galeed::hash::hex_encode(&eravon_galeed::hash::keccak256(&bytes))
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn obj(mut m: Vec<(&str, Value)>) -> Value {
    m.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Value::Obj(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn block(p: &Parts<'_>) -> (Value, Option<String>, Option<String>) {
    let std = eravon_literals::constants::active_standard();
    let ladder = p
        .ladder
        .iter()
        .enumerate()
        .map(|(i, (name, bps))| {
            let mut m = vec![
                ("buyerBps", Value::Int(*bps)),
                ("feeBearer", Value::Int(u64::from(eravon_literals::constants::fee_bearer_of_rung(i)))),
                ("sellerBps", Value::Int(10_000 - bps)),
            ];
            if !name.is_empty() {
                m.push(("note_md", s(name)));
            }
            obj(m)
        })
        .collect();
    let judges = p
        .judges
        .iter()
        .map(|j| {
            let mut m = vec![("amanahHash", s(&j.amanah_hash)), ("judge", s(&j.judge)), ("quoteHash", s(&quote_hash(&j.quote)))];
            if let Some(src) = &j.source {
                m.push(("source", s(src)));
            }
            obj(m)
        })
        .collect();
    let guilds = p
        .guilds
        .iter()
        .map(|g| obj(vec![("amanahHash", s(&g.amanah_hash)), ("pointer", s(&g.pointer)), ("quoteHash", s(&quote_hash(&g.quote)))]))
        .collect();
    let goods = goods_text(p.name, p.description);
    let delivery = delivery_text(p.t1_secs, p.t2_secs, p.delivery_note);
    let mut m = vec![
        ("amount", s(p.amount)),
        ("buyerMayOpen", Value::Bool(p.buyer_may_open)),
        ("chainId", Value::Int(std.chain_id)),
        ("confirmWindow", Value::Int(p.t2_secs)),
        ("deliveryWindow", Value::Int(p.t1_secs)),
        ("expiry", Value::Int(p.expiry)),
        ("guilds", Value::Arr(guilds)),
        ("judges", Value::Arr(judges)),
        ("ladder", Value::Arr(ladder)),
        ("logic", s(&std.logic.to_ascii_lowercase())),
        ("nonce", Value::Int(p.nonce)),
        ("openWindow", Value::Int(p.open_window)),
        ("sellerFund", s(p.seller_fund)),
        ("sellerKey", s(p.seller_key)),
        ("sellerPayout", s(p.seller_payout)),
        ("token", s(p.token)),
        ("v", s(eravon_literals::literals::DOC_TERMS)),
    ];
    if let Some((id, stage, stages)) = p.series {
        m.push(("series", obj(vec![("id", s(&id.to_ascii_lowercase())), ("stage", Value::Int(stage)), ("stages", Value::Int(stages))])));
    }
    let (goods_kept, delivery_kept) = (p.goods_private.then(|| goods.clone()), p.delivery_private.then(|| delivery.clone()));
    if p.goods_private {
        m.push(("goodsHash", s(&segment_hash(&goods))));
    } else {
        m.push(("goods_md", s(&goods)));
    }
    if p.delivery_private {
        m.push(("deliveryHash", s(&segment_hash(&delivery))));
    } else {
        m.push(("delivery_md", s(&delivery)));
    }
    (obj(m), goods_kept, delivery_kept)
}

pub fn generate(p: &Parts<'_>) -> Generated {
    let (v, goods_text, delivery_text) = block(p);
    let mut out = String::new();
    out.push_str(eravon_literals::literals::TERMS_FENCE_OPEN);
    out.push('\n');
    out.push_str(&String::from_utf8_lossy(&eravon_galeed::canon::canonical(&v).unwrap_or_default()));
    out.push('\n');
    out.push_str(eravon_literals::literals::TERMS_FENCE_CLOSE);
    out.push('\n');
    Generated { bytes: out.into_bytes(), goods_text, delivery_text }
}

pub fn goods_of(block: &Value, held: Option<&str>) -> Option<String> {
    segment_of(block, "goods_md", "goodsHash", held)
}

pub fn delivery_of(block: &Value, held: Option<&str>) -> Option<String> {
    segment_of(block, "delivery_md", "deliveryHash", held)
}

fn segment_of(block: &Value, md: &str, hash: &str, held: Option<&str>) -> Option<String> {
    if let Some(t) = block.get(md).and_then(Value::as_str) {
        return Some(t.to_string());
    }
    let want = block.get(hash).and_then(Value::as_str)?;
    held.filter(|t| segment_hash(t).eq_ignore_ascii_case(want)).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts<'a>(ladder: &'a [(String, u64)], judges: &'a [JudgeEl], private: bool) -> Parts<'a> {
        Parts {
            name: "海报",
            description: "一张 A2 海报",
            delivery_note: "PNG 一份",
            goods_private: private,
            delivery_private: private,
            seller_key: "0x00000000000000000000000000000000000000a1",
            seller_payout: "0x00000000000000000000000000000000000000a2",
            seller_fund: "0x00000000000000000000000000000000000000a3",
            token: "0x00000000000000000000000000000000000000a4",
            amount: "1000",
            t1_secs: 86_400,
            t2_secs: 3_600,
            open_window: 7_200,
            buyer_may_open: true,
            ladder,
            judges,
            guilds: &[],
            nonce: 1,
            expiry: 2_000_000_000,
            series: None,
        }
    }

    #[test]
    fn a_stage_writes_its_series() {
        let ladder = vec![("驳回".to_string(), 0), ("全额退款".to_string(), 10_000)];
        let judges = vec![JudgeEl { judge: "0x00000000000000000000000000000000000000b1".into(), amanah_hash: format!("0x{}", "11".repeat(32)), quote: "0x0102".into(), source: None }];
        let id = format!("0x{}", "Ab".repeat(32));
        let mut p = parts(&ladder, &judges, false);
        p.series = Some((&id, 2, 3));
        let t = eravon_terms::doc::terms(&generate(&p).bytes).expect("条款读得回");
        let s = t.series.expect("带 series");
        assert_eq!((s.id.as_str(), s.stage, s.stages), (id.to_ascii_lowercase().as_str(), 2, 3));
    }

    #[test]
    fn a_generated_document_reads_back_in_both_forms() {
        let ladder = vec![("驳回".to_string(), 0), ("全额退款".to_string(), 10_000)];
        let judges = vec![JudgeEl {
            judge: "0x00000000000000000000000000000000000000b1".into(),
            amanah_hash: format!("0x{}", "11".repeat(32)),
            quote: "0x0102".into(),
            source: None,
        }];
        let clear = generate(&parts(&ladder, &judges, false));
        let t = eravon_terms::doc::terms(&clear.bytes).expect("明文形是条款");
        assert_eq!((t.open_window, t.buyer_may_open, t.seller_fund.as_str()), (7_200, true, "0x00000000000000000000000000000000000000a3"));
        assert_eq!(goods_of(&t.block, None).as_deref().map(name_of).as_deref(), Some("海报"));
        assert!(clear.goods_text.is_none() && clear.delivery_text.is_none());
        let hashed = generate(&parts(&ladder, &judges, true));
        let h = eravon_terms::doc::terms(&hashed.bytes).expect("哈希形是条款");
        let kept = hashed.goods_text.expect("货物正文留在卖家");
        assert_eq!(goods_of(&h.block, None), None, "没拿到正文即读不出");
        assert_eq!(goods_of(&h.block, Some("别的字")), None, "哈希不符不收");
        assert_eq!(goods_of(&h.block, Some(&kept)).as_deref(), Some(kept.as_str()));
        assert_eq!(delivery_of(&h.block, hashed.delivery_text.as_deref()), hashed.delivery_text);
        assert_eq!(quote_hash("0x0102"), eravon_galeed::hash::hex_encode(&eravon_galeed::hash::keccak256(&[1, 2])));
    }
}
