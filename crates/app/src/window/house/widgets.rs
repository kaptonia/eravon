use super::*;

pub(in crate::window) fn card_keys<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    paint::rule(ui, eravon_ui::metrics::S3);
    keys_row(ui, add)
}

pub(in crate::window) fn said_line(name: &str, detail: &str) -> String {
    if detail.is_empty() { name } else { detail }.to_string()
}

pub(in crate::window) fn raw_of(name: &str, detail: &str) -> String {
    if detail.is_empty() { name.to_string() } else { format!("{name} · {detail}") }
}

fn said_first(lang: crate::lang::Lang, name: &str, detail: &str) -> String {
    said_line(name, &crate::house::refusal::line(lang, name, detail))
}

fn said_more(ui: &mut egui::Ui, id: &str, name: &str, detail: &str, first: &str) {
    if detail.is_empty() || first == detail {
        return;
    }
    fold::fold(ui, id, &crate::house::i18n::t(crate::house::i18n::Key::ToastDetail), |ui| {
        paint::text(ui, &raw_of(name, detail), Type::MonoSmall, c(C::Ink3));
    });
}

pub(in crate::window) fn said_hint(ui: &mut egui::Ui, id: &str, lang: crate::lang::Lang, name: &str, detail: &str, bad: bool) {
    let first = said_first(lang, name, detail);
    states::hint_ex(ui, &first, bad);
    said_more(ui, id, name, detail, &first);
}

pub(in crate::window) fn said_banner(ui: &mut egui::Ui, id: &str, lang: crate::lang::Lang, name: &str, detail: &str) {
    let first = said_first(lang, name, detail);
    states::banner(ui, states::Banner::Bad, &first, |_| ());
    said_more(ui, id, name, detail, &first);
}

pub(in crate::window) fn said_note(ui: &mut egui::Ui, id: &str, lang: crate::lang::Lang, name: &str, detail: &str) {
    let first = said_first(lang, name, detail);
    states::note_box(ui, &first);
    said_more(ui, id, name, detail, &first);
}

pub(in crate::window) fn tone_mark(t: eravon_ui::palette::Tone) -> mark::Mark {
    match t {
        eravon_ui::palette::Tone::Ok => mark::Mark::Ok,
        eravon_ui::palette::Tone::Warn => mark::Mark::Warn,
        eravon_ui::palette::Tone::Bad => mark::Mark::Bad,
        eravon_ui::palette::Tone::Grey => mark::Mark::Todo,
        eravon_ui::palette::Tone::Blue => mark::Mark::Work,
    }
}

pub(in crate::window) fn picked_beside(ui: &mut egui::Ui, path: &str) {
    let Some(name) = chosen_name(path) else { return };
    let font = eravon_ui::tokens::Type::MonoSmall.font();
    let shown = eravon_ui::width::elide_mid(ui, &name, font.clone(), ui.available_width().max(0.0));
    let r = ui.label(egui::RichText::new(&shown).font(font).color(c(C::Ink2)));
    let _ = eravon_ui::layer::tip(r, path);
}

pub(in crate::window) fn listed_gen(rows_gen: u64, search: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (rows_gen, search).hash(&mut h);
    h.finish()
}

pub(in crate::window) fn chosen_row(ui: &mut egui::Ui, path: &str, lang: crate::lang::Lang) {
    let label = crate::house::i18n::get(lang, crate::house::i18n::Key::OrdersFileSheet).0;
    let none = crate::house::i18n::get(lang, crate::house::i18n::Key::PickNone).0;
    eravon_ui::card::form(ui, |ui, f| {
        match chosen_name(path) {
            Some(n) => f.info(ui, &label, eravon_ui::card::Show::Text(&n)),
            None => f.info(ui, &label, eravon_ui::card::Show::Empty(&none)),
        };
    });
}

pub(in crate::window) fn chosen_name(p: &str) -> Option<String> {
    let n = eravon_ui::width::file_name(p);
    (!n.is_empty()).then_some(n)
}

pub(in crate::window) fn pick_zone(ui: &mut egui::Ui, id: &str, title: &str, sub: &str, chosen: Option<(&str, u64)>, lang: crate::lang::Lang) -> eravon_ui::drop::Drop {
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    let chosen = chosen.and_then(|(path, len)| chosen_name(path).map(|n| (n, super::super::faces::size_say(len))));
    let change = t(crate::house::i18n::Key::DropClickSwap);
    eravon_ui::drop::zone(ui, id, Some(Glyph::Inbox), &[title, sub], chosen.as_ref().map(|(n, s)| (n.as_str(), s.as_str(), change.as_str())), eravon_ui::house::DROP_H, eravon_ui::drop::Shape::Column, true)
}

pub(in crate::window) fn split_raw(s: &str) -> (String, Vec<String>) {
    let mut raw = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    for part in s.split(" · ") {
        let mut words: Vec<&str> = Vec::new();
        for w in part.split_whitespace() {
            let bare = w.trim_matches(|c: char| matches!(c, '(' | ')' | ',' | ':' | '\u{ff08}' | '\u{ff09}' | '\u{ff0c}' | '\u{ff1a}' | '\u{300c}' | '\u{300d}'));
            let hex = bare.strip_prefix("0x").is_some_and(|h| h.len() >= 20 && h.chars().all(|c| c.is_ascii_hexdigit()));
            let path = bare.starts_with('/') && bare.len() > 1;
            if hex || path {
                raw.push(bare.to_string());
            } else {
                words.push(w);
            }
        }
        if !words.is_empty() {
            kept.push(words.join(" "));
        }
    }
    (kept.join(" · "), raw)
}

#[cfg(test)]
mod qr_one_door {
    #[test]
    fn a_code_is_encoded_and_painted_only_on_the_plate() {
        let pats = [concat!("qr::", "encode("), concat!("paint::", "qr(")];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut dirs = vec![root.clone()];
        let mut hits: Vec<String> = Vec::new();
        while let Some(d) = dirs.pop() {
            for e in std::fs::read_dir(&d).expect("读得到源").flatten() {
                let p = e.path();
                if p.is_dir() {
                    dirs.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let text = std::fs::read_to_string(&p).unwrap_or_default();
                    for pat in pats {
                        hits.extend(text.match_indices(pat).map(|(i, _)| format!("{}@{i}", p.strip_prefix(&root).unwrap_or(&p).display())));
                    }
                }
            }
        }
        let here = include_str!("widgets.rs");
        let start = here.find(concat!("pub(in crate::window) fn ", "qr_plate(")).expect("qr_plate 在");
        let end = start + here[start..].find("\n}\n").expect("qr_plate 的尾");
        let ok = |h: &String| h.rsplit_once('@').is_some_and(|(f, i)| f.ends_with("widgets.rs") && i.parse::<usize>().is_ok_and(|i| (start..end).contains(&i)));
        let outside: Vec<&String> = hits.iter().filter(|h| !ok(h)).collect();
        assert!(outside.is_empty(), "codes made outside the plate: {outside:?}");
        assert_eq!(hits.len(), 2, "the plate encodes once and paints once");
    }
}

#[cfg(test)]
mod split_raw_tests {
    #[test]
    fn raw_hashes_and_paths_leave_the_front() {
        let (h, r) = super::split_raw("到期戳 · 0x8c42ed5cb5009606594223c1e123785923ca64a4d238f9201e65c44b4636c966");
        assert_eq!(h, "到期戳");
        assert_eq!(r.len(), 1);
        let (h, r) = super::split_raw("已导出提案 · /tmp/x/Downloads/p.json");
        assert_eq!((h.as_str(), r[0].as_str()), ("已导出提案", "/tmp/x/Downloads/p.json"));
        assert_eq!(super::split_raw("已保存").0, "已保存");
    }
}

#[cfg(test)]
mod pair_and_name {
    use super::chosen_name;

    #[test]
    fn only_the_last_segment_of_a_path_is_shown() {
        assert_eq!(chosen_name("/tmp/x/Downloads/key.json").as_deref(), Some("key.json"));
        assert_eq!(chosen_name("/tmp/x/homeS/").as_deref(), Some("homeS"));
        assert_eq!(chosen_name(""), None);
    }
}

pub(in crate::window) fn date_of(secs: Option<u64>) -> String {
    secs.map(eravon_orders::when::day).unwrap_or_default()
}

pub(in crate::window) fn short(a: &str) -> String {
    let n = a.chars().count();
    if n > 12 {
        let head: String = a.chars().take(6).collect();
        let tail: String = a.chars().skip(n - 4).collect();
        format!("{head}…{tail}")
    } else {
        a.to_string()
    }
}

pub(in crate::window) fn qr_plate(ui: &mut egui::Ui, text: &str) {
    let id = egui::Id::new(("eravon-qr", text));
    let code = ui.ctx().data_mut(|d| d.get_temp::<Option<std::sync::Arc<eravon_qr::Code>>>(id)).flatten().or_else(|| {
        let c = eravon_qr::encode(text.as_bytes()).map(std::sync::Arc::new);
        ui.ctx().data_mut(|d| d.insert_temp(id, c.clone()));
        c
    });
    match code {
        Some(code) => {
            paint::qr(ui, &code.modules, eravon_ui::house::QR_SIDE);
        }
        None => states::hint_ex(ui, &crate::house::i18n::t(crate::house::i18n::Key::QrNotMade), true),
    }
}

pub(in crate::window) fn qr_plate_w() -> f32 {
    eravon_ui::house::QR_PLATE_W
}

pub(in crate::window) fn span_text(secs: u64) -> String {
    eravon_cands::quote::lasts(crate::lang::lang(), secs)
}

pub(in crate::window) fn desc_block(ui: &mut egui::Ui, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    paint::rule(ui, eravon_ui::metrics::S3);
    md_view(ui, text);
    paint::rule(ui, eravon_ui::metrics::S3);
}

pub(in crate::window) fn titled_text(ui: &mut egui::Ui, head: &str, text: &str) {
    ui.add_space(eravon_ui::metrics::S2);
    eravon_ui::card::flat_title(ui, head);
    ui.add_space(eravon_ui::metrics::S2);
    md_view(ui, text);
    ui.add_space(eravon_ui::metrics::S2);
}

pub(in crate::window) fn md_view(ui: &mut egui::Ui, text: &str) {
    use crate::md::Block;
    ui.spacing_mut().item_spacing.y = eravon_ui::metrics::S2;
    for b in crate::md::blocks(text) {
        match b {
            Block::Heading(level, s) => {
                paint::text(ui, &s, if level <= 2 { eravon_ui::tokens::Type::Card } else { eravon_ui::tokens::Type::Strong }, c(C::Ink));
            }
            Block::Item(s) => {
                paint::text(ui, &format!("\u{2022} {s}"), eravon_ui::tokens::Type::Body, c(C::Ink));
            }
            Block::Quote(s) => {
                paint::text(ui, &format!("\u{2502} {s}"), eravon_ui::tokens::Type::Body, c(C::Ink2));
            }
            Block::Row(cells) => {
                paint::text(ui, &cells.join("  \u{2502}  "), eravon_ui::tokens::Type::Body, c(C::Ink));
            }
            Block::Para(s) => {
                paint::text(ui, &s, eravon_ui::tokens::Type::Body, c(C::Ink));
            }
            Block::Blank => ui.add_space(eravon_ui::metrics::S2),
        }
    }
}

pub(in crate::window) fn value_row(ui: &mut egui::Ui, label: &str, value: &str, mono: bool, key_at_end: Option<(&str, bool)>) -> bool {
    let mut hit = false;
    let w = ui.available_width();
    ui.allocate_ui_with_layout(egui::vec2(w, tk::KEY_H), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = tk::S3;
        if !label.is_empty() {
            let (r, _) = ui.allocate_exact_size(egui::vec2(tk::LABEL_W - tk::S3 + tk::S2, tk::KEY_H), egui::Sense::hover());
            paint::at(ui.painter(), ui, egui::pos2(r.left(), r.center().y), egui::Align2::LEFT_CENTER, label, Type::Note, c(C::Ink2), r.width());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some((words, live)) = key_at_end {
                hit = key::key(ui, words, Role::Secondary, live).clicked();
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let room = ui.available_width();
                let r = paint::line(ui, value, if mono { Type::Mono } else { Type::Body }, c(C::Ink), room);
                let _ = eravon_ui::layer::tip(r, value);
            });
        });
    });
    hit
}

pub(in crate::window) fn name_line(ui: &egui::Ui) -> egui::Rect {
    egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), Type::Note.line()))
}

pub(in crate::window) fn field_why(ui: &mut egui::Ui, line: egui::Rect, name: &str, why: &str, cell: egui::Response) {
    let named = ui.painter().layout_no_wrap(name.to_string(), egui::FontId::new(Type::Note.font().size, eravon_ui::fonts::medium()), c(C::Ink)).size().x;
    let room = line.width() - named - tk::S3;
    let drawn = paint::at(ui.painter(), ui, line.right_center(), egui::Align2::RIGHT_CENTER, why, Type::Small, c(C::BadInk), room);
    let over = ui.interact(drawn, ui.id().with(("field-why", name)), egui::Sense::hover());
    let _ = eravon_ui::layer::tip(over, why);
    let _ = eravon_ui::layer::tip(cell, why);
}

pub(in crate::window) fn input_row<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let w = ui.available_width();
    ui.allocate_ui_with_layout(egui::vec2(w, tk::INPUT_H), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = tk::S2;
        add(ui)
    })
    .inner
}

pub(in crate::window) fn detail_rows_of<'a>(ui: &egui::Ui, rows: &[(&'a str, kv::Val)]) -> (Vec<(&'a str, card::Detail)>, f32) {
    let font = eravon_ui::tokens::Type::Note.font();
    let widest = rows
        .iter()
        .flat_map(|(k, _)| [Some(*k), crate::lang::twin(k)])
        .flatten()
        .map(|k| ui.painter().layout_no_wrap(k.to_string(), font.clone(), c(C::Ink)).size().x)
        .fold(0.0, f32::max);
    let detail = rows
        .iter()
        .map(|(k, v)| {
            let d = match v {
                kv::Val::Mono(s) => card::Detail::Mono(s.clone()),
                kv::Val::Figure(s) => card::Detail::Figure(s.clone()),
                kv::Val::Amount(a) => card::Detail::Figure(a.whole.clone()),
                kv::Val::Quiet(s) => card::Detail::Quiet(s.clone()),
                kv::Val::Text(s) | kv::Val::Mark(_, s) => card::Detail::Text(s.clone()),
                kv::Val::Loading(_) => card::Detail::Quiet(String::new()),
            };
            (*k, d)
        })
        .collect();
    (detail, widest)
}

pub(in crate::window) fn info_rows(ui: &mut egui::Ui, f: &mut card::Form, rows: &[(&str, kv::Val)]) -> Option<String> {
    let mut copied = None;
    for (k, v) in rows {
        let c = match v {
            kv::Val::Text(s) => f.info(ui, k, card::Show::Text(s)),
            kv::Val::Mono(s) => f.info(ui, k, card::Show::Copy(s)),
            kv::Val::Figure(s) => f.info(ui, k, card::Show::Mono(s)),
            kv::Val::Amount(a) => f.info(ui, k, card::Show::Mono(&a.shown)),
            kv::Val::Mark(m, s) => f.info(ui, k, card::Show::Mark(*m, s)),
            kv::Val::Quiet(s) => f.info(ui, k, card::Show::Empty(s)),
            kv::Val::Loading(_) => f.info(ui, k, card::Show::Empty("…")),
        };
        copied = c.or(copied);
    }
    copied
}

pub(in crate::window) fn rows_section(ui: &mut egui::Ui, title: &str, rows: &[(&str, kv::Val)]) {
    card::section(ui, title, "", |ui| {
        let copied = card::form(ui, |ui, f| info_rows(ui, f, rows));
        tell_copied(ui.ctx(), copied);
    });
}
