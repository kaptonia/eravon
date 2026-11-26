use super::No;

fn vault(e: eravon_vault::fault::Fault) -> No {
    No::new(e.which().map_or_else(|| e.class().as_str(), |k| k.as_str()), vec![e.tail().to_string()])
}

pub fn nodes_of(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == ',').map(str::trim).filter(|x| !x.is_empty()).map(str::to_string).collect()
}


pub fn endpoint_specs(chain: u64, list: &[String]) -> Result<String, No> {
    let list: Vec<String> = list.iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
    if !list.is_empty() {
        eravon_chain::endpoints::Table::new(list.clone()).map_err(|e| No::new(e.name(), vec![e.detail()]))?;
    }
    Ok(eravon_vault::store::specs_on(chain, &list).join(" "))
}

pub fn endpoints(data: &str, me: &str, chain: Option<u64>, typed: &str) -> Result<usize, No> {
    let home = eravon_vault::store::home_settings(data, me).map_err(No::store)?.unwrap_or_default();
    let on = eravon_vault::store::nodes_chain(&home, chain).map_err(No::store)?;
    let specs = eravon_vault::chainx::typed_all(&endpoint_specs(on, &nodes_of(typed))?).map_err(vault)?;
    let n = specs.len();
    eravon_vault::store::change_home(data, me, |s| {
        s.endpoints = eravon_vault::store::typed_on(&s.endpoints, on, specs);
        s.network = None;
    })
    .map_err(No::store)?;
    Ok(n)
}

pub fn zone(data: &str, me: &str, typed: &str) -> Result<eravon_vault::when::Zone, No> {
    let z = eravon_vault::when::Zone::parse(typed.trim()).ok_or_else(|| No::new("E_V_ARG", vec![typed.to_string()]))?;
    eravon_vault::store::change_home(data, me, |s| s.zone = Some(z)).map_err(No::store)?;
    Ok(z)
}

pub fn proxy(choice: &str) -> Result<String, No> {
    let saved = eravon_vault::machine::read().map_err(vault)?.proxy;
    let written = eravon_vault::machine::proxy::choose(choice, saved.as_deref()).map_err(vault)?;
    let m = eravon_vault::machine::update(|m| m.proxy = Some(written)).map_err(vault)?;
    Ok(eravon_vault::machine::proxy::shown(&m.proxy.unwrap_or_default()))
}

pub fn cli_path(on: bool) -> Result<eravon_os::cli_path::State, No> {
    let cli = eravon_os::cli_path::beside_this_program().ok_or_else(|| vault(eravon_vault::fault::Fault::known(eravon_vault::fault::Known::CliPathUnsupported, String::new())))?;
    eravon_vault::machine::cli_path(&cli, on).map_err(vault)
}
