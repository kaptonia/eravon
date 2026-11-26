use eravon_terms::quote::Quote;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lapse {
    FullRefund,
    Half,
    ToSeller,
}

impl Lapse {
    pub const ALL: [Lapse; 3] = [Lapse::ToSeller, Lapse::Half, Lapse::FullRefund];

    pub fn of(word: u64) -> Option<Lapse> {
        let bps = share(word)?;
        Lapse::ALL.into_iter().find(|l| l.bps() == bps)
    }

    pub fn bps(self) -> u16 {
        match self {
            Lapse::ToSeller => 0,
            Lapse::Half => 5_000,
            Lapse::FullRefund => 10_000,
        }
    }

    pub fn word(self) -> u64 {
        match self {
            Lapse::ToSeller => 0,
            Lapse::Half => 1,
            Lapse::FullRefund => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lapse::ToSeller => "toSeller",
            Lapse::Half => "half",
            Lapse::FullRefund => "fullRefund",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clocks {
    pub offer_window: u64,
    pub ruling_window: u64,
    pub lapse: Lapse,
    pub auto_accept: bool,
}

pub fn share(word: u64) -> Option<u16> {
    match word {
        0 => Some(0),
        1 => Some(5_000),
        2 => Some(10_000),
        _ => None,
    }
}


pub fn clocks(q: &Quote) -> Result<Clocks, String> {
    let lapse = Lapse::of(q.lapse_default).ok_or_else(|| format!("E_QUOTE_FIELD 13 lapseDefault {}", q.lapse_default))?;
    Ok(Clocks { offer_window: q.offer_window, ruling_window: q.ruling_window, lapse, auto_accept: q.auto_accept })
}

pub fn of_spelling(spelling: &str) -> Result<Clocks, String> {
    crate::quote::decode(spelling).and_then(|q| clocks(&q))
}

pub fn of_bytes(bytes: &[u8]) -> Result<Clocks, String> {
    eravon_terms::quote::decode(bytes).map_err(|e| format!("{e:?}")).and_then(|q| clocks(&q))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    NotTaken,
    ResignBeforeDelivery,
    ResignAfterDelivery,
    ResignInDispute,
    NoRuling,
}

impl Case {
    pub const ALL: [Case; 5] = [Case::NotTaken, Case::ResignBeforeDelivery, Case::ResignAfterDelivery, Case::ResignInDispute, Case::NoRuling];

    pub fn name(self) -> &'static str {
        match self {
            Case::NotTaken => "notTaken",
            Case::ResignBeforeDelivery => "resignBeforeDelivery",
            Case::ResignAfterDelivery => "resignAfterDelivery",
            Case::ResignInDispute => "resignInDispute",
            Case::NoRuling => "noRuling",
        }
    }

    pub fn outcome(self, lapse: Lapse, guild: bool) -> Outcome {
        match self {
            Case::NotTaken if guild => Outcome::Redraw,
            Case::NotTaken | Case::ResignBeforeDelivery => Outcome::Refund,
            Case::ResignAfterDelivery => Outcome::ConfirmAsUsual,
            Case::ResignInDispute | Case::NoRuling => Outcome::Lapse(lapse),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Refund,
    Redraw,
    ConfirmAsUsual,
    Lapse(Lapse),
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Refund => "refund",
            Outcome::Redraw => "redraw",
            Outcome::ConfirmAsUsual => "confirmAsUsual",
            Outcome::Lapse(l) => l.name(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(lapse: u8, offer: u32, ruling: u32) -> Vec<u8> {
        let mut b = vec![0u8; eravon_terms::quote::LEN];
        b[31] = 1;
        let put = |b: &mut Vec<u8>, w: usize, v: u64| b[w * 32 + 24..w * 32 + 32].copy_from_slice(&v.to_be_bytes());
        put(&mut b, 11, ruling as u64);
        put(&mut b, 12, offer as u64);
        put(&mut b, 13, lapse as u64);
        b
    }

    #[test]
    fn lapse_default_reads_as_three_shares_and_nothing_else() {
        assert_eq!(Lapse::of(0), Some(Lapse::ToSeller));
        assert_eq!(Lapse::of(1), Some(Lapse::Half));
        assert_eq!(Lapse::of(2), Some(Lapse::FullRefund));
        assert_eq!(Lapse::of(3), None);
        for l in Lapse::ALL {
            assert_eq!(Lapse::of(l.word()), Some(l));
            assert_eq!(share(l.word()), Some(l.bps()));
        }
        let c = of_bytes(&quote(2, 2 * 86_400, 7 * 86_400)).unwrap();
        assert_eq!((c.offer_window, c.ruling_window, c.lapse), (2 * 86_400, 7 * 86_400, Lapse::FullRefund));
        assert_eq!(of_bytes(&quote(0, 1, 1)).unwrap().lapse, Lapse::ToSeller);
        assert_eq!(of_bytes(&quote(1, 1, 1)).unwrap().lapse, Lapse::Half);
        assert!(of_bytes(&quote(3, 1, 1)).is_err());
        assert_eq!([share(0), share(1), share(2), share(3)], [Some(0), Some(5_000), Some(10_000), None], "三档份额,集外不认");
        let spelled = format!("0x{}", quote(2, 60, 60).iter().map(|b| format!("{b:02x}")).collect::<String>());
        assert_eq!(of_spelling(&spelled).unwrap().lapse, Lapse::FullRefund);
    }

    #[test]
    fn the_five_cases_end_as_the_standard_settles_them() {
        for l in Lapse::ALL {
            assert_eq!(Case::NotTaken.outcome(l, false), Outcome::Refund);
            assert_eq!(Case::NotTaken.outcome(l, true), Outcome::Redraw);
            assert_eq!(Case::ResignBeforeDelivery.outcome(l, true), Outcome::Refund);
            assert_eq!(Case::ResignAfterDelivery.outcome(l, false), Outcome::ConfirmAsUsual);
            assert_eq!(Case::ResignInDispute.outcome(l, false), Outcome::Lapse(l));
            assert_eq!(Case::NoRuling.outcome(l, true), Outcome::Lapse(l));
        }
    }
}
