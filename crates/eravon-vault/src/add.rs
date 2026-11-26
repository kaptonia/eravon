use eravon_store::vault::Seat;

use crate::session::Session;
use crate::vault::role_of;
use crate::vfault::{Fault, R};
use crate::derive;

pub fn outside_key() -> Fault {
    Fault::PartyKey("只收本客户端生成的钥:裸私钥与密钥文件不进任何一席;新建一把,或用助记词导入,或用整机备份还原".into())
}

pub fn made(s: &Session, seat: Seat, label: &str, named: bool) -> R<String> {
    if named {
        crate::accounts::label_free(&s.vault()?, seat, label, None)?;
    }
    let secret = loop {
        let mut b = [0u8; 32];
        crate::key::fill_random(&mut b)?;
        if let Some(k) = crate::key::Secret::take(b) {
            break k;
        }
    };
    let address = secret.address().map(|a| a.hex()).ok_or_else(|| Fault::KeyFileBroken(String::new()))?;
    crate::accounts::file(label, false, |reg, checked| crate::identity::add_existing(reg, &secret, role_of(seat), false, checked))?;
    crate::accounts::adopt(s.data(), seat, &address)?;
    Ok(address)
}

pub fn derive_role(s: &Session, address: &str, role: &str, i: u32) -> R<(String, String)> {
    let (path, key) = role_key(s, address, role, i)?;
    Ok((path, derive::address(&key)?))
}

pub fn role_key(s: &Session, address: &str, role: &str, i: u32) -> R<(String, [u8; 32])> {
    let phrase = s.phrase_for(address)?;
    let seed = crate::words::seed(&phrase)?;
    let path = derive::path_of(role, i)?;
    let key = derive::at(&seed, &path)?;
    Ok((path, key))
}
