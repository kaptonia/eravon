use eravon_roster::adapter::{self, Logs, Refusal};

use crate::fault::Absent;
use crate::judge;
use crate::quote;
use crate::registry;
use crate::wire::Wire;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub listed: String,
    pub resolved: String,
    pub attempts: Vec<judge::Attempt>,
    pub name: Option<String>,
    pub home: Option<String>,
    pub note_md: Option<String>,
    pub beyond: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terms {
    pub version: Option<u64>,
    pub sha256: String,
    pub text: Result<String, String>,
}

#[derive(Clone, Debug)]
pub struct GuildView {
    pub pointer: String,
    pub address: String,
    pub word: String,
    pub bound: String,
    pub published: Option<u64>,
    pub name: Option<String>,
    pub home: Option<String>,
    pub words: Option<String>,
    pub card: &'static str,
    pub beyond: Vec<(String, String)>,
    pub pool: bool,
    pub members: Vec<Member>,
    pub issued: u64,
    pub terms: Option<Terms>,
    pub quotes: Vec<quote::Row>,
    pub decimals: Vec<(String, Option<u64>)>,
    pub needs_resign: bool,
    pub resign_items: Vec<String>,
}

impl GuildView {
    pub fn card_unread(&self) -> bool {
        self.card == adapter::CARD_UNREAD || self.card == adapter::CARD_MISMATCH
    }

    pub fn selectable(&self) -> bool {
        if self.pool { self.quotes.iter().any(|q| q.usable && q.paced()) } else { !self.members.is_empty() }
    }

    pub fn quote_hashes(&self) -> Vec<(String, String)> {
        self.quotes.iter().filter_map(|q| Some((q.token.to_ascii_lowercase(), q.hash()?))).collect()
    }

    pub fn unpaced(&self) -> bool {
        self.pool && self.quotes.iter().any(|q| q.usable) && !self.selectable()
    }
}

fn card_word(r: &adapter::CardReading) -> (Option<&adapter::GuildCard>, &'static str) {
    match r {
        adapter::CardReading::Card { document, .. } => (Some(document.as_ref()), adapter::CARD),
        adapter::CardReading::None => (None, adapter::CARD_NONE),
        adapter::CardReading::Unread(_) => (None, adapter::CARD_UNREAD),
        adapter::CardReading::Mismatch => (None, adapter::CARD_MISMATCH),
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub address: String,
    pub name: Option<String>,
    pub home: Option<String>,
    pub words: Option<String>,
    pub card: &'static str,
}

impl Card {
    pub fn read(&self) -> bool {
        self.card == adapter::CARD
    }

    pub fn none(&self) -> bool {
        self.card == adapter::CARD_NONE
    }
}

pub fn card_of(logs: &Logs, addr: &str) -> Card {
    let addr = addr.to_ascii_lowercase();
    let (at, reading) = match adapter::guild_of(logs, eravon_roster::pool::standard_chain_id(), &addr) {
        Ok(g) => (g.guild.clone(), g.card),
        Err(_) => (addr.clone(), adapter::guild_card_of(logs, &addr)),
    };
    let (card, word) = card_word(&reading);
    Card { address: at, name: card.and_then(|c| c.name.clone()), home: card.and_then(|c| c.home.clone()), words: card.and_then(|c| c.note_md.clone()), card: word }
}

fn absent_of(r: Refusal) -> Absent {
    match r {
        Refusal::Retired => Absent::Retired,
        Refusal::NoDocument => Absent::NoDocument,
        other => judge::unverified(other.name().to_string()),
    }
}

pub fn guild_due(signed_word: &str, signed_quote: &str, bound: &str, now_quote: Option<&str>) -> bool {
    crate::judge::resign_due(signed_word, bound) || (!signed_quote.is_empty() && now_quote.is_some_and(|q| !q.eq_ignore_ascii_case(signed_quote)))
}

pub fn open(w: &Wire, logs: &Logs, addr: &str) -> Result<GuildView, Absent> {
    crate::mark::mark(crate::mark::FEATURE);
    let chain = eravon_roster::pool::standard_chain_id();
    let g = adapter::guild_of(logs, chain, addr).map_err(absent_of)?;
    let p = adapter::pubs(logs, &g.guild).last().copied().ok_or(Absent::NoDocument)?;
    let r = &g.roster;
    let terms = covenant_terms(&g.covenant);
    let bound = terms.as_ref().map(|t| t.sha256.clone()).unwrap_or_default();
    let covenant = g.current_covenant().map(|c| &c.covenant);
    let (card, card_word) = card_word(&g.card);
    let quotes = quote::rows(covenant.and_then(|c| c.quotes.as_ref()));
    let decimals = judge::decimals_list(w, &quotes);
    Ok(GuildView {
        pointer: crate::table::pointer_of(&g.guild),
        address: g.guild.clone(),
        word: p.word.clone(),
        published: law_block(&g).and_then(|b| registry::stamp(w, b)),
        name: card.and_then(|c| c.name.clone()),
        home: card.and_then(|c| c.home.clone()),
        words: card.and_then(|c| c.note_md.clone()),
        card: card_word,
        beyond: judge::beyond(&r.raw, &r.unknown_fields),
        pool: covenant.is_some(),
        members: g.judges.iter().enumerate().map(|(i, row)| member(logs, row, r.raw.get("judges").and_then(|j| j.as_arr()).and_then(|a| a.get(i)))).collect(),
        issued: r.issued,
        terms,
        quotes,
        decimals,
        needs_resign: false,
        resign_items: Vec::new(),
        bound,
    })
}

pub fn law_block(g: &adapter::Guild) -> Option<u64> {
    g.current_covenant().map(|c| c.block_number)
}

fn member(logs: &Logs, row: &adapter::Row, element: Option<&eravon_galeed::Value>) -> Member {
    let doc = match adapter::judge_of(logs, &row.walk.final_addr) {
        adapter::JudgeReading::Judge { document, .. } => Some(document),
        _ => None,
    };
    let keys: Vec<String> = match element.and_then(|e| e.as_obj()) {
        Some(m) => m.iter().map(|(k, _)| k.clone()).filter(|k| k != "addr").collect(),
        None => Vec::new(),
    };
    Member {
        listed: row.listed.clone(),
        resolved: row.walk.final_addr.clone(),
        attempts: judge::attempts_of(&row.walk),
        name: doc.as_ref().and_then(|d| d.name.clone()),
        home: doc.as_ref().and_then(|d| d.home.clone()),
        note_md: doc.as_ref().and_then(|d| d.note_md.clone()),
        beyond: element.map(|e| judge::beyond(e, &keys)).unwrap_or_default(),
    }
}

pub fn covenant_by_word(logs: &Logs, guild: &str, word: &str) -> Option<Terms> {
    let want = word.trim_start_matches("0x").to_ascii_lowercase();
    adapter::pubs(logs, &guild.to_ascii_lowercase()).into_iter().rev().find_map(|p| match adapter::decode(p) {
        Ok(adapter::Doc::Covenant(c)) if c.sha256.trim_start_matches("0x").eq_ignore_ascii_case(&want) => {
            Some(Terms { version: Some(c.version), sha256: c.sha256.clone(), text: Ok(c.text_md.clone()) })
        }
        _ => None,
    })
}

fn covenant_terms(c: &adapter::CovenantReading) -> Option<Terms> {
    match c {
        adapter::CovenantReading::Covenant(r) => Some(Terms { version: Some(r.covenant.version), sha256: r.covenant.sha256.clone(), text: Ok(r.covenant.text_md.clone()) }),
        adapter::CovenantReading::Stop(e) => Some(Terms { version: None, sha256: String::new(), text: Err(e.to_string()) }),
        adapter::CovenantReading::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eravon_roster::adapter::Publication;

    fn hex(b: &[u8]) -> String {
        format!("0x{}", b.iter().map(|x| format!("{x:02x}")).collect::<String>())
    }

    fn published(emitter: &str, block: u64, doc: String) -> Publication {
        let document = doc.into_bytes();
        Publication { emitter: emitter.to_string(), block_number: block, log_index: 0, word: hex(&eravon_galeed::hash::keccak256(&document)), document }
    }

    #[test]
    fn a_guild_owes_a_re_sign_when_its_word_or_its_quote_for_the_token_moved() {
        assert!(!guild_due("0xw", "0xq", "0xw", Some("0xq")), "both as signed");
        assert!(guild_due("0xw", "0xq", "0xw", Some("0xr")), "the quote alone moved");
        assert!(guild_due("0xw", "0xq", "0xv", Some("0xq")), "the word moved");
        assert!(!guild_due("0xw", "0xq", "0xw", None), "no quote in that token now: the word decides");
    }

    #[test]
    fn three_more_kinds_from_a_guild_change_nothing_its_candidates_read() {
        let g = format!("0x{}", "a1".repeat(20));
        let j1 = format!("0x{}", "f6".repeat(20));
        let chain = eravon_roster::pool::standard_chain_id();
        let roster = format!(r#"{{"chainId":{chain},"guild":"{g}","issued":1790000000,"judges":["{j1}"],"kind":"galeed.roster/1"}}"#);
        let covenant = format!(r#"{{"chainId":{chain},"guild":"{g}","issued":1790000000,"kind":"galeed.guildamanah/1","text_md":"行会约法","version":3}}"#);
        let card = format!(r#"{{"guild":"{g}","issued":1790000000,"kind":"galeed.guildcard/1","name":"一号行会"}}"#);
        let base = vec![published(&g, 9, roster), published(&g, 10, covenant), published(&g, 11, card)];
        let more = [
            published(&g, 20, format!(r#"{{"guild":"{g}","kind":"galeed.split/1","share":1000}}"#)),
            published(&g, 21, format!(r#"{{"client":"0x{}","guild":"{g}","kind":"galeed.client/1"}}"#, "d2".repeat(20))),
            published(&g, 22, format!(r#"{{"dues":5,"guild":"{g}","kind":"galeed.dues/1"}}"#)),
        ];
        let before = Logs { publications: base.clone(), ..Logs::default() };
        let after = Logs { publications: base.into_iter().chain(more).collect(), ..Logs::default() };
        let (a, b) = (adapter::guild_of(&before, chain, &g).expect("before"), adapter::guild_of(&after, chain, &g).expect("after"));
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "the resolved guild reads byte for byte the same");
        assert_eq!(covenant_terms(&a.covenant).map(|t| t.sha256), covenant_terms(&b.covenant).map(|t| t.sha256), "the word a seat binds");
        let law = adapter::candidate_of(&after, &g, adapter::Kind::Guildamanah).map(|p| p.block_number);
        assert_eq!(law, Some(10));
        assert_eq!(law_block(&b), law, "published on: the covenant's own block");
        assert_eq!(adapter::pubs(&after, &g).last().map(|p| p.block_number), Some(22), "the guild's last publication moved");
    }

    #[test]
    fn a_guild_shows_its_own_words_its_members_words_and_the_attempts_verbatim() {
        use eravon_galeed::book::{Accession, Declaration};
        let g = format!("0x{}", "a1".repeat(20));
        let j1 = format!("0x{}", "f6".repeat(20));
        let j2 = format!("0x{}", "c3".repeat(20));
        let j3 = format!("0x{}", "d4".repeat(20));
        let j4 = format!("0x{}", "a7".repeat(20));
        let entry = format!("0x{}", "11".repeat(32));
        let roster = format!(r#"{{"chainId":1,"guild":"{g}","issued":1790000000,"judges":[{{"addr":"{j1}","since":5}},"{j2}"],"kind":"galeed.roster/1","motto":"慢而真"}}"#);
        let card = format!(r#"{{"guild":"{g}","issued":1790000000,"kind":"galeed.guildcard/1","name":"一号行会","note_md":"本会只收**亲审**的判官"}}"#);
        let judge = format!(r#"{{"amanah":{{"entry_id":"{entry}"}},"home":"https://j1.example","issued":1790000000,"judge":"{j1}","kind":"galeed.judge/1","ledger":{{"basis":{{"chains":[]}},"root":"{j1}"}},"name":"一号判官","note_md":"只审 *货不对版*","seal":[1,2]}}"#);
        let logs = Logs {
            publications: vec![published(&g, 9, roster), published(&g, 8, card), published(&j1, 10, judge)],
            statuses: vec![],
            declarations: vec![
                Declaration { emitter: j2.clone(), block_number: 20, log_index: 0, to: j3.clone() },
                Declaration { emitter: j2.clone(), block_number: 21, log_index: 0, to: j4.clone() },
            ],
            accessions: vec![Accession { emitter: j3.clone(), block_number: 20, log_index: 1, holder: j2.clone(), root: j2.clone() }],
        };
        let read = adapter::guild_of(&logs, 1, &g).expect("名录读得出");
        let r = &read.roster;
        let adapter::CardReading::Card { document, .. } = &read.card else { panic!("行会有名片") };
        assert_eq!((document.name.as_deref(), document.note_md.as_deref()), (Some("一号行会"), Some("本会只收**亲审**的判官")), "行会自述照名片原文");
        assert_eq!(judge::beyond(&r.raw, &r.unknown_fields), vec![("motto".to_string(), "\"慢而真\"".to_string())], "名录表外成员照原文");
        let elements = r.raw.get("judges").and_then(|j| j.as_arr()).expect("judges");
        let ms: Vec<Member> = read.judges.iter().enumerate().map(|(i, row)| member(&logs, row, elements.get(i))).collect();
        assert_eq!((ms[0].name.as_deref(), ms[0].home.as_deref(), ms[0].note_md.as_deref()), (Some("一号判官"), Some("https://j1.example"), Some("只审 *货不对版*")));
        assert_eq!(ms[0].beyond, vec![("since".to_string(), "5".to_string())], "元素 addr 之外的成员照原文");
        assert!(ms[0].attempts.is_empty());
        assert_eq!(ms[1].resolved, j3, "簿上承接了的那一把是现行钥");
        assert_eq!(ms[1].attempts, vec![judge::Attempt { block: 21, to: j4.clone() }], "交出记录之后的声明是尝试,照示");
        assert_eq!((ms[1].name.clone(), ms[1].beyond.clone()), (None, Vec::new()), "裸地址元素与无资料的最终址:什么都不编");
        let adapter::JudgeReading::Judge { document, .. } = adapter::judge_of(&logs, &j1) else { panic!("一号判官有资料") };
        assert_eq!(judge::beyond(&document.raw, &document.unknown_fields), vec![("seal".to_string(), "[1,2]".to_string())]);
    }

    #[test]
    fn a_case_reads_the_guild_covenant_it_froze_not_the_latest() {
        let g = format!("0x{}", "a1".repeat(20));
        let cov = |v: u64, text: &str| format!(r#"{{"chainId":1,"guild":"{g}","issued":1790000000,"kind":"galeed.guildamanah/1","text_md":"{text}","version":{v}}}"#);
        let sha = |t: &str| eravon_galeed::hash::sha256(t.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>();
        let logs = Logs { publications: vec![published(&g, 9, cov(1, "旧约法")), published(&g, 12, cov(2, "新约法"))], statuses: vec![], declarations: vec![], accessions: vec![] };
        let old = covenant_by_word(&logs, &g, &format!("0x{}", sha("旧约法"))).expect("旧版在");
        assert_eq!((old.version, old.text), (Some(1), Ok("旧约法".to_string())));
        assert_eq!(covenant_by_word(&logs, &g.to_ascii_uppercase().replacen("0X", "0x", 1), &sha("新约法")).and_then(|t| t.version), Some(2));
        assert!(covenant_by_word(&logs, &g, &sha("没发布过")).is_none());
    }
}
