use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eravon_literals::json::Out;

use crate::atomic;
use crate::doc;
use crate::fault::{Fault, R};
use crate::lock::{self, Mark, Mode};
use crate::seam;

pub struct Home {
    root: PathBuf,
}

impl Home {
    pub fn of(data: &str) -> Home {
        let t = data.trim_end_matches('/');
        Home { root: PathBuf::from(if t.is_empty() { data } else { t }) }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn dir(&self, s: Slot) -> PathBuf {
        match s {
            Slot::Settings => self.root.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Settings,
}

pub fn put_at(dir: &Path, name: &str, bytes: &[u8]) -> R<()> {
    atomic::write_atomic(&dir.join(name).to_string_lossy(), bytes)
}

pub fn same_place(a: &Path, b: &Path) -> bool {
    eravon_os::same_folder(a, b)
}

pub fn fill_random(buf: &mut [u8]) -> R<()> {
    eravon_os::fill_random(buf).map_err(|e| crate::fault::classify(&e, eravon_os::ENTROPY_SOURCE))
}

pub mod hexfmt {
    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(2 + 2 * bytes.len());
        s.push_str("0x");
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

pub(crate) const NO_HOLDER: &str = "(锁档上没写是谁)";

pub(crate) fn other_machine_said(at: &str) -> String {
    format!("{at} 记的是另一台机器")
}

pub fn lock_now(file: &std::fs::File) -> bool {
    eravon_os::lock_now(file)
}


pub fn lock_wait(file: &std::fs::File) -> bool {
    eravon_os::lock_wait(file)
}

static MACHINE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn machine_at(dir: Option<PathBuf>) {
    *MACHINE.lock().unwrap_or_else(|e| e.into_inner()) = dir;
}

pub fn machine_dir() -> R<PathBuf> {
    let d = match MACHINE.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        Some(d) => d,
        None => dir_of_home(eravon_os::user_home())?,
    };
    clear_empty_claim(&d);
    Ok(d)
}

fn clear_empty_claim(dir: &std::path::Path) {
    let at = dir.join(crate::lock::MACHINE_FILE);
    if std::fs::symlink_metadata(&at).is_ok_and(|m| m.is_file() && m.len() == 0) {
        let _ = std::fs::remove_file(&at);
    }
}

fn dir_of_home(home: Option<PathBuf>) -> R<PathBuf> {
    let home = home.ok_or_else(|| Fault::Machine("本机标没处放:系统没给出家目录".into()))?;
    match eravon_os::machine::of(&home) {
        Ok((dir, _)) => Ok(dir),
        Err(eravon_os::machine::Unread::NoHome) => Err(Fault::Machine("本机标没处放:系统没给出家目录".into())),
        Err(eravon_os::machine::Unread::PointerShape(at)) => Err(Fault::Machine(format!("本机目录的指针不是一行路径:{}", at.display()))),
        Err(eravon_os::machine::Unread::Pointer(at, e)) => Err(Fault::Machine(format!("本机目录的指针读不出:{}:{e}", at.display()))),
    }
}

pub fn mark_read(data: &str) -> Mark {
    lock::mark_read(&Home::of(data))
}

pub fn mark_of(data: &str) -> R<Option<String>> {
    Ok(lock::mark_of(&Home::of(data)))
}

pub fn mark_trouble(data: &str) -> Option<Fault> {
    match mark_read(data) {
        Mark::Unread(f) => Some(f),
        Mark::None | Mark::Machine(_) => homes(data).ok()?.iter().find_map(|h| match lock::mark_read(h) {
            Mark::Unread(f) => Some(f),
            Mark::None | Mark::Machine(_) => None,
        }),
    }
}

pub fn other_machine(data: &str) -> R<Option<String>> {
    let at = machine_dir()?.join(lock::MACHINE_FILE).to_string_lossy().into_owned();
    let me = match atomic::read_file(&at) {
        Ok(b) => String::from_utf8_lossy(&b).trim().to_string(),
        Err(Fault::Missing(_)) => String::new(),
        Err(e) => return Err(e),
    };
    let homes = homes(data)?;
    let marks = std::iter::once(mark_of(data)?).chain(homes.iter().map(lock::mark_of));
    Ok(marks.flatten().find(|m| *m != me))
}

fn homes(data: &str) -> R<Vec<Home>> {
    let root = data.trim_end_matches('/');
    let mut out = Vec::new();
    for name in atomic::entries(root)? {
        let d = format!("{root}/{name}");
        if name.len() != 40 || !name.bytes().all(|b| b.is_ascii_hexdigit()) || !Path::new(&d).is_dir() {
            continue;
        }
        for seat in atomic::entries(&d)? {
            let at = format!("{d}/{seat}/settings");
            if Path::new(&at).is_dir() {
                out.push(Home::of(&at));
            }
        }
    }
    Ok(out)
}

fn heed_homes(data: &str) -> R<()> {
    let me = lock::this_machine()?;
    let mut unmarked = Vec::new();
    for h in homes(data)? {
        match lock::mark_read(&h) {
            Mark::Machine(m) if m == me => {}
            Mark::None => unmarked.push(h),
            Mark::Machine(m) => {
                return Err(Fault::OtherMachine(format!("这一家由另一台机器在写(那台机器的标 {m});要在这台机器写,先改由本机写")));
            }
            Mark::Unread(f) => {
                return Err(Fault::OtherMachine(format!("写者标读不懂:{};标原样留着,要在这台机器写,先改由本机写", f.detail())));
            }
        }
    }
    for h in &unmarked {
        lock::write_mark(h)?;
    }
    Ok(())
}

fn attempts_path(data: &str) -> R<String> {
    let name = seam::machine_attempts_name(&place(data).to_string_lossy());
    Ok(machine_dir()?.join(name).to_string_lossy().into_owned())
}

pub fn machine_attempts(data: &str) -> R<u32> {
    let at = attempts_path(data)?;
    match atomic::read_file(&at) {
        Ok(b) => std::str::from_utf8(&b)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .ok_or_else(|| Fault::Machine(format!("{at} 不是一个错次数"))),
        Err(Fault::Missing(_)) => Ok(0),
        Err(e) => Err(e),
    }
}

pub struct AttemptsHeld {
    _held: eravon_os::LockFile,
}

pub fn hold_machine_attempts(data: &str) -> R<AttemptsHeld> {
    let dir = machine_dir()?;
    atomic::mkdirs(&dir.to_string_lossy())?;
    let file = atomic::open_lock(&format!("{}.lock", attempts_path(data)?))?;
    if !lock::grab_waiting(file.file()) {
        return Err(Fault::Locked { holder: "(本机错次记录的锁等不到)".into() });
    }
    Ok(AttemptsHeld { _held: file })
}

pub fn set_machine_attempts(data: &str, n: u32) -> R<()> {
    let dir = machine_dir()?;
    atomic::mkdirs(&dir.to_string_lossy())?;
    atomic::write_atomic(&attempts_path(data)?, format!("{n}\n").as_bytes())
}

struct Held {
    lock: Option<lock::Lock>,
    mode: Mode,
    holder: String,
    said: bool,
    trouble: Option<Fault>,
}

static HELD: Mutex<Vec<(PathBuf, Arc<Mutex<Held>>)>> = Mutex::new(Vec::new());

fn place(data: &str) -> PathBuf {
    std::fs::canonicalize(data).unwrap_or_else(|_| PathBuf::from(data.trim_end_matches('/')))
}

fn registered(data: &str) -> Option<Arc<Mutex<Held>>> {
    let p = place(data);
    HELD.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| *k == p).map(|(_, h)| h.clone())
}

pub fn take(data: &str) -> R<Mode> {
    if let Some(h) = registered(data) {
        let mut g = h.lock().unwrap_or_else(|e| e.into_inner());
        if g.mode != Mode::Reader {
            return Ok(g.mode);
        }
        g.lock = None;
        *g = taken(data)?;
        return Ok(g.mode);
    }
    let held = taken(data)?;
    let got = held.mode != Mode::Reader;
    let mut g = HELD.lock().unwrap_or_else(|e| e.into_inner());
    let p = place(data);
    if let Some((_, h)) = g.iter_mut().find(|(k, _)| *k == p) {
        let mut was = h.lock().unwrap_or_else(|e| e.into_inner());
        if got && was.mode == Mode::Reader {
            *was = held;
        }
        return Ok(was.mode);
    }
    let mode = held.mode;
    g.push((p, Arc::new(Mutex::new(held))));
    Ok(mode)
}

fn taken(data: &str) -> R<Held> {
    atomic::mkdirs(data)?;
    let home = Home::of(data);
    let mut l = lock::take(&home)?;
    heeded(&mut l, &home)?;
    let mode = l.mode();
    let (holder, trouble) = match mode {
        Mode::Writer => (String::new(), None),
        Mode::Reader => (l.holder().to_string(), None),
        Mode::OtherMachine => match l.mark_trouble() {
            Some(f) => (f.detail(), Some(f.clone())),
            None => (lock::mark_of(&home).unwrap_or_default(), None),
        },
    };
    if mode == Mode::Writer {
        finish_restore(data)?;
        sweep_home(data)?;
    }
    Ok(Held { lock: Some(l), mode, holder, said: false, trouble })
}

fn heeded(_l: &mut lock::Lock, _home: &Home) -> R<()> {
    Ok(())
}


pub fn finish_restore(data: &str) -> R<bool> {
    let next = seam::restore_next(data);
    if !atomic::exists(&next) {
        return Ok(false);
    }
    if !atomic::exists(&format!("{next}/{}", seam::F_RESTORE_READY)) {
        atomic::remove_dir_all(&next)?;
        return Ok(false);
    }
    let note = format!("{next}/{}", seam::F_RESTORE_ASIDE);
    let aside = match atomic::read_file(&note) {
        Ok(b) => String::from_utf8_lossy(&b).trim().to_string(),
        Err(Fault::Missing(_)) => {
            let at = (1u64..).map(|n| seam::restore_aside(data, n)).find(|a| !atomic::exists(a)).unwrap_or_default();
            atomic::write_atomic(&note, at.as_bytes())?;
            at
        }
        Err(e) => return Err(e),
    };
    atomic::mkdirs(&aside)?;
    let root = data.trim_end_matches('/');
    for name in [seam::D_IDENTITIES, seam::F_SETTINGS, seam::F_VAULT] {
        let (now, staged, put) = (format!("{root}/{name}"), format!("{next}/{name}"), format!("{aside}/{name}"));
        if !atomic::exists(&staged) {
            continue;
        }
        if atomic::exists(&now) {
            atomic::rename(&now, &put)?;
        }
        atomic::rename(&staged, &now)?;
    }
    let_go(&next);
    atomic::remove_dir_all(&next)?;
    Ok(true)
}

fn sweep_home(data: &str) -> R<()> {
    atomic::sweep(data)?;
    for name in atomic::entries(data)? {
        let d = format!("{data}/{name}");
        if Path::new(&d).is_dir() && !Path::new(&d).is_symlink() {
            sweep_home(&d)?;
        }
    }
    Ok(())
}

pub fn need_writer(data: &str, writer: &str, now: u64) -> R<()> {
    take(data)?;
    let Some(h) = registered(data) else { return Err(Fault::Locked { holder: "(没取到锁)".into() }) };
    let mut h = h.lock().unwrap_or_else(|e| e.into_inner());
    match h.mode {
        Mode::Writer => {}
        Mode::Reader => return Err(Fault::Locked { holder: h.holder.clone() }),
        Mode::OtherMachine if h.trouble.is_some() => {
            return Err(Fault::OtherMachine(format!("写者标读不懂:{};标原样留着,要在这台机器写,先改由本机写", h.holder)));
        }
        Mode::OtherMachine => {
            return Err(Fault::OtherMachine(format!("这一家由另一台机器在写(那台机器的标 {});要在这台机器写,先改由本机写", h.holder)));
        }
    }
    heed_homes(data)?;
    if !h.said {
        let f = atomic::open_lock(&seam::lock_path(data))?;
        atomic::say_on(f.file(), &words(writer, now))?;
        h.said = true;
    }
    Ok(())
}

fn words(writer: &str, now: u64) -> Vec<u8> {
    doc::seal(
        seam::V_LOCK,
        vec![
            ("pid", Out::N(u64::from(std::process::id()))),
            ("since", Out::N(now)),
            ("writer", Out::S(writer.to_string())),
        ],
    )
}

pub fn take_over(data: &str) -> R<Mode> {
    take(data)?;
    let Some(h) = registered(data) else { return Err(Fault::Locked { holder: "(没取到锁)".into() }) };
    let mut h = h.lock().unwrap_or_else(|e| e.into_inner());
    if h.mode == Mode::OtherMachine {
        if let Some(l) = h.lock.as_mut() {
            l.take_over(&Home::of(data))?;
            h.mode = l.mode();
            h.holder = String::new();
            h.trouble = None;
        }
    }
    if h.mode == Mode::Writer {
        for home in homes(data)? {
            lock::write_mark(&home)?;
        }
    }
    Ok(h.mode)
}

pub fn trouble_of(data: &str) -> Option<Fault> {
    registered(data).and_then(|h| h.lock().unwrap_or_else(|e| e.into_inner()).trouble.clone())
}

pub fn mode_of(data: &str) -> Option<Mode> {
    registered(data).map(|h| h.lock().unwrap_or_else(|e| e.into_inner()).mode)
}

pub fn let_go(data: &str) {
    let p = place(data);
    let gone: Vec<_> = {
        let mut g = HELD.lock().unwrap_or_else(|e| e.into_inner());
        let (out, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut *g).into_iter().partition(|(k, _)| *k == p);
        *g = keep;
        out
    };
    drop(gone);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    pub pid: u32,
    pub since: u64,
    pub writer: String,
}

pub fn probe(data: &str) -> R<Option<Words>> {
    if mode_of(data).is_some_and(|m| m != Mode::Reader) {
        return Ok(None);
    }
    let file = match atomic::open_lock_existing(&seam::lock_path(data))? {
        Some(f) => f,
        None => return Ok(None),
    };
    let free = lock::grab(file.file());
    drop(file);
    Ok(if free { None } else { Some(read_words(data)) })
}

fn read_words(data: &str) -> Words {
    let unread = Words { pid: 0, since: 0, writer: NO_HOLDER.into() };
    let Ok(b) = atomic::read_file(&seam::lock_path(data)) else { return unread };
    let Ok(v) = doc::open(seam::V_LOCK, &b) else { return unread };
    Words {
        pid: doc::need_u64(&v, "pid").ok().and_then(|p| u32::try_from(p).ok()).unwrap_or(0),
        since: doc::need_u64(&v, "since").unwrap_or(0),
        writer: doc::need_str(&v, "writer").unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TURN: Mutex<()> = Mutex::new(());

    fn turn() -> std::sync::MutexGuard<'static, ()> {
        let g = TURN.lock().unwrap_or_else(|e| e.into_inner());
        machine_at(Some(std::env::temp_dir().join(format!("eravon-lock-test-machine-{}", std::process::id()))));
        g
    }

    fn home(tag: &str) -> String {
        let d = std::env::temp_dir().join(format!("eravon-lock-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch");
        d.display().to_string()
    }

    #[cfg(unix)]
    #[test]
    fn a_lock_let_go_while_children_start_is_taken_again_as_a_writer() {
        let _t = turn();
        let d = home("children");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let starters: Vec<_> = (0..4)
            .map(|_| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    let mut n = 0usize;
                    while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                        if let Ok(mut c) = eravon_os::spawn(std::process::Command::new("/usr/bin/true").stdin(std::process::Stdio::null())) {
                            let _ = c.wait();
                            n += 1;
                        }
                    }
                    n
                })
            })
            .collect();
        let mut readers = 0usize;
        for _ in 0..1000 {
            if take(&d).expect("taken") != Mode::Writer {
                readers += 1;
            }
            let_go(&d);
        }
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let started: usize = starters.into_iter().map(|h| h.join().expect("a starter")).sum();
        assert!(started > 0, "children were started meanwhile");
        assert_eq!(readers, 0, "never a reader of its own data root");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_stale_lock_file_decides_nothing_and_one_take_is_held() {
        let _t = turn();
        let d = home("stale");
        std::fs::write(seam::lock_path(&d), b"{\"pid\":1,\"since\":1,\"v\":\"eravon.store.lock/1\",\"writer\":\"gone\"}").expect("a stale lock file");
        assert_eq!(take(&d).expect("taken"), Mode::Writer);
        assert_eq!(take(&d).expect("again"), Mode::Writer);
        need_writer(&d, "w", 9).expect("the writer writes");
        assert!(atomic::exists(&seam::lock_path(&d)), "the lock file stays");
        assert_eq!(mark_of(&d).expect("read"), Some(lock::this_machine().expect("me")));
        assert_eq!(read_words(&d).writer, "w", "the writer's words are in the lock file");
        let_go(&d);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_reader_takes_the_lock_once_it_is_let_go() {
        let _t = turn();
        let d = home("reader");
        let other = atomic::open_lock(&seam::lock_path(&d)).expect("another descriptor");
        assert!(eravon_os::lock_now(other.file()), "something else holds it");
        assert_eq!(take(&d).expect("taken"), Mode::Reader);
        assert_eq!(need_writer(&d, "w", 9).err().expect("refused").name(), "E_STORE_LOCKED");
        assert!(probe(&d).expect("looked").is_some(), "held by another");
        drop(other);
        assert_eq!(probe(&d).expect("looked"), None, "a reader's look asks the kernel now");
        need_writer(&d, "w", 9).expect("let go: this instance writes now");
        assert_eq!(take(&d).expect("again"), Mode::Writer);
        let_go(&d);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn a_reader_not_made_writer_lets_the_lock_go() {
        let _t = turn();
        let d = home("reader-fail");
        let other = atomic::open_lock(&seam::lock_path(&d)).expect("another descriptor");
        assert!(eravon_os::lock_now(other.file()), "something else holds it");
        assert_eq!(take(&d).expect("taken"), Mode::Reader);
        drop(other);
        use std::os::unix::fs::PermissionsExt;
        let ids = format!("{d}/home");
        std::fs::create_dir_all(&ids).expect("a home");
        std::fs::set_permissions(&ids, std::fs::Permissions::from_mode(0o000)).expect("unreadable");
        let refused = take(&d).is_err();
        std::fs::set_permissions(&ids, std::fs::Permissions::from_mode(0o700)).expect("readable again");
        assert!(refused, "not made writer");
        let again = atomic::open_lock(&seam::lock_path(&d)).expect("a third descriptor");
        assert!(eravon_os::lock_now(again.file()), "the lock was let go");
        drop(again);
        let_go(&d);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn another_machines_mark_is_read_only_until_taken_over() {
        let _t = turn();
        let d = home("other");
        let other = "0123456789abcdef0123456789abcdef";
        std::fs::write(seam::writer_mark_path(&d), format!("{other}\n")).expect("another machine's mark");
        assert_eq!(take(&d).expect("taken"), Mode::OtherMachine);
        assert_eq!(other_machine(&d).expect("read"), Some(other.to_string()));
        let e = need_writer(&d, "w", 9).err().expect("refused");
        assert_eq!(e.name(), "E_STORE_OTHER_MACHINE");
        assert!(e.detail().contains(other), "{}", e.detail());
        assert_eq!(take_over(&d).expect("taken over"), Mode::Writer);
        need_writer(&d, "w", 9).expect("written here now");
        assert_eq!(mark_of(&d).expect("read"), Some(lock::this_machine().expect("me")));
        assert_eq!(other_machine(&d).expect("read"), None, "this machine's own mark is no other machine");
        let_go(&d);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_homes_mark_of_another_machine_bars_the_writer_until_taken_over() {
        let _t = turn();
        let d = home("homes");
        let (a, b) = (format!("{d}/{}/seller/settings", "ab".repeat(20)), format!("{d}/{}/buyer/settings", "cd".repeat(20)));
        for x in [&a, &b] {
            std::fs::create_dir_all(x).expect("a home's settings");
        }
        let other = "0123456789abcdef0123456789abcdef";
        std::fs::write(format!("{a}/{}", lock::MARK_FILE), format!("{other}\n")).expect("another machine's mark");
        assert_eq!(take(&d).expect("taken"), Mode::Writer, "the data root's own mark is none");
        assert_eq!(other_machine(&d).expect("read"), Some(other.to_string()));
        let e = need_writer(&d, "w", 9).err().expect("refused");
        assert_eq!(e.name(), "E_STORE_OTHER_MACHINE");
        assert!(e.detail().contains(other), "{}", e.detail());
        assert_eq!(take_over(&d).expect("taken over"), Mode::Writer);
        need_writer(&d, "w", 9).expect("written here now");
        let me = lock::this_machine().expect("me");
        for x in [&a, &b] {
            assert_eq!(lock::mark_of(&Home::of(x)), Some(me.clone()), "{x}");
        }
        assert_eq!(other_machine(&d).expect("read"), None);
        let_go(&d);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_later_home_that_bars_the_write_leaves_the_earlier_ones_unmarked() {
        let _t = turn();
        let d = home("homes-whole");
        let (first, later) = (format!("{d}/{}/seller/settings", "ab".repeat(20)), format!("{d}/{}/buyer/settings", "cd".repeat(20)));
        for x in [&first, &later] {
            std::fs::create_dir_all(x).expect("a home's settings");
        }
        let other = "0123456789abcdef0123456789abcdef";
        std::fs::write(format!("{later}/{}", lock::MARK_FILE), format!("{other}\n")).expect("another machine's mark");
        assert_eq!(take(&d).expect("taken"), Mode::Writer, "the data root's own mark is none");
        let e = need_writer(&d, "w", 9).err().expect("refused");
        assert_eq!(e.name(), "E_STORE_OTHER_MACHINE");
        assert_eq!(lock::mark_of(&Home::of(&first)), None, "the earlier home is not marked by a refused write");
        assert_eq!(lock::mark_of(&Home::of(&later)), Some(other.to_string()), "the barring mark is left as it is");
        assert_eq!(take_over(&d).expect("taken over"), Mode::Writer);
        need_writer(&d, "w", 9).expect("written here now");
        assert_eq!(lock::mark_of(&Home::of(&first)), Some(lock::this_machine().expect("me")), "marked once the write goes through");
        let_go(&d);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn no_home_is_nowhere_to_keep_the_mark() {
        assert_eq!(dir_of_home(None).err().expect("refused").name(), "E_STORE_MACHINE");
    }

    #[test]
    fn a_writer_mark_that_does_not_read_opens_read_only_and_is_kept() {
        let _t = turn();
        let me = lock::this_machine().expect("me");
        for (form, bytes) in [("wrongShape", "ABC\n".to_string()), ("laterVersion", format!("{{\"v\":\"eravon.writer-mark/2\",\"machine\":\"{me}\"}}\n"))] {
            let d = home(&format!("unread-{form}"));
            let at = seam::writer_mark_path(&d);
            std::fs::write(&at, &bytes).expect("a mark that does not read");
            assert_eq!(take(&d).expect("taken"), Mode::OtherMachine, "{form}");
            assert!(trouble_of(&d).is_some() && matches!(mark_read(&d), Mark::Unread(_)), "{form}: named as unread");
            let e = need_writer(&d, "w", 9).err().expect("refused");
            assert_eq!(e.name(), "E_STORE_OTHER_MACHINE", "{form}");
            assert!(e.detail().starts_with("写者标读不懂"), "{form}: {}", e.detail());
            assert_eq!(std::fs::read_to_string(&at).expect("kept"), bytes, "{form}: not written over");
            let_go(&d);
            let _ = std::fs::remove_dir_all(&d);
        }
        let d = home("unread-dir");
        let at = seam::writer_mark_path(&d);
        std::fs::create_dir_all(&at).expect("a directory at the mark's name");
        assert_eq!(take(&d).expect("taken"), Mode::OtherMachine, "a directory");
        assert!(trouble_of(&d).is_some());
        assert!(std::path::Path::new(&at).is_dir(), "left as it is");
        let_go(&d);
        std::fs::remove_dir(&at).expect("cleared");
        std::fs::write(&at, b"ABC\n").expect("a wrong shape again");
        assert_eq!(take(&d).expect("taken"), Mode::OtherMachine);
        assert_eq!(take_over(&d).expect("taken over"), Mode::Writer, "the person's take-over");
        assert!(trouble_of(&d).is_none());
        need_writer(&d, "w", 9).expect("written here now");
        assert_eq!(mark_of(&d).expect("read"), Some(me));
        let_go(&d);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_empty_machine_mark_is_made_again_and_other_words_are_refused() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = std::env::temp_dir().join(format!("evst-claim-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("machine");
        machine_at(Some(d.clone()));
        let at = d.join(crate::lock::MACHINE_FILE);
        std::fs::write(&at, b"").expect("an empty claim");
        let made = lock::this_machine().expect("made again");
        assert_eq!(std::fs::read_to_string(&at).expect("there").trim(), made);
        std::fs::write(&at, b"not a mark\n").expect("other words");
        assert!(lock::this_machine().is_err(), "refused by name");
        assert_eq!(std::fs::read(&at).expect("there"), b"not a mark\n".to_vec(), "left as it was");
        machine_at(None);
        let _ = std::fs::remove_dir_all(&d);
    }
}
