use eravon_galeed::Value;
use eravon_literals::json::Out;

use crate::answer::ok;
use crate::fault::{Fault, R};
use crate::home::Ctx;
use crate::shared::{self, Flags};

const NAMES: [&str; 32] = [
    "amount", "attempts", "blockNumber", "buyer", "caseId", "cases", "consents", "digest", "document", "documents", "emitter", "fund", "hash",
    "instance", "judge", "key", "keys", "logIndex", "name", "position", "poster", "postings", "records", "revoked", "seller", "sellerKey",
    "source", "standing", "termsHash", "token", "who", "",
];

fn name(k: &str) -> &'static str {
    NAMES.iter().copied().find(|n| *n == k).unwrap_or("")
}

fn out(v: &Value) -> Out {
    match v {
        Value::Int(n) => Out::N(*n),
        Value::Str(s) => Out::S(s.clone()),
        Value::Bool(b) => Out::B(*b),
        Value::Arr(a) => Out::A(a.iter().map(out).collect()),
        Value::Obj(m) => Out::O(m.iter().map(|(k, x)| (name(k), out(x))).collect()),
        _ => Out::Null,
    }
}

pub fn answer(rs: &eravon_link::identity::Records, h: &eravon_link::identity::History) -> Out {
    Out::O(eravon_link::identity::history_members(rs, h).iter().map(|(k, v)| (name(k), out(v))).collect())
}

pub fn history(c: &Ctx, f: &Flags) -> R<Out> {
    let me = c.who(f.one("--address"))?;
    let who = match f.opt("--who") {
        Some(x) => eravon_standard::order::addr_norm("--who", &x).map_err(|e| Fault::Arg(e.detail()))?,
        None => me.clone(),
    };
    let w = shared::wire(f, c, Some(&me))?;
    let rs = eravon_scan::records::for_record(&w.t, &w.table, w.chain_id, &w.logic, &who).map_err(|e| Fault::Down { layer: "S8", token: e.name(), why: e.detail() })?;
    let h = eravon_link::identity::history(&rs, &who);
    Ok(ok(vec![("chainId", Out::N(w.chain_id)), ("history", answer(&rs, &h)), ("who", Out::S(who))]))
}
