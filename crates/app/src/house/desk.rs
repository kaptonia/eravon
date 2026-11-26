use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::house::shell::Seat;
use crate::task::Kind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Duty {
    Backstop,
    Sweep,
}

impl Duty {
    pub fn step(self) -> crate::house::action::Action {
        match self {
            Duty::Backstop => crate::house::action::Action::OrderScan,
            Duty::Sweep => crate::house::action::Action::Unlock,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site {
    Page,
    Sheet(&'static str),
    Ask,
}

impl Site {
    pub fn name(self) -> &'static str {
        match self {
            Site::Page => "page",
            Site::Sheet(s) => s,
            Site::Ask => "ask",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flying {
    pub site: Site,
    pub act: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remind {
    pub owner: String,
    pub key: String,
    pub say: String,
}

pub struct Desk {
    pub door_held: std::collections::VecDeque<crate::house::approve::Held>,
    pub door_cleared: bool,
    pub door_told: Vec<crate::house::approve::Told>,
    pub door_lines: std::collections::VecDeque<crate::house::shell::Toast>,
    pub sh: crate::house::shell::Shell,
    pub clock: Option<u64>,
    pub tr: crate::house::steps::Trace,
    pub data: String,
    pub addr: String,
    pub addr_seat: Option<Seat>,
    pub session: eravon_vault::session::Session,
    pub first: Option<crate::house::first::First>,
    pub ready: crate::house::first::Ready,
    pub wizard_due: bool,
    pub wizard_up: Option<&'static str>,
    pub owed: Vec<Duty>,
    pub pending_export: Option<String>,
    pub wl_form: (usize, String, String),
    pub wl_again: String,
    pub wl_again_focus: bool,
    pub wl_all_fill: bool,
    pub open_cells: Option<(String, String, String)>,
    pub vault_fund: Option<String>,
    pub vault_adjust: Option<(String, String)>,
    pub wl_receive: bool,
    pub bo_kit: Option<String>,
    pub bo_first: String,
    pub bo_note: String,
    pub wiz_role: Option<Seat>,
    pub wiz_chain: Option<u64>,
    pub wiz_chain_done: bool,
    pub new_chain: Option<u64>,
    pub pending_url: VecDeque<String>,
    pub pay_cancel: Option<Arc<AtomicBool>>,
    pub remind: Vec<Remind>,
    pub queue_query: String,
    pub order_label: Option<String>,
    pub complaint: Option<crate::house::complaint::Form>,
    pub add_form: Option<(eravon_cands::table::Side, String, String)>,
    pub info: Option<(&'static str, Vec<(String, String, bool)>)>,
    pub goods_bar: Option<bool>,
    pub goods_publish: Option<bool>,
    pub goods_import: Option<&'static str>,
    pub settle_form: Option<(String, Option<usize>, String)>,
    pub pl_payout: Option<String>,
    pub pay_payout: Option<(String, String)>,
    pub bo_input: String,
    pub bo_sheet: Option<&'static str>,
    pub rename: Option<String>,
    pub confirm_remove: bool,
    pub goods_after_leave: Option<crate::house::seller::Leave>,
    pub goods_search: Option<String>,
    pub set_card: Option<crate::house::cards::SetCard>,
    pub consent: Option<Consent>,
    pub rows_gen: u64,
    pub refresh_cause: Option<(crate::house::refresh::Cause, Option<String>)>,
    pub probes: Vec<Arc<eravon_chain::recall::Recall>>,
    pub missed_at: Option<u64>,
    pub refresh_again: Option<(crate::house::refresh::Cause, Option<String>)>,
    pub reshow_again: bool,
    pub reshow_open: bool,
    pub act_open: Option<(String, u64, String)>,
    pub pl_confirm: bool,
    pub goods_confirm: Option<&'static str>,
    pub sync_done_at: Option<u64>,
    pub close_asked: bool,
    pub goto: Option<Option<crate::house::seller::Page>>,
    pub goto_landed: Option<crate::house::seller::Page>,
    pub role_landed: Option<String>,
    pub goto_section: Option<crate::nav::Section>,
    pub flying: Vec<(Kind, Flying)>,
    pub retry: Vec<(Kind, crate::house::act::Act)>,
    pub retry_next: Option<(Kind, crate::house::act::Act)>,
    pub not_started: Option<String>,
}

impl Default for Desk {
    fn default() -> Self {
        let mut desk = Desk {
            door_held: std::collections::VecDeque::new(),
            door_cleared: false,
            door_told: Vec::new(),
            door_lines: std::collections::VecDeque::new(),
            sh: crate::house::shell::Shell::default(),
            tr: crate::house::steps::Trace::default(),
            data: String::new(),
            addr: String::new(),
            addr_seat: None,
            session: eravon_vault::session::Session::locked(""),
            first: None,
            ready: crate::house::first::Ready::default(),
            wizard_due: false,
            wizard_up: None,
            owed: Vec::new(),
            pending_export: None,
            wl_form: (0, String::new(), String::new()),
            wl_again: String::new(),
            wl_again_focus: false,
            wl_all_fill: false,
            open_cells: None,
            vault_fund: None,
            vault_adjust: None,
            wl_receive: false,
            bo_kit: None,
            bo_first: String::new(),
            bo_note: String::new(),
            wiz_role: None,
            wiz_chain: None,
            wiz_chain_done: false,
            new_chain: None,
            pending_url: VecDeque::new(),
            pay_cancel: None,
            remind: Vec::new(),
            queue_query: String::new(),
            order_label: None,
            complaint: None,
            add_form: None,
            info: None,
            goods_bar: None,
            goods_publish: None,
            goods_import: None,
            settle_form: None,
            pl_payout: None,
            pay_payout: None,
            bo_input: String::new(),
            bo_sheet: None,
            rename: None,
            confirm_remove: false,
            goods_after_leave: None,
            goods_search: None,
            set_card: None,
            consent: None,
            rows_gen: 0,
            refresh_cause: None,
            probes: Vec::new(),
            missed_at: None,
            refresh_again: None,
            reshow_again: false,
            reshow_open: false,
            act_open: None,
            pl_confirm: false,
            goods_confirm: None,
            sync_done_at: None,
            clock: None,
            close_asked: false,
            goto: None,
            goto_landed: None,
            role_landed: None,
            goto_section: None,
            flying: Vec::new(),
            retry: Vec::new(),
            retry_next: None,
            not_started: None,
        };
        desk.sh.seller.field.home = crate::house::seam::user_home("");
        desk
    }
}

impl Desk {
    pub fn seat(&self) -> Seat {
        self.sh.seat()
    }

    pub fn flying(&self, k: Kind) -> Option<Flying> {
        self.flying.iter().find(|(x, _)| *x == k).map(|(_, f)| *f)
    }

    pub fn flush(&mut self) {
        if !self.data.is_empty() {
            let _ = self.tr.save(&self.data);
        }
    }

    pub fn shown(&mut self) -> Arc<eravon_chain::recall::Recall> {
        let r = crate::house::refresh::shown_on(&self.sh, &self.addr);
        self.probes.push(r.clone());
        if self.probes.len() > 16 {
            self.probes.remove(0);
        }
        r
    }

    pub fn pace_now(&self) -> u64 {
        self.clock.unwrap_or_else(wall_now)
    }

    pub fn now(&self) -> u64 {
        if let Some(c) = self.clock {
            return c;
        }
        let wall = wall_now();
        self.sh.book.as_ref().filter(|b| b.key.eq_ignore_ascii_case(&self.addr)).and_then(|b| b.chain_now(wall)).unwrap_or(wall)
    }

    pub fn seat_reads(&self) -> bool {
        !self.addr.is_empty()
    }


    pub fn slot_toast(&mut self, slot: &'static str, read: bool, t: crate::house::shell::Toast) {
        if read {
            self.sh.toast_read(slot, t);
        } else {
            self.sh.toast(t);
        }
    }

    pub fn slot_ok(&mut self, slot: &'static str, read: bool) {
        if read {
            self.sh.read_ok(slot);
        }
    }

    pub fn busy_said(&mut self) {
        self.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: crate::house::i18n::t(crate::house::i18n::Key::ErrBusy), next: String::new(), detail: None });
    }

    pub fn retry_on(&mut self, kind: Kind, a: crate::house::act::Act) {
        self.retry_next = Some((kind, a));
    }

    pub fn launched(&mut self, kind: Kind) {
        self.retry.retain(|(k, _)| *k != kind);
        if let Some((k, a)) = self.retry_next.take() {
            if k == kind {
                self.retry.push((k, a));
            }
        }
    }

    pub fn queue_remind(&mut self, owner: &str, system: &[eravon_home::notes::Note]) {
        for n in system {
            if !self.remind.iter().any(|r| r.key == n.key) {
                self.remind.push(Remind { owner: owner.to_string(), key: n.key.clone(), say: crate::house::i18n::library(crate::lang::lang(), &n.say) });
            }
        }
    }
}

pub fn wall_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Default)]
pub struct Consent {
    pub identity: String,
    pub role: Option<usize>,
    pub pin: crate::secret::Secret,
    pub text: Option<String>,
    pub trouble: Option<crate::fault::Fault>,
}
