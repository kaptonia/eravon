use eravon_chain::endpoints::Table;
use eravon_chain::transport::Auto;
use eravon_store::settings;
use eravon_vault::store::{self, Store};
use eravon_store::vault::Seat;
use eravon_vault::vault::Vault;

use crate::fault::{Fault, R};
use crate::seam;
use eravon_vault as h2;

pub struct Ctx {
    _reads: Option<eravon_chain::action::Opened>,
    pub data: String,
    pub now: u64,
    pub writer: String,
    pub seat: Seat,
    pub sess: std::cell::RefCell<h2::session::Session>,
    pub unlock_err: std::cell::RefCell<Option<Fault>>,
}

std::thread_local! {
    static SIGNS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn opened_alone() -> bool {
    ALONE.with(std::cell::Cell::get)
}

pub const E_CREDENTIAL_DESKTOP_HOLDS: &str = "E_CREDENTIAL_DESKTOP_HOLDS";

pub fn desktop_holds(v: &eravon_literals::json::Out) -> Option<Fault> {
    use eravon_literals::json::Out;
    let Out::O(m) = v else { return None };
    let said = |k: &str| m.iter().find(|(n, _)| *n == k).and_then(|(_, x)| match x {
        Out::S(s) => Some(s.as_str()),
        _ => None,
    });
    let locked = said("downToken").or(said("token")) == Some("E_STORE_LOCKED");
    (opened_alone() && locked).then(|| Fault::Down {
        layer: "desk",
        token: E_CREDENTIAL_DESKTOP_HOLDS.to_string(),
        why: crate::out::Said::Words(
            "桌面开着并握着这一家的笔,经程序钥的命令行写不成:先退出桌面再写,或在桌面上关掉「经凭据操作」后加 --desktop 经桌面写",
            "The desktop is open and holds this data root, so a write through a program key cannot be made: quit the desktop first, or turn \"With a credential\" off on the desktop and write through it with --desktop",
        )
        .text()
        .to_string(),
    })
}

pub fn run_signs(signs: bool) {
    SIGNS.with(|s| s.set(signs));
}

pub const E_QUEUE_ONLY: &str = "E_CLI_QUEUE_ONLY";

pub fn anchor_live(m: &eravon_vault::machine::Machine) -> bool {
    m.cli_path() == eravon_vault::machine::CliPath::Desktop
}

pub fn anchor_now(m: &eravon_vault::machine::Machine) -> eravon_seal::machine::CliAnchor {
    if anchor_live(m) { m.cli_anchor } else { eravon_seal::machine::CliAnchor::Send }
}

fn held_for_desktop() -> Option<Fault> {
    if !SIGNS.with(std::cell::Cell::get) || crate::deskwire::lent() {
        return None;
    }
    let cell = anchor_now(&eravon_vault::machine::read().ok()?);
    match eravon_seal::machine::turn(cell, 1) {
        eravon_seal::machine::Turn::Leave => Some(Fault::Down {
            layer: "desk",
            token: E_QUEUE_ONLY.to_string(),
            why: crate::out::Said::Words(
                "桌面的「命令行上链」是仅加入队列:不经桌面一笔也不签不发;请加 --desktop <数据夹> 交给桌面,在卡上同意后签发",
                "The desktop's command-line send is queue only: nothing is signed or sent without the desktop; add --desktop <folder> to hand it there and approve it on the card",
            )
            .text()
            .to_string(),
        }),
        _ => None,
    }
}

pub fn open_ctx(data: String, now: u64, writer: String, seat: Seat, passcode: Option<&str>) -> Ctx {
    if let Some(s) = crate::deskwire::lent_session(&data) {
        return Ctx { _reads: None, data, now, writer, seat, sess: std::cell::RefCell::new(s), unlock_err: std::cell::RefCell::new(None) };
    }
    let reads = eravon_chain::action::open(eravon_chain::action::keeper());
    let mut s = h2::session::Session::locked(&data);
    let unlock_err = match passcode {
        Some(p) => {
            let e = s.unlock(p).err().map(down);
            if e.is_none() {
                keep_reads_sealed();
            }
            e
        }
        None if eravon_os::machine::here().is_ok_and(|(m, _)| crate::desk::writes_alone(&m)) => {
            let e = crate::sealed::open(&mut s, now).err();
            if e.is_none() {
                keep_reads_sealed();
                ALONE.with(|a| a.set(true));
            }
            e
        }
        None => None,
    };
    if s.is_open()
        && let Some(f) = held_for_desktop()
    {
        crate::out::emit(crate::verbwire::answer_of(&crate::answer::refuse(&f)));
    }
    Ctx { _reads: Some(reads), data, now, writer, seat, sess: std::cell::RefCell::new(s), unlock_err: std::cell::RefCell::new(unlock_err) }
}

impl Ctx {
    pub fn read(&self) -> R<Store> {
        Ok(store::open_read(&self.data)?)
    }

    pub fn write(&self) -> R<Store> {
        Ok(store::open_write(&self.data, &self.writer, self.now)?)
    }

    pub fn vault(&self) -> R<Vault> {
        h2::vault::read(&self.data).map_err(down)
    }

    pub fn active(&self) -> R<String> {
        let a = h2::accounts::active(&self.data, self.seat).map_err(down)?;
        a.ok_or_else(|| {
            Fault::State(format!("{} 这一席还没有当前钥:先跑 vault-init", self.seat.word()))
        })
    }

    pub fn who(&self, given: Option<&str>) -> R<String> {
        match given {
            Some(a) => Ok(a.to_string()),
            None => self.active(),
        }
    }

    pub fn identity(&self, addr: &str) -> R<settings::Identity> {
        Ok(eravon_vault::store::read_identity(&self.data, addr)?)
    }

    pub fn endpoints(&self, addr: &str) -> R<Vec<String>> {
        Ok(eravon_vault::store::endpoints(&self.data, addr)?)
    }

    pub fn session(&self, _passcode: Option<&str>) -> R<std::cell::Ref<'_, h2::session::Session>> {
        if let Some(e) = self.unlock_err.borrow().as_ref() {
            return Err(e.clone());
        }
        Ok(self.sess.borrow())
    }

    pub fn open_now(&self, passcode: &str) -> R<()> {
        self.sess.borrow_mut().unlock(passcode).map_err(down)?;
        *self.unlock_err.borrow_mut() = None;
        keep_reads_sealed();
        Ok(())
    }

    pub fn session_mut(&self) -> R<std::cell::RefMut<'_, h2::session::Session>> {
        if let Some(e) = self.unlock_err.borrow().as_ref() {
            return Err(e.clone());
        }
        Ok(self.sess.borrow_mut())
    }

    pub fn why_locked(&self, e: Fault) -> Fault {
        match (&e, self.unlock_err.borrow().as_ref()) {
            (Fault::Down { token, .. }, Some(u)) if token == "E_VAULT_LOCKED" => u.clone(),
            _ => e,
        }
    }

    pub fn key(&self, addr: &str, passcode: Option<&str>) -> R<[u8; 32]> {
        let s = self.session(passcode)?;
        let k = s.key_for(addr).map_err(down);
        k
    }
}

fn keep_reads_sealed() {
    if crate::deskwire::lent() {
        return;
    }
    if let Ok((m, _)) = eravon_os::machine::here() {
        eravon_chain::action::keep_in(std::sync::Arc::new(eravon_send::checked::Logs(eravon_send::checked::Sealed(m.clone()))));
        eravon_scan::judged::keep_in(Some(std::sync::Arc::new(eravon_send::checked::Sealed(m))));
    }
}

pub fn keep_bare_reads() {
    if crate::deskwire::lent() {
        return;
    }
    eravon_scan::bare::keep_reads_in(eravon_os::machine::here().ok().map(|(m, _)| m));
}

pub fn down(e: h2::vfault::Fault) -> Fault {
    match e {
        h2::vfault::Fault::Rule(rule) => Fault::Rule { rule, why: e.detail() },
        _ => Fault::Down { layer: "H2", token: e.name().to_string(), why: e.detail() },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Inline,
    File,
    Identity,
}

impl Source {
    pub const fn word(self) -> &'static str {
        match self {
            Source::Inline => "inline",
            Source::File => "file",
            Source::Identity => "identity",
        }
    }
}

pub const FILE_BOUND: u64 = 16 * 1024 * 1024;

pub fn read_bounded(path: &str) -> R<Vec<u8>> {
    let m = std::fs::metadata(path).map_err(|e| Fault::Io(format!("看不到 {path}:{e}")))?;
    if m.len() > FILE_BOUND {
        return Err(Fault::Io(format!("{path} 有 {} 字节,过了一读的界 {FILE_BOUND}", m.len())));
    }
    std::fs::read(path).map_err(|e| Fault::Io(format!("读不动 {path}:{e}")))
}

pub fn table(
    inline: &[String],
    file: Option<&str>,
    nodes: Option<&[String]>,
) -> R<(Table, Source)> {
    if !inline.is_empty() {
        return Ok((Table::new(inline.to_vec())?, Source::Inline));
    }
    if let Some(p) = file {
        return Ok((Table::open(&read_bounded(p)?)?, Source::File));
    }
    match nodes {
        Some(n) if !n.is_empty() => {
            Ok((Table::new(n.to_vec())?, Source::Identity))
        }
        _ => Err(Fault::State(match eravon_literals::constants::run_chain() {
            Some(c) => format!("没有端点表:这一趟照 --chain {c} 跑,身份设置里没有这条网的节点;给这条网配节点(settings-endpoints --chain {c}),或用 --endpoint、--table 给"),
            None => "没有端点表:用 --endpoint 逐个给、--table 给一份档,或先把端点写进身份设置".into(),
        })),
    }
}

pub fn transport(pem: Option<&str>, proxy: Option<&str>, by_home: bool) -> R<Auto> {
    if by_home {
        eravon_chain::rest::hold_keys(machine_rest_keys());
    }
    match proxy {
        Some(p) => {
            eravon_chain::rpc::follow_the_terminal();
            eravon_chain::transport::set_way(eravon_chain::transport::choice_of(p)?);
        }
        None if by_home => machine_route(),
        None => eravon_chain::rpc::follow_the_terminal(),
    }
    match pem {
        Some(p) => Ok(Auto::with_pem_roots(&read_bounded(p)?)?),
        None => Ok(Auto::web_roots()?),
    }
}

fn machine_route() {
    eravon_send::node::take_machine_route(machine_proxy().as_deref())
}


pub fn machine_proxy() -> Option<String> {
    eravon_vault::machine::read_face().ok().filter(|m| m.held()).and_then(|m| m.proxy)
}

pub fn machine_rest_keys() -> Vec<(String, String)> {
    eravon_vault::machine::rest_keys_now()
}

static KEEP_HOME_NETWORK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn keep_to_home_network() {
    KEEP_HOME_NETWORK.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn settle_run_network(c: &Ctx, f: &crate::shared::Flags) {
    if !KEEP_HOME_NETWORK.swap(false, std::sync::atomic::Ordering::SeqCst) || eravon_literals::constants::run_chain().is_some() {
        return;
    }
    let Ok(who) = c.who(f.one("--address")) else { return };
    if let Ok(chain) = store::network(&c.data, &who) {
        if eravon_literals::constants::network_of(chain).is_some() {
            eravon_literals::constants::pin_run_chain(chain);
            eravon_literals::constants::select_chain(chain);
        }
    }
}

pub fn chain_id(given: Option<u64>) -> u64 {
    given.unwrap_or_else(|| eravon_standard::chain::standard_table().0)
}

pub fn logic(given: Option<&str>) -> String {
    given
        .map(str::to_string)
        .unwrap_or_else(|| eravon_standard::chain::standard_table().1.to_string())
}

pub fn seat_of(word: &str) -> R<Seat> {
    match word {
        seam::SEAT_SELLER => Ok(Seat::Seller),
        seam::SEAT_BUYER => Ok(Seat::Buyer),
        _ => Err(Fault::Arg(format!("席只有 seller 与 buyer 两种,给的是 {word:?}"))),
    }
}

pub fn or_missing<T: Default>(r: R<T>) -> R<T> {
    match r {
        Ok(v) => Ok(v),
        Err(Fault::Down { layer: "A5", token, .. }) if token == "E_STORE_MISSING" => {
            Ok(T::default())
        }
        Err(e) => Err(e),
    }
}

pub const SEND_STEPS: [&str; 5] =
    ["nonce", "gas", "assemble", "sign", "send"];

#[derive(Default)]
pub struct After {
    trouble: Vec<String>,
}

impl After {
    pub fn note<T>(&mut self, r: R<T>) -> Option<T> {
        r.map_err(|e| self.trouble.push(format!("{}:{}", e.name(), e.detail()))).ok()
    }

    pub fn said(&mut self, line: String) {
        self.trouble.push(line);
    }

    pub fn born(&mut self, s: &eravon_vault::store::Store, data: &str, addr: &str, now: u64) -> Vec<(&'static str, eravon_literals::json::Out)> {
        let b = h2::onboard::born_added(s, data, addr, eravon_literals::constants::current_network().standard.chain_id, now);
        for o in b.others {
            self.note(o.map_err(down));
        }
        self.born_of(b.own)
    }

    pub fn born_of(&mut self, got: Result<h2::onboard::Born, h2::vfault::Fault>) -> Vec<(&'static str, eravon_literals::json::Out)> {
        use eravon_literals::json::Out;
        match self.note(got.map_err(down)) {
            Some(b) => vec![("endpoints", Out::N(b.endpoints as u64)), ("endpointsFrom", Out::S(b.from.into()))],
            None => vec![("endpoints", Out::N(0)), ("endpointsFrom", Out::S(String::new()))],
        }
    }

    pub fn rows(self) -> Vec<(&'static str, eravon_literals::json::Out)> {
        use eravon_literals::json::Out;
        if self.trouble.is_empty() {
            Vec::new()
        } else {
            vec![("trouble", Out::A(self.trouble.into_iter().map(Out::S).collect()))]
        }
    }
}
