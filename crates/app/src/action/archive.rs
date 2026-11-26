use super::*;

pub(super) fn set_log_asks(shell: &mut Shell, n: u64) -> Result<u64, crate::fault::Fault> {
    if !eravon_chain::action::asks_within(n) {
        return Err(crate::fault::Fault::known(crate::fault::Known::SettingsShape, crate::lang::both_of(|| crate::lang::t(crate::lang::Key::LogAsksRange).to_string())));
    }
    shell.commit_settings(|s| s.log_asks = Some(n))?;
    eravon_chain::action::take_asks_cap(n);
    Ok(n)
}

pub(super) fn set_endpoints(shell: &mut Shell, specs: &str) -> Result<usize, crate::fault::Fault> {
    let specs: Vec<String> = crate::chainx::typed_all(specs)?;
    let n = specs.len();
    shell.commit_settings(|s| {
        s.endpoints = specs;
        s.network = None;
    })?;
    Ok(n)
}
