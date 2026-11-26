use super::{need_seller, said_of};
use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    if !matches!(a.step, Step::Fund | Step::OpenWindow | Step::OpenAuto | Step::OpenBuyer | Step::OpeningSave | Step::SetPayout) {
        return None;
    }
    Some(step(sh, a))
}

fn step(sh: &mut Shell, a: &Act) -> Said {
    need_seller(sh)?;
    let (data, addr, now) = (sh.desk.data.clone(), sh.desk.addr.clone(), sh.desk.now());
    let w = |k: crate::house::i18n::Key| crate::house::i18n::t(k);
    let pay = sh.desk.sh.seller.pay.as_mut().ok_or_else(|| "先打开付款方式那一页".to_string())?;
    let (say, feature) = match a.step {
        Step::Fund => {
            pay.fund = a.arg("v")?;
            ("金库那一格".to_string(), eravon_payways::mark::FEATURE)
        }
        Step::OpenWindow => {
            pay.open_window = a.arg("v")?;
            ("开案期限那一格".to_string(), eravon_payways::mark::FEATURE)
        }
        Step::OpenAuto => {
            pay.auto_open = a.arg("v")?;
            ("见款自动开案那一格".to_string(), eravon_payways::mark::FEATURE)
        }
        Step::OpenBuyer => {
            pay.buyer_may_open = a.arg("v")?;
            ("允许买家自付 gas 开案那一格".to_string(), eravon_payways::mark::FEATURE)
        }
        Step::OpeningSave => {
            let word = if a.opt("what") == Some("window") { crate::house::i18n::Key::OpenWindowSaid } else { crate::house::i18n::Key::OpeningSaid };
            pay.save_opening(&data, &addr, now, &w(word), false);
            (format!("保存开案设置 · {}", said_of(&pay.said)), eravon_payways::mark::FEATURE)
        }
        _ => {
            pay.said = Some(crate::house::settings::payout_land(&data, &addr, &a.arg("a")?, &a.arg("again")?, now, &w(crate::house::i18n::Key::SetPayoutSaved)));
            pay.reread_payout(&data, &addr);
            (said_of(&pay.said), eravon_literals::trace::Feature::S9)
        }
    };
    sh.desk.tr.push(a.step, feature, say.clone());
    Ok(say)
}
