use crate::house::desk::{wall_now, Duty, Flying, Site};
use crate::house::landed::Landed;
use crate::house::seller::{Page, Said, Tone};
use crate::house::shell::{Seat, Toast};
use crate::shell::Shell;
use crate::task::{Done, Kind, Spawned};

const PAGE_READS: [Kind; 6] = [Kind::Home, Kind::Orders, Kind::BHome, Kind::Bo, Kind::Wallet, Kind::Export];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ask {
    Read,
    Asked,
    Act,
}

pub(crate) fn pass_kind(kind: Kind, ask: Ask) -> Kind {
    if ask == Ask::Read { kind.read_of().unwrap_or(kind) } else { kind }
}

impl From<bool> for Ask {
    fn from(act: bool) -> Ask {
        if act { Ask::Act } else { Ask::Read }
    }
}

impl Ask {
    pub(crate) fn writes(self) -> bool {
        self == Ask::Act
    }

    pub(crate) fn asked(self) -> bool {
        self != Ask::Read
    }
}

impl Shell {
    pub(crate) fn busy(&mut self, ask: Ask) {
        self.desk.retry_next = None;
        if ask.asked() {
            self.desk.busy_said();
            self.desk.not_started = Some(crate::house::i18n::t(crate::house::i18n::Key::ErrBusy));
        }
    }

    pub(crate) fn refused(&mut self, ask: Ask, s: &crate::house::seller::Said) {
        self.desk.retry_next = None;
        if ask.asked() {
            self.desk.not_started = Some(format!("{}:{}", s.name, s.detail));
        }
    }

    pub(crate) fn started(&mut self, go: bool) -> Result<(), String> {
        let why = self.desk.not_started.take();
        if go {
            return Ok(());
        }
        Err(why.unwrap_or_else(|| crate::house::i18n::t(crate::house::i18n::Key::ErrBusy)))
    }

    pub(crate) fn house_spawn(&mut self, kind: Kind, site: Site, ask: impl Into<Ask>, f: impl FnOnce(&crate::house::Ticket) -> Landed + Send + 'static) -> bool {
        let ask = ask.into();
        if ask == Ask::Read && PAGE_READS.contains(&kind) && self.seat_pending() {
            return false;
        }
        let kind = pass_kind(kind, ask);
        match crate::action::spawn_for(self, kind, move || Ok(Done::House(Box::new(f(&crate::house::Ticket))))) {
            Spawned::Started => {
                self.desk.flying.retain(|(k, _)| *k != kind);
                self.desk.flying.push((kind, Flying { site, act: ask.writes() }));
                true
            }
            Spawned::InFlight => {
                self.busy(ask);
                false
            }
        }
    }

    pub(crate) fn seat_pending(&self) -> bool {
        (self.desk.wizard_up.is_some() || self.desk.wizard_due)
            && !self.desk.ready.seat
            && eravon_vault::store::nodes_in(&self.settings).is_empty()
    }


    pub(crate) fn house_in_flight(&self, kind: Kind) -> bool {
        self.tasks.in_flight(kind)
    }

    pub(crate) fn page_in_flight(&self, kind: Kind) -> bool {
        self.house_in_flight(kind) || kind.read_of().is_some_and(|r| self.house_in_flight(r))
    }

    pub(crate) fn house_acting(&self, kind: Kind) -> bool {
        self.tasks.in_flight(kind) && self.desk.flying(kind).is_some_and(|f| f.act)
    }

    pub(crate) fn moved(&mut self) {
        self.tasks.new_epoch();
        self.desk.refresh_again = None;
        self.desk.reshow_again = false;
        self.drop_parked();
    }

    pub(crate) fn drop_parked(&mut self) {}

    pub(crate) fn covered(&self) -> bool {
        self.desk.wizard_up.is_some() || self.vault.gate_up()
    }

    pub(crate) fn write_why(&self, w: crate::house::shell::Write) -> Option<String> {
        self.write_cause(w).map(|k| crate::house::i18n::get_named(crate::lang::lang(), k).0)
    }

    pub(crate) fn write_cause(&self, w: crate::house::shell::Write) -> Option<&'static str> {
        let writer = self.lock.as_ref().map(|l| l.mode());
        let mark_unread = self.lock.as_ref().and_then(|l| l.mark_trouble()).is_some();
        crate::house::shell::write_block(!self.unlocked(), writer, mark_unread, self.home.is_none() && self.unlocked(), !self.desk.addr.is_empty(), self.desk.sh.sync.table, w)
    }

    pub(crate) fn refresh_addr(&mut self) {
        if self.take_addr() {
            self.refresh_for(crate::house::refresh::Cause::Switch, None);
        }
    }

    pub(crate) fn zone_follow(&self) {
        let z = crate::when::chosen(self.settings.zone);
        crate::when::set(z);
        eravon_orders::when::set(crate::house::settings::zone_of(z.as_str()));
    }

    fn take_addr(&mut self) -> bool {
        let seat = crate::house::shell::Seat::of_store(eravon_vault::vault::seat_of(self.settings.role));
        self.desk.sh.set_seat(seat);
        let was = std::mem::take(&mut self.desk.addr);
        let was_seat = self.desk.addr_seat.replace(seat);
        self.desk.addr = crate::house::shell::current_key(&self.desk.data, seat).unwrap_or_default();
        self.reread_ready();
        if !was.eq_ignore_ascii_case(&self.desk.addr) || was_seat.is_some_and(|s| s != seat) {
            self.moved();
            self.sync_forget();
            self.zone_follow();
            if was_seat.is_some_and(|s| s != seat) {
                self.desk.sh.export = None;
            }
            self.desk.sh.book = if self.unlocked() { crate::house::book::load(&self.desk.data, &self.desk.addr) } else { None };
            if self.unlocked() && self.seat_unready() {
                self.desk.wizard_due = true;
            }
            self.reshow();
            if !was.is_empty() && was_seat == Some(seat) {
                self.reenter_shown();
            }
            return true;
        }
        false
    }

    pub(crate) fn reenter_shown(&mut self) {
        let d = &self.desk.sh;
        let detail = d.seller.orders.as_ref().is_some_and(|o| o.sel.is_some())
            || d.buyer_orders.as_ref().is_some_and(|b| b.open.is_some())
            || d.seller.goods.as_ref().is_some_and(|g| g.edit.is_some() || g.shown.is_some())
            || d.seller.cands.as_ref().is_some_and(|c| c.detail.is_some());
        if !detail {
            let (goto, card) = (self.desk.goto.take(), self.desk.set_card.take());
            self.go_page_now(self.desk.sh.seller.page());
            self.desk.goto = goto;
            if self.desk.set_card.is_none() {
                self.desk.set_card = card;
            }
        }
    }

    fn page_stale(&self) -> bool {
        let (d, addr) = (&self.desk.sh, &self.desk.addr);
        if addr.is_empty() || self.desk.seat() != Seat::Seller {
            return false;
        }
        match d.seller.page() {
            Some(Page::Goods) => d.seller.goods.as_ref().is_some_and(|g| !g.me.is_empty() && g.me != *addr && g.edit.is_none() && g.shown.is_none()) && !self.house_in_flight(Kind::Goods),
            Some(Page::Cands) => d.seller.cands.as_ref().is_some_and(|c| !c.me.is_empty() && c.me != *addr && c.detail.is_none()) && !self.page_in_flight(Kind::Cands),
            _ => false,
        }
    }

    pub(crate) fn open_pay(&mut self) {
        let now = self.desk.now();
        self.desk.sh.seller.pay = Some(crate::house::seller::Pay::open(&self.desk.data, &self.desk.addr, now));
    }

    pub(crate) fn house_door_landed(&mut self) {
        let moved = self.settings_written();
        if !self.unlocked() || self.desk.addr.is_empty() {
            return;
        }
        if moved {
            self.endpoints_landed();
        }
        if crate::action::follow_register(self) == crate::action::Followed::Moved {
            self.seat_follow();
        }
        let moved = self.take_addr();
        if !moved {
            self.reenter_shown();
        }
        self.desk.sh.read_forget();
        self.refresh_for(if moved { crate::house::refresh::Cause::Switch } else { crate::house::refresh::Cause::Key }, None);
    }

    pub(crate) fn settings_written(&mut self) -> bool {
        if let Ok(m) = crate::machine::read() {
            self.machine = m;
        }
        let Some(h) = self.home.as_ref() else { return false };
        let Ok(now) = crate::settings::Settings::read(h) else { return false };
        let moved = crate::chainx::nodes_of(&self.settings) != crate::chainx::nodes_of(&now);
        self.settings = now;
        self.zone_follow();
        if moved {
            crate::chainx::forget_serving();
        }
        moved
    }

    fn seat_unready(&self) -> bool {
        !self.desk.ready.seat && !self.desk.addr.is_empty()
    }


    pub(crate) fn reread_ready(&mut self) {
        self.desk.ready = if self.unlocked() { crate::house::first::Ready::read(&self.desk.data, self.desk.seat().store(), &self.desk.addr) } else { crate::house::first::Ready::default() };
    }

    pub(crate) fn sync_forget(&mut self) {
        self.desk.sync_done_at = None;
        let reads = self.desk.seat_reads();
        let (data, addr) = (self.desk.data.clone(), self.desk.addr.clone());
        crate::house::sync::forget(&mut self.desk.sh, &data, &addr, reads);
    }

    pub(crate) fn refresh_for(&mut self, cause: crate::house::refresh::Cause, only: Option<String>) {
        use crate::house::action::Action as Step;
        self.desk.sh.refresh_at = Some(self.desk.pace_now());
        let quiet = cause == crate::house::refresh::Cause::Period;
        if quiet {
            self.period_draw();
        }
        if !self.unlocked() {
            self.desk.refresh_again = None;
            self.desk.tr.push(Step::Refresh, crate::house::mark::FEATURE, "锁着,不读".into());
            return;
        }
        if !self.desk.seat_reads() {
            self.desk.sh.sync.table = crate::house::shell::Table::Unread;
            self.desk.tr.push(Step::Refresh, crate::house::mark::FEATURE, "这一席还没有钥,不读".into());
            return;
        }
        match crate::house::sync::table(&self.desk.data, &self.desk.addr) {
            crate::house::sync::Endpoints::Unset if !self.desk.ready.seat => {
                self.desk.sh.sync.table = crate::house::shell::Table::Unread;
                self.desk.tr.push(Step::Refresh, crate::house::mark::FEATURE, "这一席还没就绪,不读".into());
            }
            crate::house::sync::Endpoints::Unset => {
                if !quiet {
                    crate::house::sync::unconfigured(&mut self.desk.sh);
                } else {
                    self.desk.sh.sync = crate::house::shell::Sync { configured: false, table: crate::house::shell::Table::Unset, ..Default::default() };
                }
                self.desk.tr.push(Step::Refresh, crate::house::mark::FEATURE, "未配置端点".into());
            }
            crate::house::sync::Endpoints::Unreadable(name, detail) => {
                if quiet {
                    self.desk.sh.sync = crate::house::shell::Sync { configured: false, table: crate::house::shell::Table::Unreadable, ..Default::default() };
                } else {
                    crate::house::sync::unreadable(&mut self.desk.sh, &name, &detail);
                }
                self.desk.tr.push(Step::Refresh, crate::house::mark::FEATURE, format!("设置读不成:{name}"));
            }
            crate::house::sync::Endpoints::Bad(e) => {
                let name = e.name().to_string();
                if !quiet {
                    self.desk.sh.toast(crate::house::sync::fault_toast(&e));
                }
                self.desk.tr.push(Step::Refresh, crate::house::mark::FEATURE, format!("端点表不认:{name}"));
            }
            crate::house::sync::Endpoints::Ready(t) => {
                crate::house::sync::mark_configured(&mut self.desk.sh, t.len());
                if self.house_in_flight(Kind::Sync) {
                    self.desk.refresh_again = Some(match self.desk.refresh_again.take() {
                        None => (cause, only),
                        Some((c, o)) if c == cause && o == only => (c, o),
                        Some(_) => (crate::house::refresh::Cause::Key, None),
                    });
                    return;
                }
                let plan = crate::house::refresh::plan(&self.desk.session, &self.desk.data, &self.desk.addr, self.desk.seat(), cause, only, self.desk.pace_now(), crate::house::Ticket::standing()).ok();
                let prior = plan.as_ref().and_then(|_| self.desk.sh.book.clone().filter(|b| b.key.eq_ignore_ascii_case(&self.desk.addr)));
                self.desk.refresh_cause = plan.as_ref().map(|p| (p.cause, p.only.clone()));
                if self.sync_start(t, plan.map(|p| (p, prior))) {
                    self.desk.sync_done_at = None;
                }
                self.desk.tr.push(Step::Refresh, crate::house::mark::FEATURE, "起了一趟后台读".into());
            }
        }
    }

    fn sync_start(&mut self, t: eravon_chain::endpoints::Table, refresh: crate::house::sync::Refresh) -> bool {
        self.house_spawn(Kind::Sync, Site::Page, false, move |_| {
            let (tick, failed) = crate::house::sync::read(t, refresh);
            Landed::Sync(tick.map(Box::new), failed)
        })
    }


    pub(crate) fn entered(&mut self) {
        let book = self.desk.sh.book.as_ref().filter(|b| b.key.eq_ignore_ascii_case(&self.desk.addr));
        if self.unlocked() && !self.house_in_flight(Kind::Sync) && crate::house::refresh::aged(book, self.desk.pace_now()) {
            self.refresh_for(crate::house::refresh::Cause::Page, None);
        }
    }

    pub(crate) fn reshow(&mut self) {
        self.desk.probes.clear();
        if !self.unlocked() || self.desk.addr.is_empty() || self.desk.data.is_empty() {
            self.desk.reshow_again = false;
            return;
        }
        self.desk.reshow_again = PAGE_READS.iter().any(|k| self.page_in_flight(*k));
        match self.desk.seat() {
            Seat::Seller => {
                self.spawn_home();
                self.orders_load_as(self.desk.reshow_open);
                if self.desk.sh.wallet.is_some() {
                    let shown = self.desk.shown();
                    self.spawn_wallet_at(Site::Page, false, move |s, data, me, mut w, _| {
                        crate::house::refresh::draw(&shown, || {
                            w.reread(s, data, me, true);
                            w.read_vault(data, me);
                        });
                        w
                    });
                }
            }
            Seat::Buyer => {
                self.spawn_bhome();
                let shown = self.desk.shown();
                let reread = self.desk.reshow_open;
                self.spawn_bo_at(Site::Page, false, move |s, data, me, mut cur| {
                    let b = crate::house::refresh::draw(&shown, || crate::house::buyer_orders::BOrders::load(s, data, me));
                    if !cur.fresh_for(me) {
                        return b;
                    }
                    let before = cur.open_follow();
                    cur.take_readings(b);
                    let (whole, shape) = (cur.read_whole(), cur.open_follow());
                    if cur.follow.moved(whole, before, shape) && reread {
                        cur.reopen(data, me);
                    } else {
                        cur.readvance(data, me);
                    }
                    cur
                });
                let shown = self.desk.shown();
                self.spawn_wallet_at(Site::Page, false, move |s, data, me, mut w, _| {
                    crate::house::refresh::draw(&shown, || w.reread(s, data, me, false));
                    w
                });
            }
        }
        let shown = self.desk.shown();
        self.spawn_export_at(Site::Page, false, move |s, data, me, mut cur, _| {
            crate::house::refresh::draw(&shown, || cur.reload(s, data, me));
            cur
        });
        let book = self.desk.sh.book.clone();
        if let Some(s) = self.desk.sh.seller.settings.as_mut() {
            s.from_book(book.as_ref());
        }
    }

    pub(crate) fn duty(&mut self, d: Duty) {
        if self.duty_waits(d) {
            if !self.desk.owed.contains(&d) {
                self.desk.owed.push(d);
                self.desk.tr.push(d.step(), crate::house::mark::FEATURE, "槽忙,欠着,槽空即起".into());
            }
            return;
        }
        self.duty_start(d);
    }

    fn duty_waits(&self, d: Duty) -> bool {
        match d {
            Duty::Backstop => self.page_in_flight(Kind::Orders) || (self.covered() && self.unlocked()),
            Duty::Sweep => self.house_in_flight(Kind::Goods),
        }
    }

    fn duty_start(&mut self, d: Duty) {
        match d {
            Duty::Backstop => self.orders_backstop(),
            Duty::Sweep => {
                let now = self.desk.now();
                self.spawn_goods(move |gd, data, addr, field, signer| gd.sweep(data, addr, field, now, signer));
            }
        }
    }

    pub(crate) fn pay_owed(&mut self) {
        if self.desk.owed.is_empty() {
            return;
        }
        for d in std::mem::take(&mut self.desk.owed) {
            if self.duty_waits(d) {
                self.desk.owed.push(d);
            } else {
                self.duty_start(d);
            }
        }
    }

    pub(crate) fn go_page_now(&mut self, p: Option<Page>) -> bool {
        use crate::house::action::Action as Step;
        if p.is_some() && p != Some(Page::Settings) && p != Some(Page::PayLink) && self.desk.addr.is_empty() {
            self.refresh_addr();
            if self.desk.addr.is_empty() {
                let why = crate::house::shell::current_key(&self.desk.data, self.desk.seat()).err().unwrap_or_default();
                self.desk.sh.toast(Toast { tone: Tone::Bad, head: crate::house::i18n::t(crate::house::i18n::Key::ErrNoKey), next: String::new(), detail: Some(why) });
                return false;
            }
        }
        if !self.desk.sh.go(crate::house::seller::Leave::Page(p)) {
            return false;
        }
        self.desk.add_form = None;
        self.desk.rename = None;
        self.desk.confirm_remove = false;
        self.desk.goto = Some(p);
        if matches!(p, None | Some(Page::Alerts | Page::Orders | Page::Wallet | Page::Export)) {
            self.entered();
        }
        let seat = self.desk.seat();
        match p {
            Some(Page::PayLink) => {
                let addr = self.desk.addr.clone();
                if !self.desk.sh.paylink.as_ref().is_some_and(|p| p.owner.eq_ignore_ascii_case(&addr)) {
                    self.desk.sh.paylink = Some(crate::house::paylink::PayLink::open(&self.desk.sh.seller.field.home, &self.desk.data, &addr));
                }
                self.desk.tr.push(Step::Page, crate::house::mark::FEATURE_P1, crate::house::seam::PAGE_PAYLINK.into());
            }
            Some(Page::Orders) if seat == Seat::Buyer => {
                self.load_bo();
                self.desk.tr.push(Step::Page, crate::house::mark::FEATURE_P5, crate::house::seam::PAGE_ORDERS.into());
            }
            Some(Page::Payways) => {
                self.open_pay();
                self.desk.tr.push(Step::Page, eravon_payways::mark::FEATURE, crate::house::seam::PAGE_PAYWAYS.into());
            }
            Some(Page::Cands) => {
                let tab = self.desk.sh.seller.cands.as_ref().map_or(eravon_cands::table::Side::Judge, |c| c.tab);
                let (data, addr) = (self.desk.data.clone(), self.desk.addr.clone());
                crate::house::seller::Cands::entered(&mut self.desk.sh.seller.cands, &addr);
                if self.desk.sh.seller.cands.is_none() {
                    let (d, a) = (data.clone(), addr.clone());
                    self.house_spawn(Kind::CandsLocal, Site::Page, Ask::Read, move |_| Landed::Cands(Box::new(crate::house::merge::Pass::new(None, crate::house::seller::Cands::local(&d, &a, tab), true))));
                }
                self.load_cands(tab);
                self.desk.tr.push(Step::Page, eravon_cands::mark::FEATURE, crate::house::seam::PAGE_CANDS.into());
            }
            Some(Page::Settings) => {
                self.open_settings();
                self.desk.tr.push(Step::Page, eravon_literals::trace::Feature::S9, crate::house::seam::PAGE_SETTINGS.into());
            }
            Some(Page::Goods) => {
                let (data, addr, field) = (self.desk.data.clone(), self.desk.addr.clone(), self.desk.sh.seller.field.clone());
                crate::house::seller::Goods::entered(&mut self.desk.sh.seller.goods, &addr);
                if self.desk.sh.seller.goods.is_none() {
                    let (d, a) = (data.clone(), addr.clone());
                    self.house_spawn(Kind::Goods, Site::Page, Ask::Read, move |_| Landed::Goods(Box::new(crate::house::merge::Pass::new(None, crate::house::seller::Goods::local(&d, &a), true))));
                }
                if !self.house_in_flight(Kind::Goods) {
                    let from = self.desk.sh.seller.goods.clone();
                    let signer = eravon_goods::sign::Signer::of(Some(&self.desk.session), &self.desk.addr);
                    let now = self.desk.now();
                    self.house_spawn(Kind::Goods, Site::Page, Ask::Asked, move |t| {
                        let t = *t;
                        let mut g = from.clone().unwrap_or_else(|| crate::house::seller::Goods::local(&data, &addr));
                        g.said = None;
                        g.sweep(&data, &addr, &field, now, &signer.bound(std::sync::Arc::new(move || t.live())));
                        Landed::Goods(Box::new(crate::house::merge::Pass::new(from, g, true)))
                    });
                }
                self.desk.tr.push(Step::Page, eravon_goods::mark::FEATURE, crate::house::seam::PAGE_GOODS.into());
            }
            Some(Page::Orders) => {
                self.orders_load();
                self.desk.tr.push(Step::Page, eravon_watch::mark::FEATURE, crate::house::seam::PAGE_ORDERS.into());
            }
            Some(Page::Wallet) => {
                self.load_wallet();
                self.desk.tr.push(Step::Page, eravon_wallet::mark::FEATURE, crate::house::seam::PAGE_WALLET.into());
            }
            Some(Page::Export) => {
                self.load_export();
                self.desk.tr.push(Step::Page, eravon_export::mark::FEATURE, crate::house::seam::PAGE_EXPORT.into());
            }
            Some(Page::Queue) => {
                let (data, me) = (self.desk.data.clone(), self.desk.addr.clone());
                let direct = Some(crate::house::queue::direct_of(&self.desk.session, &me));
                let orders = (!data.is_empty() && !me.is_empty()).then(|| crate::house::queue::orders_of(&data, &me));
                self.desk.sh.queue = Some(crate::house::queue::View { direct, orders, open_direct: None, said: None });
                self.desk.tr.push(Step::Page, crate::house::mark::FEATURE, crate::house::seam::PAGE_QUEUE.into());
            }
            Some(Page::Alerts) | None if seat == Seat::Buyer => {
                self.spawn_bhome();
                self.desk.tr.push(Step::Page, crate::house::mark::FEATURE_P8, if p.is_some() { crate::house::seam::PAGE_ALERTS } else { crate::house::seam::PAGE_HOME }.into());
            }
            Some(Page::Alerts) => {
                self.spawn_home();
                self.desk.tr.push(Step::Page, eravon_home::mark::FEATURE, crate::house::seam::PAGE_ALERTS.into());
            }
            None => {
                if seat == Seat::Seller {
                    self.spawn_home();
                }
            }
        }
        self.desk.flush();
        true
    }

    pub(crate) fn key_born(&mut self, row: &crate::identity::Row) {
        use crate::house::action::Action as Step;
        let data = if self.desk.data.is_empty() { crate::house::data().unwrap_or_default() } else { self.desk.data.clone() };
        if data.is_empty() {
            return;
        }
        let here = eravon_vault::vault::role_of(self.desk.seat().store());
        let Some(me) = row.address(here).or_else(|| row.seats().into_iter().find_map(|r| row.address(r))).map(|a| a.hex()) else { return };
        let chain = crate::house::first::chain_on(self.desk.new_chain.take(), None);
        match crate::house::first::born_added(&data, &me, chain, self.desk.now()) {
            Ok(n) => {
                if self.settings_written() {
                    self.endpoints_landed();
                }
                self.desk.tr.push(Step::Seat, eravon_vault::mark::FEATURE, format!("添钥出生 · {n}"));
            }
            Err(s) => {
                self.desk.tr.push(Step::Seat, eravon_vault::mark::FEATURE, format!("添钥出生没成 · {}", s.name));
                self.desk.sh.toast(Toast { tone: Tone::Bad, head: s.detail.clone(), next: String::new(), detail: Some(s.name.clone()) });
            }
        }
    }

    pub(crate) fn seat_follow(&mut self) {
        use crate::house::action::Action as Step;
        let seat = crate::house::shell::Seat::of_store(eravon_vault::vault::seat_of(self.settings.role));
        let from = (self.desk.seat(), self.desk.sh.seller.page());
        if seat != from.0 {
            if !self.desk.sh.go(crate::house::seller::Leave::Seat(seat)) {
                self.desk.role_landed = None;
                if let crate::action::Applied::Trouble(f) = crate::action::apply(self, crate::action::Action::SwitchRole) {
                    self.desk.tr.push(Step::Seat, crate::house::mark::FEATURE, format!("{} · {}", f.said(), f.tail()));
                }
                self.desk.goto = Some(self.desk.sh.seller.page());
                self.desk.flush();
                return;
            }
        }
        self.refresh_addr();
        crate::house::exec::first::role_landed(self);
        if let Some(p) = self.desk.goto_landed.take() {
            if self.desk.seat() == crate::house::shell::Seat::Buyer {
                self.desk.goto = Some(Some(p));
            }
        }
        if self.desk.sh.seller.page() == Some(Page::Settings) {
            self.open_settings();
        }
        let label = crate::house::i18n::get_named(crate::lang::lang(), &format!("seat.{}", seat.name())).0;
        self.desk.tr.push(Step::Seat, crate::house::mark::FEATURE, label);
        if self.unlocked() && self.seat_unready() {
            self.desk.wizard_due = true;
            self.desk.tr.push(Step::First, crate::house::mark::FEATURE, "这一席还没就绪".into());
        }
        self.desk.flush();
    }

    pub(crate) fn unlock_house(&mut self) {
        use crate::house::action::Action as Step;
        if self.desk.data.is_empty() {
            self.desk.data = crate::house::data().unwrap_or_default();
            self.desk.session = eravon_vault::session::Session::locked(&self.desk.data);
        }
        let _ = self.desk.session.opened();
        let data = self.desk.data.clone();
        self.desk.sh.adopt(&data);
        self.moved();
        self.take_addr();
        self.desk.sh.last_input = wall_now();
        let seat = self.desk.seat();
        if self.seat_unready() {
            self.desk.wizard_due = true;
        }
        self.desk.sh.book = crate::house::book::load(&data, &self.desk.addr);
        self.refresh_for(crate::house::refresh::Cause::Unlock, None);
        self.reshow();
        if seat == Seat::Seller && !self.desk.addr.is_empty() {
            self.duty(Duty::Backstop);
            self.desk.tr.push(Step::OrderScan, eravon_watch::mark::FEATURE, "解锁后补扫".into());
            self.spawn_home();
        }
        if self.desk.sh.seller.goods.is_some() {
            self.duty(Duty::Sweep);
        }
        if seat == Seat::Buyer && self.desk.sh.buyer_orders.as_ref().is_some_and(|b| b.rows.as_ref().is_none_or(|r| r.is_err())) {
            self.desk.sh.buyer_orders = None;
        }
        if seat == Seat::Buyer && crate::house::buyer_home::resumes(self.desk.sh.home_asked || self.desk.sh.stash.bhome, self.desk.sh.buyer_home.is_some()) {
            self.desk.sh.home_asked = false;
            self.spawn_bhome();
        }
        let stash = std::mem::take(&mut self.desk.sh.stash);
        if stash.bo.is_some() && seat == Seat::Buyer {
            self.load_bo();
        }
        if stash.wallet && seat == Seat::Buyer {
            self.load_wallet();
        }
        if stash.export.is_some() {
            self.load_export();
        }
        if stash.cands.is_some() && self.desk.sh.seller.page() == Some(Page::Cands) {
            self.go_page_now(Some(Page::Cands));
        }
        self.desk.flush();
    }

    pub(crate) fn lock_house(&mut self) {
        use crate::house::action::Action as Step;
        self.desk.session.lock();
        self.moved();
        self.desk.sh.lock_reads();
        self.desk.probes.clear();
        self.desk.missed_at = None;
        eravon_chain::read::forget_checked();
        if let Some(c) = self.desk.pay_cancel.take() {
            c.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        let word = crate::house::i18n::t(crate::house::i18n::Key::PinLockedTitle);
        self.desk.tr.push(Step::Lock, crate::house::mark::FEATURE, word);
        self.desk.flush();
    }

    fn period_draw(&mut self) {
        if self.desk.addr.is_empty() || !self.unlocked() {
            return;
        }
        if self.desk.seat() == Seat::Buyer {
            if !self.page_in_flight(Kind::Bo) {
                self.spawn_bo_at(Site::Page, false, |s, data, me, mut cur| {
                    cur.draw_due(s, data, me);
                    cur
                });
            }
            return;
        }
        if self.desk.seat() != Seat::Seller || self.page_in_flight(Kind::Orders) {
            return;
        }
        let (data, addr) = (self.desk.data.clone(), self.desk.addr.clone());
        let key = crate::house::orders::Key::of(&self.desk.session, &addr);
        let now = self.desk.now();
        self.spawn_orders(Site::Page, false, move |mut cur, t| {
            let key = key.bound(t);
            if let Some(s) = crate::house::orders::draw_round(&data, &addr, now, &key) {
                cur.said = Some(s);
            }
            cur
        });
    }

    pub fn house_wake_in(&self) -> Option<std::time::Duration> {
        if self.desk.data.is_empty() || !self.unlocked() {
            return None;
        }
        let now = self.desk.pace_now();
        let left = |last: Option<u64>, every: u64| -> Option<u64> {
            let t = last.filter(|_| every > 0)?;
            Some(if now < t { 0 } else { t.saturating_add(every).saturating_sub(now) })
        };
        let period = left(self.desk.sh.refresh_at, self.desk.sh.refresh_every);
        let missed = self.desk.probes.iter().any(|r| r.missed()).then(|| left(self.desk.missed_at, eravon_literals::constants::READ_REFRESH_SECS).unwrap_or(0));
        let door = crate::house::approve::next_due(self).map(|t| t.saturating_sub(now));
        [period, missed, door].into_iter().flatten().min().map(|s| std::time::Duration::from_secs(s.max(1)))
    }

    pub fn house_tick(&mut self) {
        if self.desk.data.is_empty() {
            return;
        }
        for u in crate::house::openurl::drain() {
            self.take_url(&u);
        }
        let open = self.unlocked();
        if open && !self.covered() && !self.house_in_flight(Kind::Sync) && crate::house::refresh::due(self.desk.sh.refresh_at, self.desk.pace_now(), self.desk.sh.refresh_every) {
            self.refresh_for(crate::house::refresh::Cause::Period, None);
        }
        if open
            && !self.house_in_flight(Kind::Sync)
            && self.desk.probes.iter().any(|r| r.missed())
            && (self.desk.missed_at.is_none() || crate::house::refresh::due(self.desk.missed_at, self.desk.pace_now(), eravon_literals::constants::READ_REFRESH_SECS))
        {
            self.desk.probes.clear();
            self.desk.missed_at = Some(self.desk.pace_now());
            self.refresh_for(crate::house::refresh::Cause::Page, None);
        }
        self.pay_owed();
        self.rotate_list_due();
        if open && self.page_stale() {
            self.reenter_shown();
        }
        if self.desk.reshow_again && !PAGE_READS.iter().any(|k| self.page_in_flight(*k)) {
            self.reshow();
        }
        if !self.desk.pending_url.is_empty() && !self.house_in_flight(Kind::PayLink) && !self.page_in_flight(Kind::Bo) && !self.covered() {
            if let Some(u) = self.desk.pending_url.pop_front() {
                self.take_url(&u);
            }
        }
        self.remind();
    }

    pub(crate) fn rotate_list_due(&mut self) {
        let owed = matches!(self.desk.set_card, Some(crate::house::cards::SetCard::Rotate { .. })) && self.desk.sh.seller.settings.as_ref().is_some_and(|s| s.rotate.is_none());
        if !owed || !self.unlocked() || self.house_in_flight(Kind::Settings) {
            return;
        }
        let now = self.desk.now();
        self.spawn_settings_keyed(crate::house::action::Action::SetRotate, move |s, ses, data| s.rotate_list(data, ses, now));
    }

    pub fn house_busy(&self) -> bool {
        Kind::ALL.iter().any(|k| self.desk.flying(*k).is_some() && self.house_in_flight(*k)) || self.desk.reshow_again || !self.desk.owed.is_empty()
    }

    pub fn page_opening(&self) -> bool {
        let f = |k: Kind| self.house_in_flight(k);
        match (self.desk.seat(), self.desk.sh.seller.page()) {
            (_, Some(Page::Settings)) => f(Kind::Settings) || f(Kind::Vault) || f(Kind::CliPath),
            (_, Some(Page::Export)) => f(Kind::Export),
            (Seat::Seller, Some(Page::Cands)) => f(Kind::Cands),
            (Seat::Seller, Some(Page::Goods)) => f(Kind::Goods),
            (Seat::Seller, Some(Page::Orders)) => f(Kind::Orders),
            (Seat::Seller, None | Some(Page::Alerts)) => f(Kind::Home),
            (Seat::Buyer, Some(Page::PayLink)) => f(Kind::PayLink),
            (Seat::Buyer, Some(Page::Orders)) => f(Kind::Bo),
            (_, Some(Page::Wallet)) => f(Kind::Wallet),
            (Seat::Buyer, None | Some(Page::Alerts)) => f(Kind::BHome),
            _ => false,
        }
    }

    fn remind(&mut self) {
        if self.covered() || self.desk.remind.is_empty() || self.house_in_flight(Kind::Remind) {
            return;
        }
        let items = std::mem::take(&mut self.desk.remind);
        let title = crate::house::i18n::t(crate::house::i18n::Key::NavAlerts);
        let to = crate::notifier::notifier(std::path::Path::new(&self.desk.data));
        let back = items.clone();
        if !self.house_spawn(Kind::Remind, Site::Page, true, move |_| {
            Landed::Remind(
                items
                    .into_iter()
                    .map(|r| {
                        let e = crate::notifier::notify(&to, &title, &r.say).err();
                        (r, e)
                    })
                    .collect(),
            )
        }) {
            self.desk.remind = back;
        }
    }

    pub(crate) fn house_landed(&mut self, kind: Kind, got: Landed, stale: bool, failed: Option<crate::fault::Fault>) {
        let fl = self.desk.flying(kind).unwrap_or(Flying { site: Site::Page, act: false });
        if !self.tasks.in_flight(kind) {
            self.desk.flying.retain(|(k, _)| *k != kind);
        }
        let failed = failed.map(|f| eravon_chain::fault::Fault::Execution(format!("{} · {}", f.said(), f.tail())));
        crate::house::landed::land(self, kind, got, stale, fl, failed);
    }

    pub(crate) fn house_failed(&mut self, kind: Kind, f: &crate::fault::Fault) {
        if !self.tasks.in_flight(kind) {
            self.desk.flying.retain(|(k, _)| *k != kind);
        }
        if kind == Kind::Settings && !self.tasks.in_flight(kind) {
            self.rekeying = false;
        }
        self.desk.sh.toast(Toast { tone: Tone::Bad, head: f.human().to_string(), next: f.next().to_string(), detail: Some(f.shown()) });
    }

    pub(crate) fn endpoints_landed(&mut self) {
        self.sync_forget();
        let ends = match crate::house::sync::table(&self.desk.data, &self.desk.addr) {
            crate::house::sync::Endpoints::Ready(t) => crate::house::refresh::endpoints_of(&t),
            _ => String::new(),
        };
        if let Some(b) = self.desk.sh.book.as_mut() {
            b.void_provenance(&ends);
        }
        self.refresh_for(crate::house::refresh::Cause::Endpoints, None);
    }

    pub(crate) fn land_stale(&mut self, kind: Kind, act: bool, said: Option<Said>) {
        let again = self.desk.retry.iter().position(|(k, _)| *k == kind).map(|i| self.desk.retry.remove(i).1);
        if !act {
            self.reenter_read(kind);
            return;
        }
        match said {
            Some(s) if s.tone == Tone::Ok => self.desk.sh.toast(Toast { tone: Tone::Ok, head: s.detail, next: String::new(), detail: None }),
            other => self.reenter_act(again, other),
        }
    }

    fn reenter_act(&mut self, again: Option<crate::house::act::Act>, said: Option<Said>) {
        match (again, said) {
            (Some(a), _) => crate::house::act::apply_said(self, a),
            (None, Some(s)) => self.desk.sh.toast(Toast { tone: s.tone, head: s.detail.clone(), next: crate::house::sync::next_of(&s), detail: Some(s.name) }),
            (None, None) => self.desk.sh.toast(Toast { tone: Tone::Bad, head: crate::house::i18n::t(crate::house::i18n::Key::ErrESTORESTALE), next: String::new(), detail: Some("E_STORE_STALE".to_string()) }),
        }
    }


    pub(crate) fn load_cands(&mut self, tab: eravon_cands::table::Side) {
        let (data, addr, field) = (self.desk.data.clone(), self.desk.addr.clone(), self.desk.sh.seller.field.clone());
        let now = self.desk.now();
        self.spawn_cands(Site::Page, Ask::Read, move |_| crate::house::seller::Cands::open(&data, &addr, &field, now, tab));
    }

    pub(crate) fn reenter_read(&mut self, kind: Kind) {
        let page = self.desk.sh.seller.page();
        match (kind, self.desk.seat(), page) {
            (Kind::Sync, ..) => self.refresh_for(crate::house::refresh::Cause::Key, None),
            (Kind::Orders, Seat::Seller, Some(Page::Orders)) => self.orders_load(),
            (Kind::Home, Seat::Seller, None | Some(Page::Alerts)) => self.spawn_home(),
            (Kind::BHome, Seat::Buyer, None | Some(Page::Alerts)) => self.spawn_bhome(),
            (Kind::Wallet, _, Some(Page::Wallet)) => self.load_wallet(),
            (Kind::Cands, Seat::Seller, Some(Page::Cands)) => self.load_cands(self.desk.sh.seller.cands.as_ref().map_or(eravon_cands::table::Side::Judge, |c| c.tab)),
            (Kind::Bo, Seat::Buyer, Some(Page::Orders)) => self.load_bo(),
            (Kind::Export, _, Some(Page::Export)) => self.load_export(),
            _ => {}
        }
    }

    pub(crate) fn site_done(&mut self, site: Site, ok: bool) {
        if !ok {
            return;
        }
        let d = &mut self.desk;
        match site {
            Site::Sheet("wallet-send") => {
                if let Some(w) = d.sh.wallet.as_mut() {
                    w.sheet = false;
                }
                d.wl_form = (0, String::new(), String::new());
                d.wl_again.clear();
            }
            Site::Sheet(crate::house::wallet::VAULT_SEND) => {
                d.vault_adjust = None;
                if let Some(w) = d.sh.wallet.as_mut() {
                    w.vault_set = None;
                }
            }
            Site::Sheet("settle") => d.settle_form = None,
            Site::Sheet("label") => d.order_label = None,
            Site::Sheet("bo-act") => {
                d.bo_sheet = None;
                d.bo_input.clear();
                d.bo_kit = None;
                d.bo_first.clear();
            }
            Site::Sheet("cands-add") => d.add_form = None,
            Site::Sheet("cands-rename") => d.rename = None,
            Site::Sheet("cands-remove") => d.confirm_remove = false,
            Site::Sheet("pl-payout") => d.pl_payout = None,
            _ => {}
        }
    }

}

pub(crate) fn origin_keeps(said: &mut Option<Said>) -> Option<Said> {
    said.clone()
}


pub(crate) fn sync_done(ok: bool, at: u64) -> Option<u64> {
    ok.then_some(at)
}

