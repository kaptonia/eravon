use std::path::Path;

fn files() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for p in [Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("mdedit.rs")] {
        out.push((p.file_name().unwrap().to_string_lossy().to_string(), std::fs::read_to_string(&p).expect("a source file reads")));
    }
    out.sort();
    out
}

fn code_only(s: &str) -> String {
    s.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
}

fn enclosing_call(code: &str, at: usize) -> String {
    let b = code.as_bytes();
    let mut depth = 0i32;
    let mut i = at;
    while i > 0 {
        i -= 1;
        match b[i] {
            b')' => depth += 1,
            b'(' if depth == 0 => {
                let head = &code[..i];
                let start = head.rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':')).map(|k| k + 1).unwrap_or(0);
                return head[start..].to_string();
            }
            b'(' => depth -= 1,
            _ => {}
        }
    }
    String::new()
}

fn enclosing_fn(code: &str, at: usize) -> String {
    let head = &code[..at];
    let Some(k) = head.rfind("fn ") else { return String::new() };
    head[k + 3..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect()
}

#[test]
fn every_scroll_area_of_the_own_widgets_goes_in_through_scroll() {
    let mut seen = 0;
    for (name, text) in files() {
        let code = code_only(&text);
        for (at, _) in code.match_indices("ScrollArea::") {
            let call = enclosing_call(&code, at);
            let editor = call == "scrolled" && {
                let body = code.split("fn scrolled<").nth(1).map(|b| b.split("\nfn ").next().unwrap_or(b)).unwrap_or("");
                body.contains("crate::scroll::nested(") && body.contains("mouse_wheel: false")
            };
            assert!(editor || ["scroll::page", "scroll::nested", "scroll::nested_rows"].iter().any(|w| call.ends_with(w)), "own/{name}: a scroll area shown by `{call}`, not through `scroll` (nor the editor's focus split)");
            seen += 1;
        }
    }
    assert_eq!(seen, 1, "the scan reaches the own widgets' one scroll area");
}

#[test]
fn no_own_widget_opens_a_row_in_the_layout_it_was_handed() {
    let owned = [("mdedit.rs", "md_editor")];
    let mut met = std::collections::BTreeSet::new();
    for (name, text) in files() {
        let code = code_only(&text);
        for opener in [".horizontal(", ".horizontal_top(", ".horizontal_wrapped(", ".horizontal_centered("] {
            for (at, _) in code.match_indices(opener) {
                let f = enclosing_fn(&code, at);
                assert!(owned.contains(&(name.as_str(), f.as_str())), "own/{name}: `{f}` opens a row ({opener}) in the layout it was handed; take one cell (`width::cell`)");
                met.insert((name.clone(), f));
            }
        }
    }
    for (file, f) in owned {
        assert!(met.contains(&(file.to_string(), f.to_string())), "the scan no longer meets own/{file} `{f}`: drop it from the containers");
    }
}

fn texts(out: &eravon_ui::egui::FullOutput) -> Vec<(String, eravon_ui::egui::Rect)> {
    out.shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            eravon_ui::egui::Shape::Text(t) => Some((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2()))),
            _ => None,
        })
        .collect()
}

#[test]
fn the_tool_row_stays_inside_its_region_at_any_width() {
    use eravon_ui::egui::{self, pos2, vec2, Rect};
    let words: [([&str; 7], &str); 2] = [
        (["标题", "粗体", "斜体", "列表", "编号", "引用", "代码"], "导入正文"),
        (["Heading", "Bold", "Italic", "List", "Numbered", "Quote", "Code"], "Import text"),
    ];
    for (tools, extra) in words {
        for w in [180.0_f32, 240.0, 320.0, 480.0, 784.0] {
            let ctx = egui::Context::default();
            let _ = eravon_ui::fonts::install(&ctx, &eravon_ui::fonts::Found::none());
            let mut src = String::from("正文 text");
            let mut region = Rect::NOTHING;
            let mut out = None;
            for _ in 0..3 {
                let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))), ..Default::default() };
                out = Some(ctx.run_ui(raw, |ui| {
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(40.0, 40.0), vec2(w, 600.0))));
                    region = child.max_rect();
                    let _ = eravon_ui::mdedit::md_editor(&mut child, "row-width", &mut src, tools, 200.0, |ui| {
                        let _ = eravon_ui::button::key(ui, extra, eravon_ui::button::Role::Secondary, true);
                    });
                }));
            }
            let drawn = texts(&out.expect("frames ran"));
            for word in tools.iter().chain(std::iter::once(&extra)) {
                let r = drawn.iter().find(|(t, _)| t == word).map(|(_, r)| *r).unwrap_or_else(|| panic!("width {w}: `{word}` is drawn"));
                assert!(r.right() <= region.right() + 0.5, "width {w}: `{word}` ends at {} past the region's right edge {}", r.right(), region.right());
            }
        }
    }
}
