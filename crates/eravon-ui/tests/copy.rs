use eravon_ui::egui;
use eravon_ui::kv::{self, Val};
use eravon_ui::palette::{c, C};

const ADDRESS: &str = "0x8ba1f109551bD432803012645Ac136ddd64DBA72";

fn frame(ctx: &egui::Context, events: Vec<egui::Event>) -> (egui::FullOutput, egui::Rect) {
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 240.0))), events, ..Default::default() };
    let out = ctx.run_ui(input, |ui| {
        eravon_ui::skin::apply(ui.ctx());
        egui::CentralPanel::default().show_inside(ui, |ui| kv::value(ui, &Val::mono(ADDRESS)));
    });
    fn walk(s: &egui::Shape, at: &mut egui::Rect) {
        match s {
            egui::Shape::Text(t) if t.galley.job.text == ADDRESS => *at = egui::Rect::from_min_size(t.pos, t.galley.size()),
            egui::Shape::Vec(v) => v.iter().for_each(|y| walk(y, at)),
            _ => {}
        }
    }
    let mut at = egui::Rect::NOTHING;
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut at));
    (out, at)
}

fn copied(out: &egui::FullOutput) -> Vec<String> {
    out.platform_output.commands.iter().filter_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None }).collect()
}

#[test]
fn a_value_dragged_over_is_selected_in_the_familys_colours_and_copied_whole() {
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let (_, at) = frame(&ctx, vec![]);
    let (_, at2) = frame(&ctx, vec![]);
    assert!(at2.width() > 0.0 && at == at2, "the value is laid out where it stands: {at:?} {at2:?}");
    assert_eq!(copied(&frame(&ctx, vec![egui::Event::Copy]).0), Vec::<String>::new(), "nothing selected, nothing copied");
    let (from, to) = (egui::pos2(at.left() + 1.0, at.center().y), egui::pos2(at.right() + 40.0, at.center().y));
    let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    frame(&ctx, vec![egui::Event::PointerMoved(from)]);
    frame(&ctx, vec![button(from, true)]);
    frame(&ctx, vec![egui::Event::PointerMoved(to)]);
    frame(&ctx, vec![button(to, false)]);
    let (out, _) = frame(&ctx, vec![egui::Event::Copy]);
    assert_eq!(copied(&out), vec![ADDRESS.to_string()], "the copy key hands the selected value whole");
    let v = ctx.global_style().visuals.selection;
    assert_eq!((v.bg_fill, v.stroke.color), (c(C::AccentWash), c(C::AccentInk)), "the selection is the skin's");
}
