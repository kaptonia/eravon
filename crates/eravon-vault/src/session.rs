use crate::vfault::{Fault, R};
use crate::vault::{self, Vault};

pub struct Session {
    data: String,
    open: bool,
    owner: bool,
    keep: Option<Keep>,
}

static HOLDERS: std::sync::Mutex<usize> = std::sync::Mutex::new(0);

pub struct Keep(());

impl Keep {
    pub fn new() -> Keep {
        *HOLDERS.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        Keep(())
    }
}

impl Default for Keep {
    fn default() -> Keep {
        Keep::new()
    }
}

impl Drop for Keep {
    fn drop(&mut self) {
        let mut n = HOLDERS.lock().unwrap_or_else(|e| e.into_inner());
        *n = n.saturating_sub(1);
        if *n == 0 {
            vault::lock_now();
        }
    }
}

fn silent(_s: &mut Session, e: Fault) -> R<()> {
    Err(e)
}


impl Session {
    pub fn locked(data: &str) -> Session {
        Session { data: data.to_string(), open: false, owner: true, keep: None }
    }

    pub fn unlock(&mut self, passcode: &str) -> R<()> {
        let keep = Keep::new();
        self.release();
        if let Err(e) = vault::unlock(&self.data, passcode).and_then(|()| vault::read(&self.data).map(|_| ())) {
            return silent(self, e);
        }
        self.open = true;
        self.owner = true;
        self.keep = Some(keep);
        took(&self.data);
        Ok(())
    }

    pub fn opened(&mut self) -> R<()> {
        let keep = Keep::new();
        self.release();
        if let Err(e) = vault::read(&self.data) {
            return silent(self, e);
        }
        self.open = true;
        self.owner = true;
        self.keep = Some(keep);
        took(&self.data);
        Ok(())
    }

    pub fn borrowed(data: &str) -> R<Session> {
        crate::keybox::local_key().map_err(Fault::from)?;
        Ok(Session { data: data.to_string(), open: true, owner: false, keep: None })
    }

    pub fn lock(&mut self) {
        if self.open && self.owner {
            vault::lock_now();
        }
        self.release();
    }

    fn release(&mut self) {
        self.open = false;
        self.keep = None;
    }


    pub fn fork(&self) -> R<Session> {
        need_open(self)?;
        Ok(Session { data: self.data.clone(), open: true, owner: false, keep: None })
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn data(&self) -> &str {
        &self.data
    }

    pub fn key_for(&self, address: &str) -> R<[u8; 32]> {
        need_open(self)?;
        vault::open_key(&self.data, address)
    }

    pub fn own_key(&self, me: &str, buyer: &str) -> R<[u8; 32]> {
        if !buyer.eq_ignore_ascii_case(me) {
            return Err(crate::vfault::Fault::NoIdentity(format!("订单的买家钥 {buyer} 不是本席的身份钥 {me}")));
        }
        self.key_for(me)
    }

    pub fn phrase_for(&self, address: &str) -> R<String> {
        need_open(self)?;
        vault::open_phrase(&self.data, address)
    }

    pub fn vault(&self) -> R<Vault> {
        need_open(self)?;
        vault::read(&self.data)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.release();
    }
}

fn took(data: &str) {
    crate::tokens::adopt(data);
    if let Ok(g) = crate::store::read_global(data) {
        eravon_literals::constants::set_past_switch(g.past_cases);
    }
}

pub fn need_open(s: &Session) -> R<()> {
    if s.is_open() { Ok(()) } else { Err(Fault::Locked) }
}

pub fn take_turn() {
    crate::glue::take_turn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_starts_locked_and_refuses_by_name() {
        let s = Session::locked("/nowhere");
        assert!(!s.is_open());
        assert_eq!(need_open(&s).unwrap_err().name(), "E_VAULT_LOCKED");
        assert_eq!(need_open(&s).unwrap_err().detail(), "已锁定");
        assert_eq!(s.key_for("0x00").unwrap_err().name(), "E_VAULT_LOCKED");
    }

    #[test]
    fn only_the_seats_own_key_signs_its_orders() {
        let s = Session::locked("/nowhere");
        let me = "0x00000000000000000000000000000000000000b1";
        let other = "0x00000000000000000000000000000000000000b2";
        assert_eq!(s.own_key(me, other).unwrap_err().name(), "E_VAULT_NO_IDENTITY");
        assert_eq!(s.own_key(me, &me.to_ascii_uppercase().replacen("0X", "0x", 1)).unwrap_err().name(), "E_VAULT_LOCKED");
        assert_eq!(s.own_key(me, me).unwrap_err().name(), "E_VAULT_LOCKED");
    }

    #[test]
    fn the_last_holder_going_out_of_scope_wipes_the_master_key() {
        let data = crate::testing::fresh();
        let mut b = vault::create("48261537", eravon_store::vault::Seat::Seller, "", None, 900).expect("a vault");
        b.write(&data).expect("written");
        let mut s = Session::locked(&data);
        s.unlock("48261537").expect("opens");
        let f = s.fork().expect("a secondary session");
        drop(f);
        assert!(crate::keybox::local_key().is_ok(), "a secondary session let go leaves the vault open");
        drop(s);
        assert!(crate::keybox::local_key().is_err(), "the last holder let go wipes it");
    }
}
