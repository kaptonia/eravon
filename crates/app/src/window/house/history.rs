use super::*;

pub(super) fn page(ui: &mut egui::Ui, lang: crate::house::i18n::Lang, id: &str, who: &str, h: &crate::house::history::Hist, reading: bool) {
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    Win::hero(ui, &format!("{} · {}", t(crate::house::i18n::Key::PlHistory), eravon_orders::table::short(who)), "", &[]);
    match h {
        Some(Ok((who, rs))) => {
            let x = eravon_link::identity::history(rs, who);
            let pending = crate::house::history::pending_of(rs, &x.keys);
            let cases = x.cases.len() + x.documents.iter().map(|d| d.opened.len()).sum::<usize>();
            card::form(ui, |ui, f| {
                f.info(ui, &t(crate::house::i18n::Key::PlHistoryKeys), card::Show::Text(&x.keys.join(" → ")));
                if let Some(p) = &pending {
                    f.info(ui, &t(crate::house::i18n::Key::IdRotStClaim), card::Show::Short(p));
                }
                f.info(ui, &t(crate::house::i18n::Key::PlHistoryDocs), card::Show::Text(&x.documents.len().to_string()));
                f.info(ui, &t(crate::house::i18n::Key::PlHistoryAttempts), card::Show::Text(&x.attempts().count().to_string()));
                f.info(ui, &t(crate::house::i18n::Key::PlHistoryCases), card::Show::Text(&cases.to_string()));
            });
        }
        Some(Err(e)) => states::err_box(ui, id, &crate::house::refusal::line(lang, &e.name, &e.detail), "", &t(crate::house::i18n::Key::U3RawError), &raw_of(&e.name, &e.detail)),
        None if reading => states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO),
        None => {}
    }
}
