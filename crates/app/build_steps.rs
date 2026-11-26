use std::fmt::Write as _;

pub fn write() {
    println!("cargo:rerun-if-changed=src/house/seam.rs");
    let src = std::fs::read_to_string("src/house/seam.rs").expect("读得到 src/house/seam.rs");
    let mut out = String::new();
    for (prefix, name) in [("STEP_", "STEPS"), ("PAGE_", "PAGES")] {
        let head = format!("pub const {prefix}");
        let mut vals: Vec<String> = Vec::new();
        let mut seen = 0usize;
        for line in src.lines() {
            let t = line.trim_start();
            if !t.starts_with(&head) {
                continue;
            }
            seen += 1;
            let v = t
                .split_once(": &str = \"")
                .and_then(|(_, rest)| rest.strip_suffix("\";"))
                .unwrap_or_else(|| panic!("seam.rs 里这一枚读不出值(要单行 `pub const X: &str = \"…\";`):{t}"));
            if !vals.iter().any(|x| x == v) {
                vals.push(v.to_string());
            }
        }
        assert!(seen > 0, "seam.rs 里一枚 {prefix} 常量也没有");
        vals.sort();
        let items: Vec<String> = vals.iter().map(|v| format!("{v:?}")).collect();
        let _ = writeln!(out, "/// 由 build.rs 从本档的 `{prefix}*` 常量现生成(去重、按字节序)。");
        let _ = writeln!(out, "pub const {name}: &[&str] = &[{}];", items.join(", "));
    }
    let dir = std::env::var("OUT_DIR").expect("有 OUT_DIR");
    std::fs::write(format!("{dir}/steps.rs"), out).expect("写得进 steps.rs");
}
