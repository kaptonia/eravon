use eravon_literals::json::Out;
use eravon_standard as a1;
use a1::company::{Item, Row};
use a1::proposal::{Original, PCall};

use crate::fault::{Fault, R};
use crate::home::Ctx;
use crate::shared::{Flags, Wire};

pub struct Export {
    pub verb: &'static str,
    pub calls: Vec<PCall>,
    pub originals: Vec<Original>,
    pub items: Vec<Item>,
    pub face: Vec<String>,
    pub deal: Deal,
}

impl Export {
    pub fn of(p: a1::proposal::Plan) -> Export {
        Export { verb: p.verb, calls: p.calls, originals: p.originals, items: p.items, face: p.face, deal: Deal::Opened(p.instance) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Deal {
    Opened(String),
    Unopened(String),
    None,
}

pub struct Company {
    pub address: String,
    pub views: a1::company::Views,
}

pub fn asked(f: &Flags) -> Option<String> {
    f.opt("--export-proposal")
}

pub fn time_switch(c: &Ctx, f: &Flags, who: &str) -> R<bool> {
    match f.one("--time-check") {
        Some("on") => Ok(true),
        Some("off") => Ok(false),
        Some(other) => Err(Fault::Arg(format!("--time-check 只收 on 与 off,给的是 {other:?}"))),
        None => Ok(settings(c, who)?.time_check_on()),
    }
}

fn settings(c: &Ctx, who: &str) -> R<eravon_store::settings::Identity> {
    match eravon_vault::store::read_identity(&c.data, who) {
        Ok(s) => Ok(s),
        Err(e) if e.name() == "E_STORE_MISSING" => Ok(eravon_store::settings::Identity::default()),
        Err(e) => Err(Fault::Down { layer: "A5", token: e.name().to_string(), why: e.detail() }),
    }
}

pub fn company(c: &Ctx, f: &Flags, w: &Wire, who: &str) -> R<Company> {
    let address = match f.opt("--company") {
        Some(a) => a,
        None => settings(c, who)?.company,
    };
    if address.is_empty() {
        return Err(Fault::Arg("公司模式要公司地址:设置里没有,也没给 --company".into()));
    }
    match a1::company::open(&w.t, &w.table, &address)? {
        Ok((views, _)) => Ok(Company { address: address.to_ascii_lowercase(), views }),
        Err(s) => Err(Fault::Precheck {
            gate: crate::fault::GATE_COMPANY.into(),
            why: format!("公司模式不可开:{} 处 {}({})", s.at, s.kind.word(), s.why),
        }),
    }
}

fn row_out(r: &Row) -> Out {
    Out::O(vec![
        ("available", Out::N(r.available)),
        ("item", Out::S(r.name.to_string())),
        ("short", Out::N(r.short)),
        ("who", Out::S(r.who.clone())),
    ])
}

fn short_say(rows: &[Row]) -> String {
    rows.iter()
        .filter(|r| r.short > 0)
        .map(|r| if r.who.is_empty() { format!("{} 差 {} 秒", r.name, r.short) } else { format!("{}({}) 差 {} 秒", r.name, r.who, r.short) })
        .collect::<Vec<_>>()
        .join(";")
}

pub fn export(c: &Ctx, w: &Wire, co: &Company, path: &str, gate_on: bool, e: Export) -> R<Vec<(&'static str, Out)>> {
    write(c.now, w, co, path, gate_on, true, e)
}

pub const HOLDER: &str = "holder";

fn holder_check(w: &Wire, co: &Company, deal: &Deal) -> R<()> {
    let other = match deal {
        Deal::None => Vec::new(),
        Deal::Unopened(key) => vec![key.to_ascii_lowercase()],
        Deal::Opened(instance) => {
            let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
            let (seller, buyer) = eravon_dispute::settle::parties(&dw, instance).map_err(|(token, why)| Fault::Down { layer: "P6", token, why })?;
            [seller, buyer].into_iter().map(|a| a.to_ascii_lowercase()).filter(|a| !a.eq_ignore_ascii_case(&co.address)).collect()
        }
    };
    for who in &other {
        match a1::company::holds(&w.t, &w.table, &co.address, who)? {
            Ok(false) => {}
            Ok(true) => {
                return Err(Fault::Precheck { gate: HOLDER.into(), why: format!("对手方 {who} 持本公司席位,能否决本公司在本案的每个动作:不导出") });
            }
            Err(s) => {
                return Err(Fault::Precheck { gate: HOLDER.into(), why: format!("对手方 {who} 是否持本公司席位读不出:{} 处 {}({})", s.at, s.kind.word(), s.why) });
            }
        }
    }
    Ok(())
}

pub const TIME_ASK: &str = "time-ask";

pub fn ask_say(bound: u64, short: &str) -> String {
    format!("时间核对关着,期限对日历上界 {bound} 秒不够:{short}")
}

pub fn time_asked(f: &Fault) -> Option<String> {
    match f {
        Fault::Precheck { gate, why } if gate == TIME_ASK => Some(why.clone()),
        _ => None,
    }
}

pub fn asks(confirmed: bool) -> bool {
    !confirmed
}


pub fn face_of(rows: &[(&'static str, Out)]) -> Vec<String> {
    rows.iter()
        .find(|(k, _)| *k == "face")
        .map(|(_, v)| match v {
            Out::A(xs) => xs.iter().filter_map(|x| if let Out::S(s) = x { Some(s.clone()) } else { None }).collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default()
}

pub fn time_short(rows: &[(&'static str, Out)]) -> bool {
    rows.iter().any(|(k, v)| *k == "time" && matches!(v, Out::O(xs) if xs.iter().any(|(n, b)| *n == "enough" && matches!(b, Out::B(false)))))
}

pub fn write(now: u64, w: &Wire, co: &Company, path: &str, gate_on: bool, confirmed: bool, e: Export) -> R<Vec<(&'static str, Out)>> {
    let chain_id = w.chain_id;
    holder_check(w, co, &e.deal)?;
    a1::gate::proposal_senders(&w.t, &w.table, &co.address, &e.calls)?;
    let verdict = a1::company::time_check(co.views.calendar_bound, now, &e.items);
    if !verdict.enough && gate_on {
        return Err(Fault::Precheck {
            gate: crate::fault::GATE_TIME.into(),
            why: format!("时间核对开着,期限对日历上界 {} 秒不够:{}", verdict.bound, short_say(&verdict.rows)),
        });
    }
    if !verdict.enough && asks(confirmed) {
        return Err(Fault::Precheck { gate: TIME_ASK.into(), why: ask_say(verdict.bound, &short_say(&verdict.rows)) });
    }
    let mut face = e.face.clone();
    if !verdict.enough {
        face.push(format!("时间核对关着,照导出;期限对日历上界 {} 秒不够:{}。", verdict.bound, short_say(&verdict.rows)));
    }
    let (p, kept) = a1::proposal::assemble(chain_id, e.calls, e.originals, vec!["eravon".into(), e.verb.into()], face.join("\n"), &eravon_galeed::hash::sha256)?;
    let docs = p.documents.clone();
    let bytes = p.bytes()?;
    let dps = kept.iter().map(|o| crate::seam::proposal_doc_path(path, &o.name).map_err(Fault::Arg)).collect::<R<Vec<_>>>()?;
    crate::write::free(&std::iter::once((path, bytes.as_slice())).chain(dps.iter().zip(&kept).map(|(dp, o)| (dp.as_str(), o.bytes.as_slice()))).collect::<Vec<_>>())?;
    if let Some(dir) = std::path::Path::new(path).parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|x| Fault::Io(format!("{} 建不了:{x}", dir.display())))?;
    }
    crate::write::land(path, &bytes)?;
    for (o, dp) in kept.iter().zip(&dps) {
        crate::write::land(dp, &o.bytes)?;
    }
    let hash = eravon_galeed::hash::sha256(&bytes);
    Ok(vec![
        (
            "company",
            Out::O(vec![
                ("address", Out::S(co.address.clone())),
                ("agreement", Out::S(co.views.agreement.to_string())),
                ("calendarBound", Out::N(co.views.calendar_bound)),
                ("ledger", Out::S(co.views.ledger.clone())),
                ("wallet", Out::S(co.views.wallet.clone())),
            ]),
        ),
        ("face", Out::A(face.into_iter().map(Out::S).collect())),
        (
            "proposal",
            Out::O(vec![
                ("calls", Out::N(p.calls.len() as u64)),
                ("digest", Out::S(a1::order::hex_of(&p.digest()))),
                (
                    "documents",
                    Out::A(
                        docs.iter()
                            .map(|d| {
                                Out::O(vec![
                                    ("kind", Out::S(d.kind.clone())),
                                    ("name", Out::S(d.name.clone())),
                                    ("sha256", Out::S(a1::order::hex_of(&d.sha256))),
                                ])
                            })
                            .collect(),
                    ),
                ),
                ("file", Out::S(path.to_string())),
                ("proposalHash", Out::S(a1::order::hex_of(&hash))),
                ("structural", Out::B(p.structural())),
            ]),
        ),
        ("sent", Out::B(false)),
        (
            "time",
            Out::O(vec![
                ("bound", Out::N(verdict.bound)),
                ("enough", Out::B(verdict.enough)),
                ("gate", Out::B(gate_on)),
                ("rows", Out::A(verdict.rows.iter().map(row_out).collect())),
            ]),
        ),
        ("why", Out::S("公司模式:这一笔须以公司身份调用或动用公司的钱,写成提案交公司的桌,本席不签不发".into())),
    ])
}

pub fn instance_call(call: &a1::call::Call, fun: a1::sol::Fun) -> R<(PCall, Original)> {
    Ok((a1::proposal::of_call(call)?, a1::proposal::abi_doc(&[fun], a1::proposal::INSTANCE_ABI_NAME)?))
}

pub fn evidence_item(w: &Wire, instance: &str, case_id: &[u8; 32]) -> R<Item> {
    Ok(a1::company::evidence_item(&w.t, &w.table, instance, case_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    const COMPANY: &str = "0x00000000000000000000000000000000000c0c0c";
    const HOLDER: &str = "0x00000000000000000000000000000000000a11ce";
    const STRANGER: &str = "0x0000000000000000000000000000000000000b0b";

    fn node(garbled: bool) -> String {
        node_opened(garbled, STRANGER, COMPANY)
    }

    fn node_says_yes() -> String {
        node_with(Answers::Yes, String::new(), String::new())
    }

    #[derive(Clone, Copy)]
    enum Answers {
        ByHolder,
        Garbled,
        Yes,
    }

    const INSTANCE: &str = "0x00000000000000000000000000000000000001e5";

    fn node_opened(garbled: bool, seller: &'static str, buyer: &'static str) -> String {
        node_with(if garbled { Answers::Garbled } else { Answers::ByHolder }, seller.to_string(), buyer.to_string())
    }

    fn node_with(answers: Answers, seller: String, buyer: String) -> String {
        let topic = |a: &str| format!("0x{}{}", "0".repeat(24), &a[2..]);
        let opened = if seller.is_empty() { "[]".to_string() } else { format!(
            r#"[{{"address":"{INSTANCE}","topics":["{}","0x{}","{}","{}"],"data":"0x","blockNumber":"0x5","logIndex":"0x0","transactionHash":"0x{}","blockHash":"0x{}","transactionIndex":"0x0","removed":false}}]"#,
            a1::order::hex_of(&a1::sol::OPENED.topic0()),
            "ca".repeat(32),
            topic(&seller),
            topic(&buyer),
            "0e".repeat(32),
            "0b".repeat(32)
        ) };
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}", l.local_addr().expect("addr"));
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let opened = opened.clone();
                std::thread::spawn(move || {
                    let mut s = s;
                    let mut raw = Vec::new();
                    let mut buf = [0u8; 4096];
                    let req = loop {
                        match s.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => raw.extend_from_slice(&buf[..n]),
                        }
                        let text = String::from_utf8_lossy(&raw).to_string();
                        if let Some(i) = text.find("\r\n\r\n") {
                            let len: usize = text[..i].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|n| n.trim().parse().ok())).unwrap_or(0);
                            if raw.len() >= i + 4 + len {
                                break text[i + 4..].to_string();
                            }
                        }
                    };
                    let id = req.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                    let method = req.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                    let result = match method.as_str() {
                        "eth_chainId" => "\"0x7a69\"".to_string(),
                        "eth_blockNumber" => "\"0x10\"".to_string(),
                        "eth_call" => match answers {
                            Answers::Garbled => "\"0x01\"".to_string(),
                            Answers::Yes => format!("\"0x{:064x}\"", 1),
                            Answers::ByHolder => format!("\"0x{:064x}\"", u8::from(req.contains(&HOLDER[2..]))),
                        },
                        "eth_getLogs" => opened.clone(),
                        _ => "null".to_string(),
                    };
                    let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}");
                    let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
                });
            }
        });
        url
    }

    fn wire(url: String) -> Wire {
        Wire {
            t: eravon_chain::transport::Auto::web_roots().expect("transport"),
            table: eravon_chain::endpoints::Table::new(vec![url]).expect("table"),
            source: crate::home::Source::Identity,
            chain_id: 1,
            logic: format!("0x{}", "1".repeat(40)),
            registries: Vec::new(),
        }
    }

    fn company() -> Company {
        let wallet = format!("0x{}", "2".repeat(40));
        let views = a1::company::Views { calendar_bound: 0, wallet, ledger: format!("0x{}", "3".repeat(40)), agreement: "agreed", endpoints: 1 };
        Company { address: COMPANY.into(), views }
    }

    fn export(deal: Deal) -> Export {
        let co = company();
        let (pc, abi) = a1::proposal::wallet_pay(&co.views.wallet, &format!("0x{}", "4".repeat(40)), &format!("0x{}", "5".repeat(40)), &[1u8; 32]).expect("pay call");
        Export { verb: "order-sign", calls: vec![pc], originals: vec![abi], items: Vec::new(), face: Vec::new(), deal }
    }

    fn dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("holder-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("dir");
        d
    }

    #[test]
    fn a_deal_whose_other_side_holds_a_seat_is_not_exported() {
        let w = wire(node(false));
        let co = company();
        let d = dir("seat");
        let path = d.join("proposal.json").display().to_string();
        match write(1, &w, &co, &path, true, true, export(Deal::Unopened(HOLDER.to_ascii_uppercase().replacen("0X", "0x", 1)))) {
            Err(Fault::Precheck { gate, why }) => assert!(gate == HOLDER_GATE && why.contains("持本公司席位"), "{gate}:{why}"),
            other => panic!("持席者该拒:{other:?}"),
        }
        assert!(!std::path::Path::new(&path).exists(), "拒了就不落档");
        write(1, &w, &co, &path, true, true, export(Deal::Unopened(STRANGER.into()))).expect("不持席照导出");
        assert!(std::path::Path::new(&path).exists());
        let path2 = d.join("other.json").display().to_string();
        match write(1, &wire(node(true)), &co, &path2, true, true, export(Deal::Unopened(STRANGER.into()))) {
            Err(Fault::Precheck { gate, why }) => assert!(gate == HOLDER_GATE && why.contains("读不出"), "{gate}:{why}"),
            other => panic!("读不出该拒:{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_opened_deal_reads_its_other_side_from_the_chain() {
        let co = company();
        let d = dir("opened");
        let path = d.join("proposal.json").display().to_string();
        let w = wire(node_opened(false, HOLDER, COMPANY));
        match write(1, &w, &co, &path, true, true, export(Deal::Opened(INSTANCE.into()))) {
            Err(Fault::Precheck { gate, why }) => assert!(gate == HOLDER_GATE && why.contains(HOLDER), "{gate}:{why}"),
            other => panic!("Opened 里的卖家持席,该拒:{other:?}"),
        }
        assert!(!std::path::Path::new(&path).exists());
        write(1, &wire(node_opened(false, STRANGER, COMPANY)), &co, &path, true, true, export(Deal::Opened(INSTANCE.into()))).expect("对手方不持席照导出");
        assert!(write(1, &w, &co, &d.join("blank.json").display().to_string(), true, true, export(Deal::Unopened(String::new()))).is_err(), "空的对手方钥不放行");
        write(1, &w, &co, &d.join("none.json").display().to_string(), true, true, export(Deal::None)).expect("无成交照导出");
        let nothing = wire(node_says_yes());
        assert!(matches!(write(1, &nothing, &co, &d.join("unopened.json").display().to_string(), true, true, export(Deal::Opened(INSTANCE.into()))), Err(Fault::Down { .. })));
        let mut split = wire(node(false));
        split.table = eravon_chain::endpoints::Table::new(vec![node(false), node_says_yes()]).expect("table");
        match write(1, &split, &co, &d.join("split.json").display().to_string(), true, true, export(Deal::Unopened(STRANGER.into()))) {
            Err(Fault::Precheck { gate, why }) => assert!(gate == HOLDER_GATE && why.contains("读不出"), "{gate}:{why}"),
            other => panic!("端点不一致该拒:{other:?}"),
        }
        assert!(!d.join("unopened.json").exists() && !d.join("split.json").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    use super::HOLDER as HOLDER_GATE;
}
