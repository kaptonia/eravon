use crate::mark;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Registration {
    Held,
    NotHeld,
    Unverified,
}

pub const NOTE_NOT_REGISTERED: &str = "卖家尚未登记";
pub const NOTE_UNVERIFIED: &str = "登记未能多端点核实";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    Signed,
    Admitted,
    Hidden(&'static str),
}

impl Admission {
    pub const fn word(self) -> &'static str {
        match self {
            Admission::Signed => "signed",
            Admission::Admitted => "admitted",
            Admission::Hidden(_) => "hidden",
        }
    }

    pub const fn choosable(self) -> bool {
        !matches!(self, Admission::Hidden(_))
    }

    pub const fn note(self) -> &'static str {
        match self {
            Admission::Hidden(n) => n,
            _ => "",
        }
    }
}

pub fn admit(r: Registration) -> Admission {
    match r {
        Registration::Held => Admission::Admitted,
        Registration::NotHeld => Admission::Hidden(NOTE_NOT_REGISTERED),
        Registration::Unverified => Admission::Hidden(NOTE_UNVERIFIED),
    }
}


pub fn gate(
    count: usize,
    by_call: &[(usize, String)],
    read: &mut dyn FnMut(usize, &str) -> Registration,
) -> Vec<Admission> {
    mark::mark(mark::FEATURE);
    (0..count)
        .map(|c| match by_call.iter().find(|(i, _)| *i == c) {
            None => Admission::Signed,
            Some((_, digest)) => admit(read(c, digest)),
        })
        .collect()
}
