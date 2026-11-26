use super::{need_seller, said_of};
use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::desk::Site;
use crate::house::orders::{Orders, Plan};
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    let s = a.step;
    if matches!(s, Step::CaseJudge | Step::CaseDocs) && sh.desk.seat() == crate::house::shell::Seat::Buyer {
        return None;
    }
    if s == Step::OrderBack && sh.desk.seat() == crate::house::shell::Seat::Buyer {
        if let Some(b) = sh.desk.sh.buyer_orders.as_mut() {
            b.leave();
            b.open = None;
        }
        sh.desk.tr.push(s, crate::house::mark::FEATURE_P5, "返回".into());
        return Some(Ok("返回".into()));
    }
    Some(match s {
        Step::OrderBack
        | Step::OrderTab
        | Step::OrderListTab
        | Step::OrderSearch
        | Step::OrderSearchOff
        | Step::OrderTerms
        | Step::OrderPasteOpen
        | Step::OrderPasteClose
        | Step::DeliverNote
        | Step::DeliverMethod
        | Step::DeliverClose
        | Step::RefundClose
        | Step::EvidenceNote
        | Step::EvidenceClose
        | Step::EvidenceJudgeClose
        | Step::SettleOpen
        | Step::SettlePreview
        | Step::SettleClose
        | Step::EvidenceOwn
        | Step::TodoCopy => local(sh, a),
        Step::OrderHistory => (|| {
            let o = sh.desk.sh.seller.orders.as_ref().ok_or_else(|| "先打开订单页".to_string())?;
            let buyer = o.open_buyer().ok_or_else(|| "先打开一单".to_string())?;
            let me = sh.desk.addr.clone();
            let said = crate::house::history::ask(sh, crate::house::history::Page::Orders, &me, &buyer)?;
            with_orders(sh, |o| o.inner = Some(crate::house::orders::Inner::History))?;
            Ok(said)
        })(),
        Step::OrderPaste => {
            let (v, now) = (a.arg("v"), sh.desk.now());
            v.and_then(|v| orders_do(sh, a, s, "贴入".into(), move |o, d, ad, _k| o.paste(d, ad, now, &v)))
        }
        Step::OrderOpen => {
            let (id, now) = (a.arg("id"), sh.desk.now());
            id.and_then(|id| orders_do(sh, a, s, id.clone(), move |o, d, ad, _k| o.open_one(d, ad, now, &id)))
        }
        Step::OrderRow => {
            let (i, now) = (a.arg("instance"), sh.desk.now());
            i.and_then(|i| orders_do(sh, a, s, i.clone(), move |o, d, ad, _k| o.open_row(d, ad, now, &i)))
        }
        Step::OrderLabel => {
            let now = sh.desk.now();
            match (a.arg("buyer"), a.arg("v")) {
                (Ok(b), Ok(v)) => orders_do(sh, a, s, v.clone(), move |o, d, ad, _k| o.set_label(d, ad, now, &b, &v)),
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }
        Step::OrderPoll => {
            let now = sh.desk.now();
            orders_do(sh, a, s, "刷新".into(), move |o, d, ad, k| o.poll(d, ad, now, k))
        }
        Step::OrderScan => {
            let now = sh.desk.now();
            orders_do(sh, a, s, "扫链".into(), move |o, d, ad, k| o.scan(d, ad, now, k))
        }
        Step::PendingBump => {
            let now = sh.desk.now();
            orders_do(sh, a, s, "加价重发".into(), move |o, d, ad, k| o.pending_bump(d, ad, now, k))
        }
        Step::PendingVoid => {
            let now = sh.desk.now();
            orders_do(sh, a, s, "作废".into(), move |o, d, ad, _k| o.pending_void(d, ad, now))
        }
        Step::DeliverOpen => orders_do(sh, a, s, "交付".into(), |o, d, ad, _k| o.deliver_open(d, ad)),
        Step::RefundOpen => orders_do(sh, a, s, "退款".into(), |o, d, ad, _k| o.refund_open(d, ad)),
        Step::EvidenceOpen => {
            let home = sh.desk.sh.seller.field.home.clone();
            orders_do(sh, a, s, "提交证据".into(), move |o, d, ad, _k| o.evidence_open(d, ad, &home))
        }
        Step::EvidenceJudge => {
            let (now, sheet) = (sh.desk.now(), a.opt("sheet") != Some("false"));
            orders_do(sh, a, s, "判官".into(), move |o, d, ad, _k| o.judge_read(d, ad, now, sheet))
        }
        Step::CaseJudge => crate::house::orders::Inner::of("judge", a.opt("tab").unwrap_or_default()).and_then(|inner| {
            let fresh = !matches!(sh.desk.sh.seller.orders.as_ref().and_then(|o| o.inner), Some(crate::house::orders::Inner::Judge(_)) | Some(crate::house::orders::Inner::Lapse));
            if fresh {
                with_orders(sh, |o| o.bench = None)?;
            }
            let (now, tab) = (sh.desk.now(), inner.tab());
            orders_do(sh, a, s, format!("本案判官 {tab}"), move |o, d, ad, _k| o.case_judge(d, ad, now, tab))
        }),
        Step::CaseDocs => crate::house::orders::Inner::of("docs", a.opt("tab").unwrap_or_default()).and_then(|inner| {
            let tab = inner.tab();
            orders_do(sh, a, s, format!("随案文书 {tab}"), move |o, d, ad, _k| o.case_docs(d, ad, tab))
        }),
        Step::EvidenceSeal => a.arg("dir").and_then(|v| orders_do(sh, a, s, v.clone(), move |o, _d, _ad, _k| o.seal_export(&v))),
        Step::DeliverSave => a.arg("dir").and_then(|v| orders_do(sh, a, s, v.clone(), move |o, d, ad, _k| o.save_originals(d, ad, &v))),
        Step::DeliverPick => a.arg("path").and_then(|p| {
            with_orders(sh, |o| o.deliver_pick_begin(&p))?;
            orders_do(sh, a, s, p, |o, _d, ad, _k| o.deliver_pick_read(ad))
        }),
        Step::EvidenceKit => a.arg("id").and_then(|id| {
            let kit = (!id.is_empty()).then_some(id.clone());
            with_orders(sh, |o| o.evidence_kit_begin(kit))?;
            orders_do(sh, a, s, id, |o, _d, ad, _k| o.evidence_read(ad))
        }),
        Step::EvidencePick => a.arg("path").and_then(|p| {
            with_orders(sh, |o| o.evidence_pick_begin(&p))?;
            orders_do(sh, a, s, p, |o, _d, ad, _k| o.evidence_read(ad))
        }),
        Step::DeliverSend => orders_send(sh, a, Sends::Deliver),
        Step::RefundSend => orders_send(sh, a, Sends::Refund),
        Step::EvidenceSend => orders_send(sh, a, Sends::Evidence),
        Step::SettleMake => match (a.arg("bps"), a.arg("days")) {
            (Ok(b), Ok(d)) => orders_send(sh, a, Sends::Settle(b, d)),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        Step::SigConfirm => a.arg("v").and_then(|v| orders_send(sh, a, Sends::SigConfirm(v))),
        Step::SigSettle => a.arg("v").and_then(|v| orders_send(sh, a, Sends::SigSettle(v))),
        Step::OrderCompanyOpen => orders_send(sh, a, Sends::Open),
        Step::OrderDeploy => {
            let now = sh.desk.now();
            orders_do(sh, a, s, "开案".into(), move |o, d, ad, k| o.deploy(d, ad, now, k))
        }
        Step::OrderAdvance => orders_do(sh, a, s, "到期结清".into(), move |o, d, ad, k| o.advance_send(d, ad, k)),
        _ => return None,
    })
}

fn with_orders(sh: &mut Shell, f: impl FnOnce(&mut Orders)) -> Result<(), String> {
    need_seller(sh)?;
    let o = sh.desk.sh.seller.orders.as_mut().ok_or_else(|| "先打开订单那一面".to_string())?;
    f(o);
    Ok(())
}

fn local(sh: &mut Shell, a: &Act) -> Said {
    need_seller(sh)?;
    let addr = sh.orders_key();
    let data = sh.desk.data.clone();
    let o = sh.desk.sh.seller.orders.as_mut().ok_or_else(|| "先打开订单那一面".to_string())?;
    match a.step {
        Step::OrderBack => o.back(),
        Step::OrderTab => {
            let v = a.arg("v")?;
            o.set_tab(crate::house::orders::Tab::of(&v).ok_or_else(|| format!("详情没有 {v} 这一页签"))?)
        }
        Step::OrderListTab => match a.arg("v")?.as_str() {
            "active" => o.list_tab(false),
            "done" => o.list_tab(true),
            other => return Err(format!("列表只有 active 与 done 两页签,给的是 {other}")),
        },
        Step::OrderSearch => o.search_set(a.opt("q").unwrap_or_default(), a.opt("from").unwrap_or_default(), a.opt("to").unwrap_or_default()),
        Step::OrderSearchOff => o.search_off(),
        Step::OrderTerms => {
            o.terms_open = a.opt("v") != Some("close");
            if o.terms_open {
                o.case_docs(&data, &addr, 0);
            } else if matches!(o.inner, Some(crate::house::orders::Inner::Docs(_))) {
                o.inner = None;
            }
        }
        Step::OrderPasteOpen => o.paste_open = true,
        Step::OrderPasteClose => o.paste_open = false,
        Step::DeliverNote => o.deliver_note(&addr, &a.arg("v")?),
        Step::DeliverMethod => {
            let v = a.arg("v")?;
            if let Some(m) = o.deliver.as_mut() {
                m.method = v;
            }
        }
        Step::DeliverClose => o.deliver_close(),
        Step::RefundClose => o.refund_close(),
        Step::EvidenceNote => o.evidence_note(&addr, &a.arg("v")?),
        Step::EvidenceClose => o.evidence_close(),
        Step::EvidenceJudgeClose => o.judge_close(),
        Step::SettleOpen => o.settle_open(),
        Step::SettlePreview => {
            o.settle_preview();
        }
        Step::SettleClose => o.settle_close(),
        Step::EvidenceOwn => {
            let n: u64 = a.arg("n")?.parse().map_err(|_| "n 要一个份序".to_string())?;
            o.open_own(n);
        }
        _ => {
            let item = a.arg("item")?;
            o.copied = Some(o.todo_item(&item).ok_or_else(|| format!("授权待办里没有 {item} 这一项(或还没有待办)"))?);
        }
    }
    let feature = feature_of(a.step);
    let said = said_of(&o.said);
    sh.desk.tr.push(a.step, feature, a.opt("v").unwrap_or_default().to_string());
    Ok(said)
}

fn feature_of(step: Step) -> eravon_literals::trace::Feature {
    let name = step.name();
    if crate::house::seam::s2_step(name) {
        eravon_orders::mark::FEATURE
    } else if name.starts_with("order-") || name.starts_with("pending-") {
        eravon_watch::mark::FEATURE
    } else if crate::house::seam::dispute_step(name) {
        eravon_dispute::mark::FEATURE
    } else {
        eravon_deliver::mark::FEATURE
    }
}

fn export_dir(o: &mut Orders, a_dir: Option<&str>, home: &str, data: &str) {
    match a_dir {
        Some(d) => o.export_dir = d.to_string(),
        None => crate::house::orders::set_export_dir(&mut o.export_dir, &mut o.export_why, home, data),
    }
}

fn orders_do(sh: &mut Shell, a: &Act, step: Step, say: String, f: impl FnOnce(&mut Orders, &str, &str, &crate::house::orders::Key) + Send + 'static) -> Said {
    need_seller(sh)?;
    let (data, addr) = (sh.desk.data.clone(), sh.orders_key());
    let key = crate::house::orders::Key::of(&sh.desk.session, &addr);
    let home = sh.desk.sh.seller.field.home.clone();
    let dir = a.opt("dir").map(str::to_string);
    sh.spawn_orders(Site::Page, true, move |mut o, t| {
        export_dir(&mut o, dir.as_deref(), &home, &data);
        f(&mut o, &data, &addr, &key.bound(t));
        o
    });
    sh.desk.tr.push(step, feature_of(step), say.clone());
    Ok(say)
}

#[derive(Clone, Debug)]
enum Sends {
    Deliver,
    Refund,
    Evidence,
    Settle(String, String),
    SigConfirm(String),
    SigSettle(String),
    Open,
}

fn orders_send(sh: &mut Shell, a: &Act, x: Sends) -> Said {
    need_seller(sh)?;
    let (data, addr, home) = (sh.desk.data.clone(), sh.orders_key(), sh.desk.sh.seller.field.home.clone());
    let dir = a.opt("dir").map(str::to_string);
    if let Some(o) = sh.desk.sh.seller.orders.as_mut() {
        export_dir(o, dir.as_deref(), &home, &data);
    }
    let (plan, site, say) = match &x {
        Sends::Deliver => (Plan::Deliver, "deliver", "claimDelivery".to_string()),
        Sends::Refund => (Plan::Refund, "refund", "refund".to_string()),
        Sends::Evidence => (Plan::Evidence, "evidence", "commit".to_string()),
        Sends::Settle(b, d) => (Plan::Settle(b.clone(), d.clone()), "settle", format!("{b} · {d}")),
        Sends::SigConfirm(_) => (Plan::Sig, "sig", "confirmBySig".to_string()),
        Sends::SigSettle(_) => (Plan::Sig, "settle", "settle".to_string()),
        Sends::Open => (Plan::Open, "open", "init".to_string()),
    };
    let key = crate::house::orders::Key::of(&sh.desk.session, &addr);
    let why = sh.write_why(crate::house::shell::Write::Local).map(|w| crate::house::seller::Said::bad("E_APP_WRITE", w));
    let asked = sh.desk.sh.seller.orders.as_ref().map_or(Ok(()), |o| crate::house::asked(o.plan(&data, &addr, &key, &plan))).and_then(|()| why.map_or(Ok(()), Err));
    if let Err(s) = asked {
        sh.desk.tr.push(a.step, eravon_watch::mark::FEATURE, format!("{say} · 未起 · {}", s.name));
        let line = format!("{} · {}", s.name, s.detail);
        sh.orders_said(s);
        return Ok(line);
    }
    sh.desk.retry_on(crate::task::Kind::Orders, a.clone());
    let now = sh.desk.now();
    sh.spawn_orders(Site::Sheet(site), true, move |mut o, t| {
        let key = key.bound(t);
        match x {
            Sends::Deliver => o.deliver_send(&data, &addr, now, &key),
            Sends::Refund => o.refund_send(&data, &addr, &key),
            Sends::Evidence => o.evidence_send(&data, &addr, now, &key),
            Sends::Settle(b, d) => o.settle_make(&data, &addr, now, &key, &b, &d),
            Sends::SigConfirm(v) => o.paste_confirm(&data, &addr, now, &key, &v),
            Sends::SigSettle(v) => o.paste_settle(&data, &addr, now, &key, &v),
            Sends::Open => o.company_open(&data, &addr, now, &key),
        }
        o
    });
    sh.desk.tr.push(a.step, eravon_watch::mark::FEATURE, say.clone());
    Ok(say)
}
