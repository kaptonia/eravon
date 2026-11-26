use eravon_literals::json::Out;
use eravon_ui::{egui, metrics};

use crate::house::seller::Page;
use crate::house::shell::Seat;
use super::super::{fresh_win, Typed, Ux, Win as Window};
use crate::house::action::Action as Step;

const STEP: f64 = eravon_ui::frames::STEP;
const BODY_TEXTS: usize = 80;
const SETTLE_MAX: usize = 400;

struct Shot {
    shapes: Vec<(egui::Shape, egui::Rect)>,
    again: bool,
}

fn flatten(s: egui::Shape, clip: egui::Rect, out: &mut Vec<(egui::Shape, egui::Rect)>) {
    match s {
        egui::Shape::Vec(v) => v.into_iter().for_each(|x| flatten(x, clip, out)),
        other => out.push((other, clip)),
    }
}

fn page_name(seat: Seat, p: Option<Page>) -> &'static str {
    match (seat, p) {
        (Seat::Seller, None) => "seller.home",
        (Seat::Seller, Some(Page::Orders)) => "seller.orders",
        (Seat::Seller, Some(Page::Goods)) => "seller.goods",
        (Seat::Seller, Some(Page::Cands)) => "seller.cands",
        (Seat::Seller, Some(Page::Payways)) => "seller.payways",
        (Seat::Seller, Some(Page::Alerts)) => "seller.alerts",
        (Seat::Seller, Some(Page::Settings)) => "seller.settings",
        (Seat::Seller, Some(Page::PayLink)) => "seller.paylink",
        (Seat::Seller, Some(Page::Wallet)) => "seller.wallet",
        (Seat::Seller, Some(Page::Export)) => "seller.export",
        (Seat::Seller, Some(Page::Queue)) => "seller.queue",
        (Seat::Buyer, None) => "buyer.home",
        (Seat::Buyer, Some(Page::Orders)) => "buyer.orders",
        (Seat::Buyer, Some(Page::Goods)) => "buyer.goods",
        (Seat::Buyer, Some(Page::Cands)) => "buyer.cands",
        (Seat::Buyer, Some(Page::Payways)) => "buyer.payways",
        (Seat::Buyer, Some(Page::Alerts)) => "buyer.alerts",
        (Seat::Buyer, Some(Page::Settings)) => "buyer.settings",
        (Seat::Buyer, Some(Page::PayLink)) => "buyer.paylink",
        (Seat::Buyer, Some(Page::Wallet)) => "buyer.wallet",
        (Seat::Buyer, Some(Page::Export)) => "buyer.export",
        (Seat::Buyer, Some(Page::Queue)) => "buyer.queue",
    }
}

fn texts(shapes: &[(egui::Shape, egui::Rect)]) -> Vec<(String, egui::Rect, f32, usize)> {
    shapes
        .iter()
        .enumerate()
        .filter_map(|(i, (s, clip))| match s {
            egui::Shape::Text(t) if !t.galley.job.text.trim().is_empty() => {
                let size = t.galley.job.sections.first().map_or_else(f32::default, |x| x.format.font_id.size);
                let r = t.galley.rect.translate(t.pos.to_vec2()).intersect(*clip);
                (r.is_positive()).then(|| (t.galley.job.text.clone(), r, size, i))
            }
            _ => None,
        })
        .collect()
}

fn cards(shapes: &[(egui::Shape, egui::Rect)], min_w: f32) -> Vec<(egui::Rect, usize)> {
    shapes
        .iter()
        .enumerate()
        .filter_map(|(i, (s, _))| match s {
            egui::Shape::Rect(r) if r.blur_width <= f32::default() && r.fill.a() > 0 && r.corner_radius != egui::CornerRadius::ZERO && r.rect.width() > min_w && r.rect.is_finite() => Some((r.rect, i)),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub enum Target {
    Page(Seat, Option<Page>),
    Here,
}

pub fn read(sh: crate::shell::Shell, pages: &[Target], width: f32, dark: bool) -> Result<Out, String> {
    crate::notifier::stand_in_notifier();
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let here = pages.iter().any(|t| matches!(t, Target::Here));
    let ux = Ux { wizard_asked: true, ..Ux::default() };
    let mut w = fresh_win(sh, Typed::default(), ux);
    if here {
        if let Some(p) = w.shell.desk.goto.take() {
            use eravon_ui::nav::Pages;
            let place = Window::place_of(p);
            let landed = w.history_of(place);
            w.frame_mut().land(landed);
            w.frame_mut().place = Some(place);
            if let Some(d) = w.house_detail() {
                eravon_ui::nav::hist(&mut w).push(d);
            }
        }
    }
    w.faults_told = usize::MAX;
    ctx.options_mut(|o| o.theme_preference = if dark { egui::ThemePreference::Dark } else { egui::ThemePreference::Light });
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, eravon_ui::house::WINDOW_H));
    let mut t = 0.0_f64;
    let mut shot = |w: &mut Window| -> Shot {
        t += STEP;
        let raw = egui::RawInput { screen_rect: Some(screen), time: Some(t), ..Default::default() };
        let out = ctx.run_ui(raw, |ui| w.draw(ui));
        let again = out.viewport_output.get(&egui::ViewportId::ROOT).is_some_and(|v| v.repaint_delay < std::time::Duration::from_secs(1));
        let mut shapes = Vec::new();
        for cs in out.shapes {
            flatten(cs.shape, cs.clip_rect, &mut shapes);
        }
        std::thread::sleep(std::time::Duration::from_secs_f64(STEP));
        Shot { shapes, again }
    };
    if !here {
        settle(&mut w, &mut shot);
    }
    let bar_left = metrics::rail_panel_w();
    let mut rows: Vec<(&'static str, Out)> = Vec::new();
    for target in pages {
        let asked_before = eravon_chain::ask::asked();
        let name = match target {
            Target::Page(seat, p) => {
                w.step(Step::Seat, &[("to", seat.name())]);
                w.step(Step::Page, &[("to", p.map_or(crate::house::seam::PAGE_HOME, |x| x.name()))]);
                page_name(*seat, *p)
            }
            Target::Here => "here",
        };
        let first = shot(&mut w);
        let head_first = head_of(&first, bar_left);
        let mut first = first;
        let mut lead = 1;
        while lead < 5 && body_of(&first, bar_left).is_empty() {
            first = shot(&mut w);
            lead += 1;
        }
        let asked_first = eravon_chain::ask::asked().saturating_sub(asked_before);
        let (last, n) = settle(&mut w, &mut shot);
        let (b0, b1) = (body_of(&first, bar_left), body_of(&last, bar_left));
        let once = |v: &Vec<(String, egui::Rect, f32, usize)>, x: &str| v.iter().filter(|y| y.0 == x).count() == 1;
        let (mut dx, mut dy_min, mut dy_max, mut dsize, mut matched) = (f32::default(), f32::MAX, f32::MIN, f32::default(), 0u64);
        for (x, r0, s0, _) in b0.iter().filter(|y| once(&b0, &y.0)) {
            let near = b1.iter().find(|y| &y.0 == x && once(&b1, x));
            if let Some((_, r1, s1, _)) = near {
                matched += 1;
                dx = dx.max((r0.left() - r1.left()).abs());
                dy_min = dy_min.min(r0.top() - r1.top());
                dy_max = dy_max.max(r0.top() - r1.top());
                dsize = dsize.max((s0 - s1).abs());
            }
        }
        if matched == 0 {
            (dy_min, dy_max) = Default::default();
        }
        let lt = texts(&last.shapes);
        let all_cards = cards(&last.shapes, f32::default());
        let right = lt.iter().map(|(_, r, _, _)| r.right()).chain(all_cards.iter().map(|(c, _)| c.right())).fold(f32::default(), f32::max);
        let big = cards(&last.shapes, eravon_ui::house::CARD_MIN_W);
        let (over_card, over_text) = lt
            .iter()
            .filter_map(|(x, r, _, i)| {
                let at = egui::pos2(r.left() + eravon_ui::house::HAIRLINE, r.center().y);
                big.iter().filter(|(c, j)| j < i && c.contains(at)).max_by_key(|(_, j)| *j).map(|(c, _)| (r.right() - c.right(), x.clone()))
            })
            .fold((f32::MIN, String::new()), |a, b| if b.0 > a.0 { b } else { a });
        let over_card = if over_card == f32::MIN { f32::default() } else { over_card };
        let right_text = lt.iter().find(|(_, r, _, _)| r.right() >= right - eravon_ui::house::HAIRLINE).map(|(x, _, _, _)| x.clone()).unwrap_or_default();
        let st = structure(&ctx, &lt);
        let num = |v: f32| Out::S(format!("{v:.2}"));
        rows.push((
            name,
            Out::O(vec![
                ("overCard", num(over_card)),
                ("overCardText", Out::S(over_text)),
                ("bodyDx", num(dx)),
                ("bodyDyMax", num(dy_max)),
                ("bodyDyMin", num(dy_min)),
                ("bodyDsize", num(dsize)),
                ("bodyMatched", Out::N(matched)),
                ("framesToSettle", Out::N(n as u64)),
                ("askedBeforeFirst", Out::N(asked_first)),
                ("bodyFirst", Out::A(b0.iter().take(BODY_TEXTS).map(|(x, _, _, _)| Out::S(x.clone())).collect())),
                ("headFirst", Out::A(head_first)),
                ("headLast", Out::A(head_of(&last, bar_left))),
                ("leadFrames", Out::N(lead)),
                ("idleAgain", Out::B(last.again)),
                ("maxRight", num(right)),
                ("strays", Out::N(st.strays)),
                ("rightSpread", num(st.right_spread)),
                ("rowHeightSpread", num(st.height_spread)),
                ("keyWidthSpread", num(st.key_width_spread)),
                ("elided", Out::A(st.elided.into_iter().map(Out::S).collect())),
                ("rightRows", Out::A(st.right_rows.into_iter().map(Out::S).collect())),
                ("maxRightText", Out::S(right_text)),
                ("width", num(width)),
            ]),
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(b.0));
    Ok(Out::O(rows))
}

fn head_of(s: &Shot, bar_left: f32) -> Vec<Out> {
    texts(&s.shapes).into_iter().filter(|(_, r, _, _)| r.center().y < metrics::TOOLBAR_H && r.left() >= bar_left).map(|(x, _, _, _)| Out::S(x)).collect()
}

fn body_of(s: &Shot, bar_left: f32) -> Vec<(String, egui::Rect, f32, usize)> {
    texts(&s.shapes).into_iter().filter(|(_, r, _, _)| r.top() >= metrics::TOOLBAR_H && r.left() >= bar_left).collect()
}

fn settle(w: &mut Window, shot: &mut impl FnMut(&mut Window) -> Shot) -> (Shot, usize) {
    let mut quiet = 0;
    let mut last = shot(w);
    let mut n = 1;
    while n < SETTLE_MAX {
        if w.shell.tasks.flying().is_empty() && !w.shell.house_busy() && !last.again {
            quiet += 1;
            if quiet >= 2 {
                break;
            }
        } else {
            quiet = 0;
        }
        last = shot(w);
        n += 1;
    }
    (last, n)
}

struct Structure {
    strays: u64,
    right_spread: f32,
    height_spread: f32,
    key_width_spread: f32,
    elided: Vec<String>,
    right_rows: Vec<String>,
}

fn structure(ctx: &egui::Context, texts: &[(String, egui::Rect, f32, usize)]) -> Structure {
    let (parts, boxes) = (eravon_ui::probe::parts(ctx), eravon_ui::probe::boxes(ctx));
    let clicks: Vec<(egui::LayerId, egui::Rect)> = ctx.viewport(|v| v.prev_pass.widgets.layers().flat_map(|(layer, ws)| ws.iter().filter(|w| w.sense.senses_click() && !w.sense.senses_drag() && w.rect.is_positive()).map(move |w| (*layer, w.rect))).collect());
    let inside = |outer: &egui::Rect, r: &egui::Rect| outer.expand(eravon_ui::house::PROBE_EDGE_SLACK).contains_rect(*r);
    let strays = clicks.iter().filter(|(l, r)| boxes.iter().any(|(bl, b)| bl == l && inside(b, r)) && !parts.iter().any(|(pl, p)| pl == l && inside(p, r))).count() as u64;
    let spread = |v: &[f32]| if v.len() < 2 { 0.0 } else { v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min) };
    let (mut right_spread, mut key_width_spread) = (0.0_f32, 0.0_f32);
    let mut right_rows: Vec<String> = Vec::new();
    for (bl, b) in &boxes {
        let mut ends: Vec<f32> = Vec::new();
        let mut widths: Vec<f32> = Vec::new();
        let mut said: Vec<String> = Vec::new();
        for (pl, p) in parts.iter().filter(|(pl, p)| pl == bl && inside(b, p)) {
            let mine: Vec<&egui::Rect> = clicks.iter().filter(|(l, r)| l == pl && inside(p, r) && r.width() < p.width() * 0.9).map(|(_, r)| r).collect();
            if let Some(end) = mine.iter().map(|r| r.right()).reduce(f32::max) {
                ends.push(end);
                said.push(format!("row {:.0},{:.0}-{:.0},{:.0} end {end:.1} of {}", p.left(), p.top(), p.right(), p.bottom(), mine.len()));
            }
            widths.extend(mine.iter().filter(|r| (r.height() - eravon_ui::tokens::KEY_H).abs() < eravon_ui::house::PROBE_SAME_H_SLACK).map(|r| r.width()));
        }
        if spread(&ends) > right_spread {
            right_rows = said;
        }
        right_spread = right_spread.max(spread(&ends));
        key_width_spread = key_width_spread.max(spread(&widths));
    }
    let mut height_spread = 0.0_f32;
    for (pl, p) in &parts {
        let hs: Vec<f32> = clicks.iter().filter(|(l, r)| l == pl && inside(p, r) && r.width() < p.width() * 0.9 && r.height() > eravon_ui::house::PROBE_CLICK_MIN_H).map(|(_, r)| r.height()).collect();
        height_spread = height_spread.max(spread(&hs));
    }
    let elided = texts.iter().filter(|(x, _, _, _)| x.contains('\u{2026}')).map(|(x, _, _, _)| x.clone()).collect();
    Structure { strays, right_spread, height_spread, key_width_spread, elided, right_rows }
}
