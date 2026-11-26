use eravon_store::vault::Seat;
use eravon_vault as h2;

use crate::house::seam;
use crate::house::seller::Said;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Ready {
    pub seat: bool,
    pub tail: bool,
    pub payout: String,
    pub unread: Option<(String, String)>,
}

impl Ready {
    pub fn read(data: &str, seat: Seat, me: &str) -> Ready {
        if me.is_empty() {
            return Ready::default();
        }
        let (laid, unread) = match h2::onboard::seat_ready(data, seat) {
            Ok(b) => (b, None),
            Err(e) => (false, Some((e.name().to_string(), e.detail()))),
        };
        let payout = if laid && seat == Seat::Seller { h2::store::read_identity(data, me).ok().map(|s| s.payout.trim().to_string()).unwrap_or_default() } else { String::new() };
        Ready { seat: laid, tail: !payout.is_empty(), payout, unread }
    }
}

#[derive(Clone)]
pub struct First {
    pub payout: String,
    pub again: String,
    pub said: Option<Said>,
    pub shake: bool,
}

impl First {
    pub fn new(_data: &str) -> First {
        First {
            payout: String::new(),
            again: String::new(),
            said: None,
            shake: false,
        }
    }

    pub fn line(&self, lang: crate::house::i18n::Lang) -> String {
        self.said.as_ref().map(|s| crate::house::refusal::line(lang, &s.name, &s.detail)).unwrap_or_default()
    }

    pub fn says(&self) -> &str {
        self.said.as_ref().map(|s| s.detail.as_str()).unwrap_or_default()
    }

    pub fn take_shake(&mut self) -> bool {
        std::mem::take(&mut self.shake)
    }

    pub fn fail(&mut self, said: Said) {
        self.said = Some(said);
        self.shake = true;
    }

    pub fn set_payout(&mut self, v: &str) {
        self.payout = v.to_string();
    }

    pub fn set_again(&mut self, v: &str) {
        self.again = v.to_string();
    }

    pub fn rows(&self) -> Vec<(&'static str, eravon_literals::json::Out)> {
        use eravon_literals::json::Out;
        let mut m = vec![
            ("payout", Out::S(self.payout.clone())),
            ("says", Out::S(self.says().to_string())),
        ];
        m.sort_by(|a, b| a.0.cmp(b.0));
        m
    }
}

pub fn chain_on(chosen: Option<u64>, home: Option<&eravon_vault::settings::Settings>) -> u64 {
    chosen.or_else(|| home.map(eravon_vault::store::chain_of)).unwrap_or(eravon_literals::constants::default_network().standard.chain_id)
}

pub fn born_added(data: &str, me: &str, chain: u64, now: u64) -> Result<usize, Said> {
    let s = h2::store::open_write(data, seam::WRITER, now).map_err(|e| Said::bad(e.name(), e.detail()))?;
    let b = h2::onboard::born_added(&s, data, me, chain, now);
    b.own.map_err(|e| crate::house::said_of(&e))?;
    let mut n = 1;
    for o in b.others {
        o.map_err(|e| crate::house::said_of(&e))?;
        n += 1;
    }
    Ok(n)
}

pub fn land_seat(data: &str, seat: Seat, me: &str, chain: u64, now: u64) -> Result<h2::onboard::Born, Said> {
    h2::onboard::seat(data, laid(seat), me, chain, now).map_err(|e| crate::house::said_of(&e))
}

fn laid(seat: Seat) -> Seat {
    seat
}


pub fn land_payout(data: &str, me: &str, payout: &str, again: &str, now: u64) -> Result<Result<usize, Said>, Said> {
    h2::payout::set(data, me, payout, again)
        .map(|_| eravon_goods::goods::note_payout(data, seam::WRITER, me, now).map_err(|e| Said::bad(e.name(), e.detail())))
        .map_err(|e| crate::house::said_of(&e))
}

#[cfg(test)]
mod tests {
    use super::{First, Ready};

    #[test]
    fn a_seat_without_a_key_or_settings_is_not_laid_out() {
        let none = std::env::temp_dir().join(format!("eravon-ready-{}", std::process::id())).to_string_lossy().to_string();
        assert_eq!(Ready::read(&none, eravon_store::vault::Seat::Seller, ""), Ready::default());
        assert_eq!(Ready::read(&none, eravon_store::vault::Seat::Buyer, "0x0000000000000000000000000000000000000001"), Ready::default());
    }

    #[test]
    fn the_receive_address_is_refused_unless_typed_twice_alike() {
        let a = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
        let b = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAee";
        let got = super::land_payout("/nowhere", "0x0000000000000000000000000000000000000001", a, b, 0);
        assert!(matches!(got, Err(ref s) if s.detail.contains("两次输入不一致")), "{:?}", got.err().map(|s| s.detail));
        let mut f = First::new("/nowhere");
        f.set_payout(a);
        f.set_again(a);
        assert_eq!((f.payout.as_str(), f.again.as_str()), (a, a), "the fields keep what was typed, case and all");
    }

    #[test]
    fn a_choice_of_network_stands_on_the_homes_own_network() {
        let head = eravon_literals::constants::default_network().standard.chain_id;
        let on_sepolia = eravon_vault::settings::Settings { chain_id: Some(11_155_111), ..Default::default() };
        let none_chosen = eravon_vault::settings::Settings::default();
        assert_eq!(super::chain_on(None, Some(&on_sepolia)), 11_155_111);
        assert_eq!(super::chain_on(None, Some(&none_chosen)), head);
        assert_eq!(super::chain_on(None, None), head);
        assert_eq!(super::chain_on(Some(1), Some(&on_sepolia)), 1, "what was chosen stands");
    }
}
