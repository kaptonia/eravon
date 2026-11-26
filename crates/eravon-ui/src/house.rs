use crate::button::{key, size_of, Key, Role};
use crate::palette::{c, C};
use crate::{motion, tokens};

pub const GOODS_TERM_MIN_W: f32 = 48.0;
pub const GOODS_EDITOR_H: f32 = 220.0;
pub const TEXT_BOX_MAX_H: f32 = 260.0;
pub const COPY_KEY: f32 = 19.0;
pub const COPY_GLYPH: f32 = 15.0;
pub const OPEN_MARK: f32 = 15.0;
pub const OPEN_MARK_INSET: f32 = 2.0;
pub const ROW_VALUE_MIN_W: f32 = 80.0;
pub const ALL_CLEAR_PAD_X: f32 = 20.0;
pub const ALL_CLEAR_PAD_Y: f32 = 44.0;
pub const ALL_CLEAR_MARK: f32 = 40.0;
pub const ALL_CLEAR_GAP: f32 = 10.0;
pub const KEYFILE_DROP_H: f32 = 72.0;
pub const PROBE_EDGE_SLACK: f32 = 0.5;
pub const PROBE_SAME_H_SLACK: f32 = 1.0;
pub const PROBE_CLICK_MIN_H: f32 = 20.0;

pub const WINDOW_W: f32 = 1180.0;
pub const WINDOW_H: f32 = 760.0;
pub const CARD_MIN_W: f32 = 200.0;
pub const HAIRLINE: f32 = 1.0;
pub const FOLD_GAP: f32 = 12.0;
pub const EMPTY_PAD_CARD: f32 = 20.0;
pub const EMPTY_PAD_INLINE: f32 = 16.0;
pub const DROP_H: f32 = 112.0;
pub const SKELETON_TWO: [f32; 2] = [0.7, 0.5];
pub const SKEL_SHORT: f32 = 0.35;
pub const SKEL_FIG: f32 = 0.5;
pub const SKEL_MID: f32 = 0.6;
pub const SKEL_LONG: f32 = 0.9;
pub const RAIL_NOTE_CHARS: usize = 40;
pub const PASTE_MAX_H: f32 = 96.0;
pub const WORDS_MASK_H: f32 = 106.0;
pub const FORM_SEG_W: f32 = 160.0;
pub const DD_MENU_MIN_W: f32 = 160.0;
pub const ROLE_MENU_W: f32 = 150.0;
pub const LADDER_PCT_W: f32 = 120.0;
pub const WIZ_SLIDE: f32 = 24.0;
pub const MESSAGE_H: f32 = 40.0;

pub const DATE_W: f32 = 136.0;
pub fn search_beside_dates(total: f32) -> f32 {
    let date = DATE_W + tokens::S2;
    total - date - date
}

pub const COL_MARK: f32 = 20.0;
pub const COL_STATE: f32 = 72.0;
pub const COL_PILL: f32 = 56.0;
pub const COL_NARROW: f32 = 64.0;
pub const COL_ORDER_NO: f32 = 104.0;
pub const COL_HOME_NO: f32 = 84.0;
pub const COL_WHO: f32 = 80.0;
pub const COL_DEADLINE: f32 = 110.0;
pub const COL_HOME_DEADLINE: f32 = 88.0;
pub const COL_REST: f32 = 1.0;
pub const COL_QUEUED_AT: f32 = 96.0;
pub const COL_AMOUNT: f32 = 120.0;
pub const COL_HOME_AMOUNT: f32 = 110.0;
pub const COL_HOME_WHO: f32 = 72.0;
pub const COL_TIME: f32 = 110.0;
pub const COL_RECORD_TIME: f32 = 150.0;
pub const COL_KIND: f32 = 90.0;
pub const COL_PRICE: f32 = 130.0;
pub const COL_JUDGES: f32 = 180.0;
pub const COL_WHEN: f32 = 150.0;
pub const REST: crate::table::Col = crate::table::Col::Fr(1.0);

pub const QR_SIDE: f32 = 160.0;
pub const QR_PLATE_W: f32 = QR_SIDE + 2.0 * crate::paint::QR_PAD;
pub const QR_CAPTION_GAP: f32 = 10.0;
pub const GAS_KEY_W: f32 = 88.0;
pub const GAS_COL_GAP: f32 = 20.0;
pub const GAS_ROW_GAP: f32 = 20.0;
pub const GAS_GROUP_GAP: f32 = 28.0;
pub const GAS_ADDR_LINE: usize = 21;

fn addr_w(ctx: &egui::Context) -> f32 {
    ctx.fonts_mut(|f| f.layout_no_wrap("0".repeat(GAS_ADDR_LINE), tokens::Type::Mono.font(), egui::Color32::BLACK).size().x).ceil()
}

pub fn address_face_w(ctx: &egui::Context, plate_w: f32) -> f32 {
    2.0 * tokens::SHEET_PAD + GAS_KEY_W + GAS_COL_GAP + addr_w(ctx) + GAS_GROUP_GAP + plate_w
}

pub fn address_face(ui: &mut egui::Ui, key_word: &str, address: &str, caption: &str, plate_w: f32, plate: impl FnOnce(&mut egui::Ui)) {
    use tokens::Type;
    let aw = addr_w(ui.ctx());
    let pw = plate_w;
    let words_w = GAS_KEY_W + GAS_COL_GAP + aw;
    let side = ui.available_width() + 0.5 >= words_w + GAS_GROUP_GAP + pw;
    let text = |s: &str, ty: Type, colour: egui::Color32, wrap: Option<f32>| {
        let mut job = egui::text::LayoutJob::single_section(s.to_string(), egui::TextFormat { font_id: ty.font(), color: colour, line_height: Some(Type::Body.line()), ..Default::default() });
        if let Some(w) = wrap {
            job.wrap = egui::text::TextWrapping { max_width: w, break_anywhere: true, ..Default::default() };
        }
        let g = ui.fonts_mut(|f| f.layout_job(job));
        let base = g.rows.first().and_then(|r| r.glyphs.first()).map(|x| x.pos.y).unwrap_or(0.0);
        (g, base)
    };
    let (kg, kb) = text(key_word, Type::Body, c(C::Ink2), None);
    let (vg, vb) = text(address, Type::Mono, c(C::Ink), Some(aw + 0.5));
    let base = kb.max(vb);
    let words_h = (base - kb + kg.size().y).max(base - vb + vg.size().y);
    let cap = {
        let mut job = egui::text::LayoutJob::single_section(caption.to_string(), egui::TextFormat { font_id: Type::Small.font(), color: c(C::Ink3), line_height: Some(Type::Body.line()), ..Default::default() });
        job.wrap.max_width = pw;
        job.halign = egui::Align::Center;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let code_h = pw + QR_CAPTION_GAP + cap.size().y;
    let room = ui.available_width();
    let h = if side { words_h.max(code_h) } else { words_h + GAS_ROW_GAP + code_h };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(room, h), egui::Sense::hover());
    let (words_at, code_at) = if side {
        (egui::pos2(rect.left(), rect.center().y - words_h / 2.0), egui::pos2(rect.right() - pw, rect.center().y - code_h / 2.0))
    } else {
        (rect.left_top(), egui::pos2(rect.center().x - pw / 2.0, rect.top() + words_h + GAS_ROW_GAP))
    };
    ui.painter().galley(egui::pos2(words_at.x, words_at.y + base - kb), kg, c(C::Ink2));
    let at = egui::pos2(words_at.x + GAS_KEY_W + GAS_COL_GAP, words_at.y + base - vb);
    let mut line = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(at, vg.size())));
    line.add(egui::Label::new(vg).selectable(true));
    let mut code_ui = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(code_at, egui::vec2(pw, code_h))).layout(egui::Layout::top_down(egui::Align::Center)));
    plate(&mut code_ui);
    code_ui.add_space(QR_CAPTION_GAP);
    let cr = code_ui.allocate_exact_size(egui::vec2(pw, cap.size().y), egui::Sense::hover()).0;
    code_ui.painter().galley(egui::pos2(cr.center().x, cr.top()), cap, c(C::Ink3));
}

pub fn cover_message(ui: &mut egui::Ui, id_salt: &str, busy: Option<&str>, trouble: Option<&str>, gap: f32) {
    let id = ui.id().with(("eravon-cover-message", id_salt));
    let on = busy.is_some() || trouble.is_some();
    let open = motion::flag(ui.ctx(), id, on, tokens::MID);
    if open <= 0.0 {
        return;
    }
    let h_id = id.with("h");
    let full_h = ui.ctx().data(|d| d.get_temp::<f32>(h_id)).unwrap_or(MESSAGE_H);
    let shown_h = (gap + full_h) * open;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, shown_h), egui::Sense::hover());
    let inner = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.top() + gap * open), egui::vec2(w, full_h));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Center)));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child.multiply_opacity(open);
    let r = child.vertical_centered(|ui| {
        if let Some(s) = busy {
            crate::sheet::busy_note(ui, s, true);
        } else if let Some(s) = trouble {
            crate::states::note_box(ui, s);
        }
    });
    let got = r.response.rect.height();
    if on && (got - full_h).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(h_id, got));
        ui.ctx().request_discard("gate message height changed");
    }
}

pub fn center_keys(ui: &mut egui::Ui, left: Option<&str>, right: &str, right_live: bool) -> (bool, bool) {
    let lw = left.map(|l| size_of(ui, &Key::new(l, Role::Secondary)).x + tokens::S2).unwrap_or(0.0);
    let rw = size_of(ui, &Key::new(right, Role::Primary)).x;
    let w = ui.available_width();
    let (row, _) = ui.allocate_exact_size(egui::vec2(w, tokens::KEY_H), egui::Sense::hover());
    let start = row.center().x - (lw + rw) / 2.0;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(start, row.top()), egui::vec2(lw + rw + 1.0, row.height()))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.spacing_mut().item_spacing.x = tokens::S2;
    let l = left.map(|l| key(&mut child, l, Role::Secondary, true).clicked()).unwrap_or(false);
    let r = key(&mut child, right, Role::Primary, right_live).clicked();
    (l, r)
}

pub fn step_slide<R>(ui: &mut egui::Ui, at: usize, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let dir_id = egui::Id::new("eravon-wizard-dir");
    let (was, dir) = ui.ctx().data(|d| d.get_temp::<(usize, f32)>(dir_id)).unwrap_or((at, 0.0));
    let dir = if was == at { dir } else if at > was { 1.0 } else { -1.0 };
    ui.ctx().data_mut(|d| d.insert_temp(dir_id, (at, dir)));
    let p = motion::enter(ui.ctx(), egui::Id::new("eravon-wizard-step"), at as u64, 0.0, 0.22 * tokens::PACE, motion::Curve::Ease);
    motion::shifted(ui, egui::vec2(WIZ_SLIDE * dir * (1.0 - p), 0.0), p, at as u64, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        add(ui)
    })
}

pub fn choice_line(ui: &mut egui::Ui, id: egui::Id, on: bool, title: &str) -> egui::Response {
    let w = ui.available_width();
    let text_x = 14.0 + 8.0 + 8.0 + 10.0;
    let h = 12.0 + tokens::Type::Strong.line() + 12.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let lit = motion::flag(ui.ctx(), id.with("on"), on, tokens::FAST);
    let dot = motion::to(ui.ctx(), id.with("dot"), if on { 1.0 } else { 0.0 }, tokens::FAST, motion::Curve::Spring);
    let p = ui.painter().clone();
    let fill = crate::palette::mix(c(C::Surface), c(C::Sel), lit);
    let edge = crate::palette::mix(c(C::Line), c(C::Accent), lit);
    p.rect(rect, tokens::Radius::Menu.egui(), fill, egui::Stroke::new(1.0_f32, edge), egui::StrokeKind::Inside);
    let rc = egui::pos2(rect.left() + 14.0 + 8.0, rect.center().y);
    p.circle_stroke(rc, 7.5, egui::Stroke::new(1.5_f32, edge));
    if dot > 0.0 {
        p.circle_filled(rc, 4.5 * dot, c(C::Accent));
    }
    p.text(egui::pos2(rect.left() + text_x, rect.center().y), egui::Align2::LEFT_CENTER, title, tokens::Type::Strong.font(), c(C::Ink));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
