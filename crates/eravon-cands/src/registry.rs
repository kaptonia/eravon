use eravon_chain::endpoints::Table;
use eravon_chain::seam::Upto;
use eravon_chain::transport::Transport;
use eravon_roster::adapter::{self, Doc, Logs, Publication, Status};
use eravon_scan::log::Log;
use eravon_standard::order::hex_of;
use eravon_standard::sol::REGISTRY_EVENTS;

use crate::fault::Absent;
use crate::wire::Wire;

pub fn logs_of(w: &Wire, addrs: &[String]) -> Result<Logs, Absent> {
    logs_over(&w.t, &w.table, &w.registries, addrs)
}

pub fn logs_over<T: Transport>(t: &T, table: &Table, registries: &[String], addrs: &[String]) -> Result<Logs, Absent> {
    let mut out = Logs::default();
    let mut asked: Vec<String> = Vec::new();
    let mut layer: Vec<String> = Vec::new();
    for a in addrs {
        let a = a.to_ascii_lowercase();
        if !layer.contains(&a) {
            layer.push(a);
        }
    }
    if layer.is_empty() {
        return Ok(out);
    }
    let book = eravon_scan::succession::logs(t, table).map_err(|e| Absent::Unreachable(e.name()))?;
    let (tt, tb) = (t.clone(), table.clone());
    let rows: Vec<Vec<eravon_literals::json::In>> = eravon_chain::read::side_by_side(registries.to_vec(), move |reg| rows_of(&tt, &tb, &reg)).into_iter().collect::<Result<_, _>>()?;
    while !layer.is_empty() {
        let before = out.publications.len();
        read(&rows, &layer, &mut out)?;
        let moved = moved_on(&book, &layer);
        asked.extend(layer.drain(..));
        let named = out.publications[before..].iter().flat_map(|p| match adapter::decode(p) {
            Ok(Doc::Roster(r)) => r.judges.iter().cloned().chain(r.bench_view().map(|b| b.verifiers.into_iter().map(|v| v.addr).collect::<Vec<_>>()).unwrap_or_default()).collect(),
            _ => Vec::new(),
        });
        for a in named.chain(moved).collect::<Vec<String>>() {
            let a = a.to_ascii_lowercase();
            if !asked.contains(&a) && !layer.contains(&a) {
                layer.push(a);
            }
        }
    }
    out.declarations = book.declarations;
    out.accessions = book.accessions;
    Ok(out)
}

fn moved_on(book: &Logs, from: &[String]) -> Vec<String> {
    let of = |a: &str| from.iter().any(|f| f.eq_ignore_ascii_case(a));
    let declared = book.declarations.iter().filter(|d| of(&d.emitter)).map(|d| d.to.clone());
    let acceded = book.accessions.iter().filter(|x| of(&x.holder) || of(&x.root)).map(|x| x.emitter.clone());
    declared.chain(acceded).collect()
}

fn rows_of<T: Transport>(t: &T, table: &Table, reg: &str) -> Result<Vec<eravon_literals::json::In>, Absent> {
    let unreachable = |e: eravon_chain::fault::Fault| Absent::Unreachable(format!("{}:{}", e.name(), reg));
    let base = eravon_literals::constants::registry_from(reg);
    let filter = [vec![hex_of(&REGISTRY_EVENTS[0].topic0()), hex_of(&REGISTRY_EVENTS[1].topic0())]];
    eravon_chain::read::logs_any_whole(t, table, Some(reg), &filter, base, Upto::Latest).map(|(r, _)| r).map_err(unreachable)
}

fn published_all(w: &Wire) -> Result<Logs, Absent> {
    let book = eravon_scan::succession::logs(&w.t, &w.table).map_err(|e| Absent::Unreachable(e.name()))?;
    let (tt, tb) = (w.t.clone(), w.table.clone());
    let rows: Vec<Vec<eravon_literals::json::In>> = eravon_chain::read::side_by_side(w.registries.clone(), move |reg| rows_of(&tt, &tb, &reg)).into_iter().collect::<Result<_, _>>()?;
    let mut out = Logs::default();
    read_where(&rows, &|_| true, &mut out)?;
    out.declarations = book.declarations;
    out.accessions = book.accessions;
    Ok(out)
}

pub fn guilds_listing(w: &Wire, judge: &str) -> Result<Vec<crate::guild::Card>, Absent> {
    let logs = published_all(w)?;
    Ok(listing_of(&logs, judge).iter().map(|g| crate::guild::card_of(&logs, g)).collect())
}

pub fn pool_guild(w: &Wire, g: &str) -> Result<crate::guild::Card, Absent> {
    let logs = published_all(w)?;
    match guild_now(&logs, g) {
        Err(adapter::Refusal::Retired) => Err(Absent::Retired),
        _ => Ok(crate::guild::card_of(&logs, g)),
    }
}

fn guild_now(logs: &Logs, g: &str) -> Result<adapter::Guild, adapter::Refusal> {
    adapter::guild_of(logs, eravon_roster::pool::standard_chain_id(), &g.to_ascii_lowercase())
}

pub fn listing_of(logs: &Logs, judge: &str) -> Vec<String> {
    let judge = judge.to_ascii_lowercase();
    let mut publishers: Vec<String> = logs.publications.iter().map(|p| eravon_galeed::book::current(logs, &p.emitter.to_ascii_lowercase()).to_ascii_lowercase()).collect();
    publishers.sort();
    publishers.dedup();
    let mut out: Vec<String> = publishers
        .into_iter()
        .filter_map(|g| guild_now(logs, &g).ok())
        .filter(|gl| gl.judges.iter().any(|row| row.listed.eq_ignore_ascii_case(&judge) || row.keys.iter().any(|k| k.eq_ignore_ascii_case(&judge))))
        .map(|gl| gl.guild.to_ascii_lowercase())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn read(rows: &[Vec<eravon_literals::json::In>], part: &[String], out: &mut Logs) -> Result<(), Absent> {
    read_where(rows, &|emitter| part.iter().any(|p| p == emitter), out)
}

fn read_where(rows: &[Vec<eravon_literals::json::In>], keep: &dyn Fn(&str) -> bool, out: &mut Logs) -> Result<(), Absent> {
    let reg_topic = hex_of(&REGISTRY_EVENTS[0].topic0());
    let status_topic = hex_of(&REGISTRY_EVENTS[1].topic0());
    for rows in rows {
        for row in rows {
            let l = Log::read(row).map_err(|e| Absent::Unreachable(e.name()))?;
            let t0 = l.topics.first().map(|x| hex_of(x)).unwrap_or_default();
            let is_pub = match t0 {
                x if x == reg_topic => true,
                x if x == status_topic => false,
                _ => continue,
            };
            let emitter = l.topic_address(1).map_err(|e| Absent::Unreachable(e.name()))?.to_ascii_lowercase();
            if !keep(&emitter) {
                continue;
            }
            let v = eravon_standard::event::decode(&l.topics, &l.data)
                .map_err(|e| Absent::Unverified(format!("无法读取{}记录:{}", if is_pub { "登记" } else { "状态" }, e.name())))?;
            if is_pub {
                let document = v.at(2).and_then(|x| x.bytes()).map_err(|e| Absent::Unverified(e.name()))?;
                let word = v.at(1).and_then(|x| x.word()).map_err(|e| Absent::Unverified(e.name()))?;
                out.publications.push(Publication {
                    emitter,
                    block_number: l.block,
                    log_index: l.log_index,
                    document: document.to_vec(),
                    word: hex_of(&word),
                });
            } else {
                out.statuses.push(Status {
                    emitter,
                    block_number: l.block,
                    log_index: l.log_index,
                    active: v.at(1).and_then(|x| x.boolean()).map_err(|e| Absent::Unverified(e.name()))?,
                });
            }
        }
    }
    Ok(())
}

pub fn stamp(w: &Wire, block: u64) -> Option<u64> {
    eravon_scan::scan::block_stamp(&w.t, &w.table, block).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guilds_whose_current_roster_lists_a_judge() {
        let pubd = |e: &str, b: u64, doc: String| {
            let document = doc.into_bytes();
            Publication { emitter: e.to_string(), block_number: b, log_index: 0, word: hex_of(&eravon_galeed::hash::keccak256(&document)), document }
        };
        let chain = eravon_roster::pool::standard_chain_id();
        let roster = |g: &str, js: &[&str]| format!(r#"{{"chainId":{chain},"guild":"{g}","issued":1790000000,"judges":[{}],"kind":"galeed.roster/1"}}"#, js.iter().map(|j| format!("\"{j}\"")).collect::<Vec<_>>().join(","));
        let j = format!("0x{}", "3d".repeat(20));
        let other = format!("0x{}", "77".repeat(20));
        let (g1, g2, g3, g4) = (format!("0x{}", "90".repeat(20)), format!("0x{}", "a1".repeat(20)), format!("0x{}", "d2".repeat(20)), format!("0x{}", "c3".repeat(20)));
        let logs = Logs {
            publications: vec![pubd(&g1, 5, roster(&g1, &[&other, &j])), pubd(&g2, 5, roster(&g2, &[&j])), pubd(&g2, 9, roster(&g2, &[&other])), pubd(&g3, 6, roster(&g4, &[&j])), pubd(&g4, 7, roster(&g4, &[&j]))],
            ..Logs::default()
        };
        assert_eq!(listing_of(&logs, &j.to_ascii_uppercase().replace("0X", "0x")), vec![g1.clone(), g4.clone()]);
        assert_eq!(listing_of(&logs, &format!("0x{}", "55".repeat(20))), Vec::<String>::new());
        let (old, new) = (format!("0x{}", "8b".repeat(20)), format!("0x{}", "f3".repeat(20)));
        let moved = Logs {
            publications: vec![pubd(&old, 5, roster(&old, &[&j])), pubd(&new, 9, roster(&new, &[&j]))],
            declarations: vec![eravon_galeed::book::Declaration { emitter: old.clone(), block_number: 7, log_index: 0, to: new.clone() }],
            accessions: vec![eravon_galeed::book::Accession { emitter: new.clone(), block_number: 8, log_index: 0, holder: old.clone(), root: old.clone() }],
            ..Logs::default()
        };
        assert_eq!(listing_of(&moved, &j), vec![new.clone()]);
        let j2 = format!("0x{}", "4e".repeat(20));
        let (g5, g6) = (format!("0x{}", "d5".repeat(20)), format!("0x{}", "f6".repeat(20)));
        let rotated = Logs {
            publications: vec![pubd(&g5, 9, roster(&g5, &[&j2])), pubd(&g6, 9, roster(&g6, &[&j]))],
            statuses: vec![Status { emitter: g6.clone(), block_number: 10, log_index: 0, active: false }],
            declarations: vec![eravon_galeed::book::Declaration { emitter: j.clone(), block_number: 7, log_index: 0, to: j2.clone() }],
            accessions: vec![eravon_galeed::book::Accession { emitter: j2.clone(), block_number: 8, log_index: 0, holder: j.clone(), root: j.clone() }],
        };
        assert_eq!(listing_of(&rotated, &j), vec![g5.clone()], "seated under the former key, listed under the new one; the retired guild left out");
        assert_eq!(listing_of(&rotated, &j2), vec![g5.clone()]);
    }

    #[derive(Clone)]
    struct Reg {
        asked: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl Reg {
        fn answer_one(&self, _url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<eravon_chain::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let result = if b.contains("eth_chainId") {
                "\"0x7a69\"".to_string()
            } else if b.contains("eth_blockNumber") {
                "\"0x7fffffff\"".to_string()
            } else {
                self.asked.lock().unwrap().push(b.clone());
                let status = hex_of(&REGISTRY_EVENTS[1].topic0());
                if !b.contains(&status) {
                    "[]".to_string()
                } else {
                    let stranger = format!("0x{}{}", "0".repeat(24), "ee".repeat(20));
                    let rows: Vec<String> = (0..40)
                        .map(|i| format!("0x{}{}", "0".repeat(24), &addr(i)[2..]))
                        .chain(std::iter::once(stranger))
                        .enumerate()
                        .map(|(i, j)| format!(r#"{{"address":"0x{}","topics":["{status}","{j}"],"data":"0x{:064x}","blockNumber":"0x1","logIndex":"0x{i:x}","transactionHash":"0x{}"}}"#, "aa".repeat(20), 1, "11".repeat(32)))
                        .collect();
                    format!("[{}]", rows.join(","))
                }
            };
            Ok(eravon_chain::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    impl Transport for Reg {
        fn post(&self, url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<eravon_chain::transport::Answer> {
            eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
        }
    }

    fn over(addrs: &[String]) -> (Logs, Vec<String>) {
        let t = Reg { asked: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())) };
        let table = Table::new(vec!["http://127.0.0.1:1".into()]).unwrap();
        let logs = logs_over(&t, &table, &[format!("0x{}", "aa".repeat(20))], addrs).unwrap();
        (logs, t.asked.lock().unwrap().clone())
    }

    fn addr(i: usize) -> String {
        format!("0x{:040x}", i + 1)
    }

    #[derive(Clone)]
    struct Grown {
        head: std::sync::Arc<std::sync::atomic::AtomicU64>,
        froms: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
    }

    impl Grown {
        fn answer_one(&self, _url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<eravon_chain::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let status = hex_of(&REGISTRY_EVENTS[1].topic0());
            let result = if b.contains("eth_chainId") {
                "\"0x7a69\"".to_string()
            } else if b.contains("eth_blockNumber") {
                format!("\"0x{:x}\"", self.head.load(std::sync::atomic::Ordering::SeqCst))
            } else if !b.contains(&format!("\"0x{}\"", "bb".repeat(20))) {
                "[]".to_string()
            } else {
                let from = b.split("\"fromBlock\":\"0x").nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
                self.froms.lock().unwrap().push(from);
                if from > 5 {
                    "[]".to_string()
                } else {
                    format!(r#"[{{"address":"0x{}","topics":["{status}","0x{}{}"],"data":"0x{:064x}","blockNumber":"0x5","logIndex":"0x0","transactionHash":"0x{}"}}]"#, "bb".repeat(20), "0".repeat(24), &addr(0)[2..], 1, "22".repeat(32))
                }
            };
            Ok(eravon_chain::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    impl Transport for Grown {
        fn post(&self, url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<eravon_chain::transport::Answer> {
            eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
        }
    }

    #[derive(Default)]
    struct Record(std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

    impl eravon_chain::settled::Keep for Record {
        fn load(&self, part: &str) -> Option<Vec<u8>> {
            self.0.lock().unwrap().get(part).cloned()
        }
        fn save(&self, part: &str, plain: &[u8]) -> Result<(), eravon_chain::settled::Unkept> {
            self.0.lock().unwrap().insert(part.to_string(), plain.to_vec());
            Ok(())
        }
        fn parts(&self) -> Vec<String> {
            self.0.lock().unwrap().keys().cloned().collect()
        }
    }

    #[test]
    fn a_registrys_rows_are_one_key_read_once_then_only_onward() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let t = Grown { head: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1_000)), froms: Default::default() };
        let table = Table::new(vec!["http://127.0.0.1:2".into()]).unwrap();
        let reg = [format!("0x{}", "bb".repeat(20))];
        let record: std::sync::Arc<Record> = Default::default();
        {
            let _a = eravon_chain::action::open(Some(record.clone()));
            let one = logs_over(&t, &table, &reg, &[addr(0)]).unwrap();
            assert_eq!(one.statuses.len(), 1);
        }
        assert_eq!(*t.froms.lock().unwrap(), vec![0], "头一读:登记与状态一问,自部署块问起");
        t.head.store(1_100, std::sync::atomic::Ordering::SeqCst);
        {
            let _a = eravon_chain::action::open(Some(record.clone()));
            let two = logs_over(&t, &table, &reg, &[addr(0), addr(1)]).unwrap();
            assert_eq!(two.statuses.len(), 1, "记录恰一回;别的判官不另问");
        }
        let settled = 1_000 - eravon_chain::settled::MARGIN;
        assert_eq!(t.froms.lock().unwrap()[1..], [settled + 1], "第二读只问落定之后");
        let _a = eravon_chain::action::open(None);
        let _ = logs_over(&t, &table, &reg, &[addr(0)]).unwrap();
        assert_eq!(t.froms.lock().unwrap()[2..], [0], "不留读数的动作整段问");
    }

    #[test]
    fn a_registry_read_is_one_query_and_keeps_out_the_rest() {
        let (logs, asked) = over(&[]);
        assert_eq!((logs.statuses.len(), asked.len()), (0, 0), "名单为空:一问不发");
        let three: Vec<String> = (0..3).map(addr).collect();
        let (logs, asked) = over(&three);
        let reg_queries: Vec<&String> = asked.iter().filter(|b| b.contains(&"aa".repeat(20))).collect();
        assert_eq!((asked.len(), reg_queries.len()), (2, 1), "换钥簿一问,登记合约一问:{asked:?}");
        let pubs = hex_of(&REGISTRY_EVENTS[0].topic0());
        assert!(reg_queries[0].contains(&pubs) && !reg_queries[0].contains(&addr(0)[2..]), "两种记录并一问,不带判官位:{}", reg_queries[0]);
        let got: Vec<&str> = logs.statuses.iter().map(|s| s.emitter.as_str()).collect();
        assert_eq!(got, three.iter().map(String::as_str).collect::<Vec<_>>(), "只收问到的那几位");
        let forty: Vec<String> = (0..40).map(addr).collect();
        let (logs, asked) = over(&forty);
        assert_eq!((asked.len(), logs.statuses.len()), (2, 40), "四十位仍是一问");
    }
}
