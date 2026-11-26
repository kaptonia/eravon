pub mod buyerpage;
pub mod cands;
pub mod candsview;
pub mod company;
pub mod complaint;
pub mod direct;
pub mod export;
pub mod home;
pub mod network;
pub mod orderpage;
pub mod payways;
pub mod paypage;
pub mod receipts;
pub mod reset;
pub mod said;
pub mod seal;
pub mod settle;
pub mod tokens;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct No {
    pub name: String,
    pub facts: Vec<String>,
}

impl No {
    pub fn new(name: &str, facts: Vec<String>) -> No {
        No { name: name.to_string(), facts }
    }

    pub fn store(e: eravon_store::fault::Fault) -> No {
        No { name: e.name().to_string(), facts: vec![e.detail()] }
    }

    pub fn detail(&self) -> String {
        self.facts.join(" · ")
    }

    pub fn fault(&self) -> crate::fault::Fault {
        crate::fault::Fault::Down { layer: "desk", token: self.name.clone(), why: self.detail() }
    }
}

impl From<No> for crate::fault::Fault {
    fn from(n: No) -> crate::fault::Fault {
        n.fault()
    }
}

pub fn write_identity(data: &str, writer: &str, now: u64, me: &str, f: impl FnOnce(&mut eravon_store::settings::Identity)) -> Result<(), No> {
    let s = eravon_vault::store::open_write(data, writer, now).map_err(No::store)?;
    let mut t = eravon_vault::store::identity_to_change(s.data(), me).map_err(No::store)?;
    f(&mut t);
    eravon_vault::store::write_identity(s.data(), me, &t).map_err(No::store)
}

pub fn write_global(data: &str, writer: &str, now: u64, f: impl FnOnce(&mut eravon_store::settings::Global)) -> Result<(), No> {
    let s = eravon_vault::store::open_write(data, writer, now).map_err(No::store)?;
    let mut g = eravon_vault::store::read_global(s.data()).map_err(No::store)?;
    let was = g.clone();
    f(&mut g);
    if g == was {
        return Ok(());
    }
    eravon_vault::store::write_global(s.data(), &g).map_err(No::store)
}
