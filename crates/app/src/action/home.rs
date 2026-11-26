use super::*;

pub fn start(shell: &mut Shell) -> String {
    if !shell.unlocked() {
        return String::new();
    }
    let root = boot_home(shell);
    if let Err(f) = shell.refresh_anchor() {
        shell.faults.push(f);
    }
    apply(shell, Action::ReadIdentities);
    root
}

pub fn boot_home(shell: &mut Shell) -> String {
    let landed = match crate::register::change_listed(crate::identity::land_at_boot).map(Option::flatten) {
        Ok(l) => l,
        Err(f) => {
            shell.faults.push(f);
            None
        }
    };
    let root = match crate::register::read().and_then(|listed| crate::identity::home_now(listed.as_ref())) {
        Ok(Some(root)) => root.display().to_string(),
        Ok(None) => String::new(),
        Err(f) => {
            shell.faults.push(f);
            String::new()
        }
    };
    if !root.is_empty() {
        let _ = apply(shell, Action::OpenHome { root: root.clone() });
    }
    if let Some((_, seat)) = landed {
        if shell.settings.role != seat {
            if shell.home.is_some() {
                if let Err(f) = shell.commit_settings(|s| s.role = seat) {
                    shell.faults.push(f);
                }
            } else {
                shell.settings.role = seat;
            }
        }
    }
    root
}

pub(super) fn set_machine_aside(shell: &mut Shell) -> Applied {
    let now = crate::shell::Vault::of(crate::keybox::state());
    let damaged = matches!(now, crate::shell::Vault::Damaged(_));
    shell.vault = now;
    if !damaged {
        let at = crate::home::machine_dir().map(|p| p.display().to_string()).unwrap_or_default();
        return shell.trouble(crate::fault::Fault::known(crate::fault::Known::KeyboxNotEmpty, at));
    }
    let day = crate::when::day(now_secs());
    match crate::home::set_machine_aside(&day) {
        Ok(a) => {
            if let Err(f) = crate::home::settle_machine() {
                shell.faults.push(f);
            }
            shell.after_lock();
            Applied::MachineAside { from: a.from.display().to_string(), to: a.to.display().to_string() }
        }
        Err(f) => shell.trouble(f),
    }
}

pub(super) fn take_writer(shell: &mut Shell) -> Result<(String, crate::lock::Mode), crate::fault::Fault> {
    let no_home = || crate::fault::Fault::known(crate::fault::Known::NoHome, crate::lang::both_of(|| crate::lang::t(crate::lang::Key::Tail005).to_string()));
    let home = shell.home.as_ref().ok_or_else(no_home)?;
    let lock = shell.lock.as_mut().ok_or_else(no_home)?;
    let root = home.root().display().to_string();
    if lock.mode() != crate::lock::Mode::OtherMachine {
        return Ok((root, lock.mode()));
    }
    let as_seat = opened_as(home.root());
    let as_seat = as_seat.as_ref().map(|(i, s)| (i.as_str(), *s));
    crate::local::check_label(home.root(), as_seat, false)?;
    crate::lock::take_over(lock, home)?;
    let label = crate::local::check_label(home.root(), as_seat, true);
    if let Err(f) = label {
        shell.faults.push(f);
    }
    Ok((root, crate::lock::Mode::Writer))
}

fn opened_as(root: &std::path::Path) -> Option<(String, crate::roles::Role)> {
    let (row, seat) = crate::register::now_row_listed().ok().flatten()?;
    row.home(seat).filter(|h| crate::home::same_place(h, root)).map(|_| (row.id.clone(), seat))
}

pub fn reopen_here(shell: &mut Shell, root: &std::path::Path) -> Result<crate::lock::Mode, crate::fault::Fault> {
    shell.lock = None;
    open_home_at(shell, &root.display().to_string(), false)
}

pub(super) fn change_home(shell: &mut Shell, root: &str) -> Result<crate::lock::Mode, crate::fault::Fault> {
    let at = crate::home::landing(root)?;
    crate::home::may_open_at(&at)?;
    let Some((id, seat, left)) = seat_home_left(&at)? else { return open_home(shell, root) };
    let was = follow(&id, seat, &left, &at, &at.display().to_string())?;
    match open_home_at(shell, &at.display().to_string(), true) {
        Ok(mode) => Ok(mode),
        Err(f) => {
            if let Some(was) = was {
                if let Err(back) = follow(&id, seat, &at, &left, &was) {
                    shell.faults.push(back);
                }
            }
            Err(f)
        }
    }
}

fn seat_home_left(at: &std::path::Path) -> Result<Option<(String, crate::roles::Role, std::path::PathBuf)>, crate::fault::Fault> {
    let Some((row, seat)) = crate::register::now_row_listed()? else { return Ok(None) };
    Ok(row.home(seat).filter(|h| !crate::home::same_place(h, at)).map(|h| (row.id.clone(), seat, h)))
}

fn follow(id: &str, seat: crate::roles::Role, from: &std::path::Path, to: &std::path::Path, value: &str) -> Result<Option<String>, crate::fault::Fault> {
    let was = crate::register::change_listed(|reg| Ok(crate::identity::seat_home_set(reg, id, seat, value)))?.flatten();
    crate::own::home_follows(from, to)?;
    Ok(was)
}

pub(super) fn open_home(shell: &mut Shell, root: &str) -> Result<crate::lock::Mode, crate::fault::Fault> {
    let root = crate::home::landing(root)?;
    let root = root.display().to_string();
    let root = root.as_str();
    let identity_home = crate::register::now_row_listed()
        .ok()
        .flatten()
        .and_then(|(row, seat)| row.home(seat))
        .map(|h| h == std::path::Path::new(root.trim()))
        .unwrap_or(false);
    open_home_at(shell, root, !identity_home)
}

pub(super) fn open_home_at(shell: &mut Shell, root: &str, pointer: bool) -> Result<crate::lock::Mode, crate::fault::Fault> {
    let o = opening(root, pointer, &OpeningAsk::of(shell))?;
    Ok(opened(shell, o))
}

#[derive(Clone, Debug)]
pub(super) struct OpeningAsk {
    held: Option<std::path::PathBuf>,
}

impl OpeningAsk {
    pub(super) fn of(shell: &Shell) -> OpeningAsk {
        OpeningAsk { held: crate::lock::writer_path(shell.lock.as_ref()).map(|p| p.to_path_buf()) }
    }
}

pub struct Opening {
    home: crate::home::Home,
    lock: Option<crate::lock::Lock>,
    mode: crate::lock::Mode,
    read: crate::shell::Hydrated,
}

impl Opening {
    pub(super) fn seat_written(&mut self, seat: crate::roles::Role) -> Result<(), crate::fault::Fault> {
        if self.read.settings.role == seat {
            return Ok(());
        }
        if !self.mode.writable() {
            let holder = self.lock.as_ref().map(|l| l.holder().to_string()).unwrap_or_default();
            return Err(crate::fault::Fault::known(crate::fault::Known::ReadOnly, crate::lang::both_of(|| crate::lang::filln(crate::lang::Key::Tail006, &[&holder]))));
        }
        let mut next = self.read.settings.clone();
        next.role = seat;
        next.write(&self.home)?;
        self.read.settings = next;
        Ok(())
    }
}

impl std::fmt::Debug for Opening {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opening").field("home", &self.home.root()).field("mode", &self.mode).finish()
    }
}

pub(super) fn opening(root: &str, pointer: bool, ask: &OpeningAsk) -> Result<Opening, crate::fault::Fault> {
    let as_seat = opened_as(std::path::Path::new(root));
    crate::local::check_label(std::path::Path::new(root), as_seat.as_ref().map(|(i, s)| (i.as_str(), *s)), false)?;
    let home = crate::home::Home::open_or_create(root)?;
    let lock = if crate::lock::holds_writer_at(ask.held.as_deref(), &home) { None } else { Some(crate::lock::take(&home)?) };
    let mode = lock.as_ref().map(|l| l.mode()).unwrap_or(crate::lock::Mode::Writer);
    crate::local::check_label(home.root(), opened_as(home.root()).as_ref().map(|(i, s)| (i.as_str(), *s)), mode.writable())?;
    let read = crate::shell::Hydrated::read(&home)?;
    if pointer && !crate::home::named_by_env(home.root()) {
        crate::home::write_pointer(home.root())?;
    }
    let writes = mode.writable();
    if writes {
        crate::home::sweep_temps(home.root());
    }
    Ok(Opening { home, lock, mode, read })
}

pub(super) fn opened(shell: &mut Shell, o: Opening) -> crate::lock::Mode {
    let Opening { home, lock, mode, read } = o;
    shell.source_changed(crate::shell::Source::Home);
    shell.install(read);
    shell.home = Some(home);
    if let Some(l) = lock {
        shell.lock = Some(l);
    }
    if let Some(f) = shell.lock.as_ref().and_then(crate::lock::mark_trouble) {
        shell.trouble(f);
    }
    if let Some(root) = shell.home.as_ref().map(|h| h.root().to_path_buf()) {
        shell.tasks.spawn(Kind::Archive, move || crate::house::measure(&root));
    }
    mode
}

pub(super) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
