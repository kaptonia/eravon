use crate::house::seam;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Seat {
    Seller,
    Buyer,
}

impl Seat {
    pub const fn name(self) -> &'static str {
        match self {
            Seat::Seller => seam::SEAT_SELLER,
            Seat::Buyer => seam::SEAT_BUYER,
        }
    }

    pub fn of(s: &str) -> Option<Seat> {
        match s {
            seam::SEAT_SELLER => Some(Seat::Seller),
            seam::SEAT_BUYER => Some(Seat::Buyer),
            _ => None,
        }
    }

    pub const fn store(self) -> eravon_store::vault::Seat {
        match self {
            Seat::Seller => eravon_store::vault::Seat::Seller,
            Seat::Buyer => eravon_store::vault::Seat::Buyer,
        }
    }

    pub const fn of_store(s: eravon_store::vault::Seat) -> Seat {
        match s {
            eravon_store::vault::Seat::Seller => Seat::Seller,
            eravon_store::vault::Seat::Buyer => Seat::Buyer,
        }
    }


    pub const ALL: [Seat; 2] = [Seat::Seller, Seat::Buyer];
}

pub fn current_key(data: &str, seat: Seat) -> Result<String, String> {
    eravon_vault::accounts::who(data, seat.store()).map_err(|e| format!("{}:{}", e.name(), e.detail()))
}

pub fn not_read_yet(name: &str) -> bool {
    name.starts_with(eravon_chain::fault::Fault::Unread(String::new()).name())
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    List,
    Sub,
    Sheet,
}

impl Mode {
    pub const fn name(self) -> &'static str {
        match self {
            Mode::List => seam::MODE_LIST,
            Mode::Sub => seam::MODE_SUB,
            Mode::Sheet => seam::MODE_SHEET,
        }
    }

    pub fn of(s: &str) -> Option<Mode> {
        match s {
            seam::MODE_LIST => Some(Mode::List),
            seam::MODE_SUB => Some(Mode::Sub),
            seam::MODE_SHEET => Some(Mode::Sheet),
            _ => None,
        }
    }

    pub const ALL: [Mode; 3] = [Mode::List, Mode::Sub, Mode::Sheet];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub tone: crate::house::seller::Tone,
    pub head: String,
    pub next: String,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Table {
    #[default]
    Unread,
    Unset,
    Unreadable,
    Bad,
    Ready,
    WrongChain,
}

impl Table {
    pub const fn name(self) -> &'static str {
        match self {
            Table::Unread => "unread",
            Table::Unset => "unset",
            Table::Unreadable => "unreadable",
            Table::Bad => "bad",
            Table::Ready => "ready",
            Table::WrongChain => "wrong-chain",
        }
    }

    pub const ALL: [Table; 6] = [Table::Unread, Table::Unset, Table::Unreadable, Table::Bad, Table::Ready, Table::WrongChain];
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sync {
    pub table: Table,
    pub configured: bool,
    pub endpoints: usize,
    pub block: Option<u64>,
    pub chain_time: Option<u64>,
    pub agreement: Option<&'static str>,
    pub read_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Shell {
    seat: Seat,
    pub mode: Mode,
    pub sheet_open: bool,
    pub url_asked: Option<(String, String)>,
    pub toasts: Vec<Toast>,
    pub win: Option<eravon_literals::json::Out>,
    pub toast_bumps: u64,
    pub quitting: bool,
    pub clock: u64,
    pub toast_at: u64,
    pub read_said: Vec<(&'static str, String)>,
    pub last_input: u64,
    pub home_asked: bool,
    pub sync: Sync,
    pub links: Vec<(&'static str, usize)>,
    pub words: Vec<(String, bool, String)>,
    pub idle_ticks: u64,
    pub seller: crate::house::seller::Seller,
    pub prev_seat: Option<Seat>,
    pub paylink: Option<crate::house::paylink::PayLink>,
    pub buyer_orders: Option<crate::house::buyer_orders::BOrders>,
    pub wallet: Option<crate::house::wallet::Wallet>,
    pub buyer_home: Option<crate::house::buyer_home::BHome>,
    pub export: Option<crate::house::export::Export>,
    pub queue: Option<crate::house::queue::View>,
    pub book: Option<crate::house::book::Book>,
    pub refreshes: Vec<crate::house::refresh::Seen>,
    pub refresh_at: Option<u64>,
    pub refresh_every: u64,
    pub read_failed: Option<eravon_chain::fault::Fault>,
    pub book_gen: u64,
    pub reshow_due: bool,
    pub draw_asked: Option<u64>,
    pub stash: Stash,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stash {
    pub home: bool,
    pub orders: Option<Option<String>>,
    pub cands: Option<eravon_cands::table::Side>,
    pub bhome: bool,
    pub bo: Option<Option<(String, crate::house::buyer_orders::Tab)>>,
    pub wallet: bool,
    pub export: Option<(bool, Option<String>)>,
}

impl Shell {
    pub fn adopt(&mut self, data: &str) {
        let (zone, every) = crate::house::settings::reading_now(data);
        self.refresh_every = every;
        eravon_orders::when::set(zone);
    }


    pub fn drop_reads(&mut self) {
        let mut s = std::mem::take(&mut self.stash);
        if self.seller.home.take().is_some() {
            s.home = true;
        }
        if let Some(o) = self.seller.orders.take() {
            s.orders = Some(o.order.map(|x| x.id));
        }
        if let Some(c) = self.seller.cands.as_ref() {
            s.cands = Some(c.tab);
        }
        if self.buyer_home.take().is_some() {
            s.bhome = true;
        }
        if let Some(b) = self.buyer_orders.take() {
            s.bo = Some(b.open.map(|o| (o.instance, o.tab)));
        }
        if let Some(w) = self.wallet.as_mut() {
            w.balances = None;
            w.txs = None;
            s.wallet = true;
        }
        if let Some(e) = self.export.take() {
            s.export = Some((e.buyer, e.sel));
        }
        if let Some(st) = self.seller.settings.as_mut() {
            st.drop_reads();
        }
        self.stash = s;
        self.book = None;
        self.read_failed = None;
    }

    pub fn lock_reads(&mut self) {
        self.drop_reads();
        self.sync = Sync::default();
    }

}

impl Default for Shell {
    fn default() -> Self {
        Shell {
            seat: Seat::Seller,
            mode: Mode::List,
            sheet_open: false,
            url_asked: None,
            toasts: Vec::new(),
            win: None,
            toast_bumps: 0,
            quitting: false,
            clock: 0,
            toast_at: 0,
            read_said: Vec::new(),
            last_input: 0,
            home_asked: false,
            sync: Sync::default(),
            links: Vec::new(),
            words: Vec::new(),
            idle_ticks: 0,
            seller: crate::house::seller::Seller::default(),
            prev_seat: None,
            paylink: None,
            buyer_orders: None,
            wallet: None,
            buyer_home: None,
            export: None,
            queue: None,
            book: None,
            refreshes: Vec::new(),
            refresh_at: None,
            refresh_every: eravon_literals::constants::READ_REFRESH_SECS,
            read_failed: None,
            book_gen: 0,
            reshow_due: false,
            draw_asked: None,
            stash: Stash::default(),
        }
    }
}

pub struct Door(());

impl Shell {
    pub fn seat(&self) -> Seat {
        self.seat
    }

    pub fn set_seat(&mut self, s: Seat) {
        self.seat = s;
    }

    pub fn go(&mut self, to: crate::house::seller::Leave) -> bool {
        use crate::house::seller::{Leave, Page};
        let leaving = match &to {
            Leave::Seat(s) => *s != self.seat,
            Leave::Page(_) => self.seller.page() == Some(Page::Goods),
            Leave::List | Leave::Close => true,
        };
        if leaving
            && let Some(gd) = self.seller.goods.as_mut()
            && !gd.request_leave(to.clone())
        {
            return false;
        }
        match to {
            Leave::Seat(s) => {
                if s != self.seat {
                    self.prev_seat = Some(self.seat);
                }
                self.seat = s;
                let page = self.seller.page();
                let keep = match s {
                    Seat::Buyer => matches!(page, None | Some(Page::Settings) | Some(Page::PayLink) | Some(Page::Orders) | Some(Page::Alerts) | Some(Page::Export) | Some(Page::Queue)),
                    Seat::Seller => !matches!(page, Some(Page::PayLink) | Some(Page::Wallet)),
                };
                if !keep {
                    self.seller.set_page(&Door(()), None);
                }
            }
            Leave::Page(p) => self.seller.set_page(&Door(()), p),
            Leave::List | Leave::Close => {}
        }
        true
    }

    pub fn mode_to(&mut self, m: Mode) {
        self.mode = m;
        self.sheet_open = m == Mode::Sheet;
    }


    pub fn close_sheet(&mut self) {
        self.sheet_open = false;
        if self.mode == Mode::Sheet {
            self.mode = Mode::List;
        }
    }

    pub fn quit(&mut self, lang: crate::house::i18n::Lang, writing: &[crate::task::Kind]) -> bool {
        if writing.is_empty() {
            self.quitting = false;
            return true;
        }
        if !self.quitting {
            self.quitting = true;
            let what: Vec<&str> = writing.iter().map(|k| k.as_str()).collect();
            self.toast(Toast { tone: crate::house::seller::Tone::Note, head: crate::house::i18n::get(lang, crate::house::i18n::Key::QuitWriting).0, next: crate::house::i18n::get(lang, crate::house::i18n::Key::QuitWritingNext).0, detail: Some(what.join(" · ")) });
        }
        false
    }

    pub fn toast(&mut self, t: Toast) {
        if self.toasts.first().is_some_and(|x| x.head == t.head && x.next == t.next) {
            if t.tone != crate::house::seller::Tone::Bad {
                self.toast_bumps = self.toast_bumps.saturating_add(1);
                self.toast_at = self.clock;
            }
            return;
        }
        self.toasts = vec![t];
        self.toast_bumps = 0;
        self.toast_at = self.clock;
    }

    pub fn at(&mut self, now: u64) {
        self.clock = now;
        let secs = |t: &Toast| if t.tone == crate::house::seller::Tone::Bad { eravon_ui::metrics::TOAST_BAD_LIFE_SECONDS } else { eravon_ui::metrics::TOAST_LIFE_SECONDS };
        let life = |t: &Toast| std::time::Duration::from_secs_f64(secs(t)).as_millis() as u64;
        if self.toasts.first().is_some_and(|t| now.saturating_sub(self.toast_at) >= life(t)) {
            self.toasts.clear();
            self.toast_bumps = 0;
        }
    }

    pub fn toast_read(&mut self, from: &'static str, t: Toast) {
        if t.detail.as_deref().is_some_and(not_read_yet) {
            return;
        }
        let key = format!("{}\u{1f}{}", t.head, t.next);
        if self.read_said.iter().any(|(f, k)| *f == from && *k == key) {
            return;
        }
        self.read_said.retain(|(f, _)| *f != from);
        self.read_said.push((from, key));
        self.toast(t);
    }

    pub fn read_ok(&mut self, from: &'static str) {
        self.read_said.retain(|(f, _)| *f != from);
    }

    pub fn read_forget(&mut self) {
        self.read_said.clear();
    }


    pub fn rows(&self) -> Vec<(&'static str, eravon_literals::json::Out)> {
        use eravon_literals::json::Out;
        let mut m = vec![
            ("idleTicks", Out::N(self.idle_ticks)),
            (
                "links",
                Out::A(
                    self.links
                        .iter()
                        .map(|(k, n)| {
                            Out::O(vec![
                                ("bytes", Out::N(*n as u64)),
                                ("kind", Out::S((*k).to_string())),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("mode", Out::S(self.mode.name().to_string())),
            ("paylink", self.paylink.as_ref().map_or(Out::O(Vec::new()), |p| p.rows())),
            ("buyerOrders", self.buyer_orders.as_ref().map_or(Out::O(Vec::new()), |p| p.rows())),
            ("buyerHome", self.buyer_home.as_ref().map_or(Out::O(Vec::new()), |p| p.rows())),
            ("wallet", self.wallet.as_ref().map_or(Out::O(Vec::new()), |p| p.rows())),
            ("export", self.export.as_ref().map_or(Out::O(Vec::new()), |p| p.rows())),
            ("seat", Out::S(self.seat.name().to_string())),
            ("sheetOpen", Out::B(self.sheet_open)),
            ("toastBumps", Out::N(self.toast_bumps)),
            (
                "toasts",
                Out::A(
                    self.toasts
                        .iter()
                        .map(|t| {
                            let mut r = vec![
                                ("head", Out::S(t.head.clone())),
                                ("next", Out::S(t.next.clone())),
                                ("tone", Out::S(t.tone.word().to_string())),
                            ];
                            if let Some(d) = &t.detail {
                                r.push(("detail", Out::S(d.clone())));
                            }
                            r.sort_by(|a, b| a.0.cmp(b.0));
                            Out::O(r)
                        })
                        .collect(),
                ),
            ),
        ];
        m.push((
            "words",
            Out::A(
                self.words
                    .iter()
                    .map(|(k, got, text)| {
                        Out::O(vec![
                            ("got", Out::B(*got)),
                            ("key", Out::S(k.clone())),
                            ("text", Out::S(text.clone())),
                        ])
                    })
                    .collect(),
            ),
        ));
        let mut s = vec![
            ("configured", Out::B(self.sync.configured)),
            ("endpoints", Out::N(self.sync.endpoints as u64)),
            ("table", Out::S(self.sync.table.name().to_string())),
        ];
        if let Some(b) = self.sync.block {
            s.push(("block", Out::N(b)));
        }
        if let Some(t) = self.sync.chain_time {
            s.push(("chainTime", Out::N(t)));
        }
        if let Some(a) = self.sync.agreement {
            s.push(("agreement", Out::S(a.to_string())));
        }
        if let Some(at) = self.sync.read_at {
            s.push(("readAt", Out::N(at)));
            s.push(("stale", Out::B(crate::house::book::stale(at, self.clock / 1000))));
        }
        s.sort_by(|a, b| a.0.cmp(b.0));
        m.push(("sync", Out::O(s)));
        if self.sync.read_at.is_some() {
            let say = crate::house::sync::say(self.sync.table, &[], false, self.read_failed.is_some(), self.sync.read_at, self.clock / 1000);
            let (line, voice) = crate::house::sync::say_words(crate::house::i18n::Lang::Zh, &say);
            m.push(("rail", Out::O(vec![("line", Out::S(line)), ("voice", Out::S(crate::house::sync::voice_word(voice).to_string()))])));
        }
        if let Some(b) = &self.book {
            m.push(("book", b.rows(self.clock / 1000)));
        }
        if let Some(n) = self.draw_asked {
            m.push(("drawAsked", Out::N(n)));
        }
        if !self.refreshes.is_empty() {
            m.push(("refresh", Out::A(self.refreshes.iter().map(crate::house::refresh::Seen::row).collect())));
        }
        m.extend(self.seller.rows());
        if let Some(w) = &self.win {
            m.push(("win", w.clone()));
        }
        if let Some((u, v)) = &self.url_asked {
            m.push(("urlOpen", Out::O(vec![("url", Out::S(u.clone())), ("verdict", Out::S(v.clone()))])));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        m
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Write {
    Vault,
    Local,
    Chain,
}

pub fn write_block(locked: bool, writer: Option<eravon_store::lock::Mode>, mark_unread: bool, unheld: bool, has_key: bool, table: Table, w: Write) -> Option<&'static str> {
    if locked {
        return Some("write.locked");
    }
    if unheld {
        return Some("write.unheld");
    }
    match writer {
        Some(eravon_store::lock::Mode::OtherMachine) if mark_unread => return Some("write.markUnread"),
        Some(eravon_store::lock::Mode::OtherMachine) => return Some("write.otherMachine"),
        Some(eravon_store::lock::Mode::Reader) => return Some("write.reader"),
        _ => {}
    }
    if w == Write::Vault {
        return None;
    }
    if !has_key {
        return Some("write.noKey");
    }
    match (w, table) {
        (Write::Chain, Table::Unset) => Some("write.noChain"),
        (Write::Chain, Table::Unreadable) => Some("write.unread"),
        (Write::Chain, Table::Bad) => Some("write.badChain"),
        (Write::Chain, Table::WrongChain) => Some("write.wrongChain"),
        (Write::Chain, Table::Unread) => Some("write.unread"),
        _ => None,
    }
}


#[cfg(test)]
mod write_block_tests {
    use super::*;

    #[test]
    fn one_place_answers_why_an_entry_cannot_write() {
        const W: Option<eravon_store::lock::Mode> = Some(eravon_store::lock::Mode::Writer);
        use eravon_store::lock::Mode;
        assert_eq!(write_block(false, Some(Mode::OtherMachine), false, false, true, Table::Ready, Write::Local), Some("write.otherMachine"));
        assert_eq!(write_block(false, Some(Mode::OtherMachine), true, false, true, Table::Ready, Write::Local), Some("write.markUnread"), "a mark that does not read is said apart");
        assert_eq!(write_block(false, Some(Mode::Reader), false, false, true, Table::Ready, Write::Vault), Some("write.reader"));
        assert_eq!(write_block(true, Some(Mode::OtherMachine), false, false, true, Table::Ready, Write::Local), Some("write.locked"));
        assert_eq!(write_block(true, W, false, false, true, Table::Ready, Write::Local), Some("write.locked"));
        assert_eq!(write_block(false, W, false, false, false, Table::Ready, Write::Local), Some("write.noKey"));
        assert_eq!(write_block(false, W, false, false, false, Table::Unset, Write::Vault), None);
        assert_eq!(write_block(true, W, false, false, false, Table::Unset, Write::Vault), Some("write.locked"));
        assert_eq!(write_block(false, W, false, false, true, Table::Unset, Write::Local), None);
        assert_eq!(write_block(false, W, false, false, true, Table::Unset, Write::Chain), Some("write.noChain"));
        assert_eq!(write_block(false, W, false, false, true, Table::Bad, Write::Chain), Some("write.badChain"));
        assert_eq!(write_block(false, W, false, false, true, Table::Unread, Write::Chain), Some("write.unread"));
        assert_eq!(write_block(false, W, false, false, true, Table::Ready, Write::Chain), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_waits_for_the_writes_in_flight_and_says_so_once() {
        use crate::task::Kind;
        let mut sh = Shell::default();
        assert!(sh.quit(crate::house::i18n::Lang::Zh, &[]), "无写在飞即关");
        assert!(!sh.quit(crate::house::i18n::Lang::Zh, &[Kind::Goods, Kind::Export]), "写在飞即不关");
        assert!(sh.quitting);
        let said = sh.toasts.first().cloned().expect("说一次还在写");
        assert_eq!(said.head, crate::house::i18n::get(crate::house::i18n::Lang::Zh, crate::house::i18n::Key::QuitWriting).0);
        assert_eq!(said.detail.as_deref(), Some("goods · export"));
        assert!(!sh.quit(crate::house::i18n::Lang::Zh, &[Kind::Goods]));
        assert_eq!(sh.toasts.len(), 1, "只说一次");
        assert!(sh.quit(crate::house::i18n::Lang::Zh, &[]), "落完即关");
        assert!(!sh.quitting);
    }

    #[test]
    fn a_seat_is_one_of_exactly_two_and_the_judge_is_not_one() {
        assert_eq!(Seat::ALL.len(), 2);
        assert_eq!(Seat::of("judge"), None);
        assert_eq!(Seat::of("seller"), Some(Seat::Seller));
    }

    #[test]
    fn a_read_error_is_said_once_until_it_changes() {
        let mut s = Shell::default();
        let t = |h: &str| Toast { tone: crate::house::seller::Tone::Bad, head: h.into(), next: "下一步".into(), detail: None };
        s.toast_read("home", t("读链没成"));
        assert_eq!(s.toasts.len(), 1);
        s.toasts.clear();
        s.toast_read("home", t("读链没成"));
        assert!(s.toasts.is_empty(), "换页又读一趟,同一句错不该重弹");
        s.toast_read("orders", t("读链没成"));
        assert_eq!(s.toasts.len(), 1, "另一处读出的错照说");
        s.toasts.clear();
        s.read_ok("home");
        s.toast_read("home", t("读链没成"));
        assert_eq!(s.toasts.len(), 1, "读成过一回之后再错即照说");
        s.toasts.clear();
        s.read_forget();
        s.toast_read("home", t("读链没成"));
        assert_eq!(s.toasts.len(), 1, "人点了刷新即照说");
    }

    #[test]
    fn switching_to_the_sheet_opens_it_and_switching_away_closes_it() {
        let mut s = Shell::default();
        s.mode_to(Mode::Sheet);
        assert!(s.sheet_open);
        s.mode_to(Mode::Sub);
        assert!(!s.sheet_open);
    }

    #[test]
    fn a_new_sentence_replaces_the_one_on_screen() {
        let mut s = Shell::default();
        let t = |h: &str| Toast { tone: crate::house::seller::Tone::Note, head: h.into(), next: String::new(), detail: None };
        s.toast(t("一"));
        s.toast(t("二"));
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].head, "二");
        s.toast(t("二"));
        assert_eq!(s.toast_bumps, 1);
    }

    #[test]
    fn the_same_error_neither_bumps_nor_lives_past_six_seconds() {
        let mut s = Shell::default();
        let bad = || Toast { tone: crate::house::seller::Tone::Bad, head: "端点表还没配".into(), next: String::new(), detail: None };
        s.at(100_000);
        s.toast(bad());
        s.at(103_000);
        s.toast(bad());
        assert_eq!(s.toast_bumps, 0);
        s.at(105_999);
        assert_eq!(s.toasts.len(), 1);
        s.at(106_000);
        assert!(s.toasts.is_empty());
        s.toast(bad());
        assert_eq!(s.toasts.len(), 1);
    }

    #[test]
    fn the_same_sentence_does_not_stack_twice() {
        let mut s = Shell::default();
        let t = Toast {
            tone: crate::house::seller::Tone::Note,
            head: "收到一条链接".into(),
            next: "到页随后落".into(),
            detail: None,
        };
        s.toast(t.clone());
        s.toast(t);
        assert_eq!(s.toasts.len(), 1);
    }
}
