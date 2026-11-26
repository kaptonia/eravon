use crate::roles::Role;

pub const PURPOSE: u32 = 44;
pub const COIN: u32 = 60;
pub const PROJECT: u32 = 2;
pub const AUTHOR_GROUP: u32 = 0;
pub const GRANTEE_GROUP: u32 = 1;
pub const AUTHOR_INDEX: u32 = 0;
pub const GRANTEE_INDEX: u32 = 0;
pub const HARDENED: u32 = 0x8000_0000;
pub const WORDS: usize = 12;
pub const ENTROPY_BYTES: usize = 16;

pub fn group_of(role: Role) -> u32 {
    match role {
        Role::Author => AUTHOR_GROUP,
        Role::Grantee => GRANTEE_GROUP,
    }
}

pub fn index_of(role: Role) -> u32 {
    match role {
        Role::Author => AUTHOR_INDEX,
        Role::Grantee => GRANTEE_INDEX,
    }
}

pub fn path(role: Role) -> [u32; 5] {
    [PURPOSE | HARDENED, COIN | HARDENED, PROJECT | HARDENED, group_of(role), index_of(role)]
}

pub fn path_text(role: Role) -> String {
    format!("m/{PURPOSE}'/{COIN}'/{PROJECT}'/{}/{}", group_of(role), index_of(role))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_seat_paths_are_this_products_rows() {
        use crate::roles::Role;
        let row = |r: &str| eravon_literals::registry::DERIVATION_PATHS.iter().find(|x| x.role == r).map(|x| x.path.to_string());
        assert_eq!(Some(super::path_text(Role::Author)), row(crate::derive::ROLE_SELLER));
        assert_eq!(Some(super::path_text(Role::Grantee)), row(crate::derive::ROLE_BUYER));
    }
}
