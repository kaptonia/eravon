#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Card {
    None,
    Passcode,
    Reseal,
    Dead,
    Recover,
    KeyFile,
    Reset,
    Damaged,
}

impl Card {
    pub const ALL: [Card; 8] = [Card::None, Card::Passcode, Card::Reseal, Card::Dead, Card::Recover, Card::KeyFile, Card::Reset, Card::Damaged];

    pub const fn name(self) -> &'static str {
        match self {
            Card::None => "none",
            Card::Passcode => "passcode",
            Card::Reseal => "reseal",
            Card::Dead => "dead",
            Card::Recover => "recover",
            Card::KeyFile => "keyfile",
            Card::Reset => "reset",
            Card::Damaged => "damaged",
        }
    }

    pub fn of(vault: &crate::shell::Vault, recover: bool, reseal: bool, recoverable: bool, primary: Option<crate::keybox::PrimaryKind>) -> Card {
        if let crate::shell::Vault::Damaged(_) = vault {
            return Card::Damaged;
        }
        if !vault.gate_up() {
            return Card::None;
        }
        let locked_out = vault.is(crate::keybox::State::LockedOut);
        if locked_out {
            return if recoverable { Card::Dead } else { Card::Reset };
        }
        if recover {
            return match primary {
                Some(crate::keybox::PrimaryKind::KeyFile) => Card::KeyFile,
                _ => Card::Recover,
            };
        }
        if reseal { Card::Reseal } else { Card::Passcode }
    }
}
