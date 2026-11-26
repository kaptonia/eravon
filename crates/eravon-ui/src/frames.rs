use crate::metrics;
use crate::palette::{c, C};
use egui::{vec2, Color32, Context, Pos2, Rect};

#[derive(Clone, Debug, PartialEq)]
pub enum Reading {
    Nums(Vec<f32>),
    Words(Vec<String>),
    Flags(Vec<(&'static str, bool)>),
}

pub const STEP: f64 = 0.02;

fn headless() -> Context {
    let ctx = Context::default();
    let mut defs = egui::FontDefinitions::empty();
    for (file, name) in [("Inter-Regular.ttf", "latin"), ("JetBrainsMono-Regular.ttf", "mono")] {
        if let Some(b) = crate::fonts::embedded(file) {
            defs.font_data.insert(name.into(), std::sync::Arc::new(egui::FontData::from_static(b)));
        }
    }
    defs.families.insert(egui::FontFamily::Proportional, vec!["latin".into()]);
    defs.families.insert(egui::FontFamily::Monospace, vec!["mono".into(), "latin".into()]);
    defs.families.insert(crate::fonts::strong(), vec!["latin".into()]);
    defs.families.insert(crate::fonts::medium(), vec!["latin".into()]);
    ctx.set_fonts(defs);
    crate::skin::apply(&ctx);
    ctx
}

fn raw(t: f64, w: f32, events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, crate::house::WINDOW_H))), time: Some(t), events, ..Default::default() }
}

fn frame(ctx: &Context, t: f64, w: f32, events: Vec<egui::Event>, draw: impl FnMut(&mut egui::Ui)) -> (Vec<egui::Shape>, bool) {
    let out = ctx.run_ui(raw(t, w, events), draw);
    let again = out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.repaint_delay < std::time::Duration::from_secs(1)).unwrap_or(false);
    let mut flat = Vec::new();
    for cs in out.shapes {
        flatten(cs.shape, &mut flat);
    }
    (flat, again)
}

fn flatten(s: egui::Shape, out: &mut Vec<egui::Shape>) {
    match s {
        egui::Shape::Vec(v) => v.into_iter().for_each(|x| flatten(x, out)),
        other => out.push(other),
    }
}

fn text_at(shapes: &[egui::Shape], text: &str) -> Option<(Pos2, f32)> {
    shapes.iter().find_map(|s| match s {
        egui::Shape::Text(t) if t.galley.job.text == text => {
            let c = t.galley.job.sections.first().map(|x| x.format.color).unwrap_or(t.fallback_color);
            Some((t.pos, c.a() as f32 / 255.0 * t.opacity_factor))
        }
        _ => None,
    })
}

fn texts(shapes: &[egui::Shape]) -> Vec<String> {
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
            _ => None,
        })
        .collect()
}

fn page(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default().show_inside(ui, add);
}

pub fn rail_slide() -> Vec<f32> {
    let ctx = headless();
    let names = ["a", "b", "c", "d", "e", "f"];
    let lines: Vec<crate::rail::Line<'_>> = names.iter().map(|n| crate::rail::Line::Item { glyph: crate::icons::Glyph::Home, label: n, count: 0 }).collect();
    let pitch = crate::tokens::RAIL_ITEM_H + 2.0;
    let mut out = vec![3.0 * pitch];
    let mut top0: Option<f32> = None;
    for i in 0..25 {
        let lit = if i < 5 { 0 } else { 3 };
        let mut list_top = 0.0;
        let (shapes, _) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            page(root, |ui| {
                list_top = ui.cursor().min.y + 4.0;
                crate::rail::nav(ui, &lines, Some(lit));
            })
        });
        top0.get_or_insert(list_top);
        let block = shapes.iter().find_map(|s| match s {
            egui::Shape::Rect(r) if (r.rect.height() - crate::tokens::RAIL_ITEM_H).abs() < 0.5 && r.stroke.width > 0.0 => Some(r.rect.top()),
            _ => None,
        });
        if i >= 4 {
            out.push(block.map_or(-1.0, |y| y - top0.unwrap_or(0.0)));
        }
    }
    out
}

pub fn same_path() -> Vec<f32> {
    let ctx = headless();
    let mut out = Vec::new();
    for i in 0..40 {
        let (shapes, _) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            page(root, |ui| {
                crate::motion::stagger(ui, egui::Id::new("frames-c1"), 1, 0, |ui| crate::states::done_state(ui, "c1", "Done", ""));
            })
        });
        let ring = shapes
            .iter()
            .filter_map(|s| match s {
                egui::Shape::Path(p) if (p.stroke.width - 2.0).abs() < 1e-3 && p.points.len() > 2 => Some(p.points.len()),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        out.push(ring as f32);
    }
    out
}

pub fn scroll_width() -> Vec<f32> {
    let ctx = headless();
    let mut out = Vec::new();
    for (i, h) in std::iter::once(150.0_f32).chain(std::iter::repeat_n(201.0_f32, 10)).enumerate() {
        let mut w = 0.0;
        let _ = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            page(root, |ui| {
                crate::scroll::nested(ui, egui::ScrollArea::vertical().id_salt("frames-c2").max_height(200.0).auto_shrink([false, false]), |ui| {
                    w = ui.available_width();
                    ui.allocate_space(vec2(10.0, h));
                });
            })
        });
        out.push(w);
    }
    out
}

pub fn discard() -> Vec<f32> {
    let ctx = headless();
    let mut out = Vec::new();
    for i in 0..7 {
        let h = if i < 5 { 100.0 } else { 300.0 };
        let (shapes, _) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            page(root, |ui| {
                let rect = ui.max_rect();
                crate::full::centered(ui, "frames-c4", rect, 400.0, 0.0, |ui| {
                    ui.label("MARK");
                    ui.allocate_space(vec2(10.0, h));
                });
            })
        });
        if i >= 4 {
            out.push(text_at(&shapes, "MARK").map(|(p, _)| p.y).unwrap_or(f32::NAN));
        }
    }
    out
}

pub fn title() -> (Vec<f32>, Vec<String>) {
    let ctx = headless();
    let (mut alpha, mut words) = (Vec::new(), Vec::new());
    for i in 0..16 {
        let (name, target) = if i < 10 { ("Detail", 0.0) } else { ("Home", 1.0) };
        let (shapes, _) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            page(root, |ui| {
                let rect = Rect::from_min_size(ui.max_rect().min, vec2(ui.available_width(), crate::tokens::TOOLBAR_H));
                crate::toolbar::toolbar(ui, rect, name, target, (false, false), false, ("", ""), |_| {});
            })
        });
        if i >= 10 {
            alpha.push(text_at(&shapes, "Home").map(|(_, a)| a).unwrap_or(0.0));
            words.extend(texts(&shapes).into_iter().filter(|t| t == "Detail" || t == "Home"));
        }
    }
    (alpha, words)
}

pub fn leave() -> (Vec<f32>, Vec<f32>) {
    let ctx = headless();
    let surface = c(C::Surface);
    let card_alpha = |shapes: &[egui::Shape]| -> f32 {
        shapes
            .iter()
            .filter_map(|s| match s {
                egui::Shape::Rect(r) if (0.94..=1.01).contains(&(r.rect.width() / metrics::MODAL_W)) && r.fill.a() > 0 && r.fill.r() as f32 >= 0.9 * r.fill.a() as f32 => Some(r.fill.a() as f32 / surface.a() as f32),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
    };
    let (mut enter, mut out) = (Vec::new(), Vec::new());
    for i in 0..24 {
        let open = i < 12;
        let (shapes, _) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            let ctx = &root.ctx().clone();
            page(root, |_| {});
            if open {
                let _ = crate::sheet::show(ctx, crate::sheet::Spec::new("frames-c6", metrics::MODAL_W), &mut (), |ui, _| {
                    ui.label("SHEET");
                }, |_, _| {});
            }
            crate::layer::fade_out(ctx);
        });
        let a = card_alpha(&shapes);
        if open && i <= 10 {
            enter.push(a);
        } else if !open && a > 0.0 {
            out.push(a);
        }
    }
    (enter, out)
}

pub fn uncoloured() -> Vec<(f32, f32)> {
    let ctx = headless();
    let ph = Color32::PLACEHOLDER;
    let faded_placeholder = |c: Color32| {
        let f = c.a() as f32 / ph.a() as f32;
        c.a() > 0 && c != ph && c.b() == 0 && (c.r() as f32 - ph.r() as f32 * f).abs() <= 1.0 && (c.g() as f32 - ph.g() as f32 * f).abs() <= 1.0
    };
    let surface = c(C::Surface);
    let mut hint = String::new();
    let mut out = Vec::new();
    for i in 0..24 {
        let open = i < 12;
        let mut opacity = 0.0_f32;
        let run = ctx.run_ui(raw(i as f64 * STEP, crate::house::WINDOW_W, vec![]), |root| {
            let ctx = &root.ctx().clone();
            page(root, |_| {});
            if open {
                let _ = crate::sheet::show(ctx, crate::sheet::Spec::new("frames-fade", metrics::MODAL_W), &mut hint, |ui, hint| {
                    ui.label("PLAIN");
                    crate::input::line(ui, hint, "HINT");
                }, |_, _| {});
            }
            crate::layer::fade_out(ctx);
        });
        let mut flat = Vec::new();
        for cs in run.shapes.iter().cloned() {
            flatten(cs.shape, &mut flat);
        }
        for s in &flat {
            if let egui::Shape::Rect(r) = s {
                if (0.94..=1.01).contains(&(r.rect.width() / metrics::MODAL_W)) && r.fill.a() > 0 && r.fill.r() as f32 >= 0.9 * r.fill.a() as f32 {
                    opacity = opacity.max(r.fill.a() as f32 / surface.a() as f32);
                }
            }
        }
        if opacity <= 0.0 {
            continue;
        }
        let mut bad = 0_u32;
        for p in ctx.tessellate(run.shapes, run.pixels_per_point) {
            if let egui::epaint::Primitive::Mesh(m) = p.primitive {
                bad += m.vertices.iter().filter(|v| faded_placeholder(v.color)).count() as u32;
            }
        }
        out.push((opacity, bad as f32));
    }
    out
}

pub fn alert() -> (Vec<f32>, Vec<String>) {
    let ctx = headless();
    let mut t = crate::toast::Toasts::new();
    t.set_labels("Details", "Hide", "Close");
    let mut nums = Vec::new();
    let mut h = crate::hush::Hush::new();
    h.set(&mut t, true, 0.0);
    let _ = frame(&ctx, 0.0, crate::house::WINDOW_W, vec![], |root| {
        h.say(&mut t, "Backup is behind", crate::toast::Tone::Alert, 0.0);
        let _ = t.draw(root.ctx(), 0.0);
        nums.push(t.live() as f32);
    });
    nums.push(h.held() as f32);
    h.set(&mut t, false, STEP);
    let _ = frame(&ctx, STEP, crate::house::WINDOW_W, vec![], |root| {
        let _ = t.draw(root.ctx(), 0.0);
        nums.push(t.live() as f32);
    });
    let hex = |c: Color32| format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
    let dot = |tone: crate::toast::Tone| {
        let ctx = headless();
        let mut t = crate::toast::Toasts::new();
        t.say("Probe", tone, 0.0);
        let mut shapes = Vec::new();
        for at in [0.9, 1.0] {
            shapes = frame(&ctx, at, crate::house::WINDOW_W, vec![], |root| {
                let _ = t.draw(root.ctx(), 0.0);
            })
            .0;
        }
        shapes
            .iter()
            .find_map(|s| match s {
                egui::Shape::Circle(c) if c.fill.a() == 255 && c.radius > 8.0 => Some(hex(c.fill)),
                _ => None,
            })
            .unwrap_or_default()
    };
    let words = vec![dot(crate::toast::Tone::Alert), dot(crate::toast::Tone::Bad)];
    (nums, words)
}

pub fn hover_press() -> Vec<(&'static str, Vec<f32>)> {
    type Draw = Box<dyn Fn(&mut egui::Ui)>;
    let parts: Vec<(&'static str, Draw)> = vec![
        ("hp.key", Box::new(|ui| {
            crate::button::key(ui, "Save", crate::button::Role::Primary, true);
        })),
        ("hp.iconKey", Box::new(|ui| {
            crate::button::icon_key(ui, crate::icons::Glyph::Gear, 26.0, true, "");
        })),
        ("hp.input", Box::new(|ui| {
            let mut s = String::from("text");
            crate::input::line_w(ui, &mut s, "hint", 300.0);
        })),
        ("hp.switch", Box::new(|ui| {
            crate::toggle::switch(ui, true, true);
        })),
        ("hp.seg", Box::new(|ui| {
            crate::seg::seg(ui, "frames-seg", &["One".into(), "Two".into()], 0);
        })),
        ("hp.fold", Box::new(|ui| {
            crate::fold::fold(ui, "frames-fold", "More", |ui| ui.label("inside"));
        })),
        ("hp.tableRow", Box::new(|ui| {
            let cols = [crate::table::col("Name", crate::table::Col::Fr(1.0))];
            let rows = [crate::table::Row { cells: vec![crate::table::Cell::Text("row".into())], click: true, gone: false, on: false }];
            crate::table::table(ui, "frames-table", &cols, true, &rows, "none");
        })),
        ("hp.tile", Box::new(|ui| {
            crate::card::tile(ui, &crate::card::Tile { title: "Orders", figure: "3", mark: None, figure_colour: None, sub: "", mono_figure: false, clickable: true });
        })),
        ("hp.pin", Box::new(|ui| {
            let mut v = crate::secret::Secret::default();
            crate::pin::pin_row(ui, "frames-pin", &mut v, 8, None, true, false);
        })),
        ("hp.menuKey", Box::new(|ui| {
            crate::menu::menu_key(ui, "frames-menu", "Menu", true, 160.0, &[crate::menu::row("One")]);
        })),
    ];
    let mut out = Vec::new();
    for (name, draw) in parts {
        let ctx = headless();
        let mut rect = Rect::NOTHING;
        let mut counts = Vec::new();
        let run = |t: f64, events: Vec<egui::Event>, counts: &mut Vec<f32>, rect: &mut Rect| {
            let (shapes, _) = frame(&ctx, t, crate::house::WINDOW_W, events, |root| {
                page(root, |ui| {
                    let r = ui.scope(|ui| draw(ui)).response.rect;
                    *rect = r;
                })
            });
            counts.push(shapes.len() as f32);
        };
        run(0.0, vec![], &mut counts, &mut rect);
        let at = rect.center();
        run(STEP, vec![egui::Event::PointerMoved(at)], &mut counts, &mut rect);
        let press = egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() };
        run(2.0 * STEP, vec![press], &mut counts, &mut rect);
        out.push((name, counts));
    }
    out
}

pub fn idle() -> f32 {
    let ctx = headless();
    let mut again = true;
    for i in 0..61 {
        let (_, a) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            page(root, |ui| {
                crate::button::key(ui, "Save", crate::button::Role::Primary, true);
                crate::seg::seg(ui, "idle-seg", &["One".into(), "Two".into()], 0);
                crate::fold::fold(ui, "idle-fold", "More", |ui| ui.label("inside"));
                crate::toggle::switch(ui, true, true);
                crate::card::tile(ui, &crate::card::Tile { title: "Orders", figure: "3", mark: None, figure_colour: None, sub: "", mono_figure: false, clickable: true });
            })
        });
        again = a;
    }
    if again { 1.0 } else { 0.0 }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuOutside {
    pub guard_took: bool,
    pub under_pressed: bool,
}

pub fn menu_outside() -> MenuOutside {
    let ctx = headless();
    let at = egui::pos2(45.0, 25.0);
    let press = |pressed: bool| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    let steps = [vec![], vec![egui::Event::PointerMoved(at)], vec![press(true)], vec![press(false)], vec![]];
    let mut out = MenuOutside::default();
    for (i, ev) in steps.into_iter().enumerate() {
        let (mut under, mut guard) = (false, false);
        frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, ev, |root| {
            let ctx = &root.ctx().clone();
            page(root, |ui| {
                under = ui.put(Rect::from_min_size(egui::pos2(10.0, 10.0), vec2(70.0, 30.0)), egui::Button::new("UNDER")).clicked();
            });
            let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("frames-menu"));
            crate::layer::place(ctx, layer, None);
            crate::layer::over(ctx, layer).show(ctx, |ui| {
                guard = crate::layer::under(ui, crate::layer::Under::Guard);
                let card = Rect::from_min_size(egui::pos2(300.0, 10.0), vec2(120.0, 30.0));
                crate::layer::body(ui, card);
                ui.put(card, egui::Label::new("ITEM"));
            });
        });
        out.guard_took |= guard;
        out.under_pressed |= under;
    }
    out
}

pub fn esc_one() -> Vec<(&'static str, bool)> {
    let ctx = headless();
    let menu = egui::Id::new("frames-esc-menu");
    let esc = || vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }];
    let mut sheet_esc = Vec::new();
    for i in 0..8 {
        let ev = if i == 4 || i == 6 { esc() } else { vec![] };
        if i == 2 {
            crate::menu::set(&ctx, menu, true);
        }
        let mut seen = false;
        frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, ev, |root| {
            let ctx = &root.ctx().clone();
            page(root, |_| {});
            let out = crate::sheet::show(ctx, crate::sheet::Spec::new("frames-esc-sheet", metrics::MODAL_W), &mut (), |ui, _| {
                let anchor = Rect::from_min_size(ui.cursor().min, vec2(120.0, 30.0));
                let _ = crate::menu::show(ui.ctx(), menu, anchor, true, 120.0, &[crate::menu::row("ITEM")]);
            }, |_, _| ());
            seen = out.esc;
        });
        sheet_esc.push(seen);
    }
    vec![("menuClosed", !crate::menu::is_open(&ctx, menu)), ("sheetSawFirstEsc", sheet_esc[4]), ("sheetSawSecondEsc", sheet_esc[6])]
}

pub fn reopen() -> Vec<f32> {
    let surface = c(C::Surface);
    let card_alpha = |shapes: &[egui::Shape]| -> f32 {
        shapes
            .iter()
            .filter_map(|s| match s {
                egui::Shape::Rect(r) if (100.0..=200.0).contains(&r.rect.width()) && r.fill.a() > 0 && r.fill.r() as f32 >= 0.9 * r.fill.a() as f32 => Some(r.fill.a() as f32 / surface.a() as f32),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
    };
    let ctx = headless();
    let id = egui::Id::new("frames-reopen");
    let anchor = Rect::from_min_size(egui::pos2(40.0, 40.0), vec2(120.0, 30.0));
    let mut alpha = Vec::new();
    for i in 0..18 {
        if i == 0 || i == 13 {
            crate::menu::set(&ctx, id, true);
        }
        if i == 10 {
            crate::menu::set(&ctx, id, false);
        }
        let (shapes, _) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
            let ctx = &root.ctx().clone();
            page(root, |_| {});
            let _ = crate::menu::show(ctx, id, anchor, true, 120.0, &[crate::menu::row("REOPEN")]);
            crate::layer::fade_out(ctx);
        });
        alpha.push(card_alpha(&shapes));
    }
    let fresh = {
        let ctx = headless();
        let id = egui::Id::new("frames-fresh");
        crate::menu::set(&ctx, id, true);
        let (shapes, _) = frame(&ctx, 0.0, crate::house::WINDOW_W, vec![], |root| {
            let ctx = &root.ctx().clone();
            page(root, |_| {});
            let _ = crate::menu::show(ctx, id, anchor, true, 120.0, &[crate::menu::row("REOPEN")]);
        });
        card_alpha(&shapes)
    };
    vec![alpha[9], alpha[12], alpha[13], fresh]
}

pub fn flip_up() -> Vec<f32> {
    let ctx = headless();
    let mut out = Vec::new();
    for (tag, y) in [("FLIPLOW", crate::house::WINDOW_H - 40.0), ("FLIPHIGH", 40.0)] {
        let id = egui::Id::new(("frames-flip", tag));
        let anchor = Rect::from_min_size(egui::pos2(40.0, y), vec2(120.0, 30.0));
        crate::menu::set(&ctx, id, true);
        let mut top = 0.0;
        for i in 0..12 {
            let (shapes, _) = frame(&ctx, i as f64 * STEP, crate::house::WINDOW_W, vec![], |root| {
                let ctx = &root.ctx().clone();
                page(root, |_| {});
                let _ = crate::menu::show(ctx, id, anchor, true, 120.0, &[crate::menu::row(tag), crate::menu::row("B"), crate::menu::row("C")]);
            });
            top = text_at(&shapes, tag).map(|x| x.0.y).unwrap_or(f32::NAN);
        }
        crate::menu::set(&ctx, id, false);
        out.push(top - anchor.top());
    }
    out
}

pub fn read_all() -> Vec<(&'static str, Reading)> {
    let mut v: Vec<(&'static str, Reading)> = Vec::new();
    v.push(("c1Points", Reading::Nums(same_path())));
    v.push(("c2Width", Reading::Nums(scroll_width())));
    v.push(("c4Top", Reading::Nums(discard())));
    let (a, w) = title();
    v.push(("c5Alpha", Reading::Nums(a)));
    v.push(("c5Words", Reading::Words(w)));
    let (i, o) = leave();
    v.push(("c6In", Reading::Nums(i)));
    v.push(("c6Out", Reading::Nums(o)));
    let u = uncoloured();
    v.push(("fadeAlpha", Reading::Nums(u.iter().map(|x| x.0).collect())));
    v.push(("fadePlaceholder", Reading::Nums(u.iter().map(|x| x.1).collect())));
    let (n, c) = alert();
    v.push(("c7Counts", Reading::Nums(n)));
    v.push(("c7Dots", Reading::Words(c)));
    for (name, counts) in hover_press() {
        v.push((name, Reading::Nums(counts)));
    }
    v.push(("railSlide", Reading::Nums(rail_slide())));
    let m = menu_outside();
    v.push(("menuOutside", Reading::Flags(vec![("guardTook", m.guard_took), ("underPressed", m.under_pressed)])));
    v.push(("idleAgain", Reading::Nums(vec![idle()])));
    v.push(("escOne", Reading::Flags(esc_one())));
    v.push(("reopen", Reading::Nums(reopen())));
    v.push(("flipUp", Reading::Nums(flip_up())));
    v
}
