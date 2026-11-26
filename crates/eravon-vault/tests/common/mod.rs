#![allow(dead_code)]

use eravon_store::record::Kind;
use eravon_store::seam::Box_;
use eravon_store::vault::Seat;
use eravon_vault::session::Session;

pub const PIN: &str = "48261537";
pub const PIN2: &str = "71639482";
pub const PHRASE: &str = "test test test test test test test test test test test junk";
pub const SELLER: &str = "0x98e503f35d0a019cb0a251ad243a4ccfcf371f46";
pub const NOW: u64 = 900;
pub const ID: &str = "k0";
pub const BODY: &[u8] = b"draft body bytes";

pub fn home(tag: &str) -> (String, Session) {
    let d = std::env::temp_dir().join(format!("eravon-vault-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("the machine directory");
    let data = d.display().to_string();
    eravon_vault::kdf::set_light(None);
    eravon_vault::records::bind(&data).expect("the run binds its machine directory");
    eravon_vault::onboard::land(&data, eravon_vault::onboard::Mode::First, Seat::Seller, "主钥", false, eravon_vault::onboard::Source::Words(PHRASE), PIN, None, None, NOW).expect("a first run");
    let mut s = Session::locked(&data);
    s.unlock(PIN).expect("unlocks");
    let st = eravon_vault::store::open_write(&data, "j", NOW).expect("writes");
    st.put(SELLER, Kind::Product, ID, &[("name".to_string(), "海报".to_string())], &[("body".to_string(), BODY.to_vec())], NOW).expect("the record");
    drop(st);
    (data, s)
}

pub fn new_identity(s: &Session) -> String {
    let (addr, _) = eravon_vault::accounts::new_words_as(s, Seat::Seller, "新主钥", true, NOW).expect("a new key");
    eravon_vault::records::seat_of(&addr).expect("filed").0.id
}

pub fn restart(data: &str, pin: &str) -> Session {
    eravon_vault::keybox::lock();
    eravon_store::writer::let_go(data);
    eravon_vault::tokens::forget(data);
    eravon_vault::local::forget_labels();
    let mut s = Session::locked(data);
    s.unlock(pin).expect("unlocks after the restart");
    s
}

pub fn reads_back(data: &str) {
    let st = eravon_vault::store::open_read(data).expect("reads");
    let rec = st.get(SELLER, Box_::Products, ID).expect("the record reads back");
    assert_eq!(rec.fields, vec![("name".to_string(), "海报".to_string())]);
    assert_eq!(st.part(SELLER, Box_::Products, ID, "body").expect("its part reads back"), BODY);
    let all = st.list(SELLER, Box_::Products).expect("the box lists");
    assert_eq!(all.len(), 1, "listed exactly once: {:?}", all.iter().map(|r| r.id.clone()).collect::<Vec<_>>());
}

pub fn record_dir(data: &str) -> (std::path::PathBuf, Vec<String>) {
    let _ = data;
    let dir = eravon_vault::records::record_dir(SELLER, Box_::Products, ID).expect("its directory");
    let boxed = dir.parent().expect("its box").to_path_buf();
    let mut names: Vec<String> = std::fs::read_dir(&boxed).expect("the box").flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    names.sort();
    (dir, names)
}
