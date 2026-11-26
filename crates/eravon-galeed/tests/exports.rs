use eravon_galeed as k1;

const KECCAK_EMPTY: &str = "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";
const SHA256_EMPTY: &str = "0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const ADDRESS_OF_KEY_ONE: &str = "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf";
const SIG_R: &str = "0xe7c93726a865578504442b1a6827f676e0ed74bdff2be3960d1e253bbcfc4462";
const SIG_S: &str = "0x6aa772b878bc912bdbb33a0014ec507c4b3896ea85aa914b74dee9b7ac3e56da";

fn key_one() -> k1::sign::Privkey {
    let mut b = [0u8; 32];
    b[31] = 1;
    k1::sign::Privkey::from_bytes(&b).expect("0x…01 在 [1, n − 1] 里")
}

#[test]
fn canon_accept_takes_the_canonical_and_names_the_rest() {
    let v = k1::canon::accept(br#"{"a":1,"b":"x"}"#).expect("正典字节该过");
    assert_eq!(k1::canon::canonical(&v).unwrap(), br#"{"a":1,"b":"x"}"#.to_vec());
    let t = k1::canon::accept(br#"{"b":"x","a":1}"#).expect_err("乱序该被拒");
    assert_eq!(t.name(), "E_NOT_CANONICAL");
}

#[test]
fn canon_read_stops_before_the_sixth_test() {
    let v = k1::canon::read(br#"{"b":"x","a":1}"#).expect("读得出");
    assert_eq!(k1::canon::canonical(&v).unwrap(), br#"{"a":1,"b":"x"}"#.to_vec());
}

#[test]
fn canon_is_skeleton_and_the_two_bounds_come_from_the_law() {
    assert_eq!(k1::canon::INT_MAX, (1u64 << 53) - 1);
    assert_eq!(k1::canon::DEPTH_MAX, 128);
    assert!(k1::canon::is_skeleton("plain text"));
    assert!(!k1::canon::is_skeleton("two\nlines"));
}

#[test]
fn keccak256_answers_what_cast_answers() {
    assert_eq!(k1::hash::hex_encode(&k1::hash::keccak256(b"")), KECCAK_EMPTY);
}

#[test]
fn sha256_answers_what_shasum_answers() {
    assert_eq!(k1::hash::hex_encode(&k1::hash::sha256(b"")), SHA256_EMPTY);
}

#[test]
fn sign_and_recover_answer_what_cast_wallet_answers() {
    let key = key_one();
    assert_eq!(k1::hash::hex_encode(&key.address()), ADDRESS_OF_KEY_ONE);
    let digest = [0x11u8; 32];
    let sig = k1::sign::sign_digest(&key, &digest);
    assert_eq!(k1::hash::hex_encode(&sig.r), SIG_R);
    assert_eq!(k1::hash::hex_encode(&sig.s), SIG_S);
    assert_eq!(sig.v, 28);
    let got = k1::sign::recover(&digest, &sig.r, &sig.s, sig.v - 27).expect("恢复得出");
    assert_eq!(k1::hash::hex_encode(&got), ADDRESS_OF_KEY_ONE);
}

#[test]
fn the_two_signature_form_gates_read_the_scalars() {
    let zero = [0u8; 32];
    assert!(!k1::sign::scalar_in_range(&zero));
    let sig = k1::sign::sign_digest(&key_one(), &[0x11u8; 32]);
    assert!(k1::sign::scalar_in_range(&sig.s));
    assert!(!k1::sign::is_high_s(&sig.s));
    let mut high = [0u8; 32];
    high[0] = 0xff;
    high[31] = 1;
    assert!(k1::sign::is_high_s(&high));
}

#[test]
fn entry_accept_names_the_seven_member_closure() {
    assert_eq!(k1::entry::KEYS.len(), 7);
    assert_eq!(k1::entry::KEYS[0], "spec");
    let t = k1::entry::accept(b"{}").err().expect("空对象缺七成员");
    assert_eq!(t.name(), "E_ENVELOPE_MISSING");
    let t = k1::entry::accept(b"[]").err().expect("根是列即被拒");
    assert_eq!(t.name(), "E_ENVELOPE");
}

#[test]
fn carried_anchor_takes_three_things_or_nothing() {
    let body = b"hello carried";
    let mut frame = vec![0u8; 64];
    frame[31] = 32;
    frame[56..64].copy_from_slice(&(body.len() as u64).to_be_bytes());
    frame.extend_from_slice(body);
    while frame.len() % 32 != 0 {
        frame.push(0);
    }
    let h = k1::hash::sha256(body);
    assert_eq!(k1::carried::anchor_bytes(&frame, &h).as_deref(), Some(&body[..]));

    let mut wrong = h;
    wrong[0] ^= 0xff;
    assert!(k1::carried::anchor_bytes(&frame, &wrong).is_none(), "摘要不符即不是携带形");

    let mut bad_frame = frame.clone();
    bad_frame[31] = 33;
    assert!(k1::carried::anchor_bytes(&bad_frame, &h).is_none(), "框不对即不是携带形");
}

#[test]
fn the_anchored_signature_is_the_one_form_the_law_names() {
    assert_eq!(k1::carried::ANCHORED_CARRIED_SIGNATURE, "Anchored(address,bytes32,bytes)");
}
