use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::complaint::{files_of, Form};
use crate::house::desk::Site;
use crate::house::flow::Ask;
use crate::house::shell::Seat;
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    if !matches!(a.step, Step::ComplaintOpen | Step::ComplaintWrite) {
        return None;
    }
    Some(if sh.desk.seat() == Seat::Buyer { buyer(sh, a) } else { seller(sh, a) })
}

fn form_of(a: &Act) -> Result<Form, String> {
    let text = a.opt("text").unwrap_or_default().to_string();
    if text.trim().is_empty() {
        return Err(crate::house::i18n::t(crate::house::i18n::Key::CpTextHint));
    }
    Ok(Form { text, dir: a.opt("dir").unwrap_or_default().to_string(), files: files_of(a.opt("files").unwrap_or_default())?, guild: a.opt("guild").unwrap_or_default().to_string() })
}

fn write_held(sh: &mut Shell) -> Option<String> {
    sh.write_why(crate::house::shell::Write::Local)
}

fn seller(sh: &mut Shell, a: &Act) -> Said {
    super::need_seller(sh)?;
    if sh.desk.sh.seller.orders.as_ref().and_then(|o| o.sel.as_ref()).is_none() {
        return Err("先打开一单".to_string());
    }
    let (data, addr, now) = (sh.desk.data.clone(), sh.orders_key(), sh.desk.now());
    let go = if a.step == Step::ComplaintOpen {
        sh.spawn_orders(Site::Page, Ask::Asked, move |mut o, _t| {
            o.complaint_open(&data, &addr, now);
            o
        })
    } else {
        let form = form_of(a)?;
        if let Some(w) = write_held(sh) {
            return Err(w);
        }
        let key = crate::house::orders::Key::of(&sh.desk.session, &addr);
        sh.spawn_orders(Site::Sheet("complaint"), Ask::Act, move |mut o, t| {
            let key = key.bound(t);
            o.complaint_write(&data, &addr, now, &key, &form);
            o
        })
    };
    sh.started(go)?;
    let say = a.step.name().to_string();
    sh.desk.tr.push(a.step, eravon_dispute::mark::FEATURE, say.clone());
    Ok(say)
}

fn buyer(sh: &mut Shell, a: &Act) -> Said {
    if sh.desk.sh.buyer_orders.as_ref().and_then(|b| b.open.as_ref()).is_none() {
        return Err("先打开一单".to_string());
    }
    let now = sh.desk.now();
    let go = if a.step == Step::ComplaintOpen {
        sh.spawn_bo_at(Site::Page, Ask::Asked, move |_, data, me, mut b| {
            b.complaint_open(data, me, now);
            b
        })
    } else {
        let form = form_of(a)?;
        if let Some(w) = write_held(sh) {
            return Err(w);
        }
        sh.spawn_bo_at(Site::Sheet("complaint"), Ask::Act, move |s, data, me, mut b| {
            b.complaint_write(s, data, me, now, &form);
            b
        })
    };
    sh.started(go)?;
    let say = a.step.name().to_string();
    sh.desk.tr.push(a.step, eravon_dispute::mark::FEATURE, say.clone());
    Ok(say)
}
