use eravon_ui::egui;

fn dressed() -> egui::Context {
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    ctx
}

fn rows_in(out: &egui::FullOutput, text: &str) -> Vec<String> {
    fn walk(s: &egui::Shape, text: &str, out: &mut Option<Vec<String>>) {
        match s {
            egui::Shape::Text(x) if x.galley.job.text == text && out.is_none() => {
                *out = Some(x.galley.rows.iter().map(|r| r.row.glyphs.iter().map(|g| g.chr).collect()).collect());
            }
            egui::Shape::Vec(v) => v.iter().for_each(|y| walk(y, text, out)),
            _ => {}
        }
    }
    let mut got = None;
    for c in &out.shapes {
        walk(&c.shape, text, &mut got);
    }
    got.unwrap_or_else(|| panic!("drawn: {text:?}"))
}

fn broken_between_words(rows: &[String], text: &str) {
    assert!(rows.len() > 1, "wrapped: {rows:?}");
    for r in &rows[..rows.len() - 1] {
        assert!(r.ends_with(char::is_whitespace), "a row ends inside a word: {rows:?}");
    }
    assert_eq!(rows.concat(), text, "every character laid out once");
}

fn broken_between_characters(rows: &[String], text: &str) {
    assert!(rows.len() > 1, "wrapped: {rows:?}");
    assert!(rows[..rows.len() - 1].iter().any(|r| !r.ends_with(char::is_whitespace)), "Chinese breaks between characters: {rows:?}");
    assert_eq!(rows.concat(), text);
}

fn value_rows(text: &str, mark: bool, width: f32) -> Vec<String> {
    use eravon_ui::kv::Val;
    let ctx = dressed();
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1180.0, 800.0))), time: Some(0.1), ..Default::default() };
    let v = if mark { Val::Mark(eravon_ui::mark::Mark::Warn, text.to_string()) } else { Val::text(text) };
    let out = ctx.run_ui(input, |root| {
        egui::CentralPanel::default().show_inside(root, |ui| {
            ui.allocate_ui(egui::vec2(width, 600.0), |ui| eravon_ui::width::body(ui, |ui| eravon_ui::kv::kv(ui, &[("Chain reading", v.clone())])));
        });
    });
    rows_in(&out, text)
}

#[test]
fn an_english_value_breaks_between_words_and_a_chinese_one_anywhere() {
    let english = "Only 1 node responded; no cross-check · 1 node(s) did not answer; see What each said under the details below";
    for mark in [false, true] {
        broken_between_words(&value_rows(english, mark, 520.0), english);
    }
    let chinese = "只有 1 处节点作答,无从互核 · 1 处节点未作答,各自的原话见「各处原话」,这一句在窄栏里要排成好几行才放得下".repeat(2);
    for mark in [false, true] {
        broken_between_characters(&value_rows(&chinese, mark, 520.0), &chinese);
    }
}
