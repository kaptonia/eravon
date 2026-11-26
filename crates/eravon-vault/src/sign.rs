#[path = "key.rs"]
pub mod key;

use crate::fault::{Fault, Known};

pub(crate) fn key_of(addr: &str) -> Result<[u8; 32], Fault> {
    crate::records::seat_of(addr)?;
    let want = crate::key::Address::parse(addr).ok_or_else(|| Fault::known(Known::NoIdentity, addr.to_string()))?;
    let s = crate::key::load_at(&crate::places::key_slot(&want))?.filter(|s| s.address() == Some(want)).ok_or_else(|| Fault::known(Known::KeyNotStored, addr.to_string()))?;
    Ok(s.with_tx_key(|k| *k))
}

pub const CONSENT_ROLES: [&str; 3] = ["eravon-seller", "eravon-buyer", "payout"];

pub fn sign_consent(s: &crate::key::Secret, root: &[u8; 20], role: &str) -> Result<eravon_glue::face::Consent, Fault> {
    let address = s.address().ok_or_else(|| Fault::known(Known::KeyMalformed, String::new()))?;
    if !CONSENT_ROLES.contains(&role) {
        return Err(Fault::known(Known::SignFailed, format!("role {role}")));
    }
    let (r, a) = (crate::key::Address(*root).hex(), address.hex());
    let sig = s.with_sign_key(|k| eravon_glue::face::consent::sign(k, &r, &a, role)).ok_or_else(|| Fault::known(Known::SignFailed, format!("domain {}", eravon_glue::face::DOMAIN)))?;
    Ok(eravon_glue::face::Consent { root: *root, address: address.0, role: role.to_string(), sig })
}

#[cfg(test)]
mod consent_tests {
    use super::*;

    #[test]
    fn a_consent_signed_here_reads_and_holds_as_zikaron_reads_it_in_each_role() {
        let key = crate::key::Secret::take([0x07; 32]).expect("a key");
        let root = [0x11u8; 20];
        let mine = key.address().expect("an address");
        for role in CONSENT_ROLES {
            let c = sign_consent(&key, &root, role).expect("signed");
            assert_eq!(c.address, mine.0, "the address is the key's own");
            let text = c.text();
            assert!(text.starts_with(&format!("zikaron-consent:{}:{}:{role}:0x", crate::key::Address(root).hex(), mine.hex())), "{text}");
            let back = eravon_glue::face::Consent::parse(&text).expect("reads back");
            assert_eq!(back, c);
            assert!(back.holds(), "{role}: holds");
            assert!(!eravon_glue::face::Consent { root: [0x77; 20], ..back.clone() }.holds(), "{role}: under no other root");
            let other = CONSENT_ROLES.iter().find(|r| **r != role).expect("another role");
            assert!(!eravon_glue::face::Consent { role: other.to_string(), ..back.clone() }.holds(), "{role}: in no other role");
        }
        assert_eq!(sign_consent(&key, &root, "judge").map(|_| ()).unwrap_err().which(), Some(Known::SignFailed), "a role outside the three");
    }
}
