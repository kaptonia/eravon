use super::*;
use crate::house::i18n::{filln, Key as K};
use eravon_cands::lapse::{Case, Clocks, Lapse, Outcome};
use eravon_ui::card::Show;

fn w(k: K) -> String {
    crate::house::i18n::t(k)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::window) enum Asked {
    Tab(u8),
    Lapse(String),
    Url(&'static str),
    Text(bool),
    Export,
    CheckImage,
}

fn span(secs: u64) -> String {
    span_text(secs)
}

pub(in crate::window) fn lapse_word(l: Lapse) -> String {
    w(match l {
        Lapse::FullRefund => K::LapseFullRefund,
        Lapse::Half => K::LapseHalf,
        Lapse::ToSeller => K::LapseToSeller,
    })
}

pub(in crate::window) fn link_row(ui: &mut egui::Ui, f: &mut card::Form, title: &str, url: &str) -> bool {
    let room = (f_width(ui) * 0.55).max(eravon_ui::house::ROW_VALUE_MIN_W);
    let mut copied = None;
    let (row, _) = f.line(ui, None, title, "", false, true, c(C::Ink), |ui| {
        let (r, resp) = ui.allocate_exact_size(egui::Vec2::splat(eravon_ui::house::COPY_KEY), egui::Sense::click());
        let hot = resp.hovered();
        eravon_ui::icons::glyph_at(ui.painter(), Glyph::Copy, r.center(), eravon_ui::house::COPY_GLYPH, if hot { c(C::Ink) } else { c(C::Ink3) });
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            ui.ctx().copy_text(url.to_string());
            copied = Some(url.to_string());
        }
        let (o, _) = ui.allocate_exact_size(egui::Vec2::splat(eravon_ui::house::OPEN_MARK), egui::Sense::hover());
        paint_open(ui.painter(), o, c(C::Ink3));
        let font = Type::MonoSmall.font();
        let shown = eravon_ui::width::elide_mid(ui, url, font.clone(), room);
        let r = ui.label(egui::RichText::new(&shown).font(font).color(c(C::Ink)));
        if shown != url {
            let _ = eravon_ui::layer::tip(r, url);
        }
    });
    let pressed = row.clicked() && copied.is_none();
    tell_copied(ui.ctx(), copied);
    pressed
}

fn paint_open(p: &egui::Painter, r: egui::Rect, ink: egui::Color32) {
    let s = egui::Stroke::new(1.5_f32, ink);
    let r = r.shrink(eravon_ui::house::OPEN_MARK_INSET);
    let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
    let mid = r.width() * 0.45;
    p.line_segment([egui::pos2(l + mid, t), egui::pos2(l, t)], s);
    p.line_segment([egui::pos2(l, t), egui::pos2(l, b)], s);
    p.line_segment([egui::pos2(l, b), egui::pos2(rr, b)], s);
    p.line_segment([egui::pos2(rr, b), egui::pos2(rr, b - mid)], s);
    p.line_segment([egui::pos2(l + r.width() * 0.4, b - r.height() * 0.4), egui::pos2(rr, t)], s);
    p.line_segment([egui::pos2(rr - mid * 0.8, t), egui::pos2(rr, t)], s);
    p.line_segment([egui::pos2(rr, t), egui::pos2(rr, t + mid * 0.8)], s);
}

fn f_width(ui: &egui::Ui) -> f32 {
    ui.available_width()
}

pub(in crate::window) fn text_box(ui: &mut egui::Ui, id: &str, text: &str, max_h: f32) {
    card::form(ui, |ui, f| {
        f.free(ui, |ui| {
            eravon_ui::scroll::nested(ui, egui::ScrollArea::vertical().id_salt(("eravon-text-box", id)).max_height(max_h).auto_shrink([false, true]), |ui| {
                ui.set_width(ui.available_width());
                md_view(ui, text);
            });
        });
    });
}

pub(in crate::window) fn text_whole(ui: &mut egui::Ui, text: &str) {
    card::form(ui, |ui, f| {
        f.free(ui, |ui| md_view(ui, text));
    });
}

pub(in crate::window) fn page_tabs(ui: &mut egui::Ui, id: &str, labels: &[String], at: u8) -> Option<u8> {
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    eravon_ui::seg::tabs(ui, id, &refs, usize::from(at)).map(|i| i as u8)
}

pub(in crate::window) struct LapseRow {
    pub(in crate::window) key: String,
    pub(in crate::window) name: String,
    pub(in crate::window) guild: bool,
    pub(in crate::window) clocks: Result<Clocks, String>,
}

pub(in crate::window) fn lapse_section(ui: &mut egui::Ui, rows: &[LapseRow]) -> Option<String> {
    if rows.is_empty() {
        return None;
    }
    let mut hit = None;
    card::section(ui, &w(K::CandsQLapse), "", |ui| {
        card::form(ui, |ui, f| {
            for r in rows {
                let title = format!("{} · {}", w(if r.guild { K::CandsGuild } else { K::PlCands }), r.name);
                match &r.clocks {
                    Ok(ck) => {
                        let sub = format!("{} · {}", filln(K::LapseOfferWindow, &[&span(ck.offer_window)]), filln(K::LapseRulingWindow, &[&span(ck.ruling_window)]));
                        let word = lapse_word(ck.lapse);
                        let (resp, _) = f.line(ui, None, &title, &sub, false, true, c_ink(), |ui| {
                            ui.label(egui::RichText::new(&word).font(Type::Note.font()).color(c(C::Ink2)));
                        });
                        if resp.clicked() {
                            hit = Some(r.key.clone());
                        }
                    }
                    Err(e) => {
                        let _ = f.line(ui, None, &title, e, false, false, c_ink(), |_| ());
                    }
                }
            }
        });
    });
    hit
}

fn c_ink() -> egui::Color32 {
    c(C::Ink)
}

fn outcome_word(o: Outcome) -> String {
    match o {
        Outcome::Refund => w(K::LapseFullRefund),
        Outcome::Redraw => w(K::LapseRedraw),
        Outcome::ConfirmAsUsual => w(K::LapseGoesOn),
        Outcome::Lapse(l) => lapse_word(l),
    }
}

pub(in crate::window) fn lapse_page(ui: &mut egui::Ui, name: &str, guild: bool, ck: &Clocks) {
    Win::hero(ui, &format!("{} · {name}", w(if guild { K::CandsGuild } else { K::PlCands })), &w(K::CandsQLapse), &[]);
    let to = lapse_word(ck.lapse);
    card::section(ui, &w(K::LapseCases), "", |ui| {
        card::form(ui, |ui, f| {
            for k in Case::ALL {
                let (title, sub) = match k {
                    Case::NotTaken => (filln(K::LapseNotTaken, &[&span(ck.offer_window)]), w(if guild { K::LapseNotTakenGuildSub } else { K::LapseNotTakenSub })),
                    Case::ResignBeforeDelivery => (w(K::LapseBeforeDelivery), if guild { format!("{} · {}", w(K::LapseDepositBack), w(K::LapseNoRedraw)) } else { w(K::LapseDepositBack) }),
                    Case::ResignAfterDelivery => (w(K::LapseAfterDelivery), filln(K::LapseAfterDeliverySub, &[&to])),
                    Case::ResignInDispute => (w(K::LapseInDispute), w(K::LapseInDisputeSub)),
                    Case::NoRuling => (filln(K::LapseNoRuling, &[&span(ck.ruling_window)]), w(K::LapseNoRulingSub)),
                };
                let word = outcome_word(k.outcome(ck.lapse, guild));
                let _ = f.line(ui, None, &title, &sub, false, false, c(C::Ink), |ui| {
                    ui.label(egui::RichText::new(&word).font(Type::Note.font()).color(c(C::Ink2)));
                });
            }
        });
    });
    details_card(ui, "lapse-raw", &[("offerWindow", kv::Val::Figure(ck.offer_window.to_string())), ("rulingWindow", kv::Val::Figure(ck.ruling_window.to_string())), ("lapseDefault", kv::Val::Figure(ck.lapse.word().to_string()))]);
}

pub(in crate::window) fn text_state(on_board: bool) -> String {
    w(if on_board { K::GoodsTextOnBoard } else { K::GoodsTextDigestOnly })
}

pub(in crate::window) fn description_section(ui: &mut egui::Ui, states: [&str; 2], page: Option<&str>) -> Option<Asked> {
    let mut hit = None;
    card::section(ui, &w(K::GoodsDescribed), "", |ui| {
        card::form(ui, |ui, f| {
            if f.go(ui, &w(K::PlSegGoods), states[0], true, "").clicked() {
                hit = Some(Asked::Text(true));
            }
            if f.go(ui, &w(K::GoodsDeliveryNote), states[1], true, "").clicked() {
                hit = Some(Asked::Text(false));
            }
            if let Some(u) = page {
                if link_row(ui, f, &w(K::GoodsPage), u) {
                    hit = Some(Asked::Url("goods-page"));
                }
            }
        });
    });
    hit
}

pub(in crate::window) fn text_page(ui: &mut egui::Ui, title: &str, sub: &str, goods: bool, text: Option<&str>, digest: Option<&str>, checked: Option<&Result<String, crate::house::seller::Said>>) -> Option<Asked> {
    let mut hit = None;
    Win::hero(ui, title, sub, &[]);
    match text {
        None => {
            states::banner(ui, states::Banner::Warn, &w(K::GoodsTextNotHeld), |_| ());
        }
        Some(t) => {
            let read = eravon_goods::refs::read(t);
            card::section(ui, &w(if goods { K::PlSegGoods } else { K::GoodsDeliveryNote }), "", |ui| text_whole(ui, &read.body));
            if let (true, refs) = (goods, &read.refs) {
                if !refs.images.is_empty() {
                    card::section(ui, &w(K::GoodsImages), "", |ui| {
                        let copied = card::form(ui, |ui, f| {
                            let mut copied = None;
                            for im in &refs.images {
                                copied = f.info(ui, &im.name, Show::Copy(&im.sha256)).or(copied);
                            }
                            if f.act(ui, &w(K::GoodsImageCheck), "", key::Key::new(&w(K::PickFile), Role::Secondary)).clicked() {
                                hit = Some(Asked::CheckImage);
                            }
                            copied
                        });
                        tell_copied(ui.ctx(), copied);
                    });
                    if let Some(c) = checked {
                        match c {
                            Ok(n) => states::okline(ui, Mark::Ok, &filln(K::GoodsImageSame, &[n])),
                            Err(x) => states::banner(ui, states::Banner::Bad, &x.detail, |_| ()),
                        }
                    }
                }
                if let Some(u) = &refs.page {
                    card::section(ui, &w(K::GoodsLinkOut), "", |ui| {
                        card::form(ui, |ui, f| {
                            if link_row(ui, f, &w(K::GoodsPage), u) {
                                hit = Some(Asked::Url("goods-page"));
                            }
                        });
                    });
                }
            }
        }
    }
    if let Some(d) = digest {
        details_card(ui, if goods { "text-goods-raw" } else { "text-delivery-raw" }, &[(&w(if goods { K::CaseGoodsDigest } else { K::CaseDeliveryDigest }), kv::Val::Mono(d.to_string()))]);
    }
    hit
}

pub(in crate::window) struct Fees {
    pub(in crate::window) premium: String,
    pub(in crate::window) lf: String,
    pub(in crate::window) deposit: String,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::window) fn bench_page(
    ui: &mut egui::Ui,
    tab: u8,
    bench: Option<&Result<crate::house::case::Bench, crate::house::seller::Said>>,
    card: Option<&Result<crate::house::judge_card::Card, crate::house::seller::Said>>,
    guild_name: Option<&str>,
    fees: &Fees,
    sat: &str,
) -> Option<Asked> {
    use crate::house::case::Track;
    let mut hit = None;
    let seated = card.and_then(|c| c.as_ref().ok());
    let sat = Some(sat).filter(|a| a.trim_start_matches("0x").chars().any(|x| x != '0'));
    let guild = bench.and_then(|b| b.as_ref().ok()).and_then(|b| match &b.track {
        Track::Guild(g) => Some(g.clone()),
        Track::Named(_) => None,
    });
    let gname = guild.as_ref().map(|g| guild_name.map(str::to_string).unwrap_or_else(|| eravon_orders::table::short(g)));
    let name = seated.map(|c| c.name.clone().unwrap_or_else(|| eravon_orders::table::short(&c.address))).or_else(|| sat.map(eravon_orders::table::short));
    let (title, sub, pill) = match (&name, &gname) {
        (Some(n), Some(g)) => (n.clone(), filln(K::CaseJudgeBy, &[g]), (w(K::OrdersJudgeAccepted), eravon_ui::palette::Tone::Ok)),
        (Some(n), None) => (n.clone(), w(K::CaseJudgeTitle), (w(K::OrdersJudgeAccepted), eravon_ui::palette::Tone::Ok)),
        (None, Some(g)) => (g.clone(), filln(K::CaseNotDrawn, &[g]), (w(K::OrdersFacePendingRedraw), eravon_ui::palette::Tone::Warn)),
        (None, None) => (w(K::CaseJudgeTitle), String::new(), (w(K::OrdersFacePendingRedraw), eravon_ui::palette::Tone::Warn)),
    };
    Win::hero(ui, &title, &sub, &[hero_pill(&pill.0, pill.1)]);
    if let Some(t) = page_tabs(ui, "case-judge-tabs", &[w(K::CandsTabOverview), w(K::CandsLaw), w(K::CandsTabQuotes)], tab) {
        hit = Some(Asked::Tab(t));
    }
    match tab {
        0 => {
            card::section(ui, &w(K::GoodsBasics), "", |ui| {
                let copied = card::form(ui, |ui, f| {
                    let mut copied = None;
                    match seated {
                        Some(c) => {
                            f.info(ui, &w(K::GoodsName), Show::Text(name.as_deref().unwrap_or_default()));
                            copied = f.info(ui, &w(K::OrdersKvJudgeAddr), Show::Short(&c.address));
                            if let Some(h) = &c.home {
                                if link_row(ui, f, &w(K::CandsHome), h) {
                                    hit = Some(Asked::Url("judge-home"));
                                }
                            }
                            if let Some(g) = &gname {
                                f.info(ui, &w(K::CandsGuild), Show::Text(g));
                            }
                            f.info(ui, &w(K::JudgeRecipient), Show::Text(&crate::house::judge_card::pick_said(c)));
                        }
                        None if sat.is_some() => {
                            copied = f.info(ui, &w(K::OrdersKvJudgeAddr), Show::Short(sat.unwrap_or_default()));
                            if let Some(g) = &gname {
                                f.info(ui, &w(K::CandsGuild), Show::Text(g));
                            }
                            f.info(ui, &w(K::GoodsName), Show::Empty(&w(K::CaseJudgeCardUnread)));
                        }
                        None => {
                            if let Some(g) = &gname {
                                f.info(ui, &w(K::CandsGuild), Show::Text(g));
                            }
                            let why = card.and_then(|c| c.as_ref().err()).map(|s| crate::house::refusal::line(crate::lang::lang(), &s.name, &s.detail)).unwrap_or_else(|| w(K::ErrEJUDGENONE));
                            f.info(ui, &w(K::PlCands), Show::Empty(&why));
                        }
                    }
                    copied
                });
                tell_copied(ui.ctx(), copied);
            });
            if let Some(n) = seated.and_then(|c| c.note()) {
                card::section(ui, &w(K::JudgeNote), "", |ui| text_box(ui, "judge-note", n, eravon_ui::house::TEXT_BOX_MAX_H));
            }
        }
        1 => match bench {
            None => states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO),
            Some(Err(s)) | Some(Ok(crate::house::case::Bench { covenant: Err(s), .. })) => {
                states::err_box(ui, "case-covenant", &crate::house::refusal::line(crate::lang::lang(), &s.name, &s.detail), "", &w(K::U3RawError), &raw_of(&s.name, &s.detail));
            }
            Some(Ok(b)) => {
                if let Ok(cv) = &b.covenant {
                    let head = format!("{} · {}", w(if guild.is_some() { K::CandsGuildLaw } else { K::CaseJudgeCovenant }), w(K::CaseBound));
                    card::section(ui, &head, "", |ui| text_whole(ui, &cv.text));
                    let version = cv.version.map(|v| v.to_string()).unwrap_or_default();
                    details_card(ui, "case-covenant-raw", &[(&w(K::CaseCovenantWord), kv::Val::Mono(cv.word.clone())), (&w(K::Version), kv::Val::Figure(version)), (&w(K::CasePublisher), kv::Val::Mono(cv.publisher.clone()))]);
                }
            }
        },
        _ => match bench.map(|b| b.as_ref().map(|b| &b.quote)) {
            None => states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO),
            Some(Err(s)) | Some(Ok(Err(s))) => {
                states::err_box(ui, "case-quote", &crate::house::refusal::line(crate::lang::lang(), &s.name, &s.detail), "", &w(K::U3RawError), &raw_of(&s.name, &s.detail));
            }
            Some(Ok(Ok(q))) => {
                card::section(ui, &format!("{} · {}", w(K::CandsTabQuotes), w(K::CasePricedAtOpen)), "", |ui| {
                    card::form(ui, |ui, f| {
                        f.info(ui, &w(K::CandsQPremium), Show::Text(&fees.premium));
                        f.info(ui, &w(K::OrdersKvLf), Show::Text(&fees.lf));
                        f.info(ui, &w(K::PlSellerDeposit), Show::Text(&fees.deposit));
                        f.info(ui, &w(K::LapseOfferWindowName), Show::Text(&span(q.clocks.offer_window)));
                        f.info(ui, &w(K::CandsQRuling), Show::Text(&span(q.clocks.ruling_window)));
                        if f.go(ui, &w(K::CandsQLapse), &lapse_word(q.clocks.lapse), true, "").clicked() {
                            hit = Some(Asked::Lapse(String::new()));
                        }
                    });
                });
                details_card(ui, "case-quote-raw", &[(&w(K::CaseQuoteHash), kv::Val::Mono(q.hash.clone()))]);
            }
        },
    }
    hit
}

pub(in crate::window) fn papers_page(ui: &mut egui::Ui, tab: u8, p: Option<&crate::house::case::Papers>, item: &str, number: &str, coin: &dyn Fn(&str) -> String, names: &dyn Fn(&str) -> String) -> Option<Asked> {
    let mut hit = None;
    Win::hero(ui, item, &format!("{number} · {}", w(K::CaseDocsTitle)), &[]);
    if let Some(t) = page_tabs(ui, "case-docs-tabs", &[w(K::PlStepTerms), w(K::PlSegGoods), w(K::GoodsDeliveryNote), w(K::CaseDigests)], tab) {
        hit = Some(Asked::Tab(t));
    }
    let Some(p) = p else {
        states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO);
        return hit;
    };
    let lang = crate::lang::lang();
    match tab {
        0 => match (&p.terms, &p.points) {
            (eravon_orders::detail::Terms::Ok(text), Some(x)) => {
                card::section(ui, &w(K::CaseKeyTerms), "", |ui| {
                    let copied = card::form(ui, |ui, f| {
                        f.info(ui, &w(K::OrdersAmount), Show::Text(&coin(&x.amount)));
                        f.info(ui, &w(K::GoodsT1), Show::Text(&span(x.delivery_window)));
                        f.info(ui, &w(K::GoodsT2), Show::Text(&span(x.confirm_window)));
                        f.info(ui, &w(K::SetOpenWindow), Show::Text(&span(x.open_window)));
                        f.info(ui, &w(K::CaseBuyerMayOpen), Show::Text(&w(if x.buyer_may_open { K::Yes } else { K::CaseNo })));
                        let ladder: Vec<String> = x.ladder.iter().map(|r| format!("{} {}%", r.name, pct_of(r.buyer_bps).trim_end_matches('%'))).collect();
                        f.info(ui, &w(K::GoodsLadder), Show::Text(&ladder.join(" · ")));
                        let cands: Vec<String> = x.judges.iter().chain(x.guilds.iter()).map(|a| names(a)).collect();
                        f.info(ui, &w(K::CandsJudges), Show::Text(&cands.join(&w(K::ListJoin))));
                        let a = f.info(ui, &w(K::IdSeatAuthor), Show::Short(&x.seller_key));
                        let b = f.info(ui, &w(K::SetPayout), Show::Short(&x.seller_payout));
                        a.or(b)
                    });
                    tell_copied(ui.ctx(), copied);
                });
                card::section(ui, &w(K::PlTerms), "", |ui| {
                    card::form(ui, |ui, f| {
                        f.free(ui, |ui| {
                            ui.add(egui::Label::new(egui::RichText::new(text).font(Type::MonoSmall.font()).color(c(C::Ink2))).wrap());
                        });
                    });
                });
            }
            (eravon_orders::detail::Terms::Missing, _) => states::banner(ui, states::Banner::Warn, &w(K::BoTermsMissing), |_| ()),
            (eravon_orders::detail::Terms::Unreadable(n, d), _) => states::err_box(ui, "case-terms", &w(K::OrdersTermsUnread), "", &w(K::U3RawError), &format!("{n} · {d}")),
            (eravon_orders::detail::Terms::Mismatch(want, got), _) => {
                states::err_box(ui, "case-terms", &w(K::OrdersTermsBad), "", &w(K::U3RawError), &format!("{} {want} · {got}", w(K::OrdersTerms)));
            }
            (eravon_orders::detail::Terms::Ok(_), None) => states::err_box(ui, "case-terms", &w(K::OrdersTermsUnread), "", &w(K::U3RawError), "E_TERMS"),
        },
        1 | 2 => {
            let goods = tab == 1;
            let seg = if goods { &p.goods } else { &p.delivery };
            match seg {
                None => states::banner(ui, states::Banner::Warn, &w(K::BoTermsMissing), |_| ()),
                Some(s) => match &s.text {
                    None => states::banner(ui, states::Banner::Warn, &w(K::GoodsTextNotHeld), |_| ()),
                    Some(t) => {
                        let read = eravon_goods::refs::read(t);
                        card::section(ui, &format!("{} · {}", w(if goods { K::PlSegGoods } else { K::GoodsDeliveryNote }), text_state(s.on_board)), "", |ui| text_whole(ui, if goods { &read.body } else { t }));
                        if let (true, refs) = (goods, &read.refs) {
                            if !refs.images.is_empty() {
                                card::section(ui, &w(K::GoodsImages), "", |ui| {
                                    let copied = card::form(ui, |ui, f| {
                                        let mut copied = None;
                                        for im in &refs.images {
                                            copied = f.info(ui, &im.name, Show::Copy(&im.sha256)).or(copied);
                                        }
                                        copied
                                    });
                                    tell_copied(ui.ctx(), copied);
                                });
                            }
                            if let Some(u) = &refs.page {
                                card::section(ui, &w(K::GoodsLinkOut), "", |ui| {
                                    card::form(ui, |ui, f| {
                                        if link_row(ui, f, &w(K::GoodsPage), u) {
                                            hit = Some(Asked::Url("goods-page"));
                                        }
                                    });
                                });
                            }
                        }
                    }
                },
            }
            if let Some(s) = seg {
                details_card(ui, if goods { "case-goods-raw" } else { "case-delivery-raw" }, &[(&w(if goods { K::CaseGoodsDigest } else { K::CaseDeliveryDigest }), kv::Val::Mono(s.hash.clone()))]);
            }
        }
        _ => {
            let d = &p.digests;
            let cell = |c: &crate::house::case::Cell| match c {
                Ok(v) if v.is_empty() => kv::Val::Text(w(K::CaseByCall)),
                Ok(v) => kv::Val::Mono(v.clone()),
                Err(s) => kv::Val::Quiet(crate::house::refusal::line(lang, &s.name, &s.detail)),
            };
            let (a, b, cc, dd, e, f) = (w(K::OrdersTerms), w(K::CaseGoodsDigest), w(K::CaseDeliveryDigest), w(K::CaseCovenantPrint), w(K::CaseOfferSig), w(K::CaseBoardTx));
            let (g, h) = (w(K::OrdersKvCase), w(K::SetContractAddr));
            details_list(ui, "case-digests", &[
                (&a, cell(&d.terms)),
                (&b, cell(&d.goods)),
                (&cc, cell(&d.delivery)),
                (&dd, cell(&d.amanah)),
                (&e, cell(&d.offer_sig)),
                (&f, cell(&d.posted_tx)),
                (&g, kv::Val::Mono(d.case_id.clone())),
                (&h, kv::Val::Mono(d.instance.clone())),
            ]);
            card::form(ui, |ui, f| {
                if f.act(ui, &w(K::CaseExportAll), "", key::Key::new(&w(K::IdExportGroup), Role::Secondary)).clicked() {
                    hit = Some(Asked::Export);
                }
            });
        }
    }
    hit
}

pub(in crate::window) fn copy_mono(ui: &mut egui::Ui, v: &str) -> Option<String> {
    let (r, resp) = ui.allocate_exact_size(egui::Vec2::splat(eravon_ui::house::COPY_KEY), egui::Sense::click());
    eravon_ui::icons::glyph_at(ui.painter(), Glyph::Copy, r.center(), eravon_ui::house::COPY_GLYPH, if resp.hovered() { c(C::Ink) } else { c(C::Ink3) });
    let copied = resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked().then(|| {
        ui.ctx().copy_text(v.to_string());
        v.to_string()
    });
    let l = ui.label(egui::RichText::new(short(v)).font(Type::MonoSmall.font()).color(c(C::Ink)));
    let _ = eravon_ui::layer::tip(l, v);
    copied
}
