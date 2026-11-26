//! §5 signing: pre-signature bytes and presig (§5.1), message M (§5.2), EIP-191 digest (§5.3),
//! signature form (§5.4), signer equals judge (§5.5). The cryptography itself is in [`crate::cryptox`];
//! this module only assembles bytes and checks forms.

use crate::cryptox;
use crate::hexform;
use crate::json::{self, Value};
use crate::token::Token;

/// The 25-byte P of the EIP-191 version 0x45 prefix: `Ethereum Signed Message:` and a newline.
const EIP191_P: &[u8] = b"Ethereum Signed Message:\n";

/// §5.1: canonical bytes without the top-level `sig` member (only that one; a member of the same name
/// inside body stays). A value outside the §3.1 universe has no canonical bytes and so nothing to sign.
pub fn b6_of(entry: &[(String, Value)]) -> Result<Vec<u8>, Token> {
    let six: Vec<(String, Value)> = entry.iter().filter(|(k, _)| k != "sig").cloned().collect();
    json::canonical(&Value::Obj(six))
}

/// §5.2: `M = D || 0x0A || hex32(presig)`.
pub fn message(domain: &[u8], presig: &[u8; 32]) -> Vec<u8> {
    let mut m = Vec::with_capacity(domain.len() + 1 + 66);
    m.extend_from_slice(domain);
    m.push(0x0A);
    m.extend_from_slice(hexform::encode(presig).as_bytes());
    m
}

/// §5.3: `keccak256(0x19 || P || decimal(len(M)) || M)`.
pub fn digest(m: &[u8]) -> [u8; 32] {
    let mut pre = Vec::with_capacity(26 + 3 + m.len());
    pre.push(0x19);
    pre.extend_from_slice(EIP191_P);
    pre.extend_from_slice(m.len().to_string().as_bytes());
    pre.extend_from_slice(m);
    cryptox::keccak256(&pre)
}

pub struct Signed {
    pub digest: [u8; 32],
    pub presig: [u8; 32],
    pub sig: [u8; 65],
    pub signer: [u8; 20],
}

/// The signing itself (§5.7 deterministic signing lives in cryptox). Private on purpose: outside this
/// module there are only two named doors, [`sign_as`] for the product (by seat and domain) and
/// [`sign_harness`] for the conformance harness.
fn sign_bytes(key: &cryptox::Privkey, b: &[u8], domain: &[u8]) -> Signed {
    let presig = cryptox::sha256(b);
    let d = digest(&message(domain, &presig));
    let s = key.sign(&d);
    let mut sig = [0u8; 65];
    sig[..32].copy_from_slice(&s.r);
    sig[32..64].copy_from_slice(&s.s);
    sig[64] = s.v;
    Signed { digest: d, presig, sig, signer: key.address() }
}

/// The conformance-harness door, for `gl1 sign` only. The harness requires any well-formed domain
/// literal (every byte below 0x80, no newline), so this door does not consult the registry; it is not
/// a product door. Every product signature goes through [`sign_as`].
pub fn sign_harness(key: &cryptox::Privkey, b: &[u8], domain: &[u8]) -> Signed {
    sign_bytes(key, b, domain)
}

/// The signing allow-list by (seat, domain), from the signing-domain roles of the embedded
/// FAMILY-LITERALS.md, listed per seat. Sorted by pair.
pub fn registered_domains_by_seat() -> Vec<(String, String)> {
    crate::literals::embedded_domains_by_seat()
}

/// The domain half of the allow-list: every domain in it, deduplicated and sorted. Computed from the
/// same table, never from a second reading of the registry.
pub fn registered_domains() -> Vec<String> {
    let mut v: Vec<String> = registered_domains_by_seat().into_iter().map(|(_, d)| d).collect();
    v.sort();
    v.dedup();
    v
}

/// An unregistered (seat, domain): a signing key never signs it. The string is `seat:domain`.
#[derive(Debug, PartialEq, Eq)]
pub struct UnregisteredDomain(pub String);

/// The product signing API, by seat: a key signs a domain only when that (seat, domain) pair is
/// registered, so a judge key cannot sign a verifier's domain. The harness door `gl1 sign` does not
/// pass here (its contract requires any well-formed literal).
#[cfg(not(feature = "ablate-k5-sign-gate"))]
pub fn sign_as(key: &cryptox::Privkey, b: &[u8], seat: &str, domain: &str) -> Result<Signed, UnregisteredDomain> {
    crate::marks::mark(crate::marks::Feature::K5);
    crate::marks::mark(crate::marks::Feature::K5);
    if registered_domains_by_seat().iter().any(|(s, d)| s == seat && d == domain) {
        Ok(sign_bytes(key, b, domain.as_bytes()))
    } else {
        Err(UnregisteredDomain(format!("{seat}:{domain}")))
    }
}

/// Fault-injection variant for the conformance gauges: any seat signs any domain.
#[cfg(feature = "ablate-k5-sign-gate")]
pub fn sign_as(key: &cryptox::Privkey, b: &[u8], seat: &str, domain: &str) -> Result<Signed, UnregisteredDomain> {
    crate::marks::mark(crate::marks::Feature::K5);
    crate::marks::mark(crate::marks::Feature::K5);
    let _ = seat;
    Ok(sign_bytes(key, b, domain.as_bytes()))
}

/// The seat that signs a registered domain: exactly one seat in the table, else `None`.
pub fn seat_of_domain(domain: &str) -> Option<String> {
    let mut seats = registered_domains_by_seat().into_iter().filter(|(_, d)| d == domain).map(|(s, _)| s);
    let first = seats.next()?;
    seats.next().is_none().then_some(first)
}

/// §4.3 step 13: `E_SIG_V`, `E_SIG_RANGE`, `E_SIG_HIGH_S`, `E_SIG_RECOVER`, `E_SIG_SIGNER`, in the order of
/// §5.4 then §5.5. `sig` already has the hex65 form (step 10) and `judge` the hex20 form (step 6).
#[cfg(not(feature = "ablate-k1-sign-recover"))]
pub fn verify(entry: &[(String, Value)], judge: &str, sig_hex: &str) -> Result<(), Token> {
    crate::marks::mark(crate::marks::Feature::K1);
    let raw = hexform::decode_form(sig_hex);
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&raw[..32]);
    s.copy_from_slice(&raw[32..64]);
    let v = raw[64];
    if v != 27 && v != 28 {
        return Err(Token::SigV);
    }
    if !cryptox::in_range(&r) || !cryptox::in_range(&s) {
        return Err(Token::SigRange);
    }
    if !cryptox::is_low_s(&s) {
        return Err(Token::SigHighS);
    }
    let presig = cryptox::sha256(&b6_of(entry)?);
    let d = digest(&message(crate::LAW.as_bytes(), &presig));
    let a = cryptox::recover_address(&d, &r, &s, v - 27).ok_or(Token::SigRecover)?;
    if hexform::encode(&a) != judge {
        return Err(Token::SigSigner);
    }
    Ok(())
}

/// Fault-injection variant for the conformance gauges: step 13 does not check the signature.
#[cfg(feature = "ablate-k1-sign-recover")]
pub fn verify(entry: &[(String, Value)], judge: &str, sig_hex: &str) -> Result<(), Token> {
    crate::marks::mark(crate::marks::Feature::K1);
    let _ = (entry, judge, sig_hex);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_and_digest_shape() {
        let presig = [0u8; 32];
        let m = message(crate::LAW.as_bytes(), &presig);
        assert_eq!(m.len(), 75);
        assert!(m.starts_with(b"galeed/1\n0x"));
        // 0x19 || P (25 bytes) || "75" || M: 26 + 2 + 75 = 103 bytes into keccak; only the length is checked here.
        let _ = digest(&m);
    }

    #[test]
    fn gate_admits_five_and_refuses_the_rest() {
        let mut kb = [0u8; 32];
        kb[31] = 1;
        let k = cryptox::Privkey::from_bytes(&kb).unwrap();
        let _ = &k;
        assert_eq!(registered_domains().len(), 5);
        assert_eq!(seat_of_domain(crate::LAW).as_deref(), Some(crate::literals::SEAT_JUDGE));
        assert_eq!(seat_of_domain("galeed.pass/1").as_deref(), Some(crate::literals::SEAT_VERIFIER));
        assert_eq!(seat_of_domain("galeed/2"), None);
    }

    #[test]
    fn the_seat_gate_pairs_five_domains_with_two_seats_and_refuses_the_rest() {
        use crate::literals::{SEAT_GUILD, SEAT_JUDGE, SEAT_VERIFIER};
        let mut kb = [0u8; 32];
        kb[31] = 1;
        let k = cryptox::Privkey::from_bytes(&kb).unwrap();
        let table = registered_domains_by_seat();
        assert_eq!(table.len(), 5, "五个域各配恰一席");
        assert!(table.iter().all(|(s, _)| s != SEAT_GUILD), "行会一域不签");
        assert!(sign_as(&k, b"{}", SEAT_JUDGE, crate::LAW).is_ok());
        assert!(sign_as(&k, b"{}", SEAT_VERIFIER, "galeed.pass/1").is_ok());
        // Registered domain, wrong seat: a judge key cannot sign the verifier's domains, and a verifier key
        // cannot sign the entry domain.
        assert_eq!(sign_as(&k, b"{}", SEAT_JUDGE, "galeed.pass/1").err(), Some(UnregisteredDomain("judge:galeed.pass/1".into())));
        assert_eq!(sign_as(&k, b"{}", SEAT_VERIFIER, crate::LAW).err(), Some(UnregisteredDomain("verifier:galeed/1".into())));
        assert_eq!(sign_as(&k, b"{}", SEAT_GUILD, crate::LAW).err(), Some(UnregisteredDomain("guild:galeed/1".into())));
        assert_eq!(sign_as(&k, b"{}", SEAT_JUDGE, "galeed/2").err(), Some(UnregisteredDomain("judge:galeed/2".into())));
        assert_eq!(registered_domains(), {
            let mut v: Vec<String> = table.iter().map(|(_, d)| d.clone()).collect();
            v.sort();
            v.dedup();
            v
        });
    }

    #[test]
    fn b6_drops_only_top_level_sig() {
        let v = json::read(b"{\"sig\":\"x\",\"body\":{\"sig\":\"keep\"},\"a\":1}").unwrap();
        assert_eq!(b6_of(v.as_obj().unwrap()).unwrap(), b"{\"a\":1,\"body\":{\"sig\":\"keep\"}}");
    }
}
