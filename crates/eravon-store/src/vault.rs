use crate::fault::{Fault, R};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityKind {
    Words,
    Existing,
}

impl IdentityKind {
    pub const fn word(self) -> &'static str {
        match self {
            IdentityKind::Words => "words",
            IdentityKind::Existing => "existing",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seat {
    Seller,
    Buyer,
}

impl Seat {
    pub const fn word(self) -> &'static str {
        match self {
            Seat::Seller => "seller",
            Seat::Buyer => "buyer",
        }
    }
    pub fn need(s: &str) -> R<Self> {
        match s {
            "seller" => Ok(Seat::Seller),
            "buyer" => Ok(Seat::Buyer),
            _ => Err(Fault::Form(format!("seat 只有 seller 与 buyer 两种,档上是 {s:?}"))),
        }
    }
}

