use crate::fault::{Fault, R};

pub const V_VAULT: &str = eravon_literals::literals::VAULT_FORMAT;
pub const V_SETTINGS: &str = "eravon.store.settings/3";
pub const V_IDENTITY_SETTINGS: &str = "eravon.store.identity-settings/2";
pub const V_CANDIDATES: &str = "eravon.store.candidates/1";
pub const V_READING: &str = "eravon.store.reading/3";
pub const V_RECORD: &str = "eravon.store.record/1";
pub const V_ARCHIVE: &str = "eravon.store.archive/1";
pub const V_LOCK: &str = "eravon.store.lock/1";

pub const V_BACKUP_PACK: &str = "eravon.store.backup-pack/1";

pub const V_MIRROR: &str = "eravon.store.mirror/1";

pub const F_VAULT: &str = "vault.json";
pub const F_SETTINGS: &str = "settings.json";
pub const D_IDENTITIES: &str = "identities";
pub const D_RESTORE_NEXT: &str = "restore.next";
pub const F_RESTORE_READY: &str = "ready";
pub const F_RESTORE_ASIDE: &str = "aside";
pub const D_RESTORE_ASIDE: &str = "restore.aside-";

pub fn restore_next(data: &str) -> String {
    format!("{}/{D_RESTORE_NEXT}", data.trim_end_matches('/'))
}

pub fn restore_aside(data: &str, stamp: u64) -> String {
    format!("{}/{D_RESTORE_ASIDE}{stamp}", data.trim_end_matches('/'))
}
pub const F_CANDIDATES: &str = "candidates.json";
pub const F_READING: &str = "reading.json";
pub const F_LOCK: &str = ".lock";
pub const F_WRITER_MARK: &str = ".writer-mark";
pub const F_MACHINE_MARK: &str = "eravon-machine-mark";

pub const F_ATTEMPTS: &str = "attempts-";

pub fn machine_attempts_name(resolved: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in resolved.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{F_ATTEMPTS}{h:016x}")
}

pub const F_MIRROR: &str = ".mirror";
pub const F_RECORD: &str = "record.json";
pub const D_PARTS: &str = "parts";
pub const D_STORE: &str = "store";
pub const D_TRACE: &str = "trace";
pub const F_TRACE: &str = "shell-trace.json";

pub const N_CANDIDATES: &str = "候选表";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Box_ {
    Products,
    Orders,
    Cases,
    Packs,
}

impl Box_ {
    pub const fn dir(self) -> &'static str {
        match self {
            Box_::Products => "products",
            Box_::Orders => "orders",
            Box_::Cases => "cases",
            Box_::Packs => "packs",
        }
    }
    pub const ALL: [Box_; 4] = [Box_::Products, Box_::Orders, Box_::Cases, Box_::Packs];
    pub fn from_dir(s: &str) -> Option<Box_> {
        Box_::ALL.into_iter().find(|b| b.dir() == s)
    }
}

pub fn address_ok(a: &str) -> bool {
    a.len() == 42
        && a.starts_with("0x")
        && a[2..].bytes().all(|c| c.is_ascii_hexdigit())
}

pub fn name_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.starts_with('.')
        && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'.')
}

pub fn need_name(what: &str, s: &str) -> R<()> {
    if name_ok(s) { Ok(()) } else { Err(Fault::Name(format!("{what} 的形不合:{s:?}"))) }
}

pub fn need_address(a: &str) -> R<()> {
    if address_ok(a) {
        Ok(())
    } else {
        Err(Fault::Name(format!("身份地址要 0x 加四十位十六进制,给的是 {a:?}")))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    Identity,
    Box(Box_),
    Staging,
}

impl NameKind {
    pub fn tag(self) -> &'static str {
        use eravon_literals::literals::NAME_KINDS as T;
        match self {
            NameKind::Identity => T[0],
            NameKind::Box(Box_::Products) => T[1],
            NameKind::Box(Box_::Orders) => T[2],
            NameKind::Box(Box_::Cases) => T[3],
            NameKind::Box(Box_::Packs) => T[4],
            NameKind::Staging => T[5],
        }
    }

    pub fn of_tag(t: &str) -> Option<NameKind> {
        let all = [
            NameKind::Identity,
            NameKind::Box(Box_::Products),
            NameKind::Box(Box_::Orders),
            NameKind::Box(Box_::Cases),
            NameKind::Box(Box_::Packs),
            NameKind::Staging,
        ];
        all.into_iter().find(|k| k.tag() == t)
    }
}

pub const TOKEN_HEX: usize = 64;

pub fn token_ok(s: &str) -> bool {
    s.len() == TOKEN_HEX && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn join(base: &str, seg: &str) -> String {
    std::path::Path::new(base).join(seg).to_string_lossy().into_owned()
}

pub fn under(data: &str, rel: &str) -> String {
    join(data, rel)
}
pub fn settings_path(data: &str) -> String {
    join(data, F_SETTINGS)
}
pub fn mirror_path(data: &str) -> String {
    join(data, F_MIRROR)
}
pub fn lock_path(data: &str) -> String {
    join(data, F_LOCK)
}
pub fn writer_mark_path(data: &str) -> String {
    join(data, F_WRITER_MARK)
}
pub fn trace_dir(data: &str) -> String {
    join(data, D_TRACE)
}
pub fn trace_path(data: &str) -> String {
    join(&trace_dir(data), F_TRACE)
}

pub fn temp_path(target: &str, hint: &str) -> R<String> {
    eravon_literals::durable::temp_beside(std::path::Path::new(target), hint).map(|p| p.to_string_lossy().into_owned()).map_err(|e| Fault::Io(e.to_string()))
}

pub fn is_temp(name: &str) -> bool {
    crate::atomic::is_temp(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_assembled_in_one_place_only() {
        assert_eq!(std::path::Path::new(&settings_path("/d")), std::path::Path::new("/d/settings.json"));
        assert_eq!(std::path::Path::new(&lock_path("/d")), std::path::Path::new("/d/.lock"));
        assert_eq!(std::path::Path::new(&trace_path("/d")), std::path::Path::new("/d/trace/shell-trace.json"));
    }

    #[test]
    fn the_name_kinds_are_the_k7_table_and_tokens_have_one_shape() {
        for t in eravon_literals::literals::NAME_KINDS {
            assert_eq!(NameKind::of_tag(t).unwrap().tag(), t);
        }
        assert!(token_ok(&"a".repeat(64)));
        assert!(!token_ok(&"A".repeat(64)));
        assert!(!token_ok(&"a".repeat(63)));
    }

    #[test]
    fn the_four_boxes_are_a_closed_set() {
        assert_eq!(Box_::ALL.len(), 4);
        assert_eq!(Box_::from_dir("products"), Some(Box_::Products));
        assert_eq!(Box_::from_dir("vault.json"), None);
        assert_eq!(Box_::from_dir("Products"), None);
    }

    #[test]
    fn the_name_gate_rejects_every_way_out_of_the_directory() {
        assert!(name_ok("p-1"));
        assert!(name_ok("terms.md"));
        assert!(!name_ok(""));
        assert!(!name_ok("."));
        assert!(!name_ok(".."));
        assert!(!name_ok(".hidden"));
        assert!(!name_ok("a/b"));
        assert!(!name_ok("../etc"));
        assert!(!name_ok(&"x".repeat(65)));
        assert!(name_ok(&"x".repeat(64)));
    }

    #[test]
    fn the_address_gate_takes_forty_hex_digits_and_nothing_else() {
        assert!(address_ok("0x00000000000000000000000000000000000000aB"));
        assert!(!address_ok("0x00"));
        assert!(!address_ok("00000000000000000000000000000000000000aB"));
        assert!(!address_ok("0x00000000000000000000000000000000000000ag"));
    }

    #[test]
    fn the_temp_file_sits_in_the_same_directory_as_its_target() {
        let t = temp_path("/d/x/record.json", "record.json").expect("entropy");
        let at = std::path::Path::new(&t);
        assert!(at.parent() == Some(std::path::Path::new("/d/x")) && at.file_name().is_some_and(|n| n.to_string_lossy().starts_with(crate::layout::TMP_PREFIX)), "{t}");
        assert!(is_temp(&t[5..]));
        assert!(!is_temp("record.json"));
    }
}
