use crate::action::{apply, Action, Applied};
use backup::{Bk, BkStep, RestoreFrom};
use crate::lang::{fill1, fill2, t, Key};
use crate::nav::{History, Place, Route, Section, Stack};
use crate::shell::{build_kind, Page, Shell};
use crate::task::Done;
use crate::trace::{self, Sink};
use eravon_ui::button::{self as key, Phase, Role};
use eravon_ui::egui;
use eravon_ui::icons::Glyph;
use eravon_ui::kv::Val;
use eravon_ui::mark::Mark;
use eravon_ui::palette::{c, Tone as PillTone, C};
use eravon_ui::toast::{Toasts, Tone};
use eravon_ui::tokens::{self as tk, Type};
use eravon_ui::{card, datepick, drop, fields, fold, full, input, kv, mark, menu, motion, page, paint, pin, rail, seg, sheet, skin, states, table, toggle, width};

pub const W: f32 = 1180.0;
pub const H: f32 = 760.0;
pub const MIN_W: f32 = eravon_ui::tokens::RAIL_W + 2.0 * eravon_ui::tokens::PAGE_PAD + eravon_ui::tokens::MAIN_MIN_W;
pub const MIN_H: f32 = 560.0;

#[derive(Default)]
struct Typed {
    proxy: crate::proxycells::Cells,
    home: String,
    rpc_new: String,
    rest_base: String,
    rest_key: crate::secret::Secret,
}

struct Win {
    shell: Shell,
    toasts: Toasts,
    typed: Typed,
    reaped: Option<crate::task::Reaped>,
    faults_told: usize,
    ux: Ux,
    asker: crate::platform::Asker,
    hx: house::Hx,
    asks_which: bool,
    backdrop: Option<(eravon_os::Window, bool, bool)>,
    asks_motion: bool,
    reads_behind: bool,
    reduced: bool,
    motion_asked: Option<f64>,
    painted_lang: crate::lang::Lang,
    drops: Option<crate::platform::drops::Watcher>,
}

impl Win {
    fn draw(&mut self, ui: &mut egui::Ui) {
        self.turn(ui.ctx());
        self.paint(ui);
    }

    fn turn(&mut self, ctx: &egui::Context) {
        let _logic = eravon_ui::motion::Logic::begin(ctx);
        let _road = crate::reads::Road::begin();
        let now = eravon_ui::motion::now(ctx);
        self.land(ctx, now);
        for a in std::mem::take(&mut self.shell.door_told) {
            self.told(a, None, now);
        }
        if ctx.input(|i| !i.events.is_empty() || i.pointer.delta() != egui::Vec2::ZERO) {
            self.ux.last_input = now;
        }
        if self.shell.idle_tick(self.ux.last_input, now) {
            self.ux.gate_clear();
        }
        if self.shell.unlocked() {
            let due = self.ux.last_input + self.shell.machine.auto_lock_secs as f64 - now;
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(due.max(0.5)));
        }
        let quiet = self.ux.wizard.is_some();
        if self.ux.wizard_asked && !quiet {
            if let Some(was) = crate::home::moved_unsaid() {
                self.toasts.say(fill1(Key::MachineMovedFrom, &was.display().to_string()), Tone::Note, now);
            }
        }
        self.own_turn(ctx, now);
        self.keep_draining(ctx);
    }

    fn keep_draining(&self, ctx: &egui::Context) {
        if self.shell.tasks.polled() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn paint(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = eravon_ui::motion::now(&ctx);
        if self.asks_motion && self.motion_asked.map(|at| now < at || now - at > 2.0).unwrap_or(true) {
            self.motion_asked = Some(now);
            self.reduced = eravon_os::reduce_motion();
        }
        eravon_ui::motion::set_reduced(&ctx, self.reduced);
        self.draw_frame(ui);
        if ctx.data_mut(|d| d.remove_temp::<String>(egui::Id::new("eravon-copied"))).is_some() {
            self.toasts.say(t(Key::SaidCopied), Tone::Note, now);
        }
        let spoke = crate::lang::lang();
        if spoke != self.painted_lang {
            self.painted_lang = spoke;
            eravon_ui::flow::retext(&ctx);
        }
        eravon_ui::hand::finish(&ctx);
    }

    fn draw_frame(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        eravon_ui::probe::begin(ctx);
        let now = eravon_ui::motion::now(&ctx);
        let blend = skin::follow(ctx, self.appearance());
        let follows = self.appearance() == skin::Appearance::System;
        if let Some((w, dark, followed)) = self.backdrop.as_mut() {
            if (blend > 0.5) != *dark || follows != *followed {
                (*dark, *followed) = (blend > 0.5, follows);
                eravon_os::backdrop_appearance(*w, *dark, *followed);
            }
        }
        self.shortcuts(ctx, now);
        self.chrome(ui, now);
        self.paths_ask(ctx);
        self.tell_faults(now);
        self.toasts.set_labels(t(Key::ToastDetail), t(Key::ToastHideDetail), t(Key::ToastClose));
        let left = if self.rail_shown() { tk::RAIL_W } else { 0.0 };
        let drawn = self.toasts.draw(ctx, left);
        if let Some(tag) = drawn.pressed {
            self.view_task(tag, now);
        }
        eravon_ui::layer::fade_out(ctx);
        self.keep_draining(ctx);
    }

    fn appearance(&self) -> skin::Appearance {
        self.shell.machine.appearance.as_deref().and_then(skin::Appearance::named).unwrap_or_default()
    }
}

fn done_is_bad(d: &Done) -> bool {
    match d {
        _ => false,
    }
}

impl eframe::App for Win {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some(w) = self.drops.as_ref() {
            feed_drops(raw, w.hovering(), w.take());
        }
    }

    fn logic(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        self.turn(ctx);
        if self.reads_behind {
            self.bar_turn(ctx, eravon_ui::motion::now(ctx));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _f: &mut eframe::Frame) {
        self.paint(ui);
        if self.reads_behind {
            self.about_window(ui.ctx());
        }
    }

    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        if eravon_ui::material::glass() { [0.0; 4] } else { eravon_ui::palette::canvas() }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.drops_let_go();
        if let Applied::Stopped(r) = apply(&mut self.shell, Action::Quit) {
            self.reaped = Some(r);
        }
        crate::keybox::lock();
    }
}

impl Win {
    pub(super) fn drops_let_go(&mut self) {
        if let Some(w) = self.drops.take() {
            w.stop();
        }
    }
}

pub(super) fn feed_drops(raw: &mut egui::RawInput, hovering: bool, dropped: Vec<std::path::PathBuf>) {
    if hovering && raw.hovered_files.is_empty() {
        raw.hovered_files.push(egui::HoveredFile::default());
    }
    raw.dropped_files.extend(dropped.into_iter().map(|p| egui::DroppedFile { path: Some(p), ..Default::default() }));
}

impl Win {
    fn task_word(&self, k: crate::task::Kind) -> Key {
        task_key(k)
    }
}

impl Win {
    fn paths_landed(&mut self, ctx: &egui::Context, landed: &[crate::task::Outcome]) {
        let pass = eravon_ui::motion::pass(ctx);
        for o in landed.iter().filter(|o| o.kind == crate::task::Kind::Path) {
            let ticket = ctx.data_mut(|d| d.get_temp_mut_or_default::<PathMail>(path_mail()).ticket.take());
            match &o.result {
                Ok(Done::Path(Some(p))) => {
                    if let Some(site) = ticket {
                        ctx.data_mut(|d| d.get_temp_mut_or_default::<PathMail>(path_mail()).answer = Some((site, p.clone(), pass)));
                    }
                }
                Ok(_) | Err(_) => {}
            }
        }
        ctx.data_mut(|d| {
            let m = d.get_temp_mut_or_default::<PathMail>(path_mail());
            if m.answer.as_ref().is_some_and(|a| a.2 + 2 < pass) {
                m.answer = None;
            }
        });
    }

    fn paths_ask(&mut self, ctx: &egui::Context) {
        let Some((site, kind)) = ctx.data_mut(|d| d.get_temp_mut_or_default::<PathMail>(path_mail()).asked.take()) else {
            return;
        };
        if dialog_open(&self.shell) {
            let now = eravon_ui::motion::now(&ctx);
            self.toasts.say(t(Key::PathAlreadyOpen), Tone::Note, now);
            return;
        }
        if kind == crate::platform::Pick::FileOrFolder && self.asks_which {
            self.ux.pick_which = Some(site);
            return;
        }
        match (self.asker)(kind) {
            Ok(wait) => {
                if crate::action::wait_path(&mut self.shell, wait) == crate::task::Spawned::Started {
                    ctx.data_mut(|d| d.get_temp_mut_or_default::<PathMail>(path_mail()).ticket = Some(site));
                }
            }
            Err(f) => self.shell.faults.push(f),
        }
    }
}


impl Win {
    fn act(&mut self, a: Action, now: f64) -> Applied {
        let want = None;
        if let Some(k) = task_of(&a) {
            self.ux.origin.retain(|(x, _, _)| *x != k);
            let h = self.hist().clone();
            self.ux.origin.push((k, self.ux.nav.stack, h));
        }
        self.tell_faults(now);
        let applied = apply(&mut self.shell, a);
        self.told(applied, want, now)
    }

    fn id_name(&self, id: &str) -> String {
        self.shell.identities.as_ref().and_then(|reg| reg.find(id)).map(identity::id_name).unwrap_or_else(|| t(Key::Unnamed).to_string())
    }

    fn told(&mut self, applied: Applied, _want: Option<String>, now: f64) -> Applied {
        let (said, tone) = match applied.clone() {
            Applied::Shown(_) => (String::new(), Tone::Note),
            Applied::House(_) => (String::new(), Tone::Note),
            Applied::CliPassAdded { .. } => (String::new(), Tone::Note),
            Applied::Consented { .. } => (String::new(), Tone::Note),
            Applied::CliPassDropped { .. } => (t(Key::SaidCliPassDropped).to_string(), Tone::Note),
            Applied::Started(k) => {
                self.ux.asked.push(k);
                (String::new(), Tone::Note)
            }
            Applied::Refused(k) => (fill1(Key::SaidInFlight, t(self.task_word(k))), Tone::Bad),
            Applied::Stopped(r) => (fill1(Key::SaidReaped, &r.joined.to_string()), Tone::Note),
            Applied::Seated(r) => (fill1(Key::SaidSeated, t(id_seat_key(r))), Tone::Note),
            Applied::Identities(_) | Applied::FreshWords | Applied::WordsShown | Applied::WordsHidden => (String::new(), Tone::Note),
            Applied::Unlocked | Applied::LockedUp => (String::new(), Tone::Note),
            Applied::PinChanged => (t(Key::SaidPinChanged).to_string(), Tone::Note),
            Applied::AutoLockSet { on: true, secs } => (fill1(Key::SaidAutoLockOn, &(secs / 60).to_string()), Tone::Note),
            Applied::AutoLockSet { on: false, .. } => (t(Key::SaidAutoLockOff).to_string(), Tone::Note),
            Applied::PrimarySet { id } => (fill1(Key::SaidPrimarySet, &self.id_name(&id)), Tone::Note),
            Applied::BackupRestored(sm) => (house::said::backup_restored(&sm), Tone::Note),
            Applied::KeyboxReset => (t(Key::SaidKeyboxReset).to_string(), Tone::Note),
            Applied::MachineAside { to, .. } => (fill1(Key::SaidMachineAside, &to), Tone::Note),
            Applied::Recovered => (t(Key::SaidRecovered).to_string(), Tone::Note),
            Applied::FreshDropped => {
                self.ux.id_confirming = false;
                self.ux.id_confirm = Default::default();
                self.ux.id_words_open = false;
                self.ux.id_words_seen = false;
                (String::new(), Tone::Note)
            }
            Applied::IdentityMade { restored, id, .. } => {
                self.ux.id_confirming = false;
                self.ux.id_confirm = Default::default();
                self.ux.id_words_open = false;
                self.ux.id_words_seen = false;
                let name = if self.ux.id_new_label.trim().is_empty() { self.id_name(&id) } else { self.ux.id_new_label.trim().to_string() };
                (if restored { t(Key::SaidKeyRestoredPlain).to_string() } else { fill1(Key::SaidIdentityMade, &name) }, Tone::Note)
            }
            Applied::IdentityNamed { label: name, .. } => {
                if name.is_empty() {
                    (t(Key::SaidNameCleared).to_string(), Tone::Note)
                } else {
                    (fill1(Key::SaidRenamed, &name), Tone::Note)
                }
            }
            Applied::IdentitySwitched { id } => (fill1(Key::SaidIdentitySwitched, &self.id_name(&id)), Tone::Note),
            Applied::IdentityKept { .. } => (String::new(), Tone::Note),
            Applied::IdentityDeleted { .. } => (fill1(Key::SaidIdentityDeleted, &self.ux.id_deleting.take().unwrap_or_else(|| t(Key::Unnamed).to_string())), Tone::Note),
            Applied::KeyBackedUp { path, .. } => (fill1(Key::SaidKeyBackedUp, &width::file_name(&path)), Tone::Note),
            Applied::Homed { root, mode } => (house::said::homed(&mut self.hx, &root, mode.writable()).unwrap_or_else(|| fill2(Key::SaidHomed, &width::file_name(&root), t(Key::NoteReader))), Tone::Note),
            Applied::Endpoints(n) => {
                self.shell.endpoints_landed();
                (fill1(Key::SaidEndpoints, &n.to_string()), Tone::Note)
            }
            Applied::LogAsks(_) => (String::new(), Tone::Note),
            Applied::RestMode(on) => (t(if on { Key::SaidRestOn } else { Key::SaidRestOff }).to_string(), Tone::Note),
            Applied::RestAdded { base } => (fill1(Key::SaidRestAdded, &eravon_net::sayable(&base)), Tone::Note),
            Applied::RestKeyed { base } => (fill1(Key::SaidRestKeyed, &eravon_net::sayable(&base)), Tone::Note),
            Applied::RestRemoved { .. } => (t(Key::SaidRestRemoved).to_string(), Tone::Note),
            Applied::Spoken(l) => (fill1(Key::SaidLang, l.label()), Tone::Note),
            Applied::Zoned(z) => (house::said::zoned(z), Tone::Note),
            Applied::Appeared(a) => (fill1(Key::SaidAppearance, appearance_label(&a)), Tone::Note),
            Applied::ProxySet(c) => (house::said::proxy(&c), Tone::Note),
            Applied::CliPathRead(_) => (String::new(), Tone::Note),
            Applied::RefreshEvery(n) => (house::said::refresh_every(n), Tone::Note),
            Applied::CliAnchorSet(to) => (fill1(Key::SaidCliAnchor, t(cli_anchor_label(to))), Tone::Note),
            Applied::CliDesktopSet(on) => (t(if on { Key::SaidCliDesktopOn } else { Key::SaidCliDesktopOff }).to_string(), Tone::Note),
            Applied::CliAloneSet(Some(expires)) => (fill1(Key::SaidCliAloneOn, &settings::cli_expiry_said(expires)), Tone::Note),
            Applied::CliAloneSet(None) => (t(Key::SaidCliAloneOff).to_string(), Tone::Note),
            Applied::Trouble(_) => (String::new(), Tone::Bad),
        };
        if !said.is_empty() {
            self.toasts.say(said, tone, now);
        }
        if let Applied::Trouble(f) = &applied {
            if self.ux.id_modal.is_some() {
                self.ux.id_trouble = Some(f.clone());
            }
        }
        applied
    }

    fn vault_or(&mut self, site: VaultSite, r: Applied, now: f64) {
        if r == Applied::Started(crate::task::Kind::Vault) {
            self.ux.vault_site = Some(site);
        } else {
            self.vault_back(site, r, now);
        }
    }

    fn vault_back(&mut self, site: VaultSite, r: Applied, now: f64) {
        match site {
            VaultSite::Wizard => match r {
                Applied::Unlocked => self.ux.gate_clear(),
                Applied::Trouble(_) => {
                    self.ux.pin_shake = Some(now);
                    self.ux.pin.clear();
                    self.ux.pin_again.clear();
                }
                _ => {}
            },
            VaultSite::Change => match r {
                Applied::PinChanged => {
                    self.ux.gate_clear();
                    self.ux.id_close();
                }
                Applied::Trouble(f) => {
                    self.ux.pin_shake = Some(now);
                    self.ux.pin_again.clear();
                    self.ux.pin_old.clear();
                    self.ux.id_trouble = Some(f);
                }
                _ => {}
            },
            VaultSite::Gate => match r {
                Applied::Unlocked | Applied::Recovered | Applied::KeyboxReset | Applied::MachineAside { .. } => self.ux.gate_clear(),
                Applied::Trouble(f) => {
                    self.ux.pin_shake = Some(now);
                    self.ux.pin_again.clear();
                    let burnt = self.shell.vault.is(crate::keybox::State::LockedOut) && f.which() == Some(crate::fault::Known::PinWrong);
                    if f.which() == Some(crate::fault::Known::KdfBelowFloor) {
                        self.ux.gate_reseal = true;
                        self.ux.pin_shake = None;
                    }
                    self.ux.gate_trouble = if burnt {
                        None
                    } else if f.which() == Some(crate::fault::Known::PinWrong) {
                        Some(fill1(Key::PinWrongLeft, f.tail()))
                    } else if f.which() == Some(crate::fault::Known::KdfBelowFloor) {
                        Some(format!("{} {}", f.human(), f.next()))
                    } else {
                        Some(f.human().to_string())
                    };
                }
                _ => {}
            },
            VaultSite::Words => match r {
                Applied::WordsShown => self.ux.pin_shake = None,
                Applied::Trouble(f) => {
                    self.ux.pin_shake = Some(now);
                    self.ux.id_trouble = Some(f);
                }
                _ => {}
            },
            VaultSite::Backup => self.bk_back(r, now),
            VaultSite::Consent => self.house_consent_back(r, now),
            VaultSite::CliPass => match r {
                Applied::CliPassAdded { pass, .. } => {
                    self.ux.pass_pin.clear();
                    self.ux.pass_out = Some(pass);
                }
                Applied::Trouble(f) => {
                    self.ux.pin_shake = Some(now);
                    self.ux.pass_pin.clear();
                    self.ux.pass_trouble = Some(f);
                }
                _ => {}
            },
            VaultSite::Cli => match r {
                Applied::CliAloneSet(_) => self.cli_close(),
                Applied::Trouble(f) => {
                    self.ux.pin_shake = Some(now);
                    self.ux.cli_trouble = Some(f);
                }
                _ => {}
            },
            VaultSite::Id => match r {
                Applied::Trouble(f) => {
                    self.ux.pin_shake = Some(now);
                    self.ux.id_trouble = Some(f);
                }
                r if !id_sheet_stays(&r) => self.ux.id_close(),
                _ => {}
            },
        }
    }
}

fn id_sheet_stays(r: &Applied) -> bool {
    use crate::task::Kind;
    match r {
        Applied::Trouble(_) | Applied::Refused(_) | Applied::FreshWords => true,
        Applied::Started(k) => !matches!(k, Kind::Seat | Kind::Keystore),
        _ => false,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PinSaid {
    Gate,
    Sheet,
    Toast,
}

impl Win {
    fn take_first_pin(&mut self, now: f64, said_at: PinSaid) -> bool {
        let pin = std::mem::take(&mut self.ux.pin);
        match crate::keybox::pin_trouble(pin.expose()) {
            None => {
                self.ux.pin_again = pin;
                true
            }
            Some(why) => {
                self.ux.pin_shake = Some(now);
                let f = crate::fault::Fault::known(crate::fault::Known::PinShape, why.as_str().to_string());
                let said = t(if why == crate::keybox::PinTrouble::Shape { Key::PinRules } else { Key::PinTooSimple });
                match said_at {
                    PinSaid::Gate => self.ux.gate_trouble = Some(said.to_string()),
                    PinSaid::Sheet => self.ux.pin_why = Some(said),
                    PinSaid::Toast => self.toasts.say_full(said.to_string(), "", &f.shown(), Tone::Bad, now),
                }
                false
            }
        }
    }

    fn say_fault(&mut self, f: &crate::fault::Fault, now: f64) {
        let (what, next) = (f.human(), f.next());
        let (what, mut next) = (what.to_string(), next.to_string());
        if f.which() == Some(crate::fault::Known::Locked) && self.shell.vault.absent() {
            next = t(Key::FaultNextNoPinYet).to_string();
        }
        let mut act: Option<(String, String)> = None;
        if f.which() == Some(crate::fault::Known::InsufficientFunds) {
            act = self.shell.anchor.map(|a| (t(Key::IdDoCopy).to_string(), a.hex()));
        }
        self.toasts.say_with(what, &next, &f.shown(), Tone::Bad, now, act);
    }

    fn tell_faults(&mut self, now: f64) {
        let n = self.shell.faults.len();
        if self.faults_told > n {
            self.faults_told = n;
        }
        let shut = self.shell.vault.gate_up();
        for i in self.faults_told..n {
            let f = self.shell.faults[i].clone();
            let locked = f.which() == Some(crate::fault::Known::Locked);
            let pin = matches!(f.which(), Some(crate::fault::Known::PinWrong));
            if shut && (locked || pin) {
                continue;
            }
            self.say_fault(&f, now);
        }
        self.faults_told = n;
    }

}

fn task_of(a: &Action) -> Option<crate::task::Kind> {
    use crate::task::Kind;
    Some(match a {
        Action::SelfCheck => Kind::SelfCheck,
        Action::Measure => Kind::Archive,
        _ => return None,
    })
}

pub enum Start {
    Page(Page),
    Place(Place, bool),
}

pub struct FaceReading {
    pub width: f32,
    pub right_widget: f32,
    pub right_text: f32,
    pub rows: usize,
    pub tables: Vec<eravon_ui::probe::Took>,
    pub inner: Option<bool>,
    pub inner_right: Option<f32>,
    pub inner_head: Option<String>,
    pub inner_head_keys: Option<bool>,
    pub inner_history: Option<(bool, bool)>,
    pub route: String,
    pub inner_route: Option<String>,
    pub texts: Vec<String>,
    pub cells: Vec<CellFit>,
    pub justified_texts: usize,
    pub tiles: Vec<(egui::Rect, bool)>,
}

pub struct CellFit {
    pub width: f32,
    pub over: usize,
    pub wrapped: usize,
}

#[derive(Default)]
struct Ux {
    about_open: bool,
    about_icon: Option<egui::TextureHandle>,
    register_at: Option<String>,
    wiz_from: Option<(crate::roles::Role, Stack, Option<Place>, Option<History>)>,
    wiz_exit_ask: bool,
    bk: Option<Bk>,
    bk_step: BkStep,
    bk_path: String,
    bk_pw: crate::secret::Secret,
    bk_pw2: crate::secret::Secret,
    bk_pin: crate::secret::Secret,
    bk_dir: String,
    bk_trouble: Option<crate::fault::Fault>,
    cli_sheet: bool,
    cli_pin: crate::secret::Secret,
    cli_span: [String; 3],
    cli_never: bool,
    cli_trouble: Option<crate::fault::Fault>,
    pass_open: bool,
    pass_note: String,
    pass_pin: crate::secret::Secret,
    pass_out: Option<crate::secret::Secret>,
    pass_for_ai: bool,
    pass_trouble: Option<crate::fault::Fault>,
    nav: skeleton::Frame,
    wizard: Option<crate::nav::Step>,
    wizard_asked: bool,
    vault_absent: Option<bool>,
    wizard_forced: bool,
    lang_applied: bool,
    page_seen: Option<Page>,
    page_drops: bool,
    now: f64,
    dir_sheet: Option<settings::DirSheet>,
    asks_edit: fields::SavedEdit,
    rpc_err: Option<String>,
    rest_err: Option<(bool, String)>,
    proxy_manual: bool,
    proxy_seeded: bool,
    asked: Vec<crate::task::Kind>,
    landed: std::collections::BTreeMap<crate::task::Kind, (bool, f64)>,
    origin: Vec<(crate::task::Kind, Stack, History)>,
    syncing: Vec<crate::task::Kind>,
    sync_said: Vec<Result<Done, crate::fault::Fault>>,
    synced_at: Option<f64>,
    id_modal: Option<IdModal>,
    id_tab: usize,
    id_seat: Option<crate::roles::Role>,
    keys_details: bool,
    id_words: [crate::secret::Secret; 12],
    id_label: String,
    id_new_label: String,
    id_network: Option<String>,
    id_switch_open: Option<String>,
    id_deleting: Option<String>,
    id_delete_ledgers: Vec<crate::roles::Role>,
    id_delete_backup: Option<crate::identity::BackupSeen>,
    id_hex: crate::secret::Secret,
    id_ks_path: String,
    id_ks_pw: crate::secret::Secret,
    id_confirm: [crate::secret::Secret; 3],
    id_confirming: bool,
    id_words_open: bool,
    id_words_seen: bool,
    id_pw: crate::secret::Secret,
    id_pw2: crate::secret::Secret,
    id_dir: String,
    id_trouble: Option<crate::fault::Fault>,
    id_auth_waiting: bool,
    id_pin: crate::secret::Secret,
    pin: crate::secret::Secret,
    pin_again: crate::secret::Secret,
    pin_old: crate::secret::Secret,
    gate_recover: bool,
    gate_trouble: Option<String>,
    pin_words: [crate::secret::Secret; 12],
    gate_words_ready: bool,
    gate_file_ready: bool,
    pin_ks_path: String,
    pin_ks_pw: crate::secret::Secret,
    pin_shake: Option<f64>,
    pin_why: Option<&'static str>,
    vault_site: Option<VaultSite>,
    gate_reseal: bool,
    gate_aside: Option<crate::home::AsidePlan>,
    gate_aside_trouble: Option<crate::fault::Fault>,
    last_input: f64,
    cli_path_seen: f64,
    sheet_shake: Option<f64>,
    pick_which: Option<egui::Id>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum VaultSite {
    Wizard,
    Change,
    Gate,
    Words,
    Id,
    Backup,
    Cli,
    CliPass,
    Consent,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum IdModal {
    New,
    Import,
    Words,
    Switch,
    Delete(String),
    Backup,
    Name(String),
    Pin,
    Primary(String),
}

impl Ux {
    fn id_open(&mut self, m: IdModal) {
        self.id_form_clear();
        self.pin_why = None;
        self.id_modal = Some(m);
    }

    fn gate_clear(&mut self) {
        self.pin.clear();
        self.pin_again.clear();
        self.pin_old.clear();
        for w in self.pin_words.iter_mut() {
            w.clear();
        }
        self.pin_ks_path.clear();
        self.pin_ks_pw.clear();
        self.pin_shake = None;
        self.gate_recover = false;
        self.gate_reseal = false;
        self.gate_trouble = None;
        self.gate_words_ready = false;
        self.gate_file_ready = false;
        self.gate_aside = None;
        self.gate_aside_trouble = None;
    }

    fn row_turn(&self, now: f64) -> bool {
        (!self.pin_again.is_empty() && self.pin.is_empty()) || self.pin_shake.is_some_and(|at| now - at < 0.5)
    }

    fn gate_back(&mut self) {
        for w in self.pin_words.iter_mut() {
            w.clear();
        }
        self.pin_ks_path.clear();
        self.pin_ks_pw.clear();
        self.pin.clear();
        self.pin_again.clear();
        self.gate_recover = false;
        self.gate_trouble = None;
        self.gate_words_ready = false;
        self.gate_file_ready = false;
    }

    fn id_close(&mut self) {
        self.id_modal = None;
        self.id_form_clear();
    }

    fn id_form_clear(&mut self) {
        self.id_trouble = None;
        self.id_auth_waiting = false;
        self.id_confirming = false;
        self.id_confirm = Default::default();
        self.id_words_open = false;
        self.id_words_seen = false;
        for w in self.id_words.iter_mut() {
            w.clear();
        }
        self.id_label.clear();
        self.id_new_label.clear();
        self.id_network = None;
        self.id_hex.clear();
        self.id_ks_pw.clear();
        self.id_pw.clear();
        self.id_pw2.clear();
        self.id_pin.clear();
        self.pin.clear();
        self.pin_again.clear();
        self.pin_old.clear();
        self.pin_shake = None;
    }

    fn claim(&mut self, k: crate::task::Kind) -> bool {
        match self.asked.iter().position(|x| *x == k) {
            Some(i) => {
                self.asked.remove(i);
                true
            }
            None => false,
        }
    }
}


use own::{own_delete_effect, own_stage, task_key, task_long, OwnBk};

mod about;
mod backup;
mod chrome;
mod common;
mod drive;
mod faces;
mod gate;
mod landing;
mod identity;
mod modals;
mod network;
mod pages;
mod probes;
mod own;
mod settings;
mod skeleton;
mod watch;
mod wizard;
pub(crate) mod house;
pub use self::faces::*;
pub use self::house::pageframes::Target as FrameTarget;
pub use self::house::form_answer;

pub fn page_frames(shell: Shell, pages: &[FrameTarget], width: f32, dark: bool) -> Result<eravon_literals::json::Out, String> {
    house::pageframes::read(shell, pages, width, dark)
}
pub use self::drive::*;
pub use self::probes::*;
use self::chrome::stagger;
use self::common::*;

pub(super) fn backup_content(s: &crate::backup::Summary) -> String {
    crate::lang::filln(Key::BackupContent, &[&s.identities.to_string(), "0", &s.records.to_string()])
}

#[cfg(test)]
mod tests {
    #[test]
    fn each_opening_of_the_identity_card_starts_from_an_empty_form() {
        let mut ux = super::Ux::default();
        ux.id_new_label = "named in the first run".into();
        ux.id_open(super::IdModal::New);
        assert_eq!(ux.id_new_label, "", "a name the first run left is not carried into the card");
        ux.id_new_label = "half typed".into();
        ux.id_close();
        ux.id_open(super::IdModal::New);
        assert_eq!(ux.id_new_label, "", "a card closed half filled opens empty");
        ux.id_new_label = "half typed".into();
        ux.id_open(super::IdModal::Import);
        assert_eq!((ux.id_new_label.as_str(), ux.id_modal.clone()), ("", Some(super::IdModal::Import)), "one card opened over another starts empty");
    }

    #[test]
    fn every_task_kind_has_its_own_name() {
        let mut seen: Vec<super::Key> = Vec::new();
        for k in crate::task::Kind::ALL.into_iter().filter(|k| k.page_of() == *k) {
            let key = super::task_key(k);
            assert!(!seen.contains(&key), "{} borrows another kind's name", k.as_str());
            seen.push(key);
        }
    }
}

fn dialog_open(shell: &crate::shell::Shell) -> bool {
    shell.tasks.in_flight(crate::task::Kind::Path)
}

