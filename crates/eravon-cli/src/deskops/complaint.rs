use eravon_dispute::complaint::{self as cp, Kind, Role};
use eravon_literals::json::Out;
use eravon_vault::lang::{filln, t, Key};

use crate::deskops::said::Said;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Form {
    pub text: String,
    pub dir: String,
    pub files: Vec<(Kind, String)>,
    pub guild: String,
}

pub fn files_arg(files: &[(Kind, String)]) -> String {
    files.iter().map(|(k, p)| format!("{}:{p}", k.word())).collect::<Vec<_>>().join("\n")
}

pub fn files_of(arg: &str) -> Result<Vec<(Kind, String)>, String> {
    arg.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (k, p) = l.split_once(':').ok_or_else(|| format!("材料要写成 kind:path,给的是 {l}"))?;
            let kind = Kind::of(k).ok_or_else(|| format!("材料种类只收 dispatch、pass、audit-input、other,给的是 {k}"))?;
            Ok((kind, p.to_string()))
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Guild {
    pub address: String,
    pub pointer: String,
    pub card: Result<eravon_cands::guild::Card, Said>,
}

#[derive(Clone, Debug)]
pub struct Complaint {
    pub instance: String,
    pub case_id: String,
    pub judge: String,
    pub guild: Result<Vec<Guild>, Said>,
    pub written: Option<Result<String, Said>>,
}

pub fn seated(row: &eravon_orders::table::Row) -> Option<String> {
    row.chain.as_ref().and_then(|v| seated_of(v.seats, &v.judge))
}

fn seated_of(seats: u64, judge: &str) -> Option<String> {
    let j = judge.trim().to_ascii_lowercase();
    (seats > 0 && eravon_galeed::hexform::is_hex20(&j) && j.trim_start_matches("0x").chars().any(|c| c != '0')).then_some(j)
}

pub fn guild_of_listing(listing: &eravon_link::listing::Listing, candidate: u64) -> Option<String> {
    (candidate as usize).checked_sub(listing.judges.len()).and_then(|g| listing.guilds.get(g)).map(|g| g.address.to_ascii_lowercase())
}

pub fn company_party() -> Said {
    Said::bad(cp::E_SIGNER, t(Key::CpCompanyParty))
}

pub fn listing_unread(detail: &str) -> Said {
    Said::bad("E_COMPLAINT_LISTING", format!("{} · {detail}", t(Key::CpListingUnread)))
}

fn not_seated() -> Said {
    Said::bad("E_COMPLAINT_SEAT", t(Key::CpNotSeated))
}

pub struct About {
    pub judge: String,
    pub pool: Result<Option<String>, Said>,
    pub chain_id: u64,
}

pub fn buyer_about(r: &crate::deskops::buyerpage::BRow) -> Result<About, Said> {
    use crate::deskops::buyerpage::{key_party, Access, E_READONLY};
    let judge = seated(&r.row).ok_or_else(not_seated)?;
    if let Err(note) = key_party(r) {
        return Err(if r.access == Access::Company { company_party() } else { Said::bad(E_READONLY, note) });
    }
    let pack = match r.pack.as_deref() {
        None => Err(listing_unread(t(Key::BoTermsMissing))),
        Some(b) => eravon_link::pack::pack(b).map_err(|f| listing_unread(f.token.name())),
    };
    let chain_id = pack.as_ref().map_or(eravon_literals::constants::active_standard().chain_id, |p| p.listing.chain_id);
    let pool = pack.map(|p| guild_of_listing(&p.listing, p.receipt.candidate));
    Ok(About { judge, pool, chain_id })
}

pub fn signer(path: Result<Option<String>, Said>) -> Result<(), Said> {
    match path {
        Ok(None) => Ok(()),
        Ok(Some(_)) => Err(company_party()),
        Err(s) => Err(s),
    }
}

pub fn seller_about(data: &str, addr: &str, r: &eravon_orders::table::Row, order: Option<&eravon_watch::order::Order>, path: impl FnOnce(&eravon_watch::order::Order) -> Result<Option<String>, Said>) -> Result<About, Said> {
    use eravon_watch::order as od;
    let judge = seated(r).ok_or_else(not_seated)?;
    if let Some(o) = order {
        signer(path(o))?;
    }
    let listing = match order {
        None => Err(listing_unread(t(Key::OrdersNoRecord))),
        Some(o) => od::part(data, addr, &o.id, &od::part_listing())
            .map_err(|(n, d)| listing_unread(&format!("{n} · {d}")))
            .and_then(|b| eravon_link::listing::listing(&b).map_err(|f| listing_unread(f.token.name())))
            .map(|l| (guild_of_listing(&l, o.candidate), l.chain_id)),
    };
    let chain_id = listing.as_ref().map_or(eravon_literals::constants::active_standard().chain_id, |(_, c)| *c);
    Ok(About { judge, pool: listing.map(|(g, _)| g), chain_id })
}

pub fn read(w: Result<eravon_cands::wire::Wire, eravon_cands::fault::Absent>, instance: &str, case_id: &str, judge: &str, pool: Result<Option<String>, Said>) -> Complaint {
    let absent = |a: eravon_cands::fault::Absent| Said::bad(a.name(), a.detail());
    let w = w.map_err(absent);
    let of = |c: eravon_cands::guild::Card| Guild { pointer: eravon_cands::table::pointer_of(&c.address), address: c.address.clone(), card: Ok(c) };
    let guild = pool.and_then(|p| match p {
        Some(g) => match w.and_then(|w| eravon_cands::registry::pool_guild(&w, &g).map_err(absent)) {
            Ok(c) => Ok(vec![Guild { pointer: eravon_cands::table::pointer_of(&c.address), address: c.address.clone(), card: Ok(c) }]),
            Err(s) if s.name == eravon_cands::fault::Absent::Retired.name() => Err(s),
            Err(s) => Ok(vec![Guild { pointer: eravon_cands::table::pointer_of(&g), address: g, card: Err(s) }]),
        },
        None => w.and_then(|w| eravon_cands::registry::guilds_listing(&w, judge).map_err(absent)).map(|v| v.into_iter().map(of).collect()),
    });
    Complaint { instance: instance.to_ascii_lowercase(), case_id: case_id.to_ascii_lowercase(), judge: judge.to_string(), guild, written: None }
}

pub struct Case<'a> {
    pub chain_id: u64,
    pub logic: &'a str,
    pub me: &'a str,
    pub role: Role,
    pub now: u64,
}

pub fn write(c: &Complaint, form: &Form, key: &[u8; 32], case: &Case<'_>) -> Result<String, Said> {
    let bad = |(n, d): eravon_dispute::Why| Said::bad(&n, d);
    let guild = match &c.guild {
        Err(s) => return Err(s.clone()),
        Ok(list) => picked(list, &form.guild).map(|g| g.pointer.clone()),
    };
    if guild.is_none() && c.guild.as_ref().is_ok_and(|l| l.len() > 1) {
        return Err(Said::bad("E_COMPLAINT_PICK", t(Key::CpPickGuild)));
    }
    if form.dir.trim().is_empty() {
        return Err(Said::bad("E_COMPLAINT_DIR", t(Key::ExNoDir)));
    }
    let attachments = form.files.iter().map(|(k, p)| cp::attachment(*k, p)).collect::<Result<Vec<_>, _>>().map_err(bad)?;
    let d = cp::Draft {
        chain_id: case.chain_id,
        instance: c.instance.clone(),
        case_id: c.case_id.clone(),
        logic: case.logic.to_string(),
        judge: c.judge.clone(),
        guild,
        complainant: case.me.to_string(),
        role: case.role,
        text: form.text.clone(),
        created_at: case.now,
        attachments,
    };
    cp::write(form.dir.trim(), &d, key).map_err(bad)
}

pub fn written_say(path: &str) -> String {
    let p = std::path::Path::new(path);
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let dir = p.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    filln(Key::CpWritten, &[&name, &dir])
}

pub fn picked<'a>(list: &'a [Guild], pick: &str) -> Option<&'a Guild> {
    match list {
        [one] => Some(one),
        _ => list.iter().find(|g| !pick.trim().is_empty() && g.address.eq_ignore_ascii_case(pick.trim())),
    }
}

pub fn home(c: &Complaint, addr: Option<&str>) -> Option<String> {
    let list = c.guild.as_ref().ok()?;
    let g = match addr {
        Some(a) => list.iter().find(|g| g.address.eq_ignore_ascii_case(a))?,
        None => list.first()?,
    };
    g.card.as_ref().ok()?.home.clone()
}

pub fn said_of(c: &Complaint) -> Said {
    match &c.written {
        Some(Ok(p)) => Said::ok(written_say(p)),
        Some(Err(s)) => s.clone(),
        None => match &c.guild {
            Err(s) => s.clone(),
            Ok(l) if l.is_empty() => Said::bad(cp::E_NO_GUILD, t(Key::ErrECOMPLAINTNOGUILD)),
            Ok(_) => Said::ok(t(Key::OrdersComplain)),
        },
    }
}

pub fn complaint_out(c: &Complaint) -> Out {
    let s = |x: &str| Out::S(x.to_string());
    let opt = |x: &Option<String>| Out::S(x.clone().unwrap_or_default());
    let guild = |g: &Guild| {
        Out::O(vec![
            ("address", s(&g.address)),
            (
                "card",
                match &g.card {
                    Ok(c) => Out::O(vec![("address", s(&c.address)), ("card", s(c.card)), ("home", opt(&c.home)), ("name", opt(&c.name)), ("words", opt(&c.words))]),
                    Err(e) => Out::O(vec![("refused", e.out())]),
                },
            ),
            ("pointer", s(&g.pointer)),
        ])
    };
    let mut m = vec![
        ("caseId", s(&c.case_id)),
        (
            "guilds",
            match &c.guild {
                Ok(l) => Out::A(l.iter().map(guild).collect()),
                Err(e) => Out::O(vec![("refused", e.out())]),
            },
        ),
        ("instance", s(&c.instance)),
        ("judge", s(&c.judge)),
        ("said", said_of(c).out()),
    ];
    if let Some(w) = &c.written {
        m.push((
            "written",
            match w {
                Ok(p) => Out::O(vec![("file", s(p))]),
                Err(e) => Out::O(vec![("refused", e.out())]),
            },
        ));
    }
    Out::O(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attachments_go_through_the_step_and_back() {
        let files = vec![(Kind::Dispatch, "/a/b/dispatch.json".to_string()), (Kind::AuditInput, "/a/审计 输入:1.json".to_string()), (Kind::Pass, "/p".into()), (Kind::Other, "/o".into())];
        assert_eq!(files_of(&files_arg(&files)), Ok(files));
        assert!(files_of("evidence:/x").is_err());
        assert_eq!(files_of(""), Ok(Vec::new()));
    }

    #[test]
    fn only_a_seated_judge_is_complained_of() {
        assert_eq!(seated_of(1, "0xAAAA000000000000000000000000000000000001").as_deref(), Some("0xaaaa000000000000000000000000000000000001"));
        assert_eq!(seated_of(0, "0xAAAA000000000000000000000000000000000001"), None);
        assert_eq!(seated_of(1, ""), None);
        assert_eq!(seated_of(1, "0x0000000000000000000000000000000000000000"), None);
    }

    #[test]
    fn the_complaint_signs_only_on_the_key_path() {
        assert!(signer(Ok(None)).is_ok());
        assert_eq!(signer(Ok(Some("0xc0".into()))).map_err(|s| s.name), Err(company_party().name));
        assert_eq!(signer(Err(Said::bad("E_ORDERS_SELLER_KEY", "x"))).map_err(|s| s.name), Err("E_ORDERS_SELLER_KEY".to_string()));
    }

    #[test]
    fn a_buyer_complains_only_on_an_order_its_key_holds() {
        use crate::deskops::buyerpage::{Access, BRow, E_READONLY};
        use eravon_terms::u256::U256;
        let view = |seats: u64| eravon_dispute::view::CaseView {
            phase: 0, seats, premium: U256::ZERO, deposit: U256::ZERO, lf_held: U256::ZERO, pending_redraw: false, redraw_deadline: 0,
            judge: "0x2222222222222222222222222222222222222222".into(), accepted: 0, window_end: 0, seat_state: None, amount: U256::ZERO,
            token: String::new(), buyer_payout: String::new(), seller_fund: String::new(), seller_payout: String::new(), lf: U256::ZERO,
            events: Vec::new(), past: false, logic: String::new(),
        };
        let row = |access: Access, seats: u64| BRow {
            row: eravon_orders::table::Row {
                instance: "0xabcdef0000000000000000000000000000000001".into(), case_id: String::new(), record: String::new(), number: String::new(),
                buyer: String::new(), label: String::new(), product: String::new(), amount: "10".into(), token: String::new(),
                status: Err((String::new(), String::new())), deadline: 0, placed: 0, block: None, chain: Some(view(seats)), local: None,
                handed: false, note: String::new(), terms_hash: String::new(), decimals: None, past: false, logic: String::new(), series: None,
            },
            owner: String::new(), record: String::new(), access, pack: None, seller: String::new(), product: String::new(), lf_gap: None, budget: None, receipt: None,
        };
        let a = buyer_about(&row(Access::Key, 1)).map_err(|s| s.name).expect("持钥那一单该说得出去处");
        assert_eq!(a.judge, "0x2222222222222222222222222222222222222222");
        assert_eq!(a.pool.map_err(|s| s.name), Err("E_COMPLAINT_LISTING".to_string()));
        assert_eq!(buyer_about(&row(Access::ByCall, 1)).map(|_| ()).map_err(|s| s.name), Err(E_READONLY.to_string()));
        assert_eq!(buyer_about(&row(Access::Company, 1)).map(|_| ()).map_err(|s| s.name), Err(cp::E_SIGNER.to_string()));
        assert_eq!(buyer_about(&row(Access::ByCall, 0)).map(|_| ()).map_err(|s| s.name), Err("E_COMPLAINT_SEAT".to_string()));
    }

    #[test]
    fn no_guild_no_file_and_a_guild_gets_a_signed_file() {
        let key = [5u8; 32];
        let me = eravon_terms::sig::sign_secp256k1(&key, &[0u8; 32]).unwrap().1;
        let dir = std::env::temp_dir().join(format!("eravon-cli-complaint-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let att = dir.join("pass.bin");
        std::fs::write(&att, b"pass").unwrap();
        let form = Form { text: "不裁".into(), dir: dir.to_string_lossy().to_string(), files: vec![(Kind::Pass, att.to_string_lossy().to_string())], guild: String::new() };
        let case = Case { chain_id: 11155111, logic: "0x1111111111111111111111111111111111111111", me: &me, role: Role::Seller, now: 1794240001 };
        let mut c = Complaint { instance: "0xabcdef0000000000000000000000000000000001".into(), case_id: format!("0x{}", "ab".repeat(32)), judge: "0x2222222222222222222222222222222222222222".into(), guild: Ok(Vec::new()), written: None };
        assert_eq!(write(&c, &form, &key, &case).map_err(|s| s.name), Err(cp::E_NO_GUILD.to_string()));
        assert_eq!(said_of(&c).name, cp::E_NO_GUILD);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "只有那份材料,没有投诉档");
        c.guild = Err(listing_unread("E_STORE_MISSING"));
        assert_eq!(write(&c, &form, &key, &case).map_err(|s| s.name), Err("E_COMPLAINT_LISTING".to_string()));
        let g = |a: &str| Guild { pointer: format!("galeed:guild:11155111:{a}"), address: a.to_string(), card: Err(Said::bad("E_UNREACHABLE", "")) };
        c.guild = Ok(vec![g("0x4444444444444444444444444444444444444444"), g("0x3333333333333333333333333333333333333333")]);
        assert_eq!(write(&c, &form, &key, &case).map_err(|s| s.name), Err("E_COMPLAINT_PICK".to_string()));
        let form = Form { guild: "0x3333333333333333333333333333333333333333".into(), ..form };
        let nodir = Form { dir: " ".into(), ..form.clone() };
        assert_eq!(write(&c, &nodir, &key, &case).map_err(|s| s.name), Err("E_COMPLAINT_DIR".to_string()));
        let path = write(&c, &form, &key, &case).expect("写得出");
        let b = std::fs::read(&path).unwrap();
        let v = eravon_galeed::canon::read_open(&b).unwrap();
        assert_eq!(v.get("guild").and_then(|x| x.as_str()), Some("galeed:guild:11155111:0x3333333333333333333333333333333333333333"));
        assert_eq!(v.get("complainant").and_then(|x| x.get("role")).and_then(|x| x.as_str()), Some("seller"));
        assert_eq!(v.get("caseId").and_then(|x| x.as_str()).map(str::to_string), Some(format!("0x{}", "ab".repeat(32))));
        assert_eq!(v.get("attachments").and_then(|x| x.as_arr()).map(Vec::len), Some(1));
        assert_eq!(cp::signer(&b).map(|x| x.0), Ok(me.clone()));
        assert!(written_say(&path).contains("complaint-abcdef00-1794240001.json"), "{}", written_say(&path));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
