use crate::fault::{Fault, Known};
use crate::shell::Shell;
use crate::task::{Done, Kind, Outcome};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use eravon_cli::door::{Reply, Request};

pub const ORIGINALS_ALL: &str = "all";

pub const REQUEST_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

pub const ANSWER_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

pub fn verbs() -> Vec<(&'static str, Vec<&'static str>)> {
    eravon_cli::doorverbs::HOME.iter().map(|(v, fl)| (*v, eravon_cli::doorverbs::names(fl))).collect()
}

pub struct Asked {
    pub req: Request,
    pub(crate) answer: Sender<Reply>,
}

pub struct Door {
    home: PathBuf,
    closer: eravon_os::door::Closer,
    asked: Receiver<Asked>,
    came: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    listener: Option<std::thread::JoinHandle<()>>,
    in_hand: InHand,
}

type InHand = std::sync::Arc<std::sync::Mutex<Option<Sender<Reply>>>>;

impl Drop for Door {
    fn drop(&mut self) {
        self.close(None);
    }
}

impl Door {
    fn close(&mut self, doing: Option<Doing>) -> usize {
        self.closer.close();
        let mut told = 0;
        if let Some(d) = doing {
            let _ = d.answer().send(Reply::Closed);
            told += 1;
        }
        let taken = std::mem::replace(&mut self.asked, std::sync::mpsc::channel().1);
        for a in taken.try_iter() {
            let _ = a.answer.send(Reply::Closed);
            told += 1;
        }
        drop(taken);
        if let Some(tx) = self.in_hand.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = tx.send(Reply::Closed);
        }
        if let Some(h) = self.listener.take() {
            let _ = h.join();
        }
        told
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    pub fn came(&self) -> usize {
        self.came.load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub enum Doing {
    Verb { answer: Sender<Reply>, serial: u64 },
}

pub enum Turn {
    Verb(Request),
}

pub type Waker = std::sync::Arc<dyn Fn() + Send + Sync>;

pub fn turn_of(_shell: &Shell, q: &Request) -> Result<Turn, Fault> {
    if !eravon_cli::verbs::HOME.iter().any(|(v, _)| *v == q.verb) {
        return Err(Fault::known(Known::DoorUnread, q.verb.clone()));
    }
    Ok(Turn::Verb(q.clone()))
}

fn on_desktop_only(verb: &str) -> bool {
    eravon_cli::verbwire::split(verb).is_some_and(|(_, v)| eravon_cli::schema::passcode_only(v))
}


pub fn refused(f: &Fault) -> Reply {
    let tail = f.tail_in(crate::lang::Lang::En).to_string();
    let (code, (zh, en)) = match f.which() {
        Some(k) => {
            let (zh, en) = crate::lang::both(f.what_key().unwrap_or(k.what()));
            (k.as_str().to_string(), (with_evidence(zh, f.tail_in(crate::lang::Lang::Zh), true), with_evidence(en, f.tail_in(crate::lang::Lang::En), false)))
        }
        None => (f.class().as_str().to_string(), (f.said().to_string(), f.said().to_string())),
    };
    eravon_cli::deskwire::refusal(code, tail, zh, en)
}

fn with_evidence(said: &str, evidence: &str, zh: bool) -> String {
    let evidence = evidence.trim();
    if evidence.is_empty() {
        return said.to_string();
    }
    match (zh, said.trim_end()) {
        (true, s) if s.ends_with('。') => format!("{s}{evidence}"),
        (true, s) => format!("{s}:{evidence}"),
        (false, s) if s.ends_with('.') => format!("{s} {evidence}"),
        (false, s) => format!("{s}: {evidence}"),
    }
}

fn wait_at(door: eravon_os::door::Listener, asked: Sender<Asked>, wake: Option<Waker>, came: std::sync::Arc<std::sync::atomic::AtomicUsize>, in_hand: InHand) {
    use eravon_cli::door::{put, take, Unread};
    loop {
        let mut s = match door.accept() {
            Ok(Some(s)) => s,
            Ok(None) => return,
            Err(_) => continue,
        };
        let _ = s.set_read_deadline(Some(REQUEST_WAIT));
        let reply = match take(&mut s) {
            Ok(bytes) => match Request::of_bytes(&bytes) {
                Some(req) => {
                    let (tx, rx) = std::sync::mpsc::channel();
                    *in_hand.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx.clone());
                    let reply = if asked.send(Asked { req, answer: tx }).is_err() {
                        Reply::Closed
                    } else {
                        came.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        if let Some(w) = &wake {
                            w();
                        }
                        rx.recv().unwrap_or(Reply::Closed)
                    };
                    *in_hand.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    reply
                }
                None => refused(&Fault::known(Known::DoorUnread, crate::lang::both_of(|| crate::lang::t(crate::lang::Key::TailDoorNotForm).to_string()))),
            },
            Err(Unread::TooLarge(n)) => refused(&Fault::known(Known::DoorUnread, crate::lang::both_of(|| crate::lang::fill1(crate::lang::Key::TailDoorTooLarge, &n.to_string())))),
            Err(Unread::Short) | Err(Unread::Io(_)) => continue,
        };
        let _ = s.set_write_deadline(Some(ANSWER_WAIT));
        let _ = put(&mut s, &reply.to_bytes());
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        if let Some(mut d) = self.door.take() {
            d.close(self.door_doing.take());
        }
    }
}

fn machine_unread(e: eravon_os::machine::Unread) -> Fault {
    use eravon_os::machine::Unread;
    match e {
        Unread::NoHome => Fault::known(Known::NoHomeDir, crate::lang::both_of(|| crate::lang::t(crate::lang::Key::Tail161).to_string())),
        Unread::PointerShape(p) => Fault::known(Known::MachineShape, p.display().to_string()),
        Unread::Pointer(p, e) => crate::fault::classify(&e, &p.display().to_string()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoorWhy {
    TooLong,
    Refused(String),
}

impl Shell {
    pub fn door_why(&self) -> Option<DoorWhy> {
        let (home, f) = self.door_shut.as_ref()?;
        if self.door_wanted().as_ref() != Some(home) || self.door.is_some() {
            return None;
        }
        Some(if self.door_too_long { DoorWhy::TooLong } else { DoorWhy::Refused(f.tail().to_string()) })
    }

    fn door_wanted(&self) -> Option<PathBuf> {
        let writer = self.lock.as_ref().map(|l| l.mode() == crate::lock::Mode::Writer).unwrap_or(false);
        let through = self.machine.cli_path() == crate::machine::CliPath::Desktop;
        if self.door_off || !self.unlocked() || !writer || !through || self.home.is_none() {
            return None;
        }
        crate::house::seam::door_machine()
    }


    pub fn door_sync(&mut self) {
        let want = self.door_wanted();
        let have = self.door.as_ref().map(|d| d.home.clone());
        if want == have {
            if want.is_none() {
                self.door_shut = None;
                self.door_place = None;
            }
            return;
        }
        if let Some(mut d) = self.door.take() {
            self.door_answered += d.close(self.door_doing.take());
            crate::house::approve::closed(self);
            self.tasks.carry(None);
        }
        let Some(home) = want else {
            self.door_shut = None;
            self.door_place = None;
            return;
        };
        match self.door_open(&home) {
            Ok(d) => {
                self.door = Some(d);
                self.door_shut = None;
            }
            Err((f, long)) => {
                self.door_shut = Some((home, f));
                self.door_too_long = long;
            }
        }
    }

    fn door_place_for(&mut self, home: &std::path::Path) -> Result<std::path::PathBuf, Fault> {
        if let Some((h, place)) = &self.door_place {
            if h == home {
                return Ok(place.clone());
            }
        }
        let (machine, _) = eravon_os::machine::here().map_err(machine_unread)?;
        let place = eravon_cli::door::place(&machine, home);
        self.door_place = Some((home.to_path_buf(), place.clone()));
        Ok(place)
    }

    fn door_open(&mut self, home: &std::path::Path) -> Result<Door, (Fault, bool)> {
        let place = self.door_place_for(home).map_err(|f| (f, false))?;
        let (listener, closer) = eravon_os::door::listen(&place).map_err(|e| (Fault::known(Known::DoorShut, e.to_string()), eravon_os::door::too_long(&place)))?;
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.door_waker.clone();
        let came = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = came.clone();
        let in_hand: InHand = std::sync::Arc::new(std::sync::Mutex::new(None));
        let held = in_hand.clone();
        let listener = match crate::task::start_named("door", move || wait_at(listener, tx, wake, counted, held)) {
            Ok(h) => h,
            Err(e) => {
                closer.close();
                return Err((Fault::known(Known::DoorShut, e.to_string()), false));
            }
        };
        Ok(Door { home: home.to_path_buf(), closer, asked: rx, came, listener: Some(listener), in_hand })
    }

    pub fn door_turn(&mut self, landed: &[Outcome]) {
        if let Some(doing) = self.door_doing.take() {
            self.door_doing = self.door_step(doing, landed);
        }
        self.door_sync();
        crate::house::approve::expire(self, landed);
        while self.door_doing.is_none() && !crate::house::approve::holding(self) {
            self.tasks.carry(None);
            let Some(a) = self.door.as_ref().and_then(|d| d.asked.try_recv().ok()) else { return };
            self.door_do(a);
        }
    }

    pub(crate) fn door_do(&mut self, a: Asked) {
        let Asked { req, answer } = a;
        let Turn::Verb(req) = match turn_of(self, &req) {
            Ok(t) => t,
            Err(f) => {
                self.door_send(&answer, refused(&f));
                return;
            }
        };
        if on_desktop_only(&req.verb) {
            self.door_send(&answer, Reply::OnDesktop);
            return;
        }
        if !self.unlocked() {
            self.door_send(&answer, refused(&Fault::known(Known::Locked, String::new())));
            return;
        }
        let Some(Asked { req, answer }) = crate::house::approve::gate(self, Asked { req, answer }) else { return };
        let data = req.home.clone();
        let started = crate::action::spawn_for(self, Kind::Door, move || {
            let session = eravon_vault::session::Session::borrowed(&data).map_err(Fault::from)?;
            let lent = eravon_cli::deskwire::Lent { session };
            Ok(Done::Door(match eravon_cli::deskwire::run_for_desktop(&req, lent) {
                Some(body) => Reply::Answered { body },
                None => refused(&Fault::known(Known::DoorUnread, req.verb.clone())),
            }))
        });
        match started {
            crate::task::Spawned::Started => match self.tasks.serial_of(Kind::Door) {
                Some(serial) => self.door_doing = Some(Doing::Verb { answer, serial }),
                None => self.door_send(&answer, refused(&Fault::known(Known::OutcomeLost, Kind::Door.as_str().to_string()))),
            },
            crate::task::Spawned::InFlight => self.door_send(&answer, refused(&Fault::known(Known::InFlight, Kind::Door.as_str().to_string()))),
        }
    }

    pub(crate) fn door_send(&mut self, answer: &Sender<Reply>, r: Reply) {
        self.door_answered += 1;
        let _ = answer.send(r);
    }

    fn door_step(&mut self, doing: Doing, landed: &[Outcome]) -> Option<Doing> {
        match doing {
            Doing::Verb { answer, serial } => match landed.iter().find(|o| o.serial == serial) {
                Some(o) => {
                    let reply = match &o.result {
                        Ok(Done::Door(r)) => r.clone(),
                        Ok(_) => refused(&Fault::known(Known::OutcomeLost, Kind::Door.as_str().to_string())),
                        Err(f) => refused(f),
                    };
                    let answered = matches!(reply, Reply::Answered { .. });
                    self.door_send(&answer, reply);
                    if answered {
                        self.house_door_landed();
                    }
                    None
                }
                None if !self.tasks.in_flight(Kind::Door) => {
                    self.door_send(&answer, refused(&Fault::known(Known::OutcomeLost, Kind::Door.as_str().to_string())));
                    None
                }
                None => Some(Doing::Verb { answer, serial }),
            },
        }
    }
}

impl Doing {
    fn answer(&self) -> &Sender<Reply> {
        match self {
            Doing::Verb { answer, .. } => answer,
        }
    }
}

