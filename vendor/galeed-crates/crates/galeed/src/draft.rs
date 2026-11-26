//! A machine's own entries read the way §8.2 and §8.4 read a ledger: the draft walk. Product use only, and no audit.
//!
//! An entry signed and not yet carried by a counted anchor is a draft (§5.7), and a draft is no entry of any ledger:
//! the audit reads only what the chain carries (§8.1), so a report never says a word about entries a machine holds
//! beside it. A writer still has to know, before it signs the next position, whether what it holds walks clean:
//! which position comes next, which entry it links to, and whether the bytes it is about to add would open a gap,
//! break a link, sign out of office or fork the line. That is this walk: the lineage (§7.4, timeless: a draft holds no records) over the given entries,
//! the walk of §8.2 and the pair test of §8.4 over those whose `judge` is in it, by the very functions the audit runs.
//! It answers findings and the head, and never a label: §8.7's four labels are what a report says about a chain, and
//! nothing here is one.

use crate::audit::{self, Parsed};
use crate::json::Value;
use std::collections::BTreeSet;

/// The draft walk's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    /// The entries walked: the given byte strings that pass §4.3 and whose `judge` is in the lineage the given
    /// entries reach from the root, byte-identical copies one.
    pub entries: usize,
    /// The findings of §8.2 and §8.4 (`EQUIVOCATION`) over those entries, in the row form and the order of a report
    /// (§8.7 item 3). `CARRIED_MALFORMED` is a finding about anchor records, and a draft has none.
    pub findings: Vec<Value>,
    /// Whether any of the findings is hard (§8.3).
    pub hard: bool,
    /// The highest `seq` walked and the `entry_id`s there, bytewise ascending; `None` when nothing was walked. Two
    /// ids at the head are a fork the findings already name.
    pub head: Option<(u64, Vec<String>)>,
}

impl Draft {
    /// Whether the entries walked make one line: no two at one position and no link to the wrong entry (no
    /// `EQUIVOCATION`, no `PREV_MISMATCH`, §8.2 and §8.4), the core's own judgment over its own finding names, so a
    /// reader above it (a hand-over's check) asks this and compares none of them.
    pub fn one_line(&self) -> bool {
        !self.findings.iter().any(|f| matches!(f.get("name").and_then(Value::as_str), Some(crate::audit::EQUIVOCATION) | Some(crate::audit::PREV_MISMATCH)))
    }

    /// The draft walks clean: no finding at all, soft or hard.
    pub fn clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Walk `entries` under `root`. Byte strings that fail §4.3 and entries whose `judge` the lineage does not hold are
/// not walked.
pub fn walk<'a>(root: &str, entries: impl IntoIterator<Item = &'a [u8]>) -> Draft {
    let distinct: BTreeSet<&[u8]> = entries.into_iter().collect();
    let (passed, _malformed) = audit::tested(distinct);
    // A draft holds bytes and no records: the timeless fixed point (§7.4 without the block times).
    let lineage = audit::fixed_point(root, passed.iter());
    let mut walked: Vec<Parsed> = passed.into_iter().filter(|e| lineage.contains(&e.judge)).collect();
    walked.sort_by(|a, b| (a.seq, a.id.as_bytes()).cmp(&(b.seq, b.id.as_bytes())));
    let mut findings = Vec::new();
    audit::walk(root, &walked, &mut findings);
    audit::equivocations(&walked, &mut findings);
    let findings = audit::in_report_order(findings);
    let hard = findings.iter().any(audit::Finding::hard);
    let head = walked.last().map(|top| (top.seq, walked.iter().filter(|e| e.seq == top.seq).map(|e| e.id.clone()).collect()));
    Draft { entries: walked.len(), findings: findings.into_iter().map(audit::Finding::value).collect(), hard, head }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{addr, entry, id, input, key, record};

    fn names(d: &Draft) -> Vec<String> {
        d.findings.iter().map(|f| f.get("name").and_then(Value::as_str).unwrap().to_string()).collect()
    }

    /// A line of three walks clean and names its head; nothing walked has no head; a byte string that is no entry
    /// and an entry outside the lineage are not walked and accuse nobody.
    #[test]
    fn a_whole_line_walks_clean_and_names_its_head() {
        let a = key(1);
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let n1 = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"one"}"#);
        let n2 = entry(&a, 2, Some(&id(&n1)), "annotation", r#"{"note_md":"two"}"#);
        let stranger = entry(&key(9), 0, None, "genesis", r#"{"statement_md":"s"}"#);
        let d = walk(&root, [g.as_slice(), n1.as_slice(), n2.as_slice(), n2.as_slice(), b"not an entry".as_slice(), stranger.as_slice()]);
        assert!(d.clean(), "{:?}", d.findings);
        assert!(!d.hard);
        assert_eq!(d.entries, 3, "同字节一份,坏形与世系外的不走");
        assert_eq!(d.head, Some((2, vec![id(&n2)])));
        assert_eq!(walk(&root, std::iter::empty()), Draft { entries: 0, findings: vec![], hard: false, head: None });
    }

    /// The four things a writer must not add, each named as a report names it: a gap (soft), a broken link, a
    /// position signed out of office after a handover, and a second entry at one position (hard).
    #[test]
    fn each_fault_a_writer_could_add_is_named_as_the_walk_names_it() {
        let (a, b) = (key(1), key(2));
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let n1 = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"one"}"#);

        let gap = entry(&a, 3, Some(&id(&n1)), "annotation", r#"{"note_md":"three"}"#);
        let d = walk(&root, [g.as_slice(), n1.as_slice(), gap.as_slice()]);
        assert_eq!(names(&d), vec!["SEQ_GAP"]);
        assert!(!d.hard, "缺位是软的");

        let unlinked = entry(&a, 2, Some(&id(&g)), "annotation", r#"{"note_md":"two"}"#);
        let d = walk(&root, [g.as_slice(), n1.as_slice(), unlinked.as_slice()]);
        assert_eq!(names(&d), vec!["EQUIVOCATION", "PREV_MISMATCH"], "{:?}", d.findings);
        assert!(d.hard);

        let hand = entry(&a, 2, Some(&id(&n1)), "succession", &format!(r#"{{"to":"{}","kind":"handover","effective":0,"statement_md":""}}"#, addr(&b)));
        let after = entry(&a, 3, Some(&id(&hand)), "annotation", r#"{"note_md":"the old key again"}"#);
        let d = walk(&root, [g.as_slice(), n1.as_slice(), hand.as_slice(), after.as_slice()]);
        assert_eq!(names(&d), vec!["AUTHORITY_MISMATCH"]);
        assert!(d.hard);
        let by_b = entry(&b, 3, Some(&id(&hand)), "annotation", r#"{"note_md":"the new key"}"#);
        assert!(walk(&root, [g.as_slice(), n1.as_slice(), hand.as_slice(), by_b.as_slice()]).clean(), "交接后新钥接写即干净");

        let twin = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"another one"}"#);
        let d = walk(&root, [g.as_slice(), n1.as_slice(), twin.as_slice()]);
        assert_eq!(names(&d), vec!["EQUIVOCATION"]);
        let mut ids = vec![id(&n1), id(&twin)];
        ids.sort();
        assert_eq!(d.head, Some((1, ids)), "分叉的头两枚都列出");
    }

    /// The draft walk and the audit agree on the walk: the same entries, all carried by counted records of the root,
    /// give the audit the same findings.
    #[test]
    fn the_walk_is_the_audits_walk() {
        let a = key(1);
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let gap = entry(&a, 2, Some(&id(&g)), "annotation", r#"{"note_md":"two"}"#);
        let records: Vec<String> = [&g, &gap].iter().enumerate().map(|(i, b)| record(i as u64 + 1, &root, "counted", &id(b), Some(b))).collect();
        let input = input(&root, &records, std::slice::from_ref(&root));
        let report = audit::audit(&audit::read_input(&input).unwrap());
        assert_eq!(report.get("findings"), Some(&Value::Arr(walk(&root, [g.as_slice(), gap.as_slice()]).findings)));
    }
}
