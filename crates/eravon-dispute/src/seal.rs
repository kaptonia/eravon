use std::path::Path;

use eravon_evidence as k6;
use eravon_zikaron as a4;

use crate::{Why, why};

pub type Pick = k6::recipients::Pick;

pub const E_SEAL_SEATS: &str = "E_EVIDENCE_SEATS";
pub const E_SEAL_RECIPIENT: &str = "E_EVIDENCE_NO_RECIPIENT";
pub const E_SEAL_CHANGED: &str = "E_EVIDENCE_CHANGED";
pub const E_SEAL_NAME: &str = "E_EVIDENCE_NAME_TAKEN";
pub const E_SEAL_WRITE: &str = "E_EVIDENCE_WRITE";
pub const E_SEAL_EXISTS: &str = "E_EVIDENCE_EXISTS";

pub const MANIFEST_NAME: &str = "manifest.json";
pub const MANIFEST_MEDIA: &str = "application/json";
pub const SEALED_SUFFIX: &str = ".age";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Origins {
    pub kit_root: Option<String>,
    pub drop: Option<String>,
}

pub fn origins(home: &str, manual: Option<&str>, kit: Option<&str>, drop: Option<&str>) -> Result<Origins, Why> {
    let kit_root = match kit {
        None => None,
        Some(id) => {
            let (_, list) = crate::evidence::kits(home, manual)?;
            Some(a4::index::pick(&list, id).map_err(|e| why(e.name(), e.detail()))?.path.clone())
        }
    };
    Ok(Origins { kit_root, drop: drop.map(str::to_string) })
}

pub fn pick_from(ledger: &Result<eravon_scan::ledger::Ledger, String>) -> Pick {
    match ledger.as_ref().ok().and_then(eravon_scan::ledger::Ledger::audit_input).and_then(|b| eravon_galeed::canon::read(&b).ok()) {
        Some(input) => k6::recipients::pick(&input),
        None => Pick::Unread,
    }
}


fn seal_one(to: &k6::recipients::Binding, name: &str, media_type: &str, b: &[u8]) -> Result<Vec<u8>, Why> {
    match k6::recipients::seal(to, name, media_type, b) {
        Ok(s) => Ok(s.ciphertext),
        Err(k6::recipients::Unsealed::Recipient(r)) => Err(why(E_SEAL_RECIPIENT, format!("判官的收件钥不能用({}):不导出,也不发明文", r.name()))),
        Err(k6::recipients::Unsealed::Listing(u)) => Err(why(E_SEAL_CHANGED, format!("{name} 列不进清单({})", u.name()))),
    }
}


pub fn recipient(seats: &[Pick]) -> Result<k6::recipients::Binding, Why> {
    match seats {
        [Pick::One(b)] => Ok(b.clone()),
        [] => Err(why(E_SEAL_RECIPIENT, "这一案还没有在席的判官:没有收件钥")),
        [Pick::Candidates { list, .. }] => on_candidates(list, &seats[0]),
        [_] => Err(why(E_SEAL_RECIPIENT, format!("判官的收件钥挑不出唯一一把({}):不导出,也不发明文", said(&seats[0])))),
        _ => Err(why(E_SEAL_SEATS, format!("这一案在席的判官有 {} 位:本端只封给一把收件钥,不逐个另封、不发明文", seats.len()))),
    }
}

fn on_candidates(_list: &[k6::recipients::Binding], p: &Pick) -> Result<k6::recipients::Binding, Why> {
    Err(why(E_SEAL_RECIPIENT, format!("判官的收件钥挑不出唯一一把({}):不导出,也不发明文", said(p))))
}


pub fn said(p: &Pick) -> String {
    match p {
        Pick::One(b) => format!("{} · {}", Pick::ONE, b.recipient()),
        Pick::Candidates { reason, list, .. } => format!("{} · {} · {} 把", Pick::CANDIDATES, reason.name(), list.len()),
        Pick::None => Pick::NONE.to_string(),
        Pick::Unread => Pick::UNREAD.to_string(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    pub dir: String,
    pub files: Vec<String>,
}

pub fn export(o: &Origins, manifest: &[u8], seats: &[Pick], dir: &str) -> Result<Sealed, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let to = recipient(seats)?;
    let m = k6::sheet::check(manifest).map_err(|e| why("E_DISPUTE_LIST", format!("清单读不成:{e:?}")))?;
    let target = Path::new(dir);
    let under = |root: &Path, name: &str| eravon_literals::seam::named_under(root, name).map_err(|w| why(E_SEAL_CHANGED, w));
    let kit = o.kit_root.as_deref().map(a4::pack::tree).transpose().map_err(|e| why(e.name(), e.detail()))?;
    let o = &Origins { kit_root: kit.as_ref().map(|t| t.root().to_string_lossy().to_string()), drop: o.drop.clone() };
    let mut ready: Vec<(std::path::PathBuf, String, String, Vec<u8>)> = Vec::new();
    for f in &m.files {
        if f.name() == MANIFEST_NAME {
            return Err(why(E_SEAL_NAME, format!("清单里有一件也叫 {MANIFEST_NAME}:与清单本身的密文同名")));
        }
        let from = original(o, f.name(), &under)?;
        let b = std::fs::read(&from).map_err(|e| why(E_SEAL_CHANGED, format!("读不到原件 {}:{e}", from.display())))?;
        let (len, sha) = k6::sheet::file_view(&b);
        if len != f.bytes() || !eravon_galeed::hash::hex_encode(&sha).eq_ignore_ascii_case(f.sha256()) {
            return Err(why(E_SEAL_CHANGED, format!("{} 已不是清单所列的那一份(列的是 {} 字节)", f.name(), f.bytes())));
        }
        ready.push((under(target, &format!("{}{SEALED_SUFFIX}", f.name()))?, f.name().to_string(), f.media_type().to_string(), b));
    }
    ready.push((under(target, &format!("{MANIFEST_NAME}{SEALED_SUFFIX}"))?, MANIFEST_NAME.to_string(), MANIFEST_MEDIA.to_string(), manifest.to_vec()));
    let mut sealed: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::with_capacity(ready.len());
    for (p, name, media, b) in ready {
        let ct = seal_one(&to, &name, &media, &b)?;
        sealed.push((p, ct));
    }
    let tops = tops(target, &sealed)?;
    taken(target, &tops)?;
    let made = make_folder(target)?;
    match land(target, &tops, &sealed) {
        Ok(()) => Ok(Sealed { dir: dir.to_string(), files: sealed.iter().map(|(p, _)| p.to_string_lossy().to_string()).collect() }),
        Err(e) => {
            for d in made.iter().rev() {
                let _ = std::fs::remove_dir(d);
            }
            Err(e)
        }
    }
}

fn tops(target: &Path, sealed: &[(std::path::PathBuf, Vec<u8>)]) -> Result<Vec<String>, Why> {
    let mut out: Vec<String> = Vec::new();
    for (p, _) in sealed {
        let top = p
            .strip_prefix(target)
            .ok()
            .and_then(|r| r.components().next())
            .and_then(|c| c.as_os_str().to_str())
            .map(str::to_string)
            .ok_or_else(|| why(E_SEAL_WRITE, format!("{} 不在所选文件夹之下", p.display())))?;
        if !out.contains(&top) {
            out.push(top);
        }
    }
    Ok(out)
}

fn taken(target: &Path, tops: &[String]) -> Result<(), Why> {
    let there: Vec<&str> = tops.iter().filter(|n| std::fs::symlink_metadata(target.join(n)).is_ok()).map(String::as_str).collect();
    if there.is_empty() {
        return Ok(());
    }
    Err(why(E_SEAL_EXISTS, format!("所选文件夹 {} 里已有同名者:{};一个字节也没写", target.display(), there.join("、"))))
}


fn back(placed: &[(std::path::PathBuf, bool)]) {
    for (at, tree) in placed.iter().rev() {
        eravon_store::landing::take_back(at, *tree);
    }
}


fn make_folder(target: &Path) -> Result<Vec<std::path::PathBuf>, Why> {
    if target.is_dir() {
        return Ok(Vec::new());
    }
    if std::fs::symlink_metadata(target).is_ok() {
        return Err(why(E_SEAL_WRITE, format!("{} 不是文件夹", target.display())));
    }
    let mut missing: Vec<&Path> = target.ancestors().take_while(|a| !a.as_os_str().is_empty() && std::fs::symlink_metadata(a).is_err()).collect();
    missing.reverse();
    let mut made = Vec::with_capacity(missing.len());
    for d in missing {
        if let Err(e) = std::fs::create_dir(d) {
            for m in made.iter().rev() {
                let _ = std::fs::remove_dir(m);
            }
            return Err(why(E_SEAL_WRITE, format!("无法创建 {}:{e}", d.display())));
        }
        made.push(d.to_path_buf());
    }
    Ok(made)
}

fn land(target: &Path, tops: &[String], sealed: &[(std::path::PathBuf, Vec<u8>)]) -> Result<(), Why> {
    use eravon_store::landing;
    let write = |t: landing::Trouble| why(E_SEAL_WRITE, format!("{}:{}", t.subject(), t.said().unwrap_or(t.code())));
    let staging = landing::staging_beside(&target.join(MANIFEST_NAME)).map_err(write)?;
    std::fs::create_dir(&staging).map_err(|e| why(E_SEAL_WRITE, format!("无法创建 {}:{e}", staging.display())))?;
    let laid = (|| -> Result<(), Why> {
        for (p, ct) in sealed {
            let rel = p.strip_prefix(target).map_err(|_| why(E_SEAL_WRITE, format!("{} 不在所选文件夹之下", p.display())))?;
            let at = staging.join(rel);
            if let Some(parent) = at.parent() {
                landing::mkdir(parent).map_err(write)?;
            }
            landing::put_new_from(&at, &mut &ct[..]).map_err(write)?;
        }
        Ok(())
    })();
    let mut placed: Vec<(std::path::PathBuf, bool)> = Vec::new();
    let result = laid.and_then(|()| {
        for top in tops {
            #[cfg(test)]
            if placed.len() >= FAIL_AFTER.with(|c| c.get()) {
                return Err(why(E_SEAL_WRITE, format!("{top}:placing refused (pretended)")));
            }
            let (from, at) = (staging.join(top), target.join(top));
            let tree = from.is_dir();
            let r = if tree { landing::land_tree(&at, &from) } else { landing::place_file(&from, &at) };
            r.map_err(|t| match t {
                landing::Trouble::Occupied(_) => why(E_SEAL_EXISTS, format!("所选文件夹 {} 里已有同名者:{top}", target.display())),
                t => write(t),
            })?;
            placed.push((at, tree));
        }
        Ok(())
    });
    if result.is_err() {
        back(&placed);
    }
    let _ = std::fs::remove_dir_all(&staging);
    result
}

#[cfg(test)]
thread_local! {
    static FAIL_AFTER: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

fn original(o: &Origins, name: &str, under: &dyn Fn(&Path, &str) -> Result<std::path::PathBuf, Why>) -> Result<std::path::PathBuf, Why> {
    if let (Some(rest), Some(root)) = (name.strip_prefix(a4::seam::kit_prefix()), o.kit_root.as_deref()) {
        return under(Path::new(root), rest);
    }
    let Some(drop) = o.drop.as_deref() else { return Err(why(E_SEAL_CHANGED, format!("找不到 {name} 的原件所在"))) };
    let p = Path::new(drop);
    if p.is_file() {
        return if p.file_name().and_then(|n| n.to_str()) == Some(name) { Ok(p.to_path_buf()) } else { Err(why(E_SEAL_CHANGED, format!("找不到 {name} 的原件所在"))) };
    }
    under(p, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("eravon-seal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("临时目录");
        d
    }

    fn put(p: &Path, b: &[u8]) {
        std::fs::create_dir_all(p.parent().expect("上一层")).expect("目录");
        std::fs::write(p, b).expect("写");
    }

    fn sheet(root: &Path) -> (Origins, Vec<u8>) {
        let drop = root.join("drop");
        let kit = root.join("kit");
        put(&drop.join("contract.pdf"), b"%PDF contract");
        put(&drop.join("chat/day1.txt"), "第一天的聊天".as_bytes());
        put(&kit.join("records/one.json"), b"{\"r\":1}");
        let mut items = k6::listing::items_from_tree(&drop, "", "application/octet-stream").expect("拖入那一半");
        items.extend(k6::listing::items_from_tree(&kit, a4::seam::kit_prefix(), "application/octet-stream").expect("披露包那一半"));
        items.sort_by(|a, b| a.name().as_bytes().cmp(b.name().as_bytes()));
        let me = format!("0x{}", "a1".repeat(20));
        let ev = crate::evidence::sheet_for(items, String::new(), &me, &format!("0x{}", "c2".repeat(20)), &format!("0x{}", "d4".repeat(32)), 1, &format!("0x{}", "ee".repeat(20)), None).expect("清单");
        (Origins { kit_root: Some(kit.to_string_lossy().to_string()), drop: Some(drop.to_string_lossy().to_string()) }, ev.bytes)
    }

    fn one(recipient: &str) -> Pick {
        use galeed::json::{self, Value};
        let mut k = [0u8; 32];
        k[31] = 1;
        let key = galeed::cryptox::Privkey::from_bytes(&k).expect("一把钥");
        let judge = galeed::hexform::encode(&key.address());
        let entry = |seq: u64, prev: Option<&str>, kind: &str, body: &str| {
            let mut six = vec![
                ("body".to_string(), json::read(body.as_bytes()).expect("体")),
                ("entryType".to_string(), Value::Str(kind.into())),
                ("judge".to_string(), Value::Str(judge.clone())),
                ("prev".to_string(), prev.map_or(Value::Null, |p| Value::Str(p.into()))),
                ("seq".to_string(), Value::Int(seq)),
                ("spec".to_string(), Value::Str(galeed::LAW.into())),
            ];
            let signed = galeed::sign::sign_as(&key, &json::canonical(&Value::Obj(six.clone())).expect("正典"), galeed::literals::SEAT_JUDGE, galeed::LAW).expect("签");
            six.push(("sig".to_string(), Value::Str(galeed::hexform::encode(&signed.sig))));
            json::canonical(&Value::Obj(six)).expect("正典")
        };
        let id = |b: &[u8]| galeed::hexform::encode(&galeed::cryptox::sha256(b));
        let g = entry(0, None, "genesis", r#"{"statement_md":""}"#);
        let b1 = entry(1, Some(&id(&g)), "keybinding", &format!(r#"{{"effective":1,"recipient":"{recipient}"}}"#));
        let rec = |n: u64, b: &[u8]| format!(r#"{{"chainId":1,"blockNumber":{n},"blockTimestamp":{n},"tx":"0x{n:064x}","sender":"{judge}","hash":"{}","verdict":"counted","entry":"{}"}}"#, id(b), galeed::hexform::encode(b));
        let text = format!(r#"{{"root":"{judge}","anchors":[{},{}],"basis":{{"chains":[{{"chainId":1,"fromBlock":0,"toBlock":100,"registries":[],"senders":["{judge}"]}}]}}}}"#, rec(1, &g), rec(2, &b1));
        let p = k6::recipients::pick(&json::read_open(text.as_bytes()).expect("审计输入"));
        assert!(matches!(p, Pick::One(_)), "挑出一枚:{p:?}");
        p
    }

    #[test]
    fn sealed_files_open_with_the_judge_key_under_their_names() {
        let root = tmp("open");
        let (o, manifest) = sheet(&root);
        let judge = galeed::cryptox::AgeIdentity::generate();
        let out = root.join("out");
        let s = export(&o, &manifest, &[one(&judge.recipient())], &out.to_string_lossy()).expect("导出");
        let m = k6::sheet::check(&manifest).expect("清单读回");
        assert_eq!(s.files.len(), m.files.len() + 1);
        for f in &m.files {
            let ct = std::fs::read(out.join(format!("{}{SEALED_SUFFIX}", f.name()))).expect("密文");
            let pt = galeed_evidence::decrypt(&judge, &ct).expect("判官钥解得开");
            let (len, sha) = k6::sheet::file_view(&pt);
            assert_eq!((len, eravon_galeed::hash::hex_encode(&sha)), (f.bytes(), f.sha256().to_string()), "{}", f.name());
        }
        assert!(m.files.iter().any(|f| f.name().starts_with(a4::seam::kit_prefix())), "披露包那一件带包内路径");
        let ct = std::fs::read(out.join(format!("{MANIFEST_NAME}{SEALED_SUFFIX}"))).expect("清单密文");
        assert_eq!(galeed_evidence::decrypt(&judge, &ct), Some(manifest), "清单本身逐字节同");
        assert!(galeed_evidence::decrypt(&galeed::cryptox::AgeIdentity::generate(), &ct).is_none(), "别的钥解不开");
    }

    #[test]
    fn no_single_recipient_or_a_changed_file_writes_nothing() {
        let root = tmp("refuse");
        let (o, manifest) = sheet(&root);
        let judge = galeed::cryptox::AgeIdentity::generate();
        let cands = Pick::Candidates { reason: galeed_evidence::Reason::Gap, list: vec![], holes: vec![] };
        for (seats, name) in [
            (vec![cands], E_SEAL_RECIPIENT),
            (vec![Pick::None], E_SEAL_RECIPIENT),
            (vec![Pick::Unread], E_SEAL_RECIPIENT),
            (vec![], E_SEAL_RECIPIENT),
            (vec![one(&judge.recipient()), one(&galeed::cryptox::AgeIdentity::generate().recipient())], E_SEAL_SEATS),
        ] {
            let out = root.join(format!("out-{}", seats.len()));
            let e = export(&o, &manifest, &seats, &out.to_string_lossy()).expect_err("该拒");
            assert_eq!(e.0, name);
            assert!(!out.exists(), "{name}:一个字节也不写");
        }
        std::fs::write(root.join("drop/contract.pdf"), b"%PDF changed").expect("改原件");
        let out = root.join("out-changed");
        assert_eq!(export(&o, &manifest, &[one(&judge.recipient())], &out.to_string_lossy()).expect_err("该拒").0, E_SEAL_CHANGED);
        assert!(!out.exists());
    }

    fn tree(d: &Path) -> Vec<(String, Option<Vec<u8>>)> {
        let mut all = Vec::new();
        let mut stack = vec![d.to_path_buf()];
        while let Some(at) = stack.pop() {
            for e in std::fs::read_dir(&at).expect("读目录") {
                let p = e.expect("目录项").path();
                let rel = p.strip_prefix(d).expect("在下面").to_string_lossy().to_string();
                if p.is_dir() {
                    all.push((rel, None));
                    stack.push(p);
                } else {
                    all.push((rel, Some(std::fs::read(&p).expect("读"))));
                }
            }
        }
        all.sort();
        all
    }

    #[test]
    fn a_name_already_in_the_folder_refuses_the_whole_export_and_writes_nothing() {
        let root = tmp("exists");
        let (o, manifest) = sheet(&root);
        let judge = galeed::cryptox::AgeIdentity::generate();
        let out = root.join("out");
        put(&out.join("contract.pdf.age"), b"an earlier export");
        std::fs::create_dir_all(out.join(a4::seam::kit_prefix().trim_end_matches('/'))).expect("披露包那一层");
        let before = tree(&out);
        let e = export(&o, &manifest, &[one(&judge.recipient())], &out.to_string_lossy()).expect_err("同名已在该拒");
        assert_eq!(e.0, E_SEAL_EXISTS);
        assert!(e.1.contains("contract.pdf.age") && e.1.contains(a4::seam::kit_prefix().trim_end_matches('/')), "说出哪几个名已在:{}", e.1);
        assert_eq!(tree(&out), before, "一个字节不写、一个字节不覆盖");
        let only = root.join("only");
        std::fs::create_dir_all(only.join(format!("{MANIFEST_NAME}{SEALED_SUFFIX}"))).expect("同名目录");
        let before = tree(&only);
        assert_eq!(export(&o, &manifest, &[one(&judge.recipient())], &only.to_string_lossy()).expect_err("该拒").0, E_SEAL_EXISTS);
        assert_eq!(tree(&only), before);
    }

    #[test]
    fn placing_failing_midway_takes_back_what_was_placed() {
        let root = tmp("midway");
        let (o, manifest) = sheet(&root);
        let judge = galeed::cryptox::AgeIdentity::generate();
        let out = root.join("out");
        put(&out.join("unrelated.txt"), b"not ours");
        let before = tree(&out);
        for n in [1usize, 2] {
            FAIL_AFTER.with(|c| c.set(n));
            let e = export(&o, &manifest, &[one(&judge.recipient())], &out.to_string_lossy()).expect_err("就位途中该拒");
            FAIL_AFTER.with(|c| c.set(usize::MAX));
            assert_eq!(e.0, E_SEAL_WRITE);
            assert_eq!(tree(&out), before, "放下 {n} 个之后失败:放下的取回、暂存清掉,目标目录回到导出前");
        }
        let fresh = root.join("fresh").join("deeper");
        FAIL_AFTER.with(|c| c.set(1));
        assert_eq!(export(&o, &manifest, &[one(&judge.recipient())], &fresh.to_string_lossy()).expect_err("该拒").0, E_SEAL_WRITE);
        FAIL_AFTER.with(|c| c.set(usize::MAX));
        assert!(!root.join("fresh").exists(), "为这次导出新建的文件夹拿掉");
        let s = export(&o, &manifest, &[one(&judge.recipient())], &out.to_string_lossy()).expect("导出");
        assert_eq!(std::fs::read(out.join("unrelated.txt")).ok(), Some(b"not ours".to_vec()));
        assert!(s.files.iter().all(|p| std::path::Path::new(p).is_file()));
        assert!(!tree(&out).iter().any(|(n, _)| n.starts_with(eravon_store::layout::TMP_PREFIX)), "暂存不留");
    }

    #[test]
    fn a_chosen_path_that_is_a_file_is_refused_and_nothing_is_written() {
        let root = tmp("file");
        let (o, manifest) = sheet(&root);
        let judge = galeed::cryptox::AgeIdentity::generate();
        let at = root.join("out");
        put(&at, b"a plain file the party already had");
        let before = tree(&root);
        let e = export(&o, &manifest, &[one(&judge.recipient())], &at.to_string_lossy()).expect_err("一个档不是文件夹");
        assert_eq!(e.0, E_SEAL_WRITE);
        assert!(e.1.contains("不是文件夹"), "{}", e.1);
        assert_eq!(tree(&root), before, "那个档原样、旁边一个字节不写");
    }

    fn zikaron_fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-zikaron/tests/fixtures").join(name)
    }

    fn reader_folders() -> Vec<std::path::PathBuf> {
        let mut all: Vec<_> = std::fs::read_dir(std::env::temp_dir())
            .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(eravon_zikaron::pack::UNPACK_HEAD))).collect())
            .unwrap_or_default();
        all.sort();
        all
    }

    #[test]
    fn a_record_package_in_one_file_seals_the_same_names_and_bytes_as_its_folder() {
        let root = tmp("onefile");
        let folder = zikaron_fixture("zikaron-0.1.4-kit");
        let mut items = k6::listing::items_from_tree(&folder, a4::seam::kit_prefix(), "application/octet-stream").expect("披露包的清单项");
        items.sort_by(|a, b| a.name().as_bytes().cmp(b.name().as_bytes()));
        let me = format!("0x{}", "a1".repeat(20));
        let manifest = crate::evidence::sheet_for(items, String::new(), &me, &format!("0x{}", "c2".repeat(20)), &format!("0x{}", "d4".repeat(32)), 1, &format!("0x{}", "ee".repeat(20)), None).expect("清单").bytes;
        let judge = galeed::cryptox::AgeIdentity::generate();
        let before = reader_folders();
        let mut names = Vec::new();
        for (form, kit) in [("one", zikaron_fixture("zikaron-0.1.4-kit.zkit")), ("folder", folder.clone())] {
            let out = root.join(format!("out-{form}"));
            let o = Origins { kit_root: Some(kit.to_string_lossy().to_string()), drop: None };
            export(&o, &manifest, &[one(&judge.recipient())], &out.to_string_lossy()).expect("导出");
            names.push(tree(&out).into_iter().map(|(n, _)| n).collect::<Vec<_>>());
            let m = k6::sheet::check(&manifest).expect("清单读回");
            for f in &m.files {
                let ct = std::fs::read(out.join(format!("{}{SEALED_SUFFIX}", f.name()))).expect("密文");
                let rest = f.name().strip_prefix(a4::seam::kit_prefix()).expect("披露包那一件带包内路径");
                assert_eq!(galeed_evidence::decrypt(&judge, &ct), std::fs::read(folder.join(rest)).ok(), "{form}:{} 解回目录形原件的字节", f.name());
            }
            let ct = std::fs::read(out.join(format!("{MANIFEST_NAME}{SEALED_SUFFIX}"))).expect("清单密文");
            assert_eq!(galeed_evidence::decrypt(&judge, &ct), Some(manifest.clone()), "{form}:清单本身逐字节同");
        }
        assert_eq!(names[0], names[1], "单档与目录:落下同一组名");
        assert_eq!(names[0].iter().filter(|n| n.ends_with(SEALED_SUFFIX)).count(), 7, "六件原件与清单本身:{:#?}", names[0]);
        assert_eq!(reader_folders(), before, "读出单档的那个文件夹事后不留");
    }
}
