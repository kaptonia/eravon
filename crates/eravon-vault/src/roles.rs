#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Author,
    Grantee,
}

impl Role {
    pub const ALL: [Role; 2] = [Role::Author, Role::Grantee];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Author => "seller",
            Role::Grantee => "buyer",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Author => crate::lang::t(crate::lang::Key::IdSeatAuthor),
            Role::Grantee => crate::lang::t(crate::lang::Key::IdSeatGrantee),
        }
    }

    pub fn other(self) -> Role {
        match self {
            Role::Author => Role::Grantee,
            Role::Grantee => Role::Author,
        }
    }
}
