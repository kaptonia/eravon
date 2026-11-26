use eravon_literals::json::Out;

use crate::draft::Draft;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Miss {
    pub cell: String,
    pub name: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Verdict {
    pub missing: Vec<Miss>,
}

impl Verdict {
    pub fn formal(&self) -> bool {
        self.missing.is_empty()
    }
    pub fn miss(&mut self, cell: impl Into<String>, name: &str, detail: impl Into<String>) {
        self.missing.push(Miss { cell: cell.into(), name: name.to_string(), detail: detail.into() });
    }
    pub fn rows(&self) -> Out {
        Out::A(
            self.missing
                .iter()
                .map(|m| {
                    Out::O(vec![
                        ("cell", Out::S(m.cell.clone())),
                        ("detail", Out::S(m.detail.clone())),
                        ("name", Out::S(m.name.clone())),
                    ])
                })
                .collect(),
        )
    }
}

pub fn free_text(d: &Draft) -> Vec<(String, &str)> {
    let mut v: Vec<(String, &str)> = vec![
        ("name".into(), d.name.as_str()),
        ("description".into(), d.description.as_str()),
        ("deliveryNote".into(), d.delivery_note.as_str()),
    ];
    for (i, t) in d.ladder.iter().enumerate() {
        v.push((format!("ladder.{i}"), t.name.as_str()));
    }
    v
}

pub fn presence(d: &Draft, v: &mut Verdict) {
    if d.name.trim().is_empty() {
        v.miss("name", "E_GOODS_EMPTY", "未填写");
    }
    for (cell, val) in [("price", &d.price), ("token", &d.token)] {
        if val.trim().is_empty() {
            v.miss(cell, "E_GOODS_EMPTY", "未填写");
        }
    }
    if is_zero(&d.price) {
        v.miss("price", "E_TERMS_AMOUNT", "价格不能为零");
    }
    if d.cands.is_empty() {
        v.miss("cands", "E_GOODS_EMPTY", "至少选择一位判官或一个行会");
    }
    if d.name.contains('\n') {
        v.miss("name", "E_GOODS_FORM", "名称只能一行");
    }
    crlf(d, v);
}

fn is_zero(price: &str) -> bool {
    let p = price.trim();
    p.bytes().filter(|b| *b == b'.').count() <= 1
        && p.bytes().any(|b| b.is_ascii_digit())
        && p.bytes().all(|b| b == b'0' || b == b'.')
}

fn crlf(d: &Draft, v: &mut Verdict) {
    for (cell, text) in free_text(d) {
        if text.contains('\r') {
            v.miss(cell, "E_GOODS_CR", "包含不支持的回车符");
        }
    }
}


pub fn cell_of_law(token: &str, d: &Draft) -> String {
    let _ = d;
    match token {
        "E_TERMS_AMOUNT" => "price".into(),
        "E_TERMS_DELIVERY_WINDOW" => "t1".into(),
        "E_TERMS_CONFIRM_WINDOW" => "t2".into(),
        "E_TERMS_GOODS" => "description".into(),
        "E_TERMS_DELIVERY" => "deliveryNote".into(),
        "E_TERMS_TOKEN" => "token".into(),
        "E_TERMS_PAYOUT" | "E_TERMS_SELLER" => "payout".into(),
        "E_TERMS_FUND" => "fund".into(),
        "E_TERMS_OPENWINDOW" => "openWindow".into(),
        t if t == "E_TERMS_LADDER" || t.starts_with("E_LADDER_") || t.starts_with("E_RUNG") => "ladder".into(),
        t if t == "E_TERMS_DUP_JUDGE" || t == "E_TERMS_NO_CANDIDATE" || t.starts_with("E_JUDGE") || t.starts_with("E_GUILD") => {
            "cands".into()
        }
        _ => "terms".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price_marks(price: &str) -> Vec<String> {
        let d = Draft { price: price.to_string(), ..Draft::default() };
        let mut v = Verdict::default();
        presence(&d, &mut v);
        v.missing.iter().filter(|m| m.cell == "price").map(|m| m.name.clone()).collect()
    }

    #[test]
    fn a_zero_price_is_marked_on_its_cell() {
        for z in ["0", "0.0", "00.000", ".0", "0.", " 0 "] {
            assert_eq!(price_marks(z), vec!["E_TERMS_AMOUNT".to_string()], "{z:?} is zero");
        }
        assert_eq!(price_marks(""), vec!["E_GOODS_EMPTY".to_string()], "empty is empty, not zero");
        for n in ["0.000001", "1", "10", "0.10"] {
            assert!(price_marks(n).is_empty(), "{n:?} is not zero");
        }
        for odd in [".", "0.0.0", "abc", "-0"] {
            assert!(!price_marks(odd).contains(&"E_TERMS_AMOUNT".to_string()), "{odd:?} is not judged zero here");
        }
    }
}
