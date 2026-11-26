use std::sync::Mutex;

static INBOX: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub const fn delivery() -> &'static str {
    crate::house::platform::LINK_DELIVERY
}

static WAKE: Mutex<Option<Box<dyn Fn() + Send + Sync>>> = Mutex::new(None);

pub fn on_arrival(f: impl Fn() + Send + Sync + 'static) {
    if let Ok(mut w) = WAKE.lock() {
        *w = Some(Box::new(f));
    }
}

pub fn push(url: String) {
    if let Ok(mut q) = INBOX.lock() {
        q.push(url);
    }
    if let Ok(w) = WAKE.lock()
        && let Some(f) = w.as_ref()
    {
        f();
    }
}

fn door_dir(root: &str) -> std::path::PathBuf {
    std::path::Path::new(root).join("link-door")
}

static DOOR: Mutex<Option<eravon_os::LockFile>> = Mutex::new(None);

pub fn hold_door(root: &str) -> bool {
    let dir = door_dir(root);
    if std::fs::create_dir_all(&dir).is_err() {
        return true;
    }
    let mut o = eravon_os::Options::new();
    o.read(true).write(true).create(true);
    let Ok(f) = eravon_os::LockFile::open(eravon_os::owner_only(&mut o), &dir.join("open.lock")) else { return true };
    if !eravon_os::lock_now(f.file()) {
        return false;
    }
    let watch = crate::task::start_named("link-door", move || loop {
        for u in take_left(&dir) {
            push(u);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    });
    if watch.is_ok()
        && let Ok(mut d) = DOOR.lock()
    {
        *d = Some(f);
    }
    true
}

pub fn leave(root: &str, url: &str) -> std::io::Result<()> {
    let dir = door_dir(root);
    let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let name = format!("{at:030}-{}", std::process::id());
    let part = dir.join(format!("{name}.part"));
    std::fs::write(&part, url.trim().as_bytes())?;
    eravon_os::replace(&part, &dir.join(format!("{name}.url")))
}

fn take_left(dir: &std::path::Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut left: Vec<std::path::PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "url")).collect();
    left.sort();
    let mut out = Vec::new();
    for p in left {
        if let Ok(s) = std::fs::read_to_string(&p) {
            let _ = std::fs::remove_file(&p);
            out.push(s.trim().to_string());
        }
    }
    out
}

pub fn drain() -> Vec<String> {
    match INBOX.lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(_) => Vec::new(),
    }
}

pub fn arm() -> Result<(), String> {
    crate::house::platform::arm_links()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_queue_hands_over_in_arrival_order_and_empties() {
        push("eravon://open?e=AAAA".into());
        push("eravon://open?e=BBBB".into());
        let got = drain();
        assert_eq!(got.len(), 2);
        assert!(got[0].ends_with("AAAA"));
        assert!(got[1].ends_with("BBBB"));
        assert!(drain().is_empty());
    }

    fn root(tag: &str) -> std::path::PathBuf {
        let r = std::env::temp_dir().join(format!("eravon-door-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        std::fs::create_dir_all(&r).expect("临时根");
        r
    }

    #[test]
    fn the_door_is_held_by_the_first_and_found_held_by_the_next() {
        let r = root("held");
        assert!(hold_door(&r.display().to_string()), "the first holds the door");
        assert!(!hold_door(&r.display().to_string()), "the next finds it held");
    }

    #[test]
    fn links_left_at_the_door_come_out_in_order_once_and_whole() {
        let r = root("left");
        let at = r.display().to_string();
        std::fs::create_dir_all(door_dir(&at)).expect("门");
        leave(&at, " eravon://open?e=FIRST\n").expect("留下一条");
        leave(&at, "eravon://open?e=SECOND").expect("再留一条");
        std::fs::write(door_dir(&at).join("9-1.part"), b"eravon://open?e=HALF").expect("写半截");
        assert_eq!(take_left(&door_dir(&at)), vec!["eravon://open?e=FIRST".to_string(), "eravon://open?e=SECOND".to_string()]);
        assert!(take_left(&door_dir(&at)).is_empty(), "taken once");
        assert!(door_dir(&at).join("9-1.part").exists(), "a link being written is not read");
    }
}
