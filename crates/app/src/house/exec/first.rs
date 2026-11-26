use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::first::First;
use crate::house::landed::Landed;
use crate::house::shell::Seat;
use crate::shell::Shell;
use crate::task::Kind;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    let s = a.step;
    if !matches!(s, Step::Network | Step::Role | Step::Payout | Step::Skip) {
        return None;
    }
    let data = sh.desk.data.clone();
    if sh.desk.first.is_none() {
        sh.desk.first = Some(First::new(&data));
    }
    Some(match s {
        Step::Network => network(sh, a),
        Step::Role => role(sh, a),
        Step::Payout => payout(sh, a),
        _ => skip(sh, a),
    })
}

fn skip(sh: &mut Shell, a: &Act) -> Said {
    let f = sh.desk.first.as_mut().ok_or_else(|| "向导没开".to_string())?;
    f.said = None;
    let say = "跳过".to_string();
    sh.desk.tr.push(a.step, crate::house::mark::FEATURE, say.clone());
    Ok(say)
}

fn network(sh: &mut Shell, a: &Act) -> Said {
    let chain = match a.opt("chain") {
        Some(c) => c.to_string(),
        None => crate::house::first::chain_on(sh.desk.wiz_chain, sh.home.as_ref().map(|_| &sh.settings)).to_string(),
    };
    let c = match super::settings::choose_network(sh, &chain) {
        Ok(c) => c,
        Err(said) => {
            let line = format!("{} · {}", said.name, said.detail);
            if let Some(f) = sh.desk.first.as_mut() {
                f.fail(said);
            }
            return Err(line);
        }
    };
    sh.desk.wiz_chain = Some(c);
    sh.desk.wiz_chain_done = true;
    if let Some(f) = sh.desk.first.as_mut() {
        f.said = None;
    }
    sh.endpoints_landed();
    let say = format!("网络 · {c}");
    sh.desk.tr.push(a.step, crate::house::mark::FEATURE, say.clone());
    sh.desk.flush();
    Ok(say)
}

fn role(sh: &mut Shell, a: &Act) -> Said {
    let to = a.arg("to")?;
    let seat = Seat::of(&to).ok_or_else(|| format!("认不出这一席:{to}"))?;
    if sh.desk.seat() != seat || sh.tasks.in_flight(Kind::Seat) {
        let want = eravon_vault::vault::role_of(seat.store());
        let got = crate::action::switch_to(sh, want);
        let got = crate::house::act::said_as_family(sh, got);
        if got == crate::action::Applied::Started(Kind::Seat) {
            sh.desk.role_landed = Some(to.clone());
            return Ok(format!("我是{} · 正在切换", crate::house::i18n::get_named(crate::lang::lang(), &format!("seat.{to}")).0));
        }
        let asked = sh.desk.sh.seller.goods.as_ref().and_then(|g| g.edit.as_ref()).and_then(|e| e.ask.clone()) == Some(crate::house::seller::Leave::Seat(seat));
        if sh.desk.seat() != seat && asked {
            return Ok(crate::house::i18n::t(crate::house::i18n::Key::GoodsAskSave));
        }
        if sh.desk.seat() != seat {
            return Err(format!("换席没成 · {}", crate::house::drive::said_of(&got)));
        }
    }
    lay_role(sh, &to)
}

pub(crate) fn role_landed(sh: &mut Shell) {
    let Some(to) = sh.desk.role_landed.take() else { return };
    if Seat::of(&to) != Some(sh.desk.seat()) {
        return;
    }
    if let Err(e) = lay_role(sh, &to) {
        sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: e, next: String::new(), detail: None });
    }
}

fn lay_role(sh: &mut Shell, to: &str) -> Said {
    let seat = Seat::of(to).ok_or_else(|| format!("认不出这一席:{to}"))?;
    let (data, me, now) = (sh.desk.data.clone(), sh.desk.addr.clone(), sh.desk.now());
    let chain = crate::house::first::chain_on(sh.desk.wiz_chain, sh.home.as_ref().map(|_| &sh.settings));
    if me.is_empty() {
        return Err(crate::house::i18n::t(crate::house::i18n::Key::ChipNoKey));
    }
    let store = seat.store();
    if !sh.house_spawn(Kind::First, crate::house::desk::Site::Page, true, move |_| Landed::FirstSeat(store, crate::house::first::land_seat(&data, store, &me, chain, now))) {
        sh.desk.busy_said();
        return Ok("忙".into());
    }
    sh.desk.wiz_role = None;
    Ok(format!("我是{}", crate::house::i18n::get_named(crate::lang::lang(), &format!("seat.{to}")).0))
}

fn payout(sh: &mut Shell, a: &Act) -> Said {
    let f = sh.desk.first.as_mut().ok_or_else(|| "向导没开".to_string())?;
    if let Some(v) = a.opt("a") {
        f.set_payout(v);
    }
    if let Some(v) = a.opt("again") {
        f.set_again(v);
    }
    if f.payout.trim().is_empty() {
        sh.desk.tr.push(Step::Payout, crate::house::mark::FEATURE, "稍后设置".into());
        return Ok("稍后设置".into());
    }
    let (addr, again) = (f.payout.clone(), f.again.clone());
    let (data, me, now) = (sh.desk.data.clone(), sh.desk.addr.clone(), sh.desk.now());
    if !sh.house_spawn(Kind::First, crate::house::desk::Site::Page, true, move |_| Landed::FirstPayout(crate::house::first::land_payout(&data, &me, &addr, &again, now))) {
        sh.desk.busy_said();
        return Ok("忙".into());
    }
    Ok("收款地址".into())
}

pub(crate) fn land_seat(sh: &mut Shell, seat: eravon_store::vault::Seat, got: Result<eravon_vault::onboard::Born, crate::house::seller::Said>) {
    let say = match &got {
        Ok(b) => format!("席位就绪 · {} · 端点 {}", seat.word(), b.endpoints),
        Err(s) => format!("席位没成 · {}", s.name),
    };
    let laid = got.is_ok();
    if let (Err(s), Some(f)) = (got, sh.desk.first.as_mut()) {
        f.fail(s);
    }
    if laid {
        sh.desk.wiz_chain = None;
        sh.desk.wiz_chain_done = false;
    }
    sh.desk.tr.push(Step::Role, eravon_vault::mark::FEATURE, say);
    sh.reread_ready();
    if laid {
        sh.settings_written();
        sh.endpoints_landed();
    }
    sh.desk.flush();
}

pub(crate) fn land_payout(sh: &mut Shell, got: Result<Result<usize, crate::house::seller::Said>, crate::house::seller::Said>) {
    let say = match got {
        Ok(goods) => {
            if let Err(s) = &goods {
                sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: crate::house::refusal::said(crate::lang::lang(), &s.name, &s.detail), next: String::new(), detail: Some(s.name.clone()) });
            }
            if let Some(f) = sh.desk.first.as_mut() {
                f.said = None;
            }
            format!("收款地址 · 过 · 待重签 {}", goods.map(|n| n.to_string()).unwrap_or_else(|s| s.name))
        }
        Err(s) => {
            let say = format!("收款地址 · 拒 · {}", s.detail);
            if let Some(f) = sh.desk.first.as_mut() {
                f.fail(s);
            }
            say
        }
    };
    sh.desk.tr.push(Step::Payout, eravon_vault::mark::FEATURE, say);
    sh.reread_ready();
    sh.desk.flush();
}
