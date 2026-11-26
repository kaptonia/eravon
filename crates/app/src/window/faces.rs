use super::*;

impl Win {
    pub(super) fn landing_ok(given: &str) -> bool {
        crate::home::landing(given).is_ok()
    }

}

pub(super) fn appearance_label(a: &str) -> &'static str {
    match a {
        "dark" => t(Key::AppearanceDark),
        "system" => t(Key::ZoneSystem),
        _ => t(Key::AppearanceLight),
    }
}

pub(super) fn cli_anchor_label(c: crate::machine::CliAnchor) -> Key {
    match c {
        crate::machine::CliAnchor::Send => Key::CliAnchorSend,
        crate::machine::CliAnchor::Queue => Key::CliAnchorQueue,
    }
}

pub(super) fn joined(words: &[crate::secret::Secret]) -> crate::secret::Secret {
    let mut out = crate::secret::Secret::new();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(w.expose().trim());
    }
    out
}

pub(super) fn id_seat_key(r: crate::roles::Role) -> Key {
    match r {
        crate::roles::Role::Author => Key::IdSeatAuthor,
        crate::roles::Role::Grantee => Key::IdSeatGrantee,
    }
}

pub(super) fn id_seat_row_key(r: crate::roles::Role) -> Key {
    match r {
        crate::roles::Role::Author => Key::IdSeatRowAuthor,
        crate::roles::Role::Grantee => Key::IdSeatRowGrantee,
    }
}

pub(super) fn id_kind_key(k: crate::identity::Kind) -> Key {
    match k {
        crate::identity::Kind::Words => Key::IdKindWords,
        crate::identity::Kind::Existing => Key::IdKindExisting,
    }
}

pub const ARGS_LINE: &str = "E_ARGS the window takes no arguments";

pub fn refuse_arguments() -> i32 {
    speak_this_machines_language();
    without_window(ARGS_LINE, t(Key::SaidNoWindowArgs).to_string());
    2
}

pub(super) fn without_window(line: &str, sentence: String) {
    crate::platform::say_without_window(line, &sentence);
}

pub(super) fn speak_this_machines_language() {
    if let Some(l) = crate::machine::read_face().ok().and_then(|m| m.lang) {
        crate::lang::set(l);
    }
}

pub(super) fn fresh_win(shell: Shell, typed: Typed, ux: Ux) -> Win {
    Win {
        shell,
        toasts: Toasts::new(),
        typed,
        reaped: None,
        faults_told: 0,
        ux,
        asker: crate::platform::ask_path,
        hx: house::Hx::default(),
        asks_which: crate::platform::asks_file_or_folder(),
        backdrop: None,
        asks_motion: false,
        reads_behind: false,
        reduced: false,
        motion_asked: None,
        painted_lang: crate::lang::lang(),
        drops: None,
    }
}

pub struct UnseenProbe {
    win: Win,
    t: f64,
    frame: eframe::Frame,
}

impl UnseenProbe {
    pub fn new(shell: Shell) -> UnseenProbe {
        let mut ux = Ux::default();
        ux.wizard_asked = true;
        let mut win = fresh_win(shell, Typed::default(), ux);
        win.faults_told = usize::MAX;
        UnseenProbe { win, t: 0.0, frame: eframe::Frame::_new_kittest() }
    }

    pub fn pass(&mut self, ctx: &egui::Context, step: f64) {
        self.t += step;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1180.0, 760.0));
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(self.t), ..Default::default() };
        let (win, frame) = (&mut self.win, &mut self.frame);
        let _ = ctx.run_ui(input, |ui| eframe::App::logic(win, ui.ctx(), frame));
    }

    pub fn pass_wakes(&mut self, ctx: &egui::Context, step: f64) -> std::time::Duration {
        self.t += step;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(super::W, super::H));
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(self.t), ..Default::default() };
        let (win, frame) = (&mut self.win, &mut self.frame);
        let out = ctx.run_ui(input, |ui| eframe::App::logic(win, ui.ctx(), frame));
        out.viewport_output.get(&egui::ViewportId::ROOT).map_or(std::time::Duration::MAX, |v| v.repaint_delay)
    }

    pub fn drawn(&mut self, ctx: &egui::Context, step: f64) {
        self.t += step;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1180.0, 760.0));
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(self.t), ..Default::default() };
        let win = &mut self.win;
        let _ = ctx.run_ui(input, |ui| win.draw(ui));
    }

    pub fn shell(&mut self) -> &mut Shell {
        &mut self.win.shell
    }
}

pub struct PathFrame {
    pub ask: Vec<(&'static str, crate::platform::Pick)>,
    pub take: Vec<&'static str>,
    pub answer: bool,
}

pub struct PathRead {
    pub took: Vec<(&'static str, Option<String>)>,
    pub faults: Vec<crate::fault::Fault>,
    pub open: bool,
}

pub fn probe_paths(ctx: &egui::Context, shell: Shell, asker: crate::platform::Asker, answer: &mut dyn FnMut(), frames: Vec<PathFrame>) -> (Shell, Vec<PathRead>) {
    let mut ux = Ux::default();
    ux.wizard_asked = true;
    let mut win = fresh_win(shell, Typed::default(), ux);
    win.asker = asker;
    win.faults_told = usize::MAX;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(W, H));
    let site = |n: &str| egui::Id::new(("eravon-probe-path", n));
    let mut t = 0.0;
    let mut out = Vec::new();
    for f in frames {
        if f.answer {
            answer();
            while win.shell.tasks.in_flight(crate::task::Kind::Path) && !win.shell.tasks.finished_in_flight(crate::task::Kind::Path) {
                std::thread::yield_now();
            }
        }
        t += 0.1;
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(t), ..Default::default() };
        let mut took = Vec::new();
        let _ = ctx.run_ui(input, |ui| {
            let c = &ui.ctx().clone();
            if c.current_pass_index() == 0 {
                for (n, kind) in &f.ask {
                    let _ = path_answer(c, site(n), true, *kind);
                }
            }
            win.draw(ui);
        });
        let input = egui::RawInput { screen_rect: Some(screen), time: Some(t + 0.05), ..Default::default() };
        t += 0.05;
        let _ = ctx.run_ui(input, |ui| {
            let c = &ui.ctx().clone();
            win.draw(ui);
            if c.current_pass_index() == 0 {
                for n in &f.take {
                    took.push((*n, path_answer(c, site(n), false, crate::platform::Pick::File)));
                }
            }
        });
        out.push(PathRead { took, faults: win.shell.faults.clone(), open: win.shell.tasks.in_flight(crate::task::Kind::Path) });
    }
    (win.shell, out)
}

pub struct Drawn {
    pub text: String,
    pub rect: egui::Rect,
    pub rows: usize,
    pub row_chars: Vec<usize>,
    pub baseline: f32,
}

pub struct WizFrame {
    pub step: String,
    pub texts: Vec<Drawn>,
    pub check: usize,
}

pub(super) fn check_points(o: &egui::FullOutput) -> usize {
    let ok = c(C::Ok);
    let mut most = 0usize;
    let mut look = |s: &egui::Shape| {
        if let egui::Shape::Path(p) = s {
            if !p.closed && p.stroke.color == egui::epaint::ColorMode::Solid(ok) {
                most = most.max(p.points.len());
            }
        }
    };
    for cl in &o.shapes {
        match &cl.shape {
            egui::Shape::Vec(v) => v.iter().for_each(&mut look),
            s => look(s),
        }
    }
    most
}

pub(super) fn texts_of(out: &egui::FullOutput) -> Vec<Drawn> {
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(s) => Some(Drawn {
                text: s.galley.job.text.clone(),
                rect: s.visual_bounding_rect(),
                rows: s.galley.rows.len(),
                row_chars: s.galley.rows.iter().map(|r| r.glyphs.len()).collect(),
                baseline: s.pos.y + s.galley.rows.first().and_then(|r| r.glyphs.first()).map(|g| g.pos.y).unwrap_or(0.0),
            }),
            _ => None,
        })
        .collect()
}

pub struct WizardExit {
    pub before: String,
    pub opened: bool,
    pub after: String,
    pub offered: bool,
    pub still_open: bool,
    pub asked: bool,
    pub asked_drawn: bool,
    pub words_dropped: bool,
    pub wiz_from: String,
    pub place_after: String,
}

#[derive(Clone, Debug, Default)]
pub struct WizardBack {
    pub pressed_on_check: bool,
    pub step_after_check: String,
    pub checking_after: bool,
    pub same_words: bool,
    pub pressed_on_words: bool,
    pub step_after_words: String,
}

#[derive(Clone, Debug, Default)]
pub struct LangPick {
    pub drawn: bool,
    pub before: String,
    pub after: String,
    pub machine_after: String,
    pub still_up: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SegTab {
    pub pressed: bool,
    pub entry_before: u64,
    pub entry_after: u64,
    pub route: String,
    pub page: String,
    pub history_empty: bool,
    pub scroll_before: f32,
    pub scroll_after: f32,
}
#[derive(Clone, Debug, Default)]
pub struct TypedUnder {
    pub bare: usize,
    pub sheet_up: bool,
    pub pressed: bool,
    pub under: usize,
    pub locked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct WizPinTyped {
    pub first: Vec<usize>,
    pub asked_again: bool,
    pub second: Vec<usize>,
    pub vault: Option<crate::shell::Vault>,
    pub next_drawn: bool,
    pub after: Option<crate::nav::Step>,
    pub cover_lang: Option<(crate::lang::Lang, crate::lang::Lang)>,
}

#[derive(Clone, Debug, Default)]
pub struct IdMenuPress {
    pub pressed: bool,
    pub wish: Option<String>,
    pub flying: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct GateAside {
    pub key_drawn: bool,
    pub sheet: Vec<String>,
    pub confirmed: bool,
    pub vault: Option<crate::shell::Vault>,
    pub wizard: Option<crate::nav::Step>,
    pub trouble: Option<crate::fault::Fault>,
}

#[derive(Clone, Debug, Default)]
pub struct GateReset {
    pub pressed: bool,
    pub vault: Option<crate::shell::Vault>,
    pub wizard: Option<crate::nav::Step>,
}

#[derive(Clone, Debug)]
pub struct GateColumn {
    pub window_h: f32,
    pub group_top: f32,
    pub title_ink_bottom: f32,
    pub cells: egui::Rect,
    pub said: Option<egui::Rect>,
    pub forgot: egui::Rect,
    pub own_key: egui::Rect,
}

#[derive(Clone, Debug, Default)]
pub struct RailWalk {
    pub item: String,
    pub pressed: bool,
    pub entered: String,
    pub link: Option<String>,
    pub detail: Option<String>,
    pub out: Option<String>,
    pub rail_again: Option<String>,
    pub from_elsewhere: Option<String>,
    pub forward: Option<String>,
}

pub(super) fn size_say(n: u64) -> String {
    let one = |x: u64, unit: u64| {
        let tenths = (x * 10 + unit / 2) / unit;
        if tenths % 10 == 0 { (tenths / 10).to_string() } else { format!("{}.{}", tenths / 10, tenths % 10) }
    };
    if n < 1000 {
        fill1(Key::SizeBytes, &n.to_string())
    } else if (n + 500) / 1000 < 1000 {
        fill1(Key::SizeKb, &((n + 500) / 1000).to_string())
    } else if n < 999_950_000 {
        fill1(Key::SizeMb, &one(n, 1_000_000))
    } else {
        fill1(Key::SizeGb, &one(n, 1_000_000_000))
    }
}

pub(super) fn zone_label(z: crate::when::Zone) -> &'static str {
    match z {
        crate::when::Zone::Utc => t(Key::ZoneUtc),
        crate::when::Zone::System => t(Key::ZoneSystem),
    }
}

