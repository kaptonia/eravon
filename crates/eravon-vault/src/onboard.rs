use eravon_store::settings;
use crate::store;
use eravon_store::vault::Seat;

use crate::vfault::{Fault, R};
use crate::session::Session;
use crate::{accounts, keyfile, payout, restore, vault, words};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    First,
    Seat,
}

impl Mode {
    pub const fn name(self) -> &'static str {
        match self {
            Mode::First => "first",
            Mode::Seat => "seat",
        }
    }

    pub fn of(s: &str) -> Option<Mode> {
        match s {
            "first" => Some(Mode::First),
            "seat" => Some(Mode::Seat),
            _ => None,
        }
    }
}

pub enum Source<'a> {
    Words(&'a str),
    KeyFile {
        bytes: &'a [u8],
        file_passcode: &'a str,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Landed {
    pub address: String,
    pub seat: Seat,
    pub endpoints_from: &'static str,
    pub endpoints: usize,
    pub payout: String,
    pub identities: usize,
}

pub const FROM_SAMPLE: &str = "k7-sample";
pub const FROM_UNSET: &str = "unset";
pub const FROM_KEPT: &str = "kept";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Born {
    pub endpoints: usize,
    pub from: &'static str,
}

fn write_sample(data: &str, address: &str, chain: u64, then: impl FnOnce(&mut settings::Identity)) -> R<(usize, &'static str)> {
    let sample: Vec<String> = eravon_literals::constants::endpoint_table_factory_sample()
        .get()
        .map(|v| v.iter().map(|x| (*x).to_string()).collect())
        .unwrap_or_default();
    let from = if eravon_literals::constants::endpoint_table_factory_sample().get().is_some() { FROM_SAMPLE } else { FROM_UNSET };
    let mut ident = match crate::store::read_identity(data, address) {
        Ok(i) => i,
        Err(eravon_store::fault::Fault::Sealed(_)) => settings::Identity::default(),
        Err(e) => return Err(Fault::from(e)),
    };
    then(&mut ident);
    crate::store::write_identity(data, address, &ident).map_err(Fault::from)?;
    if !crate::store::write_factory_endpoints(data, address, chain).map_err(Fault::from)? {
        return Ok((crate::store::endpoints(data, address).map_err(Fault::from)?.len(), FROM_KEPT));
    }
    Ok((sample.len(), from))
}

pub fn born(s: &store::Store, data: &str, address: &str, chain: u64, now: u64) -> R<Born> {
    crate::records::bind(data)?;
    let on_disk = crate::records::single_path(address, crate::local::Doc::IdentitySettings)?.exists();
    match s.init_identity(address, now) {
        Ok(()) | Err(eravon_store::fault::Fault::Exists(_)) => {}
        Err(e) => return Err(Fault::from(e)),
    }
    if on_disk {
        let n = crate::store::endpoints(data, address).map_err(Fault::from)?.len();
        return Ok(Born { endpoints: n, from: FROM_KEPT });
    }
    let (endpoints, from) = write_sample(data, address, chain, |_| {})?;
    Ok(Born { endpoints, from })
}


pub struct Births {
    pub own: R<Born>,
    pub others: Vec<R<(Seat, String, Born)>>,
}

pub fn born_added(s: &store::Store, data: &str, address: &str, chain: u64, now: u64) -> Births {
    let own = born(s, data, address, chain, now);
    let mut others = Vec::new();
    match crate::records::seat_of(address) {
        Ok((row, role)) => {
            for r in row.seats().into_iter().filter(|r| *r != role) {
                let Some(other) = row.address(r).map(|a| a.hex()) else { continue };
                let seat = vault::seat_of(r);
                match seat_ready(data, seat) {
                    Ok(false) => {}
                    Ok(true) => others.push(born(s, data, &other, chain, now).map(|b| (seat, other, b))),
                    Err(e) => others.push(Err(Fault::from(e))),
                }
            }
        }
        Err(e) => others.push(Err(Fault::from(e))),
    }
    Births { own, others }
}

pub fn seat(data: &str, seat: Seat, address: &str, chain: u64, now: u64) -> R<Born> {
    let s = store::open_write(data, WRITER, now).map_err(Fault::from)?;
    let existing = crate::store::read_global(data);
    let mut g = match &existing {
        Ok(g) => g.clone(),
        Err(eravon_store::fault::Fault::Missing(_)) | Err(eravon_store::fault::Fault::Sealed(_)) => fresh_global(),
        Err(e) => return Err(Fault::from(e.clone())),
    };
    match seat {
        Seat::Seller => g.seller_ready = true,
        Seat::Buyer => g.buyer_ready = true,
    }
    g.last_writer = WRITER.to_string();
    match existing {
        Err(eravon_store::fault::Fault::Missing(_)) => s.init(&g).map_err(Fault::from)?,
        _ => crate::store::write_global(data, &g).map_err(Fault::from)?,
    }
    let b = born(&s, data, address, chain, now)?;
    accounts::switch(data, seat, address)?;
    drop(s);
    Ok(b)
}

pub fn seat_ready(data: &str, seat: Seat) -> Result<bool, eravon_store::fault::Fault> {
    match crate::store::read_global(data) {
        Ok(g) => Ok(match seat {
            Seat::Seller => g.seller_ready,
            Seat::Buyer => g.buyer_ready,
        }),
        Err(eravon_store::fault::Fault::Missing(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn land(
    data: &str,
    mode: Mode,
    seat: Seat,
    label: &str,
    named: bool,
    source: Source<'_>,
    passcode: &str,
    open: Option<&Session>,
    payout_addr: Option<&str>,
    now: u64,
) -> R<Landed> {
    let payout_ok = match payout_addr {
        Some(a) if !a.trim().is_empty() => Some(payout::accept(a)?),
        _ => None,
    };

    let s = store::open_write(data, WRITER, now).map_err(Fault::from)?;

    let mut holder: Option<Session> = None;
    let address = match mode {
        Mode::First => {
            let a = first_run(data, seat, label, source, passcode, now)?;
            let mut h = Session::locked(data);
            h.unlock(passcode)?;
            holder = Some(h);
            a
        }
        Mode::Seat => {
            let open = open.filter(|s| s.is_open()).ok_or(Fault::Locked)?;
            seat_wizard(open, seat, label, named, source, now)?
        }
    };

    let existing = crate::store::read_global(data);
    let mut g = match &existing {
        Ok(g) => g.clone(),
        Err(eravon_store::fault::Fault::Missing(_)) | Err(eravon_store::fault::Fault::Sealed(_)) => fresh_global(),
        Err(e) => return Err(Fault::from(e.clone())),
    };
    match seat {
        Seat::Seller => g.seller_ready = true,
        Seat::Buyer => g.buyer_ready = true,
    }
    g.last_writer = WRITER.to_string();
    match existing {
        Err(eravon_store::fault::Fault::Missing(_)) => s.init(&g).map_err(Fault::from)?,
        _ => crate::store::write_global(data, &g).map_err(Fault::from)?,
    }
    s.init_identity(&address, now).map_err(Fault::from)?;
    accounts::switch(data, seat, &address)?;

    let (endpoints, from) = write_sample(data, &address, eravon_literals::constants::default_network().standard.chain_id, |ident| {
        if let Some(p) = &payout_ok {
            ident.payout = p.clone();
        }
    })?;
    let identities = vault::count(data)?;
    drop(s);
    drop(holder);

    Ok(Landed {
        address,
        seat,
        endpoints_from: from,
        endpoints,
        payout: payout_ok.unwrap_or_default(),
        identities,
    })
}

pub const WRITER: &str = "first-run";

fn fresh_global() -> settings::Global {
    settings::Global::default()
}

fn first_run(
    data: &str,
    seat: Seat,
    label: &str,
    source: Source<'_>,
    passcode: &str,
    now: u64,
) -> R<String> {
    if vault::exists(data) {
        return Err(Fault::Exists(format!(
            "{data} 里已经有一本密钥库了;要用这一份换掉它,先把现有的身份导出备份"
        )));
    }
    build(data, seat, label, source, passcode, now)
}

fn build(
    data: &str,
    seat: Seat,
    label: &str,
    source: Source<'_>,
    passcode: &str,
    now: u64,
) -> R<String> {
    match source {
        Source::Words(p) => {
            let mut born = vault::create(passcode, seat, label, Some(p), now)?;
            let addr = born.address.clone();
            born.write(data)?;
            Ok(addr)
        }
        Source::KeyFile { bytes, file_passcode } => {
            let (addr, _) = restore::rebuild(
                data,
                bytes,
                file_passcode,
                passcode,
                passcode,
                seat,
                label,
                now,
            )?;
            Ok(addr)
        }
    }
}

fn seat_wizard(
    s: &Session,
    seat: Seat,
    label: &str,
    named: bool,
    source: Source<'_>,
    now: u64,
) -> R<String> {
    match source {
        Source::Words(p) => accounts::import_words_as(s, seat, label, named, p, now),
        Source::KeyFile { bytes, file_passcode } => {
            restore::add_as(s, bytes, file_passcode, seat, label, named, now).map(|(a, _)| a)
        }
    }
}

pub fn self_check(data: &str, seat: Seat) -> R<String> {
    let v = vault::read(data)?;
    let who = accounts::who(data, seat)?;
    let one = accounts::find(&v, &who)?;
    for box_ in eravon_store::seam::Box_::ALL {
        let p = crate::records::box_dir(&who, box_)?;
        if !p.is_dir() {
            return Err(Fault::Store(format!("身份的家缺一处收纳:{}", p.display())));
        }
    }
    let ident = crate::store::read_identity(data, &who).map_err(Fault::from)?;
    let nodes = crate::store::endpoints(data, &who).map_err(Fault::from)?;
    Ok(format!(
        "自检过:密钥库 {} 枚身份 · 当前钥 {} · 四收纳处齐 · 端点 {} 个 · 收款地址 {}",
        v.identities.len(),
        one.address,
        nodes.len(),
        if ident.payout.is_empty() { "稍后设置" } else { &ident.payout }
    ))
}


pub fn fresh_phrase() -> R<String> {
    words::fresh()
}

pub fn peek(bytes: &[u8], file_passcode: &str) -> R<String> {
    keyfile::accept(bytes, file_passcode).map(|o| o.address)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PIN: &str = "48261537";
    const PIN2: &str = "71639482";
    const PHRASE: &str = "test test test test test test test test test test test junk";
    const SELLER: &str = "0x98e503f35d0a019cb0a251ad243a4ccfcf371f46";

    fn home(_tag: &str) -> String {
        crate::testing::fresh()
    }

    #[test]
    fn an_added_key_is_born_with_the_factory_table() {
        let d = home("born");
        land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, None, 900).unwrap();
        let mut open = Session::locked(&d);
        open.unlock(PIN).unwrap();
        let sample = eravon_literals::constants::endpoint_table_factory_sample().get().map_or(0, |v| v.len());
        let added = crate::add::made(&open, Seat::Buyer, "后加的钥", false).unwrap();
        let st = store::open_write(&d, WRITER, 900).unwrap();
        assert_eq!(born(&st, &d, &added, eravon_literals::constants::default_network().standard.chain_id, 900).unwrap(), Born { endpoints: sample, from: FROM_SAMPLE });
        assert_eq!(crate::store::endpoints(&d, &added).unwrap().len(), sample);
        crate::store::write_endpoints(&d, &added, &[]).unwrap();
        assert_eq!(born(&st, &d, &added, eravon_literals::constants::default_network().standard.chain_id, 900).unwrap(), Born { endpoints: 0, from: FROM_KEPT });
        assert!(crate::store::endpoints(&d, &added).unwrap().is_empty());
    }

    #[test]
    fn a_seat_with_nodes_of_its_own_keeps_them_through_its_wizard() {
        let d = home("born-own");
        land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, None, 900).unwrap();
        let mut open = Session::locked(&d);
        open.unlock(PIN).unwrap();
        let (row, _) = crate::records::seat_of(SELLER).unwrap();
        let buyer = row.address(crate::vault::role_of(Seat::Buyer)).unwrap().hex();
        let own = vec!["http://127.0.0.1:9".to_string()];
        crate::store::write_endpoints(&d, &buyer, &own).unwrap();
        let written = crate::store::home_settings(&d, &buyer).unwrap().expect("the buyer seat's home").endpoints;
        assert!(!crate::records::single_path(&buyer, crate::local::Doc::IdentitySettings).unwrap().exists(), "no identity settings yet");
        let b = seat(&d, Seat::Buyer, &buyer, eravon_literals::constants::default_network().standard.chain_id, 901).unwrap();
        assert_eq!(b, Born { endpoints: 1, from: FROM_KEPT });
        assert_eq!(crate::store::home_settings(&d, &buyer).unwrap().expect("home").endpoints, written, "the table as written, byte for byte");
        assert_eq!(crate::store::endpoints(&d, &buyer).unwrap(), own);
        assert!(!crate::store::write_factory_endpoints(&d, &buyer, eravon_literals::constants::default_network().standard.chain_id).unwrap());
        assert_eq!(crate::store::home_settings(&d, &buyer).unwrap().expect("home").endpoints, written);
    }

    #[test]
    fn an_added_key_is_born_on_each_laid_out_seat_of_its_identity() {
        let d = home("born-seats");
        land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, None, 900).unwrap();
        let mut open = Session::locked(&d);
        open.unlock(PIN).unwrap();
        let sample = eravon_literals::constants::endpoint_table_factory_sample().get().map_or(0, |v| v.len());
        let (first, _) = crate::accounts::new_words(&open, Seat::Seller, "后加一", 900).unwrap();
        let st = store::open_write(&d, WRITER, 900).unwrap();
        let b = born_added(&st, &d, &first, eravon_literals::constants::default_network().standard.chain_id, 900);
        assert_eq!(b.own.unwrap(), Born { endpoints: sample, from: FROM_SAMPLE });
        assert!(b.others.is_empty(), "the buyer seat is not laid out: nothing born there");
        drop(st);
        let (row, _) = crate::records::seat_of(SELLER).unwrap();
        let buyer_primary = row.address(crate::vault::role_of(Seat::Buyer)).unwrap().hex();
        seat(&d, Seat::Buyer, &buyer_primary, eravon_literals::constants::default_network().standard.chain_id, 900).unwrap();
        let (second, _) = crate::accounts::new_words(&open, Seat::Seller, "后加二", 900).unwrap();
        let st = store::open_write(&d, WRITER, 900).unwrap();
        let b = born_added(&st, &d, &second, eravon_literals::constants::default_network().standard.chain_id, 900);
        assert!(b.own.is_ok());
        let others: Vec<(Seat, String)> = b.others.into_iter().map(|o| o.map(|(s, a, _)| (s, a)).unwrap()).collect();
        assert_eq!(others.len(), 1, "the identity's buyer key, on the laid-out buyer seat");
        assert_eq!(others[0].0, Seat::Buyer);
        assert_eq!(crate::store::endpoints(&d, &others[0].1).unwrap().len(), sample, "born with the factory table");
    }

    #[test]
    fn a_key_is_born_on_the_row_chosen_with_that_rows_factory_nodes() {
        for n in eravon_literals::constants::NETWORKS {
            let chain = n.standard.chain_id;
            let d = home(&format!("net-{}", n.name));
            let got = land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, None, 900).unwrap();
            let head = eravon_literals::constants::default_network();
            assert_eq!(got.endpoints, head.factory.len(), "a first run: the head row's factory sample");
            let mut open = Session::locked(&d);
            open.unlock(PIN).unwrap();
            assert_eq!(crate::store::network(&d, SELLER).unwrap(), head.standard.chain_id);
            let added = crate::add::made(&open, Seat::Buyer, "后加", false).unwrap();
            let st = store::open_write(&d, WRITER, 900).unwrap();
            born(&st, &d, &added, chain, 900).unwrap();
            assert_eq!(crate::store::network(&d, &added).unwrap(), chain, "{}: the row chosen", n.name);
            let nodes = crate::store::endpoints(&d, &added).unwrap();
            assert_eq!(nodes, eravon_literals::constants::factory_of(&n).iter().map(|u| u.to_string()).collect::<Vec<_>>(), "{}: that row's factory nodes", n.name);
            let _ = std::fs::remove_dir_all(&d);
        }
    }

    #[test]
    fn a_first_run_lands_every_piece_in_one_go() {
        let d = home("first");
        let got = land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, Some("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"), 900)
        .unwrap();
        assert_eq!(got.address, SELLER);
        assert_eq!(got.identities, 2, "an identity made from words holds both seats");
        assert_eq!(got.endpoints_from, FROM_SAMPLE);
        assert_eq!(got.endpoints, eravon_literals::constants::endpoint_table_factory_sample().get().map_or(0, |v| v.len()));
        assert_eq!(got.payout, "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed");

        let mut open = Session::locked(&d);
        open.unlock(PIN).unwrap();
        let g = crate::store::read_global(&d).unwrap();
        assert_eq!(accounts::active(&d, Seat::Seller).unwrap().as_deref(), Some(SELLER));
        assert!(g.seller_ready, "所选席就绪");
        assert!(!g.buyer_ready, "另一席未就绪");
        assert!(vault::unlock(&d, PIN).is_ok());
        assert!(self_check(&d, Seat::Seller).unwrap().contains("四收纳处齐"));
    }

    #[test]
    fn a_home_that_already_has_a_vault_refuses_a_first_run() {
        let d = home("zero");
        land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, None, 900)
            .unwrap();
        let e = land(&d, Mode::First, Seat::Seller, "又一次", false, Source::Words(PHRASE), PIN2, None, None, 901)
        .unwrap_err();
        assert_eq!(e.name(), "E_VAULT_EXISTS", "零金库时才建库");
        assert!(vault::unlock(&d, PIN).is_ok());
    }

    #[test]
    fn the_seat_wizard_adds_the_other_seat_without_touching_the_passcode() {
        let d = home("seat");
        land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, None, 900)
            .unwrap();
        let other = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
        let _ = other;
        let second = words::fresh().unwrap();
        let mut open = Session::locked(&d);
        open.unlock(PIN).unwrap();
        let got = land(
            &d,
            Mode::Seat,
            Seat::Buyer,
            "买家钥",
            false,
            Source::Words(&second),
            "",
            Some(&open),
            None,
            901,
        )
        .unwrap();
        let g = crate::store::read_global(&d).unwrap();
        assert!(g.seller_ready && g.buyer_ready, "两席都就绪了");
        assert_eq!(accounts::active(&d, Seat::Buyer).unwrap().as_deref(), Some(got.address.as_str()), "置为当前钥");
        assert_eq!(got.identities, 4, "two identities made from words, two seats each");
        assert!(vault::unlock(&d, PIN).is_ok());
        let v = vault::read(&d).unwrap();
        let one = accounts::find(&v, &got.address).unwrap();
        assert!(!one.primary && !crate::keybox::has_recovery(&one.id).unwrap());
    }

    #[test]
    fn a_label_the_person_gave_a_new_primary_is_checked_against_the_seat() {
        let d = home("primary-named");
        land(&d, Mode::First, Seat::Seller, "工作室", true, Source::Words(PHRASE), PIN, None, None, 900).unwrap();
        let read_now = || {
            let mut open = Session::locked(&d);
            open.unlock(PIN).unwrap();
            vault::read(&d).unwrap()
        };
        let before = read_now();
        let mut s = Session::locked(&d);
        s.unlock(PIN).unwrap();
        let e = accounts::new_primary(&mut s, Seat::Seller, "工作室", true, None, PIN, 902).unwrap_err();
        assert_eq!(e.name(), "IDENTITY_NAME_TAKEN");
        assert_eq!(read_now(), before, "refused: nothing written");
        accounts::new_primary(&mut s, Seat::Seller, "工作室", false, None, PIN, 903).expect("the product's label repeats by design");
    }

    #[test]
    fn a_new_primary_keeps_the_other_keys_and_the_passcode() {
        let d = home("new-primary");
        land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, None, 900).unwrap();
        let second = words::fresh().unwrap();
        let mut open = Session::locked(&d);
        open.unlock(PIN).unwrap();
        land(&d, Mode::Seat, Seat::Buyer, "买家钥", false, Source::Words(&second), "", Some(&open), None, 901).unwrap();
        let (address, words, trouble) = accounts::new_primary(&mut open, Seat::Seller, "新主钥", true, None, PIN, 902).unwrap();
        assert!(trouble.is_empty(), "{trouble:?}");
        let words = words.expect("words made here are handed back");
        drop(open);
        let mut open = Session::locked(&d);
        open.unlock(PIN).expect("the passcode is the one it was");
        let v = vault::read(&d).unwrap();
        assert_eq!(v.identities.len(), 6, "其他钥一律保留(每个助记词身份占两席)");
        assert!(accounts::find(&v, &address).unwrap().primary);
        let old = accounts::find(&v, SELLER).unwrap();
        assert!(!old.primary, "the former primary is secondary now");
        assert_eq!(old.label, "主钥", "under its own label");
        let g = crate::store::read_global(&d).unwrap();
        assert!(g.seller_ready && g.buyer_ready, "两席就绪不变");
        assert_eq!(accounts::active(&d, Seat::Seller).unwrap().as_deref(), Some(address.as_str()), "the new identity is current, as the family's add leaves it");
        drop(open);
        vault::recover(&d, &words, PIN2, PIN2).expect("the new primary's words recover the vault");
    }

    #[test]
    fn a_bad_payout_is_refused_before_anything_lands() {
        let d = home("payout");
        let e = land(&d, Mode::First, Seat::Seller, "主钥", false, Source::Words(PHRASE), PIN, None, Some("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAEd"), 900)
        .unwrap_err();
        assert_eq!(e.name(), "E_VAULT_ADDRESS");
        assert!(!vault::exists(&d), "错了就一个字节也不落:盘上不该有半个家");
    }
}
