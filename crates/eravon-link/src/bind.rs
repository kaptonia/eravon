use eravon_galeed as k1;
use eravon_terms as k2;
use k2::form;

use crate::listing::Listing;
use crate::token::Verdict;

fn same_members(a: &k1::Value, b: &k1::Value, keys: &[&str]) -> bool {
    keys.iter().all(|k| a.get(k) == b.get(k))
}

pub fn bind(listing_bytes: &[u8], terms_bytes: &[u8]) -> Result<(Listing, k2::doc::Terms), Verdict> {
    let s = crate::listing::listing(listing_bytes).map_err(|f| Verdict::new("LISTING_INVALID").own(f))?;
    let t = k2::doc::terms(terms_bytes).map_err(|f| Verdict::new("TERMS_INVALID").up(f))?;
    if k1::hash::hex_encode(&t.terms_hash) != s.terms_hash {
        return Err(Verdict::new("BIND_HASH"));
    }
    compare(&s, &t)?;
    Ok((s, t))
}

pub fn compare(s: &Listing, t: &k2::doc::Terms) -> Result<(), Verdict> {
    if s.chain_id != t.chain_id {
        return Err(Verdict::new("BIND_CHAIN"));
    }
    if s.token != t.token {
        return Err(Verdict::new("BIND_TOKEN"));
    }
    if s.logic != t.logic {
        return Err(Verdict::new("BIND_LOGIC"));
    }
    let o = &s.order;
    let same_order = o.seller_key == t.seller_key
        && o.seller_payout == t.seller_payout
        && o.seller_fund == t.seller_fund
        && o.amount == t.amount
        && o.delivery_window == t.delivery_window
        && o.confirm_window == t.confirm_window
        && o.open_window == t.open_window
        && o.buyer_may_open == t.buyer_may_open
        && o.ladder == t.ladder
        && o.nonce == t.nonce
        && o.expiry == t.expiry;
    if !same_order {
        return Err(Verdict::new("BIND_ORDER"));
    }

    let sj = form::as_arr(s.value.get("judges").expect("清单已过 §2.5")).expect("已过形");
    let tj = form::as_arr(t.block.get("judges").expect("文档已过 §2.4")).expect("已过形");
    if sj.len() != tj.len() {
        return Err(Verdict::at("BIND_JUDGES", sj.len()));
    }
    for i in 0..sj.len() {
        if !same_members(&sj[i], &tj[i], &["judge", "amanahHash", "quoteHash", "source"]) {
            return Err(Verdict::at("BIND_JUDGES", i));
        }
    }
    let sg = form::as_arr(s.value.get("guilds").expect("清单已过 §2.5")).expect("已过形");
    let tg = form::as_arr(t.block.get("guilds").expect("文档已过 §2.4")).expect("已过形");
    if sg.len() != tg.len() {
        return Err(Verdict::at("BIND_GUILDS", sg.len()));
    }
    for i in 0..sg.len() {
        if !same_members(&sg[i], &tg[i], &["pointer", "amanahHash", "quoteHash"]) {
            return Err(Verdict::at("BIND_GUILDS", i));
        }
    }
    Ok(())
}

