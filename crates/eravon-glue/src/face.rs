pub const ENTRY_TYPE: &str = eravon_face::ENTRY_TYPE;

pub const DOMAIN: &str = eravon_face::DOMAIN;

pub const MARKER_PREFIX: &str = eravon_face::MARKER_PREFIX;

use zikaron_core::hexfmt;
use zikaron_core::json::Value;
pub use eravon_face::{consent, names, reading};

#[cfg(test)]
fn h20(a: &[u8; 20]) -> String {
    hexfmt::encode(a)
}

fn s(v: &str) -> Value {
    Value::Str(v.to_string())
}

pub const CONSENT_PREFIX: &str = "zikaron-consent:";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Consent {
    pub root: [u8; 20],
    pub address: [u8; 20],
    pub role: String,
    pub sig: [u8; 65],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentUnread {
    Prefix,
    Form,
}

impl Consent {
    pub fn text(&self) -> String {
        format!("{CONSENT_PREFIX}{}:{}:{}:{}", hexfmt::encode(&self.root), hexfmt::encode(&self.address), self.role, hexfmt::encode(&self.sig))
    }

    pub fn parse(text: &str) -> Result<Consent, ConsentUnread> {
        let rest = text.trim().strip_prefix(CONSENT_PREFIX).ok_or(ConsentUnread::Prefix)?;
        let parts: Vec<&str> = rest.split(':').collect();
        let [root, address, role, sig] = parts.as_slice() else { return Err(ConsentUnread::Form) };
        let h20 = |v: &str| -> Option<[u8; 20]> { (v.len() == 42).then_some(())?; hexfmt::decode(v)?.try_into().ok() };
        let root = h20(root).ok_or(ConsentUnread::Form)?;
        let address = h20(address).ok_or(ConsentUnread::Form)?;
        if !eravon_face::names::is_label(role) {
            return Err(ConsentUnread::Form);
        }
        let sig: [u8; 65] = (sig.len() == 132).then_some(()).and_then(|_| hexfmt::decode(sig)).and_then(|b| b.try_into().ok()).ok_or(ConsentUnread::Form)?;
        Ok(Consent { root, address, role: role.to_string(), sig })
    }

    pub fn holds(&self) -> bool {
        let body = Value::Obj(vec![
            ("address".into(), s(&hexfmt::encode(&self.address))),
            ("role".into(), s(&self.role)),
            ("root".into(), s(&hexfmt::encode(&self.root))),
            ("sig".into(), s(&hexfmt::encode(&self.sig))),
        ]);
        eravon_face::consent::holds(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: [u8; 20] = [0x11; 20];

    #[test]
    fn a_consent_text_reads_back_and_holds_under_its_root_alone() {
        let key = [0x07u8; 32];
        let address: [u8; 20] = zikaron_core::cryptox::address_of_privkey(&key).expect("an address");
        let sig = eravon_face::consent::sign(&key, &h20(&R), &h20(&address), "payout").expect("signed");
        let c = Consent { root: R, address, role: "payout".into(), sig };
        let text = c.text();
        assert!(text.starts_with("zikaron-consent:0x"));
        assert_eq!(Consent::parse(&format!("  {text}\n")), Ok(c.clone()));
        let up = |b: &[u8]| hexfmt::encode(b).to_ascii_uppercase().replacen("0X", "0x", 1);
        assert_eq!(Consent::parse(&format!("zikaron-consent:{}:{}:payout:{}", up(&R), up(&address), up(&sig))), Ok(c.clone()), "hex of either case");
        assert!(c.holds());
        assert!(!Consent { root: [0x77; 20], ..c.clone() }.holds(), "another root");
        assert!(!Consent { role: "judge".into(), ..c.clone() }.holds(), "another role");
        assert!(!Consent { address: [0x66; 20], ..c.clone() }.holds(), "signed by another address");
        assert_eq!(Consent::parse(&text[8..]), Err(ConsentUnread::Prefix));
        assert_eq!(Consent::parse("zikaron-consent:0x11:0x22:payout:0x33"), Err(ConsentUnread::Form));
        assert_eq!(Consent::parse(&format!("{text}:extra")), Err(ConsentUnread::Form));
        assert_eq!(Consent::parse(&text.replace(":payout:", ":Pay:")), Err(ConsentUnread::Form));
    }
}

