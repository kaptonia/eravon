use super::*;

pub(in crate::window) fn backup_restored(s: &crate::backup::Summary) -> String {
    crate::lang::filln(Key::BackupRestoredSaid, &[&s.identities.to_string(), &s.records.to_string()])
}

pub(in crate::window) fn homed(hx: &mut Hx, root: &str, writable: bool) -> Option<String> {
    if !writable {
        return None;
    }
    let same = !hx.home_seen.is_empty() && std::path::Path::new(&hx.home_seen) == std::path::Path::new(root);
    hx.home_seen = root.to_string();
    Some(t(if same { Key::HomedWriter } else { Key::HomedNew }).to_string())
}

pub(in crate::window) fn zoned(z: crate::when::Zone) -> String {
    t(match z {
        crate::when::Zone::Utc => Key::ZoneSaidUtc,
        crate::when::Zone::System => Key::ZoneSaidLocal,
    })
    .to_string()
}

pub(in crate::window) fn proxy(c: &str) -> String {
    t(match c {
        crate::machine::proxy::SYSTEM => Key::ProxySaidSystem,
        crate::machine::proxy::NONE => Key::ProxySaidOff,
        _ => Key::ProxySaidCustom,
    })
    .to_string()
}

pub(in crate::window) fn refresh_every(n: u64) -> String {
    if n == 0 { t(Key::RefreshSaidOff).to_string() } else { crate::lang::fill1(Key::RefreshSaidEvery, &n.to_string()) }
}

pub(in crate::window) fn self_checked(r: &crate::probe::Report) -> (String, Tone) {
    if r.missing.is_empty() {
        (t(Key::SaidSelfCheck).to_string(), Tone::Note)
    } else {
        (crate::lang::fill1(Key::SelfCheckShort, &r.missing.len().to_string()), Tone::Bad)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_home_writable_again_and_a_new_home_are_said_apart() {
        let mut hx = super::Hx::default();
        hx.home_seen = "/data/a".into();
        assert_eq!(super::homed(&mut hx, "/data/a", true).as_deref(), Some(super::t(super::Key::HomedWriter)));
        assert_eq!(super::homed(&mut hx, "/data/b", true).as_deref(), Some(super::t(super::Key::HomedNew)));
        assert_eq!(super::homed(&mut hx, "/data/b", false), None);
    }
}

#[cfg(test)]
mod self_check {
    #[test]
    fn the_self_check_says_all_good_only_when_nothing_is_missing() {
        let report = |missing: Vec<&'static str>| crate::probe::Report { faces: Vec::new(), missing, sink: String::new(), trace_lines: None, marks: 0 };
        let (good, tone) = super::self_checked(&report(Vec::new()));
        assert_eq!((good.as_str(), tone), (crate::lang::t(crate::lang::Key::SaidSelfCheck), super::Tone::Note));
        let (short, tone) = super::self_checked(&report(vec!["cjk", "mono"]));
        assert_eq!(short, crate::lang::fill1(crate::lang::Key::SelfCheckShort, "2"));
        assert_eq!(tone, super::Tone::Bad);
    }
}
