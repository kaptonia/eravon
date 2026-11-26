use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_galeed as k1;
use eravon_link as k3;
use eravon_literals::json::Out;
use eravon_literals::literals as lit;
use eravon_recheck::read::{self, Event};
use eravon_scan as a3;
use eravon_standard as a1;
use eravon_terms as k2;

use crate::docs::Docs;
use crate::seam;
use crate::{Why, why};

#[derive(Clone, Debug, Default)]
pub struct Ask {
    pub serial: u64,
    pub opened: u64,
}

#[derive(Clone, Debug)]
pub struct Assembled {
    pub index: Vec<u8>,
    pub files: Vec<(String, Vec<u8>)>,
    pub instance: String,
    pub case_id: String,
    pub rows: usize,
    pub terminal: Option<String>,
    pub compliance: bool,
    pub entry: bool,
    pub kinds: Vec<(&'static str, usize)>,
    pub posted: (String, String),
    pub attachments: usize,
}

fn up<E: std::fmt::Debug>(name: &str) -> impl Fn(E) -> Why + '_ {
    move |e| why(name, format!("{e:?}"))
}

fn r1w(e: eravon_recheck::fault::Fault) -> Why {
    why(&e.name(), e.detail())
}

fn hex(b: &[u8]) -> String {
    a1::order::hex_of(b)
}

pub fn assemble<T: Transport>(t: &T, table: &Table, d: &Docs, ask: &Ask) -> Result<Assembled, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let listing = k3::listing::listing(&d.listing).map_err(|f| why("E_S7_LISTING", format!("{:?}", f.to_value(true))))?;
    let receipt_bytes = match &d.receipt {
        Some(b) => b.clone(),
        None => rebuilt(t, table, &listing, d.at.as_ref())?,
    };
    let receipt = k3::receipt::receipt_under(&listing.logic, &receipt_bytes).map_err(|f| why("E_S7_RECEIPT", format!("{:?}", f.to_value(true))))?;
    let receipt_bytes = receipt.current_form();
    let bound = k3::receipt::bind_receipt(&listing, &receipt).map_err(|v| why("E_S7_BIND", format!("{:?}", v.to_answer())))?;
    let instance = bound.instance.to_ascii_lowercase();
    let case_id = hex(&bound.salt);
    let logic = listing.logic.to_ascii_lowercase();
    let c = receipt.candidate as usize;
    let offer = listing.candidate_offer(c);

    let posted = posted_of(t, table, &listing, &d.listing, d.posted.as_ref())?;
    let evs = read::instance_events(t, table, &instance, eravon_chain::read::instance_from(ask.opened)).map_err(r1w)?;

    let mut seat_no = 0u64;
    let mut judge = if c < listing.judges.len() { listing.judges[c].judge.to_ascii_lowercase() } else { String::new() };
    for e in evs.iter().filter(|e| e.ev.name == "Seated") {
        let id = e.ev.at(1).and_then(|v| v.word()).map_err(up("E_S7_FORM"))?;
        let n = e.ev.at(2).and_then(|v| v.small()).map_err(up("E_S7_FORM"))?;
        if hex(&id).eq_ignore_ascii_case(&case_id) && (n >= seat_no || judge.is_empty()) {
            seat_no = n;
            judge = hex(&e.ev.at(0).and_then(|v| v.addr()).map_err(up("E_S7_FORM"))?);
        }
    }
    let book = eravon_scan::succession::logs(t, table).map_err(|e| why(&e.name(), e.detail()))?;
    let entry = entry_of(&evs, &book, &judge);

    let terminal = terminal_of(&evs)?;
    let case = eravon_recheck::dossier::Case {
        chain_id: listing.chain_id,
        posted: eravon_dossier::index::Posted { tx: posted.0.clone(), hash: posted.1.clone() },
        listing_bytes: d.listing.clone(),
        instance: instance.clone(),
        case_id: case_id.clone(),
        logic: logic.clone(),
        proxy_code_hash: hex(&k2::derive::proxy_code_hash(&logic)),
        seat: eravon_dossier::index::Seat {
            judge: (!judge.is_empty()).then(|| judge.clone()),
            amanah_hash: hex(&offer.amanah_hash),
            quote_hash: hex(&offer.quote_hash),
            seat: seat_no,
            entry_id: entry.as_ref().map(|(w, _)| w.clone()),
        },
        rows: Vec::new(),
        terminal: terminal.as_ref().map(|(name, amounts)| eravon_recheck::dossier::Terminal { name: name.clone(), amounts: amounts.clone() }),
        buyer_consent: bound.form,
        seller_consent: if listing.by_call.iter().any(|(x, _)| *x == c) { k2::sig::Form::Call } else { k2::sig::Form::Secp256k1 },
        entry: entry.as_ref().and_then(|(_, b)| k1::entry::accept(b).ok().map(|e| eravon_recheck::dossier::Entry { bytes: b.clone(), value: e.value })),
        listing: listing.clone(),
        receipt: receipt.clone(),
        bound: bound.clone(),
    };
    let rows = rows(t, table, &case, &evs, eravon_chain::read::instance_from(ask.opened))?;
    let compliance = compliance_of(&listing.terms, &evs, &instance, ask.serial)?;

    let mut files: Vec<(String, Vec<u8>)> =
        vec![(seam::FILE_LISTING.to_string(), d.listing.clone()), (seam::FILE_RECEIPT.to_string(), receipt_bytes.clone())];
    let fref = |name: &str, b: &[u8]| Out::O(vec![("name", Out::S(name.to_string())), ("sha256", Out::S(k1::hash::hex_encode(&k1::hash::sha256(b))))]);
    let mut fobj = vec![("listing", fref(seam::FILE_LISTING, &d.listing)), ("receipt", fref(seam::FILE_RECEIPT, &receipt_bytes))];
    let mut seat = vec![("amanahHash", Out::S(hex(&offer.amanah_hash)))];
    if !judge.is_empty() {
        seat.push(("judge", Out::S(judge.clone())));
    }
    seat.push(("quoteHash", Out::S(hex(&offer.quote_hash))));
    seat.push(("seat", Out::N(seat_no)));
    if let Some((word, bytes)) = &entry {
        fobj.push(("entry", fref(seam::FILE_ENTRY, bytes)));
        seat.push(("entryId", Out::S(word.clone())));
        files.push((seam::FILE_ENTRY.to_string(), bytes.clone()));
    }
    if !d.attachments.is_empty() {
        let mut refs = Vec::with_capacity(d.attachments.len());
        for (i, text) in d.attachments.iter().enumerate() {
            let name = seam::attachment_name(i);
            refs.push(fref(&name, text));
            files.push((name, text.clone()));
        }
        fobj.insert(0, ("attachments", Out::A(refs)));
    }
    let mut idx = vec![
        ("caseId", Out::S(case_id.clone())),
        ("chainId", Out::N(listing.chain_id)),
        ("files", Out::O(fobj)),
        ("instance", Out::S(instance.clone())),
        ("logic", Out::S(logic.clone())),
        ("posted", Out::O(vec![("hash", Out::S(posted.1.clone())), ("tx", Out::S(posted.0.clone()))])),
        ("proxyCodeHash", Out::S(hex(&k2::derive::proxy_code_hash(&logic)))),
        ("seat", Out::O(seat)),
        (
            "transactions",
            Out::A(
                rows.iter()
                    .map(|r| Out::O(vec![("block", Out::N(r.2)), ("kind", Out::S(r.0.clone())), ("timestamp", Out::N(r.3)), ("tx", Out::S(r.1.clone()))]))
                    .collect(),
            ),
        ),
        ("v", Out::S(lit::DOC_DOSSIER.to_string())),
    ];
    if let Some((name, m)) = &terminal {
        idx.push((
            "terminal",
            Out::O(vec![
                ("bondToBuyer", Out::S(m[3].clone())),
                ("buyerPaid", Out::S(m[0].clone())),
                ("judgePaid", Out::S(m[2].clone())),
                ("name", Out::S(name.clone())),
                ("sellerPaid", Out::S(m[1].clone())),
            ]),
        ));
    }
    let has_compliance = compliance.is_some();
    if let Some(cmp) = compliance {
        idx.push(("compliance", cmp));
    }
    Ok(Assembled {
        index: eravon_literals::json::write(&Out::O(idx)),
        files,
        instance,
        case_id,
        rows: rows.len(),
        terminal: terminal.map(|x| x.0),
        compliance: has_compliance,
        entry: entry.is_some(),
        kinds: lit::TRANSACTION_KINDS.iter().map(|k| (*k, rows.iter().filter(|r| r.0 == *k).count())).filter(|(_, n)| *n > 0).collect(),
        posted,
        attachments: d.attachments.len(),
    })
}

fn rebuilt<T: Transport>(t: &T, table: &Table, listing: &k3::listing::Listing, at: Option<&(String, String)>) -> Result<Vec<u8>, Why> {
    let not_whole = |why_: String| why(seam::R_NOT_WHOLE, why_);
    let Some((instance, case_id)) = at else { return Err(not_whole("本机没有回执,订单记录也没记实例与案号:从链上补不出".into())) };
    let evs = read::instance_events(t, table, instance, 0).map_err(r1w)?;
    let Some(opened) = evs.iter().find(|e| e.ev.name == "Opened") else {
        return Err(not_whole(format!("本机没有回执,实例 {instance} 上也没有开案(Opened):回执只有买家手里有")));
    };
    let r = match a3::scan::rebuild(t, table, &opened.log.tx, listing.chain_id, &listing.logic, case_id).map_err(|e| why(&e.name(), e.detail()))? {
        a3::scan::Rebuild::Done(r) => r,
        a3::scan::Rebuild::Untraced => return Err(not_whole(format!("本机没有回执;开案交易 {} 的 init 在内层调用,端点追踪不了,补不出", opened.log.tx))),
    };
    let judges: Vec<String> = listing.judges.iter().map(|j| j.judge.clone()).collect();
    let guilds: Vec<String> = listing.guilds.iter().map(|g| g.address.clone()).collect();
    let seq = evs.iter().filter(|e| e.ev.name == "PanelFrozen").last().map(|e| e.ev.at(2).and_then(|v| v.small())).transpose().map_err(up("E_S7_FORM"))?;
    a3::scan::backfill(&r, listing.chain_id, &judges, &guilds, seq).map_err(|e| why(&e.name(), e.detail()))
}

fn posted_of<T: Transport>(t: &T, table: &Table, listing: &k3::listing::Listing, bytes: &[u8], given: Option<&(String, String)>) -> Result<(String, String), Why> {
    let key = a1::order::b32_of("termsHash", &listing.terms_hash).map_err(|e| why(&e.name(), e.detail()))?;
    let hash = k1::hash::sha256(bytes);
    let posts = match given {
        Some((tx, _)) => a3::board::by_tx(t, table, tx, &hash).map_err(|e| why(&e.name(), e.detail()))?,
        None => a3::board::by_key(t, table, &key, 0, eravon_chain::seam::Upto::Latest).map_err(|e| why(&e.name(), e.detail()))?,
    };
    posts
        .into_iter()
        .filter(|p| p.key == key && p.hash == hash && p.document == bytes)
        .last()
        .map(|p| (p.tx.to_ascii_lowercase(), hex(&p.hash)))
        .ok_or_else(|| why(seam::R_NOT_WHOLE, format!("公告板上没有这份清单在其 termsHash {} 下的上板记录:卷宗缺 posted", listing.terms_hash)))
}

type Row = (String, String, u64, u64);

fn topic(a: &str) -> Result<String, Why> {
    a3::log::address_topic(a).map_err(|e| why(&e.name(), e.detail()))
}

fn rows<T: Transport>(t: &T, table: &Table, c: &eravon_recheck::dossier::Case, evs: &[Event], events_from: u64) -> Result<Vec<Row>, Why> {
    let mut more: Vec<String> = Vec::new();
    let logic = c.logic.to_ascii_lowercase();
    let seller = c.listing.order.seller_key.to_ascii_lowercase();
    let opened = evs.iter().find(|e| e.ev.name == "Opened");
    let from = opened.map_or(0, |e| e.log.block);
    let hist = a3::scan::history(t, table, c.listing.chain_id, &logic, &c.instance, eravon_chain::read::instance_from(from), eravon_chain::seam::Upto::Latest).map_err(|e| why(&e.name(), e.detail()))?;
    more.extend(hist.iter().filter(|m| m.name == "Consented").map(|m| m.tx.clone()));
    let (rev, bound) = (hex(&a1::sol::REVOKED_EV.topic0()), hex(&a1::sol::FUND_BOUND_EV.topic0()));
    let (terms, fund) = (c.listing.terms_hash.to_ascii_lowercase(), topic(&c.listing.order.seller_fund)?);
    let keys = [seller.clone(), c.receipt.buyer_key.clone()];
    let records = a3::scan::logic_records(t, table, &logic, &keys, eravon_chain::seam::Upto::Latest).map_err(|e| why(&e.name(), e.detail()))?;
    let mut both = a3::scan::picked(&records, &a1::sol::REVOKED_EV.topic0(), &[Some(topic(&seller)?), Some(terms.clone())]);
    both.extend(a3::scan::picked(&records, &a1::sol::FUND_BOUND_EV.topic0(), &[Some(topic(&seller)?), Some(fund.clone())]));
    both.sort_by(|a, b| a.block.cmp(&b.block).then(a.log_index.cmp(&b.log_index)));
    for l in both {
        let (t0, t2) = (l.topics.first().map(|x| hex(x)), l.topics.get(2).map(|x| hex(x)));
        if t0.as_deref() == Some(rev.as_str()) && t2.as_deref() == Some(terms.as_str()) {
            more.push(l.tx.clone());
        } else if t0.as_deref() == Some(bound.as_str()) && t2.as_deref() == Some(fund.as_str()) && opened.is_none_or(|o| (l.block, l.log_index) < (o.log.block, o.log.log_index)) {
            more.push(l.tx.clone());
        }
    }
    if let Some((tx, _)) = deploy_of(t, table, c, opened.map(|e| e.log.block))? {
        more.push(tx);
    }
    let rows = eravon_recheck::check::rows_of(t, table, c, &more, events_from).map_err(r1w)?;
    Ok(written(rows.into_iter().map(|r| (r.kind, r.tx, r.block, r.timestamp)).collect()))
}

fn written(rows: Vec<Row>) -> Vec<Row> {
    rows
}


fn deploy_of<T: Transport>(t: &T, table: &Table, c: &eravon_recheck::dossier::Case, opened: Option<u64>) -> Result<Option<(String, u64)>, Why> {
    let code = |b: u64| read::code_at(t, table, &c.instance, Some(b)).map_err(r1w);
    let head = match opened {
        Some(b) => b,
        None => eravon_chain::read::height(t, table).map_err(|e| why(&e.name(), e.detail()))?.0,
    };
    if code(head)?.is_empty() {
        return Ok(None);
    }
    let start = read::tx(t, table, &c.posted.tx).map_err(r1w)?.map_or(0, |x| x.block);
    let (mut lo, mut hi) = (start, head);
    if !code(lo)?.is_empty() {
        hi = lo;
    }
    let ahead = eravon_chain::read::holds_ahead(table);
    while lo + 1 < hi {
        let mids: Vec<u64> = if ahead { probes(lo, hi) } else { vec![lo + (hi - lo) / 2] };
        eravon_chain::read::ahead_codes_by(t, table, &mids.iter().map(|m| (c.instance.clone(), *m)).collect::<Vec<_>>());
        for m in mids {
            if code(m)?.is_empty() {
                lo = m;
            } else {
                hi = m;
                break;
            }
        }
    }
    for (hash, input) in block_txs(t, table, hi)? {
        if eravon_recheck::check::carries_creation(c, &input) {
            return Ok(Some((hash, hi)));
        }
    }
    Ok(None)
}

fn block_txs<T: Transport>(t: &T, table: &Table, block: u64) -> Result<Vec<(String, Vec<u8>)>, Why> {
    let (hashes, raws) = eravon_scan::evidence::block_txs_at(t, table, block).map_err(|f| why(&f.name(), f.detail()))?;
    Ok(hashes.into_iter().zip(&raws).filter_map(|(h, raw)| eravon_scan::kit::input_of(raw).map(|i| (h, i))).collect())
}

fn probes(lo: u64, hi: u64) -> Vec<u64> {
    let gap = hi - lo;
    let k = DEPLOY_PROBES.min(gap - 1);
    let mut out: Vec<u64> = (1..=k).map(|i| lo + gap * i / (k + 1)).filter(|m| *m > lo && *m < hi).collect();
    out.dedup();
    out
}

const DEPLOY_PROBES: u64 = 15;

fn entry_of(evs: &[Event], book: &eravon_galeed::book::Logs, judge: &str) -> Option<(String, Vec<u8>)> {
    evs.iter()
        .filter(|e| e.ev.name == "Anchored" && e.log.topic_address(1).is_ok_and(|s| s.eq_ignore_ascii_case(&eravon_scan::succession::current_at(book, judge, (e.log.block, e.log.log_index)))))
        .filter_map(|e| {
            let word = *e.log.topics.get(2)?;
            carried(&e.log.data, &word).map(|b| (hex(&word), b))
        })
        .last()
}

fn carried(data: &[u8], word: &[u8; 32]) -> Option<Vec<u8>> {
    k1::carried::anchor_bytes(data, word)
}


fn terminal_of(evs: &[Event]) -> Result<Option<(String, [String; 4])>, Why> {
    let Some(e) = evs.iter().find(|e| e.ev.name == "Closed") else { return Ok(None) };
    let b = e.ev.at(0).and_then(|v| v.small()).map_err(up("E_S7_FORM"))?;
    let name = usize::try_from(b).ok().and_then(|i| lit::TERMINAL_NAMES.get(i)).ok_or_else(|| why("E_S7_FORM", format!("Closed 的终端字节 {b} 不在六个终端名之内")))?;
    let mut m: [String; 4] = Default::default();
    for (i, slot) in m.iter_mut().enumerate() {
        *slot = k2::u256::U256::from_be(&e.ev.at(i + 1).and_then(|v| v.word()).map_err(up("E_S7_FORM"))?).to_dec();
    }
    Ok(Some((name.to_string(), m)))
}

fn compliance_of(doc: &k2::doc::Terms, evs: &[Event], instance: &str, serial: u64) -> Result<Option<Out>, Why> {
    let Some(cmp) = doc.block.get("compliance_md") else { return Ok(None) };
    let mut m: Vec<(&'static str, Out)> = Vec::new();
    for (key, out) in [("buyer_md", "buyer_md"), ("seller_md", "seller_md")] {
        if let Some(s) = cmp.get(key).and_then(k1::Value::as_str) {
            m.push((out, Out::S(s.to_string())));
        }
    }
    let mut conv = Vec::new();
    for e in evs.iter().filter(|e| e.ev.name == "Priced") {
        let unit = hex(&e.ev.at(0).and_then(|v| v.addr()).map_err(up("E_S7_FORM"))?);
        let num = |i: usize| -> Result<String, Why> { Ok(k2::u256::U256::from_be(&e.ev.at(i).and_then(|v| v.word()).map_err(up("E_S7_FORM"))?).to_dec()) };
        let at = e.ev.at(5).and_then(|v| v.small()).map_err(up("E_S7_FORM"))?;
        conv.push(Out::O(vec![
            ("at", Out::N(at)),
            ("rate", Out::S(format!("{}/{}", num(3)?, num(4)?))),
            ("source", Out::S(format!("Priced@{instance}#{}", e.log.block))),
            ("tx", Out::S(e.log.tx.to_ascii_lowercase())),
            ("unit", Out::S(unit)),
        ]));
    }
    m.push(("serial", Out::S(serial.to_string())));
    m.push(("conversions", Out::A(conv)));
    Ok(Some(Out::O(m)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eravon_chain::seam::Url;
    use eravon_chain::transport::Answer;
    use eravon_literals::json::In;

    fn hexs(b: &[u8]) -> String {
        k1::hash::hex_encode(b)
    }

    fn block_of_three() -> (In, Vec<Vec<u8>>, Vec<In>) {
        use k1::rlp::{self, Rlp};
        let raws: Vec<Vec<u8>> = (1u8..=3)
            .map(|i| {
                let (to, input, sig) = ([0xaa; 20], [0xc0, 0xde, i], [i; 32]);
                let n = rlp::uint_bytes(u64::from(i));
                rlp::encode(&Rlp::List(vec![
                    Rlp::Bytes(&n),
                    Rlp::Bytes(&[1]),
                    Rlp::Bytes(&[0x52, 0x08]),
                    Rlp::Bytes(&to),
                    Rlp::Bytes(&[]),
                    Rlp::Bytes(&input),
                    Rlp::Bytes(&[0x1b]),
                    Rlp::Bytes(&sig),
                    Rlp::Bytes(&sig),
                ]))
            })
            .collect();
        let receipts: Vec<In> = (1u64..=3)
            .map(|i| {
                eravon_literals::json::read(format!(r#"{{"type":"0x0","status":"0x1","cumulativeGasUsed":"0x{:x}","logsBloom":"0x{}","logs":[]}}"#, i * 21_000, "00".repeat(256)).as_bytes()).unwrap()
            })
            .collect();
        let rlps: Vec<Vec<u8>> = receipts.iter().map(|r| a3::kit::receipt_rlp(r).unwrap()).collect();
        let f = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-scan/tests/fixtures/anvil-anchor.json")).unwrap();
        let f = eravon_literals::json::read(&f).unwrap();
        let hashes: Vec<In> = raws.iter().map(|r| In::Str(hexs(&k1::hash::keccak256(r)))).collect();
        let mut b = match f.get("block").unwrap().clone() {
            In::Obj(m) => m,
            _ => unreachable!(),
        };
        for (k, v) in b.iter_mut() {
            match k.as_str() {
                "transactionsRoot" => *v = In::Str(hexs(&a3::kit::trie(&raws, 0).0)),
                "receiptsRoot" => *v = In::Str(hexs(&a3::kit::trie(&rlps, 0).0)),
                "transactions" => *v = In::Arr(hashes.clone()),
                _ => {}
            }
        }
        let order: [(&str, bool); 15] = [
            ("parentHash", false), ("sha3Uncles", false), ("miner", false), ("stateRoot", false), ("transactionsRoot", false),
            ("receiptsRoot", false), ("logsBloom", false), ("difficulty", true), ("number", true), ("gasLimit", true),
            ("gasUsed", true), ("timestamp", true), ("extraData", false), ("mixHash", false), ("nonce", false),
        ];
        let later = ["baseFeePerGas", "withdrawalsRoot", "blobGasUsed", "excessBlobGas", "parentBeaconBlockRoot", "requestsHash"];
        let get = |k: &str| b.iter().find(|(x, _)| x == k).and_then(|(_, v)| v.as_str()).map(str::to_string);
        let field = |k: &str, q: bool| -> Option<Vec<u8>> {
            let h = get(k)?;
            let d = h.trim_start_matches("0x");
            let d = if q { d.trim_start_matches('0').to_string() } else { d.to_string() };
            let d = if d.len() % 2 == 1 { format!("0{d}") } else { d };
            k1::hexform::decode_digits(d.as_bytes())
        };
        let mut items: Vec<Vec<u8>> = order.iter().map(|(k, q)| field(k, *q).expect("头成员")).collect();
        for k in later {
            match field(k, !k.ends_with("Root") && !k.ends_with("Hash")) {
                Some(v) => items.push(v),
                None => break,
            }
        }
        let raw = rlp::encode(&Rlp::List(items.iter().map(|v| Rlp::Bytes(v)).collect()));
        let hash = hexs(&k1::hash::keccak256(&raw));
        for (k, v) in b.iter_mut() {
            if k == "hash" {
                *v = In::Str(hash.clone());
            }
        }
        let block = In::Obj(b);
        assert_eq!(a3::kit::header_rlp(&block).expect("造的块合它的哈希"), raw);
        (block, raws, receipts)
    }

    #[derive(Clone)]
    struct Chain {
        block: In,
        raws: Vec<Vec<u8>>,
        receipts: Vec<In>,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(u16, String, usize)>>>,
    }

    fn objects_of_three() -> Vec<In> {
        (1u8..=3)
            .map(|i| {
                let r = format!("0x{}", format!("{i:02x}").repeat(32));
                let text = format!(r#"{{"type":"0x0","nonce":"0x{i:x}","gasPrice":"0x1","gas":"0x5208","to":"0x{}","value":"0x0","input":"0xc0de{i:02x}","v":"0x1b","r":"{r}","s":"{r}"}}"#, "aa".repeat(20));
                eravon_literals::json::read(text.as_bytes()).unwrap()
            })
            .collect()
    }

    impl Chain {
        fn answer(&self, q: &In) -> String {
            let id = match q.get("id") {
                Some(In::Int(n)) => n.to_string(),
                _ => "0".into(),
            };
            let method = q.get("method").and_then(In::as_str).unwrap_or_default();
            let first = q.get("params").and_then(In::as_arr).and_then(|a| a.first()).and_then(In::as_str).unwrap_or_default().to_ascii_lowercase();
            let w = |v: &In| String::from_utf8(eravon_literals::json::write_in(v)).unwrap();
            let result = match method {
                "eth_chainId" => "\"0x7a69\"".to_string(),
                "eth_blockNumber" => "\"0x10\"".to_string(),
                "eth_getBlockByNumber" => w(&self.block),
                "eth_getBlockByHash" => match &self.block {
                    In::Obj(m) => w(&In::Obj(m.iter().map(|(k, v)| (k.clone(), if k == "transactions" { In::Arr(objects_of_three()) } else { v.clone() })).collect())),
                    other => w(other),
                },
                "eth_getRawTransactionByHash" => {
                    self.raws.iter().find(|r| hexs(&k1::hash::keccak256(r)) == first).map_or("null".into(), |r| format!("\"{}\"", hexs(r)))
                }
                "eth_getBlockReceipts" => w(&In::Arr(self.receipts.clone())),
                _ => "null".into(),
            };
            format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
        }
    }

    impl Transport for Chain {
        fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
            let v = eravon_literals::json::read(body).unwrap();
            let body = match &v {
                In::Arr(qs) => {
                    let m = qs.first().and_then(|q| q.get("method")).and_then(In::as_str).unwrap_or_default().to_string();
                    self.asked.lock().unwrap().push((url.port, m, qs.len()));
                    format!("[{}]", qs.iter().map(|q| self.answer(q)).collect::<Vec<_>>().join(","))
                }
                q => {
                    let m = q.get("method").and_then(In::as_str).unwrap_or_default().to_string();
                    self.asked.lock().unwrap().push((url.port, m, 1));
                    self.answer(q)
                }
            };
            Ok(Answer { status: 200, location: None, body: body.into_bytes() })
        }
    }

    #[test]
    fn a_blocks_transactions_are_read_from_its_lists_once() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let _action = eravon_chain::action::open(None);
        let (block, raws, receipts) = block_of_three();
        let t = Chain { block, raws, receipts, asked: Default::default() };
        let table = Table::new(vec!["http://127.0.0.1:31401".into(), "http://127.0.0.1:31402".into()]).unwrap();
        let count = |m: &str| t.asked.lock().unwrap().iter().filter(|(_, x, _)| x == m).count();
        let ports = |m: &str| t.asked.lock().unwrap().iter().filter(|(_, x, _)| x == m).map(|(p, _, _)| *p).collect::<std::collections::BTreeSet<u16>>();
        for round in 0..2 {
            let got = block_txs(&t, &table, 6).unwrap_or_else(|e| panic!("第 {round} 回:{e:?}"));
            let inputs: Vec<Vec<u8>> = got.iter().map(|(_, i)| i.clone()).collect();
            assert_eq!(inputs, vec![vec![0xc0, 0xde, 1], vec![0xc0, 0xde, 2], vec![0xc0, 0xde, 3]], "第 {round} 回:块内次序的三笔 input");
            assert_eq!((count("eth_getBlockByHash"), ports("eth_getBlockByHash").len()), (1, 1), "第 {round} 回:整块一问、只问一家,第二回不问");
            assert_eq!((count("eth_getRawTransactionByHash"), count("eth_getBlockReceipts"), count("eth_getTransactionReceipt")), (0, 0, 0), "第 {round} 回:不逐笔问原文,不问回执");
            assert_eq!(count("eth_getTransactionByHash"), 0, "第 {round} 回:零逐笔共识读");
        }
    }
}
