use crate::palette::{c, C};

pub fn shield(ctx: &egui::Context, line: &str) {
    let id = egui::Id::new("eravon-busy-shield");
    crate::layer::place(ctx, egui::LayerId::new(egui::Order::Foreground, id), None);
    crate::layer::area(id, egui::Order::Foreground).fixed_pos(egui::Pos2::ZERO).show(ctx, |ui| {
        let r = ui.allocate_rect(ctx.content_rect(), egui::Sense::click_and_drag());
        if r.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Progress);
        }
        let font = egui::FontId::proportional(13.0);
        let ink = c(C::Ink);
        let galley = ui.painter().layout_no_wrap(line.to_string(), font, c(C::Ground));
        let size = galley.size() + egui::vec2(24.0, 12.0);
        let at = egui::Rect::from_center_size(egui::pos2(ctx.content_rect().center().x, ctx.content_rect().top() + 16.0 + size.y / 2.0), size);
        ui.painter().rect_filled(at, egui::CornerRadius::same((size.y / 2.0) as u8), egui::Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 220));
        ui.painter().galley(at.min + egui::vec2(12.0, 6.0), galley, egui::Color32::PLACEHOLDER);
    });
}
