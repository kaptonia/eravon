use std::path::{Path, PathBuf};

use eravon_literals::json::{Out, write};
use eravon_literals::literals as k7;

pub const APP_DIR: &str = "ERAVON.app";
pub const CONTENTS: &str = eravon_os::BUNDLE_CONTENTS;
pub const MACOS: &str = eravon_os::BUNDLE_CODE;
pub const RESOURCES: &str = eravon_os::BUNDLE_RESOURCES;
pub const INFO_PLIST: &str = "Info.plist";
pub const LAUNCH_JSON: &str = "launch.json";
pub const EXECUTABLE: &str = "eravon-app";

pub const CLI: &str = eravon_os::cli_path::NAME;
pub const BUNDLE_NAME: &str = eravon_os::APP_FOLDER;
pub const V_LAUNCH: &str = "eravon.app.launch/1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Laid {
    pub app: String,
    pub plist: String,
    pub exe: String,
    pub cli: String,
    pub launch: String,
    pub url_delivery: &'static str,
}

pub const PLIST_TEMPLATE: &str = include_str!("../../../../packaging/macos/Info.plist");

pub fn info_plist() -> String {
    let own = format!(
        concat!(
            "  <key>CFBundleURLTypes</key>\n",
            "  <array>\n",
            "    <dict>\n",
            "      <key>CFBundleURLName</key>\n      <string>{id}</string>\n",
            "      <key>CFBundleURLSchemes</key>\n",
            "      <array>\n        <string>{scheme}</string>\n      </array>\n",
            "    </dict>\n",
            "  </array>\n",
            "  <key>LSEnvironment</key>\n",
            "  <dict>\n",
            "    <key>{envkey}</key>\n    <string>{envval}</string>\n",
            "  </dict>\n",
            "</dict>\n</plist>"
        ),
        id = k7::BUNDLE_ID,
        scheme = k7::URL_SCHEME,
        envkey = crate::house::seam::ENV_LAUNCH,
        envval = crate::house::seam::V_WINDOW,
    );
    PLIST_TEMPLATE
        .replace("@VERSION@", env!("CARGO_PKG_VERSION"))
        .replacen("</dict>\n</plist>", &own, 1)
}

pub fn launch_doc(data: &str) -> Vec<u8> {
    write(&Out::O(vec![
        ("data", Out::S(data.to_string())),
        ("v", Out::S(V_LAUNCH.to_string())),
    ]))
}

pub fn launch_path_of(exe: &Path) -> Option<PathBuf> {
    Some(eravon_os::bundle_resources(exe.parent()?)?.join(LAUNCH_JSON))
}

pub fn data_root_from_bundle() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let p = launch_path_of(&exe)?;
    let bytes = std::fs::read(&p).ok()?;
    let v = eravon_literals::json::read(&bytes).ok()?;
    match v.get("v").and_then(|x| x.as_str()) {
        Some(got) if got == V_LAUNCH => {}
        _ => return None,
    }
    match v.get("data").and_then(|x| x.as_str()) {
        Some(d) if !d.is_empty() => Some(d.to_string()),
        _ => None,
    }
}

fn lay_file(at: &Path, bytes: &[u8]) -> Result<(), String> {
    let _ = std::fs::remove_file(at);
    eravon_store::landing::land_bytes(at, bytes).map_err(|t| format!("写不下 {}:{}", at.display(), t.said().unwrap_or(t.code())))
}

pub fn lay_out(at: &str, from: &Path, data: Option<&str>) -> Result<Laid, String> {
    let app = PathBuf::from(at).join(APP_DIR);
    let contents = app.join(CONTENTS);
    let macos = contents.join(MACOS);
    let resources = contents.join(RESOURCES);
    for d in [&contents, &macos, &resources] {
        std::fs::create_dir_all(d).map_err(|e| format!("建不出 {}:{e}", d.display()))?;
    }
    let plist = contents.join(INFO_PLIST);
    lay_file(&plist, info_plist().as_bytes())?;

    let exe = macos.join(EXECUTABLE);
    let _ = std::fs::remove_file(&exe);
    std::fs::copy(from, &exe).map_err(|e| format!("拷不进 {}:{e}", exe.display()))?;
    make_executable(&exe)?;
    let cli_from = from.with_file_name(CLI);
    if !cli_from.is_file() {
        return Err(format!("{} 旁边没有 {CLI}:命令行进不了包", from.display()));
    }
    let cli = macos.join(CLI);
    let _ = std::fs::remove_file(&cli);
    std::fs::copy(&cli_from, &cli).map_err(|e| format!("拷不进 {}:{e}", cli.display()))?;
    make_executable(&cli)?;

    let launch = match data {
        None => String::new(),
        Some(d) => {
            let p = resources.join(LAUNCH_JSON);
            lay_file(&p, &launch_doc(d))?;
            p.display().to_string()
        }
    };
    Ok(Laid {
        app: app.display().to_string(),
        plist: plist.display().to_string(),
        exe: exe.display().to_string(),
        cli: cli.display().to_string(),
        launch,
        url_delivery: crate::house::openurl::delivery(),
    })
}

fn make_executable(p: &Path) -> Result<(), String> {
    crate::house::platform::make_executable(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plist_carries_the_four_keys_the_law_names() {
        let x = info_plist();
        for k in [
            "CFBundleIdentifier",
            "CFBundleName",
            "CFBundleExecutable",
            "CFBundleURLTypes",
            "CFBundleURLSchemes",
        ] {
            assert!(x.contains(k), "plist 上该有 {k}");
        }
        assert!(x.contains(k7::BUNDLE_ID));
        assert!(x.contains(&format!("<string>{}</string>", k7::URL_SCHEME)));
        assert!(x.contains(&format!("<string>{EXECUTABLE}</string>")));
        assert!(!x.contains("@VERSION@") && x.contains(&format!("<string>{}</string>", env!("CARGO_PKG_VERSION"))));
        assert!(x.contains(&format!("<key>CFBundleName</key>\n  <string>{BUNDLE_NAME}</string>")), "the bundle name is the product's");
        assert!(PLIST_TEMPLATE.contains(&format!("<key>CFBundleIdentifier</key>\n  <string>{}</string>", k7::BUNDLE_ID)));
        assert!(x.contains(&format!("<key>{}</key>\n    <string>{}</string>", crate::house::seam::ENV_LAUNCH, crate::house::seam::V_WINDOW)));
        assert!(x.trim_end().ends_with("</dict>\n</plist>") && x.matches("<plist ").count() == 1);
    }

    #[test]
    fn the_launch_file_sits_two_levels_up_from_the_executable() {
        let exe = PathBuf::from("/x/ERAVON.app/Contents/MacOS/eravon-app");
        assert_eq!(
            launch_path_of(&exe),
            Some(PathBuf::from("/x/ERAVON.app/Contents/Resources/launch.json"))
        );
        assert_eq!(launch_path_of(&PathBuf::from("/x/target/release/eravon-app")), None);
    }
}
