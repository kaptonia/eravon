use crate::fault::Fault;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Stage {
    pub at: u8,
    pub done: u32,
    pub total: u32,
}

impl Stage {
    pub fn frac(self) -> Option<f32> {
        (self.total > 0).then(|| (self.done.min(self.total) as f32) / self.total as f32)
    }

    fn pack(self) -> u64 {
        (u64::from(self.at) << 56) | (u64::from(self.done & 0x0FFF_FFFF) << 28) | u64::from(self.total & 0x0FFF_FFFF)
    }

    fn unpack(v: u64) -> Stage {
        Stage { at: (v >> 56) as u8, done: ((v >> 28) & 0x0FFF_FFFF) as u32, total: (v & 0x0FFF_FFFF) as u32 }
    }
}

static STAGES: [std::sync::atomic::AtomicU64; Kind::ALL.len()] = [const { std::sync::atomic::AtomicU64::new(0) }; Kind::ALL.len()];
const LIVE: u64 = 1 << 55;

pub fn stage_at(k: Kind, at: u8) {
    STAGES[k as usize].store(Stage { at, done: 0, total: 0 }.pack() | LIVE, std::sync::atomic::Ordering::Relaxed);
}

pub fn count(k: Kind, done: u64, total: u64) {
    let slot = &STAGES[k as usize];
    let cur = Stage::unpack(slot.load(std::sync::atomic::Ordering::Relaxed));
    let s = Stage { at: cur.at, done: done.min(0x0FFF_FFFF) as u32, total: total.min(0x0FFF_FFFF) as u32 };
    slot.store(s.pack() | LIVE, std::sync::atomic::Ordering::Relaxed);
}

pub fn stage(k: Kind) -> Option<Stage> {
    let v = STAGES[k as usize].load(std::sync::atomic::Ordering::Relaxed);
    (v & LIVE != 0).then(|| Stage::unpack(v & !LIVE))
}

static THREAD_FAILS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn pretend_thread_fails(on: bool) {
    THREAD_FAILS.store(on, std::sync::atomic::Ordering::SeqCst);
}

struct Part {
    kind: Kind,
    epoch: u64,
    nodes: u64,
    tx: Sender<Outcome>,
    wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
}

thread_local! {
    static PART: std::cell::RefCell<Option<Part>> = const { std::cell::RefCell::new(None) };
}

pub fn land_part(d: Done) -> bool {
    PART.with(|p| {
        let p = p.borrow();
        let Some(p) = p.as_ref() else { return false };
        let sent = p.tx.send(Outcome { kind: p.kind, serial: 0, result: Ok(d), epoch: p.epoch, stale: false, earlier: false, nodes: p.nodes, renoded: false, part: true }).is_ok();
        if let Some(w) = &p.wake {
            w();
        }
        sent
    })
}

pub fn ticket_void() -> bool {
    eravon_vault::glue::task::ticket_void()
}

pub fn void_ticket_fault(at: &std::path::Path) -> Fault {
    Fault::known(crate::fault::Known::HomeUnreachable, crate::lang::both_of(|| crate::lang::filln(crate::lang::Key::TailTicketVoid, &[&at.display().to_string()])))
}

fn hold_tickets(ticket: Option<(u64, std::sync::Arc<std::sync::atomic::AtomicU64>)>, store_era: Option<u64>) {
    eravon_vault::glue::task::set_ticket(ticket);
    eravon_store::atomic::hold_ticket(store_era);
}


fn stale_of(kind: Kind, earlier: bool) -> bool {
    kind.follows_source() && earlier
}


fn start_thread(kind: Kind, work: impl FnOnce() + Send + 'static) -> std::io::Result<std::thread::JoinHandle<()>> {
    start_named(kind.as_str(), work)
}

pub(crate) fn start_named(name: &str, work: impl FnOnce() + Send + 'static) -> std::io::Result<std::thread::JoinHandle<()>> {
    if THREAD_FAILS.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(std::io::Error::new(std::io::ErrorKind::OutOfMemory, "the system would not start a thread (pretended)"));
    }
    std::thread::Builder::new().name(format!("eravon-{name}")).spawn(work)
}

fn stage_clear(k: Kind) {
    STAGES[k as usize].store(0, std::sync::atomic::Ordering::Relaxed);
}

pub use crate::kinds::Kind;

#[derive(Clone, Debug)]
pub enum Done {
    Path(Option<String>),
    Kept(Vec<crate::shell::Kept>),
    CliPath(eravon_os::cli_path::State),
    Seated(Seated),
    BackupMade { path: String, summary: crate::backup::Summary },
    BackupSeen { path: String, summary: crate::backup::Summary },
    Check(crate::probe::Report),
    Archive {
        bytes: u64,
        records: usize,
        machine_items: Result<u64, crate::fault::Fault>,
    },
    Keystore(Keystore),
    Vault(Vault),
    Door(eravon_cli::door::Reply),
    House(Box<crate::house::landed::Landed>),
}

#[derive(Clone, Debug)]
pub enum Vault {
    Opened,
    Changed,
    Recovered,
    Gate(Box<crate::action::Action>),
    Identity { row: crate::identity::Row, restored: bool, fresh: bool, was: Option<(String, crate::roles::Role)> },
    PrimarySet { id: String },
    Restored { summary: crate::backup::Summary },
}

#[derive(Clone, Debug)]
pub enum Keystore {
    BackedUp { path: String, address: crate::key::Address, id: Option<String>, seat: crate::roles::Role },
}

pub struct Outcome {
    pub kind: Kind,
    pub serial: u64,
    pub result: Result<Done, Fault>,
    pub epoch: u64,
    pub stale: bool,
    pub earlier: bool,
    pub nodes: u64,
    pub renoded: bool,
    pub part: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Spawned {
    Started,
    InFlight,
}

impl Spawned {
    pub fn as_str(self) -> &'static str {
        match self {
            Spawned::Started => "started",
            Spawned::InFlight => "in_flight",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reaped {
    pub workers: usize,
    pub joined: usize,
    pub left: usize,
    pub flying: Vec<Kind>,
    pub waited: Vec<Kind>,
    pub not_waited: Vec<Kind>,
}

pub struct Tasks {
    tx: Sender<Outcome>,
    rx: Receiver<Outcome>,
    flying: std::collections::BTreeMap<Kind, u64>,
    epoch: u64,
    epoch_now: std::sync::Arc<std::sync::atomic::AtomicU64>,
    nodes: u64,
    flying_nodes: std::collections::BTreeMap<Kind, u64>,
    landed: std::collections::BTreeMap<Kind, f64>,
    landings: std::collections::BTreeMap<Kind, u64>,
    serial: u64,
    flying_serial: std::collections::BTreeMap<Kind, u64>,
    hands: Vec<(Kind, u64, JoinHandle<()>)>,
    carried: Option<eravon_chain::action::Action>,
    turnless: std::collections::BTreeSet<Kind>,
    waker: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
    started: std::collections::BTreeMap<Kind, u64>,
}

impl Default for Tasks {
    fn default() -> Self {
        Tasks::new()
    }
}

impl Tasks {
    pub fn new() -> Tasks {
        eravon_net::open_up();
        eravon_net::read_choice(crate::machine::proxy_now);
        let (tx, rx) = std::sync::mpsc::channel();
        Tasks {
            tx,
            rx,
            flying: std::collections::BTreeMap::new(),
            epoch: 0,
            epoch_now: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            nodes: 0,
            flying_nodes: std::collections::BTreeMap::new(),
            landed: std::collections::BTreeMap::new(),
            landings: std::collections::BTreeMap::new(),
            serial: 0,
            flying_serial: std::collections::BTreeMap::new(),
            hands: Vec::new(),
            carried: None,
            turnless: std::collections::BTreeSet::new(),
            waker: None,
            started: std::collections::BTreeMap::new(),
        }
    }

    pub fn starts(&self, k: Kind) -> u64 {
        self.started.get(&k).copied().unwrap_or(0)
    }

    pub fn wake_with(&mut self, waker: Option<std::sync::Arc<dyn Fn() + Send + Sync>>) {
        self.waker = waker;
    }

    pub fn polled(&self) -> bool {
        self.flying.keys().any(|k| !k.wakes_when_done())
    }

    pub fn carry(&mut self, action: Option<eravon_chain::action::Action>) {
        self.carried = action;
    }

    pub fn in_flight(&self, k: Kind) -> bool {
        match self.flying.get(&k) {
            Some(e) => (!k.follows_source() || *e == self.epoch) && (!k.reads_nodes() || self.flying_nodes.get(&k) == Some(&self.nodes)),
            None => false,
        }
    }

    pub fn started_over(&self, k: Kind) -> Option<u64> {
        self.flying.get(&k).and(self.flying_nodes.get(&k)).copied()
    }

    pub fn flying(&self) -> Vec<Kind> {
        self.flying.keys().copied().collect()
    }

    pub fn holds_turn(&self) -> bool {
        self.flying.keys().any(|k| k.writes_local() && !self.turnless.contains(k))
    }

    pub fn spawn_turnless(&mut self, kind: Kind, work: impl FnOnce() -> Result<Done, Fault> + Send + 'static) -> Spawned {
        self.start(kind, work, true)
    }

    pub fn nodes_moved(&mut self) {
        self.nodes += 1;
        self.landed.retain(|k, _| !k.reads_nodes());
    }

    pub fn new_epoch(&mut self) {
        self.epoch += 1;
        self.epoch_now.store(self.epoch, std::sync::atomic::Ordering::SeqCst);
        eravon_store::atomic::bump_era();
        self.landed.retain(|k, _| !k.follows_source());
    }

    pub fn attempted(&self, k: Kind) -> bool {
        self.in_flight(k) || self.landed.contains_key(&k)
    }

    pub fn forget(&mut self, k: Kind) {
        self.landed.remove(&k);
    }

    pub fn landed_at(&self, k: Kind) -> Option<f64> {
        self.landed.get(&k).copied()
    }

    pub fn finished_in_flight(&self, k: Kind) -> bool {
        let Some(serial) = self.flying_serial.get(&k) else { return false };
        self.in_flight(k) && self.hands.iter().any(|(x, s, h)| *x == k && s == serial && h.is_finished())
    }

    pub fn serial_of(&self, k: Kind) -> Option<u64> {
        self.flying_serial.get(&k).copied().filter(|_| self.in_flight(k))
    }

    pub fn landings(&self, k: Kind) -> u64 {
        self.landings.get(&k).copied().unwrap_or(0)
    }

    pub fn answered_since(&self, k: Kind, stamp: u64) -> bool {
        self.landings(k) > stamp && !self.in_flight(k)
    }

    pub fn spawn(
        &mut self,
        kind: Kind,
        work: impl FnOnce() -> Result<Done, Fault> + Send + 'static,
    ) -> Spawned {
        self.start(kind, work, false)
    }

    fn start(&mut self, kind: Kind, work: impl FnOnce() -> Result<Done, Fault> + Send + 'static, turnless: bool) -> Spawned {
        if self.in_flight(kind) {
            return Spawned::InFlight;
        }
        self.turnless.remove(&kind);
        if turnless {
            self.turnless.insert(kind);
        }
        self.flying.insert(kind, self.epoch);
        let nodes = self.nodes;
        if kind.reads_nodes() {
            self.flying_nodes.insert(kind, nodes);
        }
        self.serial += 1;
        *self.started.entry(kind).or_insert(0) += 1;
        let serial = self.serial;
        self.flying_serial.insert(kind, serial);
        stage_clear(kind);
        let epoch = self.epoch;
        let tx = self.tx.clone();
        let failed_tx = self.tx.clone();
        let ticket = kind.follows_source().then(|| (epoch, self.epoch_now.clone()));
        let store_era = kind.follows_source().then(eravon_store::atomic::era);
        let keeper = kind.keeps_reads().then(Kind::keeper).flatten();
        let carried = self.carried.clone().filter(|_| kind.carries());
        let wake = self.waker.clone().filter(|_| kind.wakes_when_done());
        let part = Part { kind, epoch, nodes, tx: self.tx.clone(), wake: self.waker.clone() };
        let started = start_thread(kind, move || {
            hold_tickets(ticket, store_era);
            PART.with(|p| *p.borrow_mut() = Some(part));
            let result = {
                let _action = match carried {
                    Some(a) => eravon_chain::action::enter(a),
                    None => eravon_chain::action::open(keeper),
                };
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
                    Ok(r) => r,
                    Err(p) => Err(Fault::known(
                        crate::fault::Known::WorkerPanicked,
                        crate::lang::both_of(|| format!("{} · {}", crate::lang::filln(crate::lang::Key::Tail219, &[&(kind.as_str()).to_string()]), panic_said(p.as_ref()))),
                    )),
                }
            };
            stage_clear(kind);
            let _ = tx.send(Outcome { kind, serial, result, epoch, stale: false, earlier: false, nodes, renoded: false, part: false });
            if let Some(w) = wake {
                w();
            }
        });
        match started {
            Ok(h) => self.hands.push((kind, serial, h)),
            Err(e) => {
                stage_clear(kind);
                let f = Fault::unknown(crate::lang::filln(crate::lang::Key::Tail236, &[&crate::lang::filln(crate::lang::Key::Tail219, &[&kind.as_str().to_string()]), &e.to_string()]));
                let _ = failed_tx.send(Outcome { kind, serial, result: Err(f), epoch, stale: false, earlier: false, nodes, renoded: false, part: false });
            }
        }
        Spawned::Started
    }

    pub fn drain(&mut self) -> Vec<Outcome> {
        self.drain_at(0.0)
    }

    pub fn drain_at(&mut self, now: f64) -> Vec<Outcome> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(mut o) if o.part => {
                    o.earlier = o.epoch != self.epoch;
                    o.renoded = o.kind.reads_nodes() && o.nodes != self.nodes;
                    o.stale = (o.kind.follows_source() && o.earlier) || o.renoded;
                    out.push(o);
                }
                Ok(mut o) => {
                    let this = !o.kind.reads_nodes() || self.flying_nodes.get(&o.kind) == Some(&o.nodes);
                    if self.flying.get(&o.kind) == Some(&o.epoch) && this {
                        self.flying.remove(&o.kind);
                        self.flying_nodes.remove(&o.kind);
                        self.turnless.remove(&o.kind);
                    }
                    o.earlier = o.epoch != self.epoch;
                    o.renoded = o.kind.reads_nodes() && o.nodes != self.nodes;
                    o.stale = stale_of(o.kind, o.earlier) || o.renoded;
                    if !o.stale {
                        self.landed.insert(o.kind, now);
                        *self.landings.entry(o.kind).or_insert(0) += 1;
                    }
                    out.push(o);
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        self.hands.retain(|(_, _, h)| !h.is_finished());
        out
    }

    pub fn workers(&self) -> usize {
        self.hands.len()
    }

    pub fn shutdown(&mut self) -> Reaped {
        eravon_net::close_down();
        let hands: Vec<(Kind, JoinHandle<()>)> = std::mem::take(&mut self.hands).into_iter().filter(|(_, _, h)| !h.is_finished()).map(|(k, _, h)| (k, h)).collect();
        let workers = hands.len();
        let flying: Vec<Kind> = hands.iter().map(|(k, _)| *k).collect();
        let (mut joined, mut waited, mut not_waited) = (0, Vec::new(), Vec::new());
        for (k, h) in hands {
            if k.waited_at_quit() {
                waited.push(k);
                if h.join().is_ok() {
                    joined += 1;
                }
            } else {
                not_waited.push(k);
            }
        }
        self.flying.clear();
        self.flying_nodes.clear();
        Reaped { workers, joined, left: workers - joined, flying, waited, not_waited }
    }
}

fn panic_said(p: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        return s.to_string();
    }
    if let Some(s) = p.downcast_ref::<String>() {
        return s.clone();
    }
    crate::lang::t(crate::lang::Key::Tail219Opaque).to_string()
}

#[derive(Clone)]
pub struct Seated(pub std::sync::Arc<std::sync::Mutex<Option<crate::action::Seating>>>);

impl Seated {
    pub fn new(s: crate::action::Seating) -> Seated {
        Seated(std::sync::Arc::new(std::sync::Mutex::new(Some(s))))
    }

    pub fn take(&self) -> Option<crate::action::Seating> {
        self.0.lock().ok().and_then(|mut s| s.take())
    }
}

impl std::fmt::Debug for Seated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Seated")
    }
}
