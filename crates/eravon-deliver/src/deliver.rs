use std::path::Path;

use eravon_evidence as k6;
use eravon_galeed as k1;
use eravon_standard as a1;
use eravon_store::seam::Box_;
use eravon_watch::order::Order;
use eravon_watch::send::Post;

use crate::{Why, Wire, a1w, why};

pub const FUNDED: u64 = 1;

pub fn phase(w: &Wire<'_>, instance: &str) -> Result<u64, Why> {
    let st = a1::chain::status(w.t, w.table, instance).map_err(a1w)?.value;
    st.get(a1::chain::status_col::PHASE).ok_or_else(|| why("E_DELIVER_READ", "status 没有 phase 一栏"))?.small().map_err(a1w)
}

pub fn gate(phase: u64) -> Result<(), Why> {
    if phase == FUNDED {
        Ok(())
    } else {
        Err(why("E_DELIVER_PHASE", format!("案在阶段 {phase}:交付只在开案受理之后、申报之前")))
    }
}


pub const MEDIA: &str = "application/octet-stream";

pub const MANIFEST_PART: &str = "eravon-delivery-manifest.json";

const MANIFEST_PART_BEFORE: &str = "manifest.json";

#[derive(Clone, Debug)]
pub struct Sheet {
    pub bytes: Vec<u8>,
    pub manifest: [u8; 32],
    pub files: Vec<(String, u64)>,
}

fn list_why(e: k6::listing::ListError) -> Why {
    match e {
        k6::listing::ListError::NameNotPortable(w) => why("E_DELIVER_LIST", w),
        e => why("E_DELIVER_LIST", format!("无法读取文件:{e:?}")),
    }
}

fn dir_items(dir: &Path) -> Result<Vec<k6::sheet::FileItem>, Why> {
    k6::listing::items_from_tree(dir, "", MEDIA).map_err(list_why)
}


pub fn sheet(
    path: &str,
    me: &str,
    o: &Order,
    chain_id: u64,
    logic: &str,
    note: Option<String>,
) -> Result<Sheet, Why> {
    sheet_of(items(path)?, me, o, chain_id, logic, note)
}

pub type Items = Vec<k6::sheet::FileItem>;

pub fn items(path: &str) -> Result<Items, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    if path.trim().is_empty() {
        return Ok(Vec::new());
    }
    let p = Path::new(path);
    let meta =
        std::fs::metadata(p).map_err(|e| why("E_DELIVER_LIST", format!("无法读取 {path}:{e}")))?;
    let items = if meta.is_dir() {
        dir_items(p)?
    } else {
        if meta.len() > k6::listing::FILE_BOUND {
            return Err(why(
                "E_DELIVER_LIST",
                format!("{path} 超过 16 MiB"),
            ));
        }
        let b =
            std::fs::read(p).map_err(|e| why("E_DELIVER_LIST", format!("无法读取 {path}:{e}")))?;
        let name = p
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| why("E_DELIVER_LIST", format!("{path} 文件名无效")))?;
        k6::listing::name_packable(name).map_err(|w| why("E_DELIVER_LIST", w))?;
        vec![k6::sheet::listed(name, &b, MEDIA).map_err(|u| why("E_DELIVER_LIST", format!("{path}:{}", u.name())))?]
    };
    Ok(items)
}

pub fn told(items: &Items, note: Option<&str>) -> Result<(), Why> {
    if items.is_empty() && note.is_none_or(|n| n.trim().is_empty()) {
        return Err(why(E_DELIVER_EMPTY, "没有交付物:拖入文件或写备注"));
    }
    Ok(())
}

pub const E_DELIVER_EMPTY: &str = "E_DELIVER_EMPTY";

pub fn sheet_of(
    items: Items,
    me: &str,
    o: &Order,
    chain_id: u64,
    logic: &str,
    note: Option<String>,
) -> Result<Sheet, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    told(&items, note.as_deref())?;
    let subject = k1::Value::Obj(vec![
        ("chainId".into(), k1::Value::Int(chain_id)),
        (
            "codeHash".into(),
            k1::Value::Str(a1::order::hex_of(&eravon_terms::derive::proxy_code_hash(
                logic,
            ))),
        ),
        ("contract".into(), k1::Value::Str(o.instance.clone())),
        ("id".into(), k1::Value::Str(o.case_id.clone())),
        ("kind".into(), k1::Value::Str("evm".into())),
    ]);
    let files: Vec<(String, u64)> = items.iter().map(|f| (f.name().to_string(), f.bytes())).collect();
    let m = k6::listing::from_items(subject, me, items, note.filter(|n| !n.trim().is_empty()));
    let bytes = k6::sheet::build(&m).map_err(|t| why(t.name(), "文件清单无效"))?;
    k6::sheet::check(&bytes).map_err(|e| why(e.name(), "文件清单无效"))?;
    Ok(Sheet {
        manifest: k6::listing::claim_parameter(&m).map_err(|t| why(t.name(), "文件清单无效"))?,
        bytes,
        files,
    })
}

fn manifest_of(part: impl Fn(&str) -> Option<Vec<u8>>) -> Vec<u8> {
    part(MANIFEST_PART).or_else(|| part(MANIFEST_PART_BEFORE)).unwrap_or_default()
}

pub fn record_id(o: &Order) -> String {
    format!("{}-delivery", o.id)
}

#[allow(clippy::too_many_arguments)]
fn record(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    o: &Order,
    s: &Sheet,
    method: &str,
    note: &str,
    source: &str,
    tx: &str,
) -> Result<(), Why> {
    let st = eravon_vault::store::open_write(data, writer, now)
        .map_err(|e| why(e.name(), e.detail()))?;
    st.put(
        me,
        eravon_store::record::Kind::DeliveryListing,
        &record_id(o),
        &[
            ("instance".into(), o.instance.clone()),
            ("manifest".into(), a1::order::hex_of(&s.manifest)),
            ("method".into(), method.to_string()),
            ("note".into(), note.to_string()),
            ("order".into(), o.id.clone()),
            ("source".into(), source.to_string()),
            ("tx".into(), tx.to_string()),
        ],
        &[(MANIFEST_PART.into(), s.bytes.clone())],
        now,
    )
    .map_err(|e| why(e.name(), e.detail()))?;
    Ok(())
}


#[allow(clippy::too_many_arguments)]
pub fn claim(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    o: &Order,
    s: &Sheet,
    method: &str,
    note: &str,
    source: &str,
    post: &mut dyn Post,
) -> Result<String, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    gate(phase(w, &o.instance)?)?;
    a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
    let tx = post.post(
        &a1::call::Action::ClaimDelivery(s.manifest)
            .call(&o.instance)
            .map_err(a1w)?,
    )?;
    let local = record(data, writer, now, me, o, s, method, note, source, &tx);
    eravon_watch::send::after_send(tx, local)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandOver {
    pub claim_block: u64,
    pub finalized: u64,
    pub ready: bool,
}

pub fn hand_over(w: &Wire<'_>, tx: &str) -> Result<HandOver, Why> {
    let chain = |e: eravon_chain::fault::Fault| why(&e.name(), e.detail());
    let r = eravon_chain::read::read_agreed(w.t, w.table, eravon_chain::seam::M_GET_RECEIPT, vec![eravon_literals::json::Out::S(tx.to_string())]).map_err(chain)?;
    let num = |v: &eravon_chain::json_in::In, what: &str| -> Result<u64, Why> {
        let s = v.get("blockNumber").and_then(|x| x.as_str()).ok_or_else(|| why("E_DELIVER_READ", format!("{what}没有 blockNumber")))?;
        eravon_chain::seam::read_quantity(s).map_err(chain)
    };
    if matches!(r.value, eravon_chain::json_in::In::Null) {
        return Err(why("E_DELIVER_READ", format!("申报交易 {tx} 的回执还读不到")));
    }
    let claimed = num(&r.value, "申报交易的回执")?;
    let f = eravon_chain::read::read_agreed(w.t, w.table, eravon_chain::seam::M_GET_BLOCK, vec![eravon_literals::json::Out::S(eravon_chain::seam::TAG_FINALIZED.into()), eravon_literals::json::Out::B(false)]).map_err(chain)?;
    let fin = f.value.get("number").and_then(|x| x.as_str()).map(eravon_chain::seam::read_quantity).transpose().map_err(chain)?.unwrap_or(0);
    Ok(HandOver { claim_block: claimed, finalized: fin, ready: fin >= claimed })
}

#[derive(Clone, Debug)]
pub struct Delivered {
    pub manifest: String,
    pub method: String,
    pub note: String,
    pub source: String,
    pub tx: String,
    pub at: u64,
    pub bytes: Vec<u8>,
    pub files: Result<Vec<(String, u64, String)>, Why>,
}

pub fn delivery(data: &str, me: &str, o: &Order) -> Option<Delivered> {
    let s = eravon_vault::store::open_read(data).ok()?;
    let me = &eravon_watch::order::owner_of(data, me, &record_id(o));
    let r = s.get(me, Box_::Orders, &record_id(o)).ok()?;
    let get = |k: &str| r.fields.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone()).unwrap_or_default();
    let bytes = manifest_of(|name| s.part(me, Box_::Orders, &record_id(o), name).ok());
    let files = files_of(&bytes);
    Some(Delivered {
        manifest: get("manifest"),
        method: get("method"),
        note: get("note"),
        source: get("source"),
        tx: get("tx"),
        at: r.updated,
        bytes,
        files,
    })
}

fn files_of(bytes: &[u8]) -> Result<Vec<(String, u64, String)>, Why> {
    k6::sheet::check(bytes)
        .map(|m| m.files.into_iter().map(|f| (f.name().to_string(), f.bytes(), f.sha256().to_string())).collect())
        .map_err(|e| why(e.name(), "交付记录的文件清单读不回"))
}


pub fn save_originals(d: &Delivered, dir: &str) -> Result<Vec<String>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    if d.source.is_empty() {
        return Err(why("E_DELIVER_ORIGINAL", "该订单不是在本机交付的"));
    }
    let files = d.files.as_ref().map_err(Clone::clone)?;
    let src = std::path::Path::new(&d.source);
    let single = src.is_file();
    let target = std::path::Path::new(dir);
    let under = |root: &std::path::Path, name: &str| eravon_literals::seam::named_under(root, name).map_err(|w| why("E_DELIVER_ORIGINAL", w));
    let mut ready: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
    for (name, _, sha) in files {
        let to = under(target, name)?;
        let from = if single { src.to_path_buf() } else { under(src, name)? };
        let b = std::fs::read(&from).map_err(|e| why("E_DELIVER_ORIGINAL", format!("无法获取原件 {}:{e}", from.display())))?;
        let (_, h) = k6::sheet::file_view(&b);
        if !a1::order::hex_of(&h).eq_ignore_ascii_case(sha) {
            return Err(why("E_DELIVER_ORIGINAL", format!("{name} 在交付后被修改过")));
        }
        ready.push((to, b));
    }
    let mut out = Vec::new();
    for (to, b) in ready {
        if let Some(p) = to.parent() {
            std::fs::create_dir_all(p).map_err(|e| why("E_DELIVER_ORIGINAL", format!("无法创建 {}:{e}", p.display())))?;
        }
        std::fs::write(&to, b).map_err(|e| why("E_DELIVER_ORIGINAL", format!("写不进 {}:{e}", to.display())))?;
        out.push(to.to_string_lossy().to_string());
    }
    Ok(out)
}

pub fn delivered(data: &str, me: &str, o: &Order) -> Option<(String, String, String)> {
    let s = eravon_vault::store::open_read(data).ok()?;
    let me = &eravon_watch::order::owner_of(data, me, &record_id(o));
    let r = s.get(me, Box_::Orders, &record_id(o)).ok()?;
    let get = |k: &str| {
        r.fields
            .iter()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    Some((get("manifest"), get("method"), get("tx")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_manifest_reads_back_under_the_new_name_and_one_old_one() {
        let has = |names: &'static [(&'static str, &'static [u8])]| move |n: &str| names.iter().find(|(k, _)| *k == n).map(|(_, b)| b.to_vec());
        assert_eq!(manifest_of(has(&[(MANIFEST_PART, b"new")])), b"new");
        assert_eq!(manifest_of(has(&[(MANIFEST_PART_BEFORE, b"old")])), b"old", "written before: read back");
        assert_eq!(manifest_of(has(&[(MANIFEST_PART, b"new"), (MANIFEST_PART_BEFORE, b"old")])), b"new", "both: the new one");
        assert!(manifest_of(has(&[])).is_empty(), "neither: empty");
        assert_ne!(MANIFEST_PART, MANIFEST_PART_BEFORE);
    }
    use super::*;

    #[test]
    fn a_record_naming_a_file_outside_the_folders_reads_and_writes_nothing() {
        let w = std::env::temp_dir().join(format!("eravon-deliver-names-{}", std::process::id()));
        let (src, dir) = (w.join("src"), w.join("out"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(w.join("secret.txt"), b"x").unwrap();
        let (_, h) = k6::sheet::file_view(b"x");
        for name in ["../secret.txt", "..\\secret.txt", "/etc/hosts", "a//b"] {
            let d = Delivered { source: src.display().to_string(), files: Ok(vec![(name.to_string(), 1, a1::order::hex_of(&h))]), manifest: String::new(), method: String::new(), note: String::new(), tx: String::new(), at: 0, bytes: Vec::new() };
            let got = save_originals(&d, &dir.display().to_string());
            assert!(got.is_err(), "{name:?} 该拒");
            assert!(!dir.exists(), "{name:?} 拒了却建了目标目录");
        }
        let _ = std::fs::remove_dir_all(&w);
    }

    #[test]
    fn a_record_whose_manifest_does_not_read_back_is_refused_and_saves_nothing() {
        let data = eravon_vault::testing::fresh();
        let now = 1_790_000_000;
        let mut v = eravon_vault::vault::create("48261537", eravon_store::vault::Seat::Seller, "主钥", None, now).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut session = eravon_vault::session::Session::locked(&data);
        session.unlock("48261537").expect("开得了");
        let me = v.address.clone();
        let w = std::env::temp_dir().join(format!("eravon-deliver-unread-{}", std::process::id()));
        let (src, out) = (w.join("src"), w.join("out"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"x").unwrap();
        let o = Order {
            id: "o1".into(),
            case_id: format!("0x{}", "11".repeat(32)),
            instance: format!("0x{}", "22".repeat(20)),
            buyer_key: format!("0x{}", "33".repeat(20)),
            buyer_payout: String::new(),
            verdict: String::new(),
            token: String::new(),
            need: String::new(),
            open_by: 0,
            candidate: 0,
            expiry: 0,
            state: eravon_watch::order::State::Awaiting,
            balance: String::new(),
            gap: String::new(),
            note: String::new(),
            notice: String::new(),
            notice_at: 0,
            deploy_tx: String::new(),
            expire_tx: String::new(),
            source: String::new(),
            opened: 0,
            extra: Vec::new(),
        };
        let good = sheet(&src.display().to_string(), &me, &o, 31_337, &format!("0x{}", "44".repeat(20)), None).expect("清单建得成").bytes;
        let text = String::from_utf8(good.clone()).unwrap();
        assert!(text.contains("\"a.txt\""));
        let control = text.replacen("\"a.txt\"", "\"a\\u0001.txt\"", 1).into_bytes();
        let st = eravon_vault::store::open_write(&data, "eravon-test", now).expect("可写");
        st.init(&eravon_store::settings::Global::default()).expect("铺得下");
        st.init_identity(&me, now).expect("登记身份");
        let put = |parts: &[(String, Vec<u8>)]| {
            let fields = [("manifest".to_string(), String::new()), ("order".to_string(), o.id.clone()), ("source".to_string(), src.display().to_string())];
            st.put(&me, eravon_store::record::Kind::DeliveryListing, &record_id(&o), &fields, parts, now).expect("落档");
        };
        put(&[(MANIFEST_PART.into(), good)]);
        let d = delivery(&data, &me, &o).expect("有交付记录");
        assert_eq!(d.files.as_ref().map(Vec::len), Ok(1), "读得回的清单照列");
        assert_eq!(save_originals(&d, &out.display().to_string()).map(|p| p.len()), Ok(1), "读得回的照另存");
        std::fs::remove_dir_all(&out).unwrap();
        for (form, bytes) in [("控制字符", control), ("不是清单", b"{}".to_vec()), ("没有清单", Vec::new())] {
            let refused = k6::sheet::check(&bytes).expect_err(form).name().to_string();
            put(&[(MANIFEST_PART.into(), bytes)]);
            let d = delivery(&data, &me, &o).expect("有交付记录");
            assert_eq!(d.files.as_ref().map_err(|(n, _)| n.as_str()), Err(refused.as_str()), "{form}:读不回带拒,不折成空表");
            assert_eq!(save_originals(&d, &out.display().to_string()).map_err(|(n, _)| n), Err(refused.clone()), "{form}:另存答拒");
            assert!(!out.exists(), "{form}:拒了却建了目标目录");
        }
        let _ = std::fs::remove_dir_all(&w);
    }

    fn bare_order() -> Order {
        Order {
            id: "o2".into(),
            case_id: format!("0x{}", "11".repeat(32)),
            instance: format!("0x{}", "22".repeat(20)),
            buyer_key: format!("0x{}", "33".repeat(20)),
            buyer_payout: String::new(),
            verdict: String::new(),
            token: String::new(),
            need: String::new(),
            open_by: 0,
            candidate: 0,
            expiry: 0,
            state: eravon_watch::order::State::Awaiting,
            balance: String::new(),
            gap: String::new(),
            note: String::new(),
            notice: String::new(),
            notice_at: 0,
            deploy_tx: String::new(),
            expire_tx: String::new(),
            source: String::new(),
            opened: 0,
            extra: Vec::new(),
        }
    }

    #[test]
    fn a_note_alone_is_a_delivery_and_nothing_at_all_is_refused() {
        let (me, logic) = (format!("0x{}", "55".repeat(20)), format!("0x{}", "44".repeat(20)));
        let o = bare_order();
        let w = std::env::temp_dir().join(format!("eravon-deliver-note-{}", std::process::id()));
        std::fs::create_dir_all(&w).unwrap();
        let empty = w.display().to_string();
        assert_eq!(items("").map(|v| v.len()), Ok(0));
        for path in ["", empty.as_str()] {
            let one = sheet(path, &me, &o, 31_337, &logic, Some("顺丰 SF1234567890".into())).expect("只有备注也成清单");
            assert!(one.files.is_empty());
            let m = k6::sheet::check(&one.bytes).expect("读得回");
            assert!(m.files.is_empty());
            assert!(String::from_utf8(one.bytes.clone()).unwrap().contains("SF1234567890"), "备注随清单");
            let other = sheet(path, &me, &o, 31_337, &logic, Some("顺丰 SF1234567891".into())).unwrap();
            assert_ne!(one.manifest, other.manifest, "哈希盖住备注");
            for note in [None, Some(String::new()), Some("  ".into())] {
                assert_eq!(sheet(path, &me, &o, 31_337, &logic, note).map(|_| ()).map_err(|(n, _)| n), Err(E_DELIVER_EMPTY.to_string()));
            }
        }
        let _ = std::fs::remove_dir_all(&w);
    }

    fn node(finalized: u64) -> String {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}", l.local_addr().expect("addr"));
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                std::thread::spawn(move || {
                    let mut s = s;
                    let mut raw = Vec::new();
                    let mut buf = [0u8; 4096];
                    let req = loop {
                        match s.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => raw.extend_from_slice(&buf[..n]),
                        }
                        let text = String::from_utf8_lossy(&raw).to_string();
                        if let Some(i) = text.find("\r\n\r\n") {
                            let len: usize = text[..i].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|n| n.trim().parse().ok())).unwrap_or(0);
                            if raw.len() >= i + 4 + len {
                                break text[i + 4..].to_string();
                            }
                        }
                    };
                    let id = req.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                    let method = req.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                    let first = req.split("\"params\":[\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                    let block = |n: u64| format!(r#"{{"baseFeePerGas":"0x7","number":"0x{n:x}","timestamp":"0x{:x}"}}"#, n * 12);
                    let result = match method.as_str() {
                        "eth_chainId" => "\"0x7a69\"".to_string(),
                        "eth_getTransactionReceipt" => r#"{"blockNumber":"0x5","contractAddress":null,"effectiveGasPrice":"0x1","gasUsed":"0x5208","logs":[],"status":"0x1"}"#.to_string(),
                        "eth_getBlockByNumber" if first == "finalized" => block(finalized),
                        "eth_getBlockByNumber" => block(eravon_chain::seam::read_quantity(&first).unwrap_or(0)),
                        _ => "null".to_string(),
                    };
                    let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}");
                    let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
                });
            }
        });
        url
    }

    #[test]
    fn endpoints_a_finalized_block_apart_read_the_hand_over_at_the_lower() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        eravon_chain::transport::set_way(eravon_chain::transport::choice_of("none").unwrap());
        let t = eravon_chain::transport::Auto::web_roots().unwrap().with_timeout(std::time::Duration::from_secs(5));
        let tx = "0x00000000000000000000000000000000000000000000000000000000000000ab";
        for ((a, b), want) in [((8, 9), HandOver { claim_block: 5, finalized: 8, ready: true }), ((4, 5), HandOver { claim_block: 5, finalized: 4, ready: false })] {
            let table = eravon_chain::endpoints::Table::new(vec![node(a), node(b)]).unwrap();
            let w = Wire { t: &t, table: &table, chain_id: 31337, logic: "0x0000000000000000000000000000000000000001", registries: &[] };
            assert_eq!(hand_over(&w, tx), Ok(want), "finalized {a} and {b}");
        }
    }
}
