pub use super::unix::{alert, console, downloads_dir, system_home};

pub fn open_url(url: &str) -> Result<(), String> {
    super::unix::run("xdg-open", &[url])
}

pub const COMMAND_WORD: &str = "key.commandOther";

pub const LINK_DELIVERY: &str = "argv";

pub fn arm_links() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("登记不了 eravon:// 链接:找不到本程序的位置:{e}"))?;
    let scheme = eravon_literals::literals::URL_SCHEME;
    let base = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()).map(std::path::PathBuf::from).unwrap_or_else(|| std::path::Path::new(&system_home()).join(".local").join("share"));
    let dir = base.join("applications");
    let name = format!("{scheme}-url.desktop");
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=ERAVON\nExec=\"{}\" {} --url %u\nNoDisplay=true\nTerminal=false\nMimeType=x-scheme-handler/{scheme};\n",
        exe.display().to_string().replace('\\', "\\\\").replace('"', "\\\""),
        crate::house::seam::V_WINDOW
    );
    std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join(&name), entry)).map_err(|e| format!("登记不了 eravon:// 链接:写不进 {}:{e}", dir.display()))?;
    super::unix::run("xdg-mime", &["default", &name, &format!("x-scheme-handler/{scheme}")]).map_err(|e| format!("登记不了 eravon:// 链接:{e}"))
}
