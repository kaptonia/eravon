use eravon_cands::wire::Wire;
use eravon_roster::adapter::Logs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Expiry,
    Candidate,
    Payout,
    Fund,
}

impl Reason {
    pub const fn word(self) -> &'static str {
        match self {
            Reason::Expiry => "expiry",
            Reason::Candidate => "candidate",
            Reason::Payout => "payout",
            Reason::Fund => "fund",
        }
    }
    pub fn of(s: &str) -> Option<Reason> {
        match s {
            "expiry" => Some(Reason::Expiry),
            "candidate" => Some(Reason::Candidate),
            "payout" => Some(Reason::Payout),
            "fund" => Some(Reason::Fund),
            _ => None,
        }
    }
}

pub fn named(terms_bytes: &[u8]) -> Vec<String> {
    let Ok(t) = eravon_terms::doc::terms(terms_bytes) else { return Vec::new() };
    t.judges.iter().map(|j| j.judge.clone()).chain(t.guilds.iter().map(|g| g.address.clone())).collect()
}

pub fn due(data: &str, me: &str, terms_bytes: &[u8], now: u64, chain: Option<(&Wire, &Logs)>) -> Vec<Reason> {
    let read = chain.map(|(w, logs)| Read::of(w, logs, &[terms_bytes]));
    due_with(data, me, terms_bytes, now, read.as_ref())
}

#[derive(Default)]
pub struct Read {
    judges: Vec<((String, String), Option<String>)>,
    guilds: Vec<(String, Option<(Option<String>, Vec<(String, String)>)>)>,
}

impl Read {
    pub fn of(w: &Wire, logs: &Logs, docs: &[&[u8]]) -> Read {
        let mut judges: Vec<(String, String)> = Vec::new();
        let mut guilds: Vec<String> = Vec::new();
        for t in docs.iter().filter_map(|b| eravon_terms::doc::terms(b).ok()) {
            for j in &t.judges {
                let k = (j.judge.clone(), j.amanah_hash.clone());
                if !judges.contains(&k) {
                    judges.push(k);
                }
            }
            for g in &t.guilds {
                if !guilds.contains(&g.address) {
                    guilds.push(g.address.clone());
                }
            }
        }
        let held = std::sync::Arc::new((w.clone(), logs.clone()));
        let h = held.clone();
        let jv = eravon_chain::read::side_by_side(judges.clone(), move |(a, hh)| eravon_cands::judge::open(&h.0, &h.1, &a, &hh).ok().map(|v| v.amanah));
        let gv = eravon_chain::read::side_by_side(guilds.clone(), move |g| eravon_cands::guild::open(&held.0, &held.1, &g).ok().map(|v| (v.quote_hashes(), v.terms.map(|x| x.sha256))).map(|(q, t)| (t, q)));
        Read { judges: judges.into_iter().zip(jv).collect(), guilds: guilds.into_iter().zip(gv).collect() }
    }

    fn judge(&self, addr: &str, word: &str) -> Option<&String> {
        self.judges.iter().find(|((a, h), _)| a == addr && h == word).and_then(|(_, v)| v.as_ref())
    }

    fn guild(&self, addr: &str) -> Option<&(Option<String>, Vec<(String, String)>)> {
        self.guilds.iter().find(|(a, _)| a == addr).and_then(|(_, v)| v.as_ref())
    }
}

pub fn due_with(data: &str, me: &str, terms_bytes: &[u8], now: u64, chain: Option<&Read>) -> Vec<Reason> {
    let Ok(t) = eravon_terms::doc::terms(terms_bytes) else { return Vec::new() };
    let mut out = Vec::new();
    let lead = u64::from(eravon_literals::constants::RESIGN_LEAD_DAYS) * 86_400;
    if t.expiry.saturating_sub(lead) <= now {
        out.push(Reason::Expiry);
    }
    if let Some(read) = chain {
        let fresh = t.judges.iter().any(|j| read.judge(&j.judge, &j.amanah_hash).is_some_and(|word| eravon_cands::judge::resign_due(&j.amanah_hash, word)));
        let fresh_guild = t.guilds.iter().any(|g| {
            read.guild(&g.address).is_some_and(|(terms, quotes)| {
                let now = quotes.iter().find(|(tok, _)| tok.eq_ignore_ascii_case(&t.token)).map(|(_, h)| h.as_str());
                terms.as_ref().is_some_and(|sha| eravon_cands::guild::guild_due(&g.amanah_hash, &g.quote_hash, sha, now))
            })
        });
        if fresh || fresh_guild {
            out.push(Reason::Candidate);
        }
    }
    if payout_moved(data, me, &t.seller_key, &t.seller_payout) {
        out.push(Reason::Payout);
    }
    if fund_moved(data, me, &t.seller_key, &t.seller_fund) {
        out.push(Reason::Fund);
    }
    out
}

fn fund_moved(data: &str, me: &str, seller: &str, signed: &str) -> bool {
    let ours = seller.eq_ignore_ascii_case(me) || eravon_vault::store::keys_of(data, me).is_ok_and(|v| v.iter().any(|k| k.eq_ignore_ascii_case(seller)));
    if !ours {
        return false;
    }
    let Ok(w) = eravon_payways::ways::read(data, me) else { return false };
    !w.fund.is_empty() && !w.fund.eq_ignore_ascii_case(signed)
}


fn payout_moved(data: &str, me: &str, seller: &str, signed: &str) -> bool {
    let Some(now) = payout_source(data, me, seller) else { return false };
    !now.is_empty() && now.to_ascii_lowercase() != signed.to_ascii_lowercase()
}


fn payout_source(data: &str, me: &str, seller: &str) -> Option<String> {
    seller.eq_ignore_ascii_case(me).then(|| eravon_vault::store::read_identity(data, me).map(|s| s.payout).unwrap_or_default())
}


#[cfg(test)]
mod tests {
    #[test]
    fn a_vault_moved_off_what_an_item_signed_asks_for_its_re_sign() {
        const NOW: u64 = 1_790_000_000;
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create("48261537", eravon_store::vault::Seat::Seller, "主钥", Some("test test test test test test test test test test test junk"), NOW).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut session = eravon_vault::session::Session::locked(&data);
        session.unlock("48261537").expect("开得了");
        let me = eravon_vault::accounts::who(&data, eravon_store::vault::Seat::Seller).expect("首枚钥").to_ascii_lowercase();
        let s = eravon_vault::store::open_write(&data, "goods-test", NOW).expect("可写");
        s.init(&eravon_store::settings::Global::default()).expect("铺得下");
        s.init_identity(&me, NOW).expect("登记身份");
        drop(s);
        let outside = "0x00000000000000000000000000000000000000b2";
        let company = "0x00000000000000000000000000000000000000c3";
        assert!(!super::fund_moved(&data, &me, &me, &me), "新装默认本机金库:签的就是这把钥");
        assert!(super::fund_moved(&data, &me, &me, outside), "签的是外部金库,此刻是本机金库");
        assert!(!super::fund_moved(&data, &me, company, outside), "公司那一版不看本席金库");
        let edit = |e: eravon_payways::ways::Edit| eravon_payways::ways::save(&data, "goods-test", NOW, &me, &e).expect("存得下");
        edit(eravon_payways::ways::Edit { fund: Some(outside.into()), ..Default::default() });
        assert!(super::fund_moved(&data, &me, &me, &me), "关上开关:签在本钥上的件要重签");
        assert!(!super::fund_moved(&data, &me, &me, outside));
        edit(eravon_payways::ways::Edit { own: Some(true), ..Default::default() });
        assert!(super::fund_moved(&data, &me, &me, outside), "拨回开着:签在外部金库的件要重签");
    }
}
