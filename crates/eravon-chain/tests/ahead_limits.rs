use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::json::In;

#[derive(Clone, Copy, PartialEq)]
enum Limit {
    None,
    ThirdOnce,
    Always,
}

#[derive(Clone)]
struct Two {
    port: u16,
    limit: Limit,
    batches_b: std::sync::Arc<Mutex<usize>>,
    posts: std::sync::Arc<Mutex<Vec<(u16, bool, Vec<String>)>>>,
}

impl Two {
    fn new(port: u16, limit: Limit) -> Two {
        Two { port, limit, batches_b: std::sync::Arc::new(Mutex::new(0)), posts: std::sync::Arc::new(Mutex::new(Vec::new())) }
    }

    fn of(&self, port: u16, method: &str) -> (usize, usize) {
        let p = self.posts.lock().unwrap();
        let batches = p.iter().filter(|(x, b, _)| *x == port && *b).count();
        let singles = p.iter().filter(|(x, b, m)| *x == port && !*b && m.iter().any(|y| y == method)).count();
        (batches, singles)
    }

    fn answer(q: &In) -> String {
        let id = match q.get("id") {
            Some(In::Int(n)) => n.to_string(),
            _ => "0".into(),
        };
        let params = q.get("params").and_then(In::as_arr).map(<[In]>::to_vec).unwrap_or_default();
        let s = |i: usize| params.get(i).and_then(In::as_str).unwrap_or_default().to_ascii_lowercase();
        let result = match q.get("method").and_then(In::as_str).unwrap_or_default() {
            "eth_chainId" => "\"0x1\"".to_string(),
            "eth_blockNumber" => "\"0x1f4\"".to_string(),
            "eth_getCode" => format!("\"0x60{}\"", &s(0)[40..]),
            "eth_getBalance" => format!("\"0x{}\"", &s(0)[38..]),
            "eth_getLogs" => "[]".to_string(),
            _ => "null".to_string(),
        };
        format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
    }
}

impl Transport for Two {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let methods = |qs: &[In]| qs.iter().map(|q| q.get("method").and_then(In::as_str).unwrap_or_default().to_string()).collect::<Vec<_>>();
        let In::Arr(qs) = &v else {
            self.posts.lock().unwrap().push((url.port, false, methods(std::slice::from_ref(&v))));
            return Ok(Answer { status: 200, location: None, body: Two::answer(&v).into_bytes() });
        };
        self.posts.lock().unwrap().push((url.port, true, methods(qs)));
        if url.port == self.port + 1 {
            let k = {
                let mut n = self.batches_b.lock().unwrap();
                *n += 1;
                *n
            };
            let turn = match self.limit {
                Limit::None => false,
                Limit::Always => true,
                Limit::ThirdOnce => k == 3,
            };
            if turn {
                return Ok(Answer { status: 429, location: None, body: b"rate limit exceeded".to_vec() });
            }
        }
        Ok(Answer { status: 200, location: None, body: format!("[{}]", qs.iter().map(Two::answer).collect::<Vec<_>>().join(",")).into_bytes() })
    }
}

fn table(port: u16) -> Table {
    let t = Table::new(vec![format!("http://127.0.0.1:{port}"), format!("http://127.0.0.1:{}", port + 1)]).unwrap();
    let _ = t.chain.set(1);
    t
}

fn addr(i: usize) -> String {
    format!("0x{:040x}", 0xa000 + i)
}

fn sixty(port: u16, limit: Limit) -> (Vec<Vec<u8>>, Two) {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Two::new(port, limit);
    let tb = table(port);
    let _a = eravon_chain::action::open(None);
    let wants: Vec<(String, u64)> = (0..60).map(|i| (addr(i), 0x1f4)).collect();
    eravon_chain::read::ahead_codes_by(&t, &tb, &wants);
    let codes = wants.iter().map(|(a, _)| eravon_chain::read::code(&t, &tb, a, "0x1f4").expect("码").0).collect();
    (codes, t)
}

#[test]
fn a_third_batch_turned_away_once_is_asked_again_and_every_read_is_answered_ahead() {
    let (codes, t) = sixty(31701, Limit::ThirdOnce);
    let (plain, _) = sixty(31711, Limit::None);
    let (batches_b, singles_b) = t.of(31702, "eth_getCode");
    let (batches_a, singles_a) = t.of(31701, "eth_getCode");
    println!("ASKED ahead sixty codes, third batch limited once: B batches={batches_b} B singles={singles_b}; A batches={batches_a} A singles={singles_a}");
    assert_eq!(codes, plain, "答与不限流时逐字相同");
    assert_eq!(singles_b, 0, "限流的那一家其后的读单问数为 0");
    assert_eq!(batches_b, 4, "三批,加被限流那一批再问一次");
    assert_eq!((batches_a, singles_a), (3, 0));
    assert!(eravon_chain::read::unbatched().iter().all(|(said, _)| !said.contains("31702")), "限流不当作不收批量");
}

#[test]
fn a_node_limited_throughout_leaves_the_other_nodes_batches_alone_and_the_answers_the_same() {
    let (codes, t) = sixty(31721, Limit::Always);
    let (plain, _) = sixty(31731, Limit::None);
    let (batches_b, singles_b) = t.of(31722, "eth_getCode");
    let (batches_a, singles_a) = t.of(31721, "eth_getCode");
    let pauses = eravon_chain::patience::pauses(eravon_chain::patience::Class::RateLimited).len();
    println!("ASKED ahead sixty codes, B limited throughout: B batches={batches_b} B singles={singles_b}; A batches={batches_a} A singles={singles_a}");
    assert_eq!(codes, plain, "答与不限流时逐字相同");
    assert_eq!((batches_a, singles_a), (3, 0), "另一家的批照常,其后不再问它");
    let each = (1 + pauses) + 2;
    assert!(batches_b >= each && batches_b <= 3 * each, "每一批至多照一张耐心表重发、再拆一回两半,两半都限流即歇着不再发:{batches_b}");
    assert_eq!(singles_b, 0, "歇着的一家不逐问,答由另一家给");
}

#[test]
fn questions_of_two_methods_go_in_one_batch_per_node() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Two::new(31741, Limit::None);
    let tb = table(31741);
    let _a = eravon_chain::action::open(None);
    let addrs: Vec<String> = (0..10).map(addr).collect();
    let asks: Vec<(&'static str, String)> = addrs
        .iter()
        .flat_map(|a| [(eravon_chain::seam::M_GET_CODE, format!("[\"{a}\",\"0x1f4\"]")), (eravon_chain::seam::M_GET_BALANCE, format!("[\"{a}\",\"0x1f4\"]"))])
        .collect();
    eravon_chain::read::ask_ahead_mixed(&t, &tb, &asks);
    for a in &addrs {
        eravon_chain::read::code(&t, &tb, a, "0x1f4").expect("码");
        eravon_chain::read::balance(&t, &tb, a, "0x1f4").expect("余额");
    }
    let posts = t.posts.lock().unwrap().clone();
    println!("ASKED codes and balances of ten addresses ahead together: posts={}", posts.len());
    assert_eq!(posts.len(), 2, "两家各一批,其后的读不再问");
    assert!(posts.iter().all(|(_, batch, m)| *batch && m.len() == 20 && m.iter().any(|x| x == "eth_getCode") && m.iter().any(|x| x == "eth_getBalance")));
}

#[derive(Default)]
struct Keeper(Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

impl eravon_chain::settled::Keep for Keeper {
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
fn the_same_log_query_read_twice_in_one_action_asks_each_node_once() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Two::new(31751, Limit::None);
    let tb = table(31751);
    let _a = eravon_chain::action::open(Some(std::sync::Arc::new(Keeper::default())));
    let first = eravon_chain::read::logs(&t, &tb, &addr(1), 100, eravon_chain::seam::Upto::Latest).expect("头一遍").0;
    let second = eravon_chain::read::logs(&t, &tb, &addr(1), 100, eravon_chain::seam::Upto::Latest).expect("第二遍").0;
    let asked: usize = t.posts.lock().unwrap().iter().map(|(_, _, m)| m.iter().filter(|x| *x == "eth_getLogs").count()).sum();
    println!("ASKED one log query read twice in one action: eth_getLogs={asked}");
    assert_eq!(first, second);
    assert_eq!(asked, 2, "两家各问一次");
}
