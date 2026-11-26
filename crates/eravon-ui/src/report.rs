use crate::metrics;
use crate::button::Role;
use crate::palette::{self, C};
use crate::tokens::Type;

pub fn metrics_rows() -> Vec<(&'static str, f32)> {
    table()
}


fn table() -> Vec<(&'static str, f32)> {
    let mut v = vec![
        ("groupMarginX", metrics::GROUP_MARGIN_X),
        ("groupMarginY", metrics::GROUP_MARGIN_Y),
        ("cardGap", metrics::CARD_GAP),
        ("pagePadX", metrics::PAGE_PAD_X),
        ("buttonGap", metrics::BUTTON_GAP),
        ("kvKeyW", metrics::KV_KEY_W),
        ("kvRowGap", metrics::KV_ROW_GAP),
        ("kvColGap", metrics::KV_COL_GAP),
        ("rowInset", metrics::ROW_INSET),
        ("textPageTitle", metrics::text(Type::Page)),
        ("textHeading", metrics::text(Type::Page)),
        ("textFigure", metrics::text(Type::Figure)),
        ("textModalTitle", metrics::text(Type::Sheet)),
        ("textCardTitle", metrics::text(Type::Card)),
        ("textBody", metrics::text(Type::Body)),
        ("textNav", metrics::text(Type::Rail)),
        ("textQuiet", metrics::text(Type::Note)),
        ("textMono", metrics::text(Type::Mono)),
        ("textSmall", metrics::text(Type::Small)),
        ("textRailNote", metrics::text(Type::Tiny)),
        ("textIndex", metrics::text(Type::Micro)),
        ("textFloor", metrics::text_floor()),
        ("countText", metrics::COUNT_TEXT),
        ("cornerField", metrics::corner_field()),
        ("cornerGroup", metrics::corner_group()),
        ("shapeTag", metrics::shape_tag()),
        ("shapeModal", metrics::shape_modal()),
        ("shapeToast", metrics::shape_toast()),
        ("shapeMenu", metrics::shape_menu()),
        ("segR", metrics::seg_r()),
        ("shapeSegmentCell", metrics::shape_segment_cell()),
        ("shapePinBox", metrics::shape_pin_box()),
        ("railPanelW", metrics::RAIL_PANEL_W),
        ("railW", metrics::RAIL_W),
        ("railItemH", metrics::RAIL_ITEM_H),
        ("railTop", metrics::RAIL_TOP),
        ("railPadX", metrics::RAIL_PAD_X),
        ("wizardLeftW", metrics::WIZARD_LEFT_W),
        ("toolbarH", metrics::TOOLBAR_H),
        ("modalW", metrics::MODAL_W),
        ("modalWideW", metrics::MODAL_WIDE_W),
        ("modalXwideW", metrics::MODAL_XWIDE_W),
        ("pinCardW", metrics::PIN_CARD_W),
        ("pinBoxW", metrics::PIN_BOX_W),
        ("pinBoxH", metrics::PIN_BOX_H),
        ("pinGap", metrics::PIN_GAP),
        ("motionFast", metrics::MOTION_FAST),
        ("motionMid", metrics::MOTION_MID),
        ("motionSlow", metrics::MOTION_SLOW),
        ("motionCycle", metrics::MOTION_CYCLE),
        ("toastLifeSeconds", metrics::TOAST_LIFE_SECONDS as f32),
        ("toastBadLifeSeconds", metrics::TOAST_BAD_LIFE_SECONDS as f32),
        ("buttonH", metrics::BUTTON_H),
        ("buttonPadX", metrics::BUTTON_PAD_X),
        ("buttonPressScale", metrics::BUTTON_PRESS_SCALE),
        ("buttonOffAlpha", metrics::BUTTON_OFF_ALPHA),
        ("inputH", metrics::INPUT_H),
        ("segH", metrics::SEG_H),
        ("toggleW", metrics::TOGGLE_W),
        ("toggleH", metrics::TOGGLE_H),
        ("badgeH", metrics::BADGE_H),
        ("tagH", metrics::TAG_H),
        ("tagMinW", metrics::TAG_MIN_W),
        ("tableHeadH", metrics::TABLE_HEAD_H),
        ("rowShort", metrics::ROW_SHORT),
        ("formRowPadX", metrics::FORM_ROW_PAD_X),
        ("rowTall", metrics::ROW_TALL),
        ("iconStroke", metrics::ICON_STROKE),
    ];
    v.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    v
}

pub fn installed_rows(ctx: &egui::Context) -> Vec<(&'static str, f32)> {
    let got = crate::skin::installed(ctx);
    let mut v = vec![
        ("textBody", got.body),
        ("textSmall", got.small),
        ("textHeading", got.heading),
        ("textMono", got.mono),
        ("cornerField", got.control_radius as f32),
    ];
    v.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    v
}

pub fn floor_ok(ctx: &egui::Context) -> bool {
    let got = crate::skin::installed(ctx);
    [got.body, got.small, got.heading, got.mono].iter().all(|s| *s >= metrics::text_floor())
}

pub fn key_rows() -> Vec<(&'static str, u32, u32, u32)> {
    Role::ALL
        .iter()
        .map(|k| {
            let f = crate::button::face(*k);
            (key_name(*k), hex(f.fill), hex(f.text), hex(f.line))
        })
        .collect()
}

pub const LOOKS: [&str; 2] = ["light", "dark"];

pub fn token_rows() -> Vec<(&'static str, &'static str, u32, u8)> {
    let mut v = Vec::new();
    for (look, table) in LOOKS.into_iter().zip([palette::LIGHT, palette::DARK]) {
        for (i, k) in TOKENS.iter().enumerate() {
            let [r, g, b, a] = table[i].to_srgba_unmultiplied();
            v.push((look, css(*k), (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b), a));
        }
    }
    v
}

pub fn look_distinct(look: &str) -> usize {
    palette::distinct(if look == "dark" { &palette::DARK } else { &palette::LIGHT })
}

fn hex(c: egui::Color32) -> u32 {
    ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | c.b() as u32
}

fn key_name(k: Role) -> &'static str {
    match k {
        Role::Primary => "primary",
        Role::Secondary => "secondary",
        Role::Guide => "entry",
        Role::Commit => "commit",
        Role::Plain => "plain",
        Role::Danger => "danger",
    }
}

const TOKENS: [C; 60] = [
    C::Ground, C::Surface, C::Rail, C::Segment, C::Hover, C::Press, C::Sel, C::Line, C::Line2, C::Ink, C::Ink2, C::Ink3,
    C::Accent, C::AccentPress, C::AccentWash, C::AccentInk, C::Focus, C::Pen, C::PenPress, C::PenEdge, C::PenWash, C::Ok,
    C::OkWash, C::OkInk, C::OkEdge, C::OkBox, C::Warn, C::WarnWash, C::WarnInk, C::WarnEdge, C::Bad, C::BadWash, C::BadInk,
    C::BadEdge, C::BadBox, C::Tag, C::Dash, C::Drop, C::DropHot, C::Skel, C::SkelHi, C::Scrim, C::Knob, C::Veil,
    C::Tone, C::ToneHover, C::TonePress, C::KeyFace, C::KeyFaceHover, C::KeyFacePress, C::KeyEdge, C::KeyEdgeHover, C::KeyDrop,
    C::KeyDrop2, C::GroundTint, C::RowHover, C::GlassTint, C::Ring, C::Hi, C::Glass,
];

fn css(k: C) -> &'static str {
    match k {
        C::Ground => "ground",
        C::Surface => "surface",
        C::Rail => "rail",
        C::Segment => "segment",
        C::Hover => "hover",
        C::Press => "press",
        C::Sel => "sel",
        C::Line => "line",
        C::Line2 => "line2",
        C::Ink => "ink",
        C::Ink2 => "ink2",
        C::Ink3 => "ink3",
        C::Accent => "accent",
        C::AccentPress => "accent-press",
        C::AccentWash => "accent-wash",
        C::AccentInk => "accent-ink",
        C::Focus => "focus",
        C::Pen => "pen",
        C::PenPress => "pen-press",
        C::PenEdge => "pen-edge",
        C::PenWash => "pen-wash",
        C::Ok => "ok",
        C::OkWash => "ok-wash",
        C::OkInk => "ok-ink",
        C::OkEdge => "ok-edge",
        C::OkBox => "ok-box",
        C::Warn => "warn",
        C::WarnWash => "warn-wash",
        C::WarnInk => "warn-ink",
        C::WarnEdge => "warn-edge",
        C::Bad => "bad",
        C::BadWash => "bad-wash",
        C::BadInk => "bad-ink",
        C::BadEdge => "bad-edge",
        C::BadBox => "bad-box",
        C::Tag => "tag",
        C::Dash => "dash",
        C::Drop => "drop",
        C::DropHot => "drop-hot",
        C::Skel => "skel",
        C::SkelHi => "skel-hi",
        C::Scrim => "scrim",
        C::Knob => "knob",
        C::Veil => "veil",
        C::Tone => "tone",
        C::ToneHover => "tone-h",
        C::TonePress => "tone-p",
        C::KeyFace => "key-face",
        C::KeyFaceHover => "key-face-h",
        C::KeyFacePress => "key-face-p",
        C::KeyEdge => "key-edge",
        C::KeyEdgeHover => "key-edge-h",
        C::KeyDrop => "key-drop",
        C::KeyDrop2 => "key-drop2",
        C::GroundTint => "ground-tint",
        C::RowHover => "row-hov",
        C::GlassTint => "glass-tint",
        C::Ring => "ring",
        C::Hi => "hi",
        C::Glass => "glass",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_and_has_no_duplicate_name() {
        let v = table();
        let mut names: Vec<&str> = v.iter().map(|x| x.0).collect();
        let n = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "名在表上该各出现一次");
        for w in v.windows(2) {
            assert!(w[0].0.as_bytes() < w[1].0.as_bytes(), "表按名的字节升序");
        }
    }

    #[test]
    fn the_sampled_numbers_read_back_as_the_kit_tables_say() {
        let t = table();
        let get = |n: &str| t.iter().find(|x| x.0 == n).unwrap_or_else(|| panic!("{n}")).1;
        for (name, want) in [
            ("railPanelW", 216.0),
            ("railW", 196.0),
            ("railItemH", 36.0),
            ("rowShort", 56.0),
            ("rowTall", 72.0),
            ("rowInset", 14.0),
            ("tableHeadH", 40.0),
            ("textFigure", 22.0),
            ("textQuiet", 14.0),
            ("textMono", 14.0),
            ("textFloor", 11.0),
            ("cornerField", 10.0),
            ("cornerGroup", 16.0),
            ("shapeMenu", 14.0),
            ("modalW", 460.0),
            ("toolbarH", 56.0),
            ("inputH", 38.0),
            ("motionFast", 0.084),
            ("iconStroke", 1.7),
        ] {
            assert!((get(name) - want).abs() < 0.01, "{name}:件库 {} 对件 6 令牌表 {want}", get(name));
        }
    }

    #[test]
    fn the_light_table_carries_the_kit_values_and_both_looks_are_printed() {
        let rows = token_rows();
        let get = |look: &str, n: &str| rows.iter().find(|x| x.0 == look && x.1 == n).unwrap_or_else(|| panic!("{look} {n}")).2;
        assert_eq!(get("light", "ink"), 0x1D1D1F);
        assert_eq!(get("light", "ink2"), 0x6E6E73);
        assert_eq!(get("light", "ink3"), 0xA1A1A6);
        assert_eq!(get("light", "ground"), 0xFAFAFC);
        assert_eq!(get("light", "surface"), 0xFFFFFF);
        assert_eq!(get("light", "accent"), 0x3B82F6);
        assert_eq!(get("light", "pen"), 0xA83A2C);
        assert_eq!(rows.len(), 2 * TOKENS.len());
        assert_ne!(get("dark", "ground"), get("light", "ground"));
    }

    #[test]
    fn the_six_key_roles_come_apart() {
        let rows = key_rows();
        assert_eq!(rows.len(), 6);
        for (i, a) in rows.iter().enumerate() {
            for b in rows.iter().skip(i + 1) {
                assert!((a.1, a.2) != (b.1, b.2), "{} 与 {} 的底与字该分得开", a.0, b.0);
            }
        }
    }
}
