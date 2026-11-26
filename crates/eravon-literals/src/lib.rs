#![forbid(unsafe_code)]

mod chelek;
pub mod check;
pub mod constants;
pub mod durable;
pub mod echo;
pub mod foreign;
pub mod json;
pub mod literals;
pub mod misuse;
pub mod registry;
pub mod schema;
pub mod seam;
pub mod trace;
pub mod units;

pub const FAMILY: &str = literals::URL_SCHEME;
pub const REGISTRY_NAME: &str = "eravon-family-literals";
pub const ANSWER_VERSION: &str = "eravon-literals-check/1";

pub fn own_family() -> foreign::Family {
    foreign::Family {
        name: FAMILY.to_string(),
        source: format!("{REGISTRY_NAME}(本 crate 的登记簿)"),
        literals: registry::literals().into_iter().map(|s| s.to_string()).collect(),
    }
}

pub fn preflight() -> Result<(), &'static str> {
    registry::self_check()?;
    constants::self_check()?;
    seam::self_check()?;
    check::self_check()?;
    Ok(())
}
