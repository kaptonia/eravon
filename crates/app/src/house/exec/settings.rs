use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::settings::Section;
use crate::shell::Shell;

const E_NETWORK_UNKNOWN: &str = eravon_cli::deskops::network::NETWORK_UNKNOWN;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    let s = a.step;
    if !matches!(
        s,
        Step::SetSection
            | Step::SetTokenAdd
            | Step::SetToken
            | Step::SetTokenClose
            | Step::SetTokenRemove
            | Step::SetTokenDecimals
            | Step::SetZkLink
            | Step::SetCompany
            | Step::SetTimeCheck
            | Step::SetNetwork
            | Step::SetPastCases
            | Step::SetRotate
            | Step::SetRotateGo
            | Step::SetCompanyCheck
            | Step::SetEndpoints
            | Step::SetGas
            | Step::SetZikaron
            | Step::SetResetup
    ) {
        return None;
    }
    if sh.desk.sh.seller.settings.is_none() {
        sh.open_settings();
    }
    Some(match s {
        Step::SetResetup => wizard(sh, s),
        _ => act(sh, a),
    })
}

fn wizard(sh: &mut Shell, s: Step) -> Said {
    sh.reread_ready();
    if let Some((name, detail)) = sh.desk.ready.unread.clone() {
        return Err(crate::house::refusal::said(crate::lang::lang(), &name, &detail));
    }
    sh.desk.wizard_due = true;
    sh.desk.tr.push(s, eravon_literals::trace::Feature::S9, String::new());
    Ok("向导".into())
}

fn on_off(a: &Act, k: &str) -> Result<bool, String> {
    match a.arg(k)?.as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!("开关只收 on 或 off:{other}")),
    }
}

fn act(sh: &mut Shell, a: &Act) -> Said {
    let (data, now) = (sh.desk.data.clone(), sh.desk.now());
    let env = crate::house::settings::Env { home: sh.desk.sh.seller.field.home.clone() };
    let w = |k: crate::house::i18n::Key| crate::house::i18n::t(k);
    let step = a.step;
    match step {
        Step::SetGas => {
            sh.spawn_settings(step, crate::house::flow::Ask::Asked, |s, d| s.read_gas(d));
            return Ok(step.name().to_string());
        }
        Step::SetCompanyCheck => {
            sh.spawn_settings(step, crate::house::flow::Ask::Asked, |s, d| s.company_check(d));
            return Ok(step.name().to_string());
        }
        Step::SetCompany if on_off(a, "on")? => {
            let addr = a.opt("a").unwrap_or_default().to_string();
            let me = sh.desk.sh.seller.settings.as_ref().map(|s| s.me.clone()).unwrap_or_default();
            if let Err(e) = crate::house::asked(crate::house::settings::company_plan(&data, &me, &addr)) {
                return Ok(settings_said(sh, e));
            }
            let word = w(crate::house::i18n::Key::CompanyOnSaid);
            sh.spawn_settings(step, true, move |s, d| s.set_company(d, &env, true, &addr, now, &word));
            return Ok(step.name().to_string());
        }
        Step::SetTokenAdd => {
            let (ad, l, d) = (a.arg("a")?, a.opt("label").unwrap_or_default().to_string(), a.opt("d").unwrap_or_default().to_string());
            sh.spawn_settings(step, true, move |s, data| s.token_add(data, &ad, &l, &d, now));
            return Ok(step.name().to_string());
        }
        Step::SetSection => {
            let name = a.arg("s")?;
            let to = if name == "home" { None } else { Some(Section::of(&name).ok_or_else(|| format!("没有这一节:{name}"))?) };
            let x = sh.desk.sh.seller.settings.as_mut().ok_or_else(|| "先打开设置那一页".to_string())?;
            let moved = x.go_section(to);
            sh.desk.tr.push(step, eravon_literals::trace::Feature::S9, name.clone());
            if moved && to == Some(Section::Tokens) && x.tokens.iter().any(|r| r.source == eravon_vault::tokens::FROM_PENDING) {
                sh.spawn_settings(step, false, move |s, d| s.fill_pending(d, now));
            }
            sh.desk.flush();
            return Ok(name);
        }
        _ => {}
    }
    if step == Step::SetRotate {
        sh.desk.set_card = Some(crate::house::cards::SetCard::Rotate { keep: false, let_expire: Vec::new(), revoke: Vec::new() });
        if let Some(x) = sh.desk.sh.seller.settings.as_mut() {
            x.rotate = None;
        }
        sh.rotate_list_due();
        return Ok(step.name().to_string());
    }
    if step == Step::SetRotateGo {
        let keep = on_off(a, "v")?;
        let ids = |k: &str| a.opt(k).map(|v| v.split(',').map(str::trim).filter(|x| !x.is_empty()).map(str::to_string).collect::<Vec<_>>()).unwrap_or_default();
        let (let_expire, revoke) = (ids("letExpire"), ids("revoke"));
        let leave = a.opt("leave").is_some_and(|v| v == "on");
        sh.spawn_settings_keyed(step, move |s, ses, data| s.rotate_go(data, ses, keep, &let_expire, &revoke, leave, now));
        return Ok(step.name().to_string());
    }
    if step == Step::SetNetwork {
        let got = choose_network(sh, &a.arg("chain")?).map(|_| ());
        let x = sh.desk.sh.seller.settings.as_mut().ok_or_else(|| "先打开设置那一页".to_string())?;
        x.endpoints_set(&data, &env, got, &w(crate::house::i18n::Key::SettingSaved));
        x.read_tokens(&data);
        let said = x.said.as_ref().map_or(String::new(), |s| format!("{} · {}", s.name, s.detail));
        set_after(sh, step);
        return Ok(format!("{} · {said}", step.name()));
    }
    if step == Step::SetEndpoints {
        let on = eravon_vault::store::chain_of(&sh.settings);
        let got = crate::house::settings::Settings::endpoint_specs(on, &crate::house::settings::nodes_of(a.opt("v").unwrap_or_default()))
            .and_then(|typed| crate::chainx::typed_all(&typed).map_err(|f| crate::house::said_fault(&f)))
            .and_then(|typed| {
                let specs = eravon_vault::store::typed_on(&sh.settings.endpoints, on, typed).join(" ");
                match crate::action::apply_settled(sh, crate::action::Action::SetEndpoints { specs }) {
                    crate::action::Applied::Trouble(f) => Err(crate::house::said_fault(&f)),
                    _ => Ok(()),
                }
            });
        let x = sh.desk.sh.seller.settings.as_mut().ok_or_else(|| "先打开设置那一页".to_string())?;
        let n = crate::house::settings::nodes_of(a.opt("v").unwrap_or_default()).len();
        x.endpoints_set(&data, &env, got, &crate::fill!(crate::house::i18n::Key::EndpointsSaid; n.to_string()));
        let said = x.said.as_ref().map_or(String::new(), |s| format!("{} · {}", s.name, s.detail));
        set_after(sh, step);
        return Ok(format!("{} · {said}", step.name()));
    }
    let x = sh.desk.sh.seller.settings.as_mut().ok_or_else(|| "先打开设置那一页".to_string())?;
    match step {
        Step::SetToken => x.token_open(&data, &a.arg("a")?),
        Step::SetTokenClose => x.token_close(),
        Step::SetTokenRemove => x.token_remove(&data, &a.arg("a")?, now),
        Step::SetTokenDecimals => x.token_decimals(&data, &a.arg("a")?, &a.arg("d")?, now),
        Step::SetZkLink => x.set_zk_link(&data, &env, on_off(a, "v")?, now, &w(crate::house::i18n::Key::SettingSaved)),
        Step::SetCompany => x.set_company(&data, &env, false, "", now, &w(crate::house::i18n::Key::CompanyOffSaid)),
        Step::SetTimeCheck => x.set_time_check(&data, &env, on_off(a, "v")?, now, &w(crate::house::i18n::Key::SettingSaved)),
        Step::SetPastCases => x.set_past_cases(&data, &env, on_off(a, "v")?, now, &w(crate::house::i18n::Key::SettingSaved)),
        _ => x.set_zikaron(&data, &env, a.opt("dir").unwrap_or_default(), now, &w(crate::house::i18n::Key::ZkDirSaid)),
    }
    let said = x.said.as_ref().map_or(String::new(), |s| format!("{} · {}", s.name, s.detail));
    set_after(sh, step);
    Ok(format!("{} · {said}", step.name()))
}

pub(crate) fn choose_network(sh: &mut Shell, chain: &str) -> Result<u64, crate::house::seller::Said> {
    sh.settings_written();
    let mut failed: Option<crate::fault::Fault> = None;
    eravon_cli::deskops::network::choose_by(chain, |c| {
        if sh.settings.chain_id == Some(c) {
            return Ok(());
        }
        sh.commit_settings(|s| s.chain_id = Some(c)).map_err(|f| {
            let n = eravon_cli::deskops::No::new(f.which().map_or(f.class().as_str(), |k| k.as_str()), vec![f.tail().to_string()]);
            failed = Some(f);
            n
        })
    })
    .map(|n| {
        crate::chainx::forget_serving();
        n.standard.chain_id
    })
    .map_err(|n| match failed.take() {
        Some(f) => crate::house::said_fault(&f),
        None => crate::house::seller::Said::bad(E_NETWORK_UNKNOWN, format!("链 {} 不是已知的网", n.detail())),
    })
}

fn settings_said(sh: &mut Shell, e: crate::house::seller::Said) -> String {
    let line = format!("{} · {}", e.name, e.detail);
    sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: crate::house::refusal::said(crate::lang::lang(), &e.name, &e.detail), next: String::new(), detail: Some(e.name.clone()) });
    if let Some(s) = sh.desk.sh.seller.settings.as_mut() {
        s.said = Some(e);
    }
    line
}

fn set_after(sh: &mut Shell, step: Step) {
    let writes = !matches!(step, Step::SetToken | Step::SetTokenClose);
    if writes && crate::house::settings::took(sh.desk.sh.seller.settings.as_ref().and_then(|s| s.said.as_ref())) {
        sh.moved();
    }
    if matches!(step, Step::SetEndpoints | Step::SetNetwork | Step::SetPastCases) {
        sh.endpoints_landed();
    }
    let said = sh.desk.sh.seller.settings.as_ref().and_then(|s| s.said.clone());
    if let Some(x) = said {
        sh.desk.sh.toast(crate::house::shell::Toast { tone: x.tone, head: x.detail.clone(), next: String::new(), detail: if x.tone == crate::house::seller::Tone::Bad { Some(x.name.clone()) } else { None } });
        if x.tone == crate::house::seller::Tone::Ok {
            sh.desk.set_card = None;
        }
    }
    sh.refresh_addr();
    sh.desk.tr.push(step, eravon_literals::trace::Feature::S9, String::new());
    sh.desk.flush();
}

