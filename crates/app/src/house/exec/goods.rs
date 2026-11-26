use super::{need_seller, said_of};
use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::desk::Site;
use crate::house::seller::{Goods, Leave};
use crate::house::shell::Toast;
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    Some(match a.step {
        Step::PeNew | Step::PeOpen | Step::PeSet | Step::PeSeries | Step::PeTierAdd | Step::PeTierDel | Step::PeTierSet | Step::PeCand | Step::PeCands | Step::PeTab | Step::PeSave | Step::PePublish | Step::PeImport | Step::PeImage | Step::PeImageDel | Step::PeLeave | Step::PeAsk => edit(sh, a),
        Step::GoodsSearch
        | Step::PdOpen
        | Step::PdBack
        | Step::PdCopy
        | Step::PdPreview
        | Step::PdQr
        | Step::PdQrSave
        | Step::PdQrCopy
        | Step::PdQrClose
        | Step::PdList
        | Step::PdEdit
        | Step::PdDelete
        | Step::PdExportProposal
        | Step::PdCompanyCheck => shown(sh, a),
        Step::PdPost | Step::PdRevoke => chain(sh, a),
        Step::PdFeesConfirm => fees_confirm(sh),
        _ => return None,
    })
}

fn word(k: crate::house::i18n::Key) -> String {
    crate::house::i18n::t(k)
}

fn goods(sh: &mut Shell) -> Result<&mut Goods, String> {
    need_seller(sh)?;
    sh.desk.sh.seller.goods.as_mut().ok_or_else(|| "先打开商品那一页".to_string())
}

fn save(sh: &mut Shell) -> Said {
    let (ok, draft) = (word(crate::house::i18n::Key::SettingSaved), word(crate::house::i18n::Key::GoodsDraftStored));
    let site = Site::Sheet(if sh.desk.goods_after_leave.is_some() { "goods-leave" } else { "goods-save" });
    let now = sh.desk.now();
    if !sh.spawn_goods_at(site, move |gd, data, addr, field, signer| gd.save(data, addr, field, now, signer, &ok, &draft)) {
        sh.desk.goods_after_leave = None;
    }
    sh.desk.tr.push(Step::PeSave, eravon_goods::mark::FEATURE, "保存".into());
    Ok("保存".into())
}

fn publish(sh: &mut Shell, board: bool) -> Said {
    let ok = word(if board { crate::house::i18n::Key::GoodsPublished } else { crate::house::i18n::Key::GoodsPublishedBare });
    let draft = word(crate::house::i18n::Key::GoodsSavedDraft);
    let now = sh.desk.now();
    let key = crate::house::orders::Key::of(&sh.desk.session, &sh.goods_key());
    sh.spawn_goods_at(Site::Sheet("goods-save"), move |gd, data, addr, field, signer| gd.publish(data, addr, field, now, signer, &key, board, &ok, &draft));
    let said = if board { "发布" } else { "只发布,不上公告板" };
    sh.desk.tr.push(Step::PePublish, eravon_goods::mark::FEATURE, said.into());
    Ok(said.into())
}

fn fees_confirm(sh: &mut Shell) -> Said {
    let id = goods(sh)?.shown.as_ref().map(|s| s.id.clone()).ok_or_else(|| "先打开一件商品".to_string())?;
    let now = sh.desk.now();
    let ok = word(crate::house::i18n::Key::GoodsFeesConfirmed);
    let id2 = id.clone();
    sh.spawn_goods_at(Site::Page, move |gd, data, addr, field, signer| gd.confirm_fees(data, addr, field, now, signer, &id2, &ok));
    sh.desk.tr.push(Step::PdFeesConfirm, eravon_goods::mark::FEATURE, id.clone());
    Ok(format!("确认新费用 {id}"))
}

fn chain(sh: &mut Shell, a: &Act) -> Said {
    let id = goods(sh)?.shown.as_ref().map(|s| s.id.clone()).ok_or_else(|| "先打开一件商品".to_string())?;
    if let Some(w) = sh.write_why(crate::house::shell::Write::Chain) {
        return Err(w);
    }
    let now = sh.desk.now();
    let key = crate::house::orders::Key::of(&sh.desk.session, &sh.goods_key());
    if a.step == Step::PdPost {
        let ok = word(crate::house::i18n::Key::GoodsPostedSaid);
        let id2 = id.clone();
        sh.spawn_goods_at(Site::Page, move |gd, data, addr, field, signer| gd.post(data, addr, field, now, signer, &key, &id2, &ok));
        sh.desk.tr.push(Step::PdPost, eravon_goods::mark::FEATURE, id.clone());
        return Ok(format!("上板 {id}"));
    }
    let done = word(crate::house::i18n::Key::GoodsRevokedNow);
    let at = word(crate::house::i18n::Key::OrdersProposalAt);
    let home = sh.desk.sh.seller.field.home.clone();
    let dir = a.opt("dir").map(str::to_string);
    sh.spawn_goods_at(Site::Sheet("goods-confirm"), move |gd, data, addr, _field, signer| {
        let live = || signer.live();
        let d = match dir {
            Some(d) => Ok(d),
            None => crate::house::seam::downloads(&home, data),
        };
        gd.revoke(data, addr, now, &key, &live, &done, d, &at)
    });
    sh.desk.tr.push(Step::PdRevoke, eravon_goods::mark::FEATURE, id.clone());
    Ok(format!("链上下架 {id}"))
}

fn edit(sh: &mut Shell, a: &Act) -> Said {
    let (data, addr, field, now) = (sh.desk.data.clone(), sh.goods_key(), sh.desk.sh.seller.field.clone(), sh.desk.now());
    let signer = eravon_goods::sign::Signer::of(Some(&sh.desk.session), &sh.goods_key());
    if a.step == Step::PeSave || a.step == Step::PePublish {
        if goods(sh)?.edit.is_none() {
            return Err("编辑页没开".to_string());
        }
        return if a.step == Step::PeSave {
            save(sh)
        } else {
            match a.opt("board") {
                None | Some("true") => publish(sh, true),
                Some("false") => publish(sh, false),
                Some(other) => Err(format!("board 只收 true 与 false,给的是 {other}")),
            }
        };
    }
    let gd = goods(sh)?;
    let need_edit = |gd: &Goods| -> Result<(), String> { if gd.edit.is_some() { Ok(()) } else { Err("编辑页没开".to_string()) } };
    let say = match a.step {
        Step::PeNew => {
            gd.new_one();
            word(crate::house::i18n::Key::GoodsNew)
        }
        Step::PeOpen => {
            let id = a.arg("id")?;
            gd.open_one(&data, &addr, &id);
            format!("打开 {id}")
        }
        Step::PeSet => {
            need_edit(gd)?;
            let (k, v) = (a.arg("k")?, a.arg("v")?);
            if !gd.edit.as_mut().expect("asked above").draft.set(&k, &v) {
                return Err(format!("编辑页没有这一格:{k}"));
            }
            format!("{k} 改成 {} 字", v.chars().count())
        }
        Step::PeSeries => {
            need_edit(gd)?;
            let v = a.arg("v")?;
            let known = gd.series_known();
            let d = &mut gd.edit.as_mut().expect("asked above").draft;
            match eravon_goods::series::pick(d, &v, &known) {
                Ok(eravon_goods::series::Picked::None) => "不分段".to_string(),
                Ok(eravon_goods::series::Picked::New(id)) => format!("新系列 {id}"),
                Ok(eravon_goods::series::Picked::Known(k)) => format!("系列 {} · 第 {} 段", k.id, d.stage),
                Err(eravon_goods::series::NotPicked::Full(k)) => {
                    gd.said = Some(crate::house::seller::Said::bad(eravon_goods::series::E_SERIES_JUDGE, crate::fill!(crate::house::i18n::Key::GoodsSeriesFull; k.stages.to_string())));
                    return Ok(format!("系列 {} · {}", k.id, said_of(&gd.said)));
                }
                Err(eravon_goods::series::NotPicked::Unknown(id)) => return Err(format!("本席没有这个系列:{id}")),
                Err(eravon_goods::series::NotPicked::Random(e)) => return Err(e),
            }
        }
        Step::PeImport => {
            need_edit(gd)?;
            let (k, path) = (a.arg("k")?, a.arg("path")?);
            if !matches!(k.as_str(), "description" | "deliveryNote") {
                return Err(format!("这一格不收导入:{k}"));
            }
            let e = gd.edit.as_mut().expect("asked above");
            let got = crate::house::text::import_text(std::path::Path::new(&path), e.draft.get(&k).unwrap_or_default()).map_err(|why| format!("{path} · {why}"))?;
            let carries = eravon_goods::refs::read(&got).body != got;
            if k == "description" && !carries {
                e.draft.set_body(&got);
            } else {
                e.draft.set(&k, &got);
            }
            word(crate::house::i18n::Key::TextImported)
        }
        Step::PeImage => {
            need_edit(gd)?;
            let path = a.arg("path")?;
            let bytes = std::fs::read(&path).map_err(|e| format!("读不了 {path}:{e}"))?;
            let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !gd.edit.as_mut().expect("asked above").draft.add_image(&name, &bytes) {
                return Err(format!("图名为空:{path}"));
            }
            word(crate::house::i18n::Key::GoodsImageAdded)
        }
        Step::PeImageDel => {
            need_edit(gd)?;
            let i: usize = a.arg("i")?.parse().map_err(|_| "图序要是整数".to_string())?;
            if !gd.edit.as_mut().expect("asked above").draft.remove_image(i) {
                return Err(format!("没有第 {i} 张图片"));
            }
            format!("移除第 {i} 张图片")
        }
        Step::PeTierAdd => {
            need_edit(gd)?;
            gd.edit.as_mut().expect("asked above").draft.add_tier();
            "加一档".to_string()
        }
        Step::PeTierDel => {
            need_edit(gd)?;
            let i: usize = a.arg("i")?.parse().map_err(|_| "档序要是整数".to_string())?;
            let done = gd.edit.as_mut().expect("asked above").draft.del_tier(i);
            format!("删第 {i} 档 · {}", if done { "删了" } else { "首末两档不可删" })
        }
        Step::PeTierSet => {
            need_edit(gd)?;
            let i: usize = a.arg("i")?.parse().map_err(|_| "档序要是整数".to_string())?;
            let done = gd.edit.as_mut().expect("asked above").draft.set_tier(i, a.opt("name"), a.opt("pct"));
            format!("改第 {i} 档 · {}", if done { "改了" } else { "首末两档比例锁死" })
        }
        Step::PeCand => {
            need_edit(gd)?;
            let c = a.arg("a")?;
            gd.edit.as_mut().expect("asked above").draft.toggle(&c);
            format!("勾选 {c}")
        }
        Step::PeCands => {
            need_edit(gd)?;
            let v = a.arg("v")?;
            gd.edit.as_mut().expect("asked above").draft.cands = v.split(',').map(str::trim).filter(|x| !x.is_empty()).map(|x| x.to_ascii_lowercase()).collect();
            format!("勾选表 {v}")
        }
        Step::PeTab => {
            need_edit(gd)?;
            let i: usize = a.arg("i")?.parse().map_err(|_| "页签要是 0 到 4".to_string())?;
            if i > 4 {
                return Err(format!("页签要是 0 到 4:{i}"));
            }
            gd.edit.as_mut().expect("asked above").tab = i;
            format!("页签 {i}")
        }
        Step::PeLeave => {
            need_edit(gd)?;
            let went = sh.desk.sh.go(Leave::List);
            if went {
                if let Some(g) = sh.desk.sh.seller.goods.as_mut() {
                    g.said = None;
                }
            }
            sh.desk.tr.push(Step::PeLeave, eravon_goods::mark::FEATURE, "取消".into());
            return Ok(word(if went { crate::house::i18n::Key::ModeBack } else { crate::house::i18n::Key::GoodsAskSave }));
        }
        _ => {
            let pick = a.arg("a")?;
            if pick == "save" {
                let to = gd.edit.as_ref().and_then(|e| e.ask.clone());
                if let Some(e) = gd.edit.as_mut() {
                    e.ask = None;
                }
                sh.desk.tr.push(Step::PeAsk, eravon_goods::mark::FEATURE, pick.clone());
                sh.desk.goods_after_leave = to;
                return save(sh);
            }
            let (ok, draft) = (word(crate::house::i18n::Key::SettingSaved), word(crate::house::i18n::Key::GoodsSavedDraft));
            let went = gd.answer(&data, &addr, &field, now, &signer, &pick, &ok, &draft);
            sh.desk.tr.push(Step::PeAsk, eravon_goods::mark::FEATURE, pick.clone());
            return Ok(match went {
                Some(to) => {
                    let say = format!("答 {pick} · {}", to.name());
                    follow(sh, to);
                    say
                }
                None => format!("答 {pick} · 留下"),
            });
        }
    };
    if let Some(gd) = sh.desk.sh.seller.goods.as_mut() {
        gd.mark_dirty();
    }
    sh.desk.tr.push(a.step, eravon_goods::mark::FEATURE, say.clone());
    Ok(say)
}

pub(crate) fn follow(sh: &mut Shell, to: Leave) {
    match to {
        Leave::List => {}
        Leave::Page(p) => {
            sh.go_page_now(p);
        }
        Leave::Seat(s) => {
            if s != sh.desk.seat() {
                crate::house::act::family_said_started(sh, crate::action::Action::SwitchRole);
            }
        }
        Leave::Close => sh.desk.close_asked = true,
    }
}

fn shown(sh: &mut Shell, a: &Act) -> Said {
    let (data, addr, now) = (sh.desk.data.clone(), sh.goods_key(), sh.desk.now());
    let home = sh.desk.sh.seller.field.home.clone();
    if a.step == Step::PdCompanyCheck {
        if goods(sh)?.shown.is_none() {
            return Err("先打开一件商品".to_string());
        }
        let done = word(crate::house::i18n::Key::GoodsCompanyPassedSaid);
        sh.spawn_goods(move |gd, data, addr, _field, _signer| gd.company_check(data, addr, now, &done));
        let id = sh.desk.sh.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).map(|s| s.id.clone()).unwrap_or_default();
        sh.desk.tr.push(Step::PdCompanyCheck, eravon_goods::mark::FEATURE, id);
        return Ok("核对登记".into());
    }
    let gd = goods(sh)?;
    if a.step != Step::GoodsSearch && a.step != Step::PdOpen && gd.shown.is_none() {
        return Err("先打开一件商品".to_string());
    }
    gd.said = None;
    let say = match a.step {
        Step::GoodsSearch => {
            let q = a.opt("q").unwrap_or_default().to_string();
            gd.search(&data, &addr, &q);
            sh.desk.goods_search = Some(q.clone());
            let gd = goods(sh)?;
            format!("搜索「{q}」· {} 件", gd.rows.as_ref().map_or(0, Vec::len))
        }
        Step::PdOpen => {
            let id = a.arg("id")?;
            gd.show(&data, &addr, &id, now);
            format!("商品页 {id}")
        }
        Step::PdBack => {
            gd.back();
            word(crate::house::i18n::Key::ModeBack)
        }
        Step::PdCopy => match gd.copy(&word(crate::house::i18n::Key::SaidCopied)) {
            Some(l) => format!("复制 {} 字节", l.len()),
            None => format!("没有可复制的链接 · {}", said_of(&gd.said)),
        },
        Step::PdPreview => match gd.preview() {
            Some(l) => format!("预览 · 交系统浏览器打开 {} 字节", l.len()),
            None => format!("没有可预览的链接 · {}", said_of(&gd.said)),
        },
        Step::PdQr => {
            gd.qr();
            match gd.shown.as_ref().and_then(|s| s.qr.as_ref()) {
                Some(q) if q.fits => format!("二维码 · 边长 {}", q.modules.as_ref().map_or(0, |(n, _)| *n)),
                Some(q) => format!("{} · {} / {} B", word(crate::house::i18n::Key::GoodsQrOver), q.link.len(), eravon_literals::constants::QR_BYTE_BUDGET),
                None => format!("没有二维码 · {}", said_of(&gd.said)),
            }
        }
        Step::PdQrSave => {
            if gd.shown.as_ref().and_then(|s| s.qr.as_ref()).is_none() {
                return Err("二维码卡没开".to_string());
            }
            match a.opt("path") {
                Some(p) => gd.qr_save(p, &word(crate::house::i18n::Key::QrSavedSaid)),
                None => gd.said = Some(crate::house::seller::Said::bad("E_APP_SCRIPT", crate::house::seam::QR_SAVE_NO_PATH)),
            }
            format!("另存 · {}", said_of(&gd.said))
        }
        Step::PdQrCopy => {
            if gd.shown.as_ref().and_then(|s| s.qr.as_ref()).is_none() {
                return Err("二维码卡没开".to_string());
            }
            gd.qr_copy();
            match gd.shown.as_ref().and_then(|s| s.qr.as_ref()).and_then(|q| q.image.as_ref()) {
                Some(Ok((side, _))) => format!("复制图片 · 边长 {side} 像素"),
                _ => format!("没有可复制的图片 · {}", said_of(&gd.said)),
            }
        }
        Step::PdQrClose => {
            gd.qr_close();
            "关二维码".to_string()
        }
        Step::PdList => {
            let on = a.opt("on") == Some("true");
            let w = word(if on { crate::house::i18n::Key::GoodsListedNow } else { crate::house::i18n::Key::GoodsUnlistedNow });
            gd.set_listed(&data, &addr, now, on, &w);
            format!("{} · {}", if on { "上架" } else { "下架" }, said_of(&gd.said))
        }
        Step::PdEdit => {
            let id = gd.shown.as_ref().map(|s| s.id.clone()).unwrap_or_default();
            gd.open_one(&data, &addr, &id);
            format!("编辑 {id}")
        }
        Step::PdExportProposal => {
            let dir = match a.opt("dir") {
                Some(d) => Ok(d.to_string()),
                None => crate::house::seam::downloads(&home, &data),
            };
            match dir {
                Ok(d) => gd.export_proposal(&data, &addr, &d, &word(crate::house::i18n::Key::OrdersProposalAt)),
                Err(s) => gd.said = Some(s),
            }
            format!("导出提案 · {}", said_of(&gd.said))
        }
        _ => {
            gd.delete(&data, &addr, now, &word(crate::house::i18n::Key::DraftDeletedSaid));
            format!("删除 · {}", said_of(&gd.said))
        }
    };
    let id = {
        let gd = goods(sh)?;
        gd.mark_dirty();
        if let Some(x) = gd.said.clone().filter(|_| !matches!(a.step, Step::GoodsSearch | Step::PdOpen | Step::PdBack | Step::PdQr | Step::PdQrClose | Step::PdEdit)) {
            let next = match (a.step, x.tone) {
                (Step::PdQrSave, crate::house::seller::Tone::Bad) => crate::house::refusal::next(crate::lang::lang(), &x.name).unwrap_or_default(),
                _ => String::new(),
            };
            sh.desk.sh.toast(Toast { tone: x.tone, head: x.detail.clone(), next, detail: (x.tone == crate::house::seller::Tone::Bad).then(|| x.name.clone()) });
        }
        sh.desk.sh.seller.goods.as_ref().and_then(|g| g.shown.as_ref()).map(|s| s.id.clone()).unwrap_or_else(|| a.opt("q").or(a.opt("id")).unwrap_or_default().to_string())
    };
    sh.desk.tr.push(a.step, eravon_goods::mark::FEATURE, id);
    Ok(say)
}
