use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::seller::Page;
use crate::house::shell::{Seat, Toast};
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    Some(match a.step {
        Step::InnerBack => back(sh),
        Step::LapseOpen => lapse(sh, a),
        Step::PdDoc => doc(sh, a),
        Step::PdImageCheck => check(sh, a),
        Step::UrlOpen => url(sh, a),
        _ => return None,
    })
}

fn feature(sh: &Shell) -> eravon_literals::trace::Feature {
    match sh.desk.sh.seller.page() {
        Some(Page::Goods) => eravon_goods::mark::FEATURE,
        Some(Page::Cands) => eravon_cands::mark::FEATURE,
        Some(Page::PayLink) => crate::house::mark::FEATURE_P1,
        _ => eravon_orders::mark::FEATURE,
    }
}

fn buyer(sh: &Shell) -> bool {
    sh.desk.seat() == Seat::Buyer
}

fn back(sh: &mut Shell) -> Said {
    let left = if buyer(sh) {
        match sh.desk.sh.seller.page() {
            Some(Page::Orders) => sh.desk.sh.buyer_orders.as_mut().and_then(|b| b.inner_back()).map(str::to_string),
            Some(Page::PayLink) => sh.desk.sh.paylink.as_mut().and_then(|p| {
                p.checked = None;
                p.inner.take()
            }).map(|i| i.word().to_string()),
            _ => None,
        }
    } else {
        seller_back(sh)
    };
    let left = left.ok_or_else(|| "这一页上没有开着的内页".to_string())?;
    let f = feature(sh);
    sh.desk.tr.push(Step::InnerBack, f, left.clone());
    Ok(left)
}

fn seller_back(sh: &mut Shell) -> Option<String> {
    let d = &mut sh.desk.sh.seller;
    match d.page() {
        Some(Page::Orders) => d.orders.as_mut().filter(|o| o.inner.is_some()).map(|o| {
            o.inner_back();
            o.inner.map_or("订单", |i| i.word()).to_string()
        }),
        Some(Page::Goods) => d.goods.as_mut().and_then(|g| g.shown.as_mut()).filter(|s| s.inner.is_some()).map(|s| {
            s.inner = None;
            "商品".to_string()
        }),
        Some(Page::Cands) => d.cands.as_mut().filter(|c| c.lapse.is_some()).map(|c| {
            c.lapse = None;
            "报价".to_string()
        }),
        _ => None,
    }
}

fn lapse(sh: &mut Shell, a: &Act) -> Said {
    let who = a.opt("a").unwrap_or_default().to_string();
    if buyer(sh) {
        match sh.desk.sh.seller.page() {
            Some(Page::Orders) => sh.desk.sh.buyer_orders.as_mut().ok_or_else(|| "先打开订单那一面".to_string())?.case_lapse()?,
            Some(Page::PayLink) => sh.desk.sh.paylink.as_mut().ok_or_else(|| "先打开付款链接页".to_string())?.lapse_open(&who)?,
            _ => return Err("这一页没有辞任条款".to_string()),
        }
        let f = feature(sh);
        sh.desk.tr.push(Step::LapseOpen, f, who.clone());
        return Ok(format!("辞任条款 {who}"));
    }
    let d = &mut sh.desk.sh.seller;
    match d.page() {
        Some(Page::Orders) => d.orders.as_mut().ok_or_else(|| "先打开订单那一面".to_string())?.case_lapse()?,
        Some(Page::Goods) => {
            let s = d.goods.as_mut().and_then(|g| g.shown.as_mut()).ok_or_else(|| "先打开一件商品".to_string())?;
            if !s.lapses().iter().any(|(c, _)| c.eq_ignore_ascii_case(&who)) {
                return Err(format!("这件商品的候选里没有 {who}"));
            }
            s.inner = Some(crate::house::seller::GoodsInner::Lapse(who.clone()));
        }
        Some(Page::Cands) => d.cands.as_mut().ok_or_else(|| "先打开判官与行会那一页".to_string())?.lapse_open(&who)?,
        _ => return Err("这一页没有辞任条款".to_string()),
    }
    let f = feature(sh);
    sh.desk.tr.push(Step::LapseOpen, f, who.clone());
    Ok(format!("辞任条款 {who}"))
}

fn doc(sh: &mut Shell, a: &Act) -> Said {
    let k = a.arg("k")?;
    let goods = match k.as_str() {
        "goods" => true,
        "delivery" => false,
        other => return Err(format!("k 只有 goods 与 delivery 两样,给的是 {other}")),
    };
    if buyer(sh) {
        let p = sh.desk.sh.paylink.as_mut().filter(|p| p.verified().as_ref().is_some_and(|o| o.is_ok())).ok_or_else(|| "先打开一条付款链接".to_string())?;
        if p.segment(goods).is_none() {
            return Err(format!("这条链接的条款里没有 {k}"));
        }
        p.inner = Some(crate::house::paylink::PlInner::Doc(goods));
        p.checked = None;
        sh.desk.tr.push(Step::PdDoc, crate::house::mark::FEATURE_P1, k.clone());
        return Ok(k);
    }
    let s = sh.desk.sh.seller.goods.as_mut().and_then(|g| g.shown.as_mut()).ok_or_else(|| "先打开一件商品".to_string())?;
    s.inner = Some(crate::house::seller::GoodsInner::Doc(goods));
    s.checked = None;
    sh.desk.tr.push(Step::PdDoc, eravon_goods::mark::FEATURE, k.clone());
    Ok(k)
}

fn check(sh: &mut Shell, a: &Act) -> Said {
    let path = a.arg("path")?;
    let bytes = std::fs::read(&path).map_err(|e| format!("读不了 {path}:{e}"))?;
    let got = if buyer(sh) {
        let p = sh.desk.sh.paylink.as_mut().filter(|p| p.inner == Some(crate::house::paylink::PlInner::Doc(true))).ok_or_else(|| "先打开商品说明那一页".to_string())?;
        p.check_image(&bytes);
        p.checked.clone()
    } else {
        let s = sh.desk.sh.seller.goods.as_mut().and_then(|g| g.shown.as_mut()).filter(|s| s.inner == Some(crate::house::seller::GoodsInner::Doc(true))).ok_or_else(|| "先打开商品说明那一页".to_string())?;
        s.check_image(&bytes);
        s.checked.clone()
    };
    let say = match &got {
        Some(Ok(n)) => {
            sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Ok, head: crate::fill!(crate::house::i18n::Key::GoodsImageSame; n), next: String::new(), detail: None });
            format!("核对得上 {n}")
        }
        Some(Err(x)) => {
            let next = if x.name == "E_GOODS_IMAGE_DIFFERS" { crate::house::i18n::t(crate::house::i18n::Key::ErrEGOODSIMAGEDIFFERSNext) } else { String::new() };
            sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: x.detail.clone(), next, detail: Some(x.name.clone()) });
            format!("核对不上 · {}", x.name)
        }
        None => String::new(),
    };
    sh.desk.tr.push(Step::PdImageCheck, eravon_goods::mark::FEATURE, say.clone());
    Ok(say)
}

fn url_of(sh: &Shell, k: &str, member: Option<&str>) -> Result<String, String> {
    let d = &sh.desk.sh.seller;
    let none = || format!("这一页上没有 {k}");
    if buyer(sh) {
        let page_of = |t: &str| eravon_goods::refs::read(t).refs.page;
        let open = sh.desk.sh.buyer_orders.as_ref().and_then(|b| b.open.as_ref());
        return match (k, d.page()) {
            ("judge-home", Some(Page::Orders)) => open.and_then(|o| o.judge.as_ref()).and_then(|j| j.as_ref().ok()).and_then(|c| c.home.clone()).ok_or_else(none),
            ("guild-home", Some(Page::Orders)) => open.and_then(|o| o.complaint.as_ref()).and_then(|c| crate::house::complaint::home(c, member)).ok_or_else(none),
            ("goods-page", Some(Page::Orders)) => open.and_then(|o| o.papers.as_ref()).and_then(|p| p.goods.as_ref()).and_then(|g| g.text.as_deref()).and_then(page_of).ok_or_else(none),
            ("goods-page", Some(Page::PayLink)) => sh.desk.sh.paylink.as_ref().and_then(|p| p.segment(true)).and_then(|g| g.text).as_deref().and_then(page_of).ok_or_else(none),
            _ => Err(none()),
        };
    }
    match (k, d.page()) {
        ("judge-home", Some(Page::Orders)) => d.orders.as_ref().and_then(|o| o.judge.as_ref()).and_then(|j| j.as_ref().ok()).and_then(|c| c.home.clone()).ok_or_else(none),
        ("guild-home", Some(Page::Orders)) => d.orders.as_ref().and_then(|o| o.complaint.as_ref()).and_then(|c| crate::house::complaint::home(c, member)).ok_or_else(none),
        ("judge-home", Some(Page::Cands)) => match d.cands.as_ref().and_then(|c| c.detail.as_ref()).map(|(_, x, _)| x) {
            Some(Ok(eravon_cands::view::Detail::Judge(j))) => j.home.clone().ok_or_else(none),
            Some(Ok(eravon_cands::view::Detail::Guild(g))) => g.members.iter().find(|m| member.is_some_and(|a| m.listed.eq_ignore_ascii_case(a) || m.resolved.eq_ignore_ascii_case(a))).and_then(|m| m.home.clone()).ok_or_else(none),
            _ => Err(none()),
        },
        ("guild-home", Some(Page::Cands)) => match d.cands.as_ref().and_then(|c| c.detail.as_ref()).map(|(_, x, _)| x) {
            Some(Ok(eravon_cands::view::Detail::Guild(g))) => g.home.clone().ok_or_else(none),
            _ => Err(none()),
        },
        ("goods-page", Some(Page::Goods)) => d.goods.as_ref().and_then(|g| g.shown.as_ref()).and_then(|s| s.texts()).and_then(|(g, _)| eravon_goods::refs::read(&g).refs.page).ok_or_else(none),
        ("goods-page", Some(Page::Orders)) => d.orders.as_ref().and_then(|o| o.papers.as_ref()).and_then(|p| p.goods.as_ref()).and_then(|g| g.text.as_ref()).and_then(|t| eravon_goods::refs::read(t).refs.page).ok_or_else(none),
        _ => Err(none()),
    }
}

fn url(sh: &mut Shell, a: &Act) -> Said {
    let k = a.arg("k")?;
    let u = url_of(sh, &k, a.opt("a"))?;
    let verdict = match crate::house::platform::foreign_ok(&u) {
        Ok(()) => "ok".to_string(),
        Err(name) => {
            let head = crate::house::i18n::t(if k == "goods-page" { crate::house::i18n::Key::ErrEAPPURLSCHEME } else { crate::house::i18n::Key::UrlHomeRefused });
            sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head, next: crate::house::i18n::t(crate::house::i18n::Key::ErrEAPPURLSCHEMENext), detail: Some(name.to_string()) });
            name.to_string()
        }
    };
    sh.desk.sh.url_asked = Some((u.clone(), verdict.clone()));
    let f = feature(sh);
    sh.desk.tr.push(Step::UrlOpen, f, format!("{k} · {verdict}"));
    Ok(verdict)
}
