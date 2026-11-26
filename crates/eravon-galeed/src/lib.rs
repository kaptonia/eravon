#![forbid(unsafe_code)]

pub type Token = galeed::token::Token;
pub type Value = galeed::json::Value;

pub mod mark {
    pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::K1;

    #[inline]
    pub fn mark(feature: eravon_literals::trace::Feature) {
        eravon_literals::trace::mark(feature);
    }
}

pub mod schema;

pub mod canon {
    use super::mark;

    pub const INT_MAX: u64 = galeed::json::INT_MAX;
    pub const DEPTH_MAX: usize = galeed::json::DEPTH_MAX;

    pub fn accept(b: &[u8]) -> Result<super::Value, super::Token> {
        mark::mark(mark::FEATURE);
        galeed::json::accept(b)
    }



    pub fn read(b: &[u8]) -> Result<super::Value, super::Token> {
        mark::mark(mark::FEATURE);
        galeed::json::read(b)
    }

    pub fn canonical(v: &super::Value) -> Result<Vec<u8>, super::Token> {
        mark::mark(mark::FEATURE);
        galeed::json::canonical(v)
    }


    pub fn is_skeleton(s: &str) -> bool {
        mark::mark(mark::FEATURE);
        galeed::json::is_skeleton(s)
    }

    pub fn read_open(b: &[u8]) -> Option<super::Value> {
        mark::mark(mark::FEATURE);
        galeed::json::read_open(b)
    }
}

pub mod kdf {
    use super::mark;

    pub fn cost_bounded(n: u64, r: u64, p: u64) -> bool {
        mark::mark(mark::FEATURE);
        galeed::cryptox::keystore::scrypt_cost_bounded(n, r, p)
    }

    pub fn pbkdf2_rounds_bounded(c: u64) -> bool {
        mark::mark(mark::FEATURE);
        galeed::cryptox::keystore::pbkdf2_rounds_bounded(c)
    }

    pub fn pbkdf2_max_rounds() -> u64 {
        mark::mark(mark::FEATURE);
        galeed::cryptox::keystore::pbkdf2_max_rounds()
    }
}

pub mod hash {
    use super::mark;

    pub fn keccak256(b: &[u8]) -> [u8; 32] {
        mark::mark(mark::FEATURE);
        galeed::cryptox::keccak256(b)
    }


    pub fn sha256(b: &[u8]) -> [u8; 32] {
        mark::mark(mark::FEATURE);
        galeed::cryptox::sha256(b)
    }


    pub fn hex_encode(b: &[u8]) -> String {
        mark::mark(mark::FEATURE);
        galeed::hexform::encode(b)
    }
}

pub mod literals {
    use super::mark;

    pub fn pointer_prefixes() -> Option<Vec<String>> {
        mark::mark(mark::FEATURE);
        let reg = galeed::literals::parse(galeed::literals::REGISTRY_TEXT)?;
        Some(reg.of_role(galeed::literals::ROLE_POINTER_PREFIX))
    }
}

pub mod hexform {
    use super::mark;

    pub fn is_hex20(s: &str) -> bool {
        mark::mark(mark::FEATURE);
        galeed::hexform::is_hex20(s)
    }

    pub fn is_hex32(s: &str) -> bool {
        mark::mark(mark::FEATURE);
        galeed::hexform::is_hex32(s)
    }

    pub fn is_hex65(s: &str) -> bool {
        mark::mark(mark::FEATURE);
        galeed::hexform::is_hex65(s)
    }

    pub fn decode_digits(digits: &[u8]) -> Option<Vec<u8>> {
        mark::mark(mark::FEATURE);
        galeed::hexform::decode_digits(digits)
    }

    pub fn decode_form(s: &str) -> Vec<u8> {
        mark::mark(mark::FEATURE);
        galeed::hexform::decode_form(s)
    }
}

pub mod sign {
    use super::mark;

    pub type Privkey = galeed::cryptox::Privkey;
    pub type Sig = galeed::cryptox::Sig;

    pub fn sign_digest(key: &Privkey, digest: &[u8; 32]) -> Sig {
        mark::mark(mark::FEATURE);
        key.sign(digest)
    }


    pub fn recover(digest: &[u8; 32], r: &[u8; 32], s: &[u8; 32], i: u8) -> Option<[u8; 20]> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::recover_address(digest, r, s, i)
    }

    pub fn scalar_in_range(b: &[u8; 32]) -> bool {
        mark::mark(mark::FEATURE);
        galeed::cryptox::in_range(b)
    }

    pub fn is_high_s(s: &[u8; 32]) -> bool {
        mark::mark(mark::FEATURE);
        !galeed::cryptox::is_low_s(s)
    }
}

pub mod words {
    use super::mark;

    pub type MnemonicError = galeed::cryptox::MnemonicError;
    pub type ExtendedKey = galeed::cryptox::ExtendedKey;

    pub fn random_bytes(n: usize) -> Option<Vec<u8>> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::random_bytes(n).ok()
    }

    pub fn from_entropy(entropy: &[u8]) -> Option<String> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::mnemonic_from_entropy(entropy)
    }

    pub fn entropy(mnemonic: &str) -> Result<Vec<u8>, MnemonicError> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::mnemonic_entropy(mnemonic)
    }

    pub fn seed(mnemonic: &str, passphrase: &str) -> Option<[u8; 64]> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::mnemonic_seed(mnemonic, passphrase)
    }

    pub fn master(seed: &[u8]) -> Option<ExtendedKey> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::bip32_master(seed)
    }

    pub fn parse_path(path: &str) -> Option<Vec<u32>> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::parse_path(path)
    }

    pub fn derive_path(seed: &[u8], path: &str) -> Option<ExtendedKey> {
        mark::mark(mark::FEATURE);
        galeed::cryptox::derive_path(seed, path)
    }
}

pub mod lineage {
    use super::mark;

    pub fn of(input: &super::audit::Input) -> std::collections::BTreeSet<String> {
        mark::mark(mark::FEATURE);
        galeed::audit::lineage_of(input)
    }
}

pub mod book {
    use super::mark;

    pub type Logs = galeed_roster::Logs;
    pub type Declaration = galeed_roster::Declaration;
    pub type Accession = galeed_roster::Accession;
    pub type Book = galeed_roster::Book;

    pub fn of(logs: &Logs) -> Book {
        mark::mark(mark::FEATURE);
        Book::of(logs)
    }

    pub fn current_at(logs: &Logs, a: &str, p: (u64, u64)) -> String {
        mark::mark(mark::FEATURE);
        galeed_roster::current_at(logs, a, p)
    }

    pub fn current(logs: &Logs, a: &str) -> String {
        mark::mark(mark::FEATURE);
        galeed_roster::current(logs, a)
    }

    pub fn record_keys(logs: &Logs, a: &str) -> Vec<String> {
        mark::mark(mark::FEATURE);
        galeed_roster::record_keys(logs, a)
    }
}

pub mod entry {
    use super::mark;

    pub type Entry = galeed::envelope::Entry;
    pub const KEYS: [&str; 7] = galeed::envelope::KEYS;

    pub fn accept(b: &[u8]) -> Result<Entry, super::Token> {
        mark::mark(mark::FEATURE);
        galeed::envelope::accept(b)
    }

}

pub mod carried {
    use super::mark;

    pub const ANCHORED_CARRIED_SIGNATURE: &str = galeed::kit::ANCHORED_CARRIED_SIGNATURE;

    pub fn anchor_bytes(data: &[u8], hash: &[u8; 32]) -> Option<Vec<u8>> {
        mark::mark(mark::FEATURE);
        galeed::kit::carried_anchor(data, hash)
    }

}

pub mod rlp {
    pub type Rlp<'a> = galeed::rlp::Rlp<'a>;

    pub fn decode(b: &[u8]) -> Option<Rlp<'_>> {
        galeed::rlp::decode(b)
    }

    pub fn encode(item: &Rlp<'_>) -> Vec<u8> {
        galeed::rlp::encode(item)
    }

    pub fn uint_bytes(n: u64) -> Vec<u8> {
        galeed::rlp::uint_bytes(n)
    }
}

pub mod kit {
    use super::mark;

    pub type Judgment = galeed::kit::Judgment;
    pub type Shown = galeed::kit::Shown;

    pub fn judge(kit: &super::Value) -> Judgment {
        mark::mark(mark::FEATURE);
        galeed::kit::judge(kit)
    }

    pub fn verify_proof(root: [u8; 32], key: &[u8], proof: &[Vec<u8>]) -> Result<Option<Vec<u8>>, ()> {
        mark::mark(mark::FEATURE);
        galeed::kit::verify_proof(root, key, proof)
    }
}

pub mod audit {
    use super::mark;

    pub type Record = galeed::audit::Record;
    pub type Verdict = galeed::audit::Verdict;
    pub type Input = galeed::audit::Input;
    pub const BASIS_CHAINS: &str = galeed::audit::BASIS_CHAINS;

    pub fn audit(input: &Input) -> super::Value {
        mark::mark(mark::FEATURE);
        galeed::audit::audit(input)
    }

    pub fn read_input(v: &super::Value) -> Option<Input> {
        mark::mark(mark::FEATURE);
        galeed::audit::read_input(v)
    }
}

pub mod body {
    pub const AMANAH: &str = galeed::body::AMANAH;
    pub const JUDGMENT: &str = galeed::body::JUDGMENT;
    pub const RESIGNATION: &str = galeed::body::RESIGNATION;
    pub const SUCCESSION: &str = galeed::body::SUCCESSION;
}
