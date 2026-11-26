pub fn take_turn() {
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    thread_local! {
        static HELD: std::cell::RefCell<Option<std::sync::MutexGuard<'static, ()>>> = const { std::cell::RefCell::new(None) };
    }
    HELD.with(|h| {
        if h.borrow().is_none() {
            let g = TURN.lock().unwrap_or_else(|e| e.into_inner());
            *h.borrow_mut() = Some(g);
        }
    });
}

pub mod platform {
    pub use eravon_os::Zone;

    pub fn home_dir() -> Option<std::path::PathBuf> {
        eravon_os::user_home()
    }

    pub fn app_data_dir(user_home: &std::path::Path) -> std::path::PathBuf {
        eravon_os::app_data_dir(user_home)
    }

    pub fn zone_rules() -> Option<Zone> {
        eravon_os::zone_rules()
    }
}

pub mod task {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    thread_local! {
        static TICKET: std::cell::RefCell<Option<(u64, Arc<AtomicU64>)>> = const { std::cell::RefCell::new(None) };
    }

    pub fn set_ticket(t: Option<(u64, Arc<AtomicU64>)>) {
        TICKET.with(|x| *x.borrow_mut() = t);
    }

    pub fn ticket_void() -> bool {
        TICKET.with(|t| t.borrow().as_ref().map(|(at, now)| *at != now.load(Ordering::SeqCst)).unwrap_or(false))
    }

    pub fn void_ticket_fault(at: &std::path::Path) -> crate::fault::Fault {
        crate::fault::Fault::known(crate::fault::Known::HomeUnreachable, crate::lang::both_of(|| crate::lang::filln(crate::lang::Key::TailTicketVoid, &[&at.display().to_string()])))
    }
}
