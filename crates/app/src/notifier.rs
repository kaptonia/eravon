use std::path::{Path, PathBuf};

pub const FEATURE: &str = "real-notify";

pub const NOTIFY_FILE_ENV: &str = "ERAVON_NOTIFY_FILE";

pub const NOTIFY_FILE: &str = "notify.log";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notifier {
    #[cfg(feature = "real-notify")]
    Program(String),
    File(PathBuf),
}

static STAND_IN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn stand_in_notifier() {
    STAND_IN.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn notifier(data: &Path) -> Notifier {
    chosen(data, std::env::var_os(NOTIFY_FILE_ENV), STAND_IN.load(std::sync::atomic::Ordering::SeqCst))
}

fn chosen(data: &Path, named: Option<std::ffi::OsString>, stand_in: bool) -> Notifier {
    match named.filter(|p| !p.is_empty()) {
        Some(p) => Notifier::File(PathBuf::from(p)),
        None if stand_in => Notifier::File(data.join(NOTIFY_FILE)),
        #[cfg(feature = "real-notify")]
        None => Notifier::Program(program::NAME.to_string()),
        #[cfg(not(feature = "real-notify"))]
        None => Notifier::File(data.join(NOTIFY_FILE)),
    }
}

pub fn notify(to: &Notifier, title: &str, body: &str) -> Result<(), String> {
    match to {
        #[cfg(feature = "real-notify")]
        Notifier::Program(name) => program::say(name, title, body),
        Notifier::File(path) => {
            use std::io::{Seek, Write};
            let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let line = format!("{at}\t{}\t{}\n", one_line(title), one_line(body));
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            let mut o = eravon_os::Options::new();
            o.create(true).write(true);
            eravon_os::owner_only(&mut o);
            let mut f = o.open(path).map_err(|e| e.to_string())?;
            f.seek(std::io::SeekFrom::End(0)).map_err(|e| e.to_string())?;
            f.write_all(line.as_bytes()).map_err(|e| e.to_string())
        }
    }
}

fn one_line(s: &str) -> String {
    s.chars().map(|c| if matches!(c, '\n' | '\r' | '\t') { ' ' } else { c }).collect()
}

#[cfg(feature = "real-notify")]
mod program {
    #[cfg(target_os = "macos")]
    pub const NAME: &str = "osascript";
    #[cfg(all(unix, not(target_os = "macos")))]
    pub const NAME: &str = "notify-send";
    #[cfg(windows)]
    pub const NAME: &str = "powershell";

    #[cfg(windows)]
    const TOAST_TITLE: &str = "ERAVON_TOAST_TITLE";
    #[cfg(windows)]
    const TOAST_BODY: &str = "ERAVON_TOAST_BODY";

    #[cfg(target_os = "macos")]
    fn command(name: &str, title: &str, body: &str) -> std::process::Command {
        let esc = |x: &str| x.replace('\\', "\\\\").replace('"', "\\\"");
        let mut c = std::process::Command::new(name);
        c.args(["-e", &format!("display notification \"{}\" with title \"{}\"", esc(body), esc(title))]);
        c
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn command(name: &str, title: &str, body: &str) -> std::process::Command {
        let mut c = std::process::Command::new(name);
        c.args([title, body]);
        c
    }

    #[cfg(windows)]
    fn command(name: &str, title: &str, body: &str) -> std::process::Command {
        use std::os::windows::process::CommandExt;
        let script = format!(
            "$ErrorActionPreference='Stop';\
             [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null;\
             $x = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02);\
             $t = $x.GetElementsByTagName('text');\
             $null = $t.Item(0).AppendChild($x.CreateTextNode($env:{TOAST_TITLE}));\
             $null = $t.Item(1).AppendChild($x.CreateTextNode($env:{TOAST_BODY}));\
             [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\\WindowsPowerShell\\v1.0\\powershell.exe').Show([Windows.UI.Notifications.ToastNotification]::new($x))"
        );
        let mut c = std::process::Command::new(name);
        c.args(["-NoProfile", "-NonInteractive", "-Command", &script]).env(TOAST_TITLE, title).env(TOAST_BODY, body).creation_flags(0x0800_0000);
        c
    }

    pub fn say(name: &str, title: &str, body: &str) -> Result<(), String> {
        let st = eravon_os::spawn(&mut command(name, title, body)).and_then(|mut c| c.wait()).map_err(|e| format!("{name}: {e}"))?;
        if st.success() { Ok(()) } else { Err(format!("{name}: exit {:?}", st.code())) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_notifier_is_chosen_in_one_place_and_the_stand_in_writes_a_line() {
        let data = std::env::temp_dir().join(format!("eravon-app-notifier-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let named = data.join("named").join("notes.log");
        assert_eq!(chosen(&data, Some(named.clone().into()), false), Notifier::File(named.clone()), "环境变量即替身档");
        assert_eq!(chosen(&data, Some(named.clone().into()), true), Notifier::File(named.clone()));
        assert_eq!(chosen(&data, None, true), Notifier::File(data.join(NOTIFY_FILE)), "门起的进程缺省即替身档");
        let to = chosen(&data, None, true);
        notify(&to, "题\n目", "一句\t话").unwrap();
        notify(&to, "t", "b").unwrap();
        let path = match to {
            Notifier::File(path) => path,
            #[cfg(feature = "real-notify")]
            Notifier::Program(_) => unreachable!(),
        };
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("\t题 目\t一句 话"), "{}", lines[0]);
        assert!(eravon_os::is_owner_only(&path).unwrap(), "替身档只本人可读");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[cfg(not(feature = "real-notify"))]
    #[test]
    fn without_the_feature_the_default_is_the_file() {
        let data = std::path::Path::new("/d");
        assert_eq!(chosen(data, None, false), Notifier::File(data.join(NOTIFY_FILE)));
        assert_eq!(chosen(data, Some("".into()), false), Notifier::File(data.join(NOTIFY_FILE)), "空值不算指了档");
    }

    #[cfg(feature = "real-notify")]
    #[test]
    fn with_the_feature_the_default_is_the_program() {
        let data = std::path::Path::new("/d");
        assert_eq!(chosen(data, None, false), Notifier::Program(program::NAME.to_string()));
        assert_eq!(chosen(data, Some("".into()), false), Notifier::Program(program::NAME.to_string()), "空值不算指了档");
    }

    #[test]
    fn no_build_without_the_feature_can_start_a_real_notifier() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(manifest.lines().any(|l| l.trim_start().starts_with(&format!("{FEATURE} = "))), "Cargo.toml 声明 {FEATURE}");
        let words = ["osascript", "notify-send", "ToastNotificationManager"];
        let mut stack = vec![root.join("src")];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let text = std::fs::read_to_string(&p).unwrap();
                    if p.ends_with("notifier.rs") {
                        let gated = text.split("#[cfg(feature = \"real-notify\")]\nmod program {").nth(1).and_then(|t| t.split("\n#[cfg(test)]").next()).unwrap();
                        let open = text.replace(gated, "");
                        let head = open.split("\n#[cfg(test)]").next().unwrap();
                        for w in words {
                            assert!(!head.contains(w), "notifier.rs 在功能门外说了 {w}");
                        }
                    } else {
                        for w in words {
                            assert!(!text.contains(w), "{} 起真通知 {w},不经通知者", p.display());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_release_lines_build_with_the_feature() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for script in ["packaging/macos/build.sh", "packaging/linux/build.sh", "packaging/linux/cross-build.sh"] {
            let text = std::fs::read_to_string(root.join(script)).unwrap();
            let builds: Vec<&str> = text.lines().filter(|l| l.contains("build --release") && l.contains("-p app")).collect();
            assert!(!builds.is_empty() && builds.iter().all(|l| l.contains("--features \"$FEATURES\"")), "{script} 构建应用不带发布线的功能");
            let lines: Vec<&str> = text.lines().filter(|l| l.contains("FEATURES=")).collect();
            assert_eq!(lines.len(), 1, "{script} 两条发布线是同一份构建");
            assert!(lines.iter().all(|l| l.contains(&format!("app/{FEATURE}"))), "{script} 发布线不开 {FEATURE}");
        }
    }
}
