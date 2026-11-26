use std::path::PathBuf;
use std::sync::Arc;

pub fn keep_reads_in(machine: Option<PathBuf>) {
    if eravon_chain::action::keeper().is_some() {
        return;
    }
    if let Some(m) = machine {
        eravon_chain::action::keep_in(Arc::new(eravon_send::checked::Logs(eravon_send::checked::Plain(m.clone()))));
        crate::judged::keep_in(Some(Arc::new(eravon_send::checked::Plain(m))));
    }
}

pub fn machine_if_there() -> Option<PathBuf> {
    eravon_os::machine::here().ok().map(|(m, _)| m).filter(|m| m.is_dir())
}
