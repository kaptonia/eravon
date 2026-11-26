use eravon_literals::json::{In, Out};
use eravon_ui::egui;

use super::super::{fresh_win, Typed, Ux, Win};
use crate::action::{Action as Fam, Applied};
use crate::house::action::Action;
use crate::shell::Shell;

pub struct WinDrive {
    ctx: egui::Context,
    w: Win,
    t: f64,
    vault_frames: u64,
    gate_busy_filled: u64,
}

const STEP: f64 = eravon_ui::frames::STEP;

const PACE_MS: u64 = 16;

const SETTLE_MAX: usize = 4000;

pub(crate) const SETTLE_SECS: u64 = 120;

fn arg(st: &In, k: &str) -> Result<String, String> {
    st.get(k).and_then(|x| x.as_str()).map(str::to_string).ok_or_else(|| format!("这一步要 {k}"))
}

impl WinDrive {
    pub fn start(shell: Shell, passcode: Option<&str>) -> Result<WinDrive, String> {
        let ctx = egui::Context::default();
        let _ = eravon_ui::skin::dress(&ctx);
        let mut w = fresh_win(shell, Typed::default(), Ux::default());
        w.faults_told = usize::MAX;
        let mut d = WinDrive { ctx, w, t: 0.0, vault_frames: 0, gate_busy_filled: 0 };
        d.frame();
        if let Some(p) = passcode.filter(|_| !d.w.shell.unlocked()) {
            let now = d.t;
            let _ = d.w.act(Fam::Unlock { pin: crate::secret::Secret::of(p) }, now);
            d.vault_settle();
        }
        Ok(d)
    }

    pub fn shell_mut(&mut self) -> &mut Shell {
        &mut self.w.shell
    }

    fn frame(&mut self) {
        self.frame_typing("");
    }

    fn frame_typing(&mut self, keys: &str) {
        self.t += STEP;
        let events = if keys.is_empty() { Vec::new() } else { vec![egui::Event::Text(keys.to_string())] };
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(eravon_ui::house::WINDOW_W, eravon_ui::house::WINDOW_H))),
            time: Some(self.t),
            events,
            ..Default::default()
        };
        let w = &mut self.w;
        let _ = self.ctx.run_ui(raw, |ui| w.draw(ui));
    }

    fn flying(&self) -> Vec<&'static str> {
        self.w.shell.tasks.flying().into_iter().map(|k| k.as_str()).collect()
    }

    fn all_finished(&self) -> bool {
        self.w.shell.tasks.flying().into_iter().all(|k| self.w.shell.tasks.finished_in_flight(k))
    }

    fn vault_settle(&mut self) -> usize {
        let (mut n, cap) = (0, std::time::Instant::now() + std::time::Duration::from_secs(SETTLE_SECS));
        while self.w.shell.tasks.in_flight(crate::task::Kind::Vault) && std::time::Instant::now() < cap {
            self.frame();
            n += 1;
            if !self.w.shell.tasks.finished_in_flight(crate::task::Kind::Vault) {
                std::thread::sleep(std::time::Duration::from_millis(PACE_MS));
            }
        }
        self.vault_frames += n as u64;
        self.frame();
        n
    }

    fn seat_settle(&mut self) {
        let cap = std::time::Instant::now() + std::time::Duration::from_secs(SETTLE_SECS);
        while self.w.shell.tasks.in_flight(crate::task::Kind::Seat) && std::time::Instant::now() < cap {
            self.frame();
            if !self.w.shell.tasks.finished_in_flight(crate::task::Kind::Seat) {
                std::thread::sleep(std::time::Duration::from_millis(PACE_MS));
            }
        }
        self.frame();
    }

    fn vault(&mut self, act: &str, st: &In) -> Result<(), String> {
        let f = |k: &str| st.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let now = self.t;
        match act {
            "pin" => {
                let pin = crate::secret::Secret::of(&f("d"));
                if self.w.ux.gate_recover && self.w.ux.pin_words.iter().all(|w| !w.expose().trim().is_empty()) {
                    if self.w.ux.pin_again.is_empty() {
                        self.w.ux.pin_again = pin;
                    } else {
                        let words = super::super::faces::joined(&self.w.ux.pin_words);
                        let first = std::mem::take(&mut self.w.ux.pin_again);
                        let _ = self.w.act(Fam::RecoverWords { words, pin: first, again: pin }, now);
                    }
                    return Ok(());
                }
                let a = if self.w.ux.gate_reseal { Fam::Reseal { pin } } else { Fam::Unlock { pin } };
                let _ = self.w.act(a, now);
            }
            "forget" => self.w.ux.gate_recover = true,
            "words" => {
                let w = eravon_vault::words::split(&f("w"));
                self.w.ux.pin_words = std::array::from_fn(|i| crate::secret::Secret::of(w.get(i).map(String::as_str).unwrap_or_default()));
            }
            "file" => {
                self.w.ux.pin_ks_path = f("path");
                self.w.ux.pin_ks_pw = crate::secret::Secret::of(&f("p"));
            }
            step => {
                let a = Action::of(step).ok_or_else(|| format!("口令一族里没有这一样:{step}"))?;
                match crate::house::drive::fam::fam_of(&mut self.w.shell, a, st) {
                    Some(Ok(Some(fam))) => {
                        let _ = self.w.act(fam, now);
                    }
                    Some(Ok(None)) => {}
                    Some(Err(e)) => return Err(e),
                    None => return Err(format!("口令一族里没有这一样:{step}")),
                }
            }
        }
        Ok(())
    }

    fn pick(&mut self, st: &In) -> Result<String, String> {
        let f = |k: &str| st.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let kind = match f("kind").as_str() {
            "" | "file" => crate::platform::Pick::File,
            "folder" => crate::platform::Pick::Folder,
            "any" => crate::platform::Pick::FileOrFolder,
            other => return Err(format!("选档只收 file、folder、any:{other}")),
        };
        let ask = match f("ask").as_str() {
            "" | "no" => false,
            "yes" => true,
            other => return Err(format!("先问只收 no、yes:{other}")),
        };
        let asked_before = std::mem::replace(&mut self.w.asks_which, ask);
        let line = self.pick_asked(st, kind);
        self.w.asks_which = asked_before;
        let line = line?;
        Ok(if ask { format!("{line} · which {}", self.w.ux.pick_which.is_some()) } else { line })
    }

    fn pick_asked(&mut self, st: &In, kind: crate::platform::Pick) -> Result<String, String> {
        let f = |k: &str| st.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let with = f("with");
        let site = egui::Id::new(("eravon-drive-pick", f("site")));
        let toasts = self.w.shell.faults.len();
        let showing = self.w.toasts.showing().map(str::to_string);
        match with.as_str() {
            "release" => {
                let tx = STAND_IN.lock().map_err(|_| "替身锁坏了".to_string())?.held.take().ok_or("没有一个开着的替身选档框可放")?;
                let _ = tx.send(Some(f("path")));
            }
            "answer" | "hold" | "refuse" | "cancel" => {
                if let Ok(mut s) = STAND_IN.lock() {
                    s.next = (with.clone(), f("path"));
                }
                self.w.asker = stand_in_asker;
                let _ = super::super::common::path_answer(&self.ctx, site, true, kind);
            }
            other => return Err(format!("选档替身只收 answer、hold、release、refuse、cancel:{other}")),
        }
        let cap = std::time::Instant::now() + std::time::Duration::from_secs(SETTLE_SECS);
        let mut got: Option<String> = None;
        for _ in 0..SETTLE_MAX {
            self.frame();
            if got.is_none() {
                got = super::super::common::path_answer(&self.ctx, site, false, kind);
            }
            let held = STAND_IN.lock().map(|s| s.held.is_some()).unwrap_or(false);
            if !self.w.shell.tasks.in_flight(crate::task::Kind::Path) || held || std::time::Instant::now() > cap {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(PACE_MS));
        }
        if got.is_none() {
            self.frame();
            got = super::super::common::path_answer(&self.ctx, site, false, kind);
        }
        let toast = self.w.toasts.showing().map(str::to_string).filter(|t| Some(t) != showing.as_ref());
        let said = self.w.shell.faults.get(toasts..).and_then(|x| x.last()).map(|f| f.said().to_string()).or(toast).unwrap_or_default();
        Ok(format!("{} · {with} · got {} · out {} · said {said}", f("site"), got.unwrap_or_default(), self.w.shell.tasks.in_flight(crate::task::Kind::Path)))
    }

    fn house(&mut self, step: Action, args: &[(&str, &str)]) -> String {
        match self.w.house(crate::house::act::Act::with(step, args)) {
            Ok(s) => s,
            Err(e) => format!("拒 · {e}"),
        }
    }

    pub fn step(&mut self, act: Action, st: &In) -> Result<String, String> {
        let now = self.t;
        let said = match act {
            Action::WinPage => {
                let seat = arg(st, "seat")?;
                let p = st.get("page").and_then(|x| x.as_str()).unwrap_or(crate::house::seam::PAGE_HOME).to_string();
                let a = self.house(Action::Seat, &[("to", &seat)]);
                let b = self.house(Action::Page, &[("to", &p)]);
                self.frame();
                format!("{seat} · {p} · {a} · {b}")
            }
            Action::WinRail => {
                let p = arg(st, "page")?;
                let page = crate::house::seller::Page::of(&p).ok_or_else(|| format!("认不出这一页:{p}"))?;
                self.w.rail_to(Win::place_of(page), now);
                self.frame();
                format!("{p} · {}", self.w.route().name())
            }
            Action::WinLock => {
                let _ = self.w.act(Fam::Lock, now);
                self.frame();
                "锁定".to_string()
            }
            Action::WinUnlock => {
                let pin = arg(st, "pin")?;
                let got = self.w.act(Fam::Unlock { pin: crate::secret::Secret::of(&pin) }, now);
                self.vault_settle();
                if !self.w.shell.unlocked() {
                    return Err(match got {
                        Applied::Trouble(f) => f.said().to_string(),
                        other => format!("{other:?}"),
                    });
                }
                "解锁".to_string()
            }
            Action::WinVault => {
                let a = arg(st, "act")?;
                self.vault(&a, st)?;
                if st.get("hold").and_then(|x| x.as_str()) == Some("true") {
                    format!("{a} · 在飞")
                } else {
                    let n = self.vault_settle();
                    format!("{a} · 落地前画 {n} 帧")
                }
            }
            Action::WinKey => {
                let to = arg(st, "to")?;
                let id = crate::house::drive::fam::id_of(&to)?;
                let _ = self.w.act(Fam::SwitchIdentity { id }, now);
                if st.get("hold").and_then(|x| x.as_str()) == Some("true") {
                    format!("切钥 {to} · 在飞")
                } else {
                    self.seat_settle();
                    format!("切钥 {to}")
                }
            }
            Action::WinEndpoints => {
                let urls: Vec<String> = match st.get("urls") {
                    Some(In::Arr(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
                    _ => return Err("这一步要 urls".to_string()),
                };
                let said = self.house(Action::SetEndpoints, &[("v", &urls.join(" "))]);
                format!("端点 {} 个 · {said}", urls.len())
            }
            Action::WinSync => {
                let said = self.house(Action::Refresh, &[]);
                format!("刷新 · {said}")
            }
            Action::WinGas => {
                self.w.shell.open_settings();
                let said = self.house(Action::SetGas, &[]);
                format!("余额 · {said}")
            }
            Action::WinWalletForm => {
                let pick = st.get("pick").and_then(|x| x.as_str()).and_then(|x| x.parse::<usize>().ok()).unwrap_or(0);
                if let Some(x) = self.w.shell.desk.sh.wallet.as_mut() {
                    x.sheet = true;
                }
                self.w.shell.desk.wl_form = (pick, arg(st, "amount")?, arg(st, "to")?);
                "转出页单".to_string()
            }
            Action::WinWalletSend => {
                let (_, amount, to) = self.w.shell.desk.wl_form.clone();
                let token = st.get("token").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                let said = self.house(Action::WlSend, &[("token", &token), ("amount", &amount), ("to", &to)]);
                format!("转出 {amount} · {said}")
            }
            Action::WinAwait => {
                let p = arg(st, "fifo")?;
                let mut b = [0u8; 1];
                let mut f = std::fs::File::open(&p).map_err(|e| format!("{p} 开不了:{e}"))?;
                std::io::Read::read_exact(&mut f, &mut b).map_err(|e| format!("{p} 读不到:{e}"))?;
                "后台在读".to_string()
            }
            Action::WinRelease => {
                let p = arg(st, "fifo")?;
                let mut f = std::fs::OpenOptions::new().write(true).open(&p).map_err(|e| format!("{p} 开不了:{e}"))?;
                std::io::Write::write_all(&mut f, b"1").map_err(|e| format!("{p} 写不了:{e}"))?;
                "放行".to_string()
            }
            Action::WinJoin => {
                let cap = std::time::Instant::now() + std::time::Duration::from_secs(SETTLE_SECS);
                while !self.all_finished() && std::time::Instant::now() < cap {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                format!("做完 · 在飞 {}", self.flying().join(" "))
            }
            Action::WinFrame => {
                let n = st.get("n").and_then(|x| x.as_str()).and_then(|x| x.parse::<usize>().ok()).unwrap_or(1);
                let keys = st.get("keys").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                for i in 0..n {
                    let busy = self.w.shell.tasks.in_flight(crate::task::Kind::Vault);
                    self.frame_typing(if i == 0 { &keys } else { "" });
                    if busy && self.w.shell.tasks.in_flight(crate::task::Kind::Vault) {
                        self.vault_frames += 1;
                        self.gate_busy_filled = self.gate_busy_filled.max(self.w.ux.pin.chars() as u64);
                    }
                }
                format!("画 {n} 帧 · 门上 {} 位", self.w.ux.pin.chars())
            }
            Action::WinPick => self.pick(st)?,
            Action::WinSettle => {
                let (mut n, cap) = (0, std::time::Instant::now() + std::time::Duration::from_secs(SETTLE_SECS));
                while n < SETTLE_MAX && (!self.flying().is_empty() || self.w.shell.house_busy()) && std::time::Instant::now() < cap {
                    if self.all_finished() || self.w.shell.tasks.in_flight(crate::task::Kind::Sync) {
                        self.frame();
                        n += 1;
                        if self.w.shell.tasks.in_flight(crate::task::Kind::Sync) {
                            std::thread::sleep(std::time::Duration::from_millis(PACE_MS));
                        }
                    } else {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                self.frame();
                format!("落定 · 在飞 {}", self.flying().join(" "))
            }
            other => {
                let st_args = crate::house::drive::args_of(st)?;
                let refs: Vec<(&str, &str)> = st_args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                let said = self.house(other, &refs);
                self.frame();
                said
            }
        };
        Ok(said)
    }

    pub fn finish(self) -> Shell {
        let flying: Vec<Out> = self.flying().into_iter().map(|k| Out::S(k.to_string())).collect();
        let mut sh = self.w.shell;
        let (pick, amount, to) = sh.desk.wl_form.clone();
        let mut rows = vec![
            ("vaultFrames", Out::N(self.vault_frames)),
            ("gateBusyFilled", Out::N(self.gate_busy_filled)),
            ("wordsShown", Out::B(sh.words.is_some())),
            ("primaryHere", Out::B({
                let current = sh.identities.as_ref().and_then(|r| r.now().map(|(row, _)| row.id.clone()));
                matches!((&sh.primary, current), (Some((p, _)), Some(c)) if p.eq_ignore_ascii_case(&c))
            })),
            ("flying", Out::A(flying)),
            ("pendingUrls", Out::N(sh.desk.pending_url.len() as u64)),
            ("retry", Out::A(sh.desk.retry.iter().map(|(k, _)| Out::S(k.as_str().to_string())).collect())),
            ("syncDone", Out::B(sh.desk.sync_done_at.is_some())),
            ("wlForm", Out::O(vec![("amount", Out::S(amount)), ("pick", Out::N(pick as u64)), ("to", Out::S(to))])),
        ];
        rows.sort_by(|a, b| a.0.cmp(b.0));
        sh.desk.sh.win = Some(Out::O(rows));
        sh
    }
}

struct StandIn {
    next: (String, String),
    held: Option<std::sync::mpsc::Sender<Option<String>>>,
}

static STAND_IN: std::sync::Mutex<StandIn> = std::sync::Mutex::new(StandIn { next: (String::new(), String::new()), held: None });

fn stand_in_asker(_kind: crate::platform::Pick) -> Result<crate::platform::Wait, crate::fault::Fault> {
    let (with, path) = STAND_IN.lock().map(|s| s.next.clone()).unwrap_or_default();
    match with.as_str() {
        "refuse" => Err(crate::fault::Fault::known(crate::fault::Known::DialogUnavailable, crate::house::seam::PICK_NO_PANEL.to_string())),
        "cancel" => Ok(Box::new(|| Ok(None))),
        "hold" => {
            let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
            if let Ok(mut s) = STAND_IN.lock() {
                s.held = Some(tx);
            }
            Ok(Box::new(move || Ok(rx.recv().unwrap_or(None))))
        }
        _ => Ok(Box::new(move || Ok(Some(path)))),
    }
}
