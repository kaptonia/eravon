use crate::tokens::{self, Radius, Type};

pub const GROUP_MARGIN_X: f32 = tokens::CARD_PAD_X;
pub const GROUP_MARGIN_Y: f32 = tokens::CARD_PAD_Y;
pub const CARD_GAP: f32 = tokens::CARD_GAP;
pub const PAGE_PAD_X: f32 = tokens::PAGE_PAD;
pub const S2: f32 = tokens::S2;
pub const S3: f32 = tokens::S3;

pub const KV_KEY_W: f32 = tokens::LABEL_W;
pub const KV_ROW_GAP: f32 = tokens::KV_ROW_GAP;
pub const KV_COL_GAP: f32 = tokens::KV_COL_GAP;
pub const ROW_INSET: f32 = tokens::ROW_INSET;
pub const TABLE_HEAD_H: f32 = tokens::TABLE_HEAD_H;
pub const ROW_SHORT: f32 = tokens::FORM_ROW_H;
pub const FORM_ROW_PAD_X: f32 = tokens::FORM_PAD_X;
pub const ROW_TALL: f32 = tokens::SET_ROW_H;

pub const RAIL_PANEL_W: f32 = tokens::RAIL_W;
pub const RAIL_PAD_X: f32 = tokens::RAIL_PAD;
pub const RAIL_W: f32 = tokens::RAIL_W - 2.0 * tokens::RAIL_PAD;
pub const RAIL_ITEM_H: f32 = tokens::RAIL_ITEM_H;
pub const RAIL_TOP: f32 = tokens::RAIL_TOP;
pub fn rail_panel_w() -> f32 {
    RAIL_PANEL_W
}
pub const WIZARD_LEFT_W: f32 = tokens::RAIL_W;
pub const TOOLBAR_H: f32 = tokens::TOOLBAR_H;

pub const MODAL_W: f32 = tokens::SHEET_W;
pub const MODAL_WIDE_W: f32 = tokens::SHEET_WIDE;
pub const MODAL_XWIDE_W: f32 = tokens::SHEET_XWIDE;
pub const PIN_CARD_W: f32 = tokens::SHEET_W;
pub const PIN_BOX_W: f32 = tokens::PIN_W;
pub const PIN_BOX_H: f32 = tokens::PIN_H;
pub const PIN_GAP: f32 = tokens::PIN_GAP;

pub const MOTION_FAST: f32 = tokens::FAST;
pub const MOTION_MID: f32 = tokens::MID;
pub const MOTION_SLOW: f32 = tokens::SLOW;
pub const MOTION_CYCLE: f32 = tokens::CYCLE;
pub const TOAST_LIFE_SECONDS: f64 = crate::toast::LIFE_NOTE;
pub const TOAST_BAD_LIFE_SECONDS: f64 = crate::toast::LIFE_BAD;

pub const BUTTON_H: f32 = tokens::KEY_H;
pub const BUTTON_PAD_X: f32 = tokens::KEY_PAD_X;
pub const BUTTON_PRESS_SCALE: f32 = tokens::KEY_PRESS;
pub const BUTTON_GAP: f32 = tokens::KEY_GAP;
pub const BUTTON_OFF_ALPHA: f32 = tokens::OFF;
pub const INPUT_H: f32 = tokens::INPUT_H;
pub const SEG_H: f32 = tokens::SEG_H;
pub const TOGGLE_W: f32 = tokens::SWITCH_W;
pub const TOGGLE_H: f32 = tokens::SWITCH_H;
pub const BADGE_H: f32 = tokens::PILL_H;
pub const TAG_H: f32 = tokens::TAG_H;
pub const TAG_MIN_W: f32 = tokens::TAG_MIN_W;
pub const COUNT_TEXT: f32 = tokens::COUNT_TEXT;
pub const ICON_STROKE: f32 = tokens::ICON_STROKE;

pub fn corner_field() -> f32 {
    Radius::Field.px()
}
pub fn corner_group() -> f32 {
    Radius::Card.px()
}
pub fn shape_tag() -> f32 {
    Radius::Tag.px()
}
pub fn shape_modal() -> f32 {
    Radius::Sheet.px()
}
pub fn shape_toast() -> f32 {
    Radius::Sheet.px()
}
pub fn shape_menu() -> f32 {
    Radius::Menu.px()
}
pub fn seg_r() -> f32 {
    Radius::Track.px()
}
pub fn shape_segment_cell() -> f32 {
    Radius::Thumb.px()
}
pub fn shape_pin_box() -> f32 {
    Radius::Field.px()
}

pub fn text(ty: Type) -> f32 {
    ty.size()
}
pub fn text_floor() -> f32 {
    Type::Micro.size()
}
