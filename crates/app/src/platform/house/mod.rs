#[cfg(unix)]
mod unix;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as sys;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as sys;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

pub fn home_dir(injected: &str) -> String {
    if injected.is_empty() { sys::system_home() } else { injected.to_string() }
}

pub fn downloads_dir(home: &str) -> String {
    sys::downloads_dir(home)
}

pub fn app_data_dir(home: &str) -> String {
    eravon_os::app_data_dir(std::path::Path::new(home)).to_string_lossy().into_owned()
}

pub fn open_url(url: &str) -> Result<(), String> {
    sys::open_url(url)
}

pub const E_URL_SCHEME: &str = "E_APP_URL_SCHEME";

pub fn foreign_ok(url: &str) -> Result<(), &'static str> {
    let lower = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    let rest = if lower.starts_with("https://") {
        &url[8..]
    } else if lower.starts_with("http://") {
        &url[7..]
    } else {
        return Err(E_URL_SCHEME);
    };
    if rest.is_empty() || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(E_URL_SCHEME);
    }
    Ok(())
}


pub fn open_foreign(url: &str) -> Result<(), String> {
    foreign_ok(url).map_err(str::to_string)?;
    sys::open_url(url)
}

#[cfg(test)]
mod foreign_tests {
    use super::*;

    #[test]
    fn only_http_and_https_addresses_from_others_documents_open() {
        for ok in ["https://chenheng.law", "http://a.example/x?y=1#z", "HTTPS://A.EXAMPLE", "Http://a"] {
            assert_eq!(foreign_ok(ok), Ok(()), "{ok}");
        }
        for bad in [
            "",
            "https://",
            "http://",
            "http:/a.example",
            "ftp://a.example",
            "file:///Applications/Calculator.app",
            "javascript:alert(1)",
            "x-man-page://ls",
            "mailto:someone",
            " https://a.example",
            "https://a.example/a b",
            "https://a.example\n",
            "https://a.example\u{7}",
            "chenheng.law",
        ] {
            assert_eq!(foreign_ok(bad), Err(E_URL_SCHEME), "{bad:?}");
            assert_eq!(open_foreign(bad), Err(E_URL_SCHEME.to_string()), "nothing is handed to the system: {bad:?}");
        }
    }
}

pub const LINK_DELIVERY: &str = sys::LINK_DELIVERY;

pub fn arm_links() -> Result<(), String> {
    sys::arm_links()
}

pub const COMMAND_WORD: &str = sys::COMMAND_WORD;

pub fn make_executable(p: &std::path::Path) -> Result<(), String> {
    eravon_os::make_executable(p).map_err(|e| format!("设不上 {} 的权限:{e}", p.display()))
}

pub fn console() -> bool {
    sys::console()
}

pub fn alert(text: &str) {
    sys::alert(text);
}

static START: std::sync::Mutex<Option<(std::time::Instant, String)>> = std::sync::Mutex::new(None);

const START_FRESH: std::time::Duration = std::time::Duration::from_secs(2);

pub fn dialog_starts_in(dir: &str) {
    if let Ok(mut g) = START.lock() {
        *g = (!dir.is_empty()).then(|| (std::time::Instant::now(), dir.to_string()));
    }
}

pub(crate) fn take_start() -> Option<String> {
    let got = START.lock().ok()?.take()?;
    (got.0.elapsed() < START_FRESH).then_some(got.1)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn dialog_in(d: rfd::FileDialog, start: Option<String>) -> rfd::FileDialog {
    match start {
        Some(dir) => d.set_directory(dir),
        None => d,
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn start_panel(panel: *mut std::ffi::c_void) {
    if let Some(dir) = take_start() {
        macos::panel_in(panel, &dir);
    }
}

#[cfg(test)]
mod start_tests {
    #[test]
    fn a_dialog_folder_is_taken_once_and_a_stale_one_not_at_all() {
        super::dialog_starts_in("/srv/me/Downloads");
        assert_eq!(super::take_start().as_deref(), Some("/srv/me/Downloads"));
        assert_eq!(super::take_start(), None, "taken once");
        super::dialog_starts_in("");
        assert_eq!(super::take_start(), None, "empty sets nothing");
        if let Ok(mut g) = super::START.lock() {
            *g = Some((std::time::Instant::now() - super::START_FRESH - std::time::Duration::from_millis(1), "/old".into()));
        }
        assert_eq!(super::take_start(), None, "stale");
    }
}
