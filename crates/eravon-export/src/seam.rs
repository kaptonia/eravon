pub const FILE_LISTING: &str = "listing.json";
pub const FILE_RECEIPT: &str = "receipt.json";
pub const FILE_ENTRY: &str = "entry.json";

pub fn attachment_name(i: usize) -> String {
    format!("attachment-{i}.md")
}

pub const MISSING_POSTED: &str = "上板记录";
pub const MISSING_LISTING: &str = "清单";
pub const MISSING_RECEIPT: &str = "回执";

pub const R_NOT_WHOLE: &str = "E_S7_NOT_WHOLE";

pub fn dossier_name(case_id: &str, now: u64) -> String {
    let c = case_id.trim_start_matches("0x");
    format!("dossier-{}-{now}", &c[..c.len().min(16)])
}

pub fn statement_name(case_id: &str, now: u64) -> String {
    let c = case_id.trim_start_matches("0x");
    format!("statement-{}-{now}.json", &c[..c.len().min(16)])
}

pub fn compliance_name(case_id: &str, now: u64) -> String {
    let c = case_id.trim_start_matches("0x");
    format!("compliance-{}-{now}.json", &c[..c.len().min(16)])
}

pub fn join(dir: &str, name: &str) -> String {
    std::path::Path::new(dir).join(name).to_string_lossy().into_owned()
}

pub fn staging(dir: &str, name: &str) -> String {
    join(dir, &format!(".{name}.partial"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_do_not_double_the_separator() {
        assert_eq!(join("/a/b/", "c"), "/a/b/c");
        assert_eq!(std::path::Path::new(&staging("/a", "d")), std::path::Path::new("/a/.d.partial"));
        assert_eq!(dossier_name("0x0123456789abcdef0011", 7), "dossier-0123456789abcdef-7");
    }
}
