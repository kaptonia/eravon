use super::need_seller;
use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::desk::Site;
use crate::house::seller::{Cands, CandsPlan};
use crate::house::shell::Toast;
use crate::shell::Shell;
use eravon_cands::table::Side;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    Some(match a.step {
        Step::CandsTab | Step::CandTab | Step::CandBack => local(sh, a),
        Step::CandOpen => open(sh, a),
        Step::CandAdd => match (a.arg("kind").or_else(|_| a.arg("t")), a.arg("a")) {
            (Ok(kind), Ok(input)) => match Side::of(&kind) {
                Some(side) => write(sh, a, Write::Add(side, input, a.opt("label").unwrap_or_default().to_string())),
                None => Err(format!("认不出这一类:{kind}")),
            },
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        Step::CandRename => a.arg("a").and_then(|who| write(sh, a, Write::Rename(who, a.opt("label").unwrap_or_default().to_string()))),
        Step::CandRemove => a.arg("a").and_then(|who| write(sh, a, Write::Remove(who))),
        _ => return None,
    })
}

fn local(sh: &mut Shell, a: &Act) -> Said {
    need_seller(sh)?;
    let c = sh.desk.sh.seller.cands.as_mut().ok_or_else(|| "先打开判官行会那一页".to_string())?;
    let say = match a.step {
        Step::CandsTab => {
            let t = a.arg("t")?;
            c.tab = Side::of(&t).ok_or_else(|| format!("认不出这一页签:{t}"))?;
            format!("页签 {t}")
        }
        Step::CandTab => {
            let i = a.arg("i")?;
            let n: usize = i.parse().map_err(|_| format!("页签要是 0 到 3:{i}"))?;
            if n > 3 {
                return Err(format!("页签要是 0 到 3:{i}"));
            }
            c.dtab = n;
            format!("详情页签 {i}")
        }
        _ => {
            c.detail = None;
            sh.desk.rename = None;
            crate::house::i18n::t(crate::house::i18n::Key::ModeBack)
        }
    };
    sh.desk.tr.push(a.step, eravon_cands::mark::FEATURE, say.clone());
    Ok(say)
}

fn open(sh: &mut Shell, a: &Act) -> Said {
    need_seller(sh)?;
    let who = a.arg("a")?;
    let member = a.opt("member") == Some("true");
    let (data, addr, field, now) = (sh.desk.data.clone(), sh.cands_key(), sh.desk.sh.seller.field.clone(), sh.desk.now());
    let w = who.clone();
    let go = sh.spawn_cands(Site::Page, crate::house::flow::Ask::Asked, move |cur| {
        let mut cc = cur.unwrap_or_else(|| Cands::open(&data, &addr, &field, now, if member { Side::Guild } else { Side::Judge }));
        if member {
            cc.peek(&data, &addr, &field, now, &w);
        } else {
            cc.open_one(&data, &addr, &field, now, &w);
        }
        cc
    });
    sh.started(go)?;
    let say = format!("{} {who}", if member { "成员" } else { "打开" });
    sh.desk.tr.push(Step::CandOpen, eravon_cands::mark::FEATURE, say.clone());
    Ok(say)
}

#[derive(Clone, Debug)]
enum Write {
    Add(Side, String, String),
    Rename(String, String),
    Remove(String),
}

impl Write {
    fn plan(&self) -> CandsPlan {
        match self {
            Write::Add(side, input, label) => CandsPlan::Add(*side, input.clone(), label.clone()),
            Write::Rename(_, label) => CandsPlan::Label(label.clone()),
            Write::Remove(_) => CandsPlan::Remove,
        }
    }
}

fn write(sh: &mut Shell, a: &Act, x: Write) -> Said {
    need_seller(sh)?;
    let (data, addr, field) = (sh.desk.data.clone(), sh.cands_key(), sh.desk.sh.seller.field.clone());
    let why = sh.write_why(crate::house::shell::Write::Local).map(|w| crate::house::seller::Said::bad("E_APP_WRITE", w));
    let asked = crate::house::asked(Cands::plan(&data, &addr, &x.plan())).and_then(|()| why.map_or(Ok(()), Err));
    if let Err(s) = asked {
        let line = format!("{} · {}", s.name, s.detail);
        let t = Toast { tone: crate::house::seller::Tone::Bad, head: s.detail.clone(), next: crate::house::sync::next_of(&s), detail: Some(s.name.clone()) };
        if let Some(c) = sh.desk.sh.seller.cands.as_mut() {
            c.said = Some(s);
        }
        sh.desk.sh.toast(t);
        return Ok(line);
    }
    sh.desk.retry_on(crate::task::Kind::Cands, a.clone());
    let (site, say, ok) = match &x {
        Write::Add(side, ..) => ("cands-add", side.word().to_string(), crate::house::i18n::t(if *side == eravon_cands::table::Side::Guild { crate::house::i18n::Key::CandsAddedGuild } else { crate::house::i18n::Key::CandsAddedJudge })),
        Write::Rename(..) => ("cands-rename", "改名".to_string(), crate::house::i18n::t(crate::house::i18n::Key::CandsRenamed)),
        Write::Remove(_) => ("cands-remove", "移除".to_string(), crate::house::i18n::t(crate::house::i18n::Key::CandsRemoved)),
    };
    let now = sh.desk.now();
    let ticket = crate::house::Ticket::standing();
    sh.spawn_cands(Site::Sheet(site), true, move |cur| {
        let side = match &x {
            Write::Add(s, ..) => *s,
            _ => Side::Judge,
        };
        let mut cc = cur.unwrap_or_else(|| Cands::open(&data, &addr, &field, now, side));
        if !ticket.live() {
            cc.said = Some(crate::house::stale());
            return cc;
        }
        match x {
            Write::Add(side, input, label) => cc.add(&data, &addr, &field, now, side, &input, &label, &ok, &|| ticket.live()),
            Write::Rename(who, v) => cc.rename(&data, &addr, &field, now, &who, &v, &ok),
            Write::Remove(who) => cc.remove(&data, &addr, &field, now, &who, &ok),
        }
        cc
    });
    sh.desk.tr.push(a.step, eravon_cands::mark::FEATURE, say.clone());
    Ok(say)
}
