use super::*;

pub fn probe_wake(ctx: &egui::Context, shell: Shell, place: Place, start: f64, frames: usize, step: f64, width: f32, height: f32) -> (Shell, Option<f64>) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(place, role));
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut delay: Option<f64> = None;
    for k in 0..frames.max(1) {
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(start + step * k as f64), ..Default::default() };
        let out = ctx.run_ui(input, |ui| win.draw(ui));
        delay = out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.repaint_delay).filter(|d| *d < std::time::Duration::MAX).map(|d| d.as_secs_f64());
    }
    (win.shell, delay)
}

pub fn probe_frames(ctx: &egui::Context, shell: Shell, place: Place, wizard: Option<crate::nav::Step>, width: f32, height: f32, frames: usize) -> (Shell, Vec<egui::FullOutput>) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(place, role));
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    if let Some(step) = wizard {
        win.wizard_open(step);
    }
    let mut outs = Vec::new();
    for i in 0..frames.max(1) {
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(0.1 * (i + 1) as f64), ..Default::default() };
        outs.push(ctx.run_ui(input, |ui| win.draw(ui)));
    }
    (win.shell, outs)
}

pub fn probe_proxy_section(ctx: &egui::Context, shell: Shell, width: f32, height: f32) -> (Shell, Vec<String>) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Settings(crate::nav::Section::Network), role));
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut out = egui::FullOutput::default();
    for _ in 0..12 {
        clock += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), ..Default::default() };
        out = ctx.run_ui(input, |ui| win.draw(ui));
    }
    let texts = texts_of(&out).into_iter().map(|d| d.text).collect();
    (win.shell, texts)
}

#[derive(Clone, Debug)]
pub enum NetPress {
    AddRpc(String),
    AddRest(String, String),
    AtRowEnd(String),
}

pub fn probe_network_press(ctx: &egui::Context, shell: Shell, press: NetPress, width: f32, height: f32) -> (Shell, Vec<String>, Vec<String>) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Settings(crate::nav::Section::Network), role));
    ux.wizard_asked = true;
    let mut typed = Typed::default();
    match &press {
        NetPress::AddRpc(text) => typed.rpc_new = text.clone(),
        NetPress::AddRest(base, key) => {
            typed.rest_base = base.clone();
            typed.rest_key = crate::secret::Secret::of(key);
        }
        NetPress::AtRowEnd(_) => {}
    }
    let mut win = fresh_win(shell, typed, ux);
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut run = |win: &mut Win, events: Vec<egui::Event>| -> egui::FullOutput {
        clock += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), events, ..Default::default() };
        ctx.run_ui(input, |ui| win.draw(ui))
    };
    let mut out = run(&mut win, Vec::new());
    for _ in 0..8 {
        out = run(&mut win, Vec::new());
    }
    let drawn = texts_of(&out);
    let before: Vec<String> = drawn.iter().map(|d| d.text.clone()).collect();
    let mut adds: Vec<egui::Rect> = drawn.iter().filter(|d| d.text == t(Key::ReadNetAdd)).map(|d| d.rect).collect();
    adds.sort_by(|a, b| a.top().total_cmp(&b.top()));
    let row_end = |words: &str| -> Option<egui::Pos2> {
        let r = drawn.iter().find(|d| d.text == words)?.rect;
        let keys = ctx.viewport(|v| v.prev_pass.widgets.layers().flat_map(|(_, ws)| ws.iter().filter(|w| w.sense.senses_click() && w.rect.width() < 80.0 && w.rect.height() < 60.0).map(|w| w.rect).collect::<Vec<_>>()).collect::<Vec<_>>());
        keys.into_iter().filter(|k| k.top() <= r.center().y && k.bottom() >= r.center().y && k.left() > r.left()).max_by(|a, b| a.left().total_cmp(&b.left())).map(|k| k.center())
    };
    let at = match &press {
        NetPress::AddRpc(_) => adds.first().map(|r| r.center()),
        NetPress::AddRest(..) => drawn.iter().find(|d| d.text == t(Key::NetRestKey)).and_then(|k| adds.iter().find(|r| r.top() > k.rect.bottom())).map(|r| r.center()),
        NetPress::AtRowEnd(words) => row_end(words),
    };
    if let Some(at) = at {
        let click = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        let _ = run(&mut win, vec![egui::Event::PointerMoved(at), click(true)]);
        let _ = run(&mut win, vec![click(false)]);
    }
    for _ in 0..6 {
        out = run(&mut win, Vec::new());
    }
    let after = texts_of(&out).into_iter().map(|d| d.text).collect();
    (win.shell, before, after)
}

pub fn probe_proxy_cells(ctx: &egui::Context, shell: Shell, cells: crate::proxycells::Cells, width: f32, height: f32) -> (Shell, crate::proxycells::Cells, Vec<String>) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Settings(crate::nav::Section::Network), role));
    ux.wizard_asked = true;
    ux.proxy_manual = true;
    ux.proxy_seeded = true;
    let typed = Typed { proxy: cells, ..Typed::default() };
    let mut win = fresh_win(shell, typed, ux);
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut out = egui::FullOutput::default();
    for _ in 0..8 {
        clock += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), ..Default::default() };
        out = ctx.run_ui(input, |ui| win.draw(ui));
    }
    let texts = texts_of(&out).into_iter().map(|d| d.text).collect();
    let after = win.typed.proxy.clone();
    (win.shell, after, texts)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostInput {
    Typed,
    Pasted,
    PastedByChar,
}

pub fn probe_proxy_typed(ctx: &egui::Context, shell: Shell, cells: crate::proxycells::Cells, text: &str, how: HostInput, width: f32, height: f32) -> (Shell, Vec<crate::proxycells::Cells>, crate::proxycells::Cells) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Settings(crate::nav::Section::Network), role));
    ux.wizard_asked = true;
    ux.proxy_manual = true;
    ux.proxy_seeded = true;
    let typed = Typed { proxy: cells, ..Typed::default() };
    let mut win = fresh_win(shell, typed, ux);
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut run = |win: &mut Win, events: Vec<egui::Event>| -> egui::FullOutput {
        clock += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), events, ..Default::default() };
        ctx.run_ui(input, |ui| win.draw(ui))
    };
    let mut out = run(&mut win, Vec::new());
    for _ in 0..6 {
        out = run(&mut win, Vec::new());
    }
    let hint: String = t(Key::ProxyHint).chars().take(4).collect();
    let Some(at) = texts_of(&out).iter().find(|d| d.text.starts_with(&hint)).map(|d| d.rect.center()) else {
        let now = win.typed.proxy.clone();
        return (win.shell, Vec::new(), now);
    };
    let press = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    let _ = run(&mut win, vec![egui::Event::PointerMoved(at), press(true), press(false)]);
    let _ = run(&mut win, Vec::new());
    let mut steps = Vec::new();
    match how {
        HostInput::Typed => {
            for ch in text.chars() {
                let _ = run(&mut win, vec![egui::Event::Text(ch.to_string())]);
                steps.push(win.typed.proxy.clone());
            }
        }
        HostInput::PastedByChar => {
            for ch in text.chars() {
                let _ = run(&mut win, vec![egui::Event::Paste(ch.to_string())]);
                steps.push(win.typed.proxy.clone());
            }
        }
        HostInput::Pasted => {
            let _ = run(&mut win, vec![egui::Event::Paste(text.to_string())]);
            steps.push(win.typed.proxy.clone());
        }
    }
    let tab = egui::Event::Key { key: egui::Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
    let _ = run(&mut win, vec![tab]);
    for _ in 0..4 {
        let _ = run(&mut win, Vec::new());
    }
    let after = win.typed.proxy.clone();
    (win.shell, steps, after)
}

pub fn probe_rail_walk(ctx: &egui::Context, shell: Shell, width: f32, height: f32) -> (Shell, Vec<RailWalk>) {
    let role = shell.settings.role;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut run = |win: &mut Win, events: Vec<egui::Event>| -> egui::FullOutput {
        clock += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), events, modifiers: egui::Modifiers::NONE, ..Default::default() };
        ctx.run_ui(input, |ui| win.draw(ui))
    };
    let settle = |win: &mut Win, run: &mut dyn FnMut(&mut Win, Vec<egui::Event>) -> egui::FullOutput| {
        let mut out = run(win, Vec::new());
        for _ in 0..5 {
            out = run(win, Vec::new());
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while win.shell.house_busy() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(2));
            out = run(win, Vec::new());
        }
        out
    };
    let click = |win: &mut Win, run: &mut dyn FnMut(&mut Win, Vec<egui::Event>) -> egui::FullOutput, at: egui::Pos2| {
        let press = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        let _ = run(win, vec![egui::Event::PointerMoved(at)]);
        let _ = run(win, vec![press(true), press(false)]);
        let _ = run(win, vec![egui::Event::PointerGone]);
    };
    let labels: Vec<String> = crate::nav::rail(role).iter().flat_map(|g| g.items.iter().map(|i| t(i.name).to_string())).collect();
    let mut shell = shell;
    let mut walk = Vec::new();
    for label in labels.clone() {
        let mut ux = Ux::default();
        ux.nav.land(crate::nav::history_of(Place::Home, role));
        ux.wizard_asked = true;
        let mut win = fresh_win(shell, Typed::default(), ux);
        win.faults_told = usize::MAX;
        let mut w = RailWalk { item: label.clone(), ..Default::default() };
        let on_rail = |out: &egui::FullOutput| texts_of(out).iter().find(|d| d.text == label && d.rect.center().x < tk::RAIL_W).map(|d| d.rect.center());
        let out = settle(&mut win, &mut run);
        if let Some(item) = on_rail(&out) {
            w.pressed = true;
            click(&mut win, &mut run, item);
            let _ = settle(&mut win, &mut run);
            w.entered = eravon_ui::probe::route_drawn(ctx);
            let view = win.ux.nav.stack;
            if let Some(row) = eravon_ui::probe::rows(ctx).first().map(|r| r.center()) {
                click(&mut win, &mut run, row);
                let _ = settle(&mut win, &mut run);
                let there = eravon_ui::probe::route_drawn(ctx);
                if win.ux.nav.stack != view {
                    w.link = Some(there);
                } else {
                    w.detail = Some(there);
                    let key = egui::Event::Key { key: egui::Key::OpenBracket, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
                    let _ = run(&mut win, vec![key]);
                    let _ = settle(&mut win, &mut run);
                    w.out = Some(eravon_ui::probe::route_drawn(ctx));
                    if let Some(row) = eravon_ui::probe::rows(ctx).first().map(|r| r.center()) {
                        click(&mut win, &mut run, row);
                        let out = settle(&mut win, &mut run);
                        if let Some(item) = on_rail(&out) {
                            click(&mut win, &mut run, item);
                            let _ = settle(&mut win, &mut run);
                            w.rail_again = Some(eravon_ui::probe::route_drawn(ctx));
                        }
                        if let Some(row) = eravon_ui::probe::rows(ctx).first().map(|r| r.center()) {
                            click(&mut win, &mut run, row);
                            let out = settle(&mut win, &mut run);
                            let other = labels.iter().find(|l| **l != label).and_then(|o| texts_of(&out).iter().find(|d| d.text == *o && d.rect.center().x < tk::RAIL_W).map(|d| d.rect.center()));
                            if let Some(other) = other {
                                click(&mut win, &mut run, other);
                                let out = settle(&mut win, &mut run);
                                if let Some(item) = on_rail(&out) {
                                    click(&mut win, &mut run, item);
                                    let _ = settle(&mut win, &mut run);
                                    w.from_elsewhere = Some(eravon_ui::probe::route_drawn(ctx));
                                    let key = egui::Event::Key { key: egui::Key::CloseBracket, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
                                    let _ = run(&mut win, vec![key]);
                                    let _ = settle(&mut win, &mut run);
                                    w.forward = Some(eravon_ui::probe::route_drawn(ctx));
                                }
                            }
                        }
                    }
                }
            }
        }
        walk.push(w);
        shell = win.shell;
    }
    (shell, walk)
}

#[allow(clippy::too_many_arguments)]
pub fn probe_house_seg_tab(ctx: &egui::Context, shell: Shell, seat: crate::house::shell::Seat, page: crate::house::seller::Page, words: &str, scroll: f32, width: f32, height: f32) -> (Shell, SegTab) {
    use crate::house::action::Action as Step;
    let mut win = fresh_win(shell, Typed::default(), Ux { wizard_asked: true, ..Ux::default() });
    win.faults_told = usize::MAX;
    win.step(Step::Seat, &[("to", seat.name())]);
    win.step(Step::Page, &[("to", page.name())]);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut run = |win: &mut Win, events: Vec<egui::Event>, frames: usize| -> egui::FullOutput {
        let mut out = egui::FullOutput::default();
        for k in 0..frames {
            clock += 0.1;
            let ev = if k == 0 { events.clone() } else { Vec::new() };
            out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), time: Some(clock), events: ev, ..Default::default() }, |ui| win.draw(ui));
        }
        out
    };
    let _ = run(&mut win, Vec::new(), 10);
    for _ in 0..400 {
        if !win.shell.house_busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        let _ = run(&mut win, Vec::new(), 1);
    }
    let _ = run(&mut win, Vec::new(), 10);
    let mut r = SegTab::default();
    let mid = egui::pos2(width * 0.6, height * 0.6);
    let wheel = vec![egui::Event::PointerMoved(mid), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -scroll), phase: egui::TouchPhase::Move, modifiers: egui::Modifiers::NONE }];
    let out = run(&mut win, wheel, 10);
    r.scroll_before = win.ux.nav.scroll_y;
    r.entry_before = win.ux.nav.entry_key;
    if let Some(at) = texts_of(&out).into_iter().find(|d| d.text == words).map(|d| d.rect.center()) {
        let press = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        let _ = run(&mut win, vec![egui::Event::PointerMoved(at), press(true), press(false)], 10);
        r.pressed = true;
    }
    r.entry_after = win.ux.nav.entry_key;
    r.scroll_after = win.ux.nav.scroll_y;
    r.route = win.route().name();
    r.page = win.shell.desk.sh.seller.page().map(|p| p.name().to_string()).unwrap_or_default();
    let h = win.hist();
    r.history_empty = h.back.is_empty() && h.fwd.is_empty();
    (win.shell, r)
}

#[derive(Clone, Debug, Default)]
pub struct PickWhich {
    pub asked: bool,
    pub texts: Vec<String>,
    pub opened: Vec<crate::platform::Pick>,
}

static PICKED: std::sync::Mutex<Vec<crate::platform::Pick>> = std::sync::Mutex::new(Vec::new());

fn pick_recorder(kind: crate::platform::Pick) -> Result<crate::platform::Wait, crate::fault::Fault> {
    if let Ok(mut v) = PICKED.lock() {
        v.push(kind);
    }
    Ok(Box::new(|| Ok(None)))
}

pub fn probe_pick_which(ctx: &egui::Context, shell: Shell, asks: bool, folder: bool, width: f32, height: f32) -> (Shell, PickWhich) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Home, role));
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    win.faults_told = usize::MAX;
    win.asker = pick_recorder;
    win.asks_which = asks;
    if let Ok(mut v) = PICKED.lock() {
        v.clear();
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut run = |win: &mut Win, events: Vec<egui::Event>| -> egui::FullOutput {
        clock += 0.2;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), events, ..Default::default() };
        ctx.run_ui(input, |ui| win.draw(ui))
    };
    let _ = run(&mut win, Vec::new());
    let _ = path_answer(ctx, egui::Id::new("probe-pick-which"), true, crate::platform::Pick::FileOrFolder);
    let mut out = run(&mut win, Vec::new());
    for _ in 0..4 {
        out = run(&mut win, Vec::new());
    }
    let mut r = PickWhich { asked: win.ux.pick_which.is_some(), texts: texts_of(&out).into_iter().map(|d| d.text).collect(), opened: Vec::new() };
    if r.asked {
        let want = t(if folder { Key::PickFolder } else { Key::PickFile });
        if let Some(at) = texts_of(&out).iter().rev().find(|d| d.text == want).map(|d| d.rect.center()) {
            let press = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
            let _ = run(&mut win, vec![egui::Event::PointerMoved(at)]);
            let _ = run(&mut win, vec![press(true), press(false)]);
            for _ in 0..3 {
                let _ = run(&mut win, Vec::new());
            }
        }
    }
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while dialog_open(&win.shell) && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _ = run(&mut win, Vec::new());
    }
    r.opened = PICKED.lock().map(|v| v.clone()).unwrap_or_default();
    (win.shell, r)
}

pub fn probe_drop_untaken(ctx: &egui::Context, shell: Shell, over_sheet: bool, paths: &[String], width: f32, height: f32) -> (Shell, Option<String>) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Home, role));
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    win.faults_told = usize::MAX;
    if over_sheet {
        win.ux.bk = Some(super::backup::Bk::Peek);
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut run = |win: &mut Win, dropped: Vec<egui::DroppedFile>| {
        clock += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), dropped_files: dropped, ..Default::default() };
        let _ = ctx.run_ui(input, |ui| win.draw(ui));
    };
    for _ in 0..3 {
        run(&mut win, Vec::new());
    }
    run(&mut win, paths.iter().map(|p| egui::DroppedFile { path: Some(p.into()), ..Default::default() }).collect());
    for _ in 0..3 {
        run(&mut win, Vec::new());
    }
    let toast = win.toasts.showing().map(str::to_string);
    (win.shell, toast)
}

pub fn probe_feed_drops(hovering: bool, dropped: Vec<std::path::PathBuf>, already: Vec<std::path::PathBuf>) -> (usize, Vec<std::path::PathBuf>) {
    let mut raw = egui::RawInput { dropped_files: already.into_iter().map(|p| egui::DroppedFile { path: Some(p), ..Default::default() }).collect(), ..Default::default() };
    super::feed_drops(&mut raw, hovering, dropped);
    (raw.hovered_files.len(), raw.dropped_files.into_iter().filter_map(|f| f.path).collect())
}

pub fn probe_close_with_drops(shell: Shell, w: crate::platform::drops::Watcher) -> (Shell, bool) {
    let mut win = fresh_win(shell, Typed::default(), Ux::default());
    win.drops = Some(w);
    win.drops_let_go();
    let gone = win.drops.is_none();
    (win.shell, gone)
}

pub fn probe_each_sheet_is_one(shell: Shell) -> (Shell, Vec<(&'static str, bool)>) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Home, role));
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    let mut out = Vec::new();
    let sets: [(&'static str, fn(&mut Win)); 4] = [
        ("backup", |w| w.ux.bk = Some(super::backup::Bk::Export)),
        ("wizardLeave", |w| w.ux.wiz_exit_ask = true),
        ("commandLine", |w| w.ux.cli_sheet = true),
        ("pickWhich", |w| w.ux.pick_which = Some(egui::Id::new("probe-pick-which"))),
    ];
    for (name, set) in sets {
        set(&mut win);
        out.push((name, win.any_sheet()));
        win.sheets_clear();
        win.ux.wiz_exit_ask = false;
    }
    (win.shell, out)
}

pub fn probe_pin_moves_on(ctx: &egui::Context, shell: Shell, old: &str, new: &str, width: f32, height: f32) -> (Shell, (bool, bool)) {
    let role = shell.settings.role;
    let mut ux = Ux::default();
    ux.nav.land(crate::nav::history_of(Place::Home, role));
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
    let mut clock = 0.0;
    let mut run = |win: &mut Win, events: Vec<egui::Event>| -> egui::FullOutput {
        clock += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(clock), events, ..Default::default() };
        ctx.run_ui(input, |ui| win.draw(ui))
    };
    win.id_layer_open(IdModal::Pin);
    for _ in 0..6 {
        let _ = run(&mut win, Vec::new());
    }
    for ch in old.chars() {
        let _ = run(&mut win, vec![egui::Event::Text(ch.to_string())]);
    }
    for _ in 0..3 {
        let _ = run(&mut win, Vec::new());
    }
    let at_new = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("eravon-id-pin-past-old"))).unwrap_or(false) && win.ux.pin_again.is_empty();
    for ch in new.chars() {
        let _ = run(&mut win, vec![egui::Event::Text(ch.to_string())]);
    }
    for _ in 0..3 {
        let _ = run(&mut win, Vec::new());
    }
    let at_again = !win.ux.pin_again.is_empty();
    (win.shell, (at_new, at_again))
}
