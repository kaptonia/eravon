use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    match a.step {
        Step::QueueDirectCheck | Step::QueueDirectGiveUp => Some(a.arg("tx").and_then(|tx| direct(sh, a.step, &tx))),
        _ => None,
    }
}

fn direct(sh: &mut Shell, step: Step, tx: &str) -> Said {
    let me = sh.desk.addr.clone();
    let key = sh.desk.session.key_for(&me).map_err(|e| format!("{} · {}", e.name(), e.detail()))?;
    let d = eravon_cli::deskops::direct::find(&me, &key, tx).map_err(|n| format!("{} · {}", n.name, n.detail()))?;
    let data = sh.desk.data.clone();
    let started = sh.house_spawn(crate::task::Kind::Wallet, crate::house::desk::Site::Page, true, move |_| {
        let said = if step == Step::QueueDirectGiveUp { crate::house::queue::direct_give_up(&data, &me, &key, &d) } else { crate::house::queue::direct_check(&data, &me, &key, &d) };
        crate::house::landed::Landed::Direct(step, said)
    });
    if !started {
        sh.desk.busy_said();
        return Ok("忙".into());
    }
    Ok(step.name().to_string())
}
