use eravon_seal::envelope::{self as env, Doc, Expect, Ident, Owner};

fn d() -> [u8; 32] {
    std::array::from_fn(|i| i as u8)
}

fn nonce() -> [u8; env::NONCE] {
    std::array::from_fn(|i| 0x40 + i as u8)
}

const HOME: &str = "0123456789abcdef0123456789abcdef";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn vectors() -> Vec<(Ident, &'static [u8], &'static str, &'static str)> {
    vec![
        (Ident { owner: Owner::Machine, doc: Doc::GlobalSettings, logical: "settings.json".into() }, br#"{"language":"zh"}"#, DIGEST_GLOBAL, ENVELOPE_GLOBAL),
        (Ident { owner: Owner::Home(HOME.into()), doc: Doc::Candidates, logical: "candidates.json".into() }, br#"{"entries":[]}"#, DIGEST_CANDIDATES, ENVELOPE_CANDIDATES),
        (Ident { owner: Owner::Home(HOME.into()), doc: Doc::Product, logical: "products/p1/record.json".into() }, br#"{"id":"p1"}"#, DIGEST_PRODUCT, ENVELOPE_PRODUCT),
    ]
}

const DIGEST_GLOBAL: &str = "f79f610ff2e8a2aea07bc3a56d972d65";
const ENVELOPE_GLOBAL: &str = "657261766f6e2d6c6f63616c2f320a021c657261766f6e2f6c6f63616c2f676c6f62616c2d73657474696e67730001f79f610ff2e8a2aea07bc3a56d972d65404142434445464748494a4b4c4d4e4f5051525354555657d43e6811b3881078eaf48acdcae811fbfcdddeea792a3cf411139124676356f1719c205477e8ec3282935842d603b0361cd577cbd7a5d8adee";
const DIGEST_CANDIDATES: &str = "3c43354f3dba5970b9cde35dc0cb8a12";
const ENVELOPE_CANDIDATES: &str = "657261766f6e2d6c6f63616c2f320a0217657261766f6e2f6c6f63616c2f63616e6469646174657300013c43354f3dba5970b9cde35dc0cb8a12404142434445464748494a4b4c4d4e4f5051525354555657d41c6d1fbd854326bec6b48a9aaa52aaabdbcfa7773c35aa5b03ce713c3214a82f98600d31f7e210f0d195226eb3be90ef0e0ec605f6dbf2cdde6d9a67c025312909c22949ce15218c0c93f773f7b6515f4e0eaf97cb";
const DIGEST_PRODUCT: &str = "3c43354f3dba59706b3bde0f2186278e";
const ENVELOPE_PRODUCT: &str = "657261766f6e2d6c6f63616c2f320a0214657261766f6e2f6c6f63616c2f70726f6475637400013c43354f3dba59706b3bde0f2186278e404142434445464748494a4b4c4d4e4f5051525354555657d41c6d1fbd854326bec6b48a9aaa52aaabdbcfa7773c35aa5b03ce713c3214a82f98600d31f7e210e8c286236eafb985e8440dd940f7d1ffd98e6cda79c1233a21099116368967b8b421cd1174c98151f44a9628b9409eb602970c";

#[test]
fn the_vectors_are_the_envelopes_this_version_writes_and_reads() {
    for (id, plain, digest, envelope) in vectors() {
        let got = env::envelope(&d(), &id, &nonce(), plain).expect("sealed");
        assert_eq!(hex(&id.digest_keyed(&d())), digest, "{}", id.doc.tag());
        assert_eq!(hex(&got), envelope, "{}", id.doc.tag());
        let ex = Expect { owner: Some(id.owner.clone()), doc: id.doc, rel: None };
        assert_eq!(env::unseal(&d(), &ex, &got, "t", None).expect("opens"), plain);
    }
}

fn why(bytes: &[u8], ex: &Expect, key: &[u8; 32]) -> String {
    let t = env::unseal(key, ex, bytes, "t", None).expect_err("refused");
    t.tail.split(' ').next().unwrap_or_default().to_string()
}

fn global() -> (Ident, Vec<u8>, Expect) {
    let id = Ident { owner: Owner::Machine, doc: Doc::GlobalSettings, logical: "settings.json".into() };
    let bytes = env::envelope(&d(), &id, &nonce(), br#"{"language":"zh"}"#).expect("sealed");
    let ex = Expect { owner: Some(Owner::Machine), doc: Doc::GlobalSettings, rel: None };
    (id, bytes, ex)
}

#[test]
fn an_earlier_envelope_is_refused_whole_as_not_sealed() {
    let (_, bytes, ex) = global();
    let mut first = env::MAGIC.to_vec();
    first.extend_from_slice(&bytes[env::MAGIC_V2.len()..]);
    assert_eq!(why(&first, &ex, &d()), "not-sealed");
    let mut unkeyed = bytes.clone();
    unkeyed[env::MAGIC_V2.len()] = 1;
    assert_eq!(why(&unkeyed, &ex, &d()), "not-sealed");
    assert_eq!(why(br#"{"language":"zh"}"#, &ex, &d()), "not-sealed");
    let mut later = bytes;
    later[env::MAGIC_V2.len()] = env::ALG_KEYED + 1;
    assert_eq!(why(&later, &ex, &d()), "newer-version");
}

#[test]
fn each_way_a_file_does_not_read_has_its_reason() {
    let (_, bytes, ex) = global();
    assert_eq!(why(&bytes[..env::MAGIC_V2.len() + 5], &ex, &d()), "truncated", "the head cut short");
    assert_eq!(why(&bytes, &Expect { doc: Doc::Candidates, ..ex.clone() }, &d()), "other-kind");
    let home = Expect { owner: Some(Owner::Home(HOME.into())), ..ex.clone() };
    assert_eq!(why(&bytes, &home, &d()), "swapped");
    let other: [u8; 32] = std::array::from_fn(|i| 0x80 + i as u8);
    assert_eq!(why(&bytes, &ex, &other), "unopenable", "another key does not open it");
    let mut altered = bytes.clone();
    let last = altered.len() - 1;
    altered[last] ^= 1;
    assert_eq!(why(&altered, &ex, &d()), "unopenable");
}
