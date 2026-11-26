//! The cryptography boundary: third-party cryptography appears in this module only. sha256,
//! keccak256, secp256k1 signing (§5.7: RFC 6979, low s) and recovery (§5.4, §5.5) all live here;
//! everything else calls this module and never uses a cryptography crate.
//!
//! **The signing, recovery and address functions are the family core's own** (`N`, `HALF_N`, `in_range`,
//! `is_low_s`, `recover_address`, `sign_digest`, `address_of_privkey`, as the family's `cryptox` writes
//! them, through k256's ecdsa: RFC 6979 deterministic nonce, low-s encoding, only x = r tried on recovery).
//! What follows them is this desk's own: the key value [`Privkey`] the desk's signers hold (its signing and
//! address go through the family's functions), and the age, words, key-file and vault capabilities the
//! core's own tools still answer.

use hmac::{Hmac, Mac};
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::elliptic_curve::PrimeField;
use k256::{FieldBytes, ProjectivePoint, Scalar};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::str::FromStr;

/// secp256k1 group order n (law §1).
pub const N: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
    0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36, 0x41, 0x41,
];

/// (n − 1) / 2, the low-s bound (law §5.4).
pub const HALF_N: [u8; 32] = [
    0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x5d, 0x57, 0x6e, 0x73, 0x57, 0xa4, 0x50, 0x1d, 0xdf, 0xe9, 0x2f, 0x46, 0x68, 0x1b, 0x20, 0xa0,
];

pub fn sha256(b: &[u8]) -> [u8; 32] {
    Sha256::digest(b).into()
}

pub fn keccak256(b: &[u8]) -> [u8; 32] {
    Keccak256::digest(b).into()
}

/// Law §5.4: 1 ≤ x ≤ n − 1.
pub fn in_range(x: &[u8; 32]) -> bool {
    x.iter().any(|&b| b != 0) && x[..] < N[..]
}

/// Law §5.4: s ≤ (n − 1) / 2.
pub fn is_low_s(s: &[u8; 32]) -> bool {
    s[..] <= HALF_N[..]
}

/// Law §5.5: the last 20 bytes of keccak256 over the 64-byte X||Y of the uncompressed key.
fn address_of_key(vk: &VerifyingKey) -> [u8; 20] {
    let pt = vk.to_encoded_point(false);
    let xy = &pt.as_bytes()[1..];
    let h = keccak256(xy);
    let mut a = [0u8; 20];
    a.copy_from_slice(&h[12..]);
    a
}

/// Law §5.4, last item: recover the key from (digest, r, s, i) and give its address; `None` when recovery
/// fails. r and s have passed the range and low-s tests; recid is 0 or 1.
pub fn recover_address(digest: &[u8; 32], r: &[u8; 32], s: &[u8; 32], recid: u8) -> Option<[u8; 20]> {
    crate::marks::mark(crate::marks::Feature::K1);
    let sig = Signature::from_scalars(*r, *s).ok()?;
    let rid = RecoveryId::from_byte(recid)?;
    let vk = VerifyingKey::recover_from_prehash(digest, &sig, rid).ok()?;
    Some(address_of_key(&vk))
}

/// Law §5.7: RFC 6979 deterministic nonce, low-s encoding; returns (r, s, v) with v in {27, 28}.
pub fn sign_digest(privkey: &[u8; 32], digest: &[u8; 32]) -> Option<([u8; 32], [u8; 32], u8)> {
    crate::marks::mark(crate::marks::Feature::K1);
    let sk = SigningKey::from_slice(privkey).ok()?;
    let (sig, rid): (Signature, RecoveryId) = sk.sign_prehash_recoverable(digest).ok()?;
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&sig.r().to_bytes());
    s.copy_from_slice(&sig.s().to_bytes());
    Some((r, s, 27 + rid.to_byte()))
}

/// Address of a private key (the §5.5 derivation).
pub fn address_of_privkey(privkey: &[u8; 32]) -> Option<[u8; 20]> {
    let sk = SigningKey::from_slice(privkey).ok()?;
    Some(address_of_key(sk.verifying_key()))
}

// ─── This desk's own, over the family's functions ───

fn hmac_sha256(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut m = Hmac::<Sha256>::new_from_slice(key).expect("HMAC 收任意长度的钥");
    for p in parts {
        m.update(p);
    }
    m.finalize().into_bytes().into()
}

fn bytes32(fb: &FieldBytes) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(fb);
    out
}

/// 32 bytes as a scalar, answered only in [1, n − 1].
fn scalar_in_range(b: &[u8; 32]) -> Option<Scalar> {
    Option::<Scalar>::from(Scalar::from_repr(FieldBytes::from(*b))).filter(|s| !bool::from(s.is_zero()))
}

/// A signature (§5.7): r, s and v (27 or 28).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sig {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub v: u8,
}

/// A private key held by a signer: in range by construction ([`Privkey::from_bytes`]); its address and its signatures
/// are the family's functions' ([`address_of_privkey`], [`sign_digest`]). It wipes itself on drop.
pub struct Privkey([u8; 32]);

/// A private key wipes itself on drop, so no caller has to remember to wipe it: the one place that
/// forgets is the copy left in memory.
impl Drop for Privkey {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}

impl Privkey {
    pub fn from_bytes(b: &[u8; 32]) -> Option<Privkey> {
        in_range(b).then(|| Privkey(*b))
    }

    /// The key's bytes, lent to the family's functions that take a key by its bytes (a transaction's signing).
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn address(&self) -> [u8; 20] {
        address_of_privkey(&self.0).unwrap_or_default()
    }

    /// The uncompressed SEC1 public key (`0x04 ‖ x ‖ y`), the point the address is hashed from.
    pub fn public_key(&self) -> [u8; 65] {
        let k = scalar_in_range(&self.0).unwrap_or(Scalar::ONE);
        let enc = (ProjectivePoint::GENERATOR * k).to_affine().to_encoded_point(false);
        let mut out = [0u8; 65];
        out.copy_from_slice(enc.as_bytes());
        out
    }

    /// §5.7: the family's deterministic low-s signature of a 32-byte digest ([`sign_digest`]).
    pub fn sign(&self, digest: &[u8; 32]) -> Sig {
        let (r, s, v) = sign_digest(&self.0, digest).unwrap_or(([0u8; 32], [0u8; 32], 27));
        Sig { r, s, v }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_recover_round_trip() {
        let mut kb = [0u8; 32];
        kb[31] = 1;
        let k = Privkey::from_bytes(&kb).unwrap();
        let d = sha256(b"galeed");
        let sig = k.sign(&d);
        assert!(sig.v == 27 || sig.v == 28);
        assert!(is_low_s(&sig.s));
        assert_eq!(k.sign(&d), sig, "RFC 6979:同钥同摘要同签名");
        assert_eq!(recover_address(&d, &sig.r, &sig.s, sig.v - 27), Some(k.address()));
        assert_ne!(recover_address(&d, &sig.r, &sig.s, 1 - (sig.v - 27)), Some(k.address()));
    }

    #[test]
    fn range_walls() {
        assert!(!in_range(&[0u8; 32]));
        assert!(!in_range(&[0xffu8; 32]));
        let mut one = [0u8; 32];
        one[31] = 1;
        assert!(in_range(&one) && is_low_s(&one));
        // n − 1 is in range and is a high s.
        let mut n_minus_1 = N;
        n_minus_1[31] -= 1;
        assert!(in_range(&n_minus_1) && !is_low_s(&n_minus_1));
        assert!(!in_range(&N), "n itself is out of range");
    }
}

// age (X25519) evidence encryption: callers reach age through this module only.

pub struct AgeIdentity(age::x25519::Identity);

impl AgeIdentity {
    pub fn generate() -> AgeIdentity {
        AgeIdentity(age::x25519::Identity::generate())
    }
    pub fn parse(s: &str) -> Option<AgeIdentity> {
        age::x25519::Identity::from_str(s.trim()).ok().map(AgeIdentity)
    }
    /// The recipient string (`age1…`), carried as the `recipient` token of a galeed/1 §6.7 keybinding.
    pub fn recipient(&self) -> String {
        self.0.to_public().to_string()
    }
    pub fn secret(&self) -> String {
        use age::secrecy::ExposeSecret;
        self.0.to_string().expose_secret().to_string()
    }
}

pub fn age_recipient_ok(s: &str) -> bool {
    age::x25519::Recipient::from_str(s).is_ok()
}

/// Encrypt to one recipient; a malformed recipient is None. The ciphertext is age's binary format (no armor).
pub fn age_encrypt(recipient: &str, plaintext: &[u8]) -> Option<Vec<u8>> {
    crate::marks::mark(crate::marks::Feature::K1);
    let r = age::x25519::Recipient::from_str(recipient).ok()?;
    age::encrypt(&r, plaintext).ok()
}

/// Decrypt with an identity; not addressed to this key, or a damaged ciphertext, is None.
pub fn age_decrypt(identity: &AgeIdentity, ciphertext: &[u8]) -> Option<Vec<u8>> {
    crate::marks::mark(crate::marks::Feature::K1);
    age::decrypt(&identity.0, ciphertext).ok()
}

// Key derivation and backup (outside the law): BIP-39, BIP-32, keystore v3.

type HmacSha512 = Hmac<sha2::Sha512>;

/// The BIP-39 English word list (byte-identical to `bip-0039/english.txt` of the BIP-39 repository,
/// sha256 `0x2f5eed53…dbda`).
const BIP39_ENGLISH: &str = include_str!("bip39-english.txt");

fn wordlist() -> Vec<&'static str> {
    BIP39_ENGLISH.lines().collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MnemonicError {
    /// The word count is not in {12, 15, 18, 21, 24}.
    WordCount,
    UnknownWord(usize),
    Checksum,
}

impl MnemonicError {
    pub const fn name(&self) -> &'static str {
        match self {
            MnemonicError::WordCount => "word-count",
            MnemonicError::UnknownWord(_) => "unknown-word",
            MnemonicError::Checksum => "checksum",
        }
    }
}

/// `n` bytes from the operating system's random source (`galeed_os::random`, the repository's one entropy door), or
/// the system's own words for why not (callers refuse loudly and carry those words; there is no fallback to weak
/// randomness).
pub fn random_bytes(n: usize) -> std::io::Result<Vec<u8>> {
    galeed_os::random(n)
}

/// BIP-39: entropy (16, 20, 24, 28 or 32 bytes) plus the first ENT/32 bits of its sha256 as checksum,
/// eleven bits per word. This is the byte length of a twelve-word entropy (twelve words ↔ sixteen
/// bytes); callers ask it instead of writing 16.
pub const ENTROPY_LEN: usize = 16;

/// Whether a word is on the list exactly as [`mnemonic_entropy`] reads it (the entry grid marks the others red).
pub fn on_wordlist(word: &str) -> bool {
    wordlist().contains(&word)
}

pub fn mnemonic_from_entropy(entropy: &[u8]) -> Option<String> {
    if !(16..=32).contains(&entropy.len()) || entropy.len() % 4 != 0 {
        return None;
    }
    let words = wordlist();
    let cs_bits = entropy.len() * 8 / 32;
    let check = sha256(entropy)[0];
    let bit = |i: usize| -> u32 {
        if i < entropy.len() * 8 {
            u32::from((entropy[i / 8] >> (7 - i % 8)) & 1)
        } else {
            u32::from((check >> (7 - (i - entropy.len() * 8))) & 1)
        }
    };
    let total = entropy.len() * 8 + cs_bits;
    let out: Vec<&str> = (0..total / 11).map(|w| words[(0..11).fold(0u32, |acc, j| (acc << 1) | bit(w * 11 + j)) as usize]).collect();
    Some(out.join(" "))
}

/// BIP-39 in reverse: word count, word list, checksum, in that order; answers the entropy. Words are
/// separated by ASCII whitespace and compared byte for byte.
pub fn mnemonic_entropy(mnemonic: &str) -> Result<Vec<u8>, MnemonicError> {
    crate::marks::mark(crate::marks::Feature::K1);
    let given: Vec<&str> = mnemonic.split_ascii_whitespace().collect();
    if ![12, 15, 18, 21, 24].contains(&given.len()) {
        return Err(MnemonicError::WordCount);
    }
    let words = wordlist();
    let mut bits: Vec<u8> = Vec::with_capacity(given.len() * 11);
    for (i, w) in given.iter().enumerate() {
        let idx = words.iter().position(|x| x == w).ok_or(MnemonicError::UnknownWord(i))?;
        bits.extend((0..11).rev().map(|j| ((idx >> j) & 1) as u8));
    }
    let ent_bits = bits.len() * 32 / 33;
    let entropy: Vec<u8> = bits[..ent_bits].chunks(8).map(|c| c.iter().fold(0u8, |a, b| (a << 1) | b)).collect();
    let check = sha256(&entropy)[0];
    let want: Vec<u8> = (0..bits.len() - ent_bits).map(|i| (check >> (7 - i)) & 1).collect();
    if !checksum_ok(&bits[ent_bits..], &want) {
        return Err(MnemonicError::Checksum);
    }
    Ok(entropy)
}

#[cfg(not(feature = "ablate-k1-bip39"))]
fn checksum_ok(given: &[u8], want: &[u8]) -> bool {
    given == want
}

/// Fault-injection variant for the conformance gauges: the checksum is not checked.
#[cfg(feature = "ablate-k1-bip39")]
fn checksum_ok(given: &[u8], want: &[u8]) -> bool {
    let _ = (given, want);
    true
}

/// BIP-39 seed: PBKDF2-HMAC-SHA512, 2048 rounds, salt `mnemonic` plus the passphrase. Words are rejoined
/// with single spaces; the passphrase must be ASCII (NFKD is the identity on ASCII; a non-ASCII
/// passphrase answers `None` rather than normalizing Unicode here).
pub fn mnemonic_seed(mnemonic: &str, passphrase: &str) -> Option<[u8; 64]> {
    crate::marks::mark(crate::marks::Feature::K1);
    if !passphrase.is_ascii() {
        return None;
    }
    let phrase = mnemonic.split_ascii_whitespace().collect::<Vec<_>>().join(" ");
    let mut seed = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<sha2::Sha512>(phrase.as_bytes(), format!("mnemonic{passphrase}").as_bytes(), 2048, &mut seed);
    Some(seed)
}

pub const HARDENED: u32 = 1 << 31;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtendedKey {
    pub depth: u8,
    pub parent_fingerprint: [u8; 4],
    pub child: u32,
    pub chain_code: [u8; 32],
    pub key: [u8; 32],
}

fn hmac_sha512(key: &[u8], parts: &[&[u8]]) -> [u8; 64] {
    let mut m = HmacSha512::new_from_slice(key).expect("HMAC 收任意长度的钥");
    for p in parts {
        m.update(p);
    }
    m.finalize().into_bytes().into()
}

fn split64(i: &[u8; 64]) -> ([u8; 32], [u8; 32]) {
    let mut l = [0u8; 32];
    let mut r = [0u8; 32];
    l.copy_from_slice(&i[..32]);
    r.copy_from_slice(&i[32..]);
    (l, r)
}

/// The data prefix of CKDpriv: `0x00 ‖ k` when hardened, the compressed public key otherwise.
#[cfg(not(feature = "ablate-k1-bip32"))]
fn child_data(index: u32, key: &[u8; 32], k: &Scalar) -> Vec<u8> {
    if index >= HARDENED {
        let mut d = vec![0u8];
        d.extend_from_slice(key);
        d
    } else {
        compressed_pubkey(k)
    }
}

/// Fault-injection variant for the conformance gauges: hardened steps use the public key too.
#[cfg(feature = "ablate-k1-bip32")]
fn child_data(index: u32, key: &[u8; 32], k: &Scalar) -> Vec<u8> {
    let _ = (index, key);
    compressed_pubkey(k)
}

fn compressed_pubkey(k: &Scalar) -> Vec<u8> {
    (ProjectivePoint::GENERATOR * *k).to_affine().to_encoded_point(true).as_bytes().to_vec()
}

/// BIP-32 master key: `HMAC-SHA512("Bitcoin seed", S)`; a left half outside [1, n − 1] means no master key.
pub fn bip32_master(seed: &[u8]) -> Option<ExtendedKey> {
    crate::marks::mark(crate::marks::Feature::K1);
    let (l, r) = split64(&hmac_sha512(b"Bitcoin seed", &[seed]));
    scalar_in_range(&l)?;
    Some(ExtendedKey { depth: 0, parent_fingerprint: [0; 4], child: 0, chain_code: r, key: l })
}

impl ExtendedKey {
    /// BIP-32 CKDpriv: an index ≥ 2^31 is hardened (`0x00 ‖ k ‖ i`), otherwise the compressed public key is
    /// used (`serP(K) ‖ i`). A left half out of range or a zero child means no such child (the caller moves
    /// to the next index; this function does not skip for it). Depth stops at 255.
    pub fn child(&self, index: u32) -> Option<ExtendedKey> {
        let k = scalar_in_range(&self.key)?;
        let depth = self.depth.checked_add(1)?;
        let i = index.to_be_bytes();
        let out = hmac_sha512(&self.chain_code, &[&child_data(index, &self.key, &k), &i]);
        let (l, r) = split64(&out);
        let il = scalar_in_range(&l)?;
        let child = il + k;
        if bool::from(child.is_zero()) {
            return None;
        }
        Some(ExtendedKey { depth, parent_fingerprint: self.fingerprint(), child: index, chain_code: r, key: bytes32(&child.to_bytes()) })
    }

    /// Fingerprint: the first four bytes of `RIPEMD160(SHA256(·))` of the compressed public key.
    pub fn fingerprint(&self) -> [u8; 4] {
        let pk = scalar_in_range(&self.key).map(|k| compressed_pubkey(&k)).unwrap_or_default();
        let h = ripemd::Ripemd160::digest(Sha256::digest(&pk));
        [h[0], h[1], h[2], h[3]]
    }

    /// BIP-32 mainnet private serialization (`xprv`): version `0x0488ADE4`, depth, parent fingerprint,
    /// child number, chain code, `0x00 ‖ k`, base58check.
    pub fn xprv(&self) -> String {
        let mut b = vec![0x04, 0x88, 0xad, 0xe4, self.depth];
        b.extend_from_slice(&self.parent_fingerprint);
        b.extend_from_slice(&self.child.to_be_bytes());
        b.extend_from_slice(&self.chain_code);
        b.push(0);
        b.extend_from_slice(&self.key);
        let check = sha256(&sha256(&b));
        b.extend_from_slice(&check[..4]);
        base58(&b)
    }

    pub fn privkey(&self) -> Option<Privkey> {
        Privkey::from_bytes(&self.key)
    }
}

fn base58(b: &[u8]) -> String {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let zeros = b.iter().take_while(|x| **x == 0).count();
    let mut digits: Vec<u8> = Vec::new();
    for byte in b {
        let mut carry = u32::from(*byte);
        for d in digits.iter_mut() {
            carry += u32::from(*d) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut s = String::with_capacity(zeros + digits.len());
    s.extend(std::iter::repeat_n('1', zeros));
    s.extend(digits.iter().rev().map(|d| char::from(ALPHABET[*d as usize])));
    s
}

/// A derivation path `m/…`: each segment decimal (no leading zero, in [0, 2^31 − 1]) with an optional
/// `'` for hardened. Answers the index list.
pub fn parse_path(path: &str) -> Option<Vec<u32>> {
    let mut segs = path.split('/');
    if segs.next()? != "m" {
        return None;
    }
    segs.map(|s| {
        let (digits, hard) = match s.strip_suffix('\'') {
            Some(d) => (d, true),
            None => (s, false),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || (digits.len() > 1 && digits.starts_with('0')) {
            return None;
        }
        let n: u32 = digits.parse().ok().filter(|n| *n < HARDENED)?;
        Some(if hard { n | HARDENED } else { n })
    })
    .collect()
}

pub fn derive_path(seed: &[u8], path: &str) -> Option<ExtendedKey> {
    crate::marks::mark(crate::marks::Feature::K1);
    parse_path(path)?.into_iter().try_fold(bip32_master(seed)?, |k, i| k.child(i))
}

/// Why a key file does not open. Each refusal names what is wrong: the members whose form this reader does not
/// recognise, the cost bounds broken, the password, the key, the address, a password over the bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeystoreError {
    /// The members whose form is not keystore v3's (`version`, `kdf`, `cipher`, `n`, `r`, `p`, `prf`, `c`, `dklen`,
    /// `salt`, `iv`, `mac`, `ciphertext`, `address`; `json` when the bytes are not an object with its sections).
    Form(Vec<&'static str>),
    /// The derivation's cost out of the family's bounds (`r`, `p`, `memory`, `work`; `c` for pbkdf2).
    Params(Vec<&'static str>),
    /// The MAC does not match: a wrong password, or an altered ciphertext (asked before anything is decrypted).
    Mac,
    /// What decrypted is not a 32-byte private key in range (a file sealing anything else, the sixteen bytes of an
    /// old mnemonic file among them).
    Key,
    /// The written address is not the key's.
    Address,
    /// A password longer than [`KEYSTORE_PASSWORD_MAX`] bytes: refused before any work, never cut.
    PasswordLong,
    /// No random source for a new file.
    Random,
}

impl KeystoreError {
    pub const fn name(&self) -> &'static str {
        match self {
            KeystoreError::Form(_) => "keystore-form",
            KeystoreError::Params(_) => "keystore-params",
            KeystoreError::Mac => "keystore-mac",
            KeystoreError::Key => "keystore-key",
            KeystoreError::Address => "keystore-address",
            KeystoreError::PasswordLong => "PASSWORD_LONG",
            KeystoreError::Random => "random-unavailable",
        }
    }

    /// The members or bounds named (empty for the others).
    pub fn fields(&self) -> &[&'static str] {
        match self {
            KeystoreError::Form(f) | KeystoreError::Params(f) => f,
            _ => &[],
        }
    }
}

/// The longest key file password, in bytes, on both sides of the one derivation entry (sealing and opening): one
/// longer is refused by name before any work, never cut to fit.
pub const KEYSTORE_PASSWORD_MAX: usize = 1024;

pub mod keystore {
    /// Upper bounds on the KDF cost accepted when reading a file. The cost parameters come from the file,
    /// so the bound is checked before any work; out of bounds is `keystore-params`. The four numbers
    /// (scrypt memory `128·r·n`, `p`, total work `128·r·n·p`, and pbkdf2 rounds) live only in the `data`
    /// section of FAMILY-LITERALS.md and are read by name; if the registry cannot be read the bound is zero
    /// and every cost is out of bounds. Files written by common wallets (scrypt n 2^18 r 8 p 1, the light
    /// n 2^12 files, pbkdf2 at 262,144 rounds) are within bounds.
    pub fn scrypt_max_memory() -> u64 {
        crate::literals::datum(crate::literals::DATA_KEYSTORE_SCRYPT_MAX_MEMORY).unwrap_or(0)
    }
    pub fn scrypt_max_p() -> u64 {
        crate::literals::datum(crate::literals::DATA_KEYSTORE_SCRYPT_MAX_P).unwrap_or(0)
    }
    pub fn scrypt_max_work() -> u64 {
        crate::literals::datum(crate::literals::DATA_KEYSTORE_SCRYPT_MAX_WORK).unwrap_or(0)
    }
    pub fn pbkdf2_max_rounds() -> u64 {
        crate::literals::datum(crate::literals::DATA_KEYSTORE_PBKDF2_MAX_ROUNDS).unwrap_or(0)
    }

    pub fn scrypt_cost_bounded(n: u64, r: u64, p: u64) -> bool {
        let memory = 128u64.checked_mul(r).and_then(|x| x.checked_mul(n));
        r >= 1 && (1..=scrypt_max_p()).contains(&p) && memory.is_some_and(|m| m <= scrypt_max_memory()) && memory.and_then(|m| m.checked_mul(p)).is_some_and(|w| w <= scrypt_max_work())
    }

    pub fn pbkdf2_rounds_bounded(c: u64) -> bool {
        (1..=pbkdf2_max_rounds()).contains(&c)
    }

    pub const VERSION: &str = "version";
    pub const ID: &str = "id";
    pub const ADDRESS: &str = "address";
    pub const CRYPTO: &str = "crypto";
    pub const CIPHER: &str = "cipher";
    pub const CIPHERPARAMS: &str = "cipherparams";
    pub const IV: &str = "iv";
    pub const CIPHERTEXT: &str = "ciphertext";
    pub const KDF: &str = "kdf";
    pub const KDFPARAMS: &str = "kdfparams";
    pub const MAC: &str = "mac";
    pub const DKLEN: &str = "dklen";
    pub const SALT: &str = "salt";
    pub const N: &str = "n";
    pub const R: &str = "r";
    pub const P: &str = "p";
    pub const C: &str = "c";
    pub const PRF: &str = "prf";
    pub const AES_128_CTR: &str = "aes-128-ctr";
    pub const SCRYPT: &str = "scrypt";
    pub const PBKDF2: &str = "pbkdf2";
    pub const HMAC_SHA256: &str = "hmac-sha256";
    /// scrypt parameters for writing: `n` 2^18, `r` 8, `p` 1 (geth's standard file).
    pub const WRITE_LOG_N: u8 = 18;
    pub const WRITE_R: u32 = 8;
    pub const WRITE_P: u32 = 1;
}

fn bare_hex(v: Option<&crate::json::Value>) -> Option<Vec<u8>> {
    let s = v?.as_str()?;
    crate::hexform::decode_digits(s.strip_prefix("0x").unwrap_or(s).as_bytes())
}

fn int_of(v: Option<&crate::json::Value>) -> Option<u64> {
    match v? {
        crate::json::Value::Int(n) => Some(*n),
        _ => None,
    }
}

fn aes_ctr(key: &[u8], iv: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    use ctr::cipher::{KeyIvInit, StreamCipher};
    let mut c = ctr::Ctr128BE::<aes::Aes128>::new_from_slices(key, iv).ok()?;
    let mut out = data.to_vec();
    c.apply_keystream(&mut out);
    Some(out)
}

/// A key file's form, read member by member without any derivation: the members whose form is not v3's
/// ([`KeystoreError::Form`]'s list, empty when none), the cost bounds it breaks, and the written address (`None`
/// when absent). An address present but malformed is a form fault; absent is fine (some tools never write it).
pub struct KeystoreShape {
    pub bad: Vec<&'static str>,
    pub over: Vec<&'static str>,
    pub address: Option<Vec<u8>>,
}

/// Whether `n` has scrypt's own shape together with `r`: a power of two from 2, and below 2^(16·r).
fn scrypt_n_shaped(n: u64, r: u64) -> bool {
    n >= 2 && n.is_power_of_two() && (r == 0 || r >= 4 || u128::from(n) < (1u128 << (16 * r as u32)))
}

/// Which of the family's scrypt bounds `n`, `r`, `p` break (the registry's numbers), empty when none.
fn scrypt_over(n: u64, r: u64, p: u64) -> Vec<&'static str> {
    if keystore::scrypt_cost_bounded(n, r, p) {
        return Vec::new();
    }
    let mut bad = Vec::new();
    if r < 1 {
        bad.push("r");
    }
    if !(1..=keystore::scrypt_max_p()).contains(&p) {
        bad.push("p");
    }
    let memory = 128u64.checked_mul(r).and_then(|x| x.checked_mul(n));
    if memory.is_none_or(|m| m > keystore::scrypt_max_memory()) {
        bad.push("memory");
    }
    if memory.and_then(|m| m.checked_mul(p)).is_none_or(|w| w > keystore::scrypt_max_work()) {
        bad.push("work");
    }
    if bad.is_empty() {
        bad.push("work");
    }
    bad
}

/// Read a key file's form (no derivation). Bytes that are not an object with `crypto`, `kdfparams` and
/// `cipherparams` objects are `Form(["json"])`.
pub fn keystore_shape(bytes: &[u8]) -> Result<KeystoreShape, KeystoreError> {
    use crate::json::Value;
    use keystore as k;
    let json = || KeystoreError::Form(vec!["json"]);
    let v = crate::json::read_open(bytes).filter(|v| v.as_obj().is_some()).ok_or_else(json)?;
    let c = v.get(k::CRYPTO).filter(|c| c.as_obj().is_some()).ok_or_else(json)?;
    let kp = c.get(k::KDFPARAMS).filter(|p| p.as_obj().is_some()).ok_or_else(json)?;
    let cp = c.get(k::CIPHERPARAMS).filter(|p| p.as_obj().is_some()).ok_or_else(json)?;
    let text = |o: &Value, m: &str| o.get(m).and_then(Value::as_str).unwrap_or_default().to_string();
    // A hex member as written: absent, not text or empty is `None`; not hex is `Some(None)`; else its length.
    let hex_len = |o: &Value, m: &str| match o.get(m).and_then(Value::as_str).filter(|s| !s.is_empty()) {
        Some(_) => Some(bare_hex(o.get(m)).map(|b| b.len())),
        None => None,
    };
    let kdf = text(c, k::KDF);
    let (scrypt, pbkdf2) = (kdf == k::SCRYPT, kdf == k::PBKDF2);
    let (n, r, p, cc) = (int_of(kp.get(k::N)), int_of(kp.get(k::R)), int_of(kp.get(k::P)), int_of(kp.get(k::C)));
    let mut bad = Vec::new();
    if scrypt && r.is_none() {
        bad.push("r");
    }
    if scrypt && p.is_none() {
        bad.push("p");
    }
    if pbkdf2 && text(kp, k::PRF) != k::HMAC_SHA256 {
        bad.push("prf");
    }
    if pbkdf2 && cc.is_none() {
        bad.push("c");
    }
    if !matches!(hex_len(c, k::MAC), Some(Some(32))) {
        bad.push("mac");
    }
    if !matches!(hex_len(c, k::CIPHERTEXT), Some(Some(l)) if l > 0) {
        bad.push("ciphertext");
    }
    if int_of(v.get(k::VERSION)) != Some(3) {
        bad.push("version");
    }
    if !scrypt && !pbkdf2 {
        bad.push("kdf");
    }
    if text(c, k::CIPHER) != k::AES_128_CTR {
        bad.push("cipher");
    }
    if scrypt && !scrypt_n_shaped(n.unwrap_or(0), r.unwrap_or(0)) {
        bad.push("n");
    }
    if int_of(kp.get(k::DKLEN)) != Some(32) {
        bad.push("dklen");
    }
    if !matches!(hex_len(kp, k::SALT), Some(Some(l)) if l > 0) {
        bad.push("salt");
    }
    if !matches!(hex_len(cp, k::IV), Some(Some(16))) {
        bad.push("iv");
    }
    let address_text = text(&v, k::ADDRESS);
    let address = bare_hex(v.get(k::ADDRESS)).filter(|a| a.len() == 20);
    if !address_text.is_empty() && address.is_none() {
        bad.push("address");
    }
    let over = if pbkdf2 {
        if cc.is_some_and(keystore::pbkdf2_rounds_bounded) { Vec::new() } else { vec!["c"] }
    } else if scrypt {
        scrypt_over(n.unwrap_or(0), r.unwrap_or(0), p.unwrap_or(0))
    } else {
        Vec::new()
    };
    Ok(KeystoreShape { bad, over, address })
}

/// keystore v3 decryption, the one entry: the password's length, the form member by member, the cost bounds, then
/// the derivation; the MAC (keccak256 of the derived key's second half and the ciphertext) before anything is
/// decrypted; AES-128-CTR yields the secret, which must be a 32-byte private key in range matching any written
/// address. Every refusal before the derivation costs nothing.
fn keystore_open(bytes: &[u8], password: &[u8]) -> Result<zeroize::Zeroizing<[u8; 32]>, KeystoreError> {
    use keystore as k;
    if password.len() > KEYSTORE_PASSWORD_MAX {
        return Err(KeystoreError::PasswordLong);
    }
    let shape = keystore_shape(bytes)?;
    if !shape.bad.is_empty() {
        return Err(KeystoreError::Form(shape.bad));
    }
    if !shape.over.is_empty() {
        return Err(KeystoreError::Params(shape.over));
    }
    let v = crate::json::read_open(bytes).ok_or(KeystoreError::Form(vec!["json"]))?;
    let c = v.get(k::CRYPTO).ok_or(KeystoreError::Form(vec!["json"]))?;
    let p = c.get(k::KDFPARAMS).ok_or(KeystoreError::Form(vec!["json"]))?;
    let iv = bare_hex(c.get(k::CIPHERPARAMS).and_then(|x| x.get(k::IV))).ok_or(KeystoreError::Form(vec!["iv"]))?;
    let ciphertext = bare_hex(c.get(k::CIPHERTEXT)).ok_or(KeystoreError::Form(vec!["ciphertext"]))?;
    let mac = bare_hex(c.get(k::MAC)).ok_or(KeystoreError::Form(vec!["mac"]))?;
    let salt = bare_hex(p.get(k::SALT)).ok_or(KeystoreError::Form(vec!["salt"]))?;
    let mut dk = zeroize::Zeroizing::new([0u8; 32]);
    if c.get(k::KDF).and_then(crate::json::Value::as_str) == Some(k::PBKDF2) {
        let rounds = int_of(p.get(k::C)).and_then(|c| u32::try_from(c).ok()).ok_or(KeystoreError::Params(vec!["c"]))?;
        pbkdf2::pbkdf2_hmac::<Sha256>(password, &salt, rounds, &mut dk[..]);
    } else {
        let (n, r, pp) = (int_of(p.get(k::N)).unwrap_or(0), int_of(p.get(k::R)).unwrap_or(0), int_of(p.get(k::P)).unwrap_or(0));
        let params = scrypt::Params::new(n.trailing_zeros() as u8, u32::try_from(r).map_err(|_| KeystoreError::Params(vec!["r"]))?, u32::try_from(pp).map_err(|_| KeystoreError::Params(vec!["p"]))?, 32)
            .map_err(|_| KeystoreError::Params(vec!["work"]))?;
        scrypt::scrypt(password, &salt, &params, &mut dk[..]).map_err(|_| KeystoreError::Params(vec!["work"]))?;
    }
    let mut body = dk[16..].to_vec();
    body.extend_from_slice(&ciphertext);
    if !mac_ok(&keccak256(&body), &mac) {
        return Err(KeystoreError::Mac);
    }
    let plain = zeroize::Zeroizing::new(aes_ctr(&dk[..16], &iv, &ciphertext).ok_or(KeystoreError::Form(vec!["iv"]))?);
    let key: [u8; 32] = plain.as_slice().try_into().map_err(|_| KeystoreError::Key)?;
    let key = zeroize::Zeroizing::new(key);
    let pk = Privkey::from_bytes(&key).ok_or(KeystoreError::Key)?;
    if let Some(a) = shape.address {
        if a[..] != pk.address()[..] {
            return Err(KeystoreError::Address);
        }
    }
    Ok(key)
}

/// A keystore v3 file read as one private key: thirty-two bytes, in [1, n − 1], matching any written address.
pub fn keystore_decrypt(bytes: &[u8], password: &[u8]) -> Result<[u8; 32], KeystoreError> {
    keystore_open(bytes, password).map(|k| *k)
}

#[cfg(not(feature = "ablate-k1-keystore"))]
fn mac_ok(computed: &[u8; 32], given: &[u8]) -> bool {
    computed[..] == given[..]
}

/// Fault-injection variant for the conformance gauges: the MAC is not checked.
#[cfg(feature = "ablate-k1-keystore")]
fn mac_ok(computed: &[u8; 32], given: &[u8]) -> bool {
    let _ = (computed, given);
    true
}

/// A secret read from a file (passcode, mnemonic): the file bytes minus at most one trailing line end
/// (`\r\n` or `\n`; editors usually save one).
pub fn file_secret(b: &[u8]) -> &[u8] {
    crate::marks::mark(crate::marks::Feature::V6);
    b.strip_suffix(b"\r\n").or_else(|| b.strip_suffix(b"\n")).unwrap_or(b)
}

/// keystore v3 encryption: scrypt (`n` 2^18, `r` 8, `p` 1), random salt and IV, random UUID v4; answers the
/// canonical file bytes. A key file holds a private key and nothing else.
pub fn keystore_encrypt(key: &Privkey, password: &[u8]) -> Result<Vec<u8>, KeystoreError> {
    keystore_seal(&key.0, &key.address(), password)
}

/// Seal a 32-byte private key as keystore v3 with its address. A password over [`KEYSTORE_PASSWORD_MAX`] bytes is
/// refused by name before any work.
pub fn keystore_seal(secret: &[u8; 32], address: &[u8; 20], password: &[u8]) -> Result<Vec<u8>, KeystoreError> {
    if password.len() > KEYSTORE_PASSWORD_MAX {
        return Err(KeystoreError::PasswordLong);
    }
    keystore_sealed(secret, address, password).ok_or(KeystoreError::Random)
}

fn keystore_sealed(secret: &[u8; 32], address: &[u8; 20], password: &[u8]) -> Option<Vec<u8>> {
    use crate::json::Value;
    use keystore as k;
    let rnd = random_bytes(64).ok()?;
    let (salt, iv, id) = (&rnd[..32], &rnd[32..48], &rnd[48..64]);
    let params = scrypt::Params::new(k::WRITE_LOG_N, k::WRITE_R, k::WRITE_P, 32).ok()?;
    let mut dk = [0u8; 32];
    scrypt::scrypt(password, salt, &params, &mut dk).ok()?;
    let ciphertext = aes_ctr(&dk[..16], iv, &secret[..])?;
    let mut body = dk[16..].to_vec();
    body.extend_from_slice(&ciphertext);
    let mac = keccak256(&body);
    let bare = |b: &[u8]| Value::Str(crate::hexform::encode(b)[2..].to_string());
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(id);
    uuid[6] = (uuid[6] & 0x0f) | 0x40;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;
    let u = crate::hexform::encode(&uuid)[2..].to_string();
    let uuid_str = format!("{}-{}-{}-{}-{}", &u[..8], &u[8..12], &u[12..16], &u[16..20], &u[20..]);
    let o = |m: Vec<(&str, Value)>| Value::Obj(m.into_iter().map(|(a, b)| (a.to_string(), b)).collect());
    let doc = o(vec![
        (k::ADDRESS, bare(address)),
        (
            k::CRYPTO,
            o(vec![
                (k::CIPHER, Value::Str(k::AES_128_CTR.into())),
                (k::CIPHERPARAMS, o(vec![(k::IV, bare(iv))])),
                (k::CIPHERTEXT, bare(&ciphertext)),
                (k::KDF, Value::Str(k::SCRYPT.into())),
                (k::KDFPARAMS, o(vec![(k::DKLEN, Value::Int(32)), (k::N, Value::Int(1 << k::WRITE_LOG_N)), (k::P, Value::Int(u64::from(k::WRITE_P))), (k::R, Value::Int(u64::from(k::WRITE_R))), (k::SALT, bare(salt))])),
                (k::MAC, bare(&mac)),
            ]),
        ),
        (k::ID, Value::Str(uuid_str)),
        (k::VERSION, Value::Int(3)),
    ]);
    crate::json::canonical(&doc).ok()
}

#[cfg(test)]
mod key_tests {
    use super::*;

    #[test]
    fn keystore_cost_bounds_and_secret_line_endings() {
        assert!(keystore::scrypt_cost_bounded(1 << 18, 8, 1), "常见钱包的档在界内");
        assert!(!keystore::scrypt_cost_bounded(1 << 40, 8, 1), "一 TiB 的内存越界");
        assert!(!keystore::scrypt_cost_bounded(2, 1, 1 << 29), "p 越界");
        assert!(keystore::scrypt_cost_bounded(2, 1, keystore::scrypt_max_p()) && !keystore::scrypt_cost_bounded(2, 1, keystore::scrypt_max_p() + 1), "p 的界两侧");
        assert!(!keystore::scrypt_cost_bounded(2, 1, 0) && !keystore::scrypt_cost_bounded(2, 0, 1), "p、r 为零");
        assert!(keystore::scrypt_cost_bounded(1 << 23, 1, 1) && !keystore::scrypt_cost_bounded(1 << 24, 1, 1), "内存 1 GiB 的界两侧");
        assert!(!keystore::scrypt_cost_bounded(u64::MAX, 2, 1), "128·r·n 溢出 u64 即越界");
        assert!(keystore::pbkdf2_rounds_bounded(1) && keystore::pbkdf2_rounds_bounded(keystore::pbkdf2_max_rounds()), "轮数的界内两端");
        assert!(!keystore::pbkdf2_rounds_bounded(0) && !keystore::pbkdf2_rounds_bounded(keystore::pbkdf2_max_rounds() + 1) && !keystore::pbkdf2_rounds_bounded(1 << 32), "零、界外一与折回 u32 的 2^32");
        let kdf = |n: &str, c: &str| format!(r#"{{"version":3,"crypto":{{"cipher":"aes-128-ctr","cipherparams":{{"iv":"00000000000000000000000000000000"}},"ciphertext":"00","mac":"{}","kdf":"{n}","kdfparams":{{"dklen":32,"salt":"00",{c}}}}}}}"#, "00".repeat(32));
        assert!(matches!(keystore_decrypt(kdf("scrypt", r#""n":1099511627776,"r":8,"p":1"#).as_bytes(), b"pw"), Err(KeystoreError::Params(f)) if f.contains(&"memory")));
        assert_eq!(keystore_decrypt(kdf("pbkdf2", r#""prf":"hmac-sha256","c":4294967295"#).as_bytes(), b"pw"), Err(KeystoreError::Params(vec!["c"])));
        assert_eq!(file_secret(b"pw\r\n"), b"pw");
        assert_eq!(file_secret(b"pw\n"), b"pw");
        assert_eq!(file_secret(b"pw"), b"pw");
    }

    #[test]
    fn bip39_trezor_first_vector_and_walls() {
        let m = mnemonic_from_entropy(&[0u8; 16]).unwrap();
        assert_eq!(m, "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about");
        assert_eq!(mnemonic_entropy(&m).unwrap(), vec![0u8; 16]);
        let seed = mnemonic_seed(&m, "TREZOR").unwrap();
        assert_eq!(crate::hexform::encode(&seed), "0xc55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04");
        assert_eq!(bip32_master(&seed).unwrap().xprv(), "xprv9s21ZrQH143K3h3fDYiay8mocZ3afhfULfb5GX8kCBdno77K4HiA15Tg23wpbeF1pLfs1c5SPmYHrEpTuuRhxMwvKDwqdKiGJS9XFKzUsAF");
        assert_eq!(mnemonic_entropy("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon"), Err(MnemonicError::Checksum));
        assert_eq!(mnemonic_entropy("abandon abandon"), Err(MnemonicError::WordCount));
        assert_eq!(mnemonic_entropy("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandonx"), Err(MnemonicError::UnknownWord(11)));
    }

    #[test]
    fn bip32_vector_one_and_paths() {
        let seed = crate::hexform::decode_digits(b"000102030405060708090a0b0c0d0e0f").unwrap();
        assert_eq!(derive_path(&seed, "m/0'").unwrap().xprv(), "xprv9uHRZZhk6KAJC1avXpDAp4MDc3sQKNxDiPvvkX8Br5ngLNv1TxvUxt4cV1rGL5hj6KCesnDYUhd7oWgT11eZG7XnxHrnYeSvkzY7d2bhkJ7");
        assert_eq!(derive_path(&seed, "m/0'/1").unwrap().xprv(), "xprv9wTYmMFdV23N2TdNG573QoEsfRrWKQgWeibmLntzniatZvR9BmLnvSxqu53Kw1UmYPxLgboyZQaXwTCg8MSY3H2EU4pWcQDnRnrVA1xe8fs");
        assert_eq!(parse_path("m/2147483647'"), Some(vec![u32::MAX]));
        assert_eq!(parse_path("m/2147483648"), None);
        assert_eq!(parse_path("m/01"), None);
        assert_eq!(parse_path("44'/60'"), None);
    }

    #[test]
    fn keystore_round_trip_and_walls() {
        let mut kb = [0u8; 32];
        kb[31] = 7;
        let key = Privkey::from_bytes(&kb).unwrap();
        let file = keystore_encrypt(&key, b"pw").unwrap();
        assert_eq!(keystore_decrypt(&file, b"pw"), Ok(kb));
        assert_eq!(keystore_decrypt(&file, b"wrong"), Err(KeystoreError::Mac));
        let v2 = String::from_utf8(file.clone()).unwrap().replace("\"version\":3", "\"version\":2");
        assert_eq!(keystore_decrypt(v2.as_bytes(), b"pw"), Err(KeystoreError::Form(vec!["version"])));
        let kdf = String::from_utf8(file).unwrap().replace("\"kdf\":\"scrypt\"", "\"kdf\":\"argon2\"");
        assert!(matches!(keystore_decrypt(kdf.as_bytes(), b"pw"), Err(KeystoreError::Form(f)) if f.contains(&"kdf")));
    }

    fn hex(s: &str) -> Vec<u8> {
        crate::hexform::decode_digits(s.as_bytes()).expect("测试向量的十六进制")
    }

    /// The two primitives against printed vectors: SP 800-38A F.5.1 (CTR-AES128, first block) and RFC 7914 §11's
    /// first two scrypt sets.
    #[test]
    fn aes_ctr_and_scrypt_match_the_printed_vectors() {
        let out = aes_ctr(&hex("2b7e151628aed2a6abf7158809cf4f3c"), &hex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"), &hex("6bc1bee22e409f96e93d7e117393172a")).unwrap();
        assert_eq!(out, hex("874d6191b620e3261bef6864990db6ce"));
        let mut a = [0u8; 64];
        scrypt::scrypt(b"", b"", &scrypt::Params::new(4, 1, 1, 64).unwrap(), &mut a).unwrap();
        assert_eq!(a.to_vec(), hex("77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"));
        let mut b = [0u8; 64];
        scrypt::scrypt(b"password", b"NaCl", &scrypt::Params::new(10, 8, 16, 64).unwrap(), &mut b).unwrap();
        assert_eq!(b.to_vec(), hex("fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b3731622eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640"));
    }

    const VECTOR: &[u8] = br#"{"crypto":{"cipher":"aes-128-ctr","cipherparams":{"iv":"6087dab2f9fdbbfaddc31a909735c1e6"},"ciphertext":"5318b4d5bcd28de64ee5559e671353e16f075ecae9f99c7a79a38af5f869aa46","kdf":"pbkdf2","kdfparams":{"c":262144,"dklen":32,"prf":"hmac-sha256","salt":"ae3cd4e7013836a3df6bd7241b12db061dbe2c6785853cce422d148a624ce0bd"},"mac":"517ead924a9d0dc3124507e3393d175ce3ff7c1e96529c6c555ce9e51205e9b2"},"id":"3198bc9c-6672-5ab3-d995-4942343ae5b6","version":3}"#;

    /// The published v3 pbkdf2 vector (Web3 Secret Storage, `testpassword`) opens to its published key; a wrong
    /// password is the MAC, asked before anything is decrypted.
    #[test]
    fn the_published_pbkdf2_vector_opens_to_its_key() {
        assert_eq!(keystore_decrypt(VECTOR, b"testpassword").unwrap().to_vec(), hex("7a28b5ba57c53603b0b07b56bba752f7784bf506fa95edc395f5cf6c7514fe9d"));
        assert_eq!(keystore_decrypt(VECTOR, b"wrongpassword"), Err(KeystoreError::Mac));
    }

    /// Every member's form refused by its own name before any work: absent, not text, wrong length; `n` not a power
    /// of two; `dklen` not 32; pbkdf2's rounds at 2^24 taken and one past refused; the scrypt bounds named.
    #[test]
    fn each_member_is_refused_by_its_own_name() {
        let base = String::from_utf8(VECTOR.to_vec()).unwrap();
        let form = |from: &str, to: &str| {
            assert!(base.contains(from), "{from}");
            match keystore_shape(base.replace(from, to).as_bytes()) {
                Ok(s) => s.bad,
                Err(e) => e.fields().to_vec(),
            }
        };
        for (from, to, name) in [
            (r#""version":3"#, r#""version":"3""#, "version"),
            (r#""cipher":"aes-128-ctr""#, r#""cipher":"aes-256-ctr""#, "cipher"),
            (r#""iv":"6087dab2f9fdbbfaddc31a909735c1e6""#, r#""iv":"6087""#, "iv"),
            (r#""iv":"6087dab2f9fdbbfaddc31a909735c1e6""#, r#""iv":16"#, "iv"),
            (r#""ciphertext":"5318b4d5bcd28de64ee5559e671353e16f075ecae9f99c7a79a38af5f869aa46""#, r#""ciphertext":"""#, "ciphertext"),
            (r#""mac":"517ead924a9d0dc3124507e3393d175ce3ff7c1e96529c6c555ce9e51205e9b2""#, r#""mac":"517e""#, "mac"),
            (r#""mac":"517ead924a9d0dc3124507e3393d175ce3ff7c1e96529c6c555ce9e51205e9b2""#, r#""mac":"zz""#, "mac"),
            (r#""salt":"ae3cd4e7013836a3df6bd7241b12db061dbe2c6785853cce422d148a624ce0bd""#, r#""salt":"""#, "salt"),
            (r#""dklen":32"#, r#""dklen":16"#, "dklen"),
            (r#""prf":"hmac-sha256""#, r#""prf":"hmac-sha512""#, "prf"),
            (r#""c":262144"#, r#""c":"262144""#, "c"),
            (r#""kdf":"pbkdf2""#, r#""kdf":"PBKDF2""#, "kdf"),
            (r#""version":3"#, r#""address":"0x12","version":3"#, "address"),
        ] {
            assert!(form(from, to).contains(&name), "{name}: {from} → {to}");
        }
        let scrypt = |n: u64, r: u64, p: u64| base.replace(r#""kdf":"pbkdf2""#, r#""kdf":"scrypt""#).replace(r#""c":262144,"#, &format!(r#""n":{n},"p":{p},"r":{r},"#));
        assert!(keystore_shape(scrypt(1000, 8, 1).as_bytes()).unwrap().bad.contains(&"n"), "n 不是二的幂");
        assert!(keystore_shape(scrypt(1 << 18, 8, 1).as_bytes()).unwrap().bad.is_empty());
        assert!(keystore_shape(scrypt(1 << 18, 8, 1).as_bytes()).unwrap().over.is_empty(), "常见钱包的档在界内");
        assert!(keystore_shape(scrypt(1 << 23, 1, 1).as_bytes()).unwrap().over.is_empty(), "内存恰 1 GiB 收");
        assert_eq!(keystore_shape(scrypt(1 << 24, 1, 1).as_bytes()).unwrap().over, vec!["memory"], "多一倍即越界");
        assert!(keystore_shape(scrypt(2, 1, keystore::scrypt_max_p()).as_bytes()).unwrap().over.is_empty(), "p 恰界");
        assert!(keystore_shape(scrypt(2, 1, keystore::scrypt_max_p() + 1).as_bytes()).unwrap().over.contains(&"p"), "p 越界");
        assert!(keystore_shape(scrypt(2, 0, 1).as_bytes()).unwrap().over.contains(&"r"), "r 为零");
        let pbkdf2 = |c: u64| base.replace(r#""c":262144"#, &format!(r#""c":{c}"#));
        assert!(keystore_shape(pbkdf2(1 << 24).as_bytes()).unwrap().over.is_empty(), "2^24 恰好收");
        assert_eq!(keystore_shape(pbkdf2((1 << 24) + 1).as_bytes()).unwrap().over, vec!["c"], "多一拒");
        assert_eq!(keystore_shape(pbkdf2(0).as_bytes()).unwrap().over, vec!["c"], "零拒");
    }

    /// The password's bound on both sides of the one derivation entry: exactly 1024 bytes is taken, 1025 refused by
    /// name before any work, never cut.
    #[test]
    fn a_password_of_1024_bytes_is_taken_and_1025_refused_on_both_sides() {
        let key = Privkey::from_bytes(&[9u8; 32]).unwrap();
        let (at, over) = (vec![b'p'; KEYSTORE_PASSWORD_MAX], vec![b'p'; KEYSTORE_PASSWORD_MAX + 1]);
        assert_eq!(keystore_encrypt(&key, &over), Err(KeystoreError::PasswordLong));
        let file = keystore_encrypt(&key, &at).unwrap();
        assert_eq!(keystore_decrypt(&file, &at), Ok([9u8; 32]));
        assert_eq!(keystore_decrypt(&file, &over), Err(KeystoreError::PasswordLong), "多一字节即拒,不截断后去对");
    }

    /// A key file holds a 32-byte private key: one sealing sixteen bytes (an older mnemonic file's entropy) is
    /// refused by name; and not one byte of the private key is in what is written, raw or in hex of either case.
    #[test]
    fn only_a_private_key_opens_and_none_of_it_is_written_in_the_clear() {
        let raw: [u8; 32] = [0x4c, 0x0b, 0x39, 0x8f, 0x21, 0x77, 0x0a, 0x5e, 0x11, 0x2c, 0x63, 0x84, 0x9d, 0x02, 0xb7, 0x1f, 0x88, 0x45, 0xea, 0x30, 0x7c, 0x59, 0x16, 0xd3, 0x6a, 0xf1, 0x08, 0x4b, 0x92, 0x27, 0xc5, 0x60];
        let key = Privkey::from_bytes(&raw).unwrap();
        let file = keystore_encrypt(&key, b"pw").unwrap();
        let bare = crate::hexform::encode(&raw)[2..].to_string();
        let text = String::from_utf8_lossy(&file).to_string();
        assert!(!text.contains(&bare) && !text.contains(&bare.to_uppercase()), "明文私钥的十六进制");
        assert!(!file.windows(32).any(|w| w == raw), "明文私钥的裸字节");
        let sixteen = keystore_sealed_raw(&[7u8; 16], b"pw");
        assert_eq!(keystore_decrypt(&sixteen, b"pw"), Err(KeystoreError::Key), "十六字节熵的旧档具名拒");
    }

    /// An older product's file: sixteen bytes sealed the same way (no address written).
    fn keystore_sealed_raw(secret: &[u8], password: &[u8]) -> Vec<u8> {
        use crate::json::Value;
        use keystore as k;
        let (salt, iv) = ([1u8; 32], [2u8; 16]);
        let params = scrypt::Params::new(4, 8, 1, 32).unwrap();
        let mut dk = [0u8; 32];
        scrypt::scrypt(password, &salt, &params, &mut dk).unwrap();
        let ciphertext = aes_ctr(&dk[..16], &iv, secret).unwrap();
        let mut body = dk[16..].to_vec();
        body.extend_from_slice(&ciphertext);
        let bare = |b: &[u8]| Value::Str(crate::hexform::encode(b)[2..].to_string());
        let o = |m: Vec<(&str, Value)>| Value::Obj(m.into_iter().map(|(a, b)| (a.to_string(), b)).collect());
        crate::json::canonical(&o(vec![
            (k::CRYPTO, o(vec![(k::CIPHER, Value::Str(k::AES_128_CTR.into())), (k::CIPHERPARAMS, o(vec![(k::IV, bare(&iv))])), (k::CIPHERTEXT, bare(&ciphertext)), (k::KDF, Value::Str(k::SCRYPT.into())), (k::KDFPARAMS, o(vec![(k::DKLEN, Value::Int(32)), (k::N, Value::Int(16)), (k::P, Value::Int(1)), (k::R, Value::Int(8)), (k::SALT, bare(&salt))])), (k::MAC, bare(&keccak256(&body)))])),
            (k::VERSION, Value::Int(3)),
        ]))
        .unwrap()
    }
}

// Vault cryptography (outside the law; never a decision input).

/// The bech32 human-readable prefix of an age identity string (lowercase `AGE-SECRET-KEY-1…`; the
/// separator `1` is added by the encoder).
const AGE_SECRET_HRP: &str = "age-secret-key-";

/// The bech32 (BIP-173) alphabet and generator. Used only for age identity strings: a checksummed
/// encoding like [`base58`] rather than cryptography, so it lives here instead of pulling in another crate.
const BECH32_CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const BECH32_GENERATOR: [u32; 5] = [0x3b6a_57b2, 0x2650_8e6d, 0x1ea1_19fa, 0x3d42_33dd, 0x2a14_62b3];

fn bech32_polymod(values: &[u8]) -> u32 {
    let mut chk: u32 = 1;
    for v in values {
        let b = chk >> 25;
        chk = ((chk & 0x01ff_ffff) << 5) ^ u32::from(*v);
        for (i, g) in BECH32_GENERATOR.iter().enumerate() {
            if (b >> i) & 1 == 1 {
                chk ^= *g;
            }
        }
    }
    chk
}

/// Regroup 8-bit bytes into 5-bit groups (bech32 payload); the last group is zero-padded.
fn bits8_to_5(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 8 / 5 + 1);
    let (mut acc, mut bits) = (0u32, 0u32);
    for b in data {
        acc = (acc << 8) | u32::from(*b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 31) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 31) as u8);
    }
    out
}

/// bech32 encoding (lowercase). `hrp` must be lowercase ASCII.
fn bech32(hrp: &str, data: &[u8]) -> String {
    let payload = bits8_to_5(data);
    let mut values: Vec<u8> = hrp.bytes().map(|c| c >> 5).chain(std::iter::once(0)).chain(hrp.bytes().map(|c| c & 31)).collect();
    values.extend_from_slice(&payload);
    values.extend_from_slice(&[0u8; 6]);
    let polymod = bech32_polymod(&values) ^ 1;
    let checksum: Vec<u8> = (0..6).map(|i| ((polymod >> (5 * (5 - i))) & 31) as u8).collect();
    let mut s = String::with_capacity(hrp.len() + 1 + payload.len() + 6);
    s.push_str(hrp);
    s.push('1');
    s.extend(payload.iter().chain(checksum.iter()).map(|v| char::from(BECH32_CHARSET[*v as usize])));
    s
}

/// Vault cryptography. Capabilities only, bytes in and bytes out: the vault file's fields and its
/// reading and writing belong to the store, and this module knows no file or field name.
///
/// Five parts: passcode derivation (scrypt, calibration moves only N), sealing (XChaCha20-Poly1305,
/// one AEAD and only one), two HKDF-SHA256 derivations (recovery key and age identity), the name key and
/// the second sealed-envelope form, and wiping.
pub mod vault {
    use super::{keystore, random_bytes, AgeIdentity, AGE_SECRET_HRP};
    use zeroize::Zeroize;
    /// The carrier of secrets: wiped by `zeroize` on drop. A secret leaves this module only in this type,
    /// so no caller has to remember to wipe it.
    pub use zeroize::Zeroizing;

    /// Two scrypt dimensions are fixed; calibration moves only N.
    pub const KDF_R: u32 = 8;
    pub const KDF_P: u32 = 1;
    pub const CALIBRATE_TARGET_MS: u128 = 1_000;
    /// Byte length of the master key, the passcode key and the recovery key (the master key is thirty-two
    /// random bytes).
    pub const KEY_LEN: usize = 32;
    pub const SALT_LEN: usize = 32;
    pub const NONCE_LEN: usize = 24;
    pub const TAG_LEN: usize = 16;

    /// Lower bound of calibration: never weaker than the product's own keystore v3 files
    /// ([`keystore::WRITE_LOG_N`], geth's standard).
    pub const fn min_log_n() -> u8 {
        keystore::WRITE_LOG_N
    }

    /// Upper bound of calibration: with `r` and `p` fixed, the largest log2 N the cost bound
    /// ([`keystore::scrypt_cost_bounded`]) admits. The bound has one owner; this asks it instead of
    /// writing a number.
    pub fn max_log_n() -> u8 {
        (1u8..=63).take_while(|l| 1u64.checked_shl(u32::from(*l)).is_some_and(|n| keystore::scrypt_cost_bounded(n, u64::from(KDF_R), u64::from(KDF_P)))).last().unwrap_or(0)
    }

    /// The cost gate: calibrated values and values read from a file both pass it, and the upper bound wins.
    #[cfg(not(feature = "ablate-k6-passcode-kdf"))]
    pub fn admits(log_n: u8) -> bool {
        log_n >= min_log_n() && 1u64.checked_shl(u32::from(log_n)).is_some_and(|n| keystore::scrypt_cost_bounded(n, u64::from(KDF_R), u64::from(KDF_P)))
    }

    /// Fault-injection variant for the conformance gauges: the upper bound is not asked and `r` and `p` are
    /// not the fixed ones; every derivation uses one weak setting.
    #[cfg(feature = "ablate-k6-passcode-kdf")]
    pub fn admits(log_n: u8) -> bool {
        let _ = log_n;
        true
    }

    /// The setting the fault-injection variant uses (weak enough not to wait).
    #[cfg(feature = "ablate-k6-passcode-kdf")]
    const ABLATED_LOG_N: u8 = 14;

    /// Wipe after use: a secret's in-memory form passes here before it goes. From outside the process this
    /// is all that can be observed: the same memory handed out after use is all zeros. Every wipe in this
    /// crate goes through this function (secrets returned to callers are wiped by [`Zeroizing`] with the
    /// same `zeroize` on drop).
    #[cfg(not(feature = "ablate-k6-zeroize"))]
    pub fn scrub<Z: Zeroize + ?Sized>(secret: &mut Z) {
        secret.zeroize();
    }

    /// Fault-injection variant for the conformance gauges: nothing is wiped.
    #[cfg(feature = "ablate-k6-zeroize")]
    pub fn scrub<Z: Zeroize + ?Sized>(secret: &mut Z) {
        let _ = secret;
    }

    /// Calibrate N: from the lower bound up until one derivation takes about [`CALIBRATE_TARGET_MS`]
    /// milliseconds or the upper bound is hit. Answers log2 N; the result itself passes [`admits`], and
    /// hitting the upper bound refuses (`None`).
    pub fn calibrate_log_n() -> Option<u8> {
        let salt = [0u8; SALT_LEN];
        let mut chosen = min_log_n();
        let ceiling = max_log_n();
        loop {
            let started = std::time::Instant::now();
            passcode_key(b"calibration", &salt, chosen)?;
            if started.elapsed().as_millis() >= CALIBRATE_TARGET_MS || chosen >= ceiling {
                break;
            }
            chosen += 1;
        }
        admits(chosen).then_some(chosen)
    }

    /// Passcode key P = scrypt(passcode, salt) with `r` [`KDF_R`] and `p` [`KDF_P`] fixed; only N follows
    /// calibration. An empty passcode or salt, or an out-of-bounds cost, is `None` (never a weaker fallback).
    pub fn passcode_key(passcode: &[u8], salt: &[u8], log_n: u8) -> Option<Zeroizing<[u8; KEY_LEN]>> {
        if passcode.is_empty() || salt.is_empty() || !admits(log_n) {
            return None;
        }
        #[cfg(feature = "ablate-k6-passcode-kdf")]
        let (log_n, r, p) = (ABLATED_LOG_N, 1u32, 1u32);
        #[cfg(not(feature = "ablate-k6-passcode-kdf"))]
        let (log_n, r, p) = (log_n, KDF_R, KDF_P);
        let params = scrypt::Params::new(log_n, r, p, KEY_LEN).ok()?;
        let mut out = Zeroizing::new([0u8; KEY_LEN]);
        scrypt::scrypt(passcode, salt, &params, out.as_mut()).ok()?;
        Some(out)
    }

    /// Whole-machine backup key K = scrypt(backup password, salt) with `r` [`KDF_R`] and `p` [`KDF_P`] fixed;
    /// only N is calibrated and it is stored in the file header. Same derivation and same cost gate as the
    /// passcode key.
    pub fn backup_key(password: &[u8], salt: &[u8], log_n: u8) -> Option<Zeroizing<[u8; KEY_LEN]>> {
        passcode_key(password, salt, log_n)
    }

    /// One sealed value: nonce and ciphertext. Every seal draws a fresh random nonce; callers cannot reuse one.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Sealed {
        pub nonce: [u8; NONCE_LEN],
        pub ciphertext: Vec<u8>,
    }

    /// Seal with XChaCha20-Poly1305 (one AEAD and only one). `None` without a random source.
    pub fn seal(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Option<Sealed> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        let raw = random_bytes(NONCE_LEN).ok()?;
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&raw);
        let cipher = chacha20poly1305::XChaCha20Poly1305::new(key.into());
        let ciphertext = cipher.encrypt(&nonce.into(), plaintext).ok()?;
        Some(Sealed { nonce, ciphertext })
    }

    /// Open: a wrong key, a wrong nonce or one flipped ciphertext bit all give `None`.
    #[cfg(not(feature = "ablate-k6-aead"))]
    pub fn open(key: &[u8; KEY_LEN], nonce: &[u8], ciphertext: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        let n: [u8; NONCE_LEN] = nonce.try_into().ok()?;
        let cipher = chacha20poly1305::XChaCha20Poly1305::new(key.into());
        cipher.decrypt(&n.into(), ciphertext).ok().map(Zeroizing::new)
    }

    /// Fault-injection variant for the conformance gauges: the tag is not checked and the ciphertext minus
    /// its tag is handed out as plaintext.
    #[cfg(feature = "ablate-k6-aead")]
    pub fn open(key: &[u8; KEY_LEN], nonce: &[u8], ciphertext: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
        let _ = (key, nonce);
        (ciphertext.len() >= TAG_LEN).then(|| Zeroizing::new(ciphertext[..ciphertext.len() - TAG_LEN].to_vec()))
    }

    /// Seal local data: XChaCha20-Poly1305 with a fresh random 24-byte nonce per seal and the AAD the
    /// caller gives (the whole envelope head for local data files, `galeed_store::envelope`). `None` without a random source.
    pub fn seal_bound(key: &[u8; KEY_LEN], plaintext: &[u8], aad: &[u8]) -> Option<Sealed> {
        use chacha20poly1305::aead::{Aead, KeyInit, Payload};
        let nonce = fresh_nonce()?;
        let cipher = chacha20poly1305::XChaCha20Poly1305::new(key.into());
        let ciphertext = cipher.encrypt(&nonce.into(), Payload { msg: plaintext, aad: bound(aad) }).ok()?;
        Some(Sealed { nonce, ciphertext })
    }

    /// Open local data: a wrong key, nonce, ciphertext or AAD gives `None` (another kind or a lower version
    /// never opens).
    pub fn open_bound(key: &[u8; KEY_LEN], nonce: &[u8], ciphertext: &[u8], aad: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
        use chacha20poly1305::aead::{Aead, KeyInit, Payload};
        let n: [u8; NONCE_LEN] = nonce.try_into().ok()?;
        let cipher = chacha20poly1305::XChaCha20Poly1305::new(key.into());
        cipher.decrypt(&n.into(), Payload { msg: ciphertext, aad: bound(aad) }).ok().map(Zeroizing::new)
    }

    /// For AADs that contain the nonce (the whole-machine backup writes the nonce in its header and
    /// authenticates the whole header): the nonce is still drawn here (the same source as [`seal_bound`];
    /// callers cannot pass one in), then `aad_of` builds the AAD from it before sealing. `None` without a
    /// random source.
    pub fn seal_bound_with<F>(key: &[u8; KEY_LEN], plaintext: &[u8], aad_of: F) -> Option<Sealed>
    where
        F: FnOnce(&[u8; NONCE_LEN]) -> Vec<u8>,
    {
        use chacha20poly1305::aead::{Aead, KeyInit, Payload};
        let nonce = fresh_nonce()?;
        let aad = aad_of(&nonce);
        let cipher = chacha20poly1305::XChaCha20Poly1305::new(key.into());
        let ciphertext = cipher.encrypt(&nonce.into(), Payload { msg: plaintext, aad: bound(&aad) }).ok()?;
        Some(Sealed { nonce, ciphertext })
    }

    /// Seal with a given nonce (the envelope's head carries the nonce and is the AAD, so it is chosen before the
    /// AAD is built): answers the nonce with the ciphertext and tag. Every write draws the nonce from the
    /// system's entropy source first; a fixed nonce is only for the standard's test vectors. `None` when the AEAD
    /// refuses.
    pub fn seal_bound_at(key: &[u8; KEY_LEN], nonce: &[u8; NONCE_LEN], plaintext: &[u8], aad: &[u8]) -> Option<Sealed> {
        use chacha20poly1305::aead::{Aead, KeyInit, Payload};
        let cipher = chacha20poly1305::XChaCha20Poly1305::new(key.into());
        let ciphertext = cipher.encrypt(&(*nonce).into(), Payload { msg: plaintext, aad: bound(aad) }).ok()?;
        Some(Sealed { nonce: *nonce, ciphertext })
    }

    fn fresh_nonce() -> Option<[u8; NONCE_LEN]> {
        let raw = random_bytes(NONCE_LEN).ok()?;
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&raw);
        Some(nonce)
    }

    fn bound(aad: &[u8]) -> &[u8] {
        aad
    }

    pub const SEAL_UNREADABLE: &str = "seal-unreadable";

    /// Local data key D = HKDF-SHA256(M, empty salt, purpose string `galeed/local/v1`); an empty salt is the
    /// RFC 5869 default string of zeros. `None` when the registry does not yield the purpose string.
    pub fn local_key(master: &[u8]) -> Option<Zeroizing<[u8; KEY_LEN]>> {
        if master.len() != KEY_LEN {
            return None;
        }
        Some(hkdf32(None, master, &purpose(USE_LOCAL)?))
    }

    /// Name key N: one per vault, thirty-two random bytes. `None` without a random source.
    pub fn new_name_key() -> Option<Zeroizing<[u8; KEY_LEN]>> {
        let raw = random_bytes(KEY_LEN).ok()?;
        let mut n = Zeroizing::new([0u8; KEY_LEN]);
        n.copy_from_slice(&raw);
        Some(n)
    }

    pub fn seal_name_key(master: &[u8; KEY_LEN], name_key: &[u8; KEY_LEN]) -> Option<Sealed> {
        seal(master, name_key)
    }

    /// Open the name key N under M; a wrong key, an alteration or a wrong length gives `None`.
    pub fn open_name_key(master: &[u8; KEY_LEN], nonce: &[u8], ciphertext: &[u8]) -> Option<Zeroizing<[u8; KEY_LEN]>> {
        let opened = open(master, nonce, ciphertext)?;
        let mut n = Zeroizing::new([0u8; KEY_LEN]);
        (opened.len() == KEY_LEN).then(|| n.copy_from_slice(&opened))?;
        Some(n)
    }

    /// Purpose strings live only in the literal registry and are looked up by use name here; `None` when
    /// the registry cannot be read, and callers refuse rather than derive with an empty purpose.
    fn purpose(use_name: &str) -> Option<String> {
        crate::literals::derivation_purpose(use_name)
    }

    /// Use name of the recovery-key derivation (the desk's own word; the literal lives in the registry).
    pub const USE_RECOVERY: &str = "master-recovery";
    pub const USE_AGE: &str = "age-identity";
    pub const USE_LOCAL: &str = "local-data";

    /// HKDF-SHA256 to thirty-two bytes with both salt and purpose string. The two derivations are kept
    /// apart by `info`.
    #[cfg(not(feature = "ablate-k6-derive"))]
    fn hkdf32(salt: Option<&[u8]>, ikm: &[u8], info: &str) -> Zeroizing<[u8; KEY_LEN]> {
        let mut out = Zeroizing::new([0u8; KEY_LEN]);
        let h = hkdf::Hkdf::<sha2::Sha256>::new(salt, ikm);
        h.expand(info.as_bytes(), out.as_mut()).expect("32 字节在 HKDF-SHA256 的长度内");
        out
    }

    /// Fault-injection variant for the conformance gauges: neither salt nor purpose string goes in.
    #[cfg(feature = "ablate-k6-derive")]
    fn hkdf32(salt: Option<&[u8]>, ikm: &[u8], info: &str) -> Zeroizing<[u8; KEY_LEN]> {
        let _ = (salt, info);
        let mut out = Zeroizing::new([0u8; KEY_LEN]);
        let h = hkdf::Hkdf::<sha2::Sha256>::new(None, ikm);
        h.expand(b"", out.as_mut()).expect("32 字节在 HKDF-SHA256 的长度内");
        out
    }

    /// Recovery key R = HKDF-SHA256(seed, salt, purpose `recovery`). An empty seed or salt, or a registry
    /// that does not yield the purpose, is `None` (never derive with an empty purpose).
    pub fn recovery_key(seed: &[u8], salt: &[u8]) -> Option<Zeroizing<[u8; KEY_LEN]>> {
        if seed.is_empty() || salt.is_empty() {
            return None;
        }
        Some(hkdf32(Some(salt), seed, &purpose(USE_RECOVERY)?))
    }

    /// Private bytes of an age identity = HKDF-SHA256(the identity's seed or private key, no salt, purpose
    /// `galeed-age/1`). The same mnemonic yields the same identity on any machine: no randomness here.
    pub fn age_secret_bytes(ikm: &[u8]) -> Option<Zeroizing<[u8; KEY_LEN]>> {
        if ikm.is_empty() {
            return None;
        }
        Some(hkdf32(None, ikm, &purpose(USE_AGE)?))
    }

    /// The age identity of this identity, computed from its seed or private key (no randomness, so another
    /// machine computes the same one). The intermediate identity string is secret and is wiped with
    /// [`scrub`] once read.
    pub fn age_identity(ikm: &[u8]) -> Option<AgeIdentity> {
        let secret = age_secret_bytes(ikm)?;
        let mut text = super::bech32(AGE_SECRET_HRP, secret.as_ref()).to_uppercase();
        let id = AgeIdentity::parse(&text);
        scrub(&mut text);
        id
    }
}

#[cfg(test)]
mod vault_tests {
    use super::vault::*;
    use super::{bech32, sha256, AgeIdentity};

    const SALT: [u8; SALT_LEN] = [7u8; SALT_LEN];

    #[test]
    fn bech32_matches_age_own_spelling() {
        // The hand-written bech32 agrees with age's own serialization: bytes encoded to an identity string are
        // read by age, and age writes the same string back.
        for tag in [0u8, 1, 0xff] {
            let raw = [tag; 32];
            let mine = bech32("age-secret-key-", &raw).to_uppercase();
            let id = AgeIdentity::parse(&mine).expect("age 解得开自写的 bech32");
            assert_eq!(id.secret(), mine, "age 自己写出来的与我编的逐字同");
        }
        // Known vector (BIP-173): beyond the empty-HRP case only self-consistency is checked here, hence the
        // age round trip above.
        assert!(bech32("age-secret-key-", &[0u8; 32]).starts_with("age-secret-key-1"));
    }

    #[test]
    fn the_cost_gate_owns_the_ceiling_and_the_calibration_stays_inside_it() {
        let hi = max_log_n();
        assert!(hi >= min_log_n(), "上界不低于下界");
        assert!(super::keystore::scrypt_cost_bounded(1 << hi, u64::from(KDF_R), u64::from(KDF_P)));
        assert!(!super::keystore::scrypt_cost_bounded(1 << (hi + 1), u64::from(KDF_R), u64::from(KDF_P)), "上界外一档即出界");
        // Hitting the upper bound refuses: one step past it derives no key.
        assert!(passcode_key(b"12345678", &SALT, hi + 1).is_none());
        assert!(passcode_key(b"12345678", &SALT, min_log_n() - 1).is_none(), "弱于写档那一档也拒");
        let log_n = calibrate_log_n().expect("标定答得出一档");
        assert!((min_log_n()..=hi).contains(&log_n), "标定值落在两界之内");
    }

    #[test]
    fn a_passcode_key_is_deterministic_and_an_empty_input_is_refused() {
        let a = passcode_key(b"12345678", &SALT, min_log_n()).expect("派得出");
        let b = passcode_key(b"12345678", &SALT, min_log_n()).expect("派得出");
        assert_eq!(&a[..], &b[..], "同口令同盐同档,同一把钥");
        let c = passcode_key(b"12345679", &SALT, min_log_n()).expect("派得出");
        assert_ne!(&a[..], &c[..], "差一位的口令是另一把钥");
        assert!(passcode_key(b"", &SALT, min_log_n()).is_none() && passcode_key(b"12345678", &[], min_log_n()).is_none());
    }

    #[test]
    fn a_sealed_master_opens_once_and_refuses_every_change() {
        let key = [3u8; KEY_LEN];
        let master = [9u8; KEY_LEN];
        let sealed = seal(&key, &master).expect("随机源");
        assert_eq!(sealed.ciphertext.len(), master.len() + TAG_LEN, "密文比明文长一个标签");
        assert_eq!(open(&key, &sealed.nonce, &sealed.ciphertext).map(|p| p.to_vec()), Some(master.to_vec()));
        // A fresh nonce per seal: the same key sealing the same plaintext twice uses two nonces.
        assert_ne!(seal(&key, &master).unwrap().nonce, sealed.nonce);
        let mut flipped = sealed.ciphertext.clone();
        flipped[0] ^= 1;
        assert!(open(&key, &sealed.nonce, &flipped).is_none(), "密文翻一位即解不开");
        let mut other_nonce = sealed.nonce;
        other_nonce[0] ^= 1;
        assert!(open(&key, &other_nonce, &sealed.ciphertext).is_none(), "nonce 不对即解不开");
        assert!(open(&[4u8; KEY_LEN], &sealed.nonce, &sealed.ciphertext).is_none(), "钥不对即解不开");
        assert!(open(&key, &sealed.nonce[..NONCE_LEN - 1], &sealed.ciphertext).is_none(), "nonce 短一字节即解不开");
    }

    #[test]
    fn the_two_derivations_are_distinct_and_the_age_identity_is_reproducible() {
        let ikm = [5u8; 32];
        let r = recovery_key(&ikm, &SALT).expect("登记簿里有 recovery");
        let a = age_secret_bytes(&ikm).expect("登记簿里有 galeed-age/1");
        assert_ne!(&r[..], &a[..], "两路用途串不同,派出的钥不同");
        // The same ikm computed twice gives the same identity (no randomness).
        let one = age_identity(&ikm).expect("算得出");
        let two = age_identity(&ikm).expect("算得出");
        assert_eq!(one.recipient(), two.recipient());
        assert_ne!(age_identity(&[6u8; 32]).unwrap().recipient(), one.recipient(), "另一份 ikm 是另一枚身份");
        let cipher = super::age_encrypt(&one.recipient(), b"evidence").expect("加密");
        assert_eq!(super::age_decrypt(&one, &cipher), Some(b"evidence".to_vec()));
        assert!(recovery_key(&[], &SALT).is_none() && recovery_key(&ikm, &[]).is_none() && age_secret_bytes(&[]).is_none());
        // The recovery key follows the salt: another salt, another key.
        assert_ne!(&recovery_key(&ikm, &[8u8; SALT_LEN]).unwrap()[..], &r[..]);
    }

    #[test]
    fn scrub_leaves_zeroes_behind() {
        let mut secret = *b"passcode-and-seed-bytes---------";
        assert_ne!(sha256(&secret), sha256(&[0u8; 32]));
        scrub(&mut secret[..]);
        assert_eq!(secret, [0u8; 32], "用过即清");
    }
}
