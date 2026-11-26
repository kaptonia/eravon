//! Answer shapes of the twelve harness commands: the result of a law predicate as one JSON value (member
//! order from the core's canonical writer, by UTF-8 bytes).

use crate::{key, BenchThree, Candidates, CardReading, Covenant, CovenantFace, CovenantReading, Doc, Guild, GuildCard, Identity, JudgeDoc, JudgeReading, Logs, PointerResult, QuoteReading, Refusal, ResolvedJudge, Row, Venue};
use galeed::json::Value;

pub const OK: &str = "ok";
pub const TOKEN: &str = "token";
pub const KNOWN: &str = "known";
pub const ROSTER: &str = "roster";
pub const GUILDCARD: &str = "guildcard";
pub const CARD: &str = "card";
pub const CURRENT: &str = "current";
pub const JUDGE: &str = "judge";
pub const GUILDAMANAH: &str = "guildamanah";
pub const UNKNOWN_FIELDS: &str = "unknownFields";
pub const ACTIVE: &str = "active";
pub const CANDIDATE: &str = "candidate";
pub const BLOCK_NUMBER: &str = "blockNumber";
pub const LOG_INDEX: &str = "logIndex";
pub const DECODED: &str = "decoded";
pub const ATTEMPTS: &str = "attempts";
pub const FINAL: &str = "final";
pub const TRAIL: &str = "trail";
pub const LISTED: &str = "listed";
pub const COVENANT: &str = "covenant";
pub const CANDIDATES: &str = "candidates";
pub const PER_POINTER: &str = "perPointer";
pub const POINTER: &str = "pointer";
pub const VERDICT: &str = "verdict";
pub const DOCUMENT: &str = crate::harness::F_DOCUMENT;
pub const WORD: &str = crate::harness::F_WORD;
pub const QUOTE: &str = "quote";
pub const KEY: &str = "key";
/// The second value of §5.5.
pub const UNRESOLVED: &str = "unresolved";

fn obj(m: Vec<(&str, Value)>) -> Value {
    Value::Obj(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}
fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}
fn strs(v: &[String]) -> Value {
    Value::Arr(v.iter().map(|x| s(x)).collect())
}
fn refused(token: &str) -> Vec<(&'static str, Value)> {
    vec![(OK, Value::Bool(false)), (TOKEN, s(token))]
}

/// A member's original value from the document, carried only when present ("exactly where present").
fn carried(m: &mut Vec<(&str, Value)>, from: &Value, k: &'static str) {
    if let Some(v) = from.get(k) {
        m.push((k, v.clone()));
    }
}

fn roster_face(r: &crate::Roster) -> Value {
    let mut m = vec![
        (key::GUILD, s(&r.guild)),
        (key::CHAIN_ID, Value::Int(r.chain_id)),
        (key::ISSUED, Value::Int(r.issued)),
        (key::JUDGES, strs(&r.judges)),
        (UNKNOWN_FIELDS, strs(&r.unknown_fields)),
    ];
    if let Some(b) = r.bench_view() {
        let verifiers = b
            .verifiers
            .iter()
            .map(|v| match &v.named {
                None => obj(vec![(key::ADDR, s(&v.addr))]),
                Some((name, protocols)) => obj(vec![(key::ADDR, s(&v.addr)), (key::NAME, s(name)), (key::PROTOCOLS, strs(protocols))]),
            })
            .collect();
        m.push((key::BENCH, obj(vec![(key::PROTOCOLS, strs(&b.protocols)), (key::VERIFIERS, Value::Arr(verifiers))])));
    }
    obj(m)
}

/// The judge object `decode` shows (§3a): `amanah` with its one member, `ledger` with its two, the basis as read.
pub fn judge_face(d: &JudgeDoc) -> Value {
    let mut m = vec![
        (key::JUDGE, s(&d.judge)),
        (key::ISSUED, Value::Int(d.issued)),
        (key::AMANAH, obj(vec![(key::ENTRY_ID, s(&d.entry_id))])),
        (key::LEDGER, obj(vec![(key::BASIS, d.basis.clone()), (key::ROOT, s(&d.root))])),
        (UNKNOWN_FIELDS, strs(&d.unknown_fields)),
    ];
    carried(&mut m, &d.raw, key::NAME);
    carried(&mut m, &d.raw, key::HOME);
    carried(&mut m, &d.raw, key::NOTE_MD);
    obj(m)
}

/// The guild card object `decode` shows (§3c): `guild`, `issued`, `unknownFields`, and the three optional members
/// exactly where the document carries them.
pub fn card_face(c: &GuildCard) -> Value {
    let mut m = vec![(key::GUILD, s(&c.guild)), (key::ISSUED, Value::Int(c.issued)), (UNKNOWN_FIELDS, strs(&c.unknown_fields))];
    carried(&mut m, &c.raw, key::NAME);
    carried(&mut m, &c.raw, key::HOME);
    carried(&mut m, &c.raw, key::NOTE_MD);
    obj(m)
}

fn venues_face(v: &[Venue]) -> Value {
    Value::Arr(v.iter().map(|e| obj(vec![(key::CHAIN_ID, Value::Int(e.chain_id)), (key::CODE_HASH, s(&e.code_hash)), (key::CONTRACT, s(&e.contract))])).collect())
}

fn covenant_decoded(c: &Covenant) -> Value {
    let mut m = vec![
        (key::GUILD, s(&c.guild)),
        (key::CHAIN_ID, Value::Int(c.chain_id)),
        (key::ISSUED, Value::Int(c.issued)),
        (key::VERSION, Value::Int(c.version)),
        (key::TEXT_MD, s(&c.text_md)),
        (key::SHA256, s(&c.sha256)),
        (UNKNOWN_FIELDS, strs(&c.unknown_fields)),
    ];
    if let Some(v) = &c.venues {
        m.push((key::VENUES, venues_face(v)));
    }
    if let Some(q) = &c.quotes {
        m.push((key::QUOTES, q.clone()));
    }
    obj(m)
}

pub fn decode(r: &Result<Doc, crate::DecodeError>) -> Value {
    let t = Value::Bool(true);
    match r {
        Err(e) => obj(refused(e.name())),
        Ok(Doc::Unknown(k)) => obj(vec![(key::KIND, s(k)), (KNOWN, Value::Bool(false)), (OK, t)]),
        Ok(Doc::Roster(r)) => obj(vec![(key::KIND, s(crate::KIND_ROSTER)), (KNOWN, t.clone()), (OK, t), (ROSTER, roster_face(r))]),
        Ok(Doc::Judge(d)) => obj(vec![(JUDGE, judge_face(d)), (key::KIND, s(crate::KIND_JUDGE)), (KNOWN, t.clone()), (OK, t)]),
        Ok(Doc::Covenant(c)) => obj(vec![(GUILDAMANAH, covenant_decoded(c)), (key::KIND, s(crate::KIND_GUILDAMANAH)), (KNOWN, t.clone()), (OK, t)]),
        Ok(Doc::GuildCard(c)) => obj(vec![(GUILDCARD, card_face(c)), (key::KIND, s(crate::KIND_GUILDCARD)), (KNOWN, t.clone()), (OK, t)]),
    }
}

/// `gro publication`: `candidateOf(address, kind)` and `active` over the address itself.
pub fn publication(logs: &Logs, addr: &str, k: crate::Kind) -> Value {
    let candidate = logs.candidate_of(addr, k).map_or(Value::Null, |p| obj(vec![(BLOCK_NUMBER, Value::Int(p.block_number)), (DECODED, decode(&crate::decode(p))), (LOG_INDEX, Value::Int(p.log_index))]));
    obj(vec![(ACTIVE, Value::Bool(logs.active(addr))), (CANDIDATE, candidate)])
}

/// `gro resolve-identity`: §5.3, always resolved.
pub fn identity(id: &Identity) -> Value {
    obj(vec![
        (ATTEMPTS, Value::Arr(id.attempts.iter().map(|x| obj(vec![(BLOCK_NUMBER, Value::Int(x.block_number)), (LOG_INDEX, Value::Int(x.log_index)), (key::TO, s(&x.to))])).collect())),
        (FINAL, s(&id.final_addr)),
        (OK, Value::Bool(true)),
        (TRAIL, strs(&id.trail)),
    ])
}

/// `gro current-at`: §5.2.
pub fn current(addr: &str) -> Value {
    obj(vec![(CURRENT, s(addr))])
}

/// One row: `listed`, `final` and `trail`; verifier objects also carry `name` and `protocols`.
fn row(r: &Row) -> Value {
    let mut m = vec![(LISTED, s(&r.listed)), (FINAL, s(&r.walk.final_addr)), (TRAIL, strs(&r.walk.trail))];
    if let Some((name, protocols)) = &r.named {
        m.extend([(key::NAME, s(name)), (key::PROTOCOLS, strs(protocols))]);
    }
    obj(m)
}

/// The `covenant` object of `gro covenant`: `blockNumber`, `sha256`, `version`, `word`, and `venues`, `quotes`
/// exactly where the document carries them.
fn covenant_face(c: &CovenantFace) -> Value {
    let mut m = vec![(BLOCK_NUMBER, Value::Int(c.block_number)), (key::SHA256, s(&c.sha256)), (key::VERSION, Value::Int(c.version)), (WORD, s(&c.word))];
    if let Some(v) = &c.venues {
        m.push((key::VENUES, venues_face(v)));
    }
    if let Some(q) = &c.quotes {
        m.push((key::QUOTES, q.clone()));
    }
    obj(m)
}

/// `resolve-guild`'s `covenant` member: `null`, the stop, or the covenant object. The one face of a guild's covenant
/// reading: every answer that carries one spells it here.
pub fn guild_covenant(r: &CovenantReading) -> Value {
    match r {
        CovenantReading::None => Value::Null,
        CovenantReading::Stop(t) => obj(refused(t)),
        CovenantReading::Covenant(c) => covenant_face(c),
    }
}

fn guild_members(r: &Result<Guild, Refusal>) -> Vec<(&'static str, Value)> {
    match r {
        Ok(g) => vec![
            (key::BENCH, strs(&g.bench)),
            (CARD, card(&g.card)),
            (COVENANT, guild_covenant(&g.covenant)),
            (key::GUILD, s(&g.guild)),
            (key::ISSUED, Value::Int(g.issued)),
            (key::JUDGES, Value::Arr(g.judges.iter().map(row).collect())),
            (OK, Value::Bool(true)),
            (key::PROTOCOLS, strs(&g.protocols)),
            (TRAIL, strs(&g.trail)),
            (UNKNOWN_FIELDS, strs(&g.roster.unknown_fields)),
            (key::VERIFIERS, Value::Arr(g.verifiers.iter().map(row).collect())),
        ],
        Err(e) => refused(e.name()),
    }
}

pub fn guild(r: &Result<Guild, Refusal>) -> Value {
    obj(guild_members(r))
}

/// `gro judge`: the four §5.7 readings.
pub fn judge(r: &JudgeReading) -> Value {
    match r {
        JudgeReading::None | JudgeReading::Mismatch => obj(vec![(VERDICT, s(r.name()))]),
        JudgeReading::Unread(t) => obj(vec![(TOKEN, s(t)), (VERDICT, s(r.name()))]),
        JudgeReading::Judge { word, block_number, document } => obj(vec![(BLOCK_NUMBER, Value::Int(*block_number)), (DOCUMENT, judge_face(document)), (VERDICT, s(r.name())), (WORD, s(word))]),
    }
}

/// `gro guild-card`: the four §5.11 readings.
pub fn card(r: &CardReading) -> Value {
    match r {
        CardReading::None | CardReading::Mismatch => obj(vec![(VERDICT, s(r.name()))]),
        CardReading::Unread(t) => obj(vec![(TOKEN, s(t)), (VERDICT, s(r.name()))]),
        CardReading::Card { word, block_number, document } => obj(vec![(BLOCK_NUMBER, Value::Int(*block_number)), (DOCUMENT, card_face(document)), (VERDICT, s(r.name())), (WORD, s(word))]),
    }
}

/// `gro resolve-judge`: §5.8.
pub fn resolved_judge(j: &ResolvedJudge) -> Value {
    obj(vec![(ACTIVE, Value::Bool(j.active)), (FINAL, s(&j.final_addr)), (JUDGE, judge(&j.judge)), (OK, Value::Bool(true)), (TRAIL, strs(&j.trail))])
}

/// `gro covenant`: §5.9 over the address as the final address.
pub fn covenant(r: &CovenantReading) -> Value {
    match r {
        CovenantReading::None => obj(vec![(COVENANT, Value::Null), (OK, Value::Bool(true))]),
        CovenantReading::Stop(t) => obj(refused(t)),
        CovenantReading::Covenant(c) => obj(vec![(COVENANT, covenant_face(c)), (OK, Value::Bool(true))]),
    }
}

/// `gro quote`: the three §5.10 readings.
pub fn quote(r: &QuoteReading) -> Value {
    match r {
        QuoteReading::Quote { key: k, quote: q } => obj(vec![(KEY, s(k)), (QUOTE, s(q)), (VERDICT, s(r.name()))]),
        QuoteReading::Form { key: k } => obj(vec![(KEY, s(k)), (VERDICT, s(r.name()))]),
        QuoteReading::None => obj(vec![(VERDICT, s(r.name()))]),
    }
}

pub fn bench(b: &BenchThree) -> Value {
    obj(vec![(
        key::BENCH,
        match b {
            BenchThree::Null => Value::Null,
            BenchThree::Unresolved => s(UNRESOLVED),
            BenchThree::Sorted(list) => strs(list),
        },
    )])
}

pub fn candidates(c: &Candidates) -> Value {
    let per = c
        .per_pointer
        .iter()
        .map(|(p, r)| {
            let mut m = match r {
                PointerResult::NotPointer => refused(PointerResult::E_POINTER),
                PointerResult::OtherChain => refused(PointerResult::E_POINTER_CHAIN),
                PointerResult::Guild(g) => guild_members(g),
            };
            m.push((POINTER, s(p)));
            obj(m)
        })
        .collect();
    obj(vec![(CANDIDATES, strs(&c.list)), (PER_POINTER, Value::Arr(per))])
}
