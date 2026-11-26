use eravon_store::record::Kind;
use eravon_store::seam::Box_;

use crate::draft::Draft;

pub const E_SERIES: &str = "E_GOODS_SERIES";
pub const E_SERIES_ONE: &str = "E_GOODS_SERIES_ONE";
pub const E_SERIES_JUDGE: &str = "E_GOODS_SERIES_JUDGE";
pub const E_SERIES_PREV: &str = "E_GOODS_SERIES_PREV";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Series {
    pub id: String,
    pub stage: u64,
    pub stages: u64,
}

pub fn cells(d: &Draft) -> Result<Option<Series>, (&'static str, &'static str)> {
    let (id, stage, stages) = (d.series_id.trim(), d.stage.trim(), d.stages.trim());
    if id.is_empty() && stage.is_empty() && stages.is_empty() {
        return Ok(None);
    }
    let hex = id.len() == 66 && id.starts_with("0x") && id[2..].bytes().all(|c| c.is_ascii_hexdigit());
    if !hex || id[2..].bytes().all(|c| c == b'0') {
        return Err(("seriesId", "系列号要是 0x 加六十四位十六进制,不能全零"));
    }
    let (Ok(stage), Ok(stages)) = (stage.parse::<u64>(), stages.parse::<u64>()) else {
        return Err(("stage", "段号与段数要是整数"));
    };
    if stage == 0 || stage > stages {
        return Err(("stage", "段号从 1 起,不超过段数"));
    }
    Ok(Some(Series { id: id.to_ascii_lowercase(), stage, stages }))
}

pub fn fresh() -> Result<String, String> {
    loop {
        let b = eravon_vault::key::random(32).map_err(|e| e.raw())?;
        if b.iter().any(|x| *x != 0) {
            return Ok(format!("0x{}", b.iter().map(|x| format!("{x:02x}")).collect::<String>()));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Known {
    pub id: String,
    pub stages: u64,
    pub written: Vec<u64>,
    pub name: String,
}

pub fn known(written: &[Written], except: Option<&str>) -> Vec<Known> {
    let mut out: Vec<(Known, u64)> = Vec::new();
    for w in written.iter().filter(|w| Some(w.id.as_str()) != except) {
        let s = &w.series;
        match out.iter_mut().find(|(k, _)| k.id == s.id) {
            Some((k, low)) => {
                k.written.push(s.stage);
                if s.stage < *low {
                    *low = s.stage;
                    k.name = w.name.clone();
                }
            }
            None => out.push((Known { id: s.id.clone(), stages: s.stages, written: vec![s.stage], name: w.name.clone() }, s.stage)),
        }
    }
    out.into_iter().map(|(mut k, _)| {
        k.written.sort_unstable();
        k
    }).collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    pub id: String,
    pub series: Series,
    pub name: String,
}

pub fn stages(data: &str, me: &str) -> Vec<Written> {
    signed(data, me).into_iter().map(|(m, name)| Written { id: m.id, series: m.series, name }).collect()
}

pub fn next_stage(k: &Known) -> Option<u64> {
    (1..=k.stages).find(|n| !k.written.contains(n))
}

pub const E_SERIES_UNKNOWN: &str = "E_GOODS_SERIES_UNKNOWN";

pub const PICK_NONE: &str = "none";
pub const PICK_NEW: &str = "new";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    None,
    New(String),
    Known(Known),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotPicked {
    Full(Known),
    Unknown(String),
    Random(String),
}

impl NotPicked {
    pub fn name(&self) -> &'static str {
        match self {
            NotPicked::Full(_) => E_SERIES_JUDGE,
            NotPicked::Unknown(_) => E_SERIES_UNKNOWN,
            NotPicked::Random(_) => E_SERIES,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            NotPicked::Full(k) => format!("系列 {} 的 {} 段都已写好,没有下一段可写", k.id, k.stages),
            NotPicked::Unknown(id) => format!("本席没有这个系列:{id}(新系列选 {PICK_NEW},不分段选 {PICK_NONE})"),
            NotPicked::Random(e) => format!("造不出系列号:{e}"),
        }
    }
}

pub fn pick(d: &mut Draft, v: &str, known: &[Known]) -> Result<Picked, NotPicked> {
    let v = v.trim();
    if let Some(k) = known.iter().find(|k| k.id.eq_ignore_ascii_case(v)) {
        let Some(next) = next_stage(k) else { return Err(NotPicked::Full(k.clone())) };
        d.set("seriesId", &k.id);
        d.set("stages", &k.stages.to_string());
        d.set("stage", &next.to_string());
        return Ok(Picked::Known(k.clone()));
    }
    match v {
        PICK_NONE => {
            for k in ["seriesId", "stage", "stages"] {
                d.set(k, "");
            }
            Ok(Picked::None)
        }
        PICK_NEW => {
            let id = fresh().map_err(NotPicked::Random)?;
            d.set("seriesId", &id);
            d.set("stage", "1");
            d.set("stages", "");
            Ok(Picked::New(id))
        }
        other => Err(NotPicked::Unknown(other.to_string())),
    }
}

pub fn span(t1: u64, t2: u64, offer_window: u64, ruling_window: u64, guild: bool) -> u64 {
    let seat = if guild { offer_window.saturating_mul(2).saturating_add(t1).saturating_mul(3) } else { offer_window.saturating_add(t1) };
    seat.saturating_add(t2).saturating_add(ruling_window)
}

#[derive(Clone, Debug)]
pub struct Mate {
    pub id: String,
    pub series: Series,
    pub cands: Vec<String>,
    pub terms: Option<eravon_terms::doc::Terms>,
}

pub fn of_terms(t: &eravon_terms::doc::Terms) -> Option<Series> {
    t.series.as_ref().map(|s| Series { id: s.id.to_ascii_lowercase(), stage: s.stage, stages: s.stages })
}

pub fn mates(data: &str, me: &str, id: &str, except: Option<&str>) -> Vec<Mate> {
    let mut out: Vec<Mate> = signed(data, me).into_iter().map(|(m, _)| m).filter(|m| m.series.id == id && Some(m.id.as_str()) != except).collect();
    out.sort_by_key(|m| m.series.stage);
    out
}

fn signed(data: &str, me: &str) -> Vec<(Mate, String)> {
    let Ok(s) = eravon_vault::store::open_read(data) else { return Vec::new() };
    let Ok(rows) = crate::goods::records_here(data, me) else { return Vec::new() };
    let mut out = Vec::new();
    for (owner, r) in rows.into_iter().filter(|(_, r)| r.kind == Kind::Product) {
        let me = owner.as_str();
        let Some(d) = s.part(me, Box_::Products, &r.id, crate::goods::PART_DRAFT).ok().and_then(|b| Draft::open(&b)) else { continue };
        let Ok(Some(series)) = cells(&d) else { continue };
        let terms = r
            .field(crate::goods::TERMS_ID)
            .and_then(|tid| s.part(me, Box_::Products, tid, crate::goods::PART_TERMS).ok())
            .and_then(|b| eravon_terms::doc::terms(&b).ok());
        if terms.is_none() {
            continue;
        }
        let name = d.name.clone();
        out.push((Mate { id: r.id.clone(), series, cands: d.cands.iter().map(|c| c.to_ascii_lowercase()).collect(), terms }, name));
    }
    out
}

pub fn judge_stage<'a>(s: &Series, cands: &[String], mates: &'a [Mate]) -> Result<Option<&'a eravon_terms::doc::Terms>, (&'static str, &'static str, String)> {
    if cands.len() != 1 {
        return Err(("cands", E_SERIES_ONE, format!("系列的每一段只许一位候选(判官或行会),现选 {} 位", cands.len())));
    }
    let mine = cands[0].to_ascii_lowercase();
    for m in mates {
        if m.series.stage == s.stage {
            return Err(("stage", E_SERIES_JUDGE, format!("这个系列的第 {} 段已有商品 {}", s.stage, m.id)));
        }
        if m.series.stages != s.stages {
            return Err(("stages", E_SERIES_JUDGE, format!("这个系列的商品 {} 写的是 {} 段", m.id, m.series.stages)));
        }
        if m.cands != [mine.clone()] {
            return Err(("cands", E_SERIES_JUDGE, format!("系列各段要同一位候选:商品 {} 选的是 {}", m.id, m.cands.join("、"))));
        }
    }
    if s.stage == 1 {
        return Ok(None);
    }
    match mates.iter().find(|m| m.series.stage == s.stage - 1).and_then(|m| m.terms.as_ref()) {
        Some(t) => Ok(Some(t)),
        None => Err(("stage", E_SERIES_PREV, format!("先写好并签出第 {} 段:这一段的开案窗与有效期由它推出", s.stage - 1))),
    }
}

pub fn floors(before: &eravon_terms::doc::Terms, clocks: &eravon_cands::lapse::Clocks) -> (u64, u64) {
    let guild = !before.guilds.is_empty();
    let sp = span(before.delivery_window, before.confirm_window, clocks.offer_window, clocks.ruling_window, guild);
    let sp = sp.saturating_add(1);
    (before.open_window.saturating_add(sp), before.expiry.saturating_add(sp))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(id: &str, stage: &str, stages: &str) -> Draft {
        Draft { series_id: id.into(), stage: stage.into(), stages: stages.into(), ..Draft::default() }
    }

    #[test]
    fn series_cells_read_whole_or_refuse_by_cell() {
        let id = format!("0x{}", "ab".repeat(32));
        assert_eq!(cells(&d("", "", "")), Ok(None));
        assert_eq!(cells(&d(&id, "2", "3")), Ok(Some(Series { id: id.clone(), stage: 2, stages: 3 })));
        assert_eq!(cells(&d(&format!("0x{}", "00".repeat(32)), "1", "2")).unwrap_err().0, "seriesId");
        assert_eq!(cells(&d("0x12", "1", "2")).unwrap_err().0, "seriesId");
        assert_eq!(cells(&d(&id, "0", "2")).unwrap_err().0, "stage");
        assert_eq!(cells(&d(&id, "3", "2")).unwrap_err().0, "stage");
        assert_eq!(cells(&d(&id, "", "2")).unwrap_err().0, "stage");
    }

    #[test]
    fn span_bounds_the_latest_end() {
        assert_eq!(span(10, 20, 3, 40, false), 3 + 10 + 20 + 40);
        assert_eq!(span(10, 20, 3, 40, true), 3 * (6 + 10) + 20 + 40);
        assert_eq!(span(u64::MAX, 1, 1, 1, true), u64::MAX);
    }

    #[test]
    fn a_stage_is_judged_against_its_series() {
        let s = |stage| Series { id: "0x1".into(), stage, stages: 3 };
        let m = |stage, c: &str| Mate { id: format!("p{stage}"), series: s(stage), cands: vec![c.into()], terms: None };
        let one = vec!["0xaa".to_string()];
        assert_eq!(judge_stage(&s(1), &["0xaa".into(), "0xbb".into()], &[]).unwrap_err().1, E_SERIES_ONE);
        assert!(judge_stage(&s(1), &one, &[]).unwrap().is_none());
        assert_eq!(judge_stage(&s(2), &one, &[m(1, "0xbb")]).unwrap_err().1, E_SERIES_JUDGE);
        assert_eq!(judge_stage(&s(2), &one, &[m(2, "0xaa")]).unwrap_err().1, E_SERIES_JUDGE);
        assert_eq!(judge_stage(&s(2), &one, &[m(1, "0xaa")]).unwrap_err().1, E_SERIES_PREV);
    }
}

#[cfg(test)]
mod pick_tests {
    use super::*;

    fn row(id: &str, name: &str, (sid, stage, stages): (&str, u64, u64)) -> Written {
        Written { id: id.into(), name: name.into(), series: Series { id: sid.into(), stage, stages } }
    }

    #[test]
    fn a_series_id_is_made_and_a_known_series_offers_its_next_stage() {
        let (a, b) = (fresh().expect("随机数"), fresh().expect("随机数"));
        assert_ne!(a, b);
        let mut d = crate::draft::Draft::default();
        for (k, v) in [("seriesId", a.as_str()), ("stage", "1"), ("stages", "3")] {
            d.set(k, v);
        }
        assert_eq!(cells(&d), Ok(Some(Series { id: a.clone(), stage: 1, stages: 3 })), "产品造的号照 cells 读得回");
        let s = "0x".to_string() + &"ab".repeat(32);
        let rows = [row("p2", "海报 · 二", (&s, 2, 3)), row("p1", "海报 · 一", (&s, 1, 3))];
        let k = known(&rows, None);
        assert_eq!(k, vec![Known { id: s.clone(), stages: 3, written: vec![1, 2], name: "海报 · 一".into() }]);
        assert_eq!(known(&rows, Some("p2"))[0].written, vec![1], "编辑中的那一件不占段");
        assert_eq!(next_stage(&k[0]), Some(3));
        assert_eq!(next_stage(&Known { written: vec![1, 2, 3], ..k[0].clone() }), None, "写满了不给段号");
    }

    #[test]
    fn the_series_pick_is_one_for_every_entry() {
        let s = "0x".to_string() + &"cd".repeat(32);
        let k = Known { id: s.clone(), stages: 3, written: vec![1], name: "海报 · 一".into() };
        let mut d = crate::draft::Draft::default();
        d.set("stages", "5");
        let Ok(Picked::New(id)) = pick(&mut d, PICK_NEW, &[k.clone()]) else { panic!("新系列该造号") };
        assert_eq!((d.series_id.as_str(), d.stage.as_str(), d.stages.as_str()), (id.as_str(), "1", ""), "新系列:产品造号、第 1 段、段数待填");
        assert!(id.len() == 66 && id.starts_with("0x"));
        assert_eq!(pick(&mut d, &s.to_ascii_uppercase().replace("0X", "0x"), &[k.clone()]), Ok(Picked::Known(k.clone())));
        assert_eq!((d.series_id.as_str(), d.stage.as_str(), d.stages.as_str()), (s.as_str(), "2", "3"), "本席的系列:段数与下一段填好");
        let before = d.clone();
        let full = Known { written: vec![1, 2, 3], ..k.clone() };
        assert_eq!(pick(&mut d, &s, &[full.clone()]).unwrap_err().name(), E_SERIES_JUDGE);
        let other = "0x".to_string() + &"ef".repeat(32);
        assert_eq!(pick(&mut d, &other, &[k.clone()]).unwrap_err().name(), E_SERIES_UNKNOWN, "手填的号不是本席的系列");
        assert_eq!(d, before, "拒了就一格不动");
        assert_eq!(pick(&mut d, PICK_NONE, &[]), Ok(Picked::None));
        assert_eq!((d.series_id.as_str(), d.stage.as_str(), d.stages.as_str()), ("", "", ""));
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;
    use eravon_store::vault::Seat;

    const NOW: u64 = 1_790_000_000;

    #[test]
    fn a_draft_never_signed_holds_no_stage_a_new_one_is_offered() {
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create("48261537", Seat::Seller, "主钥", Some("test test test test test test test test test test test junk"), NOW).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut session = eravon_vault::session::Session::locked(&data);
        session.unlock("48261537").expect("开得了");
        let me = eravon_vault::accounts::who(&data, Seat::Seller).expect("首枚钥").to_ascii_lowercase();
        let s = eravon_vault::store::open_write(&data, "series-test", NOW).expect("可写");
        s.init(&eravon_store::settings::Global::default()).expect("铺得下");
        s.init_identity(&me, NOW).expect("登记身份");
        let sid = format!("0x{}", "ab".repeat(32));
        let draft = |name: &str, stage: &str| Draft { name: name.into(), series_id: sid.clone(), stage: stage.into(), stages: "3".into(), ..Draft::default() }.seal();
        let terms = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base/eravon-conformance/terms-corpus/docs/d001_ok_full.md")).expect("条款样本");
        s.put(&me, Kind::TermsDocument, "terms-p1-v1", &[], &[(crate::goods::PART_TERMS.to_string(), terms)], NOW).expect("条款落档");
        s.put(&me, Kind::Product, "p1", &[(crate::goods::TERMS_ID.to_string(), "terms-p1-v1".to_string())], &[(crate::goods::PART_DRAFT.to_string(), draft("一", "1"))], NOW).expect("第一段落档");
        s.put(&me, Kind::Product, "p2", &[], &[(crate::goods::PART_DRAFT.to_string(), draft("二", "2"))], NOW + 1).expect("第二段草稿落档");
        drop(s);
        let w = stages(&data, &me);
        assert_eq!(w.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["p1"], "没签出的草稿不占段");
        let k = known(&w, None);
        assert_eq!(next_stage(&k[0]), Some(2), "下一段仍是第二段");
        assert_eq!(mates(&data, &me, &sid, None).len(), 1, "与 mates 同一读法");
    }
}
