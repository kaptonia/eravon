use super::*;
use crate::fault::{Fault, Known};

pub(super) fn face_consent(shell: &mut Shell, identity: &str, role: &str) -> Result<String, Fault> {
    let text = identity.trim();
    let hex = text.strip_prefix(eravon_glue::face::MARKER_PREFIX).unwrap_or(text);
    let root = crate::key::Address::parse(hex).filter(|_| hex.len() == 42).ok_or_else(|| Fault::known(Known::AddressShape, "root".to_string()))?;
    let view = crate::register::view()?;
    let acct = crate::identity::account_now(&view).ok_or_else(|| Fault::known(Known::SeatUnseated, crate::lang::both_of(|| crate::lang::t(crate::lang::Key::IdSeatEmpty).to_string())))?;
    let key = crate::key::load(Some(&acct))?.ok_or_else(|| Fault::known(Known::KeychainMissing, acct.clone()))?;
    if key.address() != shell.anchor {
        return Err(Fault::known(Known::SignFailed, "the seat's key in office".to_string()));
    }
    Ok(eravon_vault::sign::sign_consent(&key, &root.0, role.trim())?.text())
}
