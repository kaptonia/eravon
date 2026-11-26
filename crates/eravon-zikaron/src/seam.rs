use eravon_literals::literals as k7;

pub fn pointer_path(base: &str) -> String {
    std::path::Path::new(base).join(k7::ZIKARON_POINTER_FILE).to_string_lossy().into_owned()
}

pub fn default_dir(base: &str) -> String {
    std::path::Path::new(base).join(k7::ZIKARON_DEFAULT_DIR).to_string_lossy().into_owned()
}

pub fn legacy_dir(base: &str) -> String {
    k7::ZIKARON_LEGACY_DIR.split('/').fold(std::path::PathBuf::from(base), |p, s| p.join(s)).to_string_lossy().into_owned()
}

pub fn index_path(machine: &str) -> String {
    format!("{machine}/{}", k7::ZIKARON_INDEX_PATH)
}

pub fn kit_prefix() -> &'static str {
    k7::ZIKARON_KIT_PREFIX
}

pub const F_FORM: &str = "form";

pub fn form() -> &'static str {
    k7::ZIKARON_INDEX_FORM
}

pub const F_KITS: &str = "kits";

pub const F_ID: &str = "id";
pub const F_CREATED: &str = "created";
pub const F_ROOT: &str = "root";
pub const F_CONTENTS: &str = "contents";
pub const F_NOTE: &str = "note_md";
pub const F_LINK: &str = "link";
pub const F_PATH: &str = "path";

pub const F_ONE_TIME: &str = "oneTimeKey";

pub const MEDIA_DEFAULT: &str = "application/octet-stream";

pub const L_MANUAL: &str = "manual";
pub const L_POINTER: &str = "pointer";
pub const L_LEGACY: &str = "legacy";
pub const L_DEFAULT: &str = "default";
