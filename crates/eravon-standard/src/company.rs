use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_literals::constants as k7;
use eravon_literals::json::In;
use eravon_literals::literals::COMPANY_VIEWS;
use eravon_terms as k2;

use crate::abi::{self, Ty, Val};
use crate::fault::{Fault, R};
use crate::order;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    CalendarBound,
    Wallet,
    Ledger,
}

impl Face {
    pub const ALL: [Face; 3] = [Face::CalendarBound, Face::Wallet, Face::Ledger];

    pub const fn sig(self) -> &'static str {
        match self {
            Face::CalendarBound => COMPANY_VIEWS[0],
            Face::Wallet => COMPANY_VIEWS[1],
            Face::Ledger => COMPANY_VIEWS[2],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site {
    Company,
    Wallet,
    Ledger,
}

impl Site {
    pub const fn word(self) -> &'static str {
        match self {
            Site::Company => "company",
            Site::Wallet => "wallet",
            Site::Ledger => "ledger",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Unread,
    Disagree,
    Mismatch,
}

impl Kind {
    pub const fn word(self) -> &'static str {
        match self {
            Kind::Unread => "unread",
            Kind::Disagree => "disagree",
            Kind::Mismatch => "mismatch",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shut {
    pub kind: Kind,
    pub at: &'static str,
    pub why: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Views {
    pub calendar_bound: u64,
    pub wallet: String,
    pub ledger: String,
    pub agreement: &'static str,
    pub endpoints: usize,
}

fn fragment(face: Face) -> R<([u8; 4], Ty)> {
    surface(face.sig())
}

fn surface(sig: &'static str) -> R<([u8; 4], Ty)> {
    let (name, args) = sig.strip_suffix(')').and_then(|s| s.split_once('(')).unwrap_or((sig, ""));
    let want: Vec<&str> = if args.is_empty() { Vec::new() } else { args.split(',').collect() };
    let doc = eravon_literals::json::read(k7::CHELEK_ABI.company_views.as_bytes())
        .map_err(|e| Fault::Abi(format!("K7 的公司视图片段读不成:{e}")))?;
    let hit = doc.as_arr().unwrap_or(&[]).iter().find(|x| {
        x.get("name").and_then(In::as_str) == Some(name)
            && x.get("inputs").and_then(In::as_arr).is_some_and(|ins| ins.iter().map(|i| i.get("type").and_then(In::as_str)).eq(want.iter().map(|t| Some(*t))))
    });
    let Some(item) = hit else {
        return Err(Fault::Abi(format!("K7 的公司视图片段里没有 {sig}")));
    };
    let outs = item.get("outputs").and_then(In::as_arr).unwrap_or(&[]);
    let ty = match outs.iter().map(|o| o.get("type").and_then(In::as_str)).collect::<Vec<_>>().as_slice() {
        [Some("uint256")] => Ty::Uint,
        [Some("address")] => Ty::Addr,
        [Some("bool")] => Ty::Bool,
        other => return Err(Fault::Abi(format!("K7 片段里 {sig} 的返回形不是一枚 uint256、address 或 bool:{other:?}"))),
    };
    let h = k2::derive::keccak(sig.as_bytes());
    Ok(([h[0], h[1], h[2], h[3]], ty))
}

fn read_face<T: Transport>(
    t: &T,
    table: &Table,
    company: &str,
    face: Face,
) -> R<Result<(Val, a2::read::Reading), Shut>> {
    let (sel, ty) = fragment(face)?;
    read_view(t, table, company, face.sig(), &sel, &[], ty)
}

fn read_view<T: Transport>(
    t: &T,
    table: &Table,
    company: &str,
    at: &'static str,
    sel: &[u8; 4],
    args: &[Val],
    ty: Ty,
) -> R<Result<(Val, a2::read::Reading), Shut>> {
    let mut data = sel.to_vec();
    data.extend(abi::encode(args));
    let got = a2::read::call(t, table, company, &order::hex_of(&data), a2::seam::TAG_LATEST);
    let (hex, reading) = match got {
        Ok(x) => x,
        Err(f @ a2::fault::Fault::Usage(_)) => return Err(f.into()),
        Err(a2::fault::Fault::Disagree(d)) => {
            return Ok(Err(Shut { kind: Kind::Disagree, at, why: format!("端点之间答得不一样:{d}") }));
        }
        Err(f) => {
            return Ok(Err(Shut { kind: Kind::Unread, at, why: format!("读不到:{}", f.detail()) }));
        }
    };
    let decoded = order::bytes_of("答", &hex).and_then(|b| abi::decode(&[ty], &b));
    match decoded.map(|v| v.into_iter().next()) {
        Ok(Some(v)) => Ok(Ok((v, reading))),
        _ => Ok(Err(Shut {
            kind: Kind::Unread,
            at,
            why: format!("答读不成(公司地址上也许没有码):{hex}"),
        })),
    }
}

fn settle_face(_face: Face, shut: Shut) -> Result<Val, Shut> {
    Err(shut)
}


pub fn views<T: Transport>(t: &T, table: &Table, company: &str) -> R<Result<Views, Shut>> {
    crate::mark::mark(crate::mark::FEATURE);
    order::addr_of("公司地址", company)?;
    let mut vals = Vec::with_capacity(3);
    let mut single = false;
    let mut endpoints = table.len();
    let faces: Vec<(String, String)> = Face::ALL.iter().filter_map(|f| Some((company.to_string(), order::hex_of(&fragment(*f).ok()?.0)))).collect();
    a2::read::ahead_calls(t, table, None, &faces, a2::seam::TAG_LATEST);
    for face in Face::ALL {
        let v = match read_face(t, table, company, face)? {
            Ok((v, r)) => {
                single |= r.agreement == a2::read::Agreement::Single;
                endpoints = r.endpoints;
                v
            }
            Err(shut) => match settle_face(face, shut) {
                Ok(v) => v,
                Err(shut) => return Ok(Err(shut)),
            },
        };
        vals.push(v);
    }
    let bound = vals[0].small().map_err(|_| Fault::Abi("calendarBound() 超出 u64 的界,本处不截断".into()))?;
    Ok(Ok(Views {
        calendar_bound: bound,
        wallet: order::hex_of(&vals[1].addr()?),
        ledger: order::hex_of(&vals[2].addr()?),
        agreement: if single { a2::read::Agreement::Single.word() } else { a2::read::Agreement::Agreed.word() },
        endpoints,
    }))
}

pub fn holds<T: Transport>(t: &T, table: &Table, company: &str, who: &str) -> R<Result<bool, Shut>> {
    crate::mark::mark(crate::mark::FEATURE);
    order::addr_of("公司地址", company)?;
    let addr = order::addr_of("对手方地址", who)?;
    let sig = COMPANY_VIEWS[3];
    let (sel, ty) = surface(sig)?;
    Ok(match read_view(t, table, company, sig, &sel, &[Val::Addr(addr)], ty)? {
        Ok((v, _)) => Ok(v.boolean()?),
        Err(shut) => Err(shut),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub site: Site,
    pub address: String,
    pub code_hash: String,
}

fn same(got: &str, want: &str) -> bool {
    got.eq_ignore_ascii_case(want)
}


pub fn identity<T: Transport>(
    t: &T,
    table: &Table,
    company: &str,
    v: &Views,
) -> R<Result<[Checked; 3], Shut>> {
    crate::mark::mark(crate::mark::FEATURE);
    let id = k7::CHELEK_IDENTITY;
    let sites = [
        (Site::Company, company.to_ascii_lowercase(), id.proxy_code_hash),
        (Site::Wallet, v.wallet.clone(), id.wallet_code_hash),
        (Site::Ledger, v.ledger.clone(), id.ledger_code_hash),
    ];
    let mut out = Vec::with_capacity(3);
    a2::read::ahead_codes(t, table, &sites.iter().map(|(_, a, _)| a.clone()).collect::<Vec<_>>(), a2::seam::TAG_LATEST);
    for (site, address, want) in sites {
        let code = match a2::read::code(t, table, &address, a2::seam::TAG_LATEST) {
            Ok((c, _)) => c,
            Err(f @ a2::fault::Fault::Usage(_)) => return Err(f.into()),
            Err(a2::fault::Fault::Disagree(d)) => {
                return Ok(Err(Shut { kind: Kind::Disagree, at: site.word(), why: format!("端点之间答得不一样:{d}") }));
            }
            Err(f) => {
                return Ok(Err(Shut { kind: Kind::Unread, at: site.word(), why: format!("码读不到:{}", f.detail()) }));
            }
        };
        let got = if code.is_empty() {
            format!("0x{}", "0".repeat(64))
        } else {
            order::hex_of(&k2::derive::keccak(&code))
        };
        if !same(&got, want) {
            return Ok(Err(Shut {
                kind: Kind::Mismatch,
                at: site.word(),
                why: format!("{} 上的 codeHash 是 {got},CHELEK 身份表那一枚是 {want}", address),
            }));
        }
        out.push(Checked { site, address, code_hash: got });
    }
    let [a, b, c]: [Checked; 3] = out.try_into().map_err(|_| Fault::Abi("身份核对该恰三处".into()))?;
    Ok(Ok([a, b, c]))
}

pub fn open<T: Transport>(t: &T, table: &Table, company: &str) -> R<Result<(Views, [Checked; 3]), Shut>> {
    let v = match views(t, table, company)? {
        Ok(v) => v,
        Err(s) => return Ok(Err(s)),
    };
    Ok(identity(t, table, company, &v)?.map(|c| (v, c)))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Window {
        name: &'static str,
        who: String,
        secs: u64,
    },
    Deadline {
        name: &'static str,
        who: String,
        at: u64,
    },
}

pub fn resign_lead_item() -> Item {
    Item::Window { name: "resignLead", who: String::new(), secs: u64::from(k7::RESIGN_LEAD_DAYS) * 86_400 }
}

pub fn evidence_item<T: Transport>(t: &T, table: &Table, instance: &str, case_id: &[u8; 32]) -> R<Item> {
    let st = crate::chain::status(t, table, instance)?;
    let seats = st.value.get(crate::chain::status_col::SEATS).map(|v| v.small()).transpose()?.unwrap_or(0);
    let seat = crate::chain::seat_of(t, table, instance, case_id, seats.saturating_sub(1))?;
    let end = seat.value.get(crate::sol::src_abi::I_GALEED_VENUE_T_SEAT_COL_WINDOW_END).map(|v| v.small()).transpose()?.unwrap_or(0);
    Ok(Item::Deadline { name: "evidenceT3", who: String::new(), at: end })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: &'static str,
    pub who: String,
    pub available: u64,
    pub short: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub bound: u64,
    pub now: u64,
    pub enough: bool,
    pub rows: Vec<Row>,
}

impl Verdict {
    pub fn say_short(&self) -> String {
        self.rows
            .iter()
            .filter(|r| r.short > 0)
            .map(|r| if r.who.is_empty() { format!("{} 差 {} 秒", r.name, r.short) } else { format!("{}({}) 差 {} 秒", r.name, r.who, r.short) })
            .collect::<Vec<_>>()
            .join(";")
    }
}

fn short_of(available: u64, bound: u64) -> u64 {
    if available > bound { 0 } else { (bound - available).saturating_add(1) }
}


pub fn time_check(bound: u64, now: u64, items: &[Item]) -> Verdict {
    crate::mark::mark(crate::mark::FEATURE);
    let rows: Vec<Row> = items
        .iter()
        .map(|it| {
            let (name, who, available) = match it {
                Item::Window { name, who, secs } => (*name, who.clone(), *secs),
                Item::Deadline { name, who, at } => (*name, who.clone(), at.saturating_sub(now)),
            };
            Row { name, who, available, short: short_of(available, bound) }
        })
        .collect();
    Verdict { bound, now, enough: rows.iter().all(|r| r.short == 0), rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_faces_come_from_the_k7_fragment() {
        for f in Face::ALL {
            let (sel, _) = fragment(f).expect("K7 片段里有这一面");
            let h = k2::derive::keccak(f.sig().as_bytes());
            assert_eq!(sel, [h[0], h[1], h[2], h[3]]);
        }
        assert!(matches!(fragment(Face::CalendarBound).unwrap().1, Ty::Uint));
        assert!(matches!(fragment(Face::Wallet).unwrap().1, Ty::Addr));
        let (sel, ty) = surface(COMPANY_VIEWS[3]).expect("K7 片段里有 holds(address)");
        let h = k2::derive::keccak(b"holds(address)");
        assert_eq!((sel, matches!(ty, Ty::Bool)), ([h[0], h[1], h[2], h[3]], true));
    }

    #[test]
    fn a_view_missing_from_the_fragment_is_refused_by_name() {
        for sig in ["absent(address)", "holds(uint256)", "holds()"] {
            match surface(sig) {
                Err(Fault::Abi(m)) => assert!(m.contains(sig), "{m}"),
                other => panic!("{sig} 该拒:{other:?}"),
            }
        }
    }

    #[test]
    fn a_window_must_exceed_the_bound() {
        let w = |secs| Item::Window { name: "t1", who: String::new(), secs };
        let v = time_check(100, 0, &[w(101), w(100), w(99)]);
        assert!(!v.enough);
        assert_eq!(v.rows.iter().map(|r| r.short).collect::<Vec<_>>(), [0, 1, 2]);
        let d = Item::Deadline { name: "offerExpiry", who: String::new(), at: 1_000 };
        let v = time_check(100, 950, &[d.clone()]);
        assert_eq!((v.rows[0].available, v.rows[0].short), (50, 51));
        let v = time_check(100, 2_000, &[d]);
        assert_eq!((v.rows[0].available, v.rows[0].short), (0, 101));
        assert!(time_check(100, 0, &[w(101)]).enough);
    }
}
