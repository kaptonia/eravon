#![forbid(unsafe_code)]

pub mod accounts;
mod forms;
pub mod add;
pub mod backup;
pub mod cryptx;
pub mod vfault;
pub mod derive;
pub mod glue;
pub mod kdf;
pub mod keybox;
pub mod keyfile;
pub mod keystore;
pub mod keyset;
pub mod session;
pub use eravon_store::lock;
pub mod onboard;
pub mod payout;
pub mod restore;
pub mod rules;
pub mod seam;
pub mod vault;
pub mod words;

pub mod chainx;
pub mod deploy;
pub mod family;
pub mod fault;
pub mod feature;
pub mod home;
pub mod identity;
pub mod landwire;
pub mod lang;
pub mod local;
pub mod machine;
pub mod names;
pub mod own;
pub mod places;
pub mod reading;
pub mod reads;
pub mod records;
pub mod store;
pub mod tokens;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod sign;
pub use sign::key;
pub mod register;
pub mod rekey;
pub mod roles;
pub mod settings;
pub mod trace;
pub mod when;
pub(crate) use glue::{platform, task};
pub mod secret {
    pub use eravon_os::secret::*;
}

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::H2;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}
