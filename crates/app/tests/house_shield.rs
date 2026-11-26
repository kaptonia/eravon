use eravon_ui::egui;

fn frame(ctx: &egui::Context, events: Vec<egui::Event>, shield: bool, hits: &mut [bool; 3]) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
        events,
        ..Default::default()
    };
    let _ = ctx.run_ui(input, |ui| {
        let ctx = &ui.ctx().clone();
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.put(egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(80.0, 30.0)), egui::Button::new("page"))
                .clicked()
                .then(|| hits[0] = true);
        });
        egui::Area::new(egui::Id::new("card")).order(egui::Order::Foreground).fixed_pos(egui::pos2(200.0, 10.0)).show(ctx, |ui| {
            ui.add_sized([80.0, 30.0], egui::Button::new("card")).clicked().then(|| hits[1] = true);
        });
        if shield {
            eravon_ui::scrim::shield(ctx, "busy");
        }
        egui::Area::new(egui::Id::new("toast")).order(egui::Order::Tooltip).fixed_pos(egui::pos2(400.0, 10.0)).show(ctx, |ui| {
            ui.add_sized([80.0, 30.0], egui::Button::new("toast")).clicked().then(|| hits[2] = true);
        });
    });
}

fn click(shield: bool, at: egui::Pos2) -> [bool; 3] {
    let ctx = egui::Context::default();
    let mut hits = [false; 3];
    frame(&ctx, vec![], shield, &mut hits);
    frame(&ctx, vec![egui::Event::PointerMoved(at)], shield, &mut hits);
    let press = egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() };
    frame(&ctx, vec![press], shield, &mut hits);
    let release = egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() };
    frame(&ctx, vec![release], shield, &mut hits);
    frame(&ctx, vec![], shield, &mut hits);
    hits
}

#[test]
fn without_shield_every_layer_takes_a_click() {
    assert!(click(false, egui::pos2(40.0, 25.0))[0], "页体的键该点得动");
    assert!(click(false, egui::pos2(240.0, 25.0))[1], "卡上的键该点得动");
    assert!(click(false, egui::pos2(440.0, 25.0))[2], "吐司的键该点得动");
}

#[test]
fn with_shield_only_toasts_take_a_click() {
    assert!(!click(true, egui::pos2(40.0, 25.0))[0], "遮挡在时页体的键不该点得动");
    assert!(!click(true, egui::pos2(240.0, 25.0))[1], "遮挡在时卡上的键不该点得动");
    assert!(click(true, egui::pos2(440.0, 25.0))[2], "吐司在遮挡之上,该点得动");
}
