use eravon_literals::json::{read, In, Out};

use crate::action::{Action as Fam, Applied};
use crate::house::act::Act;
use crate::house::action::Action;
use crate::house::i18n::Lang;
use crate::house::seam;
use crate::shell::Shell;

mod cli;
pub(crate) mod fam;
mod gate;
mod menu;
mod wizard;

pub struct Run<'a> {
    pub data: &'a str,
    pub lang: Lang,
    pub budget_ms: u64,
    pub now: u64,
    pub door_tallied: std::cell::Cell<usize>,
    pub registries: Vec<String>,
    pub home: String,
    pub passcode: Option<String>,
}

pub struct Driven {
    pub shell: Shell,
    pub path: String,
    pub first: Out,
    pub write: Out,
    pub gate: Out,
    pub menu: Out,
    pub shown: Vec<crate::secret::Secret>,
}

struct Drive {
    gate: gate::Gate,
    menu: Option<menu::Menu>,
    wiz: wizard::Wiz,
    win: Option<crate::window::house::windrive::WinDrive>,
    shown: Vec<crate::secret::Secret>,
}

static CLOCK: std::sync::Mutex<Option<(usize, String, std::time::Instant)>> = std::sync::Mutex::new(None);

pub const STEP_SPANS: u64 = 3;

fn clock_on(i: usize, what: &str) {
    *CLOCK.lock().unwrap_or_else(|e| e.into_inner()) = Some((i, what.to_string(), std::time::Instant::now()));
}

fn clock_off() {
    *CLOCK.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub fn watch(budget_ms: u64, then: impl Fn(String) + Send + 'static) -> Result<(), String> {
    crate::task::start_named("drive-clock", move || loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        overdue_then(budget_ms, &then);
    })
    .map(|_| ())
    .map_err(|e| format!("驱动门的钟起不来:{e}"))
}

pub fn overdue_then(budget_ms: u64, then: impl FnOnce(String)) {
    let g = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    let backstop = crate::window::house::windrive::SETTLE_SECS;
    let limit = std::time::Duration::from_millis(budget_ms.max(1).saturating_mul(STEP_SPANS)) + std::time::Duration::from_secs(backstop);
    if let Some((i, what, at)) = g.as_ref() {
        if at.elapsed() >= limit {
            let said = if *i == 0 { "开局解锁".to_string() } else { format!("第 {i} 步 {what}") };
            then(format!("{said} 过了步单预算({budget_ms} 毫秒 × {STEP_SPANS},加窗步落定上限 {backstop} 秒),没有回来;驱动门按自己的钟收场"));
        }
    }
}

pub fn run(script: &[u8], r: &Run<'_>) -> Result<Driven, String> {
    let out = run_steps(script, r);
    clock_off();
    out
}

fn run_steps(script: &[u8], r: &Run<'_>) -> Result<Driven, String> {
    crate::notifier::stand_in_notifier();
    let doc = read(script).map_err(|e| format!("脚本不是一份读得动的 JSON:{e}"))?;
    let steps = match doc.get("steps") {
        Some(In::Arr(a)) => a.clone(),
        _ => return Err("脚本上要有一张 steps 表".to_string()),
    };
    crate::house::i18n::set(r.lang);
    eravon_vault::records::bind(r.data).map_err(|f| format!("{} · {}", f.said(), f.tail()))?;
    clock_on(0, "unlock");
    let booted = boot(r);
    clock_off();
    let mut sh = booted?;
    let mut d = Drive { gate: gate::Gate::default(), menu: None, wiz: wizard::Wiz::default(), win: None, shown: Vec::new() };
    for (i, st) in steps.iter().enumerate() {
        let what = st.get("do").and_then(|x| x.as_str()).ok_or_else(|| format!("第 {} 步上没有 do", i + 1))?;
        let act = Action::of(what).ok_or_else(|| if d.win.is_some() { format!("窗步认不出:{what}") } else { format!("脚本上认不出这一步:{what}") })?;
        clock_on(i + 1, what);
        if let Some(w) = d.win.as_mut() {
            let say = w.step(act, st)?;
            w.shell_mut().desk.tr.push(act, eravon_literals::trace::Feature::H1, say);
            continue;
        }
        if act == Action::Win {
            let pass = st.get("passcode").and_then(|x| x.as_str()).map(str::to_string).or_else(|| r.passcode.clone());
            let shell = std::mem::replace(&mut sh, boot_bare());
            let mut w = crate::window::house::windrive::WinDrive::start(shell, pass.as_deref())?;
            w.shell_mut().desk.tr.push(act, eravon_literals::trace::Feature::H1, "起窗".into());
            d.win = Some(w);
            continue;
        }
        let before = sh.desk.tr.taken;
        let say = d.step(&mut sh, r, act, st)?;
        if st.get("hold").and_then(|x| x.as_str()) != Some("true") {
            settle(&mut sh, r.budget_ms);
            if act.name().starts_with("set-") {
                let (data, env) = (sh.desk.data.clone(), crate::house::settings::Env { home: sh.desk.sh.seller.field.home.clone() });
                if let Some(st) = sh.desk.sh.seller.settings.as_mut() {
                    st.reload(&data, &env);
                }
            }
        }
        let fresh = (sh.desk.tr.taken - before) as usize;
        let own = sh.desk.tr.steps.iter().rev().take(fresh).any(|x| x.what == act.name());
        if !own {
            sh.desk.tr.push(act, feature_of(&sh, act), say);
        }
    }
    clock_on(steps.len() + 1, "finish");
    if let Some(w) = d.win.take() {
        sh = w.finish();
    }
    clock_off();
    let path = sh.desk.tr.save(r.data)?;
    let first = d.wiz.rows(&sh);
    let write = write_rows(&sh);
    let gate = d.gate.rows(&sh);
    let menu = d.menu.as_ref().map_or(Out::O(Vec::new()), menu::Menu::rows);
    Ok(Driven { shell: sh, path, first, write, gate, menu, shown: d.shown })
}

fn boot(r: &Run<'_>) -> Result<Shell, String> {
    let mut sh = boot_bare();
    sh.desk.data = r.data.to_string();
    sh.desk.session = eravon_vault::session::Session::locked(r.data);
    sh.desk.clock = Some(r.now);
    sh.desk.sh.at(std::time::Duration::from_secs(r.now).as_millis() as u64);
    sh.desk.sh.seller.field.registries = r.registries.clone();
    crate::house::orders::inject(&r.registries);
    if !r.home.is_empty() {
        sh.desk.sh.seller.field.home = seam::user_home(&r.home);
    }
    let _ = crate::action::start(&mut sh);
    if let Some(l) = sh.speaks() {
        crate::lang::set(l);
    }
    if let Some(p) = &r.passcode {
        match crate::action::apply_settled(&mut sh, Fam::Unlock { pin: crate::secret::Secret::of(p) }) {
            Applied::Unlocked => {}
            Applied::Trouble(f) => return Err(format!("开局口令没过:{} · {}", f.said(), f.tail())),
            other => return Err(format!("开局口令没过:{other:?}")),
        }
        settle(&mut sh, r.budget_ms.max(1));
    }
    Ok(sh)
}

fn boot_bare() -> Shell {
    crate::window::house::shell_headless()
}

pub(crate) fn settle(sh: &mut Shell, budget_ms: u64) {
    let start = std::time::Instant::now();
    let until = start + std::time::Duration::from_millis(budget_ms.max(1));
    loop {
        let _ = sh.drain_at(start.elapsed().as_secs_f64());
        sh.house_tick();
        sh.goods_taken();
        let busy = sh.house_busy() || crate::task::Kind::ALL.iter().any(|k| sh.tasks.in_flight(*k));
        if !busy || std::time::Instant::now() >= until {
            return;
        }
        if sh.house_in_flight(crate::task::Kind::Sync) {
            sh.desk.sh.idle_ticks = sh.desk.sh.idle_ticks.saturating_add(1);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

impl Drive {
    fn step(&mut self, sh: &mut Shell, r: &Run<'_>, act: Action, st: &In) -> Result<String, String> {
        if let Some(said) = wizard::step(&mut self.wiz, sh, r, act, st) {
            return said;
        }
        if let Some(said) = gate::step(&mut self.gate, sh, r, act, st) {
            return said;
        }
        if let Some(said) = menu::step(&mut self.menu, sh, act, st) {
            return said;
        }
        if let Some(said) = cli::step(&mut self.shown, sh, act, st) {
            return said;
        }
        if let Some(said) = fam::step(sh, r, act, st) {
            return said;
        }
        if act == Action::DoorWait {
            return door_wait(sh, r, st);
        }
        if matches!(act, Action::Page | Action::PageMarked) {
            return page(sh, r, act, st);
        }
        let a = act_of(act, st)?;
        match crate::action::apply_settled(sh, Fam::House(a)) {
            Applied::House(said) => said,
            Applied::Trouble(f) => Err(format!("{} · {}", f.said(), f.tail())),
            other => Ok(format!("{other:?}")),
        }
    }
}

fn page(sh: &mut Shell, r: &Run<'_>, act: Action, st: &In) -> Result<String, String> {
    use crate::house::seller::Page;
    let to = arg(st, "to")?;
    let p = Page::of(&to).ok_or_else(|| format!("认不出这一页:{to}"))?;
    let mark = if act == Action::PageMarked { Some(arg(st, "mark")?) } else { None };
    let reading = matches!(p, None | Some(Page::Alerts | Page::Orders | Page::Wallet | Page::Export));
    settle(sh, r.budget_ms);
    if let Some(m) = &mark {
        stand_mark(m, &format!("before:{to}"))?;
    }
    let before = eravon_chain::ask::asked();
    let said = match crate::action::apply_settled(sh, Fam::House(act_of(act, st)?)) {
        Applied::House(said) => said,
        Applied::Trouble(f) => Err(format!("{} · {}", f.said(), f.tail())),
        other => Ok(format!("{other:?}")),
    };
    settle(sh, r.budget_ms);
    sh.desk.sh.draw_asked = reading.then(|| eravon_chain::ask::asked().saturating_sub(before));
    if let Some(m) = &mark {
        stand_mark(m, &format!("after:{to}"))?;
    }
    said
}

fn stand_mark(control: &str, name: &str) -> Result<(), String> {
    use eravon_chain::transport::Transport as _;
    let url = eravon_chain::seam::parse_url(&format!("{}/mark", control.trim_end_matches('/'))).map_err(|e| e.detail())?;
    eravon_chain::transport::Http::with_timeout(std::time::Duration::from_secs(5)).post(&url, name.as_bytes()).map(|_| ()).map_err(|e| format!("记号 {name} 打不到 {control}:{}", e.detail()))
}

fn act_of(act: Action, st: &In) -> Result<Act, String> {
    Ok(Act { step: act, args: args_of(st)? })
}

pub(crate) fn args_of(st: &In) -> Result<Vec<(String, String)>, String> {
    let mut args: Vec<(String, String)> = Vec::new();
    if let In::Obj(m) = st {
        for (k, v) in m {
            if k == "do" {
                continue;
            }
            match v {
                In::Str(s) => args.push((k.clone(), s.clone())),
                In::Int(n) => args.push((k.clone(), n.to_string())),
                In::Bool(b) => args.push((k.clone(), b.to_string())),
                In::Arr(items) => {
                    for (i, it) in items.iter().enumerate() {
                        if let Some(s) = it.as_str() {
                            args.push((format!("{k}{i}"), s.to_string()));
                        }
                    }
                }
                _ => return Err(format!("{k} 这一栏的形认不出")),
            }
        }
    }
    Ok(args)
}

fn door_wait(sh: &mut Shell, r: &Run<'_>, st: &In) -> Result<String, String> {
    let n: usize = arg(st, "n")?.parse().map_err(|_| "n 要是要接的请求数".to_string())?;
    sh.door_sync();
    if sh.door.is_none() {
        let why = match &sh.door_shut {
            Some((_, f)) => f.said().to_string(),
            None if !sh.unlocked() => "锁着".to_string(),
            None => "本实例不是这一数据根的写者".to_string(),
        };
        return Ok(format!("door-wait · 门没开:{why}"));
    }
    let start = r.door_tallied.get().min(sh.door_answered);
    let until = std::time::Instant::now() + std::time::Duration::from_millis(r.budget_ms.max(1));
    while (sh.door_answered - start < n || sh.door_doing.is_some()) && !crate::house::approve::holding(sh) && std::time::Instant::now() < until {
        settle(sh, 20);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    r.door_tallied.set(sh.door_answered);
    match crate::house::approve::awaiting(sh) {
        Some(q) => Ok(format!("door-wait · 接了 {} 件 · 待批 {} · 旗 {}", sh.door_answered - start, q.verb, crate::house::approve::awaiting_flags(sh).join(","))),
        None => Ok(format!("door-wait · 接了 {} 件", sh.door_answered - start)),
    }
}

fn write_rows(sh: &Shell) -> Out {
    let writer = sh.lock.as_ref().map(|l| l.mode());
    let mark_unread = sh.lock.as_ref().and_then(|l| l.mark_trouble()).is_some();
    let unheld = sh.home.is_none() && sh.unlocked();
    let why = |w| Out::S(write_word(crate::house::shell::write_block(!sh.unlocked(), writer, mark_unread, unheld, !sh.desk.addr.is_empty(), sh.desk.sh.sync.table, w)).to_string());
    let standing = Out::S(
        match writer {
            Some(eravon_store::lock::Mode::OtherMachine) if mark_unread => MARK_UNREAD,
            w => w.map(|m| m.as_str()).unwrap_or(crate::house::vocab::BLANK),
        }
        .to_string(),
    );
    let unheld = Out::S(if unheld { "write.unheld" } else { crate::house::vocab::BLANK }.to_string());
    Out::O(vec![("chain", why(crate::house::shell::Write::Chain)), ("local", why(crate::house::shell::Write::Local)), ("vault", why(crate::house::shell::Write::Vault)), ("unheld", unheld), ("writer", standing)])
}

pub const MARK_UNREAD: &str = "mark-unread";

pub fn write_word(why: Option<&'static str>) -> &'static str {
    why.unwrap_or(crate::house::vocab::BLANK)
}

pub fn rows(d: &Driven) -> Vec<(&'static str, Out)> {
    let sh = &d.shell;
    let mut m = sh.desk.sh.rows();
    m.push(("queue", sh.desk.sh.queue.as_ref().map_or(Out::O(Vec::new()), |p| p.rows())));
    m.push(("first", d.first.clone()));
    m.push(("wizard", Out::S(sh.desk.wizard_up.unwrap_or_default().to_string())));
    m.push(("write", d.write.clone()));
    m.push(("gate", d.gate.clone()));
    m.push(("idMenu", d.menu.clone()));
    m.push(("setCard", sh.desk.set_card.as_ref().map_or(Out::Null, crate::house::cards::SetCard::out)));
    m.push(("cliPasses", cli::rows(sh, &d.shown)));
    m.push((
        "door",
        Out::O(match crate::house::approve::awaiting(sh) {
            Some(q) => vec![("flags", Out::A(crate::house::approve::awaiting_flags(sh).into_iter().map(Out::S).collect())), ("held", Out::S(q.verb.clone()))],
            None => Vec::new(),
        }),
    ));
    m.push((
        "faults",
        Out::A(sh.faults.iter().map(|f| Out::O(vec![("code", Out::S(f.which().map(|k| k.as_str().to_string()).unwrap_or_default())), ("human", Out::S(f.human().to_string())), ("raw", Out::S(f.raw()))])).collect()),
    ));
    m.push(("flying", Out::A(crate::task::Kind::ALL.iter().filter(|k| sh.tasks.in_flight(**k)).map(|k| Out::S(k.as_str().to_string())).collect())));
    m.push(("steps", Out::N(sh.desk.tr.taken)));
    m.push(("trace", sh.desk.tr.rows()));
    m.push(("tracePath", Out::S(d.path.clone())));
    m.sort_by(|a, b| a.0.cmp(b.0));
    m
}

fn feature_of(sh: &Shell, act: Action) -> eravon_literals::trace::Feature {
    let what = act.name();
    if gate::is_gate(act) {
        eravon_literals::trace::Feature::H2
    } else if what.starts_with("set-") {
        eravon_literals::trace::Feature::S9
    } else if what.starts_with("pe-") || what.starts_with("pd-") || what == seam::STEP_GOODS_SEARCH {
        eravon_goods::mark::FEATURE
    } else if what.starts_with(seam::WL_STEP_PREFIX) {
        eravon_wallet::mark::FEATURE
    } else if what == seam::STEP_EX_RUN && sh.desk.sh.export.as_ref().is_some_and(|e| e.tab == crate::house::export::Tab::Statement) {
        eravon_statement::mark::FEATURE
    } else if what.starts_with(seam::EX_STEP_PREFIX) {
        eravon_export::mark::FEATURE
    } else if what.starts_with(seam::PL_STEP_PREFIX) {
        crate::house::mark::FEATURE_P1
    } else if what.starts_with(seam::BO_STEP_PREFIX) {
        crate::house::mark::FEATURE_P5
    } else {
        eravon_literals::trace::Feature::H1
    }
}

pub(super) fn arg(st: &In, key: &str) -> Result<String, String> {
    st.get(key).and_then(|x| x.as_str()).map(str::to_string).ok_or_else(|| format!("这一步上要有 {key}"))
}

pub(super) fn said_of(a: &Applied) -> String {
    match a {
        Applied::Trouble(f) => format!("拒 · {} · {}", f.which().map_or(f.class().as_str(), |k| k.as_str()), f.said()),
        other => format!("{other:?}"),
    }
}
