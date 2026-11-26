use eravon_literals::json::Out;

use crate::house::seller::Said;

pub type Pick = eravon_dispute::seal::Pick;

#[derive(Clone, Debug)]
pub struct Card {
    pub address: String,
    pub name: Option<String>,
    pub home: Option<String>,
    pub note_md: Option<String>,
    pub frozen: String,
    pub frozen_version: Option<u64>,
    pub frozen_text: Option<String>,
    pub frozen_discarded: bool,
    pub pick: Pick,
    pub unread: Option<String>,
}

impl Card {
    pub fn note(&self) -> Option<&str> {
        self.note_md.as_deref().filter(|s| !s.trim().is_empty())
    }

}

pub fn read(data: &str, me: &str, judge: &str, amanah: &str, now: u64) -> Result<Card, Said> {
    if judge.trim_start_matches("0x").chars().all(|c| c == '0') {
        return Err(Said::bad(eravon_cli::deskops::seal::R_NO_JUDGE, crate::house::i18n::t(crate::house::i18n::Key::ErrEJUDGENONE)));
    }
    on_table(data, me, judge)?;
    let w = eravon_cands::wire::Wire::of(data, me, &crate::house::orders::injected(), now).map_err(|a| Said::bad(a.name(), a.detail()))?;
    let r = eravon_cli::deskops::seal::read(&w, judge, amanah).map_err(|n| Said::bad(&n.name, n.detail()))?;
    Ok(Card {
        address: r.address,
        name: r.name,
        home: r.home,
        note_md: r.note_md,
        frozen: r.frozen,
        frozen_version: r.frozen_version,
        frozen_text: r.frozen_text,
        frozen_discarded: r.frozen_discarded,
        pick: r.pick,
        unread: r.unread,
    })
}

fn on_table(_data: &str, _me: &str, _judge: &str) -> Result<(), Said> {
    Ok(())
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealable {
    pub origins: eravon_dispute::seal::Origins,
    pub manifest: Vec<u8>,
}

pub fn export(card: &Card, s: &Sealable, dir: &str) -> Result<eravon_dispute::seal::Sealed, Said> {
    eravon_dispute::seal::export(&s.origins, &s.manifest, std::slice::from_ref(&card.pick), dir).map_err(|(n, d)| Said::bad(&n, d))
}

pub fn pick_out(p: &Pick) -> Out {
    eravon_cli::deskops::seal::pick_out(p)
}

pub fn card_out(c: &Result<Card, Said>) -> Out {
    match c {
        Err(s) => Out::O(vec![("refused", s.out())]),
        Ok(c) => {
            let opt = |x: &Option<String>| Out::S(x.clone().unwrap_or_default());
            let mut r = vec![
                ("address", Out::S(c.address.clone())),
                ("frozen", Out::S(c.frozen.clone())),
                ("home", opt(&c.home)),
                ("name", opt(&c.name)),
                ("noteMd", Out::S(c.note().unwrap_or_default().to_string())),
                ("pick", pick_out(&c.pick)),
            ];
            if let Some(v) = c.frozen_version {
                r.push(("frozenVersion", Out::N(v)));
            }
            if let Some(u) = &c.unread {
                r.push(("unread", Out::S(u.clone())));
            }
            r.sort_by(|a, b| a.0.cmp(b.0));
            Out::O(r)
        }
    }
}

pub fn sealed_out(s: &Result<eravon_dispute::seal::Sealed, Said>) -> Out {
    match s {
        Ok(x) => Out::O(vec![("dir", Out::S(x.dir.clone())), ("files", Out::A(x.files.iter().map(|f| Out::S(f.clone())).collect()))]),
        Err(e) => Out::O(vec![("refused", e.out())]),
    }
}

pub fn pick_said(c: &Card) -> String {
    use crate::house::i18n::{Key, filln, t};
    match &c.pick {
        Pick::One(_) => t(Key::JudgeRecipientOne),
        Pick::Candidates { reason, list, .. } => filln(Key::JudgeRecipientCandidates, &[reason.name(), &list.len().to_string()]),
        Pick::None => t(Key::JudgeRecipientNone),
        Pick::Unread => filln(Key::JudgeRecipientUnread, &[c.unread.as_deref().unwrap_or("")]),
    }
}
