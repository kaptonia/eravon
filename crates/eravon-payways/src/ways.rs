use eravon_literals::json::In;
use eravon_store::settings::Identity;

use crate::fault::{Fault, R};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crypto {
    Always,
}

pub const AUTO_OPEN_DEFAULT: bool = true;
pub const OPEN_WINDOW_DEFAULT: u64 = 2 * 86_400;
pub const BUYER_MAY_OPEN_DEFAULT: bool = false;

const K_AUTO: &str = "openAuto";
const K_WINDOW: &str = "openWindow";
const K_BUYER: &str = "buyerMayOpen";
const K_FUND: &str = "sellerFund";
const K_OWN: &str = "fundOwn";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ways {
    pub crypto: Crypto,
    pub auto_open: bool,
    pub open_window: u64,
    pub buyer_may_open: bool,
    pub fund: String,
    pub kept: String,
    pub own: bool,
}

fn member<'a>(t: &'a Identity, k: &str) -> Option<&'a In> {
    t.extra.iter().find(|(n, _)| n == k).map(|(_, v)| v)
}

pub fn view(t: &Identity, addr: &str) -> Ways {
    let flag = |k: &str, d: bool| match member(t, k) {
        Some(In::Bool(b)) => *b,
        _ => d,
    };
    let open_window = match member(t, K_WINDOW) {
        Some(In::Int(n)) if *n > 0 && *n <= i64::from(u32::MAX) => *n as u64,
        _ => OPEN_WINDOW_DEFAULT,
    };
    let kept = match member(t, K_FUND) {
        Some(In::Str(s)) if eravon_galeed::hexform::is_hex20(s) => s.to_ascii_lowercase(),
        _ => String::new(),
    };
    let own = match member(t, K_OWN) {
        Some(In::Bool(b)) => *b,
        _ => kept.is_empty() || kept.eq_ignore_ascii_case(addr),
    };
    let fund = if own { addr.to_ascii_lowercase() } else { kept.clone() };
    Ways { crypto: Crypto::Always, auto_open: flag(K_AUTO, AUTO_OPEN_DEFAULT), open_window, buyer_may_open: flag(K_BUYER, BUYER_MAY_OPEN_DEFAULT), fund, kept, own }
}

pub fn read(data: &str, addr: &str) -> R<Ways> {
    Ok(view(&read_identity(data, addr)?, addr))
}

fn read_identity(data: &str, addr: &str) -> R<Identity> {
    match eravon_vault::store::read_identity(data, addr) {
        Ok(t) => Ok(t),
        Err(eravon_store::fault::Fault::Missing(_)) => Ok(Identity::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn carry(t: &mut Identity, old: &str) {
    let w = view(t, old);
    put(t, K_OWN, In::Bool(w.own));
    if w.kept.eq_ignore_ascii_case(old) {
        t.extra.retain(|(n, _)| n != K_FUND);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Edit {
    pub auto_open: Option<bool>,
    pub open_window: Option<u64>,
    pub buyer_may_open: Option<bool>,
    pub fund: Option<String>,
    pub own: Option<bool>,
}

pub fn save(data: &str, writer: &str, now: u64, addr: &str, e: &Edit) -> R<Ways> {
    crate::mark::mark(crate::mark::FEATURE);
    let s = eravon_vault::store::open_write(data, writer, now)?;
    let mut t = read_identity(s.data(), addr)?;
    let judged = judge_into(&mut t, e);
    if lands(&judged) {
        eravon_vault::store::write_identity(s.data(), addr, &t)?;
    }
    judged?;
    Ok(view(&t, addr))
}

fn lands(judged: &R<()>) -> bool {
    judged.is_ok()
}


fn judge_into(t: &mut Identity, e: &Edit) -> R<()> {
    let judged = judge(e)?;
    apply(t, judged);
    Ok(())
}


pub fn open_window(secs: u64) -> R<u64> {
    if secs == 0 || secs > u64::from(u32::MAX) {
        return Err(Fault::OpenWindow(secs.to_string()));
    }
    Ok(secs)
}

pub fn fund(s: &str) -> R<String> {
    let a = s.trim().to_ascii_lowercase();
    if !eravon_galeed::hexform::is_hex20(&a) || a.bytes().skip(2).all(|b| b == b'0') {
        return Err(Fault::Fund(s.to_string()));
    }
    Ok(a)
}

fn judge(e: &Edit) -> R<Edit> {
    Ok(Edit {
        auto_open: e.auto_open,
        open_window: e.open_window.map(open_window).transpose()?,
        buyer_may_open: e.buyer_may_open,
        fund: e.fund.as_deref().map(fund).transpose()?,
        own: e.own,
    })
}

fn put(t: &mut Identity, k: &str, v: In) {
    match t.extra.iter_mut().find(|(n, _)| n == k) {
        Some(slot) => slot.1 = v,
        None => t.extra.push((k.to_string(), v)),
    }
}

fn apply(t: &mut Identity, j: Edit) {
    if let Some(b) = j.auto_open {
        put(t, K_AUTO, In::Bool(b));
    }
    if let Some(n) = j.open_window {
        put(t, K_WINDOW, In::Int(n as i64));
    }
    if let Some(b) = j.buyer_may_open {
        put(t, K_BUYER, In::Bool(b));
    }
    if let Some(a) = j.fund {
        put(t, K_FUND, In::Str(a));
        put(t, K_OWN, In::Bool(false));
    }
    if let Some(b) = j.own {
        put(t, K_OWN, In::Bool(b));
    }
}

pub fn rows(w: &Ways) -> Vec<(&'static str, eravon_literals::json::Out)> {
    use eravon_literals::json::Out;
    vec![
        ("autoOpen", Out::B(w.auto_open)),
        ("buyerMayOpen", Out::B(w.buyer_may_open)),
        ("crypto", Out::S("always".to_string())),
        ("fund", Out::S(w.kept.clone())),
        ("fundOwn", Out::B(w.own)),
        ("openWindow", Out::N(w.open_window)),
        ("vault", Out::S(w.fund.clone())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opening_window_is_a_uint32_and_the_vault_a_nonzero_address() {
        assert!(open_window(0).is_err());
        assert_eq!(open_window(u64::from(u32::MAX)).ok(), Some(u64::from(u32::MAX)));
        assert!(open_window(u64::from(u32::MAX) + 1).is_err());
        assert!(fund("0x0000000000000000000000000000000000000000").is_err());
        assert!(fund("0x12").is_err());
        assert_eq!(fund("0xAE550C700A6A38C925777B13252912015732BD1D").ok().as_deref(), Some("0xae550c700a6a38c925777b13252912015732bd1d"));
    }

    #[test]
    fn one_bad_item_refuses_the_whole_edit_in_memory() {
        let mut t = Identity::default();
        let e = Edit { auto_open: Some(false), open_window: Some(0), ..Edit::default() };
        assert_eq!(judge_into(&mut t, &e).unwrap_err().name(), "E_PAYWAYS_OPEN_WINDOW");
        assert_eq!(t, Identity::default(), "一项不过,整份一处也没动");
        let me = "0x00000000000000000000000000000000000000e1";
        assert_eq!(view(&t, me), Ways { crypto: Crypto::Always, auto_open: true, open_window: OPEN_WINDOW_DEFAULT, buyer_may_open: false, fund: me.to_string(), kept: String::new(), own: true });
        let fund_at = "0x00000000000000000000000000000000000000F1";
        let e = Edit { auto_open: Some(false), open_window: Some(u64::from(u32::MAX)), buyer_may_open: Some(true), fund: Some(fund_at.into()), own: None };
        judge_into(&mut t, &e).expect("全过");
        let w = view(&t, me);
        assert_eq!((w.auto_open, w.open_window, w.buyer_may_open, w.fund.as_str(), w.own), (false, u64::from(u32::MAX), true, "0x00000000000000000000000000000000000000f1", false));
        assert_eq!(Identity::open(&t.seal()).map(|x| view(&x, me)).ok(), Some(w), "写回再读一字不差");
        assert!(open_window(1).is_ok() && open_window(u64::from(u32::MAX) + 1).is_err());
        for bad in ["", "0x", "0x0000000000000000000000000000000000000000", "0xzz00000000000000000000000000000000000000", "0x00000000000000000000000000000000000000f"] {
            assert_eq!(fund(bad).unwrap_err().name(), "E_PAYWAYS_FUND", "{bad:?}");
        }
    }

    fn with(kept: Option<&str>, own: Option<bool>) -> Identity {
        let mut t = Identity::default();
        if let Some(k) = kept {
            put(&mut t, K_FUND, In::Str(k.to_string()));
        }
        if let Some(b) = own {
            put(&mut t, K_OWN, In::Bool(b));
        }
        t
    }

    #[test]
    fn the_vault_in_effect_is_the_seller_key_while_the_switch_is_on() {
        let me = "0x00000000000000000000000000000000000000a1";
        let outside = "0x00000000000000000000000000000000000000b2";
        let former = "0x00000000000000000000000000000000000000c3";
        let cases: [(Option<&str>, Option<bool>, &str, bool); 12] = [
            (None, None, me, true),
            (Some(me), None, me, true),
            (Some(outside), None, outside, false),
            (Some(former), None, former, false),
            (None, Some(true), me, true),
            (Some(me), Some(true), me, true),
            (Some(outside), Some(true), me, true),
            (Some(former), Some(true), me, true),
            (None, Some(false), "", false),
            (Some(me), Some(false), me, false),
            (Some(outside), Some(false), outside, false),
            (Some(former), Some(false), former, false),
        ];
        for (kept, own, fund, on) in cases {
            let w = view(&with(kept, own), me);
            assert_eq!((w.fund.as_str(), w.own, w.kept.as_str()), (fund, on, kept.unwrap_or("")), "{kept:?} {own:?}");
        }
    }

    #[test]
    fn writing_a_vault_address_turns_the_switch_off_and_keeps_it_under_the_switch() {
        let (me, outside) = ("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000b2");
        let mut t = Identity::default();
        judge_into(&mut t, &Edit { fund: Some(outside.into()), ..Edit::default() }).unwrap();
        assert_eq!((view(&t, me).fund.as_str(), view(&t, me).own), (outside, false));
        judge_into(&mut t, &Edit { own: Some(true), ..Edit::default() }).unwrap();
        assert_eq!((view(&t, me).fund.as_str(), view(&t, me).kept.as_str()), (me, outside));
        judge_into(&mut t, &Edit { own: Some(false), ..Edit::default() }).unwrap();
        assert_eq!(view(&t, me).fund, outside);
        let new = "0x00000000000000000000000000000000000000d4";
        judge_into(&mut t, &Edit { own: Some(true), ..Edit::default() }).unwrap();
        assert_eq!(view(&t, new).fund, new);
        let rows = rows(&view(&t, me));
        let get = |k: &str| rows.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        use eravon_literals::json::Out;
        assert!(matches!(get("fund"), Some(Out::S(x)) if x == outside));
        assert!(matches!(get("vault"), Some(Out::S(x)) if x == me));
        assert!(matches!(get("fundOwn"), Some(Out::B(true))));
    }

    #[test]
    fn a_rotation_carries_the_switch_and_the_new_key_is_its_own_vault() {
        let (old, new, outside) = ("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000d4", "0x00000000000000000000000000000000000000b2");
        for (kept, own) in [(None, None), (Some(old), None), (Some(outside), Some(true)), (Some(old), Some(true))] {
            let mut t = with(kept, own);
            carry(&mut t, old);
            let w = view(&t, new);
            assert_eq!((w.fund.as_str(), w.own), (new, true), "{kept:?} {own:?}");
            assert!(!w.kept.eq_ignore_ascii_case(old), "{kept:?} {own:?}: the old key stays behind");
        }
        let mut t = with(Some(outside), None);
        carry(&mut t, old);
        assert_eq!((view(&t, new).fund.as_str(), view(&t, new).own), (outside, false));
    }
}
