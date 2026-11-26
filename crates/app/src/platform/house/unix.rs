pub fn system_home() -> String {
    eravon_os::user_home().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn downloads_dir(home: &str) -> String {
    std::path::Path::new(home).join("Downloads").to_string_lossy().into_owned()
}

pub fn console() -> bool {
    true
}

pub fn alert(_text: &str) {}

pub fn run(bin: &str, args: &[&str]) -> Result<(), String> {
    let st = eravon_os::spawn(std::process::Command::new(bin).args(args)).and_then(|mut c| c.wait()).map_err(|e| crate::fill!(crate::house::i18n::Key::ErrCantStart; bin, e.to_string()))?;
    if st.success() { Ok(()) } else { Err(crate::fill!(crate::house::i18n::Key::ErrExited; bin, format!("{:?}", st.code()))) }
}
