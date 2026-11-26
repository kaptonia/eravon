//! This desk's trace channel, outside the family components: moved here from the core's `trace.rs`, which now holds
//! the family's core trace (component `app-skeleton`).
//!
//! The diagnostic trace channel: one place the shell and the lower crates (cryptox, sign, vault, key source, CLI
//! form checks, chain time) all write to. It lives in the lowest crate so lower crates can mark without depending on
//! the app. A mark's content is one feature code from a closed table ([`Feature`]) and nothing else: free text cannot
//! be put in, so marks cannot grow into a second log. Line form: `<id>\n`.
//!
//! The channel ships with the build and has no `cfg` at all: there is no switch that compiles it out. A mark is
//! observation only and never feeds a decision. Two sinks: an in-memory ring (always) and a file (added when
//! `GALEED_DESK_TRACE` names one). The file stops at a cap set once in `.cargo/config.toml`
//! (`GALEED_DESK_TRACE_CAP_BYTES`); a file that cannot be opened, and the cap reached, are each kept to be said once on
//! the face ([`take_trouble`], [`take_full`]), never silent.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Where marks go: `GALEED_DESK_TRACE` names a path; unset means the ring alone.
pub const ENV: &str = "GALEED_DESK_TRACE";

/// At most this many marks are kept in the ring.
pub const RING: usize = 512;

macro_rules! features {
    ($($f:ident),* $(,)?) => {
        /// The closed table of feature codes (read mechanically from every mark in the product; a code outside it
        /// does not compile).
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
        pub enum Feature {
            $($f,)*
        }

        impl Feature {
            pub const ALL: [Feature; 53] = [$(Feature::$f),*];

            /// The entire content of a trace mark: one feature code.
            pub fn id(self) -> &'static str {
                match self {
                    $(Feature::$f => stringify!($f),)*
                }
            }
        }
    };
}

features!(A1, A2, A3, A4, A5, G1, G2, G3, G4, G5, G6, G7, G8, G9, G10, G11, G12, G13, H0, H1, H2, H3, H4, H5, H6, J1, J2, J3, J5, J6, J7, J8, J9, J13, J14, J15, J16, K1, K5, K6, P1, P2, R1, R2, R3, R4, R5, V1, V2, V3, V4, V5, V6);

#[derive(Debug, Default)]
struct Trace {
    ring: Vec<&'static str>,
    sink: Option<Option<PathBuf>>,
    /// The sink file that could not be opened or written, and what the system said (said once).
    trouble: Option<(PathBuf, String)>,
    trouble_said: bool,
    /// The sink file at its cap (said once).
    full: Option<PathBuf>,
    full_said: bool,
    dropped: u64,
}

fn cell() -> &'static Mutex<Trace> {
    static C: OnceLock<Mutex<Trace>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Trace::default()))
}

/// Settle the sink once: the first call reads the environment (or a path given by [`open_to`]).
fn settle(t: &mut Trace) -> Option<PathBuf> {
    if let Some(s) = &t.sink {
        return s.clone();
    }
    let sink = std::env::var_os(ENV).filter(|p| !p.is_empty()).map(PathBuf::from);
    if let Some(p) = &sink {
        if let Err(e) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            t.trouble = Some((p.clone(), e.to_string()));
        }
    }
    t.sink = Some(sink.clone());
    sink
}

/// Drop a trace mark: its content is the feature code and nothing else. It answers nothing, so no decision can be
/// based on it.
pub fn mark(f: Feature) {
    let mut t = cell().lock().unwrap_or_else(|e| e.into_inner());
    let id = f.id();
    if t.ring.len() == RING {
        t.ring.remove(0);
    }
    t.ring.push(id);
    let Some(p) = settle(&mut t) else { return };
    match std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        Ok(mut h) => {
            // Writing stops at the cap; the first time it is reached is kept to be said once.
            let size = h.metadata().map(|m| m.len()).unwrap_or(0);
            if over_cap(size, id.len() as u64 + 1) {
                if t.full.is_none() {
                    t.full = Some(p);
                }
                t.dropped += 1;
                return;
            }
            if writeln!(h, "{id}").is_err() {
                t.dropped += 1;
            }
        }
        Err(e) => {
            if t.trouble.is_none() {
                t.trouble = Some((p, e.to_string()));
            }
            t.dropped += 1;
        }
    }
}

/// The marks kept in memory now (the newest last; at most [`RING`]).
pub fn ring() -> Vec<&'static str> {
    cell().lock().unwrap_or_else(|e| e.into_inner()).ring.clone()
}

/// The sink file that could not be opened or written, with the system's words: handed out once (`None` after).
pub fn take_trouble() -> Option<(PathBuf, String)> {
    let mut t = cell().lock().unwrap_or_else(|e| e.into_inner());
    if t.trouble_said {
        return None;
    }
    let got = t.trouble.clone()?;
    t.trouble_said = true;
    Some(got)
}

/// The sink file that reached its cap: handed out once (`None` after).
pub fn take_full() -> Option<PathBuf> {
    let mut t = cell().lock().unwrap_or_else(|e| e.into_inner());
    if t.full_said {
        return None;
    }
    let got = t.full.clone()?;
    t.full_said = true;
    Some(got)
}

/// Marks not written to the sink file while it was open (full, or not writable).
pub fn dropped() -> u64 {
    cell().lock().unwrap_or_else(|e| e.into_inner()).dropped
}

/// Where marks go (the debug driver reads it to count marks; the product only writes).
pub fn sink() -> Option<PathBuf> {
    let mut t = cell().lock().unwrap_or_else(|e| e.into_inner());
    settle(&mut t)
}

/// Open the channel on a path from outside (the debug driver does this: its environment is cleared). Works only
/// before the sink is settled; answers `false` once settled.
pub fn open_to(path: PathBuf) -> bool {
    let mut t = cell().lock().unwrap_or_else(|e| e.into_inner());
    if t.sink.is_some() {
        return false;
    }
    t.sink = Some(Some(path));
    true
}

/// Whether a sink file is in use (reporting only; never a decision input).
pub fn open() -> bool {
    sink().is_some()
}

/// The trace sink file holds at most this many bytes: read at compile time from `[env] GALEED_DESK_TRACE_CAP_BYTES`
/// in the workspace's `.cargo/config.toml`; a malformed value fails the build.
pub const TRACE_CAP_BYTES: u64 = cap_of(env!("GALEED_DESK_TRACE_CAP_BYTES"));

/// The name of the build environment variable the cap is read from (`env!` takes it as a literal, above; a test holds
/// the two spellings equal).
pub const CAP_ENV: &str = "GALEED_DESK_TRACE_CAP_BYTES";

const fn cap_of(s: &str) -> u64 {
    let b = s.as_bytes();
    assert!(!b.is_empty(), "GALEED_DESK_TRACE_CAP_BYTES 空着");
    let mut n: u64 = 0;
    let mut i = 0;
    while i < b.len() {
        assert!(b[i] >= b'0' && b[i] <= b'9', "GALEED_DESK_TRACE_CAP_BYTES 只许十进制数字");
        n = n * 10 + (b[i] - b'0') as u64;
        i += 1;
    }
    n
}

/// Whether writing `line` more bytes to a file of `size` bytes would pass the cap.
pub fn over_cap(size: u64, line: u64) -> bool {
    size + line > TRACE_CAP_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_answers_nothing_and_lands_in_the_ring() {
        let answer: () = mark(Feature::H1);
        assert_eq!(answer, ());
        assert!(ring().contains(&"H1"));
    }

    #[test]
    fn the_cap_is_read_by_its_registered_name_from_the_build_environment() {
        assert!(include_str!("marks.rs").contains(&format!("env!(\"{CAP_ENV}\")")), "env! 读的就是登记的名");
        assert!(include_str!("../../../.cargo/config.toml").contains(&format!("{CAP_ENV} = \"{TRACE_CAP_BYTES}\"")), "上限只在 .cargo/config.toml 给一次");
    }

    #[test]
    fn the_cap_takes_exactly_its_bytes_and_one_more_is_over() {
        assert!(!over_cap(TRACE_CAP_BYTES - 3, 3), "恰好收");
        assert!(over_cap(TRACE_CAP_BYTES - 2, 3), "多一字节即越");
    }

    #[test]
    fn every_feature_has_its_own_id() {
        let mut ids: Vec<&str> = Feature::ALL.iter().map(|f| f.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), Feature::ALL.len());
    }
}
