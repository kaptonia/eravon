use eravon_literals::json::{In, Out};

pub const V_DRAFT: &str = "eravon.goods.draft/1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tier {
    pub name: String,
    pub pct: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub name: String,
    pub description: String,
    pub price: String,
    pub token: String,
    pub t1: String,
    pub t1h: String,
    pub t1m: String,
    pub t2: String,
    pub t2h: String,
    pub t2m: String,
    pub delivery_note: String,
    pub ladder: Vec<Tier>,
    pub goods_private: bool,
    pub delivery_private: bool,
    pub cands: Vec<String>,
    pub series_id: String,
    pub stage: String,
    pub stages: String,
}

impl Default for Draft {
    fn default() -> Draft {
        let t = eravon_literals::constants::REFUND_LADDER_TEMPLATE;
        let ladder = t
            .iter()
            .enumerate()
            .map(|(i, r)| Tier {
                name: if i == 0 { "驳回".into() } else if i + 1 == t.len() { "全额退款".into() } else { String::new() },
                pct: pct_of(r.buyer_bps),
            })
            .collect();
        Draft {
            name: String::new(),
            description: String::new(),
            price: String::new(),
            token: String::new(),
            t1: String::new(),
            t1h: String::new(),
            t1m: String::new(),
            t2: String::new(),
            t2h: String::new(),
            t2m: String::new(),
            delivery_note: String::new(),
            ladder,
            goods_private: false,
            delivery_private: false,
            cands: Vec::new(),
            series_id: String::new(),
            stage: String::new(),
            stages: String::new(),
        }
    }
}

pub fn pct_of(bps: u16) -> String {
    let (i, f) = (bps / 100, bps % 100);
    if f == 0 { i.to_string() } else if f % 10 == 0 { format!("{i}.{}", f / 10) } else { format!("{i}.{f:02}") }
}

pub const PAGE: &str = "goodsPage";

pub const FIELDS: [&str; 16] = [
    "name", "description", "price", "token", "t1", "t1h", "t1m", "t2", "t2h", "t2m", "deliveryNote", "goodsPrivate", "deliveryPrivate", "seriesId", "stage",
    "stages",
];

impl Draft {
    pub fn set(&mut self, k: &str, v: &str) -> bool {
        if let Some(b) = match k {
            "goodsPrivate" => Some(&mut self.goods_private),
            "deliveryPrivate" => Some(&mut self.delivery_private),
            _ => None,
        } {
            match v {
                "true" => *b = true,
                "false" => *b = false,
                _ => return false,
            }
            return true;
        }
        if k == PAGE {
            return self.set_page(v);
        }
        let slot = match k {
            "name" => &mut self.name,
            "description" => &mut self.description,
            "price" => &mut self.price,
            "token" => &mut self.token,
            "t1" => &mut self.t1,
            "t1h" => &mut self.t1h,
            "t1m" => &mut self.t1m,
            "t2" => &mut self.t2,
            "t2h" => &mut self.t2h,
            "t2m" => &mut self.t2m,
            "deliveryNote" => &mut self.delivery_note,
            "seriesId" => &mut self.series_id,
            "stage" => &mut self.stage,
            "stages" => &mut self.stages,
            _ => return false,
        };
        *slot = v.to_string();
        true
    }

    pub fn get(&self, k: &str) -> Option<&str> {
        let word = |b: bool| if b { "true" } else { "false" };
        Some(match k {
            "name" => &self.name,
            "description" => &self.description,
            "price" => &self.price,
            "token" => &self.token,
            "t1" => &self.t1,
            "t1h" => &self.t1h,
            "t1m" => &self.t1m,
            "t2" => &self.t2,
            "t2h" => &self.t2h,
            "t2m" => &self.t2m,
            "deliveryNote" => &self.delivery_note,
            "seriesId" => &self.series_id,
            "stage" => &self.stage,
            "stages" => &self.stages,
            "goodsPrivate" => word(self.goods_private),
            "deliveryPrivate" => word(self.delivery_private),
            _ => return None,
        })
    }

    pub fn refs(&self) -> crate::refs::Read {
        crate::refs::read(&self.description)
    }

    pub fn page(&self) -> String {
        self.refs().refs.page.unwrap_or_default()
    }

    pub fn body(&self) -> String {
        self.refs().body
    }

    pub fn set_body(&mut self, body: &str) {
        let refs = self.refs().refs;
        self.description = crate::refs::compose(body, &refs);
    }

    pub fn set_page(&mut self, entered: &str) -> bool {
        let mut refs = self.refs().refs;
        let Ok(page) = crate::refs::page_of(entered) else { return false };
        refs.page = page;
        self.description = crate::refs::compose(&self.body(), &refs);
        true
    }

    pub fn add_image(&mut self, file_name: &str, bytes: &[u8]) -> bool {
        let mut refs = self.refs().refs;
        let Some(name) = crate::refs::name_of(file_name) else { return false };
        refs.images.push(crate::refs::Image { sha256: crate::refs::digest(bytes), name });
        self.description = crate::refs::compose(&self.body(), &refs);
        true
    }

    pub fn remove_image(&mut self, i: usize) -> bool {
        let mut refs = self.refs().refs;
        if i >= refs.images.len() {
            return false;
        }
        refs.images.remove(i);
        self.description = crate::refs::compose(&self.body(), &refs);
        true
    }

    pub fn toggle(&mut self, addr: &str) {
        let a = addr.to_ascii_lowercase();
        if let Some(i) = self.cands.iter().position(|x| *x == a) {
            self.cands.remove(i);
        } else {
            self.cands.push(a);
        }
    }

    pub fn add_tier(&mut self) {
        let at = self.ladder.len().saturating_sub(1);
        self.ladder.insert(at, Tier { name: String::new(), pct: String::new() });
    }

    pub fn del_tier(&mut self, i: usize) -> bool {
        if i == 0 || i + 1 >= self.ladder.len() {
            return false;
        }
        self.ladder.remove(i);
        true
    }

    pub fn set_tier(&mut self, i: usize, name: Option<&str>, pct: Option<&str>) -> bool {
        let last = self.ladder.len().saturating_sub(1);
        let Some(t) = self.ladder.get_mut(i) else { return false };
        if let Some(n) = name {
            t.name = n.to_string();
        }
        if let Some(p) = pct {
            if i == 0 || i == last {
                return false;
            }
            t.pct = p.to_string();
        }
        true
    }

    pub fn seal(&self) -> Vec<u8> {
        let s = |x: &str| Out::S(x.to_string());
        eravon_literals::json::write(&Out::O(vec![
            ("cands", Out::A(self.cands.iter().map(|c| s(c)).collect())),
            ("deliveryNote", s(&self.delivery_note)),
            ("deliveryPrivate", Out::B(self.delivery_private)),
            ("description", s(&self.description)),
            ("goodsPrivate", Out::B(self.goods_private)),
            (
                "ladder",
                Out::A(self.ladder.iter().map(|t| Out::O(vec![("name", s(&t.name)), ("pct", s(&t.pct))])).collect()),
            ),
            ("name", s(&self.name)),
            ("price", s(&self.price)),
            ("seriesId", s(&self.series_id)),
            ("stage", s(&self.stage)),
            ("stages", s(&self.stages)),
            ("t1", s(&self.t1)),
            ("t1h", s(&self.t1h)),
            ("t1m", s(&self.t1m)),
            ("t2", s(&self.t2)),
            ("t2h", s(&self.t2h)),
            ("t2m", s(&self.t2m)),
            ("token", s(&self.token)),
            ("v", s(V_DRAFT)),
        ]))
    }

    pub fn open(b: &[u8]) -> Option<Draft> {
        let v = eravon_literals::json::read(b).ok()?;
        if v.get("v").and_then(|x| x.as_str()) != Some(V_DRAFT) {
            return None;
        }
        let st = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let more = |k: &str| st(k).unwrap_or_default();
        let switch = |k: &str| matches!(v.get(k), Some(In::Bool(true)));
        let ladder = match v.get("ladder") {
            Some(In::Arr(a)) => a
                .iter()
                .map(|t| Some(Tier { name: t.get("name")?.as_str()?.to_string(), pct: t.get("pct")?.as_str()?.to_string() }))
                .collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
        let cands = match v.get("cands") {
            Some(In::Arr(a)) => a.iter().map(|x| x.as_str().map(str::to_string)).collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
        Some(Draft {
            name: st("name")?,
            description: st("description")?,
            price: st("price")?,
            token: fold_token(&st("token")?),
            t1: st("t1")?,
            t1h: more("t1h"),
            t1m: more("t1m"),
            t2: st("t2")?,
            t2h: more("t2h"),
            t2m: more("t2m"),
            delivery_note: st("deliveryNote")?,
            ladder,
            goods_private: switch("goodsPrivate"),
            delivery_private: switch("deliveryPrivate"),
            cands,
            series_id: more("seriesId"),
            stage: more("stage"),
            stages: more("stages"),
        })
    }
}

pub fn token_address(cell: &str) -> Option<String> {
    let t = cell.trim();
    if t.len() == 42 && t.starts_with("0x") && t[2..].bytes().all(|c| c.is_ascii_hexdigit()) {
        return Some(t.to_ascii_lowercase());
    }
    symbol(t)
}

fn symbol(t: &str) -> Option<String> {
    eravon_vault::tokens::on_chain(&eravon_vault::tokens::factory(), eravon_vault::tokens::chain()).into_iter().find(|r| r.label == t).map(|r| r.address)
}


pub fn fold_token(cell: &str) -> String {
    if cell.trim().is_empty() { String::new() } else { token_address(cell).unwrap_or_else(|| cell.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_round_trips_and_the_ends_of_the_ladder_are_locked() {
        let mut d = Draft::default();
        assert_eq!(d.ladder.len(), 2);
        assert_eq!((d.ladder[0].pct.as_str(), d.ladder[1].pct.as_str()), ("0", "100"));
        d.add_tier();
        assert!(d.set_tier(1, Some("退一半"), Some("50")));
        assert!(!d.set_tier(0, None, Some("10")), "首档比例锁死");
        assert!(!d.del_tier(0) && !d.del_tier(2), "首末两档不可删");
        d.set("name", "海报");
        assert!(d.set("goodsPrivate", "true") && !d.set("deliveryPrivate", "yes"), "开关只收 true 与 false");
        d.toggle("0xAB");
        assert_eq!(Draft::open(&d.seal()), Some(d));
        assert_eq!(pct_of(1250), "12.5");
        assert_eq!(pct_of(1), "0.01");
    }
}
