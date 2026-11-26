use std::path::Path;

use eravon_evidence as k6;
use eravon_galeed as k1;
use eravon_standard as a1;
use eravon_watch::order::{self, Order};
use eravon_watch::send::Post;
use eravon_zikaron as a4;

use crate::{Why, Wire, a1w, case_of, why};

pub fn kits(home: &str, manual: Option<&str>) -> Result<(String, Vec<a4::index::Kit>), Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let s = a4::survey(home, manual).map_err(|e| why(e.name(), e.detail()))?;
    if let Some(a) = &s.absent {
        return Err(why(a.name(), a.detail()));
    }
    Ok((s.link.at, s.kits))
}

#[derive(Clone, Debug)]
pub struct Evidence {
    pub bytes: Vec<u8>,
    pub manifest: [u8; 32],
    pub files: Vec<(String, u64)>,
    pub source: String,
}

pub use k6::listing::charset_refused;

fn list_why(e: k6::listing::ListError) -> Why {
    match e {
        k6::listing::ListError::NameNotPortable(w) => why("E_DISPUTE_LIST", w),
        e => why("E_DISPUTE_LIST", format!("无法读取文件:{e:?}")),
    }
}

fn dropped(path: &str) -> Result<Vec<k6::sheet::FileItem>, Why> {
    let p = Path::new(path);
    let meta =
        std::fs::metadata(p).map_err(|e| why("E_DISPUTE_LIST", format!("无法读取 {path}:{e}")))?;
    if meta.is_dir() {
        return k6::listing::items_from_tree(p, "", a4::seam::MEDIA_DEFAULT).map_err(list_why);
    }
    if meta.len() > k6::listing::FILE_BOUND {
        return Err(why(
            "E_DISPUTE_LIST",
            format!("{path} 超过 16 MiB"),
        ));
    }
    let b = std::fs::read(p).map_err(|e| why("E_DISPUTE_LIST", format!("无法读取 {path}:{e}")))?;
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| why("E_DISPUTE_LIST", format!("{path} 文件名无效")))?;
    k6::listing::name_packable(name).map_err(|w| why("E_DISPUTE_LIST", w))?;
    Ok(vec![k6::sheet::listed(name, &b, a4::seam::MEDIA_DEFAULT).map_err(|u| why("E_DISPUTE_LIST", format!("{path}:{}", u.name())))?])
}

fn merge(
    kit: Vec<k6::sheet::FileItem>,
    drop: Vec<k6::sheet::FileItem>,
) -> Vec<k6::sheet::FileItem> {
    let mut all: Vec<k6::sheet::FileItem> = kit.into_iter().chain(drop).collect();
    all.sort_by(|a, b| a.name().as_bytes().cmp(b.name().as_bytes()));
    all
}


#[allow(clippy::too_many_arguments)]
pub fn sheet(
    home: &str,
    manual: Option<&str>,
    kit: Option<&str>,
    drop: Option<&str>,
    me: &str,
    o: &Order,
    chain_id: u64,
    logic: &str,
    note: Option<String>,
) -> Result<Evidence, Why> {
    let (items, source) = items(home, manual, kit, drop)?;
    sheet_of(items, source, me, o, chain_id, logic, note)
}

pub type Items = (Vec<k6::sheet::FileItem>, String);

pub fn items(home: &str, manual: Option<&str>, kit: Option<&str>, drop: Option<&str>) -> Result<Items, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let mut kit_root = String::new();
    let kit_items = match kit {
        None => Vec::new(),
        Some(id) => {
            let (_, list) = kits(home, manual)?;
            let k = a4::index::pick(&list, id).map_err(|e| why(e.name(), e.detail()))?;
            kit_root = k.path.clone();
            a4::pack::items(k, a4::seam::MEDIA_DEFAULT)
                .map_err(|e| why(e.name(), e.detail()))?
        }
    };
    let drop_items = match drop {
        None => Vec::new(),
        Some(p) => dropped(p)?,
    };
    let items = merge(kit_items, drop_items);
    if items.is_empty() {
        return Err(why(
            "E_DISPUTE_EMPTY",
            "一份证据也没有:上半选一个披露包,或下半拖入文件或目录",
        ));
    }
    Ok((items, drop.map(str::to_string).unwrap_or(kit_root)))
}

pub fn sheet_of(
    items: Vec<k6::sheet::FileItem>,
    source: String,
    me: &str,
    o: &Order,
    chain_id: u64,
    logic: &str,
    note: Option<String>,
) -> Result<Evidence, Why> {
    sheet_for(items, source, me, &o.instance, &o.case_id, chain_id, logic, note)
}

#[allow(clippy::too_many_arguments)]
pub fn sheet_for(
    items: Vec<k6::sheet::FileItem>,
    source: String,
    me: &str,
    instance: &str,
    case_id: &str,
    chain_id: u64,
    logic: &str,
    note: Option<String>,
) -> Result<Evidence, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let subject = k1::Value::Obj(vec![
        ("chainId".into(), k1::Value::Int(chain_id)),
        (
            "codeHash".into(),
            k1::Value::Str(a1::order::hex_of(&eravon_terms::derive::proxy_code_hash(
                logic,
            ))),
        ),
        ("contract".into(), k1::Value::Str(instance.to_string())),
        ("id".into(), k1::Value::Str(case_id.to_string())),
        ("kind".into(), k1::Value::Str("evm".into())),
    ]);
    let files = items.iter().map(|f| (f.name().to_string(), f.bytes())).collect();
    let (_, bytes, manifest) = a4::pack::manifest(subject, me, items, note.filter(|n| !n.trim().is_empty()))
        .map_err(|t| why(t.name(), "证据清单无效"))?;
    k6::sheet::check(&bytes).map_err(|e| why(e.name(), "证据清单无效"))?;
    Ok(Evidence {
        bytes,
        manifest,
        files,
        source,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn commit(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    o: &Order,
    ev: &Evidence,
    post: &mut dyn Post,
) -> Result<String, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
    let call = a1::call::Action::Commit {
        case_id: case_of(o)?,
        manifest: ev.manifest,
    }
    .call(&o.instance)
    .map_err(a1w)?;
    let tx = post.post(&call)?;
    let local = keep(data, writer, now, me, o, ev, &tx);
    eravon_watch::send::after_send(tx, local)
}

fn keep(data: &str, writer: &str, now: u64, me: &str, o: &Order, ev: &Evidence, tx: &str) -> Result<(), Why> {
    let mut o2 = order::load(data, me, &o.id)?;
    let n: u64 = o2
        .extra("evidence")
        .and_then(|x| x.parse().ok())
        .unwrap_or(0)
        + 1;
    o2.set_extra("evidence", &n.to_string());
    o2.set_extra("evidenceLast", &a1::order::hex_of(&ev.manifest));
    o2.set_extra("evidenceTx", tx);
    o2.set_extra(&key("Manifest", n), &a1::order::hex_of(&ev.manifest));
    o2.set_extra(&key("Source", n), &ev.source);
    o2.set_extra(&key("Tx", n), tx);
    order::save(
        data,
        writer,
        now,
        me,
        &o2,
        &[(part_name(n), ev.bytes.clone())],
    )
}

fn part_name(n: u64) -> String {
    format!("evidence-{n}.json")
}

fn key(what: &str, n: u64) -> String {
    format!("evidence{what}-{n}")
}

#[derive(Clone, Debug)]
pub struct Own {
    pub n: u64,
    pub manifest: String,
    pub source: String,
    pub tx: String,
    pub bytes: Vec<u8>,
    pub files: Result<Vec<(String, u64)>, Why>,
}

pub fn ours(data: &str, me: &str, o: &Order) -> Vec<Own> {
    let n: u64 = o.extra("evidence").and_then(|x| x.parse().ok()).unwrap_or(0);
    (1..=n)
        .map(|i| {
            let bytes = order::part(data, me, &o.id, &part_name(i)).unwrap_or_default();
            let files = k6::sheet::check(&bytes).map(|m| m.files.into_iter().map(|f| (f.name().to_string(), f.bytes())).collect()).map_err(|e| why(e.name(), "证据清单读不回"));
            Own {
                n: i,
                manifest: o.extra(&key("Manifest", i)).unwrap_or_default().to_string(),
                source: o.extra(&key("Source", i)).unwrap_or_default().to_string(),
                tx: o.extra(&key("Tx", i)).unwrap_or_default().to_string(),
                bytes,
                files,
            }
        })
        .collect()
}
