use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::desk::Site;
use crate::house::shell::Seat;
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    let s = a.step;
    if !matches!(
        s,
        Step::PlTab
            | Step::PlOpen
            | Step::PlPick
            | Step::PlFile
            | Step::PlImport
            | Step::PlBump
            | Step::PlHistory
            | Step::PlPayout
            | Step::PlAttach
            | Step::PlPay
            | Step::PlForm
            | Step::PlCompanyOpen
    ) {
        return None;
    }
    if sh.desk.seat() != Seat::Buyer {
        return Some(Err("付款链接页只在买家席".to_string()));
    }
    if sh.desk.sh.paylink.is_none() {
        return Some(Err("先打开付款链接页".to_string()));
    }
    Some(match s {
        Step::PlOpen => open(sh, a),
        Step::PlPay => pay(sh, a),
        Step::PlCompanyOpen => company_open(sh, a),
        Step::PlImport => import(sh),
        Step::PlBump => bump(sh, a),
        Step::PlHistory => history(sh, a),
        _ => local(sh, a),
    })
}

fn local(sh: &mut Shell, a: &Act) -> Said {
    let pl = sh.desk.sh.paylink.as_mut().ok_or_else(|| "先打开付款链接页".to_string())?;
    let (say, feature) = match a.step {
        Step::PlTab => {
            let v = a.arg("v")?;
            pl.tab = crate::house::paylink::Tab::of(&v).ok_or_else(|| format!("认不出这一页签:{v}"))?;
            (v, crate::house::mark::FEATURE_P1)
        }
        Step::PlPick => {
            let n: usize = a.arg("n")?.parse().map_err(|_| "n 要是整数".to_string())?;
            let say = match pl.choose(n) {
                Ok(()) => n.to_string(),
                Err(e) => format!("{n} · 不收 · {e}"),
            };
            (say, crate::house::mark::FEATURE_P1)
        }
        Step::PlFile => {
            pl.take_path(&a.arg("path")?);
            let say = match &pl.import.said {
                Some(x) if x.tone == crate::house::seller::Tone::Bad => format!("{} · {}", x.name, x.detail),
                _ => pl.import.file.as_ref().map(|x| x.0.clone()).unwrap_or_default(),
            };
            (say, crate::house::mark::FEATURE_P1)
        }
        Step::PlPayout => {
            let typed = a.arg("a")?;
            pl.set_payout(&typed);
            let say = match &pl.payout_said {
                None => pl.payout.clone(),
                Some(s) => format!("不收 · {} · {}", s.name, s.detail),
            };
            sh.desk.pl_payout = pl.payout_said.is_some().then_some(typed);
            sh.desk.tr.push(Step::PlPayout, eravon_pay::mark::FEATURE, say.clone());
            return Ok(say);
        }
        Step::PlAttach => {
            let path = a.arg("path")?;
            let got = pl.attach(&path);
            let say = match got {
                Ok(seg) => {
                    sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Ok, head: crate::house::i18n::t(crate::house::i18n::Key::PlSegSaid), next: String::new(), detail: None });
                    seg.to_string()
                }
                Err(s) => {
                    let line = format!("{} · {}", s.name, s.detail);
                    sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: s.detail.clone(), next: String::new(), detail: Some(s.name.clone()) });
                    line
                }
            };
            sh.desk.tr.push(Step::PlAttach, eravon_pay::mark::FEATURE, say.clone());
            return Ok(say);
        }
        Step::PlForm => {
            let v = a.arg("v")?;
            pl.form = Some(eravon_pay::company::Form::of(&v).ok_or_else(|| format!("提案式只有 registered 与 direct,给的是 {v}"))?);
            (v, eravon_pay::mark::FEATURE)
        }
        _ => return Err(format!("{:?} 不是付款链接页的步", a.step)),
    };
    sh.desk.tr.push(a.step, feature, say.clone());
    Ok(say)
}

fn open(sh: &mut Shell, a: &Act) -> Said {
    if let (Some(l), Some(pl)) = (a.opt("link"), sh.desk.sh.paylink.as_mut()) {
        pl.link = l.to_string();
    }
    sh.spawn_paylink_open();
    sh.desk.tr.push(Step::PlOpen, crate::house::mark::FEATURE_P1, String::new());
    Ok("pl-open".into())
}

fn pay(sh: &mut Shell, a: &Act) -> Said {
    export_here(sh, a);
    sh.desk.pl_confirm = false;
    let company = sh.desk.sh.paylink.as_ref().is_some_and(|p| p.company_on);
    let site = Site::Sheet(if company { "pl-company" } else { "pl-pay" });
    let why = sh.write_why(if company { crate::house::shell::Write::Local } else { crate::house::shell::Write::Chain }).map(|w| crate::house::seller::Said::bad("E_APP_WRITE", w));
    let asked = sh.desk.sh.paylink.as_ref().map_or(Ok(()), |p| crate::house::asked(p.pay_plan())).and_then(|()| why.map_or(Ok(()), Err));
    if let Err(e) = asked {
        let line = format!("{} · {}", e.name, e.detail);
        sh.pl_said(site, e);
        return Ok(line);
    }
    let (registries, now) = (sh.desk.sh.seller.field.registries.clone(), sh.desk.now());
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    sh.desk.pay_cancel = Some(cancel.clone());
    sh.desk.retry_on(crate::task::Kind::PayLink, a.clone());
    let at = a.opt("lock").unwrap_or_default().to_string();
    sh.spawn_pl_act(site, move |pl, s, buyer, t| {
        let asked = std::cell::Cell::new(0u32);
        let stop = || {
            asked.set(asked.get() + 1);
            cancel.load(std::sync::atomic::Ordering::SeqCst) || !t.live() || matches!((at.as_str(), asked.get()), ("before", 1) | ("between", 2))
        };
        pl.pay(s, buyer, &registries, now, &stop);
    });
    sh.desk.tr.push(Step::PlPay, eravon_pay::mark::FEATURE, String::new());
    Ok(if company { "pl-company" } else { "pl-pay" }.into())
}

fn export_here(sh: &mut Shell, a: &Act) {
    let (data, home) = (sh.desk.data.clone(), sh.desk.sh.seller.field.home.clone());
    if let Some(pl) = sh.desk.sh.paylink.as_mut() {
        match a.opt("dir") {
            Some(d) => pl.export_dir = d.to_string(),
            None => crate::house::orders::set_export_dir(&mut pl.export_dir, &mut pl.export_why, &home, &data),
        }
    }
}

fn company_open(sh: &mut Shell, a: &Act) -> Said {
    let site = Site::Sheet("pl-co");
    export_here(sh, a);
    let why = sh.write_why(crate::house::shell::Write::Local).map(|w| crate::house::seller::Said::bad("E_APP_WRITE", w));
    let id = a.opt("id").unwrap_or_default().to_string();
    let exported = if id.is_empty() { sh.desk.sh.paylink.as_ref().map_or(Ok(()), |p| crate::house::asked(p.co_open_plan())) } else { Ok(()) };
    let asked = exported.and_then(|()| why.map_or(Ok(()), Err));
    if let Err(e) = asked {
        let line = format!("{} · {}", e.name, e.detail);
        sh.pl_said(site, e);
        return Ok(line);
    }
    let (registries, now) = (sh.desk.sh.seller.field.registries.clone(), sh.desk.now());
    sh.desk.retry_on(crate::task::Kind::PayLink, a.clone());
    sh.spawn_pl_act(site, move |pl, s, buyer, t| {
        if !t.live() {
            pl.co_open = Some(Err(crate::house::stale()));
            return;
        }
        pl.company_open(s, buyer, &registries, now, &id);
    });
    sh.desk.tr.push(Step::PlCompanyOpen, eravon_pay::mark::FEATURE, String::new());
    Ok("pl-co".into())
}

fn import(sh: &mut Shell) -> Said {
    let site = Site::Sheet("pl-import");
    let why = sh.write_why(crate::house::shell::Write::Vault).map(|w| crate::house::seller::Said::bad("E_APP_WRITE", w));
    let asked = sh.desk.sh.paylink.as_ref().map_or(Ok(()), |p| crate::house::asked(p.import_plan())).and_then(|()| why.map_or(Ok(()), Err));
    if let Err(e) = asked {
        let line = format!("{} · {}", e.name, e.detail);
        sh.pl_said(site, e);
        return Ok(line);
    }
    let now = sh.desk.now();
    let word = crate::house::i18n::t(crate::house::i18n::Key::PlImported);
    sh.desk.retry_on(crate::task::Kind::PayLink, Act::of(Step::PlImport));
    sh.spawn_pl_act(site, move |pl, s, me, t| {
        if let Err(e) = crate::house::asked(pl.import_plan()) {
            pl.import.said = Some(e);
            return;
        }
        if !t.live() {
            pl.import.said = Some(crate::house::stale());
            return;
        }
        pl.import_file(s, me, now, &word);
    });
    sh.desk.tr.push(Step::PlImport, eravon_import::mark::FEATURE, String::new());
    Ok("pl-import".into())
}

fn history(sh: &mut Shell, a: &Act) -> Said {
    let who = a.arg("a")?;
    if sh.desk.sh.paylink.is_none() {
        return Err("先打开付款链接页".to_string());
    }
    let buyer = crate::house::shell::current_key(&sh.desk.data, Seat::Buyer).unwrap_or_default();
    let said = crate::house::history::ask(sh, crate::house::history::Page::PayLink, &buyer, &who)?;
    if let Some(p) = sh.desk.sh.paylink.as_mut() {
        p.inner = Some(crate::house::paylink::PlInner::History(said.clone()));
    }
    Ok(said)
}

fn bump(sh: &mut Shell, a: &Act) -> Said {
    let n = a.opt("n").map(|v| v.parse::<usize>().map_err(|_| "n 要是整数".to_string())).transpose()?;
    let buyer = crate::house::shell::current_key(&sh.desk.data, Seat::Buyer).unwrap_or_default();
    let now = sh.desk.now();
    let session = &sh.desk.session;
    let pl = sh.desk.sh.paylink.as_mut().ok_or_else(|| "先打开付款链接页".to_string())?;
    pl.bump(session, &buyer, n, now);
    let say = match &pl.key_for {
        Some(Ok(p)) => format!("{} · openBy {} · {}", if p.bumped { "重签" } else { "没撞" }, p.open_by, p.instance),
        Some(Err(s)) => format!("{} · {}", s.name, s.detail),
        None => String::new(),
    };
    sh.desk.tr.push(Step::PlBump, crate::house::mark::FEATURE_P1, say.clone());
    Ok(say)
}
