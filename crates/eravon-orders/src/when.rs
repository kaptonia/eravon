
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone {
    Utc,
    System,
}

impl Zone {
    pub const ALL: [Zone; 2] = [Zone::Utc, Zone::System];

    pub fn as_str(self) -> &'static str {
        match self {
            Zone::Utc => "utc",
            Zone::System => "system",
        }
    }

    pub fn parse(s: &str) -> Option<Zone> {
        Zone::ALL.iter().copied().find(|z| z.as_str() == s)
    }
}

pub const BORN: Zone = Zone::Utc;

static ZONE: AtomicU8 = AtomicU8::new(0);

static SYSTEM: OnceLock<Option<Rules>> = OnceLock::new();

pub fn zone() -> Zone {
    if ZONE.load(Ordering::Relaxed) == 0 { Zone::Utc } else { Zone::System }
}

pub fn set(z: Zone) {
    if z == Zone::System {
        SYSTEM.get_or_init(Rules::of_system);
    }
    ZONE.store(if z == Zone::Utc { 0 } else { 1 }, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Probe {
    Offset(i64),
    Unread,
}

pub fn inject(p: Probe) -> bool {
    let r = match p {
        Probe::Offset(o) => Some(Rules { transitions: Vec::new(), initial: o, footer: None }),
        Probe::Unread => None,
    };
    SYSTEM.set(r).is_ok()
}

pub fn system_read() -> bool {
    SYSTEM.get().map(|r| r.is_some()).unwrap_or(false)
}

pub fn shows_utc() -> bool {
    zone() == Zone::Utc || !system_read()
}

pub fn offset_at(unix_secs: u64) -> i64 {
    match (zone(), SYSTEM.get()) {
        (Zone::System, Some(Some(r))) => r.offset(unix_secs as i64),
        _ => 0,
    }
}


pub fn when(unix_secs: u64) -> String {
    write(unix_secs, offset_at(unix_secs), 19)
}

pub fn minute(unix_secs: u64) -> String {
    write(unix_secs, offset_at(unix_secs), 16)
}

pub fn day(unix_secs: u64) -> String {
    write(unix_secs, offset_at(unix_secs), 10)
}

pub fn short(unix_secs: u64) -> String {
    minute(unix_secs).chars().skip(5).collect()
}

pub fn hm(unix_secs: u64) -> String {
    minute(unix_secs).chars().skip(11).collect()
}

pub fn clock_or_day(then: u64, now: u64) -> String {
    if day(then) == day(now) { hm(then) } else { short(then) }
}

pub fn within(at: u64, from: Option<&str>, to: Option<&str>) -> bool {
    within_at(at, from, to, offset_at(at))
}

pub fn within_at(at: u64, from: Option<&str>, to: Option<&str>, offset: i64) -> bool {
    if from.is_none() && to.is_none() {
        return true;
    }
    if at == 0 {
        return false;
    }
    let d = write(at, offset, 10);
    from.is_none_or(|f| d.as_str() >= f) && to.is_none_or(|t| d.as_str() <= t)
}

pub fn write(unix_secs: u64, offset: i64, len: usize) -> String {
    if unix_secs == 0 {
        return String::new();
    }
    let t = (unix_secs as i64).saturating_add(offset).max(0) as u64;
    let (y, mo, d, h, mi, s) = civil(t);
    let full = format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}");
    full.chars().take(len).collect()
}

pub fn civil(t: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (t / 86_400) as i64;
    let rem = t % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, (rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn month_len(y: i64, m: u32) -> u32 {
    match m {
        2 if leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[derive(Clone, Debug)]
struct Rules {
    transitions: Vec<(i64, i64)>,
    initial: i64,
    footer: Option<Posix>,
}

impl Rules {
    fn of_system() -> Option<Rules> {
        match eravon_os::zone_rules()? {
            eravon_os::Zone::File(b) => Rules::parse(&b),
            eravon_os::Zone::Rule(r) => Posix::parse(&r).map(|p| Rules { transitions: Vec::new(), initial: p.std, footer: Some(p) }),
            eravon_os::Zone::ByYear(list) => list.last().and_then(|(_, r)| Posix::parse(r)).map(|p| Rules { transitions: Vec::new(), initial: p.std, footer: Some(p) }),
        }
    }

    fn parse(b: &[u8]) -> Option<Rules> {
        if b.len() < 44 || &b[0..4] != b"TZif" {
            return None;
        }
        let version = b[4];
        let (block, rest) = Rules::block(b, 4)?;
        if version < b'2' {
            return Some(block);
        }
        let (mut block, rest) = Rules::block(rest, 8)?;
        let footer = rest.strip_prefix(b"\n").and_then(|f| f.split(|c| *c == b'\n').next()).and_then(|f| std::str::from_utf8(f).ok()).and_then(Posix::parse);
        block.footer = footer;
        Some(block)
    }

    fn block(b: &[u8], width: usize) -> Option<(Rules, &[u8])> {
        if b.len() < 44 || &b[0..4] != b"TZif" {
            return None;
        }
        let n = |i: usize| u32::from_be_bytes([b[20 + i * 4], b[21 + i * 4], b[22 + i * 4], b[23 + i * 4]]) as usize;
        let (isut, isstd, leaps, times, types, chars) = (n(0), n(1), n(2), n(3), n(4), n(5));
        let data = &b[44..];
        let len = times * width + times + types * 6 + chars + leaps * (width + 4) + isstd + isut;
        if data.len() < len || types == 0 {
            return None;
        }
        let at = |i: usize| -> i64 {
            let s = &data[i * width..(i + 1) * width];
            if width == 8 {
                i64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
            } else {
                i32::from_be_bytes([s[0], s[1], s[2], s[3]]) as i64
            }
        };
        let idx = &data[times * width..times * width + times];
        let info = &data[times * width + times..];
        let utoff = |t: usize| -> Option<i64> {
            let s = info.get(t * 6..t * 6 + 4)?;
            Some(i32::from_be_bytes([s[0], s[1], s[2], s[3]]) as i64)
        };
        let mut transitions = Vec::with_capacity(times);
        for i in 0..times {
            transitions.push((at(i), utoff(idx[i] as usize)?));
        }
        let initial = (0..types).find(|t| info.get(t * 6 + 4) == Some(&0)).and_then(utoff).or_else(|| utoff(0))?;
        Some((Rules { transitions, initial, footer: None }, &data[len..]))
    }

    fn offset(&self, t: i64) -> i64 {
        if let Some(p) = &self.footer {
            if self.transitions.last().is_none_or(|(last, _)| t >= *last) {
                return p.offset(t);
            }
        }
        match self.transitions.partition_point(|(at, _)| *at <= t) {
            0 => self.initial,
            i => self.transitions[i - 1].1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Posix {
    std: i64,
    dst: Option<(i64, Date, Date)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Date {
    Month(u32, u32, u32, i64),
    Julian(u32, i64),
    Day(u32, i64),
}

impl Posix {
    fn parse(s: &str) -> Option<Posix> {
        let mut p = Cursor { s: s.as_bytes(), i: 0 };
        p.name()?;
        let std = -p.offset()?;
        if p.done() {
            return Some(Posix { std, dst: None });
        }
        p.name()?;
        let dst = if p.peek() == Some(b',') { std + 3600 } else { -p.offset()? };
        if !p.eat(b',') {
            return None;
        }
        let start = p.date()?;
        if !p.eat(b',') {
            return None;
        }
        let end = p.date()?;
        p.done().then_some(Posix { std, dst: Some((dst, start, end)) })
    }

    fn offset(&self, t: i64) -> i64 {
        let Some((dst, start, end)) = self.dst else { return self.std };
        let (y, ..) = civil(t.max(0) as u64);
        let on = start.local(y) - self.std;
        let off = end.local(y) - dst;
        let in_dst = if on < off { t >= on && t < off } else { !(t >= off && t < on) };
        if in_dst {
            dst
        } else {
            self.std
        }
    }
}

impl Date {
    fn local(self, y: i64) -> i64 {
        let (days, at) = match self {
            Date::Month(m, w, d, at) => {
                let first = days_from_civil(y, m, 1);
                let wd = (first + 4).rem_euclid(7) as u32;
                let mut day = 1 + (d + 7 - wd) % 7 + (w - 1) * 7;
                while day > month_len(y, m) {
                    day -= 7;
                }
                (days_from_civil(y, m, day), at)
            }
            Date::Julian(n, at) => {
                let n = n as i64;
                let skip = if leap(y) && n >= 60 { 1 } else { 0 };
                (days_from_civil(y, 1, 1) + n - 1 + skip, at)
            }
            Date::Day(n, at) => (days_from_civil(y, 1, 1) + n as i64, at),
        };
        days * 86_400 + at
    }
}

struct Cursor<'a> {
    s: &'a [u8],
    i: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn done(&self) -> bool {
        self.i == self.s.len()
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn name(&mut self) -> Option<()> {
        if self.eat(b'<') {
            while self.peek()? != b'>' {
                self.i += 1;
            }
            self.i += 1;
            return Some(());
        }
        let from = self.i;
        while self.peek().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
            self.i += 1;
        }
        (self.i - from >= 3).then_some(())
    }

    fn number(&mut self) -> Option<i64> {
        let from = self.i;
        while self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[from..self.i]).ok()?.parse().ok()
    }

    fn offset(&mut self) -> Option<i64> {
        let sign = if self.eat(b'-') {
            -1
        } else {
            self.eat(b'+');
            1
        };
        let mut secs = Some(self.number()?).filter(|h| *h <= 167)? * 3600;
        if self.eat(b':') {
            secs += Some(self.number()?).filter(|m| *m <= 59)? * 60;
            if self.eat(b':') {
                secs += Some(self.number()?).filter(|s| *s <= 59)?;
            }
        }
        Some(sign * secs)
    }

    fn date(&mut self) -> Option<Date> {
        let d = if self.eat(b'M') {
            let m = self.number()?;
            self.eat(b'.').then_some(())?;
            let w = self.number()?;
            self.eat(b'.').then_some(())?;
            let d = self.number()?;
            ((1..=12).contains(&m) && (1..=5).contains(&w) && (0..=6).contains(&d)).then_some(Date::Month(m as u32, w as u32, d as u32, 0))?
        } else if self.eat(b'J') {
            let n = self.number()?;
            (1..=365).contains(&n).then_some(Date::Julian(n as u32, 0))?
        } else {
            let n = self.number()?;
            (0..=365).contains(&n).then_some(Date::Day(n as u32, 0))?
        };
        let at = if self.eat(b'/') { self.offset()? } else { 7200 };
        Some(match d {
            Date::Month(m, w, dd, _) => Date::Month(m, w, dd, at),
            Date::Julian(n, _) => Date::Julian(n, at),
            Date::Day(n, _) => Date::Day(n, at),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUMMER: i64 = 1_790_293_572;
    const WINTER: i64 = 1_798_761_600;

    #[test]
    fn a_moment_is_written_in_the_offset_given_and_zero_is_empty() {
        assert_eq!(write(SUMMER as u64, 0, 19), "2026-09-24 23:46:12");
        assert_eq!(write(SUMMER as u64, 8 * 3600, 19), "2026-09-25 07:46:12");
        assert_eq!(write(SUMMER as u64, -4 * 3600, 16), "2026-09-24 19:46");
        assert_eq!(write(0, 8 * 3600, 19), "");
    }

    #[test]
    fn a_date_range_is_cut_by_the_day_in_the_zone() {
        let t = SUMMER as u64;
        assert!(within_at(t, Some("2026-09-24"), Some("2026-09-24"), 0));
        assert!(!within_at(t, Some("2026-09-24"), Some("2026-09-24"), 8 * 3600));
        assert!(within_at(t, Some("2026-09-25"), Some("2026-09-25"), 8 * 3600));
        assert!(within_at(t, None, None, 8 * 3600));
        assert!(!within_at(0, Some("2026-01-01"), None, 0));
        assert!(within_at(0, None, None, 0));
    }

    #[test]
    fn posix_rules_give_daylight_offsets_on_both_hemispheres() {
        let ny = Posix::parse("EST5EDT,M3.2.0,M11.1.0").expect("rule");
        assert_eq!(ny.offset(SUMMER), -4 * 3600);
        assert_eq!(ny.offset(WINTER), -5 * 3600);
        let sydney = Posix::parse("AEST-10AEDT,M10.1.0,M4.1.0/3").expect("rule");
        assert_eq!(sydney.offset(SUMMER), 10 * 3600);
        assert_eq!(sydney.offset(WINTER), 11 * 3600);
        let fixed = Posix::parse("<+08>-8").expect("rule");
        assert_eq!(fixed.offset(SUMMER), 8 * 3600);
    }

    #[test]
    fn a_zone_given_only_as_a_rule_follows_its_daylight_time() {
        let p = Posix::parse("EST5EDT,M3.2.0,M11.1.0").expect("rule");
        let only_rule = Rules { transitions: Vec::new(), initial: p.std, footer: Some(p) };
        assert_eq!(only_rule.offset(SUMMER), -4 * 3600);
        assert_eq!(only_rule.offset(WINTER), -5 * 3600);
    }

    #[test]
    fn an_offset_out_of_range_is_no_rule() {
        assert_eq!(Posix::parse("AAA9999999999999999"), None);
        assert_eq!(Posix::parse("AAA168"), None);
        assert_eq!(Posix::parse("AAA5:60"), None);
        assert!(Posix::parse("AAA5:30").is_some());
    }

    #[test]
    fn the_switch_happens_at_the_named_local_second() {
        let ny = Posix::parse("EST5EDT,M3.2.0,M11.1.0").expect("rule");
        let switch = days_from_civil(2026, 3, 8) * 86_400 + 7 * 3600;
        assert_eq!(ny.offset(switch - 1), -5 * 3600);
        assert_eq!(ny.offset(switch), -4 * 3600);
    }

    #[test]
    fn days_from_civil_inverts_civil() {
        for t in [0u64, 951_782_400, SUMMER as u64, WINTER as u64] {
            let (y, m, d, ..) = civil(t);
            assert_eq!(days_from_civil(y, m, d), (t / 86_400) as i64);
        }
    }
}
