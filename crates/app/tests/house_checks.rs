use std::path::{Path, PathBuf};

fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read_src_file(name: &str) -> std::io::Result<String> {
    let p = crates().join(name);
    if p.is_file() {
        return std::fs::read_to_string(&p);
    }
    let dir = p.with_extension("");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?.flatten().map(|e| e.path()).filter(|x| x.extension().and_then(|e| e.to_str()) == Some("rs")).collect();
    files.sort_by_key(|f| (f.file_name().and_then(|n| n.to_str()) != Some("mod.rs"), f.clone()));
    let mut out = String::new();
    for f in files {
        out.push_str(&std::fs::read_to_string(&f)?);
        out.push('\n');
    }
    Ok(out)
}

fn code_only(s: &str) -> String {
    let body = s.split("\n#[cfg(test)]").next().unwrap_or(s);
    body.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

fn shipped() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    for c in std::fs::read_dir(crates()).expect("读得出 crates/").flatten() {
        walk(&c.path().join("src"), &mut files);
    }
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            (p.strip_prefix(crates()).unwrap_or(&p).display().to_string().replace('\\', "/"), code_only(&text))
        })
        .collect()
}

fn body_of<'a>(code: &'a str, head: &str) -> Option<&'a str> {
    let start = code.find(head)?;
    let end = code[start..].find("\n}").map(|e| start + e + 2)?;
    Some(&code[start..end])
}

fn callers(code: &str, call: &str) -> Vec<String> {
    let lines: Vec<&str> = code.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        let Some(rest) = ["fn ", "pub fn ", "pub(crate) fn ", "pub(super) fn "].iter().find_map(|h| t.strip_prefix(h)) else { continue };
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let close = format!("{}}}", &l[..l.len() - t.len()]);
        let body: Vec<&str> = lines[i + 1..].iter().take_while(|x| **x != close).copied().collect();
        if body.iter().any(|x| x.contains(call)) && !out.contains(&name) {
            out.push(name);
        }
    }
    out.sort();
    out
}

fn app_sources() -> Vec<(String, String)> {
    shipped().into_iter().filter_map(|(n, c)| n.strip_prefix("app/src/").map(|x| (x.to_string(), c))).collect()
}

fn method_of<'a>(code: &'a str, head: &str) -> Option<&'a str> {
    let start = code.find(head)?;
    let end = code[start..].find("\n    }\n").map(|e| start + e + 6)?;
    Some(&code[start..end])
}

fn plain_copies(code: &str) -> usize {
    code.matches(".expose().to_string()").count() + code.matches(".expose().to_owned()").count()
}

#[test]
fn the_places_are_set_only_where_the_data_root_is_bound() {
    let mut found: Vec<(String, usize)> = shipped().into_iter().map(|(n, c)| (n, c.matches("places::set(").count())).filter(|(_, k)| *k > 0).collect();
    found.sort();
    assert_eq!(found, vec![("eravon-vault/src/records.rs".to_string(), 1), ("eravon-vault/src/testing.rs".to_string(), 1)], "places are set where the data root is bound, and in the test support");
    let records = code_only(&read_src_file("eravon-vault/src/records.rs").expect("读得出"));
    assert!(body_of(&records, "pub fn bind(").is_some_and(|b| b.contains("places::set(")), "the one shipped call is `bind`'s");
    let lib = read_src_file("eravon-vault/src/lib.rs").expect("读得出");
    assert!(lib.contains("#[cfg(any(test, feature = \"testing\"))]\npub mod testing;"), "the test support is behind its own feature");
}

#[test]
fn every_secret_field_is_the_secret_type() {
    assert_eq!(plain_copies("let a = p.expose().to_string(); let b = q.expose().to_owned();"), 2, "the counter sees both forms");
    let listed: [(&str, usize, &str); 4] = [
        ("house/drive/cli.rs", 1, "the program key the drive door made, in that run's answer once"),
        ("proxycells.rs", 1, "the transport's secret to the cells' secret"),
        ("window/drive.rs", 1, "the wizard-back probe's reading"),
        ("window/settings.rs", 1, "the program key shown this once on its sheet"),
    ];
    let mut found: Vec<(String, usize)> = app_sources().into_iter().map(|(n, c)| (n, plain_copies(&c))).filter(|(_, k)| *k > 0).collect();
    found.sort();
    let want: Vec<(String, usize)> = listed.iter().map(|(n, k, _)| (n.to_string(), *k)).collect();
    assert_eq!(found, want, "a secret's text copied into a plain String outside the listed places");
    let paylink = code_only(&read_src_file("app/src/house/paylink.rs").expect("读得出"));
    let import = body_of(&paylink, "pub struct Import {").expect("Import");
    assert!(!import.contains("file_pass") && !import.contains("cells"), "付款链接导入页只收文件,没有口令与十二格");
    assert!(method_of("    pub fn a() {\n        x();\n    }\n    pub fn b() {\n        y();\n    }\n", "    pub fn a(").is_some_and(|b| !b.contains("y()")), "a method's body ends at its own brace");
}

#[test]
fn no_first_layer_takes_a_lower_layers_sentence() {
    let bare = |code: &str| ["fail(e.detail())", "head: e.detail()", "next: e.detail()"].iter().map(|p| code.matches(p).count()).sum::<usize>();
    assert_eq!(bare("self.fail(e.detail()); Toast { head: e.detail(), next: e.detail() }"), 3, "the counter sees all three");
    let found: Vec<String> = app_sources().into_iter().filter(|(n, c)| (n.starts_with("window/house/") || n.starts_with("house/exec/") || n == "house/first.rs") && bare(c) > 0).map(|(n, _)| n).collect();
    assert!(found.is_empty(), "a first layer takes a lower layer's sentence: {found:?}");
}

#[test]
fn every_lock_clears_the_words_being_viewed() {
    let shell = code_only(&read_src_file("app/src/shell.rs").expect("读得出"));
    for head in ["pub fn begin_lock(", "pub fn after_lock("] {
        assert!(method_of(&shell, &format!("    {head}")).is_some_and(|b| b.contains("self.words = None;")), "{head} 抹正在看的恢复词");
    }
    let own: Vec<String> = app_sources().into_iter().filter(|(n, c)| n.starts_with("house/") && (c.contains("gate.lock(") || c.contains(" g.lock("))).map(|(n, _)| n).collect();
    assert!(own.is_empty(), "本家自己锁门:{own:?}");
}

#[test]
fn the_word_checks_and_the_word_display_lend_and_never_copy() {
    let copies = |body: &str| [".trim().to_ascii_lowercase()", "to_string()", ".join(", "vec![", "String::"].iter().filter(|p| body.contains(*p)).count();
    assert_eq!(copies("let w = c.trim().to_ascii_lowercase(); vec![w; 12].join(\" \")"), 3, "the check sees copies");
    let words = code_only(&read_src_file("eravon-vault/src/words.rs").expect("读得出"));
    for head in ["pub fn strangers<", "fn stranger("] {
        let body = body_of(&words, head).expect(head);
        assert_eq!(copies(body), 0, "{head} 不在堆上复制词:{body}");
    }
    assert!(body_of(&words, "fn stranger(").is_some_and(|b| b.contains("eravon_os::wipe(&mut buf)")), "问过即抹零");
    let pin = code_only(&read_src_file("eravon-ui/src/pin.rs").expect("读得出"));
    assert!(pin.contains("pub fn mask(ui: &mut egui::Ui, id_salt: &str, words: Option<&[crate::secret::Secret]>,"), "遮罩收秘密型");
}

#[test]
fn the_kit_is_the_familys_files_and_the_additions_on_the_table() {
    use eravon_literals::json::Raw;
    const KIT: &str = "crates/eravon-ui/src/";
    let root = crates().join("..");
    let read = |p: &str| eravon_literals::json::read_raw(&std::fs::read(root.join(p)).expect(p)).expect(p);
    let rename = read("tools/family/family-rename.json");
    let Some(Raw::Arr(files)) = rename.get("files") else { panic!("files") };
    let mut rows = Vec::new();
    for r in files {
        let Some(home) = r.get("home").and_then(Raw::as_str).filter(|h| h.starts_with(KIT)) else { continue };
        assert!(root.join(home).is_file(), "{home}: a ui-kit row is a file in the kit");
        assert!(matches!(r.get("from"), Some(Raw::Str(_))), "{home}: a ui-kit row names the family file it carries");
        rows.push(home.to_string());
    }
    assert_eq!(rows.len(), 52, "the family's kit files, one row each");
}

#[test]
fn the_two_languages_carry_the_same_key_set() {
    let bad = app::lang::trouble();
    assert!(bad.is_empty(), "键表不齐:{bad:?}");
    assert_eq!(app::lang::TABLE.len(), app::lang::Key::ALL.len());
    assert_eq!(app::lang::Lang::ALL.len(), 2);
    for l in app::lang::Lang::ALL {
        app::lang::set(l);
        assert_eq!(app::lang::lang(), l);
        for k in app::lang::Key::ALL {
            assert!(!app::lang::t(k).is_empty(), "{k:?} 在 {} 下是空的", l.as_str());
        }
    }
    app::lang::set(app::lang::Lang::Zh);
}



const CLICKABLE: [&str; 22] = [
    "key::key(",
    "key::Key::new(",
    "key::link(",
    ".press_long_saying(ui,",
    "page::Guide::key(",
    "page::Guide::last(",
    ".press(ui,",
    ".primary_with(ui,",
    "menu::row(",
    "f.row(ui,",
    "f.danger(ui,",
    "f.nav_big(ui,",
    "drop::file_row(",
    "self.long_key(ui,",
    "pick::key(",
    "path_row(ui,",
    "danger_zone(ui,",
    ".press_saying(ui,",
    ".press_long(ui,",
    "foot_item(ui,",
    "choice_line(ui,",
    "toggle::toggle(ui,",
];

fn drawn_ellipses(name: &str, code: &str) -> Vec<String> {
    let words: std::collections::BTreeMap<String, (&str, &str)> = app::lang::TABLE.iter().map(|(k, zh, en)| (format!("{k:?}"), (*zh, *en))).collect();
    let dotted = |s: &str| s.contains('…') || s.contains("\\u{2026}") || s.ends_with("...");
    let mut out = Vec::new();
    for pat in CLICKABLE {
        let mut from = 0;
        while let Some(at) = code[from..].find(pat) {
            let start = from + at + pat.len();
            let mut depth = 1usize;
            let mut end = start;
            for (i, ch) in code[start..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let args = &code[start..end.max(start)];
            let args = if pat.contains("_saying(") { args.split(',').next().unwrap_or(args) } else { args };
            let line = code[..start].matches('\n').count() + 1;
            if dotted(args) {
                out.push(format!("{name}:{line}: {pat} {}", args.trim()));
            }
            for head in ["Key::", "K::"] {
                let mut rest = args;
                while let Some(k) = rest.find(head) {
                    let tail = &rest[k + head.len()..];
                    let id: String = tail.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                    if let Some((zh, en)) = words.get(&id).filter(|(zh, en)| dotted(zh) || dotted(en)) {
                        out.push(format!("{name}:{line}: {pat} {head}{id} ({zh} / {en})"));
                    }
                    rest = &tail[id.len()..];
                }
            }
            from = start;
        }
    }
    out
}

#[test]
fn no_drawn_button_word_carries_an_ellipsis() {
    let planted = "    import = key::key(ui, &format!(\"{}\\u{2026}\", t(K::TextImport)), Role::Plain, true).clicked();\n";
    assert_eq!(drawn_ellipses("planted", planted).len(), 1, "the counter-example: an ellipsis put on when drawing");
    let dotted_key = app::lang::TABLE.iter().find(|(_, zh, en)| zh.contains('…') || en.contains('…')).map(|(k, _, _)| format!("{k:?}")).expect("a key with an ellipsis, as a status word has");
    assert_eq!(drawn_ellipses("planted", &format!("    danger_zone(ui, &t(K::{dotted_key}));\n")).len(), 1, "the counter-example: a key whose words carry one");
    assert!(drawn_ellipses("planted", "    import = key::key(ui, &t(K::TextImport), Role::Plain, true).clicked();\n").is_empty());
    let lines_kept = |n: &str| {
        let text = std::fs::read_to_string(crates().join(n)).unwrap_or_default();
        let body = text.split("\n#[cfg(test)]").next().unwrap_or(&text).to_string();
        body.lines().map(|l| if l.trim_start().starts_with("//") { "" } else { l }).collect::<Vec<_>>().join("\n")
    };
    let bad: Vec<String> = shipped()
        .into_iter()
        .filter(|(n, _)| n.starts_with("app/src/") || n.starts_with("eravon-ui/src/"))
        .flat_map(|(n, _)| drawn_ellipses(&n, &lines_kept(&n)))
        .collect();
    assert!(bad.is_empty(), "按钮画出来的字带尾号:\n{}", bad.join("\n"));
}

fn call_sites(call: &str) -> Vec<String> {
    let mut out: Vec<String> = shipped().into_iter().flat_map(|(n, c)| callers(&c, call).into_iter().map(move |f| format!("{n}::{f}"))).collect();
    out.sort();
    out
}

#[test]
fn every_signer_signs_the_order_its_collision_question_predicts() {
    let planted = "    pub fn order_sign(c: &Ctx) {\n        let key = c.key(&who, pass)?;\n        let sd = sign_order(&SignIn { key: &key, buyer: &who });\n    }\n";
    assert_eq!(callers(planted, "sign_order("), vec!["order_sign".to_string()], "the counter-example: a signer is seen");
    assert!(!callers(planted, "key_pick(").contains(&"order_sign".to_string()), "the counter-example: and seen not asking");
    assert_eq!(
        call_sites("sign_order("),
        vec!["eravon-cli/src/buyer/order.rs::order_sign".to_string(), "eravon-pay/src/pay.rs::pay".to_string()],
        "the two places that sign an order"
    );
    assert_eq!(
        call_sites("key_pick("),
        vec!["app/src/house/paylink/import.rs::bump".to_string(), "eravon-cli/src/buyer/order.rs::order_sign".to_string(), "eravon-pay/src/pay.rs::pay".to_string()],
        "both signers and the page's question share one derivation"
    );
    assert_eq!(
        call_sites("signing_key("),
        vec!["eravon-cli/src/buyer/order.rs::order_sign".to_string(), "eravon-pay/src/pay.rs::pay".to_string()],
        "both signers take the key from one place"
    );
    assert!(call_sites("bump::pick(").is_empty() && !crates().join("eravon-casekeys").exists(), "no per-case key: the pick is this seat's identity key or refused");
    assert!(call_sites("pick_on_chain(").is_empty() && !shipped().iter().any(|(_, c)| c.contains("fn pick_on_chain(")), "no second road that reads its own endpoints");
    let order = code_only(&read_src_file("eravon-cli/src/buyer/order.rs").expect("读得出"));
    let sign = body_of(&order, "pub(super) fn order_sign(").expect("order_sign");
    assert!(sign.contains("buyer: &pick.address") && sign.contains("order_of(p.terms_text.as_bytes(), candidate, &pick.address,") && sign.contains("pay_gate(&sw, &o)"), "the command line checks the picked key's order before it signs it");
    let import = code_only(&read_src_file("app/src/house/paylink/import.rs").expect("读得出"));
    let bump = method_of(&import, "    pub fn bump(").expect("PayLink::bump");
    assert!(bump.contains("eravon_cli::buyer::key_pick(s, buyer, &w, &p, n, &self.payout, open_by, Some(now), &prev)"), "the page asks with its own refund address, the current key and the second it signs at");
    assert!(!bump.contains("order_of("), "the page computes no order of its own");
}

#[test]
fn a_candidates_choosability_is_decided_in_one_place() {
    let sets = |code: &str| {
        code.matches("choosable: ").count() - code.matches("choosable: bool").count() - code.matches("let mut choosable: ").count() - code.matches("let choosable: ").count()
            + code.matches(".choosable = ").count()
    };
    assert_eq!(sets("pub choosable: bool,\nlist.push(Cand { choosable: ok_gate && chain_note.is_none() });\nfor x in o.cands.iter_mut() { x.choosable = false; }"), 2, "the counter-example: a second setter is seen");
    let setters: Vec<(String, usize)> = shipped().into_iter().filter(|(n, _)| n.starts_with("app/src/") || n.starts_with("eravon-cli/src/")).map(|(n, c)| (n, sets(&c))).filter(|(_, k)| *k > 0).collect();
    assert_eq!(setters, vec![("eravon-cli/src/deskops/paypage.rs".to_string(), 1)], "the row sets choosable once");
    let paypage = code_only(&read_src_file("eravon-cli/src/deskops/paypage.rs").expect("读得出"));
    assert!(body_of(&paypage, "pub fn page(").expect("paypage::page").contains("choosable: ok_gate && chain_note.is_none(),"), "from the offline gate and the chain notes");
    assert_eq!(call_sites("(h.candidates)("), vec!["eravon-cli/src/deskops/paypage.rs::verify_listing".to_string()], "the page reads each candidate once");
    assert!(!body_of(&paypage, "pub fn verify_listing(").expect("paypage::verify_listing").contains("quote_bytes_of("), "and does not read a candidate a second time");
    let verify = code_only(&read_src_file("app/src/house/paylink/verify.rs").expect("读得出"));
    assert!(verify.contains("candidates: eravon_pay::check::candidates_on,"), "the window's reading is the shared one");
    let cands = body_of(&paypage, "pub fn candidates(").expect("paypage::candidates");
    assert!(cands.contains("quote_bytes_of(&held.0.chain(), &held.1, n)") && !cands.contains("judge::open") && !cands.contains("guild::open"), "the candidate reading is the act's seat, nothing compared a second time");
    let spelled: Vec<String> = shipped().into_iter().filter(|(n, c)| n != "app/src/house/refusal.rs" && c.contains("\"E_ACT_JUDGE_AMANAH\"")).map(|(n, _)| n).collect();
    assert_eq!(spelled, vec!["eravon-cands/src/act.rs".to_string()], "the judge's law word is compared in the act alone");
    let act = code_only(&read_src_file("eravon-cands/src/act.rs").expect("读得出"));
    assert_eq!(callers(&act, "offered("), vec!["named".to_string()], "the seat's named judge asks it");
}

#[test]
fn a_link_to_the_other_seat_never_waits_on_the_switch() {
    let settles = |code: &str| code.contains("apply_settled") || code.contains("family_said_landed");
    assert!(settles("crate::action::apply_settled(self, crate::action::Action::SwitchRole)"), "the counter sees a settling call");
    let links = code_only(&read_src_file("app/src/house/links.rs").expect("links.rs"));
    let take = method_of(&links, "    pub(crate) fn take_url(").expect("take_url");
    assert!(!settles(take), "take_url waits on the switch:\n{take}");
    assert!(take.contains("family_said_started") && take.contains("goto_landed"), "take_url starts the switch and leaves the page for its landing:\n{take}");
    let flow = code_only(&read_src_file("app/src/house/flow.rs").expect("flow.rs"));
    assert!(method_of(&flow, "    pub(crate) fn seat_follow(").expect("seat_follow").contains("goto_landed.take()"), "the landing takes the page");
    let office = code_only(&read_src_file("app/src/action/office.rs").expect("office.rs"));
    let landed = body_of(&office, "pub(super) fn seat_landed(").expect("seat_landed");
    assert_eq!(landed.matches("goto_landed = None").count(), 2, "both failed landings drop the page:\n{landed}");
}

#[test]
fn a_rotations_card_goes_back_only_to_its_new_key() {
    let landed = code_only(&read_src_file("app/src/house/landed.rs").expect("landed.rs"));
    let at = landed.find("follow_register(sh)").expect("the landing follows the register");
    let put = landed[at..].find("x.rotate = Some(r)").map(|e| at + e).expect("and puts the run back");
    assert!(landed[at..put].contains("same_row(&sh.desk.data, &v.new, &sh.desk.addr)"), "only on the identity it belongs to:\n{}", &landed[at..put]);
    assert!(!landed.contains("desk.rotated"), "no run carried across a later landing");
}

#[test]
fn a_scan_opens_the_open_row_again_by_its_instance() {
    let list = code_only(&read_src_file("app/src/house/orders/list.rs").expect("list.rs"));
    let scan = method_of(&list, "    pub fn scan(").expect("scan");
    assert!(scan.contains("self.reopen(") && !scan.contains("self.order.as_ref().map(|x| x.id.clone())"), "the scan reopens by row:\n{scan}");
    let reopen = method_of(&list, "    pub(crate) fn reopen(").expect("reopen");
    assert!(reopen.contains("self.row()") && reopen.contains("r.record") && reopen.contains("open_one("), "by the row's record:\n{reopen}");
}

#[test]
fn every_write_to_a_case_passes_the_past_contract_gate_first() {
    let first = |body: &str, gates: &[&str], acts: &[&str]| -> Result<(), String> {
        let gate = gates.iter().filter_map(|g| body.find(g)).min().ok_or("no gate")?;
        match acts.iter().filter_map(|a| body.find(a)).min() {
            Some(at) if at < gate => Err("a write before the gate".into()),
            _ => Ok(()),
        }
    };
    assert!(first("sign(k); past_gate(c)?;", &["past_gate(c)"], &["sign("]).is_err(), "the counter sees a late gate");
    let act = code_only(&read_src_file("eravon-after/src/act.rs").expect("act.rs"));
    let gates = ["past_gate(c)", "guard(c)", "guard_any(c)"];
    let writes = ["post(", "sign(", "to_wallet(", "record_confirm_in(", "as_buyer(", "as_buyer_key("];
    for f in ["pub fn confirm(", "pub fn confirm_hand_out(", "pub fn dispute(", "pub fn dispute_pull(", "pub fn commit(", "pub fn advance(", "pub fn settle(", "pub fn cancel(", "pub fn expire(", "pub fn sweep("] {
        let body = body_of(&act, f).unwrap_or_else(|| panic!("{f}"));
        assert_eq!(first(body, &gates, &writes), Ok(()), "{f}:\n{body}");
    }
    let gate = body_of(&act, "fn past_gate(").expect("past_gate");
    assert!(gate.contains("past_gate(c.past())"), "{gate}");
    for g in ["fn guard(", "fn guard_any("] {
        assert!(body_of(&act, g).is_some_and(|b| b.contains("past_gate(c)")), "{g} asks the past gate");
    }
    let chain = code_only(&read_src_file("eravon-standard/src/chain.rs").expect("chain.rs"));
    assert!(body_of(&chain, "pub fn code_matches<").is_some_and(|b| b.contains("writable(judge(")), "the code gate judges past rows");
    let orders = code_only(&read_src_file("app/src/house/orders.rs").expect("orders.rs"));
    let plan = method_of(&orders, "    pub fn plan(").expect("Orders::plan");
    assert!(plan.contains("past_gate(r.past)"), "{plan}");
    let bo = code_only(&read_src_file("app/src/house/buyer_orders/acts.rs").expect("acts.rs"));
    let bplan = method_of(&bo, "    pub fn plan(").expect("BOrders::plan");
    assert!(bplan.contains("past_gate(r.row.past)"), "{bplan}");
    let watch = code_only(&read_src_file("eravon-cli/src/seller/watch.rs").expect("watch.rs"));
    for f in ["pub fn expire(", "pub fn deploy("] {
        let body = body_of(&watch, f).unwrap_or_else(|| panic!("{f}"));
        assert!(body.contains("h.writable()") || body.contains("made_past("), "{f}:\n{body}");
    }
    let case = code_only(&read_src_file("eravon-cli/src/buyer/case.rs").expect("case.rs"));
    assert!(body_of(&case, "pub(super) fn cancel(").is_some_and(|b| b.contains("h.writable()")), "cancel");
    let bact = code_only(&read_src_file("eravon-cli/src/buyer/actions.rs").expect("actions.rs"));
    assert_eq!(first(body_of(&bact, "pub(super) fn settle_sign(").expect("settle_sign"), &["past_at("], &["shared::sign("]), Ok(()));
    let sact = code_only(&read_src_file("eravon-cli/src/seller/actions.rs").expect("actions.rs"));
    assert_eq!(first(body_of(&sact, "pub(super) fn settle_offer(").expect("settle_offer"), &["past_gate(g.is_past())", "code_matches("], &["offer(&sign", "post_offer(", "company::settle("]), Ok(()));
}

#[test]
fn the_wizard_seat_step_goes_to_its_seat_and_never_waits_on_the_switch() {
    let turns = |code: &str| code.contains("apply_settled") || code.contains("Action::SwitchRole");
    assert!(turns("crate::action::apply_settled(sh, crate::action::Action::SwitchRole)"), "the counter sees a turning call");
    let first = code_only(&read_src_file("app/src/house/exec/first.rs").expect("first.rs"));
    let role = body_of(&first, "fn role(").expect("role");
    assert!(!turns(role), "the wizard's seat step turns the seat round or waits on it:\n{role}");
    let begun = role.find("Applied::Started(Kind::Seat)").expect("the step answers a begun switch by its kind");
    let wish = role.find("role_landed = Some(").expect("the step leaves the laying out to the landing");
    assert!(role.contains("switch_to(") && begun < wish, "a wish is kept only for a switch begun:\n{role}");
    assert!(role.contains("换席没成"), "anything else is said as a switch that did not happen:\n{role}");
    let keys = code_only(&read_src_file("app/src/action/keys.rs").expect("keys.rs"));
    let to = body_of(&keys, "pub fn switch_to(").expect("switch_to");
    assert!(to.contains("seat_wish = Some(want)") && to.contains("seat_start(shell, want)") && !to.contains(".other()"), "the switch goes to the seat named:\n{to}");
    let office = code_only(&read_src_file("app/src/action/office.rs").expect("office.rs"));
    let landed = body_of(&office, "pub(super) fn seat_landed(").expect("seat_landed");
    assert_eq!(landed.matches("role_landed = None").count(), 2, "both failed landings drop the step's wish:\n{landed}");
    let flow = code_only(&read_src_file("app/src/house/flow.rs").expect("flow.rs"));
    let follow = method_of(&flow, "    pub(crate) fn seat_follow(").expect("seat_follow");
    assert!(follow.contains("first::role_landed(self)"), "the landing lays the seat out:\n{follow}");
    let ask = follow.find("Leave::Seat(seat)").expect("the leave question");
    let back = follow[ask..].find("return;").map(|e| ask + e).expect("the question's early return");
    assert!(follow[ask..back].contains("role_landed = None"), "a leave question at the landing drops the step's wish:\n{follow}");
}

#[test]
fn a_case_is_rebuilt_under_the_row_one_place_picks() {
    let own_pick = |body: &str| body.contains("generation_of_logic") || body.contains("past_rows()") || body.contains("made_under");
    assert!(own_pick("let l = made_under(&x, w.logic); rebuild(t, l)"), "the counter sees a caller's own pick");
    let scan = code_only(&read_src_file("eravon-scan/src/scan.rs").expect("scan.rs"));
    let entry = body_of(&scan, "pub fn rebuild<").expect("rebuild");
    assert!(entry.contains("rebuild_among(") && entry.contains("past_rows()"), "{entry}");
    let among = body_of(&scan, "pub fn rebuild_among<").expect("rebuild_among");
    assert!(among.contains("row_of(") && among.contains("traced_init("), "{among}");
    for (file, head) in [
        ("eravon-watch/src/scan.rs", "fn enroll_one("),
        ("eravon-after/src/act.rs", "fn lapse_split("),
        ("eravon-cli/src/buyer/order.rs", "pub(super) fn import("),
    ] {
        let code = code_only(&read_src_file(file).expect(file));
        let body = body_of(&code, head).unwrap_or_else(|| panic!("{file} {head}"));
        assert!(body.contains("scan::rebuild(") && body.contains("w.logic"), "{file} {head} rebuilds through the one pick");
        assert!(!own_pick(body), "{file} {head} picks no row of its own");
    }
}

#[test]
fn the_sweep_keeps_nothing_before_the_code_gate() {
    let sweep = code_only(&read_src_file("eravon-dispute/src/sweep.rs").expect("sweep.rs"));
    let body = body_of(&sweep, "pub fn due(").expect("due");
    let unregistered = body.find("if !registered(").expect("the unregistered branch");
    let branch = &body[unregistered..];
    let note = branch.find("order::save(").expect("the note is kept");
    let gate = branch.find("code_matches(").expect("the branch's code gate");
    assert!(gate < note, "the code gate before the note:\n{branch}");
    let nothing = body.find("Swept::Nothing(format!(\"判官").expect("nothing owed");
    let before_nothing = &body[unregistered..nothing];
    assert_eq!(before_nothing.matches("code_matches(").count(), 1, "only the unregistered branch reads the code before the nothing-owed answer");
    let post = body.find("Due::Call(").expect("the push");
    assert!(body[nothing..post].contains("code_matches("), "the push is gated");
    for head in ["pub fn judge(", "pub fn company("] {
        assert!(body_of(&sweep, head).unwrap_or_else(|| panic!("{head}")).contains("due("), "{head} takes the one set of checks");
    }
}

#[test]
fn a_goods_text_is_drawn_only_from_its_read_body() {
    let raw = |args: &str| !args.contains("read.body");
    assert!(raw("text_whole(ui, t)"), "the counter sees a raw text");
    let mut calls = 0;
    for (name, code) in app_sources().into_iter().filter(|(n, _)| n.starts_with("window/")) {
        for (i, _) in code.match_indices("text_whole(ui,") {
            let line = &code[code[..i].rfind('\n').map_or(0, |n| n + 1)..i];
            if !line.contains("PlSegGoods") {
                continue;
            }
            let args = &code[i..code[i..].find(")").map_or(code.len(), |e| i + e)];
            calls += 1;
            assert!(!raw(args), "{name}: a goods text drawn raw: {args}");
        }
    }
    assert!(calls >= 2, "the scan reached the text pages ({calls} calls)");
}

#[test]
fn a_home_reading_stays_until_a_new_one_lands() {
    let clears = |code: &str| code.contains("seller.home = None");
    assert!(clears("self.desk.sh.seller.home = None;"), "the counter sees a clear");
    for (name, code) in app_sources() {
        assert!(!clears(&code), "{name} clears the home reading before reading again");
    }
    let landed = code_only(&read_src_file("app/src/house/landed.rs").expect("landed.rs"));
    let at = landed.find("Kind::BHome").expect("the buyer home landing");
    let arm = &landed[at..at + landed[at..].find("whole = Some(crate::house::buyer_home::BHome").expect("the failure reading")];
    assert!(arm.contains("eq_ignore_ascii_case(&me)") && arm.contains("return;"), "a failed buyer home read keeps this key's reading:\n{arm}");
}

fn seller_right_over_history(src: &str) -> bool {
    let lines: Vec<&str> = src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("//")).collect();
    lines.iter().position(|l| l.contains("Key::PlHistory), \"\"")).is_some_and(|i| i > 0 && lines[i - 1].contains("Key::IdSeatAuthor"))
}

#[test]
fn a_cards_history_row_sits_under_the_party_it_belongs_to() {
    let planted = "f.info(ui, &t(Key::PlTitle), x);\n// the seller's history\nif f.row(ui, &t(Key::PlHistory), \"\", y) {";
    assert!(!seller_right_over_history(planted), "the counter-example: a history row under another row is seen");
    let good = "f.info(ui, &t(Key::IdSeatAuthor), x);\n// the seller's history\nif f.row(ui, &t(Key::PlHistory), \"\", y) {";
    assert!(seller_right_over_history(good), "and the right order is seen");
    for page in ["app/src/window/house/buyer_orders.rs", "app/src/window/house/paylink.rs"] {
        let src = read_src_file(page).unwrap_or_else(|e| panic!("{page}: {e}"));
        assert!(seller_right_over_history(&src), "{page}:「全部历史」上方不是卖家那一行");
    }
    let table = read_src_file("app/src/window/house/buyer_orders.rs").unwrap_or_default();
    assert!(!table.contains("table::Cell::Text(e.name.clone())"), "买家订单表一格画了拒名");
}

#[test]
fn the_about_icon_is_the_master_made_smaller_byte_for_byte() {
    let png = std::fs::read(crates().join("../packaging/icon/eravon.png")).expect("the icon master");
    let (side, master) = eravon_ui::page_marks::master_rgba(&png).expect("the master reads");
    assert_eq!(side, eravon_ui::page_marks::ICON_MASTER_SIDE);
    let made = eravon_ui::page_marks::icon_at(&master, side, 256);
    assert_eq!(app::about::ICON.len(), made.len(), "256 x 256 RGBA");
    assert!(app::about::ICON == made.as_slice(), "assets/icon-256.rgba is the master made smaller (make it again: release icon)");
    for (x, y) in [(0, 0), (255, 0), (0, 255), (255, 255)] {
        assert_eq!(made[(y * 256 + x) * 4 + 3], 0, "the corner ({x}, {y}) is transparent");
    }
}
