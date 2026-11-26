use eravon_literals::json::In;

use crate::endpoints::Table;
use crate::fault::{Fault, R};
use crate::read::Agreement;
use crate::transport::Transport;

pub const SUPPORT_WORDS: [&str; 3] = [WORD_CALL_TRACER, WORD_PARITY, WORD_NONE];
pub const WORD_CALL_TRACER: &str = "callTracer";
pub const WORD_PARITY: &str = "parity";
pub const WORD_NONE: &str = "none";

const M_DEBUG_TRACE: &str = "debug_traceTransaction";
const M_TRACE_TX: &str = "trace_transaction";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub from: String,
    pub input: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Traced {
    Found { call: Call, agreement: Agreement, endpoints: usize },
    NotFound { agreement: Agreement, endpoints: usize },
    Untraced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Support {
    pub endpoint: String,
    pub word: String,
    pub source: &'static str,
}

const TRACES_KEPT: usize = 64;

struct Learnt {
    kinds: Vec<(String, &'static str)>,
    traces: std::collections::VecDeque<(String, String, In)>,
}

static LEARNT: std::sync::Mutex<Learnt> = std::sync::Mutex::new(Learnt { kinds: Vec::new(), traces: std::collections::VecDeque::new() });

fn kind_of(e: &crate::seam::NodeAt) -> Option<&'static str> {
    LEARNT.lock().ok()?.kinds.iter().find(|(p, _)| *p == e.place()).map(|(_, w)| *w)
}

fn learn_kind(e: &crate::seam::NodeAt, w: &'static str) {
    if let Ok(mut g) = LEARNT.lock() {
        g.kinds.retain(|(p, _)| *p != e.place());
        g.kinds.push((e.place(), w));
    }
}

fn kept(e: &crate::seam::NodeAt, tx: &str) -> Option<In> {
    let tx = tx.to_ascii_lowercase();
    LEARNT.lock().ok()?.traces.iter().find(|(p, h, _)| *p == e.place() && *h == tx).map(|(_, _, v)| v.clone())
}

fn keep(e: &crate::seam::NodeAt, tx: &str, v: &In) {
    let tx = tx.to_ascii_lowercase();
    if let Ok(mut g) = LEARNT.lock() {
        if g.traces.iter().any(|(p, h, _)| *p == e.place() && *h == tx) {
            return;
        }
        if g.traces.len() >= TRACES_KEPT {
            g.traces.pop_front();
        }
        g.traces.push_back((e.place(), tx, v.clone()));
    }
}

pub(crate) fn forget() {
    if let Ok(mut g) = LEARNT.lock() {
        g.kinds.clear();
        g.traces.clear();
    }
}

fn traced_by<T: Transport>(t: &T, e: &crate::seam::NodeAt, word: &str, tx: &str) -> R<In> {
    if let Some(v) = kept(e, tx) {
        return Ok(v);
    }
    let v = ask(t, e, word, tx)?;
    keep(e, tx, &v);
    Ok(v)
}

pub fn ahead<T: Transport>(t: &T, table: &Table, txs: &[String]) {
    if txs.len() < 2 || crate::recall::current().is_some() {
        return;
    }
    let nodes = table.nodes();
    let recorded = |e: &crate::seam::NodeAt| -> Option<&'static str> {
        let w = table.trace.iter().find(|(x, _)| crate::seam::NodeAt::new(x.as_str()) == *e).map(|(_, w)| w.as_str())?;
        SUPPORT_WORDS.into_iter().find(|x| *x == w)
    };
    if nodes.iter().any(|e| recorded(e).is_none() && kind_of(e).is_none()) {
        let _ = find(t, table, &txs[0], "0x0000000000000000000000000000000000000000", "0x00000000");
    }
    let known: Vec<(crate::seam::NodeAt, &'static str)> =
        nodes.iter().filter_map(|e| recorded(e).or_else(|| kind_of(e)).filter(|w| *w != WORD_NONE).map(|w| (e.clone(), w))).collect();
    let work: Vec<(crate::seam::NodeAt, &'static str, String)> =
        known.iter().flat_map(|(e, w)| txs.iter().filter(|h| kept(e, h).is_none()).map(move |h| (e.clone(), *w, h.clone()))).collect();
    for part in work.chunks(crate::seam::TRACES_IN_FLIGHT.max(1)) {
        let tt = t.clone();
        let _ = crate::endpoints::until_enough(
            part.to_vec(),
            move |(e, w, h)| {
                if let Ok(v) = ask_once(&tt, &e, w, &h) {
                    keep(&e, &h, &v);
                }
            },
            || (),
            crate::endpoints::every,
        );
    }
}

fn ask_once<T: Transport>(t: &T, e: &crate::seam::NodeAt, word: &str, tx: &str) -> R<In> {
    match word {
        WORD_CALL_TRACER => crate::ask::call_one_raw_once(t, e, M_DEBUG_TRACE, &format!("[\"{tx}\",{{\"tracer\":\"callTracer\"}}]")),
        _ => crate::ask::call_one_raw_once(t, e, M_TRACE_TX, &format!("[\"{tx}\"]")),
    }
}

pub fn find<T: Transport>(
    t: &T,
    table: &Table,
    tx: &str,
    to: &str,
    selector: &str,
) -> R<(Traced, Vec<Support>)> {
    crate::mark::mark(crate::mark::FEATURE);
    let sel = selector.to_ascii_lowercase();
    if sel.len() != 10 || !sel.starts_with("0x") || !sel[2..].bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Usage(format!("选择子要是 0x 加八位十六进制,给的是 {selector:?}")));
    }
    let to = to.to_ascii_lowercase();
    if let Some(kept) = crate::recall::traced(tx, &to, &sel) {
        return kept.map(|x| (x, Vec::new()));
    }
    let mut rows = Vec::new();
    let mut first: Option<(String, Option<Call>)> = None;
    let mut supporting = 0usize;
    let mut keys: Vec<(crate::seam::NodeAt, In)> = Vec::new();
    let nodes = table.nodes();
    let (tt, recorded_of, txs) = (t.clone(), table.trace.clone(), tx.to_string());
    let asked = crate::read::asked_until(
        nodes.clone(),
        move |e| {
            let (t, tx, e) = (&tt, txs.as_str(), &e);
            let recorded = recorded_of.iter().find(|(x, _)| crate::seam::NodeAt::new(x.as_str()) == *e).map(|(_, w)| w.clone());
            let probed = || match probe(t, e, tx) {
                Some((w, v)) => {
                    learn_kind(e, w);
                    keep(e, tx, &v);
                    (w.to_string(), "probed", Some(v))
                }
                None => {
                    learn_kind(e, WORD_NONE);
                    (WORD_NONE.to_string(), "probed", None)
                }
            };
            match recorded.as_deref() {
                Some(WORD_NONE) => Ok((WORD_NONE.to_string(), "recorded", None)),
                Some(w) => traced_by(t, e, w, tx).map(|v| (w.to_string(), "recorded", Some(v))),
                None => Ok(match kind_of(e) {
                    Some(WORD_NONE) => (WORD_NONE.to_string(), "probed", None),
                    Some(w) => traced_by(t, e, w, tx).map_or_else(|_| probed(), |v| (w.to_string(), "probed", Some(v))),
                    None => probed(),
                }),
            }
        },
        crate::read::all,
    )
    .into_iter()
    .map(|g| g.unwrap_or_else(|| Err(Fault::Transport("一处端点的追踪没问完就停了".into()))));
    for ((written, e), got) in table.list.iter().zip(&nodes).zip(asked) {
        let (word, source, answer) = got?;
        rows.push(Support { endpoint: written.clone(), word: word.clone(), source });
        let Some(v) = answer else { continue };
        supporting += 1;
        let found = match word.as_str() {
            WORD_CALL_TRACER => in_tree(&v, &to, &sel),
            _ => in_flat(&v, &to, &sel),
        };
        let key = match &found {
            Some(c) => format!("{}|{}", c.from, c.input),
            None => "none".to_string(),
        };
        keys.push((e.clone(), In::Str(key.clone())));
        if first.is_none() {
            first = Some((key, found));
        }
    }
    if let Some(why) = crate::ask::split(&format!("{tx} 的追踪"), &keys) {
        return Err(Fault::Disagree(why));
    }
    let agreement = if supporting >= 2 { Agreement::Agreed } else { Agreement::Single };
    let out = match first {
        None => Traced::Untraced,
        Some((_, Some(call))) => Traced::Found { call, agreement, endpoints: supporting },
        Some((_, None)) => Traced::NotFound { agreement, endpoints: supporting },
    };
    Ok((out, rows))
}

fn ask<T: Transport>(t: &T, e: &crate::seam::NodeAt, word: &str, tx: &str) -> R<In> {
    match word {
        WORD_CALL_TRACER => crate::ask::call_one_raw(t, e, M_DEBUG_TRACE, &format!("[\"{tx}\",{{\"tracer\":\"callTracer\"}}]")),
        _ => crate::ask::call_one_raw(t, e, M_TRACE_TX, &format!("[\"{tx}\"]")),
    }
}

fn probe<T: Transport>(t: &T, e: &crate::seam::NodeAt, tx: &str) -> Option<(&'static str, In)> {
    if let Ok(v) = ask(t, e, WORD_CALL_TRACER, tx) {
        if v.get("type").is_some() || v.get("calls").is_some() || v.get("to").is_some() {
            return Some((WORD_CALL_TRACER, v));
        }
    }
    if let Ok(v) = ask(t, e, WORD_PARITY, tx) {
        if v.as_arr().is_some() {
            return Some((WORD_PARITY, v));
        }
    }
    None
}

fn low(v: Option<&In>) -> String {
    v.and_then(In::as_str).unwrap_or("").to_ascii_lowercase()
}

fn hit(to: &str, input: &str, want_to: &str, sel: &str) -> bool {
    to == want_to && input.starts_with(sel)
}

fn in_tree(v: &In, to: &str, sel: &str) -> Option<Call> {
    let found = walk(v, to, sel)?;
    Some(caller_of(v, found))
}

fn caller_of(_root: &In, found: Call) -> Call {
    found
}


fn walk(v: &In, to: &str, sel: &str) -> Option<Call> {
    if v.get("error").is_some() {
        return None;
    }
    let (t, i) = (low(v.get("to")), low(v.get("input")));
    if hit(&t, &i, to, sel) {
        return Some(Call { from: low(v.get("from")), input: i });
    }
    for c in v.get("calls").and_then(In::as_arr).unwrap_or(&[]) {
        if let Some(x) = walk(c, to, sel) {
            return Some(x);
        }
    }
    None
}

fn in_flat(v: &In, to: &str, sel: &str) -> Option<Call> {
    let rows = v.as_arr()?;
    let addr = |r: &In| -> Vec<i64> {
        r.get("traceAddress")
            .and_then(In::as_arr)
            .map(|a| a.iter().filter_map(|x| if let In::Int(n) = x { Some(*n) } else { None }).collect())
            .unwrap_or_default()
    };
    let failed: Vec<Vec<i64>> = rows.iter().filter(|r| r.get("error").is_some()).map(addr).collect();
    for r in rows {
        let act = r.get("action")?;
        let (t, i) = if r.get("type").and_then(In::as_str) == Some("create") {
            (low(r.get("result").and_then(|x| x.get("address"))), low(act.get("init")))
        } else {
            (low(act.get("to")), low(act.get("input")))
        };
        if !hit(&t, &i, to, sel) {
            continue;
        }
        let me = addr(r);
        if failed.iter().any(|f| me.starts_with(f)) {
            continue;
        }
        let found = Call { from: low(act.get("from")), input: i };
        let root = rows.first().and_then(|x| x.get("action")).cloned().unwrap_or(In::Null);
        return Some(caller_of_flat(&root, found));
    }
    None
}

fn caller_of_flat(_root: &In, found: Call) -> Call {
    found
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoints::Table;
    use crate::transport::Transport;

    fn rows(s: &str) -> In {
        eravon_literals::json::read(s.as_bytes()).unwrap_or(In::Null)
    }

    #[test]
    fn a_parity_create_frame_is_found_by_its_address_and_init() {
        let v = rows(
            r#"[{"action":{"from":"0xaa","input":"0x01","to":"0xdd"},"traceAddress":[],"type":"call"},
                {"action":{"from":"0xdd","init":"0x3d602d80"},"result":{"address":"0xee"},"traceAddress":[0],"type":"create"}]"#,
        );
        let c = in_flat(&v, "0xee", "0x3d602d80");
        assert_eq!(c.map(|x| x.from), Some("0xdd".to_string()));
    }

    #[derive(Clone)]
    struct Gate {
        traces: u16,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(u16, String)>>>,
    }

    impl Transport for Gate {
        fn post(&self, url: &crate::seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let v = eravon_literals::json::read(body).expect("问是 JSON");
            let method = v.get("method").and_then(In::as_str).unwrap_or_default().to_string();
            let id = match v.get("id") {
                Some(In::Int(n)) => n.to_string(),
                _ => "0".into(),
            };
            self.asked.lock().unwrap().push((url.port, method.clone()));
            let result = match method.as_str() {
                "eth_chainId" => r#""result":"0x7a69""#.to_string(),
                M_DEBUG_TRACE if url.port == self.traces => {
                    let call = |s: &str| format!(r#"{{"from":"0xc0","to":"0xaa","input":"0x{s}00","type":"CALL"}}"#);
                    format!(r#""result":{{"from":"0xe0","to":"0xc0","input":"0x","type":"CALL","calls":[{},{},{}]}}"#, call("11111111"), call("22222222"), call("33333333"))
                }
                _ => r#""error":{"code":-32005,"message":"rate limit exceeded"}"#.to_string(),
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0",{result}}}"#).into_bytes() })
        }
    }

    impl Gate {
        fn count(&self, port: u16, traces: bool) -> usize {
            self.asked.lock().unwrap().iter().filter(|(p, m)| *p == port && (!traces || m == M_DEBUG_TRACE || m == M_TRACE_TX)).count()
        }
    }

    #[test]
    fn a_transaction_is_traced_once_and_a_gateway_that_cannot_is_probed_once() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let t = Gate { traces: 31301, asked: Default::default() };
        let table = Table::new(vec!["http://127.0.0.1:31301".into(), "http://127.0.0.1:31302".into()]).unwrap();
        let tx = format!("0x{}", "ab".repeat(32));
        for (i, sel) in ["0x11111111", "0x22222222", "0x33333333"].into_iter().enumerate() {
            let (got, _) = find(&t, &table, &tx, "0xaa", sel).expect("追踪");
            assert!(matches!(&got, Traced::Found { call, endpoints: 1, .. } if call.input.starts_with(sel)), "第 {i} 个选择子:{got:?}");
        }
        assert_eq!(t.count(31301, true), 1, "能追踪的端点这一笔只追踪一回");
        let probed = t.count(31302, true);
        assert!(probed >= 2, "不能追踪的那家探过两种");
        let tx2 = format!("0x{}", "cd".repeat(32));
        find(&t, &table, &tx2, "0xaa", "0x11111111").expect("下一笔");
        assert_eq!((t.count(31301, true), t.count(31302, true)), (2, probed), "下一笔:能追踪的问一回,不能追踪的不再探");
        forget();
        std::thread::sleep(crate::endpoints::REST_FIRST + std::time::Duration::from_millis(100));
        find(&t, &table, &tx2, "0xaa", "0x22222222").expect("锁后");
        assert!(t.count(31302, true) > probed, "锁后忘了,歇期过了再探一回");
    }

    #[test]
    fn a_failed_parity_create_frame_does_not_count() {
        let v = rows(
            r#"[{"action":{"from":"0xdd","init":"0x3d602d80"},"error":"Reverted","traceAddress":[],"type":"create"}]"#,
        );
        assert_eq!(in_flat(&v, "0xee", "0x3d602d80"), None);
    }
}
