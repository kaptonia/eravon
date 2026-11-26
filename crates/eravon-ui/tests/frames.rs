use eravon_ui::frames::{self, Reading};

#[test]
fn class1_one_draw_path_keeps_the_check_growing() {
    let pts = frames::same_path();
    assert!(pts.iter().any(|n| *n >= 48.0), "那一圈该画满:{pts:?}");
    assert!(pts.windows(2).all(|w| w[1] >= w[0]), "点数该一路只增不减(进场终点处回落即重来):{pts:?}");
}

#[test]
fn class2_the_scroll_bar_takes_no_width() {
    let w = frames::scroll_width();
    assert!(w.iter().all(|x| (x - w[0]).abs() < 0.01), "刚好溢出连读 10 帧,可用宽该与不溢出时同:{w:?}");
}

#[test]
fn class4_a_changed_size_is_laid_out_again_in_the_same_frame() {
    let t = frames::discard();
    assert!((t[1] - t[2]).abs() < 0.01, "换内容那一帧即在最终位置(与下一帧同):{t:?}");
    assert!((t[0] - t[1]).abs() > 1.0, "换内容之前与之后该在两处:{t:?}");
}

#[test]
fn class5_a_new_page_title_is_there_from_its_first_frame() {
    let (a, words) = frames::title();
    assert!(a.iter().all(|x| (x - 1.0).abs() < 0.01), "换页后逐帧新题该满显:{a:?}");
    assert!(words.iter().all(|w| w == "Home"), "换页后只有新题:{words:?}");
}

#[test]
fn fading_never_paints_uncoloured_text_in_the_placeholder_colour() {
    let u = frames::uncoloured();
    assert!(u.iter().filter(|(a, _)| *a > 0.0 && *a < 0.99).count() >= 10, "进场与离场途中各该读到多帧(透明度在 0 与 1 之间):{u:?}");
    assert!(u.iter().all(|(_, n)| *n == 0.0), "淡入淡出途中没有一个顶点该是乘过的占位色(黄绿):{u:?}");
}

#[test]
fn class6_a_sheet_leaves_quicker_than_it_came_and_never_jumps() {
    let (i, o) = frames::leave();
    assert!(!i.is_empty() && !o.is_empty(), "进场与离场各该有帧:{i:?} {o:?}");
    assert!(o.len() < i.len(), "离场该比进场快:{i:?} {o:?}");
    assert!((o[0] - i[i.len() - 1]).abs() < 0.02, "离场从进场落定的那一处起:{i:?} {o:?}");
    assert!(o[1] > 0.9, "离场第一步不该掉一大截(掉一半以上肉眼即当「一下没了」):{o:?}");
    assert!(i.windows(2).all(|w| w[1] >= w[0] - 1e-3), "进场一路只升不降:{i:?}");
    assert!(o.windows(2).all(|w| w[1] <= w[0] + 1e-3), "离场一路只降不升:{o:?}");
}

#[test]
fn class7_an_alert_is_held_in_a_full_screen_flow_and_is_not_a_failure() {
    let (n, dots) = frames::alert();
    assert_eq!(n, vec![0.0, 1.0, 1.0], "按住时屏上零条、记下一条,放开即出:{n:?}");
    assert_ne!(dots[0], dots[1], "提醒那一枚圆不该借失败的色:{dots:?}");
}

#[test]
fn every_part_draws_idle_hovered_and_pressed() {
    for (name, counts) in frames::hover_press() {
        assert_eq!(counts.len(), 3, "{name} 三态该各画一帧");
        assert!(counts.iter().all(|c| *c > 0.0), "{name} 三态该都画得出:{counts:?}");
    }
}

#[test]
fn a_settled_page_asks_for_no_frame() {
    assert_eq!(frames::idle(), 0.0, "动效走完之后再画一帧,不该再请下一帧");
}

#[test]
fn the_rail_block_slides_on_and_stops_on_the_new_item() {
    let r = frames::rail_slide();
    let (end, ys) = (r[0], &r[1..]);
    assert!(end > 0.0 && !ys.is_empty(), "新项该在旧项之下,且换之后该有帧:{r:?}");
    assert!(ys.windows(2).all(|w| w[1] + 0.01 >= w[0]), "块的顶该一路不减:{ys:?}");
    assert!(ys.iter().any(|y| *y > 0.5 && *y < end - 0.5), "块该经过两项之间,而不是一步跳到:{ys:?}");
    assert!((ys[ys.len() - 1] - end).abs() < 0.5, "末帧该停在新项 {end}:{ys:?}");
}

#[test]
fn a_click_outside_a_menu_only_reaches_the_guard() {
    let m = frames::menu_outside();
    assert!(m.guard_took, "菜单开着时点外那一下该由守层接住:{m:?}");
    assert!(!m.under_pressed, "底下那枚键不该被按下:{m:?}");
}

#[test]
fn readings_land_in_a_file_when_asked() {
    let Ok(path) = std::env::var("ERAVON_FRAMES_OUT") else { return };
    let mut text = String::new();
    for (name, r) in frames::read_all() {
        let v = match r {
            Reading::Nums(v) => v.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(" "),
            Reading::Words(v) => v.join(" "),
            Reading::Flags(v) => v.iter().map(|(k, b)| format!("{k}={b}")).collect::<Vec<_>>().join(" "),
        };
        text.push_str(&format!("{name}\t{v}\n"));
    }
    std::fs::write(&path, text).expect("读数档写不进");
}

#[test]
fn one_esc_closes_one_layer() {
    let r = frames::esc_one();
    assert_eq!(r, vec![("menuClosed", true), ("sheetSawFirstEsc", false), ("sheetSawSecondEsc", true)]);
}

#[test]
fn a_menu_opened_again_halfway_out_comes_back_from_halfway() {
    let r = frames::reopen();
    let (open, leaving, back, fresh) = (r[0], r[1], r[2], r[3]);
    assert!(open > 0.95, "fully in before closing: {r:?}");
    assert!(leaving > 0.0 && leaving < open, "partly out while leaving: {r:?}");
    assert!(back >= leaving * 0.9, "reopened from where it was: {r:?}");
    assert!(back > fresh + 0.2, "not from nothing (a fresh menu's first frame): {r:?}");
}

#[test]
fn a_menu_with_no_room_below_opens_above() {
    let r = frames::flip_up();
    assert!(r[0] < 0.0, "near the bottom: above the anchor ({r:?})");
    assert!(r[1] > 0.0, "near the top: below the anchor ({r:?})");
}
