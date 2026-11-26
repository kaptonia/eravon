#![allow(unsafe_code)]

use std::path::Path;

pub fn make_executable(path: &Path) -> std::io::Result<()> {
    std::fs::metadata(path).map(|_| ())
}

pub fn at_exit(cb: extern "C" fn()) {
    unsafe extern "C" {
        fn atexit(cb: extern "C" fn()) -> std::os::raw::c_int;
    }
    // SAFETY: `atexit` takes a no-argument C callback and keeps it until exit; the caller's callback does not unwind.
    unsafe {
        atexit(cb);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[repr(C)]
struct TimeZoneInformation {
    bias: i32,
    standard_name: [u16; 32],
    standard_date: SystemTime,
    standard_bias: i32,
    daylight_name: [u16; 32],
    daylight_date: SystemTime,
    daylight_bias: i32,
}

const TIME_ZONE_ID_INVALID: u32 = 0xFFFF_FFFF;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetTimeZoneInformation(info: *mut TimeZoneInformation) -> u32;
}

pub fn zone_rules() -> Option<super::Zone> {
    if let Ok(tz) = std::env::var("TZ") {
        let tz = tz.trim().trim_start_matches(':');
        if !tz.is_empty() {
            return Some(super::Zone::Rule(tz.to_string()));
        }
    }
    let mut info = TimeZoneInformation {
        bias: 0,
        standard_name: [0; 32],
        standard_date: SystemTime::default(),
        standard_bias: 0,
        daylight_name: [0; 32],
        daylight_date: SystemTime::default(),
        daylight_bias: 0,
    };
    if unsafe { GetTimeZoneInformation(&mut info) } == TIME_ZONE_ID_INVALID {
        return None;
    }
    posix_rule(&info).map(super::Zone::Rule)
}

fn posix_rule(z: &TimeZoneInformation) -> Option<String> {
    let offset = |west: i32| {
        let (sign, m) = if west < 0 { ("-", -west) } else { ("", west) };
        if m % 60 == 0 { format!("{sign}{}", m / 60) } else { format!("{sign}{}:{:02}", m / 60, m % 60) }
    };
    let switch = |t: &SystemTime| -> Option<String> {
        let rule = t.year == 0 && (1..=12).contains(&t.month) && (1..=5).contains(&t.day) && t.day_of_week <= 6;
        rule.then(|| format!("M{}.{}.{}/{}:{:02}:{:02}", t.month, t.day, t.day_of_week, t.hour, t.minute, t.second))
    };
    let std = offset(z.bias + z.standard_bias);
    if z.daylight_date.month == 0 {
        return Some(format!("<STD>{std}"));
    }
    let dst = offset(z.bias + z.daylight_bias);
    Some(format!("<STD>{std}<DST>{dst},{},{}", switch(&z.daylight_date)?, switch(&z.standard_date)?))
}

#[cfg(test)]
mod zone_tests {
    use super::*;

    fn zone(bias: i32, std: (u16, u16, u16, u16), dst: Option<((u16, u16, u16, u16), i32)>) -> TimeZoneInformation {
        let at = |(month, day, day_of_week, hour): (u16, u16, u16, u16)| SystemTime { year: 0, month, day_of_week, day, hour, ..SystemTime::default() };
        TimeZoneInformation {
            bias,
            standard_name: [0; 32],
            standard_date: if dst.is_some() { at(std) } else { SystemTime::default() },
            standard_bias: 0,
            daylight_name: [0; 32],
            daylight_date: dst.map(|(d, _)| at(d)).unwrap_or_default(),
            daylight_bias: dst.map(|(_, b)| b).unwrap_or(0),
        }
    }

    #[test]
    fn the_system_zone_becomes_a_posix_rule() {
        assert_eq!(posix_rule(&zone(-60, (10, 5, 0, 3), Some(((3, 5, 0, 2), -60)))).as_deref(), Some("<STD>-1<DST>-2,M3.5.0/2:00:00,M10.5.0/3:00:00"));
        assert_eq!(posix_rule(&zone(300, (11, 1, 0, 2), Some(((3, 2, 0, 2), -60)))).as_deref(), Some("<STD>5<DST>4,M3.2.0/2:00:00,M11.1.0/2:00:00"));
        assert_eq!(posix_rule(&zone(-330, (0, 0, 0, 0), None)).as_deref(), Some("<STD>-5:30"));
        assert_eq!(posix_rule(&zone(-480, (0, 0, 0, 0), None)).as_deref(), Some("<STD>-8"));
        let mut once = zone(-60, (10, 5, 0, 3), Some(((3, 5, 0, 2), -60)));
        once.daylight_date.year = 2026;
        assert_eq!(posix_rule(&once), None);
    }
}

#[cfg(test)]
mod path_tests {
    const PATH_MAX_CHARS: usize = 32_767;

    #[test]
    fn a_path_that_would_grow_too_long_is_refused_and_left_as_it_was() {
        use crate::cli_path::{enable, Refused, ENV_DIR, PATH_STAND_IN};
        let d = std::env::temp_dir().join(format!("eravon-os-cli-long-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (place, app) = (d.join("place"), d.join("app"));
        std::fs::create_dir_all(&place).expect("place");
        std::fs::create_dir_all(&app).expect("app");
        let cli = app.join(format!("{}.exe", crate::cli_path::NAME));
        std::fs::write(&cli, b"MZ").expect("a command line");
        let long = format!(r"C:\{}", "a".repeat(PATH_MAX_CHARS));
        std::fs::write(place.join(PATH_STAND_IN), &long).expect("a long user path");
        unsafe { std::env::set_var(ENV_DIR, &place) };
        let r = enable(&cli);
        unsafe { std::env::remove_var(ENV_DIR) };
        assert!(matches!(r, Err(Refused::TooLong(n)) if n > PATH_MAX_CHARS), "{r:?}");
        assert_eq!(std::fs::read_to_string(place.join(PATH_STAND_IN)).expect("read"), long, "left as it was");
        let _ = std::fs::remove_dir_all(&d);
    }
}
