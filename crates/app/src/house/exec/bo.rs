use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::buyer_orders::Tab;
use crate::house::desk::Site;
use crate::house::shell::{Seat, Toast};
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    let s = a.step;
    if !matches!(
        s,
        Step::BoListTab
            | Step::BoOpen
            | Step::BoTab
            | Step::BoReload
            | Step::BoConfirmOpen
            | Step::BoConfirm
            | Step::BoConfirmHand
            | Step::BoRefundOpen
            | Step::BoDispute
            | Step::BoCommit
            | Step::BoOffer
            | Step::BoOffersScan
            | Step::BoSettle
            | Step::BoAdvance
            | Step::BoExpire
            | Step::BoCancel
            | Step::BoOpenCase
            | Step::BoOwed
            | Step::BoSweep
            | Step::BoEvidenceOpen
            | Step::BoEvidenceJudge
            | Step::BoEvidenceJudgeClose
            | Step::BoHistory
            | Step::BoEvidenceSeal
            | Step::BoSearch
            | Step::CaseJudge
            | Step::CaseDocs
    ) {
        return None;
    }
    if sh.desk.seat() != Seat::Buyer {
        return Some(Err("买家订单页只在买家席".to_string()));
    }
    if sh.desk.sh.buyer_orders.is_none() && s != Step::BoOpen {
        return Some(Err("先打开买家席订单页".to_string()));
    }
    Some(match s {
        Step::BoListTab | Step::BoTab | Step::BoSearch => local(sh, a),
        Step::BoOpen => a.arg("instance").and_then(|i| {
            let inst = i.clone();
            let go = sh.spawn_bo_at(Site::Page, crate::house::flow::Ask::Asked, move |_, data, me, mut b| {
                b.open_row(data, me, &inst, Tab::Overview);
                b
            });
            sh.started(go)?;
            sh.desk.tr.push(Step::BoOpen, crate::house::mark::FEATURE_P5, i.clone());
            Ok(i)
        }),
        Step::BoEvidenceJudge => (|| {
            let (now, sheet) = (sh.desk.now(), a.opt("sheet") != Some("false"));
            let go = sh.spawn_bo_at(Site::Page, crate::house::flow::Ask::Asked, move |_, data, me, mut b| {
                b.judge_read(data, me, now, sheet);
                b
            });
            sh.started(go)?;
            sh.desk.tr.push(s, eravon_dispute::mark::FEATURE, "判官".into());
            Ok("判官".into())
        })(),
        Step::CaseJudge => crate::house::orders::Inner::of("judge", a.opt("tab").unwrap_or_default()).and_then(|inner| {
            let fresh = !matches!(sh.desk.sh.buyer_orders.as_ref().and_then(|b| b.open.as_ref()).and_then(|o| o.inner), Some(crate::house::orders::Inner::Judge(_)) | Some(crate::house::orders::Inner::Lapse));
            let (now, tab) = (sh.desk.now(), inner.tab());
            let go = sh.spawn_bo_at(Site::Page, crate::house::flow::Ask::Asked, move |_, data, me, mut b| {
                b.case_judge(data, me, now, tab, fresh);
                b
            });
            sh.started(go)?;
            sh.desk.tr.push(s, eravon_orders::mark::FEATURE, format!("本案判官 {tab}"));
            Ok(format!("本案判官 {tab}"))
        }),
        Step::CaseDocs => crate::house::orders::Inner::of("docs", a.opt("tab").unwrap_or_default()).and_then(|inner| {
            let b = sh.desk.sh.buyer_orders.as_mut().ok_or_else(|| "先打开买家席订单页".to_string())?;
            if b.open.is_none() {
                return Err("先打开一单".to_string());
            }
            b.case_docs(inner.tab());
            let say = format!("随案文书 {}", inner.tab());
            sh.desk.tr.push(s, eravon_orders::mark::FEATURE, say.clone());
            sh.desk.flush();
            Ok(say)
        }),
        Step::BoHistory => (|| {
            let b = sh.desk.sh.buyer_orders.as_ref().ok_or_else(|| "先打开买家席订单页".to_string())?;
            let o = b.open.as_ref().ok_or_else(|| "先打开一单".to_string())?;
            let seller = b.row_of(&o.instance).map(|r| r.seller.clone()).filter(|x| !x.is_empty()).ok_or_else(|| "这一单读不出卖家".to_string())?;
            let me = crate::house::shell::current_key(&sh.desk.data, Seat::Buyer).unwrap_or_default();
            let said = crate::house::history::ask(sh, crate::house::history::Page::Bo, &me, &seller)?;
            if let Some(o) = sh.desk.sh.buyer_orders.as_mut().and_then(|b| b.open.as_mut()) {
                o.inner = Some(crate::house::orders::Inner::History);
            }
            Ok(said)
        })(),
        Step::BoEvidenceJudgeClose => {
            if let Some(b) = sh.desk.sh.buyer_orders.as_mut() {
                b.judge_close();
            }
            sh.desk.tr.push(s, eravon_dispute::mark::FEATURE, String::new());
            Ok("关闭".into())
        }
        Step::BoEvidenceSeal => a.arg("dir").and_then(|v| {
            let dir = v.clone();
            let go = sh.spawn_bo_at(Site::Page, crate::house::flow::Ask::Asked, move |_, _data, _me, mut b| {
                b.seal_export(&dir);
                b
            });
            sh.started(go)?;
            sh.desk.tr.push(s, eravon_dispute::mark::FEATURE, v.clone());
            Ok(v)
        }),
        Step::BoReload => {
            sh.load_bo();
            sh.desk.tr.push(Step::BoReload, crate::house::mark::FEATURE_P5, String::new());
            Ok("bo-reload".into())
        }
        _ => act(sh, a),
    })
}

fn local(sh: &mut Shell, a: &Act) -> Said {
    let b = sh.desk.sh.buyer_orders.as_mut().ok_or_else(|| "先打开买家席订单页".to_string())?;
    let say = match a.step {
        Step::BoListTab => {
            let v = a.arg("v")?;
            b.done_tab = v == "done";
            v
        }
        Step::BoTab => {
            let v = a.arg("v")?;
            let t = Tab::of(&v).ok_or_else(|| format!("认不出这一页签:{v}"))?;
            if let Some(o) = b.open.as_mut() {
                o.tab = t;
            }
            v
        }
        _ => {
            b.q = a.opt("q").unwrap_or_default().to_string();
            b.from = a.opt("from").unwrap_or_default().to_string();
            b.to = a.opt("to").unwrap_or_default().to_string();
            format!("{} · {} · {}", b.q, b.from, b.to)
        }
    };
    sh.desk.tr.push(a.step, crate::house::mark::FEATURE_P5, say.clone());
    Ok(say)
}

fn named(a: &Act) -> Result<(&'static str, String), String> {
    Ok(match a.step {
        Step::BoConfirmOpen => ("confirm", String::new()),
        Step::BoConfirm => ("confirmSend", String::new()),
        Step::BoConfirmHand => ("confirmHandSend", String::new()),
        Step::BoRefundOpen => ("dispute", String::new()),
        Step::BoDispute => (if a.opt("way") == Some("pull") || a.opt("form") == Some("personal") { "disputePull" } else { "disputeSend" }, String::new()),
        Step::BoEvidenceOpen => ("commit", String::new()),
        Step::BoCommit => ("commitSend", a.opt("path").unwrap_or_default().to_string()),
        Step::BoOffer => ("settle", a.arg("v")?),
        Step::BoOffersScan => ("offersScan", String::new()),
        Step::BoSettle => (if a.opt("mode") == Some("call") { "settleCall" } else { "settleSign" }, String::new()),
        Step::BoAdvance => ("advanceSend", String::new()),
        Step::BoExpire => ("expireSend", String::new()),
        Step::BoCancel => ("cancelSend", String::new()),
        Step::BoOpenCase => ("openSend", String::new()),
        Step::BoOwed => ("sweep", a.opt("token").unwrap_or_default().to_string()),
        _ => ("sweepSend", a.opt("token").unwrap_or_default().to_string()),
    })
}

fn act(sh: &mut Shell, a: &Act) -> Said {
    let (name, text) = named(a)?;
    let send = matches!(name, "confirmSend" | "confirmHandSend" | "disputeSend" | "disputePull" | "commitSend" | "settleSign" | "settleCall" | "sweepSend" | "advanceSend" | "expireSend" | "cancelSend" | "openSend");
    let home = sh.desk.sh.seller.field.home.clone();
    let data = sh.desk.data.clone();
    let dir = a.opt("dir").map(str::to_string);
    if let Some(b) = sh.desk.sh.buyer_orders.as_mut() {
        match &dir {
            Some(d) => b.export_dir = d.clone(),
            None => crate::house::orders::set_export_dir(&mut b.export_dir, &mut b.export_why, &home, &data),
        }
    }
    if send {
        let why = sh.write_why(crate::house::shell::Write::Local).map(|w| crate::house::seller::Said::bad("E_APP_WRITE", w));
        let asked = sh.desk.sh.buyer_orders.as_ref().map_or(Ok(()), |b| crate::house::asked(b.plan(&data, name, &text))).and_then(|()| why.map_or(Ok(()), Err));
        if let Err(e) = asked {
            let line = format!("{} · {}", e.name, e.detail);
            sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: e.detail.clone(), next: String::new(), detail: Some(e.name.clone()) });
            if let Some(b) = sh.desk.sh.buyer_orders.as_mut() {
                b.tell(e);
            }
            return Ok(line);
        }
        sh.desk.retry_on(crate::task::Kind::Bo, a.clone());
    }
    let tok = (!text.is_empty()).then(|| text.clone());
    let kit = a.opt("kit").map(str::to_string);
    let note = a.opt("note").unwrap_or_default().to_string();
    let path = a.opt("path").map(str::to_string);
    let site = if send { Site::Sheet("bo-act") } else { Site::Page };
    let now = sh.desk.now();
    let dhome = home.clone();
    let ask = if send { crate::house::flow::Ask::Act } else { crate::house::flow::Ask::Asked };
    let go = sh.spawn_bo_at(site, ask, move |s, data, me, mut b| {
        match &dir {
            Some(d) => b.export_dir = d.clone(),
            None => crate::house::orders::set_export_dir(&mut b.export_dir, &mut b.export_why, &dhome, data),
        }
        if send {
            if let Err(e) = crate::house::asked(b.plan(data, name, &text)) {
                b.said = Some(e);
                return b;
            }
        }
        let co = b.open_company();
        match name {
            "confirmSend" if co => b.co_act(data, me, now, |w, r| eravon_after::company::confirm(w, &r.case())),
            "disputeSend" | "disputePull" if co => {
                let form = if name == "disputeSend" { eravon_after::company::Form::Wallet } else { eravon_after::company::Form::Personal };
                let first = if kit.is_some() || path.is_some() { Some(b.co_evidence(data, me, &home, path.as_deref(), kit.as_deref(), "")) } else { None };
                match first {
                    Some(Err(e)) => b.said = Some(e),
                    first => {
                        let (ev, sealable) = first.and_then(|x| x.ok()).map_or((None, None), |(e, s)| (Some(e), s));
                        b.seal_next = sealable;
                        b.co_dispute(data, me, now, form, ev.as_ref());
                    }
                }
            }
            "commitSend" if co => match b.co_evidence(data, me, &home, path.as_deref(), kit.as_deref(), &note) {
                Ok((ev, sealable)) => {
                    b.seal_next = sealable;
                    b.co_act(data, me, now, |w, r| eravon_after::company::commit(w, &r.case(), &ev));
                }
                Err(e) => b.said = Some(e),
            },
            "settleSign" | "settleCall" if co => {
                if let Some(Ok(of)) = b.open.as_ref().and_then(|o| o.offer.clone()) {
                    b.co_act(data, me, now, |w, r| eravon_after::company::settle(w, &r.case(), &of));
                }
            }
            "sweepSend" if co => b.co_act(data, me, now, |w, r| eravon_after::company::sweep(w, &r.case(), tok.as_deref())),
            "confirm" => b.confirm_open(data, me),
            "confirmSend" => b.act(s, data, me, &crate::house::i18n::t(crate::house::i18n::Key::BoConfirmSaid), |w, r, b| eravon_after::act::confirm(w, b, &r.case())),
            "confirmHandSend" => b.confirm_hand(s, data, me, now),
            "dispute" => b.refund_open(data, me, &home),
            "disputeSend" => b.act(s, data, me, &crate::house::i18n::t(crate::house::i18n::Key::BoDisputeSaid), |w, r, b| eravon_after::act::dispute(w, b, &r.case())),
            "disputePull" => b.act(s, data, me, &crate::house::i18n::t(crate::house::i18n::Key::BoDisputeSaid), |w, r, b| eravon_after::act::dispute_pull(w, b, &r.case())),
            "commit" => b.evidence_open(data, me, &home),
            "commitSend" => b.commit(s, data, me, &home, path.as_deref(), kit.as_deref(), &note, now),
            "settle" => b.offer_paste(data, me, &text),
            "offersScan" => b.offers_scan(data, me),
            "settleSign" | "settleCall" => {
                if let Some(Ok(of)) = b.open.as_ref().and_then(|o| o.offer.clone()) {
                    let by_call = name == "settleCall";
                    b.act(s, data, me, &crate::house::i18n::t(crate::house::i18n::Key::BoSettleSaid), |w, r, b| eravon_after::act::settle(w, b, &r.case(), &of, by_call));
                }
            }
            "advanceSend" => b.act(s, data, me, &crate::house::i18n::t(crate::house::i18n::Key::BoAdvanceSaid), |w, r, b| eravon_after::act::advance(w, b, &r.case())),
            "expireSend" if co => b.co_expire(data, me, now),
            "expireSend" => b.expire(s, data, me, now),
            "cancelSend" => b.cancel(s, data, me, now),
            "openSend" if co => b.co_open(data, me, now),
            "openSend" => b.open_case(s, data, me),
            "sweep" => b.owed_open(data, me, tok.as_deref()),
            _ => b.sweep(s, data, me, tok.as_deref()),
        }
        b
    });
    sh.started(go)?;
    sh.desk.tr.push(a.step, eravon_after::mark::FEATURE, name.into());
    Ok(name.to_string())
}
