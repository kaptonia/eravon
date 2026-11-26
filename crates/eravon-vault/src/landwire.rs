#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trouble {
    Io(String, String),
    Occupied(String),
}

impl From<eravon_store::landing::Trouble> for Trouble {
    fn from(t: eravon_store::landing::Trouble) -> Trouble {
        match t {
            eravon_store::landing::Trouble::Occupied(p) => Trouble::Occupied(p),
            eravon_store::landing::Trouble::Io(p, w) => Trouble::Io(p, w),
        }
    }
}

impl Trouble {
    pub fn code(&self) -> &'static str {
        match self {
            Trouble::Io(..) => "E_IO",
            Trouble::Occupied(_) => "E_OCCUPIED",
        }
    }

    pub fn said(&self) -> Option<&str> {
        match self {
            Trouble::Io(_, w) => Some(w.as_str()),
            Trouble::Occupied(_) => None,
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Trouble::Io(p, _) | Trouble::Occupied(p) => Some(p.as_str()),
        }
    }

    pub fn subject(&self) -> String {
        match self {
            Trouble::Io(p, _) | Trouble::Occupied(p) => p.clone(),
        }
    }
}

impl From<eravon_store::fault::Fault> for crate::fault::Fault {
    fn from(f: eravon_store::fault::Fault) -> crate::fault::Fault {
        use crate::fault::{Fault, Known};
        use eravon_store::fault::Fault as S;
        match f {
            S::Machine(d) => Fault::known(Known::MachineShape, d),
            S::Form(d) => Fault::known(Known::SettingsShape, d),
            S::Missing(d) => Fault::known(Known::FileMissing, d),
            other => Fault::unknown(format!("{}: {}", other.name(), other.detail())),
        }
    }
}
