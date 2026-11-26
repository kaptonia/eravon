use super::need_seller;
use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::desk::Site;
use crate::house::seller::{Page, Tone};
use crate::house::shell::{Mode, Seat, Toast};
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    Some(match a.step {
        Step::Page | Step::PageMarked => page(sh, a),
        Step::Seat => seat(sh, a),
        Step::Url => a.arg("u").map(|u| {
            sh.take_url(&u);
            sh.desk.sh.links.last().map(|(k, n)| format!("{k} · {n} 字节")).unwrap_or_default()
        }),
        Step::Word => a.arg("key").map(|key| {
            let (text, got) = crate::house::i18n::get_named(crate::lang::lang(), &key);
            sh.desk.sh.words.push((key.clone(), got, text.clone()));
            if !got {
                sh.desk.sh.toast(Toast { tone: Tone::Bad, head: format!("取不到界面词:{key}"), next: text.clone(), detail: None });
                sh.desk.tr.push(Step::I18nMiss, crate::house::i18n::FEATURE, format!("取不到 {key}"));
            }
            text
        }),
        Step::Mode => a.arg("to").and_then(|to| {
            let m = Mode::of(&to).ok_or_else(|| format!("认不出这一种页面模式:{to}"))?;
            sh.desk.sh.mode_to(m);
            Ok(crate::house::i18n::get_named(crate::lang::lang(), &format!("mode.{to}")).0)
        }),
        Step::Close => {
            sh.desk.sh.close_sheet();
            Ok(crate::house::i18n::t(crate::house::i18n::Key::ToastClose))
        }
        Step::Input => a.arg("at").and_then(|at| {
            let at = at.parse::<u64>().map_err(|_| "at 要是整数".to_string())?;
            sh.desk.sh.last_input = at;
            Ok(format!("输入 · {at}"))
        }),
        Step::Tick => a.arg("at").and_then(|at| {
            let at = at.parse::<u64>().map_err(|_| "at 要是整数".to_string())?;
            sh.desk.sh.at(std::time::Duration::from_secs(at).as_millis() as u64);
            let m = &sh.machine;
            if sh.unlocked() && m.auto_lock && crate::machine::idle_due(sh.desk.sh.last_input as f64, at as f64, m.auto_lock_secs) {
                crate::house::act::family_said(sh, crate::action::Action::Lock);
                return Ok(format!("{} · 闲置到时", crate::house::i18n::t(crate::house::i18n::Key::PinLockedTitle)));
            }
            sh.desk.clock = Some(at);
            if sh.unlocked() && !sh.covered() && !sh.house_in_flight(crate::task::Kind::Sync) && crate::house::refresh::due(sh.desk.sh.refresh_at, at, sh.desk.sh.refresh_every) {
                sh.refresh_for(crate::house::refresh::Cause::Period, None);
                return Ok(format!("周期到 · {at}"));
            }
            Ok(format!("未到时 · {at}"))
        }),
        Step::Refresh => Ok(refresh(sh)),
        Step::TimeAsk => time_ask(sh, a),
        Step::DoorApprove => door_answer(sh, true),
        Step::DoorDecline => door_answer(sh, false),
        Step::OrderExport | Step::BoExport => export_from(sh, a),
        Step::HomeOrder => home_order(sh, a),
        Step::HomeCand => a.arg("a").and_then(|c| {
            need_seller(sh)?;
            sh.desk.tr.push(Step::HomeCand, eravon_home::mark::FEATURE, c.clone());
            sh.go_page_now(Some(Page::Cands));
            crate::house::act::apply_said(sh, crate::house::act::Act::with(crate::house::action::Action::CandOpen, &[("a", &c)]));
            Ok(format!("首页判官行会那一行 · {c}"))
        }),
        Step::NoteHandle | Step::NoteCopy | Step::HomeRefresh => note(sh, a),
        Step::OrderInfo => order_info(sh, a),
        _ => return None,
    })
}

fn page(sh: &mut Shell, a: &Act) -> Said {
    let to = a.arg("to")?;
    let p = Page::of(&to).ok_or_else(|| format!("认不出这一页:{to}"))?;
    let seat = sh.desk.seat();
    if let Some(pg) = p {
        match pg {
            Page::PayLink if seat != Seat::Buyer => return Err("付款链接页只在买家席".to_string()),
            Page::Orders | Page::Alerts | Page::Export | Page::Queue | Page::Settings | Page::PayLink | Page::Wallet => {}
            _ => need_seller(sh)?,
        }
        if sh.desk.addr.is_empty() && pg != Page::Settings && pg != Page::PayLink {
            let why = crate::house::shell::current_key(&sh.desk.data, seat).err().unwrap_or_default();
            return Err(format!("这一席还没有当前钥,身份目录里的页开不了:{why}"));
        }
    }
    if !sh.go_page_now(p) {
        return Ok(crate::house::i18n::t(crate::house::i18n::Key::GoodsAskSave));
    }
    use crate::house::i18n::Key as K;
    Ok(crate::house::i18n::t(match p {
        Some(Page::Settings) => K::NavSettings,
        Some(Page::Payways) => K::NavPayways,
        Some(Page::Cands) => K::NavCands,
        Some(Page::Goods) => K::NavGoods,
        Some(Page::Orders) => K::NavOrders,
        Some(Page::Alerts) => K::NavAlerts,
        Some(Page::PayLink) => K::TaskPaylink,
        Some(Page::Wallet) => K::TaskWallet,
        Some(Page::Export) => K::IdExportGroup,
        Some(Page::Queue) => K::NavQueue,
        None => K::NavHome,
    }))
}

fn seat(sh: &mut Shell, a: &Act) -> Said {
    let to = a.arg("to")?;
    let s = Seat::of(&to).ok_or_else(|| format!("认不出这一席:{to}"))?;
    if !switch_seat(sh, s)? {
        return Ok(crate::house::i18n::t(crate::house::i18n::Key::GoodsAskSave));
    }
    Ok(crate::house::i18n::get_named(crate::lang::lang(), &format!("seat.{to}")).0)
}

pub(super) fn switch_seat(sh: &mut Shell, s: Seat) -> Result<bool, String> {
    if s != sh.desk.seat() {
        if !sh.desk.sh.go(crate::house::seller::Leave::Seat(s)) {
            return Ok(false);
        }
        if let crate::action::Applied::Trouble(f) = crate::action::apply_settled(sh, crate::action::Action::SwitchRole) {
            return Err(format!("{} · {}", f.said(), f.tail()));
        }
    }
    Ok(true)
}

pub(super) fn refresh(sh: &mut Shell) -> String {
    sh.desk.sh.read_forget();
    sh.refresh_for(crate::house::refresh::Cause::Key, None);
    sh.orders_catch_up();
    sh.desk.sh.sync.table.name().to_string()
}

fn door_answer(sh: &mut Shell, go: bool) -> Said {
    let Some(verb) = crate::house::approve::awaiting(sh).map(|q| q.verb.clone()) else {
        return Ok("门 · 没有待批的请求".to_string());
    };
    if go {
        crate::house::approve::approve(sh);
        Ok(format!("门 · 同意 {verb}"))
    } else {
        crate::house::approve::decline(sh);
        sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Note, head: crate::house::i18n::t(crate::house::i18n::Key::DoorDeclined), next: String::new(), detail: None });
        Ok(format!("门 · 拒绝 {verb}"))
    }
}

fn time_ask(sh: &mut Shell, a: &Act) -> Said {
    let go = match a.arg("v")?.as_str() {
        "go" => true,
        "cancel" => false,
        other => return Err(format!("那一问只有 go 与 cancel 两键,给的是 {other}")),
    };
    let word = if go { "go" } else { "cancel" };
    let now = sh.desk.now();
    let d = &sh.desk.sh;
    let (g, o, b, p) = (d.seller.goods.as_ref().is_some_and(|x| x.ask.is_some()), d.seller.orders.as_ref().is_some_and(|x| x.ask.is_some()), d.buyer_orders.as_ref().is_some_and(|x| x.ask.is_some()), d.paylink.as_ref().is_some_and(|x| x.ask.is_some()));
    if g {
        let done = crate::house::i18n::t(crate::house::i18n::Key::OrdersProposalAt);
        sh.spawn_goods(move |gd, data, addr, _field, _signer| gd.ask_answer(data, addr, go, &done));
    } else if o {
        let (data, addr) = (sh.desk.data.clone(), sh.desk.addr.clone());
        sh.spawn_orders(Site::Page, true, move |mut x, _t| {
            x.ask_answer(&data, &addr, now, go);
            x
        });
    } else if b {
        sh.spawn_bo_at(Site::Page, true, move |_s, data, me, mut x| {
            x.ask_answer(data, me, now, go);
            x
        });
    } else if p {
        let (registries, data) = (sh.desk.sh.seller.field.registries.clone(), sh.desk.data.clone());
        sh.spawn_pl_act(Site::Ask, move |pl, _s, buyer, _t| pl.ask_answer(&data, buyer, &registries, now, go));
    } else {
        return Ok("那一问 · 没有待答的".to_string());
    }
    sh.desk.tr.push(Step::TimeAsk, eravon_dispute::mark::FEATURE, word.into());
    Ok(format!("那一问 · {word}"))
}

fn export_from(sh: &mut Shell, a: &Act) -> Said {
    let instance = if a.step == Step::OrderExport {
        need_seller(sh)?;
        sh.desk.sh.seller.orders.as_ref().and_then(|o| o.sel.clone()).ok_or_else(|| "先打开一单".to_string())?
    } else {
        if sh.desk.seat() != Seat::Buyer {
            return Err("买家订单详情只在买家席".to_string());
        }
        sh.desk.sh.buyer_orders.as_ref().and_then(|b| b.open.as_ref().map(|o| o.instance.clone())).ok_or_else(|| "先打开一单".to_string())?
    };
    sh.desk.tr.push(a.step, eravon_export::mark::FEATURE, instance.clone());
    sh.desk.pending_export = Some(instance.clone());
    if !sh.go_page_now(Some(Page::Export)) {
        return Ok(crate::house::i18n::t(crate::house::i18n::Key::GoodsAskSave));
    }
    Ok(format!("{} · {instance}", a.step.name()))
}

fn home_order(sh: &mut Shell, a: &Act) -> Said {
    let i = a.arg("i")?;
    if sh.desk.addr.is_empty() {
        return Err("这一席还没有当前钥,身份目录里的页开不了".to_string());
    }
    if sh.desk.seat() == Seat::Buyer {
        sh.desk.pending_url.clear();
        if !sh.desk.sh.go(crate::house::seller::Leave::Page(Some(Page::Orders))) {
            return Ok(crate::house::i18n::t(crate::house::i18n::Key::GoodsAskSave));
        }
        sh.desk.goto = Some(Some(Page::Orders));
        sh.desk.tr.push(Step::HomeOrder, crate::house::mark::FEATURE_P8, i.clone());
        let v = i.clone();
        let go = sh.spawn_bo_at(Site::Page, crate::house::flow::Ask::Asked, move |s, data, me, _| {
            let mut b = crate::house::buyer_orders::BOrders::load(s, data, me);
            b.open_row(data, me, &v, crate::house::buyer_orders::Tab::Overview);
            b
        });
        sh.started(go)?;
        return Ok(format!("首页订单那一行 · {i}"));
    }
    sh.desk.tr.push(Step::HomeOrder, eravon_home::mark::FEATURE, i.clone());
    sh.go_page_now(Some(Page::Orders));
    let go = sh.orders_open_row(i.clone());
    sh.started(go)?;
    Ok(format!("首页订单那一行 · {i}"))
}

fn note(sh: &mut Shell, a: &Act) -> Said {
    let (data, addr, now) = (sh.desk.data.clone(), sh.desk.addr.clone(), sh.desk.now());
    if sh.desk.seat() == Seat::Buyer {
        if a.step == Step::HomeRefresh {
            sh.spawn_bhome();
            return Ok("首页 · 重读".to_string());
        }
        let h = sh.desk.sh.buyer_home.as_mut().ok_or_else(|| "先打开首页或通知页".to_string())?;
        return match a.step {
            Step::NoteHandle => {
                let key = a.arg("key")?;
                let r = h.handle(&data, now, &key);
                sh.desk.tr.push(Step::NoteHandle, crate::house::mark::FEATURE_P8, key.clone());
                match r {
                    Ok(()) => Ok(format!("已处理 · {key}")),
                    Err(e) => {
                        sh.desk.sh.toast(Toast { tone: Tone::Bad, head: e.detail.clone(), next: String::new(), detail: Some(e.name.clone()) });
                        Err(format!("{}:{}", e.name, e.detail))
                    }
                }
            }
            _ => {
                let (key, item) = (a.arg("key")?, a.arg("item")?);
                h.copied = Some(h.item(&key, &item).ok_or_else(|| format!("通知 {key} 里没有 {item} 这一项"))?);
                Ok(format!("复制 · {item}"))
            }
        };
    }
    need_seller(sh)?;
    if a.step == Step::HomeRefresh {
        sh.spawn_home();
        return Ok("首页 · 重读".to_string());
    }
    let h = sh.desk.sh.seller.home.as_mut().ok_or_else(|| "先打开首页或通知页".to_string())?;
    match a.step {
        Step::NoteHandle => {
            let key = a.arg("key")?;
            let r = h.handle(&data, &addr, now, &key);
            sh.desk.tr.push(Step::NoteHandle, eravon_home::mark::FEATURE, key.clone());
            match r {
                Ok(()) => Ok(format!("已处理 · {key}")),
                Err(e) => {
                    sh.desk.sh.toast(Toast { tone: Tone::Bad, head: e.detail.clone(), next: String::new(), detail: Some(e.name.clone()) });
                    Err(format!("{}:{}", e.name, e.detail))
                }
            }
        }
        _ => {
            let (key, item) = (a.arg("key")?, a.arg("item")?);
            h.copied = Some(h.item(&key, &item).ok_or_else(|| format!("通知 {key} 里没有 {item} 这一项"))?);
            Ok(format!("复制 · {item}"))
        }
    }
}

fn order_info(sh: &mut Shell, a: &Act) -> Said {
    let o = sh.desk.sh.seller.orders.as_ref().ok_or_else(|| "先打开订单那一面".to_string())?;
    let d = o.detail.as_ref().ok_or_else(|| "先打开那一单".to_string())?;
    let n: usize = a.arg("n")?.parse().map_err(|_| "n 要是整数".to_string())?;
    use crate::house::i18n::Key as K;
    let t = crate::house::i18n::t;
    match a.arg("tab")?.as_str() {
        "record" => {
            let rows = d.record.as_ref().map_err(|s| format!("{} · {}", s.name, s.detail))?;
            let m = rows.get(n).ok_or_else(|| format!("记录页签没有第 {n} 行"))?;
            Ok(format!("{} · {} · {}", t(K::SendTx), m.say, m.tx))
        }
        "delivery" => {
            let files = d.delivery.files.as_ref().map_err(|(name, why)| format!("{name} · {why}"))?;
            let f = files.get(n).ok_or_else(|| format!("交付文件表没有第 {n} 行"))?;
            Ok(format!("{} · {} · {} · {}", t(K::OrdersFileSheet), f.0, f.1, f.2))
        }
        other => Err(format!("页单只开记录与交付两张表:{other}")),
    }
}
